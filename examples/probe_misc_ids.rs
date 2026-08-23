//! Probe the four nvapioc/vertminer-sourced bindings added 2026-08-23:
//! SetForcePstate (0x025BFB10, private), GetPerfClocks (0x1EA54A3B),
//! GetSerialNumber (0x14B83A5F), RestartDisplayDriver (0xB4B26B65).
//! All read-only except RestartDisplayDriver which we only RESOLVE, never
//! call (it would bounce the display).
//!
//! Run: cargo run --release -p nvapi --example probe_misc_ids

use nvapi::initialize;
use nvapi::sys::api::{
    NvAPI_EnumPhysicalGPUs, NvAPI_GPU_GetPerfClocks, NvAPI_GPU_GetSerialNumber,
};
use nvapi::sys::gpu::clock::NV_GPU_PERF_CLOCKS;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::nvapi::StructVersion;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use nvapi::sys::api::NvAPI_RestartDisplayDriver;
use nvapi::sys::api::private::NvAPI_GPU_SetForcePstate;
use nvapi::sys::nvapi_QueryInterface;

fn resolve(id: u32, name: &str) {
    let p = unsafe { nvapi_QueryInterface(id) };
    let ok = match p { Ok(_) => "RESOLVED", Err(_) => "NULL/Err" };
    println!("{name:24} 0x{id:08X} -> {ok}");
}

fn main() {
    let _ = initialize();

    resolve(0x025BFB10, "SetForcePstate");
    resolve(0xB4B26B65, "RestartDisplayDriver"); // resolve only; never call here

    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS as usize];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    println!("{count} physical GPU(s)");

    for i in 0..count as usize {
        let gpu = handles[i];

        let mut serial = nvapi::sys::NvAPI_ShortString::default();
        let st = unsafe { NvAPI_GPU_GetSerialNumber(gpu, &mut serial) };
        let raw: Vec<u8> = serial.as_bytes()[..16].to_vec();
        println!(
            "GPU{i} GetSerialNumber st={st} cstr={:?} raw16={:02X?}",
            serial.as_cstr().map(|c| c.to_string_lossy().into_owned()),
            raw
        );

        // 10868-byte V2 buffer on the heap (too big for the stack comfort)
        let mut clocks = Box::new(<NV_GPU_PERF_CLOCKS as StructVersion<2>>::versioned());
        let st = unsafe { NvAPI_GPU_GetPerfClocks(gpu, 32, &mut *clocks) };
        println!(
            "GPU{i} GetPerfClocks st={st} pStateId={} memFreq1={} (expect NotSupported on Pascal+)",
            clocks.pStateId, clocks.memFreq1
        );
    }

    // SetForcePstate live-call is intentionally skipped: forcing a pstate
    // changes GPU state; we only verify resolution above.
    let _ = NvAPI_GPU_SetForcePstate;
}
