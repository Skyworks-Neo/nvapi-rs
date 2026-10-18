// Live probe for the PerfPolicies family (GetInfo 0x409D9841 / GetStatus
// 0x3D358A0C) — dumps both raw buffers dword-by-dword to calibrate the
// active-policy mask bits (status `unknown` dword) and the timer fields
// against real hardware states (idle / load).
//
// Run: cargo test -p nvapi --test perf_policies_probe -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;
use nvapi::sys::api::{NvAPI_GPU_PerfPoliciesGetInfo, NvAPI_GPU_PerfPoliciesGetStatus};
use nvapi::sys::gpu::power::undocumented::{
    NV_GPU_PERF_POLICIES_INFO_PARAMS, NV_GPU_PERF_POLICIES_STATUS_PARAMS,
};

fn dump_dwords(tag: &str, bytes: &[u8]) {
    eprintln!("== {tag} (total {} bytes) ==", bytes.len());
    for off in (0..bytes.len() / 4 * 4).step_by(4) {
        let d = u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap());
        if d != 0 {
            eprintln!("  +{off:04x}: {d:#010x}  ({d})");
        }
    }
}

unsafe fn as_bytes<T>(v: &T) -> &[u8] {
    std::slice::from_raw_parts(std::ptr::from_ref(v).cast::<u8>(), std::mem::size_of::<T>())
}

#[test]
#[ignore]
fn perf_policies_raw_dump() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    eprintln!("GPU: {:?}", gpu.full_name());

    // --- GetInfo ---
    let mut info: NV_GPU_PERF_POLICIES_INFO_PARAMS = Default::default();
    let st = unsafe { NvAPI_GPU_PerfPoliciesGetInfo(*gpu.handle(), ptr_from_mut(&mut info)) };
    eprintln!("GetInfo: status={:#x}", st as i32);
    if st != 0 {
        panic!("GetInfo failed");
    }
    dump_dwords("GetInfo", unsafe { as_bytes(&info) });

    // --- GetStatus ---
    let mut status: NV_GPU_PERF_POLICIES_STATUS_PARAMS = Default::default();
    let st = unsafe { NvAPI_GPU_PerfPoliciesGetStatus(*gpu.handle(), ptr_from_mut(&mut status)) };
    eprintln!("GetStatus: status={:#x}", st as i32);
    if st != 0 {
        panic!("GetStatus failed");
    }
    eprintln!(
        "GetStatus decoded: flags={} limits={:#x} unknown(active raw)={:#x} timer={} timers={:?}",
        status.flags, status.limits.value, status.unknown, status.timer, status.timers
    );
    dump_dwords("GetStatus", unsafe { as_bytes(&status) });
}

unsafe fn ptr_from_mut<T>(v: &mut T) -> *mut T {
    std::ptr::from_mut(v)
}
