// Raw dword dump of the VoltVoltRails family (GetInfo/GetControl/GetStatus)
// on the live GPU — find undecoded fields beyond the known
// type@+72 / values@+76 dense-entry map.
//
// Run: cargo test -p nvapi --test volt_rails_raw_dump -- --nocapture --ignored

#![allow(unused_must_use)]

use core::ptr;
use nvapi::PhysicalGpu;
use nvapi::sys::api::{
    NvAPI_GPU_VoltVoltRailsGetControl, NvAPI_GPU_VoltVoltRailsGetInfo,
    NvAPI_GPU_VoltVoltRailsGetStatus,
};
use nvapi::sys::gpu::power::undocumented::{
    NV_GPU_VOLT_RAILS_CONTROL, NV_GPU_VOLT_RAILS_INFO, NV_GPU_VOLT_RAILS_STATUS,
};
use nvapi::sys::nvapi::NvVersion;

fn dump_dwords(tag: &str, version: u32, rest: &[u8]) {
    eprintln!("== {tag} (total {} bytes) ==", 8 + rest.len());
    // full non-zero dword sweep, whole buffer
    let mut d0 = version.to_le_bytes();
    let mut all = Vec::with_capacity(8 + rest.len());
    all.extend_from_slice(&d0);
    all.extend_from_slice(rest);
    for off in (0..all.len() / 4 * 4).step_by(4) {
        let d = u32::from_le_bytes(all[off..off + 4].try_into().unwrap());
        if d != 0 {
            eprintln!("  +{off:04x}: {d:#010x}  ({d})");
        }
    }
}

#[test]
#[ignore]
fn volt_rails_raw_dump() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    eprintln!("GPU: {:?}", gpu.full_name());

    // --- GetInfo (V2 production stamp) ---
    let mut info = unsafe {
        let b = Box::<NV_GPU_VOLT_RAILS_INFO>::new_zeroed();
        let mut b = b.assume_init();
        b.version = NvVersion::with_version((2 << 16) | 6220);
        b
    };
    let st =
        unsafe { NvAPI_GPU_VoltVoltRailsGetInfo(*gpu.handle(), ptr::from_mut(&mut *info).cast()) };
    eprintln!("GetInfo: status={:#x}", st as i32);
    if st != 0 {
        panic!("GetInfo failed");
    }
    let rail_mask = info.rail_mask;
    eprintln!("rail_mask = 0x{rail_mask:08X}");
    dump_dwords("GetInfo", info.version.data, &info.rest);

    // --- GetControl (V2, seeded from info like production) ---
    let mut control = unsafe {
        let b = Box::<NV_GPU_VOLT_RAILS_CONTROL>::new_zeroed();
        let mut b = b.assume_init();
        b.version = NvVersion::with_version((2 << 16) | 2760);
        b.seed_from_info(&info);
        b
    };
    let st = unsafe {
        NvAPI_GPU_VoltVoltRailsGetControl(*gpu.handle(), ptr::from_mut(&mut *control).cast())
    };
    eprintln!("\nGetControl: status={:#x}", st as i32);
    dump_dwords("GetControl", control.version.data, &control.rest);

    // --- GetStatus (V1 stamp 0x10AC8, seeded from info) ---
    let mut status = unsafe {
        let b = Box::<NV_GPU_VOLT_RAILS_STATUS>::new_zeroed();
        let mut b = b.assume_init();
        b.version = NvVersion::with_version(NV_GPU_VOLT_RAILS_CONTROL::MAGIC_V1);
        b.seed_from_info(&info);
        b
    };
    let st = unsafe {
        NvAPI_GPU_VoltVoltRailsGetStatus(*gpu.handle(), ptr::from_mut(&mut *status).cast())
    };
    eprintln!("\nGetStatus: status={:#x}", st as i32);
    dump_dwords("GetStatus", status.version.data, &status.rest);
}
