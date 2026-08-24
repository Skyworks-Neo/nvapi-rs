//! THE definitive probe for the private ClockClient V/F-points family
//! (replaces the probe_vfp2..9 / probe_v2_* / probe_vfpoints drafts — the
//! RE chain they documented is summarized here and in
//! reverse/melonvolt/PLAN_clk_domains.md; never commit that path).
//!
//! Read-only chain, live-verified on an RTX 4060 Laptop / driver R610.74:
//!
//!   GetInfo 0x8895B510 (magic 0x78604, 493060B)
//!     → per-bank 2048-bit point masks + 104B descriptors
//!   seed GetStatus's +4..+132 header from GetInfo's mask output
//!     (zero seed → no records; garbage → -1)
//!   GetStatus 0x7FEE9032 (magic 2000388, 0x1E8604 — NOT sizeof|ver<<16)
//!     → 488B type-8 records: voltage µV @+0x58 (grid axis, mirrored
//!       +0x68), default MHz @+0x24, current MHz @+0x64 (= default +
//!       applied offset); type-7 records = pstate bins. Calibrated 1:1
//!       against the public GPC VFP curve.
//!   GetControl 0xDA025C3E (magic 4670980, 4343300B, 1060B records)
//!     → the OVERRIDE table (mode@+36, value@+56, flag@+96) — the RMW
//!       snapshot source for SetControl 0xFEC00D04.
use nvapi::initialize;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use nvapi::sys::api::NvAPI_EnumPhysicalGPUs;
use nvapi::sys::gpu::clock::private::*;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use std::ptr;

fn main() {
    let _ = initialize();
    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    let gpu = handles[0];

    use nvapi::sys::api::{
        NvAPI_GPU_ClockClkVfPointsGetControl, NvAPI_GPU_ClockClkVfPointsGetInfo,
        NvAPI_GPU_ClockClkVfPointsGetStatus,
    };

    // 1. GetInfo — the point census + the GetStatus header seed
    let mut info = Box::new(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1::default());
    let st = unsafe { NvAPI_GPU_ClockClkVfPointsGetInfo(gpu, ptr::from_mut(&mut *info).cast()) };
    println!("GetInfo  st={st} (magic 0x{:X})", info.version.data);
    assert_eq!(st, 0);

    // 2. GetStatus — full V/F table, header seeded
    let mut status = Box::new(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1::default());
    info.seed_status_header(&mut status);
    let st =
        unsafe { NvAPI_GPU_ClockClkVfPointsGetStatus(gpu, ptr::from_mut(&mut *status).cast()) };
    println!("GetStatus st={st} (magic 0x{:X})", status.version.data);
    assert_eq!(st, 0);

    // 3. decode: per-bank first/last present records
    for bank in 0..2 {
        let mut first: Option<usize> = None;
        let mut last: Option<usize> = None;
        for idx in 0..clk_vfp_info::POINTS {
            if info.point_present(bank, idx) == Some(true) {
                first.get_or_insert(idx);
                last = Some(idx);
            }
        }
        if let (Some(lo), Some(hi)) = (first, last) {
            for idx in [lo, hi] {
                println!(
                    "bank{bank} pt[{idx}] V={:?}µV def={:?}MHz cur={:?}MHz type={:?}",
                    status.voltage_uv(bank, idx),
                    status.freq_default_mhz(bank, idx),
                    status.freq_current_mhz(bank, idx),
                    status.record_type(bank, idx),
                );
            }
        }
    }

    // 4. GetControl — the override table (all-zero at stock; this is the
    //    surface SetControl 0xFEC00D04 writes: mode 0=abs / 1=delta, value)
    let mut ctrl = Box::new(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE_V1::default());
    ctrl.seed_masks_from_info(&info);
    let st = unsafe { NvAPI_GPU_ClockClkVfPointsGetControl(gpu, ptr::from_mut(&mut *ctrl).cast()) };
    println!("GetControl st={st} (magic 0x{:X})", ctrl.version.data);
    if st == 0 {
        let mut nonzero = 0;
        for bank in 0..2 {
            for idx in 0..clk_vfp_control::POINTS {
                if info.point_present(bank, idx) != Some(true) {
                    continue;
                }
                if ctrl.value(bank, idx).unwrap_or(0) != 0 {
                    nonzero += 1;
                }
            }
        }
        println!("override table: {nonzero} non-zero values (0 = stock)");
    }
}
