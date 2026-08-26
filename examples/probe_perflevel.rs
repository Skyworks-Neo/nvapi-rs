//! Probe the real semantics of 0x75DD3E6A (2026-08-26 correction).
//!
//! Impl @0x1802B2260 RE: `fn(hGpu: i32, subcmd: i32)` — elevation-free,
//! RM escape cmd 0x7000040 with a 0x38B workbuf: hGpu@+0x30, subcmd@+0x34
//! (same layout as the RatedTdp trio's 0x7000048 escape). NO level field in
//! the buffer — the caller's 2nd arg IS the sub-command:
//!   0 -> lock P8, 1 -> P5, 2 -> P4, 3 -> P3, 4 -> P0   (live-verified,
//!   user-measured; NOT the 0=Adaptive/1=MaxPerf/2=Auto power-mode dropdown)
//! This probe hunts the RELEASE sub-command (candidates -1 and 16, the
//! SetForcePstate release sentinel) by trying each and reporting status.
//!
//! Run: cargo run --release -p nvapi --example probe_perflevel -- [values...]

use nvapi::initialize;
use nvapi::sys::api::NvAPI_EnumPhysicalGPUs;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use nvapi::sys::nvapi_QueryInterface;

fn main() {
    let _ = initialize();

    type SetPerfLevel = unsafe extern "system" fn(NvPhysicalGpuHandle, i32) -> i32;
    let call: Option<SetPerfLevel> = nvapi_QueryInterface(0x75DD3E6A)
        .ok()
        .and_then(|p| unsafe { std::mem::transmute(p) });
    let Some(call) = call else {
        println!("0x75DD3E6A NOT RESOLVED");
        return;
    };

    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    let gpu = handles[0];
    println!("{count} physical GPU(s); probing 0x75DD3E6A(gpu, subcmd)");

    // Values to try, from argv or the release candidates.
    let args: Vec<i32> = std::env::args()
        .skip(1)
        .map(|a| a.parse().expect("i32 arg"))
        .collect();
    let values: Vec<i32> = if args.is_empty() {
        vec![-1, 16]
    } else {
        args
    };

    for v in values {
        let st = unsafe { call(gpu, v) };
        println!("subcmd {v:>6} ({:#010X}) -> status {st}", v as u32);
        // status 0 = accepted; check lock state externally via
        // `nvoc-cli get-pstate-native` between runs (Locked: header).
    }
}
