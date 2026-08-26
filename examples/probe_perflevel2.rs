//! Read the perf-level lock state via 0x77D8F573 (escape 0x7000042 GET).
//!
//! Impl @0x18027D3D0 RE: `fn(hGpu, *mut u32, *mut u32, *mut u32)` — sends
//! escape 0x7000042 with a 0x40 buffer (hGpu inside), on success writes
//! 3 dwords from buffer offsets 0x34/0x38/0x3C into out params.
//! Companion of SetPerfLevel 0x75DD3E6A (escape 0x7000040, subcmd 0..4 =
//! P8/P5/P4/P3/P0 lock). This probe reads the state while the GPU is
//! locked (P8 via subcmd 0) to learn the encoding + whether "no lock"
//! is representable (a release hint).
//!
//! Run: cargo run --release -p nvapi --example probe_perflevel2

use nvapi::initialize;
use nvapi::sys::api::NvAPI_EnumPhysicalGPUs;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use nvapi::sys::nvapi_QueryInterface;

fn main() {
    let _ = initialize();

    type GetPerfLevel =
        unsafe extern "system" fn(NvPhysicalGpuHandle, *mut u32, *mut u32, *mut u32) -> i32;
    let call: Option<GetPerfLevel> = nvapi_QueryInterface(0x77D8F573)
        .ok()
        .and_then(|p| unsafe { std::mem::transmute(p) });
    let Some(call) = call else {
        println!("0x77D8F573 NOT RESOLVED");
        return;
    };

    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    let gpu = handles[0];
    println!("{count} physical GPU(s); 0x77D8F573(gpu, &a, &b, &c)");

    let (mut a, mut b, mut c) = (0u32, 0u32, 0u32);
    let st = unsafe { call(gpu, &mut a, &mut b, &mut c) };
    println!("status {st} -> a={a:#010X} ({a})  b={b:#010X} ({b})  c={c:#010X} ({c})");
}
