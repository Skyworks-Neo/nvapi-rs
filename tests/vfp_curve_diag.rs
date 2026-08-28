// Diagnostic: call PhysicalGpu::vfp_curve directly (no hi-layer allowable_result
// swallowing) and print the raw Result, so we can see whether GetStatus returns
// Ok(empty)/Err(NOT_SUPPORTED)/Err(IncompatibleStructVersion) on legacy HW.
// Run with: cargo test --test vfp_curve_diag -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::{PhysicalGpu};
use nvapi::sys::nvapi::NvVersion;

#[test]
#[ignore]
fn vfp_curve_diag() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    eprintln!("=== {} GPU(s) ===", gpus.len());
    for (i, gpu) in gpus.iter().enumerate() {
        eprintln!("\n--- GPU {} ---", i);
        // V3 (default) path first
        match gpu.vfp_info() {
            Ok(info) => {
                eprintln!("vfp_info: ok mask={:?} domains={:?}", info.mask.mask, info.domains);
                // try each stamp variant explicitly by repeating — vfp_curve
                // internally does V3 then falls back to the V1 stamp configured
                // in gpu.rs (currently 0x11C28 ver1).
                match gpu.vfp_curve(&info) {
                    Ok(c) => eprintln!("vfp_curve: Ok  points={}", c.points.len()),
                    Err(e) => eprintln!("vfp_curve: Err {:?}", e),
                }
            }
            Err(e) => eprintln!("vfp_info: Err {:?}", e),
        }
    }
}

/// Probe the PRIVATE ClockClient VfPoints family with the R610 large-table
/// stamp AND the 391.35 small-table stamp, to determine whether the private
/// escape path returns data on legacy HW (the public path's escape 0x0700004A
/// is kernel-unimplemented on 391.35). Prints raw status for each stamp.
#[test]
#[ignore]
fn private_vfp_probe() {
    use nvapi::sys::api::NvAPI_GPU_ClockClkVfPointsGetInfo;
    use nvapi::sys::gpu::clock::undocumented::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE;

    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");

    for (label, magic) in [
        ("R610 large 0x78604", 0x78604u32),
        ("391.35 small 0x1481C", 83996u32),
    ] {
        // allocate big buffer (covers both layouts); stamp the variant magic
        let mut info = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE>::new_zeroed();
            let mut b = b.assume_init();
            b.version = NvVersion::with_version(magic);
            b
        };
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetInfo(*gpu.handle(), core::ptr::from_mut(&mut *info).cast())
        };
        eprintln!("private GetInfo {:20}: status={:#x}", label, st as i32);
        if st == 0 {
            // peek first 32 bytes of payload after the version dword
            let bytes = &info.rest[..32];
            eprintln!("  payload[0..32]={:02x?}", bytes);
        }
    }

    // GetStatus with the 391.35 old stamp (0x14C18=85016). Seed the header
    // from a fresh old-stamp GetInfo first (the R610 seed offsets may differ
    // on the small layout — try both seeded and unseeded).
    use nvapi::sys::api::NvAPI_GPU_ClockClkVfPointsGetStatus;
    use nvapi::sys::gpu::clock::undocumented::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE;

    // old-stamp GetInfo to harvest a seed
    let mut info_old = unsafe {
        let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE>::new_zeroed();
        let mut b = b.assume_init();
        b.version = NvVersion::with_version(83996u32);
        b
    };
    let st = unsafe {
        NvAPI_GPU_ClockClkVfPointsGetInfo(*gpu.handle(), core::ptr::from_mut(&mut *info_old).cast())
    };
    eprintln!("seed GetInfo old: status={:#x}", st as i32);

    for (label, magic, seed) in [
        ("R610 large 0x1E8604 seeded", 2000388u32, true),
        ("391.35 old 0x14C18 seeded", 85016u32, true),
        ("391.35 old 0x14C18 unseeded", 85016u32, false),
    ] {
        let mut status = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE>::new_zeroed();
            let mut b = b.assume_init();
            b.version = NvVersion::with_version(magic);
            b
        };
        if seed {
            // copy +4..+132 from info_old (R610 MASK1 region) as the seed
            let src = &info_old.rest[..128];
            status.rest[..128].copy_from_slice(src);
        }
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetStatus(*gpu.handle(), core::ptr::from_mut(&mut *status).cast())
        };
        eprintln!("private GetStatus {:28}: status={:#x}", label, st as i32);
        if st == 0 {
            // scan for first non-zero u32 in the payload
            let nz = status.rest.chunks_exact(4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .enumerate()
                .find(|(_, v)| *v != 0);
            eprintln!("  first nonzero u32: {:?}", nz);
            // dump first 64 bytes of the records region (R610 REC1=772)
            eprintln!("  rest[768..832]={:02x?}", &status.rest[768..832]);
        }
    }
}

