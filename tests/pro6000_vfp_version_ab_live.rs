//! VF SetControl version A/B: the 580.173.02 shim's SetControl handler
//! (sub_38B80) treats ver1 (0x12420) as VERBATIM and ver2 (0x22420) as
//! SCALED: landed = value * (100 / descriptor_divisor) — integer division.
//! If this SKU's divisor > 100 the landed value is ZERO: the call reports
//! success (rc echoes the INPUT), nothing lands, mode0/1 look dead.
//! nvapi-rs sends ver2 by default, so this A/B writes bank0 point40 with
//! each version, reads back what actually LANDED, and derives the divisor.
//!
//!   sudo target/debug/deps/pro6000_vfp_version_ab_live-<hash> --nocapture --ignored
//!
//! Everything restored via reset_vfp_private. READ the landed values, don't
//! trust the rc.

use std::ptr;

use nvapi::PhysicalGpu;
use nvapi::sys::api::{
    NvAPI_GPU_ClockClkVfPointsGetControl, NvAPI_GPU_ClockClkVfPointsGetInfo,
    NvAPI_GPU_ClockClkVfPointsSetControl,
};
use nvapi::sys::gpu::clock::undocumented::{
    NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL, NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO,
};
use nvapi::sys::nvapi::NvVersion;

const POINT: usize = 40;
const VALUE: i32 = 50_000;

fn get_mask(gpu: &PhysicalGpu) -> u32 {
    unsafe {
        let mut info = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO>::new_zeroed();
        let mut info = info.assume_init();
        info.version = NvVersion::with_version(
            nvapi::sys::gpu::clock::undocumented::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_V1::MAGIC,
        );
        let st = NvAPI_GPU_ClockClkVfPointsGetInfo(*gpu.handle(), ptr::from_mut(&mut *info).cast());
        assert_eq!(st, 0, "GetInfo rc={st}");
        info.mask.mask
    }
}

/// SET with an explicit version stamp; returns the GET-readback of the
/// point's freqDeltaKHz (what actually LANDED).
fn set_and_readback(gpu: &PhysicalGpu, mask: u32, version_magic: u32, tag: &str) -> Option<i32> {
    unsafe {
        let mut ctl = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL>::new_zeroed();
        let mut ctl = ctl.assume_init();
        ctl.version = NvVersion::with_version(version_magic);
        ctl.mask = mask;
        ctl.points[POINT].freqDeltaKHz = VALUE;
        let st = NvAPI_GPU_ClockClkVfPointsSetControl(
            *gpu.handle(),
            ptr::from_ref(&*ctl).cast_const().cast(),
        );
        println!("  {tag}: SetControl rc={st}");
        if st != 0 {
            return None;
        }
        // readback
        let mut rd = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL>::new_zeroed();
        let mut rd = rd.assume_init();
        rd.version = NvVersion::with_version(version_magic);
        rd.mask = mask;
        let st =
            NvAPI_GPU_ClockClkVfPointsGetControl(*gpu.handle(), ptr::from_mut(&mut *rd).cast());
        assert_eq!(st, 0, "GetControl rc={st}");
        Some(rd.points[POINT].freqDeltaKHz)
    }
}

#[test]
#[ignore = "live GPU WRITE probe — root required; restores via reset_vfp_private"]
fn vfp_version_ab() {
    let uid = std::process::Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    assert_eq!(uid, 0, "run under sudo");

    nvapi::initialize().expect("init");
    let gpu = &PhysicalGpu::enumerate().expect("enumerate")[0];
    println!("gpu: {}", gpu.full_name().unwrap_or_default());
    let mask = get_mask(gpu);
    println!("mask = {mask:#x}");

    let ver2 = 0x22420; // scaled path (shim: landed = v * (100/div))
    let ver1 = 0x12420; // verbatim path

    println!("\n== ver2 (0x22420, scaled — nvapi-rs default) ==");
    if let Some(landed) = set_and_readback(gpu, mask, ver2, "ver2") {
        println!("  input {VALUE} -> landed {landed}");
        if landed == 0 {
            println!("  -> LANDED ZERO: divisor > 100, the scale crushed the write");
        } else if landed != VALUE {
            let div = (VALUE * 100) / landed;
            println!("  -> implied descriptor divisor ≈ {div}");
        } else {
            println!("  -> verbatim (divisor 100)");
        }
    }
    let n = gpu.reset_vfp_private(0, None).expect("reset");
    println!("  restore: {n} points reset");

    println!("\n== ver1 (0x12420, verbatim) ==");
    if let Some(landed) = set_and_readback(gpu, mask, ver1, "ver1") {
        println!("  input {VALUE} -> landed {landed}");
        if landed == VALUE {
            println!("  -> verbatim confirmed on the record plane");
        } else {
            println!("  -> ver1 also transformed (unexpected — paste me)");
        }
    }
    let n = gpu.reset_vfp_private(0, None).expect("reset");
    println!("  restore: {n} points reset");

    println!("\nVERDICT GUIDE:");
    println!("  ver2 landed 0 (divisor > 100)  -> THIS is the mode0/1 no-effect mechanism:");
    println!("    nvoc must send ver1 (verbatim) or divide by the descriptor first");
    println!("  ver2 landed scaled value       -> note the divisor; raw writes must pre-divide");
    println!("  both landed correctly but clocks frozen -> scaling fine, detachment confirmed");
}
