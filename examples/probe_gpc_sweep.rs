//! GPC V/F curve sweep: for each of the 127 GPC points (bank0, idx 0-126),
//! write a kHz frequency offset (mode 0 absolute), read back the FULL
//! GetStatus curve, then restore. Collects the complete interpolation
//! behavior per-point.
//!
//! Run as admin. Dumps results to stderr as CSV.
//! Usage: probe_gpc_sweep [delta_khz]
//!   Default delta_khz = 150000 (= 150 MHz, safe on 4060 Laptop, 7.5MHz granularity)
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
    -> Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1>
{
    let mut status = Box::new(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1::default());
    info.seed_status_header(&mut status);
    unsafe { NvAPI_GPU_ClockClkVfPointsGetStatus(gpu, ptr::from_mut(&mut *status).cast()) };
    status
}

fn get_control(gpu: NvPhysicalGpuHandle, info: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1)
    -> Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE>
{
    let mut ctrl: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE> =
        Box::new(unsafe { std::mem::zeroed() });
    ctrl.version = NvVersion::with_version(clk_vfp_control::MAGIC);
    ctrl.seed_masks_from_info(info);
    unsafe { NvAPI_GPU_ClockClkVfPointsGetControl(gpu, ptr::from_mut(&mut *ctrl).cast()) };
    ctrl
}

/// Write a kHz frequency offset (mode 0 absolute) to one point, then SetControl.
fn set_point_abs(gpu: NvPhysicalGpuHandle, idx: usize, value_khz: u32,
             info: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1)
{
    let mut snap = get_control(gpu, info);
    snap.set_mask_bit(0, idx);
    snap.set_record_type(0, idx, 8); // bank0 V/F point
    snap.set_absolute(0, idx, value_khz);
    unsafe { NvAPI_GPU_ClockClkVfPointsSetControl(gpu, ptr::from_ref(&*snap).cast()) };
}

/// Write a 0.1mV voltage-axis delta (mode 1 delta) to one point, then SetControl.
fn set_point_delta(gpu: NvPhysicalGpuHandle, idx: usize, value_01mv: i16,
             info: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1)
{
    let mut snap = get_control(gpu, info);
    snap.set_mask_bit(0, idx);
    snap.set_record_type(0, idx, 8);
    snap.set_delta(0, idx, value_01mv);
    unsafe { NvAPI_GPU_ClockClkVfPointsSetControl(gpu, ptr::from_ref(&*snap).cast()) };
}

/// Restore a point to stock (mode 0, value 0).
fn restore_point(gpu: NvPhysicalGpuHandle, idx: usize,
             info: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1)
{
    set_point_abs(gpu, idx, 0, info);
}

fn main() {
    let delta_khz: u32 = std::env::args()
        .nth(1).and_then(|s| s.parse().ok()).unwrap_or(150000);

    let _ = initialize();
    let mut h = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut n = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut h, &mut n) };
    let gpu = h[0];

    let info = get_info(gpu);

    // 1. Baseline: read stock curve
    let baseline = get_status(gpu, &info);

    // 2. Sweep: for each GPC point, write kHz offset, read back, restore
    eprintln!("=== GPC sweep: delta_khz={delta_khz} ({:.1} MHz) ===",
        delta_khz as f64 / 1000.0);
    eprintln!("idx,voltage_uV,def_stock,cur_stock,cur_after,effect_mhz");

    for idx in 0..=126usize {
        if info.point_present(0, idx) != Some(true) { continue; }

        let volt = baseline.voltage_uv(0, idx).unwrap_or(0);
        let def_stock = baseline.freq_default_mhz(0, idx).unwrap_or(0);
        let cur_stock = baseline.freq_current_mhz(0, idx).unwrap_or(0);

        set_point_abs(gpu, idx, delta_khz, &info);
        let after = get_status(gpu, &info);
        let cur_after = after.freq_current_mhz(0, idx).unwrap_or(0);
        restore_point(gpu, idx, &info);

        let effect = cur_after as i64 - cur_stock as i64;
        eprintln!("{idx},{volt},{def_stock},{cur_stock},{cur_after},{effect}");
    }

    // 3. Curve shape around a few points to see flattening + slope limit
    for &test_idx in &[0usize, 40, 80, 120] {
        eprintln!("\n=== Curve shape around point {test_idx} (delta={delta_khz} kHz) ===");
        eprintln!("idx,voltage_uV,def_mhz,cur_before,cur_after,delta_mhz");

        let before = get_status(gpu, &info);
        set_point_abs(gpu, test_idx, delta_khz, &info);
        let after = get_status(gpu, &info);

        let lo = if test_idx >= 10 { test_idx - 10 } else { 0 };
        let hi = (test_idx + 10).min(126);
        for i in lo..=hi {
            let v = after.voltage_uv(0, i).unwrap_or(0);
            let d = after.freq_default_mhz(0, i).unwrap_or(0);
            let cb = before.freq_current_mhz(0, i).unwrap_or(0);
            let ca = after.freq_current_mhz(0, i).unwrap_or(0);
            eprintln!("{i},{v},{d},{cb},{ca},{}", ca as i64 - cb as i64);
        }
        restore_point(gpu, test_idx, &info);
    }

    // 4. Also test mode 1 (0.1mV delta) on a few points for comparison
    eprintln!("\n=== Mode 1 (0.1mV delta) comparison ===");
    eprintln!("idx,voltage_uV,def_stock,cur_stock,delta_01mv,cur_after,effect_mhz");
    for &(idx, d) in &[(0usize, 1i16), (40, 1), (80, 1), (80, 5), (80, 10), (80, 20)] {
        let def_stock = baseline.freq_default_mhz(0, idx).unwrap_or(0);
        let cur_stock = baseline.freq_current_mhz(0, idx).unwrap_or(0);
        let volt = baseline.voltage_uv(0, idx).unwrap_or(0);
        set_point_delta(gpu, idx, d, &info);
        let after = get_status(gpu, &info);
        let cur_after = after.freq_current_mhz(0, idx).unwrap_or(0);
        restore_point(gpu, idx, &info);
        let effect = cur_after as i64 - cur_stock as i64;
        eprintln!("{idx},{volt},{def_stock},{cur_stock},{d},{cur_after},{effect}");
    }

    eprintln!("\nDone.");
}
