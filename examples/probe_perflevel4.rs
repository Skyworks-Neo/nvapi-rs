//! Test whether SetPowerMizerInfo 0x50016C78 releases the 0x75DD3E6A pstate
//! lock (escape 0x7000040). Impl @0x180261BC0:
//!   fn(hGpu, a2 in {1,2}, a3==3, a4 in {6,7}) -> escape 0x700003A
//! a2: 1 -> flag 0, 2 -> flag 1; a4: 6 -> mode 0, 7 -> mode 1.
//! Companion GET 0x76BFA16B reads current mode (6/7).
//!
//! Run: cargo run --release -p nvapi --example probe_perflevel4

use nvapi::initialize;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use nvapi::sys::api::NvAPI_EnumPhysicalGPUs;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::nvapi_QueryInterface;

type SetPowerMizerInfo = unsafe extern "system" fn(NvPhysicalGpuHandle, i32, i32, i32) -> i32;
type GetPowerMizerInfo = unsafe extern "system" fn(NvPhysicalGpuHandle) -> i32; // returns mode?

fn main() {
    let _ = initialize();

    let setm: Option<SetPowerMizerInfo> = nvapi_QueryInterface(0x50016C78)
        .ok()
        .and_then(|p| unsafe { std::mem::transmute(p) });
    let getm: Option<GetPowerMizerInfo> = nvapi_QueryInterface(0x76BFA16B)
        .ok()
        .and_then(|p| unsafe { std::mem::transmute(p) });

    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    let gpu = handles[0];

    if let Some(get) = getm {
        let mode = unsafe { get(gpu) };
        println!("0x76BFA16B GET mode raw ret: {mode}");
    } else {
        println!("0x76BFA16B NOT RESOLVED");
    }

    let Some(set) = setm else {
        println!("0x50016C78 NOT RESOLVED");
        return;
    };

    // Try both a2 variants under both modes.
    for (a2, a4) in [(1, 6), (2, 6), (1, 7), (2, 7), (1, 3), (2, 3)] {
        let st = unsafe { set(gpu, a2, 3, a4) };
        println!("SetPowerMizerInfo(gpu, {a2}, 3, {a4}) -> status {st}");
    }
}
