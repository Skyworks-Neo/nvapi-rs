//! Guard-page version/size brute-forcer for NVAPI versioned structs —
//! the technique HYDRA 2.2B PRO uses in its `Detect` (nvapioc.cpp,
//! sub_180006B30/sub_180006F20): sandwich the candidate buffer between
//! PAGE_NOACCESS pages so an out-of-bounds handler read/write faults
//! immediately instead of corrupting the heap, then sweep a (version,
//! size) grid controlled by environment variables.
//!
//! Rust has no stable SEH, so the grid driver runs each candidate in a
//! sacrificial CHILD PROCESS (the tool re-spawns itself with --single);
//! a child that dies with 0xC0000005 is itself the "reject" data point.
//!
//! GET-semantics only: the resolved interface is called once with the
//! candidate buffer. Refuses nothing mechanically, but pointing it at a
//! SET interface is on you.
//!
//! Usage:
//!   cargo run --release --example probe_guard_scan -- --id 0x9DF23CA1
//!   env: NVPROBE_VER_MIN/MAX (default 1..=7)
//!        NVPROBE_SIZE_MIN/MAX/STEP (default 8..=1024 step 4)
//!   child protocol (internal): --single <id> <ver> <size>
//!
//! Output: one line per candidate: `R <ver> <size> <status|CRASH>` —
//! status 0 means the driver accepted that (version, size) pair.

use std::env;
use std::process::Command;

type QIFn = unsafe extern "C" fn(u32) -> *const ();
type GpuFn = unsafe extern "C" fn(usize, *mut core::ffi::c_void) -> i32;

#[link(name = "kernel32")]
extern "system" {
    fn VirtualAlloc(addr: *mut u8, size: usize, alloc: u32, prot: u32) -> *mut u8;
    fn VirtualProtect(addr: *mut u8, size: usize, newprot: u32, oldprot: *mut u32) -> i32;
}

const MEM_COMMIT: u32 = 0x1000;
const MEM_RESERVE: u32 = 0x2000;
const PAGE_READWRITE: u32 = 0x04;
const PAGE_NOACCESS: u32 = 0x01;
const PAGE_SIZE: usize = 0x1000;

fn qi(id: u32) -> QIFn {
    // resolve through nvapi-sys's QueryInterface binding
    let p = nvapi::sys::nvapi_QueryInterface(id).expect("QueryInterface returned NULL");
    unsafe { std::mem::transmute(p) }
}

/// One candidate call, buffer sandwiched between PAGE_NOACCESS pages.
/// Tight layout: the buffer's LAST byte is flush against the trailing
/// guard page, so any handler access past `size` faults immediately.
fn single(id: u32, ver: u32, size: usize) -> i32 {
    let size = size.max(4) & !3; // dword-aligned
    let content = ((size + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE;
    let region = PAGE_SIZE + content + PAGE_SIZE;
    let base = unsafe {
        VirtualAlloc(
            std::ptr::null_mut(),
            region,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        )
    };
    assert!(!base.is_null(), "VirtualAlloc failed");
    let mut old = 0u32;
    // guard sandwich: first and last page PAGE_NOACCESS
    unsafe {
        VirtualProtect(base, PAGE_SIZE, PAGE_NOACCESS, &mut old);
        VirtualProtect(
            base.add(PAGE_SIZE + content),
            PAGE_SIZE,
            PAGE_NOACCESS,
            &mut old,
        );
    }
    // buffer ends exactly at the trailing guard page
    let buf = unsafe { base.add(PAGE_SIZE + content - size) };
    // magic = version << 16 | size (NvVersion convention, e.g. 0x10028 = v1/40B)
    unsafe { (buf as *mut u32).write_unaligned(((ver & 0xFFFF) << 16) | size as u32) };

    let f: GpuFn = unsafe { std::mem::transmute(qi(id)) };

    // enumerate first GPU
    nvapi::initialize().expect("init");
    let gpus = nvapi::PhysicalGpu::enumerate().expect("enum");

    let t0 = std::time::Instant::now();
    let h = *gpus[0].handle();
    let hptr = unsafe { std::mem::transmute::<_, *const core::ffi::c_void>(h) } as usize;
    let st = unsafe { f(hptr, buf as *mut core::ffi::c_void) };
    let dt = t0.elapsed().as_micros();
    // count nonzero dwords the driver left (format fingerprint)
    let mut nz = 0usize;
    let words = unsafe { std::slice::from_raw_parts(buf as *const u32, size / 4) };
    for (i, w) in words.iter().enumerate() {
        if i == 0 {
            continue; // our own magic
        }
        if *w != 0 {
            nz += 1;
        }
    }
    println!("R {ver} {size} {st} nz={nz} {dt}us");
    st
}

fn env_num(name: &str, default: u32) -> u32 {
    env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() >= 5 && args[1] == "--single" {
        let id = u32::from_str_radix(args[2].trim_start_matches("0x"), 16).unwrap();
        let ver: u32 = args[3].parse().unwrap();
        let size: usize = args[4].parse().unwrap();
        std::process::exit(single(id, ver, size));
    }

    let id = args
        .iter()
        .position(|a| a == "--id")
        .and_then(|i| args.get(i + 1))
        .map(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap())
        .expect("usage: probe_guard_scan --id 0xXXXXXXXX");
    // sanity-print what interface this is, if nvapi-rs knows it
    println!("guard-scan QI id 0x{id:08X} (GET semantics; do NOT point at SET ids)");

    let ver_min = env_num("NVPROBE_VER_MIN", 1);
    let ver_max = env_num("NVPROBE_VER_MAX", 7);
    let size_min = env_num("NVPROBE_SIZE_MIN", 8) as usize;
    let size_max = env_num("NVPROBE_SIZE_MAX", 1024) as usize;
    let step = env_num("NVPROBE_STEP", 4) as usize;

    let exe = env::current_exe().unwrap();
    let mut accepted = 0;
    let mut crashed = 0;
    for ver in ver_min..=ver_max {
        for size in (size_min..=size_max).step_by(step.max(1)) {
            let out = Command::new(&exe)
                .arg("--single")
                .arg(format!("{id:x}"))
                .arg(ver.to_string())
                .arg(size.to_string())
                .output()
                .expect("spawn child");
            let code = out.status.code().unwrap_or(-1);
            let stdout = String::from_utf8_lossy(&out.stdout);
            if let Some(line) = stdout.lines().find(|l| l.starts_with("R ")) {
                println!("{line}");
                if line.split(' ').nth(3) == Some("0") {
                    accepted += 1;
                }
            } else {
                println!("R {ver} {size} CRASH exit={code}");
                crashed += 1;
            }
        }
    }
    println!("summary: {accepted} accepted, {crashed} crashed");
}
