//! Verify: mode1 (reverse-volt) delta=N produces the SAME curve as
//! mode0 (freq-offset) with the equivalent kHz that mode1 resolved to.
//!
//! From probe_mode_cmp: mode1 delta=500 at 800mV (idx56) → +225 MHz.
//! So mode0 offset=225000 kHz should produce the same curve.
//! Also test at 950mV (idx80): mode1 delta=500 → +285 MHz → mode0 offset=285000.
//! And at 700mV (idx40): mode1 delta=500 → +180 MHz → mode0 offset=180000.
//!
//! Run as admin.
use nvapi::initialize;
use nvapi::sys::api::{
    NvAPI_EnumPhysicalGPUs, NvAPI_GPU_ClockClkVfPointsGetControl,
    NvAPI_GPU_ClockClkVfPointsGetInfo, NvAPI_GPU_ClockClkVfPointsGetStatus,
    NvAPI_GPU_ClockClkVfPointsSetControl,
};
use nvapi::sys::gpu::clock::private::*;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use nvapi::sys::api::NvVersion;
use std::ptr;

fn get_info(gpu: NvPhysicalGpuHandle) -> Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1> {
    let mut info = Box::new(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1::default());
    unsafe { NvAPI_GPU_ClockClkVfPointsGetInfo(gpu, ptr::from_mut(&mut *info).cast()) };
    info
}

fn get_status(gpu: NvPhysicalGpuHandle, info: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1)
    -> Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1> {
    let mut s = Box::new(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1::default());
    info.seed_status_header(&mut s);
    unsafe { NvAPI_GPU_ClockClkVfPointsGetStatus(gpu, ptr::from_mut(&mut *s).cast()) };
    s
}

fn write_point(gpu: NvPhysicalGpuHandle, idx: usize, freq_mode: bool, value: u32,
               info: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1) {
    let mut snap: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE> =
        Box::new(unsafe { std::mem::zeroed() });
    snap.version = NvVersion::with_version(clk_vfp_control::MAGIC);
    snap.seed_masks_from_info(info);
    unsafe { NvAPI_GPU_ClockClkVfPointsGetControl(gpu, ptr::from_mut(&mut *snap).cast()) };
    snap.set_mask_bit(0, idx);
    snap.set_record_type(0, idx, 8);
    if freq_mode { snap.set_absolute(0, idx, value); }
    else { snap.set_delta(0, idx, value as i16); }
    unsafe { NvAPI_GPU_ClockClkVfPointsSetControl(gpu, ptr::from_ref(&*snap).cast()) };
}

fn main() {
    let _ = initialize();
    let mut h = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut n = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut h, &mut n) };
    let gpu = h[0];

    let info = get_info(gpu);
    let baseline = get_status(gpu, &info);

    // Test cases: (idx, delta_01mv, expected_effect_mhz)
    let tests = [
        (56usize, 500i16, 225u32),  // 800mV → +225 MHz
        (80, 500, 285),             // 950mV → +285 MHz
        (40, 500, 180),             // 700mV → +180 MHz
    ];

    for (idx, delta, expected_effect) in tests {
        let def = baseline.freq_default_mhz(0, idx).unwrap_or(0);
        let volt = baseline.voltage_uv(0, idx).unwrap_or(0);
        let mode0_khz = expected_effect * 1000; // kHz offset = MHz * 1000

        eprintln!("\n########## Point {idx} ({:.0}mV, def={def} MHz) ##########", volt as f64 / 1000.0);
        eprintln!("## reverse-volt delta={delta} → expected +{expected_effect} MHz → freq-offset {mode0_khz} kHz");

        // Mode 1 (reverse-volt)
        write_point(gpu, idx, false, delta as u32, &info);
        let st1 = get_status(gpu, &info);
        let cur1 = st1.freq_current_mhz(0, idx).unwrap_or(0);
        let eff1 = cur1 as i64 - def as i64;
        write_point(gpu, idx, true, 0, &info); // restore

        // Mode 0 (freq-offset) with the equivalent kHz
        write_point(gpu, idx, true, mode0_khz, &info);
        let st0 = get_status(gpu, &info);
        let cur0 = st0.freq_current_mhz(0, idx).unwrap_or(0);
        let eff0 = cur0 as i64 - def as i64;
        write_point(gpu, idx, true, 0, &info); // restore

        eprintln!("  reverse-volt: cur={cur1}, effect={eff1}");
        eprintln!("  freq-offset:  cur={cur0}, effect={eff0}");

        if cur1 == cur0 {
            eprintln!("  EXACT MATCH! reverse-volt delta={delta} == freq-offset {mode0_khz} kHz");
        } else {
            eprintln!("  MISMATCH: {cur1} vs {cur0} (delta={})", cur1 as i64 - cur0 as i64);
        }

        // Full curve comparison (±10 points)
        eprintln!("\n  idx,voltage,def,cur_rev_volt,cur_freq_off,diff");
        // Re-do both writes for curve dump
        write_point(gpu, idx, false, delta as u32, &info);
        let s1 = get_status(gpu, &info);
        write_point(gpu, idx, true, 0, &info);
        write_point(gpu, idx, true, mode0_khz, &info);
        let s0 = get_status(gpu, &info);
        write_point(gpu, idx, true, 0, &info);

        let lo = idx.saturating_sub(10);
        let hi = (idx + 10).min(126);
        for i in lo..=hi {
            let v = s1.voltage_uv(0, i).unwrap_or(0);
            let d = s1.freq_default_mhz(0, i).unwrap_or(0);
            let c1 = s1.freq_current_mhz(0, i).unwrap_or(0);
            let c0 = s0.freq_current_mhz(0, i).unwrap_or(0);
            let diff = c1 as i64 - c0 as i64;
            eprintln!("  {i},{v},{d},{c1},{c0},{diff}");
        }
    }
    eprintln!("\nDone.");
}
