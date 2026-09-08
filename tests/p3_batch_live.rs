//! Live census for the nvClocks.spec P3 batch — READ-ONLY.
//! These trees were empty on every tested part (1650S/TU116); this test
//! answers one question per driver/part: is the tree STILL empty? A
//! non-zero mask graduates that surface to a typed struct.
//!
//! Run with:
//!   cargo test -p nvapi --test p3_batch_live -- --nocapture --ignored
#![allow(unused_must_use)]

use nvapi::PhysicalGpu;

fn census(label: &str, st: i32, buf: &[u8]) {
    if st != 0 {
        println!(
            "{label}: status {} ({:?})",
            st as i32,
            nvapi::sys::status::Status::from_raw(st)
        );
        return;
    }
    let mask = u32::from_le_bytes(buf[4..8].try_into().unwrap());
    let words: Vec<(usize, u32)> = buf
        .chunks_exact(4)
        .enumerate()
        .map(|(i, c)| (i * 4 + 4, u32::from_le_bytes(c.try_into().unwrap())))
        .filter(|&(_, v)| v != 0)
        .collect();
    let first: Vec<(usize, u32)> = words.iter().copied().take(5).collect();
    println!(
        "{label}: OK mask={mask:#010x} nonzero_dwords={} first={first:?}",
        words.len()
    );
}

#[test]
#[ignore]
fn p3_batch_live_census() {
    nvapi::initialize().expect("NvAPI init failed");
    let gpus = PhysicalGpu::enumerate().expect("enumerate failed");
    let gpu = gpus.first().expect("no gpu");
    let h = *gpu.handle();
    println!("=== GPU: {} ===", gpu.full_name().unwrap_or_default());

    macro_rules! probe {
        ($label:literal, $ffi:path, $magic:literal, $size:expr, $seed_at:expr) => {{
            let mut buf = vec![0u8; $size];
            buf[0..4].copy_from_slice(&($magic as u32).to_le_bytes());
            if let Some(off) = $seed_at {
                buf[off..off + 4].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
            }
            let st = unsafe { $ffi(h, buf.as_mut_ptr()) };
            census($label, st, &buf);
        }};
    }

    // Progs: 0x183E4 (99300 B), no seed. 1650S: OK mask 0x07FFFFFF (27
    // progs) but records all-zero.
    probe!(
        "ProgsGetInfo",
        nvapi::sys::api::NvAPI_GPU_ClockClkProgsGetInfo,
        0x183E4,
        99_300,
        None
    );

    // VfRels: 0x7E53C (517436 B), no seed. 1650S: OK mask 0.
    probe!(
        "VfRelsGetInfo",
        nvapi::sys::api::NvAPI_GPU_ClockClkVfRelsGetInfo,
        0x7E53C,
        517_436,
        None
    );

    // VfRels control: 0x5E33C (385852 B), mask-seeded at +4. 1650S: OK,
    // records zero.
    probe!(
        "VfRelsGetControl",
        nvapi::sys::api::NvAPI_GPU_ClockClkVfRelsGetControl,
        0x5E33C,
        385_852,
        Some(4)
    );

    // Tops: 0x10D4C (68940 B), no seed (mask is OUTPUT at +8). 1650S: OK
    // mask 0.
    probe!(
        "TopsGetInfo",
        nvapi::sys::api::NvAPI_GPU_ClockClkPropTopsGetInfo,
        0x10D4C,
        68_940,
        None
    );

    // Enums: 0x14C18 (85016 B), no seed. 1650S: OK mask 0.
    probe!(
        "EnumsGetInfo",
        nvapi::sys::api::NvAPI_GPU_ClockClkEnumsGetInfo,
        0x14C18,
        85_016,
        None
    );

    // ThermDevice: 0x106A8 (67048 B), mask-seeded at +4. 1650S: OK, real
    // device mask 0x20F, 5 devices (types 1,2,2,2,3) — EXPECTED non-empty.
    probe!(
        "ThermDeviceGetInfo",
        nvapi::sys::api::NvAPI_GPU_ThermDeviceGetInfo,
        0x106A8,
        67_048,
        Some(4)
    );

    println!();
    println!("(done — read-only; a non-zero mask means that surface");
    println!(" graduated from raw probe to a typed struct)");
}
