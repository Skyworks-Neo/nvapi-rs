// V100: raw hexdump of the PUBLIC VFP family (GetInfo / GetStatus V3 /
// GetControl) under the gpc-slot1-shifted state that breaks
// get-public-vftable. Question: is the raw buffer zeroed by the driver
// (fill-path break) or is our decoder misreading a shifted layout?
//
// Run: cargo test -p nvapi --test v100_public_vfp_raw -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;
use nvapi::sys::api::{
    NvAPI_GPU_ClockClientClkVfPointsGetControl, NvAPI_GPU_ClockClientClkVfPointsGetInfo,
    NvAPI_GPU_ClockClientClkVfPointsGetStatus,
};
use nvapi::sys::gpu::clock::undocumented::{
    NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL, NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO,
};
use nvapi::sys::gpu::power::undocumented::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS;

fn nonzero_runs(buf: &[u8], label: &str, max_runs: usize) {
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < buf.len() {
        if buf[i] != 0 {
            let start = i;
            while i < buf.len() && buf[i] != 0 {
                i += 1;
            }
            runs.push((start, i - start));
        } else {
            i += 1;
        }
    }
    // merge runs separated by <8 zero bytes
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (s, l) in runs {
        match merged.last_mut() {
            Some((_, end)) if s - *end < 8 => *end = s + l,
            _ => merged.push((s, s + l)),
        }
    }
    eprintln!("{label}: {} nonzero regions", merged.len());
    for (s, e) in merged.iter().take(max_runs) {
        eprintln!("  +{s:05x}..+{e:05x} ({}B)", e - s);
    }
    if merged.len() > max_runs {
        eprintln!("  ... ({} more)", merged.len() - max_runs);
    }
}

fn dump_head(buf: &[u8], base: usize, label: &str, entries: usize, stride: usize) {
    eprintln!("{label} head:");
    for e in 0..entries {
        let off = base + e * stride;
        if off + 16 > buf.len() {
            break;
        }
        let words: Vec<u32> = (0..4)
            .map(|w| u32::from_le_bytes(buf[off + w * 4..off + w * 4 + 4].try_into().unwrap()))
            .collect();
        eprintln!("  entry {e} +{off:05x}: {words:08X?}");
    }
}

#[test]
#[ignore]
fn v100_public_vfp_raw_dump() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    eprintln!("GPU: {:?}", gpu.full_name());

    // GetInfo: mask + per-point clock types (stamped Default — the family
    // packs (ver<<16)|size literals, hand-rolled stamps get -9)
    let mut info = NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO::default();
    let st = unsafe {
        NvAPI_GPU_ClockClientClkVfPointsGetInfo(*gpu.handle(), ptr::from_mut(&mut info).cast())
    };
    eprintln!("GetInfo: {st:#x}");
    assert_eq!(st, 0);
    let mask = info.mask;
    eprintln!("mask words: {:08X?}", mask.mask);

    // GetStatus V3
    let mut status = NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS::default();
    status.mask = mask;
    let st = unsafe {
        NvAPI_GPU_ClockClientClkVfPointsGetStatus(*gpu.handle(), ptr::from_mut(&mut status).cast())
    };
    eprintln!("GetStatus(v3): {st:#x}");
    if st == 0 {
        let raw = {
            let bytes: *const u8 = ptr::from_ref(&status).cast();
            unsafe { std::slice::from_raw_parts(bytes, std::mem::size_of_val(&status)) }
        };
        nonzero_runs(raw, "STATUS v3", 40);
        // header 0x4C? entries base = version(4)+mask+0x44 pad
        dump_head(raw, 0x4c, "STATUS v3", 4, 0x17c);
    }

    // GetControl (ver2 family)
    let mut ctrl = NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL::default();
    ctrl.mask = mask;
    let st = unsafe {
        NvAPI_GPU_ClockClientClkVfPointsGetControl(*gpu.handle(), ptr::from_mut(&mut ctrl).cast())
    };
    eprintln!("GetControl(ver2): {st:#x}");
    if st == 0 {
        let raw = {
            let bytes: *const u8 = ptr::from_ref(&ctrl).cast();
            unsafe { std::slice::from_raw_parts(bytes, std::mem::size_of_val(&ctrl)) }
        };
        nonzero_runs(raw, "CONTROL ver2", 40);
        dump_head(raw, 0x2c, "CONTROL", 4, 0x24);
    }
}

use core::ptr;
