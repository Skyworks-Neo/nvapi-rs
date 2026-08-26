//! Live read-only probe for the private PerfVfeEqu / PerfVfeVar family
//! (escape 0x070001C6, R610.74 nvapi64_impl.dll RE 2026-08-26).
//!
//!   0x4C75C9FE  PerfVfeEquGetControl  @0x1802AA9C0  RM 0x2080A0B6
//!   0x68B798C4  PerfVfeEquSetControl  @0x1802ABD90  RM 0x2080E0B7  (never called)
//!   0x8D49471C  PerfVfeEquGetInfo     @0x1802AB410  RM 0x2080A0B5
//!   0x5D387298  PerfVfeVarGetControl  @0x1802AC850  RM 0x2080A0B3
//!   0x79FA23A2  PerfVfeVarSetControl  @0x1802AE0C0  RM 0x2080E0B0  (never called)
//!   0xB9DA41D6  PerfVfeVarGetInfo     @0x1802AD1E0  RM 0x2080A0B1
//!
//! All 6: `fn(hGpu, versioned-struct*)` — arg1 is the PHYSICAL GPU handle
//! (`!a1 -> -101`), NOT a domain selector (same lesson as core-voltage B1).
//! Escape buffer 0x100440; hGpu @ dword[12]; RM cmd @ dword[13].
//!
//! Equ GetInfo accepted magics (sub_1802AB410): 83996 / 209092 / 221508 / 885828.
//!   Output: +4 mask echo (256 dwords, bits 0..8191), info entries from +1092
//!   stride 76B: {str@+0, u32@+4, u16@+6, type i8@+8 (0..14), extras@+34/+36}.
//! Equ GetControl accepted magics (sub_1802AA9C0):
//!   85016 / 209092 / 221508 / 352580 / 1410116.
//!   Input: +4 1024B entry-selection mask (copied into the escape);
//!   output entries from +1136 stride 172B, type tag 1/2/3/6/7.
//! Var magics: 68300 (0x10ACC) / 171976 (0x29FF8).
//!
//! Run: `cargo run --release -p nvapi --example probe_vfe`

use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use nvapi::sys::api::{NvAPI_EnumPhysicalGPUs, NvAPI_GPU_GetFullName};
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::nvapi_QueryInterface;
use nvapi::sys::types::NvAPI_ShortString;

const EQU_GET_CONTROL: u32 = 0x4C75C9FE;
const EQU_SET_CONTROL: u32 = 0x68B798C4;
const EQU_GET_INFO: u32 = 0x8D49471C;
const VAR_GET_CONTROL: u32 = 0x5D387298;
const VAR_SET_CONTROL: u32 = 0x79FA23A2;
const VAR_GET_INFO: u32 = 0xB9DA41D6;

const EQU_GET_INFO_MAGICS: &[u32] = &[83996, 209092, 221508, 885828];
const EQU_GET_CONTROL_MAGICS: &[u32] = &[85016, 209092, 221508, 352580, 1410116];
const VAR_MAGICS: &[u32] = &[68300, 171976];
const VAR_GET_INFO_MAGICS: &[u32] = &[70344, 70600, 489736, 3118440];

type RawFn = unsafe extern "C" fn(usize, *mut u8) -> i32;

fn resolve(id: u32, label: &str) -> Option<RawFn> {
    match nvapi_QueryInterface(id) {
        Ok(ptr) => {
            println!("0x{id:08X} {label:28} -> 0x{ptr:016X}  RESOLVED");
            if ptr == 0 {
                None
            } else {
                Some(unsafe { std::mem::transmute::<usize, RawFn>(ptr) })
            }
        }
        Err(e) => {
            println!("0x{id:08X} {label:28} -> NULL  ({e:?})");
            None
        }
    }
}

fn status_name(st: i32) -> &'static str {
    match st {
        0 => "OK",
        -4 => "NOT_INITIALIZED",
        -9 => "INVALID_ARGUMENT (magic rejected)",
        -14 => "NULL_PTR",
        -101 => "INVALID_HANDLE",
        -104 => "API_NOT_INITIALIZED (lifecycle/elevation gate?)",
        -130 => "ALLOC_FAILED",
        -163 => "NOT_SUPPORTED",
        _ => "other",
    }
}

/// Count nonzero bytes and first/last nonzero offsets in a region.
fn survey(buf: &[u8], label: &str) {
    let mut first: Option<usize> = None;
    let mut last: usize = 0;
    let mut nz = 0usize;
    for (i, &b) in buf.iter().enumerate() {
        if b != 0 {
            nz += 1;
            if first.is_none() {
                first = Some(i);
            }
            last = i;
        }
    }
    match first {
        Some(f) => println!(
            "    {label}: {nz} nonzero bytes, span 0x{f:X}..0x{last:X} (len 0x{:X})",
            buf.len()
        ),
        None => println!("    {label}: ALL ZERO"),
    }
}

fn hex_dump(buf: &[u8], at: usize, len: usize) {
    let end = (at + len).min(buf.len());
    let start = at.min(end);
    for row in (start..end).step_by(16) {
        let mut line = format!("      +0x{row:05X}:");
        for i in row..(row + 16).min(end) {
            line.push_str(&format!(" {:02X}", buf[i]));
        }
        println!("{line}");
    }
}

fn main() {
    // GCOFF wake dance (mobile dGPU; mirrors probe_clk_domains).
    let mut initialized = false;
    for attempt in 0..5 {
        match nvapi::initialize() {
            Ok(()) => {
                println!("(NvAPI_Initialize ok on attempt {attempt})\n");
                initialized = true;
                break;
            }
            Err(e) => {
                println!("(NvAPI_Initialize attempt {attempt}: {e:?})");
                let gpus = nvapi::PhysicalGpu::enumerate().unwrap_or_default();
                if let Some(gpu) = gpus.first() {
                    let _ = gpu.force_gc6_exit();
                }
                std::thread::sleep(std::time::Duration::from_millis(700));
            }
        }
    }
    if !initialized {
        println!("(could not initialize — QI may still resolve)\n");
    }

    println!("=== PerfVfeEqu / PerfVfeVar private family (escape 0x070001C6) ===");
    let equ_get_control = resolve(EQU_GET_CONTROL, "PerfVfeEquGetControl");
    resolve(EQU_SET_CONTROL, "PerfVfeEquSetControl (SET)");
    let equ_get_info = resolve(EQU_GET_INFO, "PerfVfeEquGetInfo");
    let var_get_control = resolve(VAR_GET_CONTROL, "PerfVfeVarGetControl");
    resolve(VAR_SET_CONTROL, "PerfVfeVarSetControl (SET)");
    let var_get_info = resolve(VAR_GET_INFO, "PerfVfeVarGetInfo");

    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut count = 0u32;
    let st = unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    println!("\nEnumPhysicalGPUs: status={st} count={count}");
    if st != 0 || count == 0 {
        return;
    }
    let gpu = handles[0];
    let h = gpu.as_ptr() as usize;
    let mut name = NvAPI_ShortString::default();
    let _ = unsafe { NvAPI_GPU_GetFullName(gpu, &mut name) };
    println!("GPU[0]: {}", name.to_string_lossy());

    // ---------- Equ GetInfo: mask is pure OUTPUT, zeroed input struct ------
    if let Some(f) = equ_get_info {
        println!("\n--- PerfVfeEquGetInfo (mask out, entries out) ---");
        for &magic in EQU_GET_INFO_MAGICS {
            let size = magic as usize + 0x2000; // generous headroom
            let mut buf = vec![0u8; size];
            buf[0..4].copy_from_slice(&magic.to_le_bytes());
            let st = unsafe { f(h, buf.as_mut_ptr()) };
            println!(
                "  magic {magic} (0x{magic:X}): status {st} = {}",
                status_name(st)
            );
            if st == 0 {
                // mask echo @ +4 (256 dwords)
                let mask = &buf[4..4 + 1024];
                let set_bits: usize = mask
                    .chunks_exact(4)
                    .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]).count_ones() as usize)
                    .sum();
                let nz_dwords: Vec<usize> = mask
                    .chunks_exact(4)
                    .enumerate()
                    .filter(|(_, c)| c != &[0, 0, 0, 0])
                    .map(|(i, _)| i)
                    .collect();
                println!(
                    "    mask: {set_bits} bits set in {} nonzero dwords (idx {nz_dwords:?})",
                    nz_dwords.len()
                );
                // entries @ +1092 stride 76 — count entries with type byte @+8 != 0
                let mut typed = 0usize;
                let mut first_entries: Vec<(usize, [u8; 24])> = Vec::new();
                for i in 0..8192usize {
                    let e = 1092 + i * 76;
                    if e + 76 > buf.len() {
                        break;
                    }
                    if buf[e + 8] != 0 {
                        typed += 1;
                        if first_entries.len() < 6 {
                            let mut snap = [0u8; 24];
                            snap.copy_from_slice(&buf[e..e + 24]);
                            first_entries.push((i, snap));
                        }
                    }
                }
                println!("    entries with type!=0: {typed}");
                for (i, snap) in &first_entries {
                    print!("      entry[{i}] +0..24:");
                    for b in snap {
                        print!(" {b:02X}");
                    }
                    println!();
                }
                survey(&buf[1028..1092], "header +1028..1092");
                // dump around first typed entry for layout calibration
                if let Some((i, _)) = first_entries.first() {
                    hex_dump(&buf, 1092 + i * 76 - 8, 76 + 16);
                }
                break; // first accepted magic wins
            }
        }
    }

    // ---------- Equ GetControl: seed modest input mask (bits 0..63) --------
    if let Some(f) = equ_get_control {
        println!("\n--- PerfVfeEquGetControl (mask IN/OUT, entries out) ---");
        for &magic in EQU_GET_CONTROL_MAGICS {
            let size = magic as usize + 0x2000;
            let mut buf = vec![0u8; size];
            buf[0..4].copy_from_slice(&magic.to_le_bytes());
            // seed 2 input mask dwords (bits 0..63)
            buf[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
            buf[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
            let st = unsafe { f(h, buf.as_mut_ptr()) };
            println!(
                "  magic {magic} (0x{magic:X}): status {st} = {}",
                status_name(st)
            );
            if st == 0 {
                let mask = &buf[4..4 + 1024];
                let set_bits: usize = mask
                    .chunks_exact(4)
                    .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]).count_ones() as usize)
                    .sum();
                println!("    mask echo: {set_bits} bits set");
                survey(&buf[1028..1136], "header +1028..1136");
                survey(&buf[1136..], "body +1136..end");
                // entries @ +1136 stride 172; type dword per RE at entry+0
                let mut typed = 0usize;
                let mut samples: Vec<(usize, [u8; 32])> = Vec::new();
                for i in 0..8192usize {
                    let e = 1136 + i * 172;
                    if e + 172 > buf.len() {
                        break;
                    }
                    let t = u32::from_le_bytes([buf[e], buf[e + 1], buf[e + 2], buf[e + 3]]);
                    if t != 0 {
                        typed += 1;
                        if samples.len() < 6 {
                            let mut snap = [0u8; 32];
                            snap.copy_from_slice(&buf[e..e + 32]);
                            samples.push((i, snap));
                        }
                    }
                }
                println!("    entries with type dword!=0: {typed}");
                for (i, snap) in &samples {
                    print!("      entry[{i}] +0..32:");
                    for b in snap {
                        print!(" {b:02X}");
                    }
                    println!();
                }
                if let Some((i, _)) = samples.first() {
                    hex_dump(&buf, 1136 + i * 172, 172);
                }
                // ALSO seed the full info mask (cascade, like the medium wrap)
                if let Some(info_fn) = equ_get_info {
                    let mut ibuf = vec![0u8; 83996 + 0x2000];
                    ibuf[0..4].copy_from_slice(&83996u32.to_le_bytes());
                    if unsafe { info_fn(h, ibuf.as_mut_ptr()) } == 0 {
                        let mut cbuf = vec![0u8; size];
                        cbuf[0..4].copy_from_slice(&magic.to_le_bytes());
                        for dword in 0..256usize {
                            let mut v = 0u32;
                            for b in 0..32 {
                                let idx = dword * 32 + b;
                                if idx < 8192 && ibuf[4 + idx / 8] & (1 << (idx % 8)) != 0 {
                                    v |= 1 << b;
                                }
                            }
                            cbuf[4 + dword * 4..8 + dword * 4].copy_from_slice(&v.to_le_bytes());
                        }
                        let st2 = unsafe { f(h, cbuf.as_mut_ptr()) };
                        println!(
                            "    full-info-mask cascade: status {st2} = {}",
                            status_name(st2)
                        );
                        if st2 == 0 {
                            let m2 = &cbuf[4..4 + 1024];
                            let bits2: usize = m2
                                .chunks_exact(4)
                                .map(|c| {
                                    u32::from_le_bytes([c[0], c[1], c[2], c[3]]).count_ones()
                                        as usize
                                })
                                .sum();
                            println!("      mask echo: {bits2} bits set");
                            survey(&cbuf[1136..], "cascade body +1136..end");
                            hex_dump(&cbuf, 1136, 0x100);
                        }
                    }
                }
                break;
            }
        }
    }

    // ---------- Var GetInfo -------------------------------------------------
    if let Some(f) = var_get_info {
        println!("\n--- PerfVfeVarGetInfo ---");
        for &magic in VAR_GET_INFO_MAGICS {
            let size = magic as usize + 0x2000;
            let mut buf = vec![0u8; size];
            buf[0..4].copy_from_slice(&magic.to_le_bytes());
            let st = unsafe { f(h, buf.as_mut_ptr()) };
            println!(
                "  magic {magic} (0x{magic:X}): status {st} = {}",
                status_name(st)
            );
            if st == 0 {
                survey(&buf[0..0x80.min(buf.len())], "header +0..0x80");
                survey(&buf, "whole buffer");
                // scan for repeated-stride structure: dump first 0x100 after header
                hex_dump(&buf, 0, 0x100);
                break;
            }
        }
    }

    // ---------- Var GetControl ----------------------------------------------
    if let Some(f) = var_get_control {
        println!("\n--- PerfVfeVarGetControl ---");
        for &magic in VAR_MAGICS {
            let size = magic as usize + 0x2000;
            let mut buf = vec![0u8; size];
            buf[0..4].copy_from_slice(&magic.to_le_bytes());
            // Var GetControl has a 32B header per RE — seed modest mask bits too
            if buf.len() >= 12 {
                buf[4..8].copy_from_slice(&0xFFFF_u32.to_le_bytes());
            }
            let st = unsafe { f(h, buf.as_mut_ptr()) };
            println!(
                "  magic {magic} (0x{magic:X}): status {st} = {}",
                status_name(st)
            );
            if st == 0 {
                survey(&buf[0..0x60.min(buf.len())], "header +0..0x60");
                hex_dump(&buf, 0, 0xC0);
                break;
            }
        }
    }

    println!("\n(probe complete — SET handlers never called)");
}
