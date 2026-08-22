//! Staircase-regression C/D0 calibration — fixes the 2-point rounding problem.
//!
//! Problem with 2-point (d1,d2) sampling: effects quantize to the curve step
//! Q (15 MHz on CMP 170HX), so the slope estimate lands only on multiples of
//! Q/(d2-d1) (= 0.075 with 200/400) and flips with d1/d2 choice.
//!
//! Model per point: E(d) = Q * floor( (C*d - B) / Q ), B = C*D0, clamped at 0.
//! Each ladder sample (d_i, E_i) with E_i > 0 yields exact linear constraints
//!   E_i <= C*d_i - B < E_i + Q
//! Feasible (C, B) region = intersection; we grid-scan C and keep the widest
//! feasible interval, reporting its midpoint plus [C_lo, C_hi] uncertainty.
//! Q is auto-detected as the GCD of nonzero effects (fallback 15). Samples
//! past forward-flattening saturation (E stops rising) are excluded.
//!
//! Negative deltas are included (safe: they only lower the point; points
//! that cannot drop show a floor-clamped flat which is trimmed). Negative
//! effects join the fit as constraints — the downward half of the staircase
//! pins C/D0 far tighter than a one-sided positive ladder. A negative fit
//! D0 just means the point responds immediately (crossing left of d=0).
//!
//! Usage: cargo run --release --example probe_c_stair -- [pt_step] [d_step] [dmax] [dmin] [idx_lo] [idx_hi]
//!   pt_step: sample every Nth present point (default 16)
//!   d_step / dmax / dmin: delta ladder dmin..=dmax step d_step
//!   (defaults 50 600 -600)
//!   idx_lo..idx_hi: curve-point index range, INCLUSIVE (default 0 127).
//!   One DOMAIN per run — the 2048-point space is segmented (GPC 0-127,
//!   XBAR 128-255, HOST 256+ ...); mixing domains into one C table is
//!   meaningless since each domain has its own scaling.
//! Run as admin; every point restored (mode-0 value 0) after its ladder.
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

/// Exact staircase fit: samples (d, E), saturation-trimmed. Constraint form
/// with B = C*D0:
///   E_i <= C*d_i - B < E_i + Q
/// Pairwise subtraction eliminates B and gives EXACT rational bounds on C:
///   C*(d_i - d_j) in (E_i - E_j - Q, E_i - E_j + Q)
/// C_lo/C_hi = tightest pairwise bounds (no grid quantization). Point
/// estimate = interval midpoint, snapped to the nearest k/400 multiple when
/// that lies inside the interval — every true C observed on CMP 170HX and
/// RTX 4060 is k/400 (0.3, 0.3375, 0.625, 0.28, 0.4525, ...), strongly
/// suggesting RM stores C as a /400 fixed-point.
fn stair_fit(pts: &[(i64, i64)], q: f64) -> Option<(f64, f64, f64, f64)> {
    if pts.len() < 2 { return None; }
    let mut lo = 0.0f64;
    let mut hi = f64::INFINITY;
    for i in 0..pts.len() {
        for j in 0..pts.len() {
            if i == j { continue; }
            let (di, ei) = pts[i];
            let (dj, ej) = pts[j];
            let dd = (di - dj) as f64;
            let de = (ei - ej) as f64;
            if dd > 0.0 {
                lo = lo.max((de - q) / dd);
                hi = hi.min((de + q) / dd);
            } else if dd < 0.0 {
                lo = lo.max((de + q) / dd);
                hi = hi.min((de - q) / dd);
            }
        }
    }
    if !(lo < hi) { return None; }
    let mut c = (lo + hi) / 2.0;
    let snapped = (c * 400.0).round() / 400.0;
    if snapped >= lo && snapped <= hi { c = snapped; }
    let mut b_hi = f64::MAX;
    let mut b_lo = f64::MIN;
    for &(d, e) in pts {
        let x = c * d as f64 - e as f64;
        b_hi = b_hi.min(x);
        b_lo = b_lo.max(x - q);
    }
    if !(b_lo < b_hi) { return None; }
    let b = (b_lo + b_hi) / 2.0;
    Some((c, lo, hi, b / c))
}

fn main() {
    let mut args = std::env::args().skip(1);
    let pt_step: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(16).max(1);
    let d_step: i64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(50).max(10);
    let dmax: i64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(600);
    let dmin: i64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(-dmax).min(0);
    let idx_lo: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let idx_hi: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(127).max(idx_lo);

    let _ = initialize();
    let mut h = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut n = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut h, &mut n) };
    let gpu = h[0];

    let info = get_info(gpu);
    let baseline = get_status(gpu, &info);

    let ladder: Vec<i64> = (dmin..=dmax).step_by(d_step as usize).collect();
    let present: Vec<usize> = (idx_lo..=idx_hi.min(clk_vfp_info::POINTS - 1))
        .filter(|&i| info.point_present(0, i) == Some(true))
        .collect();
    eprintln!("=== staircase C fit: idx {idx_lo}..{idx_hi} ({} present), every {pt_step}, ladder {:?} ===",
        present.len(), &ladder[..ladder.len().min(6)]);
    eprintln!("idx,volt_mV,def_mHz,Q,n_used,C,C_lo,C_hi,D0,E_range");

    let mut results: Vec<(f64, f64)> = Vec::new(); // (volt_mV, C)
    let mut row = 0usize;
    while row < present.len() {
        let idx = present[row];
        row += pt_step;
        // Pascal-generation decode: type-1 records report frequency terms
        // DOUBLED (live-observed on a 10-series) — mirror the middle layer's
        // ÷2. Type 8/13/18 (Ada+) are plain MHz.
        let typ = baseline.record_type(0, idx).unwrap_or(0);
        let div: i64 = if typ == 1 { 2 } else { 1 };
        let volt = baseline.voltage_uv(0, idx).unwrap_or(0);
        let def = baseline.freq_default_mhz(0, idx).unwrap_or(0) as i64 / div;
        // Some Pascal points lack current/voltage fields entirely (cur=0) —
        // skip with diagnostics instead of feeding E = -def into the fit
        let base_cur = baseline.freq_current_mhz(0, idx).unwrap_or(0) as i64 / div;
        if base_cur == 0 || def == 0 {
            eprintln!("{idx},{},{},CUR-ABSENT t={typ} def={def} cur0={base_cur} volt={}",
                volt / 1000, def, baseline.voltage_uv(0, idx).unwrap_or(0));
            continue;
        }

        // walk the ladder ascending (negatives first); collect (d, E)
        let mut samples: Vec<(i64, i64)> = Vec::new();
        let mut first_e: Option<i64> = None;
        let mut flat_from_start = 0usize;
        for &d in &ladder {
            write_point(gpu, idx, false, d as u32, &info);
            let cur = get_status(gpu, &info).freq_current_mhz(0, idx).unwrap_or(0) as i64 / div;
            let e = cur - def; // may be negative — allowed in the fit
            match first_e {
                None => { first_e = Some(e); flat_from_start = 1; }
                Some(fe) if e == fe => flat_from_start += 1,
                _ => flat_from_start = 0,
            }
            // early-exit ONLY on the positive side: a flat leading run is
            // normal with a negative ladder (most points clamp to -15/0
            // downward within one grid step) — break only if still flat at
            // d >= +200 (genuinely dead point)
            if flat_from_start >= 5 && d >= 200 { break; }
            samples.push((d, e));
        }
        write_point(gpu, idx, true, 0, &info); // restore

        // trim floor/flatten-clamped flats at both ends (E stops changing)
        while samples.len() > 2 && samples[0].1 == samples[1].1 { samples.remove(0); }
        while samples.len() > 2 && samples[samples.len() - 1].1 == samples[samples.len() - 2].1 {
            samples.pop();
        }

        if samples.len() < 2 {
            eprintln!("{idx},{},{},CLAMPED (flat across ladder)", volt / 1000, def);
            continue;
        }
        // require a real staircase: >=3 distinct effect levels (top-of-curve
        // points pinned at the P0 ceiling respond flat-zero — real info, but
        // not a C; a 2-sample zero fit is pure noise)
        let mut levels: Vec<i64> = Vec::new();
        for &(_, e) in &samples {
            if levels.last() != Some(&e) { levels.push(e); }
        }
        if levels.len() < 3 {
            eprintln!("{idx},{},{},NO-RESPONSE (flat E={}),pinned/clamped",
                volt / 1000, def, samples.first().map(|&(_, e)| e).unwrap_or(0));
            continue;
        }
        // Q = GCD of nonzero |effects|
        let mut q_gcd = 0i64;
        for &(_, e) in &samples { if e != 0 { q_gcd = gcd(q_gcd, e.abs()); } }
        let q = if q_gcd > 0 { q_gcd as f64 } else { 15.0 };

        match stair_fit(&samples, q) {
            Some((c, lo, hi, d0)) => {
                results.push((volt as f64 / 1000.0, c));
                let (emin, emax) = samples.iter().fold((i64::MAX, i64::MIN),
                    |(a, b), &(_, e)| (a.min(e), b.max(e)));
                eprintln!("{idx},{},{},{},{},{:.4},{:.4},{:.4},{:.0},E[{emin},{emax}]",
                    volt / 1000, def, q, samples.len(), c, lo, hi, d0);
            }
            None => {
                eprintln!("{idx},{},{},{},{},FIT-FAILED (inconsistent staircase)",
                    volt / 1000, def, q, samples.len());
            }
        }
    }

    if results.is_empty() { return; }
    let mut vals: Vec<f64> = results.iter().map(|&(_, c)| c).collect();
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    eprintln!("\nC stats: n={} median={:.4} min={:.4} max={:.4}",
        vals.len(), vals[vals.len() / 2], vals[0], *vals.last().unwrap());
    eprintln!("volt_mV -> C (read top-to-bottom: constant == global C; monotone == C(V))");
    for &(v, c) in &results {
        eprintln!("  {v:5.0} -> {c:.4}");
    }
}
