// Volta (V100/GV100) private V/F-POINTS GetInfo layout probe.
//
// Live 2026-08-31: the V100 ACCEPTS the R610 stamp (0x78604) yet the
// returned buffer decodes as a ~72B-entry table — repeating u64
// 0x0000FFFF00000001 at 9-u64 (72B) stride plus exact-decimal µV values
// (450000 = 450 mV curve-start, 456250) — NOT the R610 "2048-bit mask +
// 104B descriptor" layout, so the production reader floods ~200 noise
// points (see gpu.rs masks fix + reverse/v100-vfp-snapshot.json).
//
// This probe dumps the raw GetInfo response under BOTH stamps —
//   R610    = NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE::MAGIC (0x78604)
//   LEGACY  = ...::MAGIC_LEGACY (0x1481C, the R391 small-table stamp)
// — to distinguish "the Volta handler only knows the legacy small-table
// shape" from "accepts the big magic but fills Volta-shaped content".
// Read-only (GetInfo + GetStatus, no SetControl).
//
// Run: cargo test --test volta_vfp_layout_probe -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;
use nvapi::sys::api::{NvAPI_GPU_ClockClkVfPointsGetInfo, NvAPI_GPU_ClockClkVfPointsGetStatus};
use nvapi::sys::gpu::clock::undocumented::{
    NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE,
    NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE,
};
use nvapi::sys::nvapi::NvVersion;

fn hexdump(rest: &[u8], len: usize) {
    for row in 0..len / 16 {
        let off = row * 16;
        let line: String = rest[off..off + 16]
            .iter()
            .map(|b| format!("{b:02x} "))
            .collect();
        eprintln!("  +{off:04x}: {line}");
    }
}

/// nonzero u64 words over the first `words` u64s of `rest`, with index
/// (byte offset = 8*i + 4, since `rest` starts after the version dword).
fn nonzero_u64s(rest: &[u8], words: usize) -> Vec<(usize, u64)> {
    (0..words)
        .map(|i| {
            let off = i * 8;
            let v = u64::from_le_bytes(rest[off..off + 8].try_into().unwrap());
            (i, v)
        })
        .filter(|(_, v)| *v != 0)
        .collect()
}

#[test]
#[ignore]
fn volta_vfp_layout_probe() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    eprintln!("=== {} GPU(s) ===", gpus.len());
    for (i, gpu) in gpus.iter().enumerate() {
        eprintln!(
            "--- GPU {} {:?} / {:?} ---",
            i,
            gpu.short_name(),
            gpu.full_name()
        );

        for (stamp, tag) in [
            (
                NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE::MAGIC,
                "R610",
            ),
            (
                NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE::MAGIC_LEGACY,
                "LEGACY",
            ),
        ] {
            let mut info = unsafe {
                let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE>::new_zeroed();
                let mut b = b.assume_init();
                b.version = NvVersion::with_version(stamp);
                b
            };
            let st = unsafe {
                NvAPI_GPU_ClockClkVfPointsGetInfo(
                    *gpu.handle(),
                    core::ptr::from_mut(&mut *info).cast(),
                )
            };
            eprintln!("GetInfo[{tag}] stamp={stamp:#x}: status={:#x}", st as i32);
            if st != 0 {
                continue;
            }
            // first 0x200 bytes — header + whatever structure lives there
            eprintln!("  --- info.rest[0..0x200] hexdump ---");
            hexdump(&info.rest, 0x200);
            // nonzero u64 words, first 512 u64s (4 KB) — entry-value census
            let nz = nonzero_u64s(&info.rest, 512);
            eprintln!("  nonzero u64 words (first 4KB): {}/512", nz.len());
            for (i, v) in nz.iter().take(40) {
                eprintln!("    u64[{i:3}] (+{:04x}) = 0x{v:016X} ({v})", i * 8);
            }

            // GetStatus under the matching stamp, header seeded from GetInfo
            let status_stamp = if tag == "R610" {
                NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE::MAGIC
            } else {
                NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE::MAGIC_LEGACY
            };
            let mut status = unsafe {
                let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE>::new_zeroed();
                let mut b = b.assume_init();
                b.version = NvVersion::with_version(status_stamp);
                b
            };
            status.rest[..128].copy_from_slice(&info.rest[..128]);
            let st = unsafe {
                NvAPI_GPU_ClockClkVfPointsGetStatus(
                    *gpu.handle(),
                    core::ptr::from_mut(&mut *status).cast(),
                )
            };
            eprintln!(
                "GetStatus[{tag}] stamp={status_stamp:#x}: status={:#x}",
                st as i32
            );
            if st == 0 {
                eprintln!("  --- status.rest[0..0x100] hexdump ---");
                hexdump(&status.rest, 0x100);
            }
        }
    }
}

/// Census of the LEGACY layout: walk the 132-bit mask population and dump
/// candidate u32 fields across ALL records — the frequency field is the
/// one that ascends monotonically with the voltage grid (6.25 mV steps).
#[test]
#[ignore]
fn volta_legacy_record_census() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");

    let mut info = unsafe {
        let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE>::new_zeroed();
        let mut b = b.assume_init();
        b.version =
            NvVersion::with_version(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE::MAGIC_LEGACY);
        b
    };
    let st = unsafe {
        NvAPI_GPU_ClockClkVfPointsGetInfo(*gpu.handle(), core::ptr::from_mut(&mut *info).cast())
    };
    assert_eq!(st, 0, "legacy GetInfo failed: {st:#x}");

    let mut status = unsafe {
        let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE>::new_zeroed();
        let mut b = b.assume_init();
        b.version =
            NvVersion::with_version(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE::MAGIC_LEGACY);
        b
    };
    status.rest[..128].copy_from_slice(&info.rest[..128]);
    let st = unsafe {
        NvAPI_GPU_ClockClkVfPointsGetStatus(*gpu.handle(), core::ptr::from_mut(&mut *status).cast())
    };
    assert_eq!(st, 0, "legacy GetStatus failed: {st:#x}");

    // INFO-side: legacy descriptor candidates at 0x48 stride from +0x60
    eprintln!("=== INFO legacy descriptors (0x48 stride from +0x60) ===");
    for k in 0..8usize {
        let base = 0x60 + k * 0x48;
        let fields: Vec<u32> = [
            0x00, 0x04, 0x08, 0x0c, 0x28, 0x2c, 0x30, 0x34, 0x38, 0x3c, 0x40, 0x44,
        ]
        .iter()
        .map(|&o| u32::from_le_bytes(info.rest[base + o..base + o + 4].try_into().unwrap()))
        .collect();
        eprintln!("  desc[{k:3}] @+{base:04x}: {fields:08x?}");
    }

    // STATUS-side: sequential walk at the established base 0x60 /
    // stride 0x4c — where does the {1, voltage_uV, freq} alignment
    // desync? That boundary is a segment transition (2nd curve / bins),
    // mirroring how Pascal packs bank 0.
    eprintln!("=== STATUS sequential walk: base 0x60 stride 0x4c ===");
    for k in 0..132usize {
        let b0 = 0x60 + k * 0x4c;
        let f: Vec<u32> = (0..4)
            .map(|j| {
                let o = b0 + j * 4;
                u32::from_le_bytes(status.rest[o..o + 4].try_into().unwrap())
            })
            .collect();
        eprintln!("  rec[{k:3}] @+{b0:05x}: {f:08x?}");
    }

    // hexdump the first 3 records region raw for visual boundary check
    eprintln!("=== STATUS raw 0x40..0x340 ===");
    hexdump(&status.rest, 0x340);
}
