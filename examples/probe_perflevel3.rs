//! Live experiment matrix for the perf-level lock pair:
//!   SET 0x75DD3E6A (escape 0x7000040, a2 0..4 = P8/P5/P4/P3/P0 lock)
//!   GET 0x77D8F573 (escape 0x7000042, out (a,b,c) — currently locked P8 reads (1,1,0))
//!
//! Phase 1: SET 4 (P0) then GET — learn encoding per level.
//! Phase 2: SET 0 (P8) then GET — restore known state.
//! Phase 3: sweep candidate release values, print status + GET after each.
//! Phase 4: toggle test — SET 0 twice, see if re-applying releases.
//!
//! Run: cargo run --release -p nvapi --example probe_perflevel3 [-- setvals...]

use nvapi::initialize;
use nvapi::sys::api::NvAPI_EnumPhysicalGPUs;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use nvapi::sys::nvapi_QueryInterface;

type SetPerfLevel = unsafe extern "system" fn(NvPhysicalGpuHandle, i32) -> i32;
type GetPerfLevel =
    unsafe extern "system" fn(NvPhysicalGpuHandle, *mut u32, *mut u32, *mut u32) -> i32;

fn main() {
    let _ = initialize();

    let setf: Option<SetPerfLevel> = nvapi_QueryInterface(0x75DD3E6A)
        .ok()
        .and_then(|p| unsafe { std::mem::transmute(p) });
    let getf: Option<GetPerfLevel> = nvapi_QueryInterface(0x77D8F573)
        .ok()
        .and_then(|p| unsafe { std::mem::transmute(p) });
    let (Some(set), Some(get)) = (setf, getf) else {
        println!("QI failed");
        return;
    };

    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    let gpu = handles[0];

    let read = |tag: &str| {
        let (mut a, mut b, mut c) = (0u32, 0u32, 0u32);
        let st = unsafe { get(gpu, &mut a, &mut b, &mut c) };
        println!("  GET[{tag}] st={st} a={a} b={b} c={c}");
        (a, b, c)
    };

    println!("== baseline (locked P8 from subcmd 0) ==");
    read("base");

    println!("== phase1: SET 4 (P0) ==");
    println!("  status {}", unsafe { set(gpu, 4) });
    read("P0");

    println!("== phase2: SET 0 (P8) ==");
    println!("  status {}", unsafe { set(gpu, 0) });
    read("P8");

    println!("== phase3: release-candidate sweep ==");
    let args: Vec<i32> = std::env::args().skip(1).map(|a| a.parse().expect("i32")).collect();
    let candidates: Vec<i32> = if args.is_empty() {
        vec![
            5, 6, 7, 8, 9, 10, 12, 16, 17, 20, 32, 64, 100, 128, 255, 0x10000, 0x1000000,
            -2, -4, -8, -16, -17, -255, i32::MIN, -1,
        ]
    } else {
        args
    };
    for v in candidates {
        let st = unsafe { set(gpu, v) };
        let (a, b, c) = read(&format!("after {v}"));
        println!("  SET {v:>11} -> st={st}, GET=({a},{b},{c})");
    }

    println!("== phase4: toggle test (SET 0 twice) ==");
    println!("  status {}", unsafe { set(gpu, 0) });
    read("first");
    println!("  status {}", unsafe { set(gpu, 0) });
    read("second");
}
