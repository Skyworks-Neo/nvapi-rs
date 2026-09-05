// Diagnostic: probe power-sensing channels on legacy GPUs (GT730/Fermi +
// R391.35). The IDA scan of 391.35 nvapi64.dll shows PowerMonitorGetInfo
// (0xC12EB19E, accepts stamp 0x30CA8 = v3|3240), PowerMonitorGetStatus
// (0xF40238EF, accepts v1|392 / v1|1436 / v1|3872), and
// ClientPowerTopologyGetStatus (0xEDCF624E) are ALL present in the dispatch
// table. This probe prints what each path actually returns on this GPU.
//
// Run: cargo test --test power_monitor_probe -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;
use nvapi::sys::nvapi::NvVersion;

#[test]
#[ignore]
fn power_monitor_probe() {
    use nvapi::sys::api::NvAPI_GPU_PowerMonitorGetInfo;
    use nvapi::sys::gpu::power::undocumented::{
        NV_GPU_POWER_MONITOR_GET_INFO_V1_2728, NV_GPU_POWER_MONITOR_GET_INFO_V3_3240,
        NV_GPU_POWER_MONITOR_GET_INFO_V4,
    };

    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    eprintln!("=== power probe ===");

    // GetInfo cascade exactly as nvoc sends it (v4 -> v3|3240 -> v1|2728)
    let v4 = {
        let mut info = NV_GPU_POWER_MONITOR_GET_INFO_V4 {
            version: NvVersion::new(
                std::mem::size_of::<NV_GPU_POWER_MONITOR_GET_INFO_V4>() as usize,
                4,
            ),
            ..Default::default()
        };
        unsafe { NvAPI_GPU_PowerMonitorGetInfo(*gpu.handle(), &mut info as *mut _ as *mut _) }
    };
    eprintln!("GetInfo v4|6312 : status={:#x}", v4 as i32);

    let mut info3 = NV_GPU_POWER_MONITOR_GET_INFO_V3_3240 {
        version: NvVersion::new(
            std::mem::size_of::<NV_GPU_POWER_MONITOR_GET_INFO_V3_3240>() as usize,
            3,
        ),
        ..Default::default()
    };
    let st3 =
        unsafe { NvAPI_GPU_PowerMonitorGetInfo(*gpu.handle(), &mut info3 as *mut _ as *mut _) };
    eprintln!("GetInfo v3|3240 : status={:#x}", st3 as i32);
    if st3 == 0 {
        eprintln!(
            "  v3 header: channel_mask={:#x} sample_count={} supported={}",
            info3.channel_mask,
            info3.sample_count,
            info3.b_supported.get()
        );
    }

    let mut info1 = NV_GPU_POWER_MONITOR_GET_INFO_V1_2728 {
        version: NvVersion::new(
            std::mem::size_of::<NV_GPU_POWER_MONITOR_GET_INFO_V1_2728>() as usize,
            1,
        ),
        ..Default::default()
    };
    let st1 =
        unsafe { NvAPI_GPU_PowerMonitorGetInfo(*gpu.handle(), &mut info1 as *mut _ as *mut _) };
    eprintln!("GetInfo v1|2728 : status={:#x}", st1 as i32);

    // GetStatus v1|392 with the v3 mask (if v3 succeeded) or ch0 only
    use nvapi::sys::api::NvAPI_GPU_PowerMonitorGetStatus;
    let masks: Vec<(String, u32)> = if st3 == 0 {
        vec![
            ("ch0-only".into(), 1),
            ("v3-full-mask".into(), info3.channel_mask),
        ]
    } else {
        vec![("ch0-only".into(), 1)]
    };
    for (label, mask) in masks {
        let mut buf = [0u8; 392];
        buf[0..4].copy_from_slice(&NvVersion::new(392, 1).data.to_le_bytes());
        buf[4..8].copy_from_slice(&mask.to_le_bytes());
        let st = unsafe { NvAPI_GPU_PowerMonitorGetStatus(*gpu.handle(), buf.as_mut_ptr().cast()) };
        eprintln!(
            "GetStatus v1|392 [{label:14} mask={mask:#x}]: status={:#x}",
            st as i32
        );
        if st == 0 {
            // dump first 8 dwords of payload after version+mask
            let vals: Vec<u32> = buf[8..8 + 32]
                .chunks_exact(4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            eprintln!("  payload dwords: {:?}", vals);
        }
    }

    // ClientPowerTopology GetStatus (public): what nvoc's power_usage uses
    use nvapi::sys::api::NvAPI_GPU_ClientPowerTopologyGetStatus;
    use nvapi::sys::gpu::power::undocumented::NV_GPU_CLIENT_POWER_TOPOLOGY_STATUS;
    let mut topo = NV_GPU_CLIENT_POWER_TOPOLOGY_STATUS::default();
    let st = unsafe {
        NvAPI_GPU_ClientPowerTopologyGetStatus(*gpu.handle(), &mut topo as *mut _ as *mut _)
    };
    eprintln!("PowerTopology GetStatus: status={:#x}", st as i32);
    if st == 0 {
        eprintln!("  entries: {:?}", topo.entries());
    }
}
