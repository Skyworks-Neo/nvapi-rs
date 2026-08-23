//! Live probe for the ClientPowerModes family (NVIDIA App's Balanced/Max
//! power-mode switcher): GetInfo (0xF21C2D56, 5388B) + GetControl
//! (0x180A9468, 4108B) + readback dump. SET is NOT called by default.
//!
//! Run: `cargo run --release --example probe_powermodes`

use nvapi::initialize;
use nvapi::sys::api::{NvAPI_EnumPhysicalGPUs, NvVersion};
use nvapi::sys::gpu::power::private::{
    NV_GPU_CLIENT_POWER_MODES_CONTROL, NV_GPU_CLIENT_POWER_MODES_INFO,
};
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::nvapi::VersionedStruct;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use std::ptr;

fn main() {
    let _ = initialize();
    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS as usize];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    let gpu = handles[0];

    // 1. GetInfo
    let mut info = Box::new(unsafe { std::mem::zeroed::<NV_GPU_CLIENT_POWER_MODES_INFO>() });
    *info.nvapi_version_mut() =
        NvVersion::with_struct::<NV_GPU_CLIENT_POWER_MODES_INFO>(1);
    let r1 = unsafe {
        nvapi::sys::api::NvAPI_GPU_ClientPowerModesGetInfo(gpu, ptr::from_mut(&mut *info))
    };
    println!("GetInfo st={r1}");
    if r1 == 0 {
        let b = unsafe {
            std::slice::from_raw_parts(ptr::from_ref(&*info).cast::<u8>(), 5388)
        };
        let nz = b[4..].chunks(4).enumerate().filter(|(_, c)| c.iter().any(|&x| x != 0)).count();
        println!("  nonzero dwords after header: {nz}");
        for (i, w) in b[4..100].chunks(4).enumerate() {
            let v = u32::from_le_bytes(w.try_into().unwrap());
            if v != 0 { println!("  +{}: {v} (0x{v:X})", 4 + i * 4); }
        }
    }

    // 2. GetControl
    let mut ctrl = Box::new(unsafe { std::mem::zeroed::<NV_GPU_CLIENT_POWER_MODES_CONTROL>() });
    *ctrl.nvapi_version_mut() =
        NvVersion::with_struct::<NV_GPU_CLIENT_POWER_MODES_CONTROL>(1);
    let r2 = unsafe {
        nvapi::sys::api::NvAPI_GPU_ClientPowerModesGetControl(gpu, ptr::from_mut(&mut *ctrl))
    };
    println!("GetControl st={r2}");
    if r2 == 0 {
        let b = unsafe {
            std::slice::from_raw_parts(ptr::from_ref(&*ctrl).cast::<u8>(), 4108)
        };
        for (i, w) in b[4..64].chunks(4).enumerate() {
            let v = u32::from_le_bytes(w.try_into().unwrap());
            if v != 0 { println!("  +{}: {v} (0x{v:X})", 4 + i * 4); }
        }
    }
}
