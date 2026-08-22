//! Pascal (10-series) mode-1 calibration via PUBLIC voltage lock + clock
//! read. On Pascal the modern read paths are unusable for effects (STATUS
//! current field empty on type-1 records; ClockClient MEASURE_FREQ returns
//! NotSupported) — but:
//!   * the mode-1 private WRITE works (P100-verified),
//!   * the public PerfClientLimits lock (`set_vfp_locks`,
//!     PerfLimitId::Voltage + ClockLockMode::ManualVoltage) can pin the
//!     operating voltage to a point inside the P0 segment (voltage locks
//!     only hold within P0 — the driver refuses outside; P0 is enough),
//!   * GetAllClocks V2 (public, Pascal-native) reads the live GPC clock.
//!
//! With the voltage pinned at point idx's V, the live GPC clock IS that
//! point's effective curve value: write mode-1 deltas along a ladder, read
//! the live clock, effect = live − baseline, fit C/D0 with the exact
//! staircase fit (`nvapi::clk_vf_stair_fit`).
//!
//! Usage: cargo run --release --example probe_vfp_diff -- [idx] [dmax] [d_step]
//!   defaults: idx = a mid-P0 present point, dmax 400, d_step 50
//! Run as admin. Point restored (mode-0 0) and lock released on exit.
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

fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 { let t = a % b; a = b; b = t; }
    a
}

fn main() {
    let mut args = std::env::args().skip(1);
    let idx_arg: Option<usize> = args.next().and_then(|s| s.parse().ok());
    let dmax: i64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(400);
    let d_step: i64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(50);

    let _ = initialize();
    let mut h = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut n = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut h, &mut n) };
    let gpu = h[0];
    let pgpu = nvapi::PhysicalGpu::enumerate()
        .ok()
        .and_then(|g| g.into_iter().next())
        .expect("no PhysicalGpu");

    let info = get_info(gpu);
    let idx = idx_arg.unwrap_or_else(|| {
        // mid-P0 default: the present point closest to the middle of bank 0
        let present: Vec<usize> = (0..clk_vfp_info::POINTS)
            .filter(|&i| info.point_present(0, i) == Some(true))
            .collect();
        present.get(present.len() / 2).copied().unwrap_or(0)
    });
    assert!(info.point_present(0, idx) == Some(true), "idx {idx} not present");

    let baseline = get_status(gpu, &info);
    let typ = baseline.record_type(0, idx).unwrap_or(0);
    let div: u32 = if typ == 1 { 2 } else { 1 };
    let volt_uv = baseline.voltage_uv(0, idx).unwrap_or(0);
    let def = baseline.freq_default_mhz(0, idx).unwrap_or(0) / div;
    println!("=== Pascal locked-voltage mode-1 calibration ===");
    println!("idx={idx} type={typ} V={}mV def={}MHz (Pascal div {div})",
        volt_uv / 1000, def);

    // live GPC clock via public GetAllClocks V2 (MEASURE NotSupported here)
    let live = |pgpu: &nvapi::PhysicalGpu| -> Option<f64> {
        pgpu.all_clocks().ok()?.get(&nvapi::ClockDomainId::Gpc).map(|khz| khz.0 as f64 / 1000.0)
    };

    // 1. lock the operating voltage at this point (public PerfClientLimits)
    let lock = nvapi::ClockLockEntry {
        limit: nvapi::PerfLimitId::Voltage,
        lock_value: Some(nvapi::ClockLockValue::Voltage(nvapi::Microvolts(volt_uv))),
        clock: nvapi::ClockDomain::Graphics,
    };
    if let Err(e) = pgpu.set_vfp_locks([lock]) {
        println!("voltage lock FAILED ({e:?}) — P0-only; pick a P0-segment idx");
        return;
    }
    println!("voltage locked @ {}mV", volt_uv / 1000);
    std::thread::sleep(std::time::Duration::from_millis(200));
    let Some(live0) = live(&pgpu) else {
        println!("GetAllClocks returned no GPC clock — abort");
        let _ = pgpu.set_vfp_locks([nvapi::ClockLockEntry {
            limit: nvapi::PerfLimitId::Voltage,
            lock_value: None,
            clock: nvapi::ClockDomain::Graphics,
        }]);
        return;
    };
    println!("baseline live GPC = {live0:.1} MHz (def={def} — offset {:.1})",
        live0 - def as f64);

    // 2. ladder: write mode-1 delta → read live clock
    let ladder: Vec<i64> = (0..=dmax).step_by(d_step as usize).collect();
    println!("\ndelta,live_mHz,effect_mHz");
    let mut samples: Vec<nvapi::ClkVfStairSample> = Vec::new();
    for &d in &ladder {
        write_point(gpu, idx, false, d as u32, &info);
        std::thread::sleep(std::time::Duration::from_millis(150));
        let l = live(&pgpu).unwrap_or(f64::NAN);
        let e = l - live0;
        println!("{d},{:.1},{:.1}", l, e);
        samples.push((d, e.round() as i64));
    }

    // 3. restore + unlock
    write_point(gpu, idx, true, 0, &info);
    let _ = pgpu.set_vfp_locks([nvapi::ClockLockEntry {
        limit: nvapi::PerfLimitId::Voltage,
        lock_value: None,
        clock: nvapi::ClockDomain::Graphics,
    }]);
    std::thread::sleep(std::time::Duration::from_millis(200));
    println!("\nrestored+unlocked; live GPC = {:?}", live(&pgpu));

    // 4. staircase fit (same math as the calibrator)
    let mut q_gcd = 0i64;
    for &(_, e) in &samples { if e != 0 { q_gcd = gcd(q_gcd, e.abs()); } }
    let q = if q_gcd > 0 { q_gcd } else { 15 };
    match nvapi::clk_vf_stair_fit(&samples, q) {
        Some(fit) => {
            println!("\nFIT: C={:.4} [{:.4},{:.4}] D0={:.0} (Q={q}MHz, n={})",
                fit.c, fit.c_lo, fit.c_hi, fit.d0, samples.len());
            if let Some((pc, _)) = nvapi::clk_vf_g_prior(def) {
                println!("prior(def={def}) C={pc:.4} -> deviation {:+.4}",
                    fit.c - pc);
            }
        }
        None => println!("\nFIT FAILED (inconsistent staircase)"),
    }
}
