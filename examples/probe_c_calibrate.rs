//! Brute-force per-point C calibration for mode-1 (reverse-volt) delta.
//!
//! Empirical model (established via probe_delta_scan on CMP 170HX / A100):
//!   effect(V, delta) = C × (delta - D0(V))
//! — C is the domain slope in MHz per delta unit (~0.3 on the CMP, constant
//! across tested points), D0(V) a per-point deadzone from RM forward-
//! flattening (0 @ 800 mV, 50 @ ~730 mV). The sibling-ID table
//! (probe_scale_factors, 0xCF08E934/0x4F11EAA4) is UNPOPULATED on the CMP
//! (bitmask 0), so empirical calibration is the only route to C/D0.
//!
//! Method per sampled point: write delta=d1 (mode 1) -> read effect1;
//! write delta=d2 -> read effect2; restore (mode-0 value 0). Then
//!   C  = (eff2 - eff1) / (d2 - d1)
//!   D0 = d1 - eff1 / C
//! Effects quantize to the curve step (15 MHz on the CMP), so a wider
//! (d2 - d1) spread reduces the slope's quantization error (15/200 = 0.075
//! with the 200/400 defaults).
//!
//! Usage: cargo run --release --example probe_c_calibrate -- [step] [d1] [d2]
//!   step: sample every Nth PRESENT point (default 8)
//!   d1 d2: the two calibration deltas (default 200 400)
//! Run as admin (SetControl writes); every point is restored immediately.
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
    let mut args = std::env::args().skip(1);
    let step: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(8).max(1);
    let d1: i32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(200);
    let d2: i32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(400);
    if d2 <= d1 {
        eprintln!("d2 ({d2}) must be > d1 ({d1})");
        std::process::exit(2);
    }

    let _ = initialize();
    let mut h = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut n = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut h, &mut n) };
    let gpu = h[0];

    let info = get_info(gpu);
    let baseline = get_status(gpu, &info);

    // collect present points in bank 0, then sample every step-th
    let present: Vec<usize> = (0..clk_vfp_info::POINTS)
        .filter(|&i| info.point_present(0, i) == Some(true))
        .collect();
    eprintln!("=== C calibration: {} present pts, sampling every {step}, d1={d1} d2={d2} ===",
        present.len());
    eprintln!("idx,volt_mV,def_mHz,eff1,eff2,C_mhz_per_delta,D0_est");

    let mut cs: Vec<(f64, f64)> = Vec::new(); // (volt_mV, C)
    let mut row = 0usize;
    while row < present.len() {
        let idx = present[row];
        row += step;

        let volt = baseline.voltage_uv(0, idx).unwrap_or(0);
        let def = baseline.freq_default_mhz(0, idx).unwrap_or(0) as i64;

        write_point(gpu, idx, false, d1 as u32, &info);
        let eff1 = get_status(gpu, &info).freq_current_mhz(0, idx).unwrap_or(0) as i64 - def;
        write_point(gpu, idx, false, d2 as u32, &info);
        let eff2 = get_status(gpu, &info).freq_current_mhz(0, idx).unwrap_or(0) as i64 - def;
        write_point(gpu, idx, true, 0, &info); // restore

        let c = (eff2 - eff1) as f64 / (d2 - d1) as f64;
        let d0 = if c.abs() > 1e-9 {
            d1 as f64 - eff1 as f64 / c
        } else {
            f64::NAN
        };
        if c.abs() > 1e-9 && eff1 > 0 {
            cs.push((volt as f64 / 1000.0, c));
        }
        eprintln!("{idx},{},{},{},{},{:.4},{:.1}", volt / 1000, def, eff1, eff2, c, d0);
    }

    if cs.is_empty() {
        eprintln!("\nno lifted points (all effects 0?) — try higher d1/d2 or check P0 residency");
        return;
    }
    let mut vals: Vec<f64> = cs.iter().map(|&(_, c)| c).collect();
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = vals[vals.len() / 2];
    eprintln!(
        "\nC stats: n={} median={:.4} min={:.4} max={:.4}",
        vals.len(),
        median,
        vals.first().unwrap(),
        vals.last().unwrap()
    );
    eprintln!("(tight spread == constant global C; widening with V == per-voltage C(V))");
    for &(v, c) in &cs {
        if (c - median).abs() > 0.05 {
            eprintln!("  outlier: {v:.0} mV -> C={c:.4}");
        }
    }
}
