//! Pascal (10-series) mode-1 effect probe. The STATUS current-frequency
//! field is empty on type-1 records (probe_c_stair reports CUR-ABSENT for
//! every point), but the mode-1 WRITE itself works (P100-verified). This
//! probe measures the effect two ways per ladder step:
//!
//!   1. RAW RECORD DIFF — dump every dword of the point's 488-byte STATUS
//!      record before/after each write. Whichever offset moves IS the
//!      applied-frequency field on Pascal (Ada+ layout: default +0x24,
//!      voltage +0x58 (mirror +0x68), current +0x64 — Pascal may differ).
//!      If NO dword moves and the live clock is flat, the write is inert.
//!   2. LIVE MEASURE_FREQ — the middle layer's two-sample V1/V2 measure of
//!      the GPC domain clock, showing whether the running clock responds.
//!
//! A 150 ms settle delay follows each write (driver-update lag was
//! observed on Turing: every-other-point flat E=120 responses).
//!
//! Usage: cargo run --release --example probe_vfp_diff -- [idx] [d1] [d2]
//!   defaults: idx = first present point, d1 = 100, d2 = 400
//! Run as admin (SetControl writes; each step is restored via mode-0 0).
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

fn record_bytes(s: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1, idx: usize)
    -> &[u8] {
    let bytes = unsafe {
        std::slice::from_raw_parts(
            (s as *const NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1).cast::<u8>(),
            std::mem::size_of_val(s),
        )
    };
    let base = clk_vfp_status::REC1 + clk_vfp_status::STRIDE * idx;
    &bytes[base..base + clk_vfp_status::STRIDE]
}

fn diff_record(base: &[u8], cur: &[u8], label: &str) {
    let mut changed = 0;
    for k in 0..base.len() / 4 {
        let off = k * 4;
        let a = u32::from_le_bytes(base[off..off + 4].try_into().unwrap());
        let b = u32::from_le_bytes(cur[off..off + 4].try_into().unwrap());
        if a != b {
            println!("    +0x{off:03X}: {a:#10x} -> {b:#10x}");
            changed += 1;
        }
    }
    println!("    [{label}] {changed} dword(s) changed", );
}

fn main() {
    let mut args = std::env::args().skip(1);
    let idx_arg: Option<usize> = args.next().and_then(|s| s.parse().ok());
    let d1: i64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(100);
    let d2: i64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(400);

    let _ = initialize();
    let mut h = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut n = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut h, &mut n) };
    let gpu = h[0];
    // hi-level handle for the two-sample MEASURE_FREQ
    let pgpu = nvapi::PhysicalGpu::enumerate()
        .ok()
        .and_then(|g| g.into_iter().next())
        .expect("no PhysicalGpu for MEASURE_FREQ");

    let info = get_info(gpu);
    let idx = idx_arg.unwrap_or_else(|| {
        (0..clk_vfp_info::POINTS)
            .find(|&i| info.point_present(0, i) == Some(true))
            .unwrap_or(0)
    });
    assert!(info.point_present(0, idx) == Some(true), "idx {idx} not present");

    let baseline = get_status(gpu, &info);
    let typ = baseline.record_type(0, idx).unwrap_or(0);
    let div: i64 = if typ == 1 { 2 } else { 1 };
    println!("=== V/F record diff probe: idx={idx} type={typ} V={:?}mV def={:?} cur={:?} (div={div}) ===",
        baseline.voltage_uv(0, idx).map(|v| v / 1000),
        baseline.freq_default_mhz(0, idx).map(|f| f / div as u32),
        baseline.freq_current_mhz(0, idx).map(|f| f / div as u32),
    );
    let base_rec = record_bytes(&baseline, idx).to_vec();

    let live = |label: &str| match pgpu.clk_domain_freq(0) {
        Ok(f) => println!("    {label}: GPC live = {:.1} MHz", f.freq_mhz),
        Err(e) => println!("    {label}: GPC live measure failed: {e:?}"),
    };

    live("baseline");
    // baseline non-zero dwords of the record (context for the diff)
    println!("  baseline record non-zero dwords:");
    for k in 0..base_rec.len() / 4 {
        let off = k * 4;
        let v = u32::from_le_bytes(base_rec[off..off + 4].try_into().unwrap());
        if v != 0 {
            println!("    +0x{off:03X} = {v:#10x} ({v})");
        }
    }

    for &d in &[d1, d2] {
        println!("\n-- write mode-1 delta={d} at idx {idx} --");
        write_point(gpu, idx, false, d as u32, &info);
        std::thread::sleep(std::time::Duration::from_millis(150));
        let s = get_status(gpu, &info);
        diff_record(&base_rec, record_bytes(&s, idx), &format!("delta={d}") );
        live(&format!("after delta={d}"));
    }

    println!("\n-- restore (mode-0 value 0) --");
    write_point(gpu, idx, true, 0, &info);
    std::thread::sleep(std::time::Duration::from_millis(150));
    let s = get_status(gpu, &info);
    diff_record(&base_rec, record_bytes(&s, idx), "restored");
    live("after restore");
}
