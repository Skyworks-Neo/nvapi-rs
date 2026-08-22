//! Pascal (10-series) BATCH mode-1 calibration — public voltage lock +
//! classic clock read. Confirmed working on a P100:
//!
//!   * voltage lock via set_vfp_locks(Voltage) pins the operating point
//!     EXACTLY on the curve (baseline live == def_pub, offset +0.0);
//!     P0 pstate pin is NOT needed (PrivateLifecycleInit even fails with
//!     NvidiaDeviceNotFound there) — removed.
//!   * live GPC clock from classic GetAllClockFrequencies[Graphics]
//!     (GetAllClocks V2 has no Gpc key on Pascal; ClockClient MEASURE is
//!     NotSupported; private STATUS cur is empty on type-1 records).
//!   * the classic clock has ±0.5 MHz noise on a 12.5 MHz grid — samples
//!     are taken in HALF-MHz integer units, the grid is auto-detected as
//!     the minimum positive gap, E is snapped to it, then the exact
//!     staircase fit runs (result C halved back to MHz).
//!
//! Usage: cargo run --release --example probe_vfp_diff -- [idx_lo] [idx_hi] [pt_step] [dmax] [d_step]
//!   defaults: 0 79 4 600 50   (10-series public/private tables span 0..79)
//! Run as admin. Every point: lock → ladder → restore (mode-0 0) → unlock.
use nvapi::initialize;
use nvapi::sys::api::{
    NvAPI_EnumPhysicalGPUs, NvAPI_GPU_ClockClkVfPointsGetControl,
    NvAPI_GPU_ClockClkVfPointsGetInfo, NvAPI_GPU_ClockClkVfPointsSetControl,
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
    let idx_lo: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let idx_hi: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(79);
    let pt_step: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(4).max(1);
    let dmax: i64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(600);
    let d_step: i64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(50);

    let _ = initialize();
    // GPU 0 throughout (multi-GPU: lock/measure/write all target handles[0])
    let mut h = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut n = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut h, &mut n) };
    let gpu = h[0];
    let pgpu = nvapi::PhysicalGpu::enumerate()
        .ok()
        .and_then(|g| g.into_iter().next())
        .expect("no PhysicalGpu");

    // public VFP curve (voltage source; `configured()` — default is EMPTY
    // on Pascal, and get_voltage_by_point/set-vfp-voltage-lock use this)
    let curve = pgpu
        .vfp_info()
        .and_then(|vi| pgpu.vfp_curve(&vi))
        .expect("public vfp_curve failed");
    let entries = curve
        .points
        .get(&nvapi::ClockDomain::Graphics)
        .expect("public VFP has no Graphics domain");

    let info = get_info(gpu);
    let live = |pgpu: &nvapi::PhysicalGpu| -> Option<f64> {
        if let Some(map) = pgpu.all_clocks().ok() {
            if let Some(khz) = map.get(&nvapi::ClockDomainId::Gpc) {
                return Some(khz.0 as f64 / 1000.0);
            }
        }
        pgpu.clock_frequencies(nvapi::sys::gpu::clock::ClockFrequencyType::Current)
            .ok()?
            .get(&nvapi::ClockDomain::Graphics)
            .map(|khz| khz.0 as f64 / 1000.0)
    };

    let volt_lock = |uv: Option<u32>| {
        pgpu.set_vfp_locks([nvapi::ClockLockEntry {
            limit: nvapi::PerfLimitId::Voltage,
            lock_value: uv.map(|v| nvapi::ClockLockValue::Voltage(nvapi::Microvolts(v))),
            clock: nvapi::ClockDomain::Graphics,
        }])
    };

    let ladder: Vec<i64> = (0..=dmax).step_by(d_step as usize).collect();
    println!("=== Pascal batch locked-voltage calibration: idx {idx_lo}..{idx_hi} every {pt_step}, ladder 0..={dmax} step {d_step} ===");
    println!("idx,volt_mV,def_mHz,C,C_lo,C_hi,D0,prior_C,dev");

    let mut row = 0usize;
    for idx in idx_lo..=idx_hi {
        let Some((_, e)) = entries.iter().find(|(i, _)| *i == idx) else { continue };
        if info.point_present(0, idx) != Some(true) { continue; }
        row += 1;
        if (row - 1) % pt_step != 0 { continue; }

        let p = e.configured();
        let volt_uv = p.voltage.0 as u32;
        let def_pub = p.frequency.0 as f64 / 1000.0;
        if volt_uv == 0 || def_pub == 0.0 {
            println!("{idx},,,,MISSING public point");
            continue;
        }

        // lock → baseline (offset must be small: the lock pins the op point)
        if let Err(err) = volt_lock(Some(volt_uv)) {
            println!("{idx},{},{},LOCK-FAILED {err:?}", volt_uv / 1000, def_pub as i64);
            continue;
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
        let Some(live0) = live(&pgpu) else {
            let _ = volt_lock(None);
            println!("{idx},,,,NO-LIVE");
            continue;
        };
        if (live0 - def_pub).abs() > 100.0 {
            println!("{idx},{},{},UNPINNED off={:+.0}", volt_uv / 1000, def_pub as i64,
                live0 - def_pub);
            let _ = volt_lock(None);
            continue;
        }

        // ladder — samples in HALF-MHz integer units (classic clock has
        // ±0.5 MHz noise; the half-unit keeps the grid integral)
        let mut samples: Vec<(i64, i64)> = Vec::new();
        for &d in &ladder {
            write_point(gpu, idx, false, d as u32, &info);
            std::thread::sleep(std::time::Duration::from_millis(150));
            let e2 = ((live(&pgpu).unwrap_or(live0) - live0) * 2.0).round() as i64;
            samples.push((d, e2));
        }
        write_point(gpu, idx, true, 0, &info); // restore
        let _ = volt_lock(None);

        // trim clamped flats at both ends, then require >=3 levels
        while samples.len() > 2 && samples[0].1 == samples[1].1 { samples.remove(0); }
        while samples.len() > 2
            && samples[samples.len() - 1].1 == samples[samples.len() - 2].1
        {
            samples.pop();
        }
        let mut levels: Vec<i64> = Vec::new();
        for &(_, e) in &samples {
            if levels.last() != Some(&e) { levels.push(e); }
        }
        if levels.len() < 3 {
            println!("{idx},{},{},PINNED flat={}", volt_uv / 1000, def_pub as i64,
                samples.first().map(|&(_, e)| e).unwrap_or(0));
            continue;
        }
        // saturation guard: near-ceiling points cap early and the trimmed
        // remnant fits a FAKE small C (observed: def 1987 -> "0.25")
        let span = *levels.last().unwrap() - *levels.first().unwrap();
        if span < 4 * 25 {
            println!("{idx},{},{},SATURATED span={:.0}MHz", volt_uv / 1000, def_pub as i64,
                span as f64 / 2.0);
            continue;
        }

        // grid selection: classic-clock noise (±1 half-MHz) breaks a GCD
        // (gaps mix 25/26/51 -> gcd 1); instead pick the candidate grid
        // minimizing total snap error (candidates = 5..25 MHz)
        let mut q2 = 25i64;
        let mut best_err = i64::MAX;
        for &g in &[10i64, 15, 20, 25, 30, 50] {
            let err: i64 = samples
                .iter()
                .map(|&(_, e)| {
                    let snapped = ((e as f64) / g as f64).round() as i64 * g;
                    (e - snapped).abs()
                })
                .sum();
            if err < best_err {
                best_err = err;
                q2 = g;
            }
        }
        for s in &mut samples { s.1 = ((s.1 as f64) / q2 as f64).round() as i64 * q2; }

        match nvapi::clk_vf_stair_fit(&samples, q2) {
            // C came out in half-MHz effect units — halve back
            Some(f) => {
                let c = f.c / 2.0;
                let lo = f.c_lo / 2.0;
                let hi = f.c_hi / 2.0;
                let d0 = f.d0; // invariant under uniform E scaling
                let prior = nvapi::clk_vf_g_prior(def_pub as u32)
                    .map(|(pc, _)| pc)
                    .unwrap_or(f64::NAN);
                println!("{idx},{},{},{:.4},{:.4},{:.4},{:.0},{:.4},{:+.4}",
                    volt_uv / 1000, def_pub as i64, c, lo, hi, d0, prior, c - prior);
            }
            None => println!("{idx},{},{},FIT-FAILED n={}", volt_uv / 1000, def_pub as i64,
                samples.len()),
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(200));
    println!("\nfinal live GPC = {:?}", live(&pgpu));
}
