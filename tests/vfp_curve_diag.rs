// Diagnostic: call PhysicalGpu::vfp_curve directly (no hi-layer allowable_result
// swallowing) and print the raw Result, so we can see whether GetStatus returns
// Ok(empty)/Err(NOT_SUPPORTED)/Err(IncompatibleStructVersion) on legacy HW.
// Run with: cargo test --test vfp_curve_diag -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;
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
                eprintln!(
                    "vfp_info: ok mask={:?} domains={:?}",
                    info.mask.mask, info.domains
                );
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
            NvAPI_GPU_ClockClkVfPointsGetStatus(
                *gpu.handle(),
                core::ptr::from_mut(&mut *status).cast(),
            )
        };
        eprintln!("private GetStatus {:28}: status={:#x}", label, st as i32);
        if st == 0 {
            // scan for first non-zero u32 in the payload
            let nz = status
                .rest
                .chunks_exact(4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .enumerate()
                .find(|(_, v)| *v != 0);
            eprintln!("  first nonzero u32: {:?}", nz);
            // dump first 64 bytes of the records region (R610 REC1=772)
            eprintln!("  rest[768..832]={:02x?}", &status.rest[768..832]);
        }
    }
}

/// Probe private GetControl (0xDA025C3E) with R610 vs 391.35 legacy stamp.
#[test]
#[ignore]
fn private_control_probe() {
    use nvapi::sys::api::NvAPI_GPU_ClockClkVfPointsGetControl;
    use nvapi::sys::gpu::clock::undocumented::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE;

    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");

    for (label, magic) in [("R610 0x474604", 4670980u32), ("391.35 0x16004", 90116u32)] {
        let mut ctrl = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE>::new_zeroed();
            let mut b = b.assume_init();
            b.version = NvVersion::with_version(magic);
            b
        };
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetControl(
                *gpu.handle(),
                core::ptr::from_mut(&mut *ctrl).cast(),
            )
        };
        eprintln!(
            "private GetControl {:16}: sent_ver={:#x} ({}) status={:#x}",
            label, ctrl.version.data, ctrl.version.data, st as i32
        );
    }
}

/// Probe NvAPI_GPU_GetVbiosImage (0xFC13EE11) — does escape 0x0700004F
/// succeed on 391.35 where the VFP escape 0x0700004A is kernel-unimplemented?
/// Both V1 (0x10008, 64 KiB) and V2 (0x20010, 1 MiB) stamps, plus the
/// size-query mode (size=0).
#[test]
#[ignore]
fn vbios_image_probe() {
    use nvapi::sys::api::NvAPI_GPU_GetVbiosImage;
    use nvapi::sys::gpu::NV_GPU_VBIOS_IMAGE;

    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");

    // size-query: V2 stamp, size=0 → handler fills default size, no copy
    let mut q = NV_GPU_VBIOS_IMAGE {
        version: NvVersion::with_version(0x20010),
        size: 0,
        pImage: 0,
    };
    let st = unsafe { NvAPI_GPU_GetVbiosImage(*gpu.handle(), &mut q) };
    eprintln!(
        "vbios query V2 size=0: status={:#x} size_out={}",
        st as i32, q.size
    );

    // V2 full read into 1 MiB buffer
    let mut buf = vec![0u8; 1024000];
    let mut img = NV_GPU_VBIOS_IMAGE {
        version: NvVersion::with_version(0x20010),
        size: buf.len() as u32,
        pImage: buf.as_mut_ptr() as usize,
    };
    let st = unsafe { NvAPI_GPU_GetVbiosImage(*gpu.handle(), &mut img) };
    eprintln!(
        "vbios read V2 1MB: status={:#x} size_out={}",
        st as i32, img.size
    );
    if st == 0 && img.size > 0 {
        let actual = (img.size as usize).min(buf.len());
        // VBIOS signature: 0x55 0xAA at offset 0x180 (boot sector magic),
        // and "BIT" at the BIT-table pointer target. Dump first 32 bytes
        // + check for the NVIDIA BIT header.
        eprintln!("  first 32 bytes: {:02x?}", &buf[..32]);
        // scan for "BIT" signature
        if let Some(pos) = buf[..actual].windows(3).position(|w| w == b"BIT") {
            eprintln!("  found 'BIT' at offset {:#x}", pos);
        }
    }
}
