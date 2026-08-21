//! Cross-check the sys-layer V/F-points structs against probe_vfp7's raw
//! dword dumps: read rec0/rec1 via the accessors AND dump raw dwords
//! +0x20..+0x70 for offset reconciliation.
use nvapi::initialize;
use nvapi::sys::api::NvAPI_EnumPhysicalGPUs;
use nvapi::sys::gpu::clock::private::*;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;

fn main() {
    let _ = initialize();
    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    let gpu = handles[0];

    use nvapi::sys::api::{NvAPI_GPU_ClockClkVfPointsGetInfo, NvAPI_GPU_ClockClkVfPointsGetStatus};
    use std::ptr;

    let mut info = Box::new(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1::default());
    println!("info magic: 0x{:X}", info.version.data);
    let st = unsafe { NvAPI_GPU_ClockClkVfPointsGetInfo(gpu, ptr::from_mut(&mut *info).cast()) };
    println!("GetInfo st={st}");

    let mut status = Box::new(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1::default());
    println!("status magic: 0x{:X}", status.version.data);
    info.seed_status_header(&mut status);
    let st = unsafe { NvAPI_GPU_ClockClkVfPointsGetStatus(gpu, ptr::from_mut(&mut *status).cast()) };
    println!("GetStatus st={st}");

    for idx in 0..2usize {
        println!("rec[{idx}] type={:?} voltage={:?} voltage2={:?} freq={:?}",
            status.record_type(0, idx), status.voltage(0, idx), status.voltage2(0, idx), status.freq_khz(0, idx));
        // raw dwords +0x10..+0x70 via rest
        let base = clk_vfp_status::REC1 + clk_vfp_status::STRIDE * idx;
        let off = base - 4;
        let row: Vec<String> = (0..0x70usize).step_by(4).map(|k| {
            format!("+{:02x}={:08x}", k, u32::from_le_bytes(status.rest[off + k..off + k + 4].try_into().unwrap()))
        }).collect();
        println!("  {}", row.join(" "));
    }
}
