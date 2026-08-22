//! Verify mode 1 (0.1mV delta) = voltage-axis lookup, and that it
//! produces the SAME curve as mode 0 (absolute kHz) with the equivalent
//! frequency.
//!
//! Test plan:
//! 1. For point at 800mV (idx 56, def=1890), delta=500
//!    → expected: freq at 850mV (idx 64, def=2085)
//! 2. Write mode1 delta=500 to idx56 → read GetStatus
//! 3. Restore
//! 4. Write mode0 absolute=2085000 kHz to idx56 → read GetStatus
//! 5. Compare: are the two curves identical?
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

fn set_abs(gpu: NvPhysicalGpuHandle, idx: usize, val_khz: u32,
           info: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1) {
    let mut snap: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE> =
        Box::new(unsafe { std::mem::zeroed() });
    snap.version = NvVersion::with_version(clk_vfp_control::MAGIC);
    snap.seed_masks_from_info(info);
    unsafe { NvAPI_GPU_ClockClkVfPointsGetControl(gpu, ptr::from_mut(&mut *snap).cast()) };
    snap.set_mask_bit(0, idx);
    snap.set_record_type(0, idx, 8);
    snap.set_absolute(0, idx, val_khz);
    unsafe { NvAPI_GPU_ClockClkVfPointsSetControl(gpu, ptr::from_ref(&*snap).cast()) };
}

fn set_delta(gpu: NvPhysicalGpuHandle, idx: usize, val_01mv: i16,
            info: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1) {
    let mut snap: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE> =
        Box::new(unsafe { std::mem::zeroed() });
    snap.version = NvVersion::with_version(clk_vfp_control::MAGIC);
    snap.seed_masks_from_info(info);
    unsafe { NvAPI_GPU_ClockClkVfPointsGetControl(gpu, ptr::from_mut(&mut *snap).cast()) };
    snap.set_mask_bit(0, idx);
    snap.set_record_type(0, idx, 8);
    snap.set_delta(0, idx, val_01mv);
    unsafe { NvAPI_GPU_ClockClkVfPointsSetControl(gpu, ptr::from_ref(&*snap).cast()) };
}

fn dump_curve(status: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1,
              label: &str, center: usize) {
    let lo = center.saturating_sub(10);
    let hi = (center + 10).min(126);
    eprintln!("\n=== {label} (around idx {center}) ===");
    eprintln!("idx,voltage_uV,def_mhz,cur_mhz,delta");
    for i in lo..=hi {
        let v = status.voltage_uv(0, i).unwrap_or(0);
        let d = status.freq_default_mhz(0, i).unwrap_or(0);
        let c = status.freq_current_mhz(0, i).unwrap_or(0);
        eprintln!("{i},{v},{d},{c},{}", c as i64 - d as i64);
    }
}

fn main() {
    let _ = initialize();
    let mut h = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut n = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut h, &mut n) };
    let gpu = h[0];

    let info = get_info(gpu);
    let baseline = get_status(gpu, &info);

    // Test cases: (idx, voltage_mV, delta_01mv, target_volt_mV, expected_def_at_target)
    let tests: &[(usize, &str, i16, &str, u32)] = &[
        // idx, label, delta, target_desc, expected_mode0_khz
        (56, "800mV", 500, "850mV", 2085000),  // 1890→2085, +195
        (80, "950mV", 500, "1000mV", 2535000),  // 2415→2535, +120
        (40, "700mV", 500, "750mV", 1710000),  // 1515→1710, +195
        (56, "800mV", 1000, "900mV", 2265000), // 1890→2265, +375
    ];

    for &(idx, vlabel, delta, tgt_desc, mode0_khz) in tests {
        let def_stock = baseline.freq_default_mhz(0, idx).unwrap_or(0);
        let volt = baseline.voltage_uv(0, idx).unwrap_or(0);

        eprintln!("\n\n########## Point {idx} ({vlabel} = {volt}uV, def={def_stock} MHz) ##########");
        eprintln!("## delta={delta} → target {tgt_desc}, expected mode0_khz={mode0_khz} ({:.1} MHz)", mode0_khz as f64 / 1000.0);

        // --- Mode 1 (delta) ---
        eprintln!("\n--- Mode 1 (delta={delta}) ---");
        set_delta(gpu, idx, delta, &info);
        let st1 = get_status(gpu, &info);
        let cur1 = st1.freq_current_mhz(0, idx).unwrap_or(0);
        let effect1 = cur1 as i64 - def_stock as i64;
        eprintln!("  cur_after = {cur1}, effect = {effect1} MHz");
        dump_curve(&st1, "Mode 1 delta", idx);

        // Restore
        set_abs(gpu, idx, 0, &info);

        // --- Mode 0 (absolute kHz) ---
        eprintln!("\n--- Mode 0 (absolute={mode0_khz} kHz = {:.1} MHz) ---", mode0_khz as f64 / 1000.0);
        set_abs(gpu, idx, mode0_khz, &info);
        let st0 = get_status(gpu, &info);
        let cur0 = st0.freq_current_mhz(0, idx).unwrap_or(0);
        let effect0 = cur0 as i64 - def_stock as i64;
        eprintln!("  cur_after = {cur0}, effect = {effect0} MHz");
        dump_curve(&st0, "Mode 0 absolute", idx);

        // Restore
        set_abs(gpu, idx, 0, &info);

        // --- Comparison ---
        eprintln!("\n--- Comparison ---");
        eprintln!("  mode1 effect = {effect1}, mode0 effect = {effect0}");
        if effect1 == effect0 {
            eprintln!("  MATCH! mode1 delta={delta} ≡ mode0 kHz={mode0_khz}");
        } else {
            eprintln!("  MISMATCH: mode1={effect1} vs mode0={effect0}");
            // Try to find the actual mode0 equivalent
            let ratio = if effect0 != 0 { effect1 as f64 / effect0 as f64 } else { 0.0 };
            eprintln!("  ratio mode1/mode0 = {ratio:.4}");
        }
    }

    eprintln!("\nDone.");
}
