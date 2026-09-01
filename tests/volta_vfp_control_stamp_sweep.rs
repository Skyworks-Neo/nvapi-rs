// V100/GV100: sweep the V/F-POINTS CONTROL (GetControl 0xDA025C3E) magic
// space. The R610 handler accepts the canonical 4670980 (0x474604) plus
// smaller snapshot magics {82976, 401472, 737404, 1348740}; the legacy
// (1<<16)|0x6004 variant is rejected on V100 — find whether ANY control
// stamp is live on Volta. Read-only.
//
// Run: cargo test -p nvapi --test volta_vfp_control_stamp_sweep -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;
use nvapi::sys::api::NvAPI_GPU_ClockClkVfPointsGetControl;
use nvapi::sys::gpu::clock::undocumented::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE;
use nvapi::sys::nvapi::NvVersion;

#[test]
#[ignore]
fn volta_vfp_control_stamp_sweep() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    eprintln!("GPU: {:?}", gpu.full_name());

    let stamps: &[(u32, &str)] = &[
        (4670980, "R610 canonical 0x474604"),
        (82976, "R610 snapshot 0x14420"),
        (401472, "R610 snapshot 0x621C0"),
        (737404, "R610 snapshot 0xB407C"),
        (1348740, "R610 snapshot 0x149104"),
        ((1 << 16) | 0x6004, "legacy (1<<16)|0x6004"),
        ((1 << 16) | 0x474604 % 0x10000, "hybrid nonsense guard"),
    ];
    for &(stamp, tag) in stamps {
        let mut ctrl = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE>::new_zeroed();
            let mut b = b.assume_init();
            b.version = NvVersion::with_version(stamp);
            b
        };
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetControl(*gpu.handle(), ptr::from_mut(&mut *ctrl).cast())
        };
        eprintln!("GetControl stamp={stamp} ({tag}): status={:#x}", st as i32);
        if st == 0 {
            let nz: Vec<(usize, u32)> = (0..ctrl.rest.len() / 4)
                .map(|i| {
                    (
                        i * 4,
                        u32::from_le_bytes(ctrl.rest[i * 4..i * 4 + 4].try_into().unwrap()),
                    )
                })
                .filter(|(_, v)| *v != 0)
                .collect();
            eprintln!(
                "  ACCEPTED — nonzero u32: {}/{}",
                nz.len(),
                ctrl.rest.len() / 4
            );
            for (off, v) in nz.iter().take(40) {
                eprintln!("    +{off:04x}: 0x{v:08X} ({v})");
            }
        }
    }
}

use core::ptr;
