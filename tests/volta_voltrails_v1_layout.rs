// VoltRails V1 layout mapper (V100/GV100 — the only live V1 part on this
// bench). Full-buffer nonzero-u32 census of the three V1 responses, with a
// TWO-SAMPLE diff: fields that change between samples are LIVE telemetry
// (current voltage candidates); static fields are descriptors.
//
//   INFO   V1: stamp (1<<16)|0xACC = 68300, total 2764B
//   STATUS V1: stamp (1<<16)|0xAC8 = 68296, total 2760B (production uses
//              this stamp already; layout unmapped)
//   CONTROL V1: stamp candidates (1<<16)|2760 / (1<<16)|2764
//
// Read-only (GetInfo/GetStatus/GetControl).
//
// Run: cargo test -p nvapi --test volta_voltrails_v1_layout -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;
use nvapi::sys::api::{
    NvAPI_GPU_VoltVoltRailsGetControl, NvAPI_GPU_VoltVoltRailsGetInfo,
    NvAPI_GPU_VoltVoltRailsGetStatus,
};
use nvapi::sys::gpu::power::undocumented::{
    NV_GPU_VOLT_RAILS_CONTROL, NV_GPU_VOLT_RAILS_INFO, NV_GPU_VOLT_RAILS_STATUS_V1,
};
use nvapi::sys::nvapi::NvVersion;

fn census(tag: &str, buf: &[u8]) -> Vec<(usize, u32)> {
    let nz: Vec<(usize, u32)> = (0..buf.len() / 4)
        .map(|i| {
            (
                i * 4,
                u32::from_le_bytes(buf[i * 4..i * 4 + 4].try_into().unwrap()),
            )
        })
        .filter(|(_, v)| *v != 0)
        .collect();
    eprintln!("=== {tag}: {}/{} u32 nonzero ===", nz.len(), buf.len() / 4);
    for (off, v) in &nz {
        eprintln!(
            "  +{off:04x} (d{:^3}): 0x{v:08X} ({v}) i32=({}) f32bits=0x{:08X}",
            off / 4,
            *v as i32,
            v
        );
    }
    nz
}

fn diff_census(tag: &str, a: &[u8], b: &[u8]) {
    let changed: Vec<usize> = (0..a.len() / 4)
        .filter(|&i| a[i * 4..i * 4 + 4] != b[i * 4..i * 4 + 4])
        .map(|i| i * 4)
        .collect();
    eprintln!(
        "=== {tag}: {} changed u32 between samples ===",
        changed.len()
    );
    for off in &changed {
        let va = u32::from_le_bytes(a[*off..*off + 4].try_into().unwrap());
        let vb = u32::from_le_bytes(b[*off..*off + 4].try_into().unwrap());
        eprintln!("  +{off:04x}: 0x{va:08X} ({va}) -> 0x{vb:08X} ({vb})");
    }
}

#[test]
#[ignore]
fn volta_voltrails_v1_layout() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    eprintln!("GPU: {:?}", gpu.full_name());

    // ── INFO V1 ──
    let mut info = unsafe {
        let b = Box::<NV_GPU_VOLT_RAILS_INFO>::new_zeroed();
        let mut b = b.assume_init();
        b.version = NvVersion::with_version((1 << 16) | 0xACC);
        b
    };
    let st =
        unsafe { NvAPI_GPU_VoltVoltRailsGetInfo(*gpu.handle(), ptr::from_mut(&mut *info).cast()) };
    eprintln!("GetInfo[V1] status={:#x}", st as i32);
    assert_eq!(st, 0, "INFO V1 must be accepted for this probe");
    let info_a: Vec<u8> = info.rest.to_vec();
    census("INFO V1 rest (sample A)", &info_a);

    // ── STATUS V1 ── (stamp 0x10AC8, seeded from the V1 info)
    let mut status = unsafe {
        let b = Box::<NV_GPU_VOLT_RAILS_STATUS_V1>::new_zeroed();
        let mut b = b.assume_init();
        b.version = NvVersion::with_version((1 << 16) | 0xAC8);
        b
    };
    status.rail_mask = info.rail_mask;
    let st = unsafe {
        NvAPI_GPU_VoltVoltRailsGetStatus(*gpu.handle(), ptr::from_mut(&mut *status).cast())
    };
    eprintln!("GetStatus[V1] status={:#x}", st as i32);
    let status_a: Vec<u8> = if st == 0 {
        let v = status.rest.to_vec();
        census("STATUS V1 rest (sample A)", &v);
        v
    } else {
        Vec::new()
    };

    // ── CONTROL V1 stamp candidates ──
    for &cstamp in &[(1 << 16) | 2760, (1 << 16) | 2764] {
        let mut ctrl = unsafe {
            let b = Box::<NV_GPU_VOLT_RAILS_CONTROL>::new_zeroed();
            let mut b = b.assume_init();
            b.version = NvVersion::with_version(cstamp);
            b
        };
        ctrl.rail_mask = info.rail_mask;
        let st = unsafe {
            NvAPI_GPU_VoltVoltRailsGetControl(*gpu.handle(), ptr::from_mut(&mut *ctrl).cast())
        };
        eprintln!("GetControl stamp={cstamp:#x}: status={:#x}", st as i32);
        if st == 0 {
            let v = ctrl.rest.to_vec();
            census(&format!("CONTROL V1 rest (stamp {cstamp:#x})"), &v);
            break;
        }
    }

    // ── second sample after a pause → live-field diff ──
    std::thread::sleep(std::time::Duration::from_millis(3000));
    let mut info2 = unsafe {
        let b = Box::<NV_GPU_VOLT_RAILS_INFO>::new_zeroed();
        let mut b = b.assume_init();
        b.version = NvVersion::with_version((1 << 16) | 0xACC);
        b
    };
    let st =
        unsafe { NvAPI_GPU_VoltVoltRailsGetInfo(*gpu.handle(), ptr::from_mut(&mut *info2).cast()) };
    if st == 0 {
        let info_b: Vec<u8> = info2.rest.to_vec();
        diff_census("INFO V1", &info_a, &info_b);
    }

    if !status_a.is_empty() {
        let mut status2 = unsafe {
            let b = Box::<NV_GPU_VOLT_RAILS_STATUS_V1>::new_zeroed();
            let mut b = b.assume_init();
            b.version = NvVersion::with_version((1 << 16) | 0xAC8);
            b
        };
        status2.rail_mask = info.rail_mask;
        let st = unsafe {
            NvAPI_GPU_VoltVoltRailsGetStatus(*gpu.handle(), ptr::from_mut(&mut *status2).cast())
        };
        if st == 0 {
            let status_b: Vec<u8> = status2.rest.to_vec();
            diff_census("STATUS V1", &status_a, &status_b);
        }
    }
}

use core::ptr;
