//! Scan the reverse-volt (mode 1) delta→voltage→frequency mapping.
//! For each voltage point from 450mV to 700mV (step 50mV), write
//! delta=300 (mode 1), read back the effect, then find which voltage's
//! default freq equals (def + effect) to determine the actual voltage shift.
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
    let delta: i16 = std::env::args()
        .nth(1).and_then(|s| s.parse().ok()).unwrap_or(300);

    let _ = initialize();
    let mut h = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut n = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut h, &mut n) };
    let gpu = h[0];

    let info = get_info(gpu);
    let baseline = get_status(gpu, &info);

    // Build voltage→(idx, default_freq) lookup from the baseline curve
    let mut volt_to_freq: Vec<(u32, usize, u32)> = Vec::new();
    for idx in 0..=126usize {
        if info.point_present(0, idx) != Some(true) { continue; }
        let v = baseline.voltage_uv(0, idx).unwrap_or(0);
        let f = baseline.freq_default_mhz(0, idx).unwrap_or(0);
        volt_to_freq.push((v, idx, f));
    }

    // Target voltages: 450, 500, 550, 600, 650, 700 mV
    let target_mvs = [450, 500, 550, 600, 650, 700];

    eprintln!("=== reverse-volt delta scan: delta={delta} ===");
    eprintln!("src_mV,src_idx,src_def,cur_after,effect_mHz,target_freq,lookup_mV,lookup_idx,lookup_def,voltage_shift_mV,delta_per_mV");

    for target_mv in &target_mvs {
        let target_uv = target_mv * 1000;
        // Find the point closest to this voltage
        let best = volt_to_freq.iter()
            .min_by_key(|&&(v, _, _)| (v as i64 - target_uv as i64).abs())
            .copied();
        let (src_volt, src_idx, src_def) = match best { Some(x) => x, None => continue };

        // Write mode 1 delta
        write_point(gpu, src_idx, false, delta as u32, &info);
        let after = get_status(gpu, &info);
        let cur_after = after.freq_current_mhz(0, src_idx).unwrap_or(0);
        let effect = cur_after as i64 - src_def as i64;
        write_point(gpu, src_idx, true, 0, &info); // restore

        // Find which voltage's default freq == cur_after (the lookup target)
        let target_freq = cur_after;
        let lookup = volt_to_freq.iter()
            .find(|&&(_, _, f)| f == target_freq)
            .or_else(|| volt_to_freq.iter()
                .min_by_key(|&&(_, _, f)| (f as i64 - target_freq as i64).abs()))
            .copied();
        let (lookup_volt, lookup_idx, lookup_def) = match lookup { Some(x) => x, None => (0, 0, 0) };
        let voltage_shift = (lookup_volt as i64 - src_volt as i64) / 1000; // mV
        let delta_per_mv = if voltage_shift != 0 {
            delta as f64 / voltage_shift as f64
        } else { 0.0 };

        eprintln!("{},{},{},{},{},{},{},{},{},{},{:.4}",
            target_mv, src_idx, src_def, cur_after, effect,
            target_freq, lookup_volt / 1000, lookup_idx, lookup_def,
            voltage_shift, delta_per_mv);
    }

    // Also try different deltas at a fixed point (800mV = idx 56)
    eprintln!("\n=== Fixed point 800mV (idx 56), varying delta ===");
    eprintln!("delta,effect_mHz,target_freq,lookup_mV,voltage_shift_mV,delta_per_mV");
    for &d in &[100i16, 200, 300, 400, 500, 600, 700] {
        let src_def = baseline.freq_default_mhz(0, 56).unwrap_or(0);
        let src_volt = baseline.voltage_uv(0, 56).unwrap_or(0);

        write_point(gpu, 56, false, d as u32, &info);
        let after = get_status(gpu, &info);
        let cur = after.freq_current_mhz(0, 56).unwrap_or(0);
        write_point(gpu, 56, true, 0, &info);

        let effect = cur as i64 - src_def as i64;
        let lookup = volt_to_freq.iter()
            .find(|&&(_, _, f)| f == cur)
            .or_else(|| volt_to_freq.iter()
                .min_by_key(|&&(_, _, f)| (f as i64 - cur as i64).abs()))
            .copied();
        let (lv, _, _) = match lookup { Some(x) => x, None => (0, 0, 0) };
        let vshift = (lv as i64 - src_volt as i64) / 1000;
        let dpm = if vshift != 0 { d as f64 / vshift as f64 } else { 0.0 };

        eprintln!("{},{},{},{},{},{:.4}", d, effect, cur, lv / 1000, vshift, dpm);
    }

    eprintln!("\nDone.");
}
