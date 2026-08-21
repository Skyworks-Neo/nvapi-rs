//! Probe GetAllClocks V2 raw status + how many extended domains are non-zero.
use nvapi::initialize;
use nvapi::sys::api::{NvAPI_GPU_GetAllClocks, NvAPI_EnumPhysicalGPUs};
use nvapi::sys::gpu::clock::private::NV_GPU_CLOCK_INFO_V2;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use std::ptr;

fn main() {
    let _ = initialize();
    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    let gpu = handles[0];

    let mut data = NV_GPU_CLOCK_INFO_V2 {
        version: nvapi::sys::api::NvVersion::new(
            std::mem::size_of::<NV_GPU_CLOCK_INFO_V2>(),
            2,
        ),
        ..Default::default()
    };
    let st = unsafe { NvAPI_GPU_GetAllClocks(gpu, ptr::from_mut(&mut data).cast()) };
    println!("GetAllClocks st={st}");
    if st != 0 {
        return;
    }
    let mut n = 0;
    for (i, d) in data.extended_domain.iter().enumerate() {
        if d.effective_frequency != 0 {
            println!("  ext[{i:2}] = {} kHz (ratio_dom={} ratio={})",
                d.effective_frequency, d.ratio_domain as u32, d.ratio);
            n += 1;
        }
    }
    println!("{n} non-zero extended domains");
    let mut np = 0;
    for (i, d) in data.domain.iter().enumerate() {
        if d.is_present() {
            println!("  dom[{i:2}] present, base={} kHz", d.frequency);
            np += 1;
        }
    }
    println!("{np} present base domains");
}
