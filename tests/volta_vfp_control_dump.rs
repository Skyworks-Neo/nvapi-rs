// V100: post-SET-state dump — READ-ONLY. After the point-64 write attempt
// (SET accepted, readback mismatched): dump record 64's full 68 bytes, all
// nonzero control words, and the private-vftable point 64 defaults.
//
// Run: cargo test -p nvapi --test volta_vfp_control_dump -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;
use nvapi::sys::api::NvAPI_GPU_ClockClkVfPointsGetControl;
use nvapi::sys::gpu::clock::undocumented::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE;
use nvapi::sys::nvapi::NvVersion;

const SNAPSHOT_MAGIC: u32 = 82976;
const SCAN: usize = 64 * 1024;

#[test]
#[ignore]
fn volta_vfp_control_dump_record64() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    eprintln!("GPU: {:?}", gpu.full_name());

    let mut ctrl = unsafe {
        let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE>::new_zeroed();
        let mut b = b.assume_init();
        b.version = NvVersion::with_version(SNAPSHOT_MAGIC);
        b
    };
    // 17B LE-bitfield mask over records 0..131 (all present bits)
    for r in 0..=131usize {
        ctrl.rest[r / 8] |= 1 << (r % 8);
    }
    let st = unsafe {
        NvAPI_GPU_ClockClkVfPointsGetControl(*gpu.handle(), ptr::from_mut(&mut *ctrl).cast())
    };
    eprintln!("GET status: {st:#x}");
    assert_eq!(st, 0);

    // record 64 full window
    let base = 0x60 + 64 * 0x44;
    eprintln!("record 64 window +{base:05x}..+{:05x}:", base + 0x44);
    for w in 0..0x44 / 4 {
        let off = base + w * 4;
        let v = u32::from_le_bytes(ctrl.rest[off..off + 4].try_into().unwrap());
        if v != 0 {
            eprintln!(
                "  rec64+{:#04x} (+{off:05x}): 0x{v:08X} ({v} as i32 = {})",
                v, v as i32
            );
        }
    }

    // ALL nonzero words in the records region
    eprintln!("all nonzero words +0x00..+{SCAN:05x}:");
    for i in 0..SCAN / 4 {
        let off = i * 4;
        let v = u32::from_le_bytes(ctrl.rest[off..off + 4].try_into().unwrap());
        if v != 0 {
            eprintln!("  +{off:05x}: 0x{v:08X} ({})", v as i32);
        }
    }

    // private vftable point 64 — did the +2MHz land in driver state?
    let vfp = gpu.clk_vf_points_private().expect("points");
    for p in vfp
        .points
        .iter()
        .filter(|p| p.bank == 0 && (64..=65).contains(&p.index))
    {
        eprintln!(
            "vfp point {}: default {} cur {}",
            p.index, p.freq_default_mhz, p.freq_current_mhz
        );
    }

    // raw STATUS records (0x60 + r*0x4C, full 0x4C window) for 0/1/63/64/65
    use nvapi::sys::api::{NvAPI_GPU_ClockClkVfPointsGetInfo, NvAPI_GPU_ClockClkVfPointsGetStatus};
    use nvapi::sys::gpu::clock::undocumented::{
        NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE,
        NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE,
    };
    let mut info = unsafe {
        let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE>::new_zeroed();
        let mut b = b.assume_init();
        b.version =
            NvVersion::with_version(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE::MAGIC_LEGACY);
        b
    };
    let st = unsafe {
        NvAPI_GPU_ClockClkVfPointsGetInfo(*gpu.handle(), ptr::from_mut(&mut *info).cast())
    };
    eprintln!("INFO legacy status: {st:#x}");
    let mut status = unsafe {
        let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE>::new_zeroed();
        let mut b = b.assume_init();
        b.version =
            NvVersion::with_version(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE::MAGIC_LEGACY);
        b
    };
    info.seed_status_header(&mut status);
    let st = unsafe {
        NvAPI_GPU_ClockClkVfPointsGetStatus(*gpu.handle(), ptr::from_mut(&mut *status).cast())
    };
    eprintln!("STATUS legacy status: {st:#x}");
    if st == 0 {
        for r in [0usize, 1, 63, 64, 65, 128, 131] {
            let base = 0x60 + r * 0x4C;
            let words: Vec<u32> = (0..0x4C / 4)
                .map(|w| {
                    u32::from_le_bytes(
                        status.rest[base + w * 4..base + w * 4 + 4]
                            .try_into()
                            .unwrap(),
                    )
                })
                .collect();
            eprintln!("status rec {r}: {words:08X?}");
        }
    }
}

use core::ptr;
