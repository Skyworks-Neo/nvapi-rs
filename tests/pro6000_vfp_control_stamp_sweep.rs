// Blackwell RTX PRO 6000 / 580.173.02: sweep the private (non-Client)
// V/F-POINTS GetControl (0xDA025C3E) stamp × layout space, live.
//
// Timeline this probe arbitrates: 2026-10-11 13:48 the 4.3MB big-block RMW
// (R610 canonical stamp) worked on this machine; 2026-10-12 the same path
// answers -9 (user report). Audit T2-5 claims the 580 gate is
// `(*a2-0x12420)&0xFFFEFFFF==0` ({0x12420, 0x22420}, 9248B compact block —
// same family as the Client API struct). Read-only (GetControl only).
//
// Run: cargo test -p nvapi --test pro6000_vfp_control_stamp_sweep -- --nocapture --ignored

#![allow(unused_must_use)]

use std::ptr;

use nvapi::PhysicalGpu;
use nvapi::sys::api::{
    NvAPI_GPU_ClockClientClkVfPointsGetControl, NvAPI_GPU_ClockClkVfPointsGetControl,
};
use nvapi::sys::gpu::clock::undocumented::{
    NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL, NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE,
};
use nvapi::sys::nvapi::{NvVersion, VersionedStructField};

fn dump_block(bytes: &[u8], points_base: usize, point_stride: usize, points: usize) {
    let nz: Vec<(usize, u32)> = (0..bytes.len() / 4)
        .map(|i| {
            (
                i * 4,
                u32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap()),
            )
        })
        .filter(|&(_, v)| v != 0)
        .collect();
    eprintln!("  ACCEPTED — nonzero u32: {}/{}", nz.len(), bytes.len() / 4);
    for (off, v) in nz.iter().take(40) {
        eprintln!("    +{off:04x}: 0x{v:08X} ({v})");
    }
    let mut rows = 0usize;
    for i in 0..points {
        let base = points_base + i * point_stride;
        if base + 24 > bytes.len() {
            break;
        }
        let ct = u32::from_le_bytes(bytes[base..base + 4].try_into().unwrap());
        let dv = i32::from_le_bytes(bytes[base + 20..base + 24].try_into().unwrap());
        if ct != 0 || dv != 0 {
            eprintln!("    rec[{i}] type=0x{ct:x} value(i32)={dv}");
            rows += 1;
            if rows >= 48 {
                eprintln!("    ... (more records truncated)");
                break;
            }
        }
    }
}

#[test]
#[ignore]
fn pro6000_vfp_control_stamp_sweep() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    eprintln!("GPU: {:?}", gpu.full_name());

    // --- A: the existing 4.3MB big-block stamps ---
    for (stamp, tag) in [
        (4670980u32, "R610 canonical 0x474604"),
        (82976, "Volta snapshot 0x14420"),
        (90116, "legacy 0x16004"),
    ] {
        let mut ctrl = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE>::new_zeroed();
            let mut b = b.assume_init();
            b.version = NvVersion::with_version(stamp);
            b
        };
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetControl(*gpu.handle(), ptr::from_mut(&mut *ctrl).cast())
        };
        eprintln!(
            "big-block stamp={stamp:#x} ({tag}): status={:#x}",
            st as i32
        );
        if st == 0 {
            dump_block(&ctrl.rest, 772, 1060, 64);
        }
    }

    // --- B: the Client API on the compact 9248B block (control) ---
    // Does the Client face still answer, and with which stamp?
    for (stamp, tag) in [
        (0u32, "client default (nvversion macro)"),
        (0x12420, "client ver1 0x12420"),
        (0x22420, "client ver2 0x22420"),
    ] {
        let mut ctrl: NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL = Default::default();
        if stamp != 0 {
            *ctrl.nvapi_version_mut() = NvVersion::with_version(stamp);
        }
        let st = unsafe { NvAPI_GPU_ClockClientClkVfPointsGetControl(*gpu.handle(), &mut ctrl) };
        eprintln!("client stamp={stamp:#x} ({tag}): status={:#x}", st as i32);
    }

    // --- C: the private (non-Client) handler on the COMPACT block ---
    for (stamp, tag) in [
        (0x12420u32, "private compact ver1 0x12420"),
        (0x22420, "private compact ver2 0x22420"),
    ] {
        let mut ctrl: NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL = Default::default();
        *ctrl.nvapi_version_mut() = NvVersion::with_version(stamp);
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetControl(*gpu.handle(), ptr::from_mut(&mut ctrl).cast())
        };
        eprintln!(
            "private-compact stamp={stamp:#x} ({tag}): status={:#x}",
            st as i32
        );
        if st == 0 {
            let bytes =
                unsafe { std::slice::from_raw_parts(ptr::from_ref(&ctrl) as *const u8, 9248) };
            dump_block(bytes, 68, 36, 255);
        }
    }
}

#[test]
#[ignore]
fn pro6000_vfp_control_cli_sequence_repro() {
    // Replicate the CLI's exact sequence: GetInfo (canonical stamp) →
    // seed masks → GetControl 0x474604. Isolates whether the -9 needs
    // the seeded buffer or the GetInfo-fallback branch.
    use nvapi::sys::api::NvAPI_GPU_ClockClkVfPointsGetInfo;
    use nvapi::sys::gpu::clock::undocumented::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE;

    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    for (gi, gpu) in gpus.iter().enumerate() {
        eprintln!("=== GPU {gi}: {:?} ===", gpu.full_name());
        for (info_stamp, tag) in [
            (0x78604u32, "INFO canonical 0x78604"),
            (369_796, "INFO R535_WIDE 0x5A484"),
            (83996, "INFO LEGACY 0x1481C"),
        ] {
            let mut info = unsafe {
                let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE>::new_zeroed();
                let mut b = b.assume_init();
                b.version = NvVersion::with_version(info_stamp);
                b
            };
            let st = unsafe {
                NvAPI_GPU_ClockClkVfPointsGetInfo(*gpu.handle(), ptr::from_mut(&mut *info).cast())
            };
            eprintln!("{tag}: status={:#x}", st as i32);
            if st != 0 {
                continue;
            }
            // seed the big-block control masks from THIS info output
            for (ctrl_stamp, ctag) in [(4670980u32, "CTRL 0x474604"), (82976, "CTRL 0x14420")] {
                let mut ctrl = unsafe {
                    let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE>::new_zeroed();
                    let mut b = b.assume_init();
                    b.version = NvVersion::with_version(ctrl_stamp);
                    b
                };
                ctrl.seed_masks_from_info(&info);
                let stc = unsafe {
                    NvAPI_GPU_ClockClkVfPointsGetControl(
                        *gpu.handle(),
                        ptr::from_mut(&mut *ctrl).cast(),
                    )
                };
                eprintln!("  seeded {ctag}: status={:#x}", stc as i32);
                if stc == 0 {
                    // masks echoed? bank1 mask @+4 (128B)
                    let words: Vec<u32> = (0..8)
                        .map(|i| {
                            u32::from_le_bytes(ctrl.rest[i * 4..i * 4 + 4].try_into().unwrap())
                        })
                        .collect();
                    eprintln!("    mask1[0..8]: {:08X?}", words);
                }
            }
        }
    }
}
