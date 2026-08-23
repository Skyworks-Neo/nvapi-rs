//! GPU illumination (LED) probe — QueryIlluminationSupport + GetIllumination
//! for both attributes on every physical GPU. Harmless read-only; run on a
//! GPU with a controllable logo/SLI LED to confirm the wrapping (no such
//! hardware on the bench so far — laptop 4060 reports unsupported).
//!
//! Run: cargo run --release --example probe_illumination

use nvapi::initialize;
use nvapi::sys::api::{
    NvAPI_EnumPhysicalGPUs, NvAPI_GPU_GetIllumination, NvAPI_GPU_QueryIlluminationSupport,
};
use nvapi::sys::gpu::illumination::{
    IlluminationAttrib, NV_GPU_GET_ILLUMINATION_PARM,
    NV_GPU_QUERY_ILLUMINATION_SUPPORT_PARM,
};
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::nvapi::StructVersion;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use std::ptr;

fn main() {
    let _ = initialize();
    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS as usize];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    println!("{count} physical GPU(s)");

    for i in 0..count as usize {
        let gpu = handles[i];
        for (attr, name) in [
            (IlluminationAttrib::LogoBrightness, "Logo"),
            (IlluminationAttrib::SliBrightness, "SLI bridge"),
        ] {
            let mut q = NV_GPU_QUERY_ILLUMINATION_SUPPORT_PARM::versioned();
            q.hPhysicalGpu = gpu;
            q.attribute = attr.into();
            let st = unsafe { NvAPI_GPU_QueryIlluminationSupport(ptr::from_mut(&mut q)) };
            if st != 0 {
                println!("GPU{i} {name}: QueryIlluminationSupport st={st}");
                continue;
            }
            println!("GPU{i} {name}: supported={}", q.bSupported);
            if q.bSupported == 0 { continue; }
            let mut g = NV_GPU_GET_ILLUMINATION_PARM::versioned();
            g.hPhysicalGpu = gpu;
            g.attribute = attr.into();
            let st = unsafe { NvAPI_GPU_GetIllumination(ptr::from_mut(&mut g)) };
            println!("GPU{i} {name}: GetIllumination st={st} value={}", g.value);
        }
    }
}
