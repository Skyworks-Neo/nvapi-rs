//! Live raw-enum dump for the RTX PRO 6000 Blackwell identity fields that the
//! CLI renders as "Unknown" — READ-ONLY getters only.
//!   cargo test -p nvapi --test blackwell_pro6000_identity_live -- --nocapture --ignored
//!
//! Captures the raw i32 behind GetGPUType / GetSystemType / GetRamType /
//! GetFoundry / GetBusType / GetArchitectureId so the enum tables can be
//! extended from evidence instead of guesses. See
//! docs/reverse-engineering/nvapi/blackwell-pro6000-580-173-02-audit.md T1.
//!
//! The out-pointer is an `&mut i32` cast to the generated `*mut NvValue<_>` —
//! the wrappers are repr(i32), and casting keeps out-of-range values visible
//! instead of tripping TryFrom validation.

use nvapi::PhysicalGpu;
use nvapi::sys::gpu::{
    NvAPI_GPU_GetArchInfo, NvAPI_GPU_GetBusId, NvAPI_GPU_GetBusSlotId, NvAPI_GPU_GetBusType,
    NvAPI_GPU_GetGPUType, NvAPI_GPU_GetIRQ, NvAPI_GPU_GetSystemType,
    undocumented::{NvAPI_GPU_GetFoundry, NvAPI_GPU_GetRamType},
};

#[test]
#[ignore = "live GPU read-only identity dump"]
fn blackwell_identity_raw() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    for (idx, gpu) in gpus.iter().enumerate() {
        let h = *gpu.handle();
        println!("== GPU {idx}: {} ==", gpu.full_name().unwrap_or_default());
        unsafe {
            let mut v = -1i32;
            let rc = NvAPI_GPU_GetGPUType(h, &mut v as *mut i32 as *mut _);
            println!("  GetGPUType rc={rc} raw={v} ({v:#x})");
            let mut v = -1i32;
            let rc = NvAPI_GPU_GetSystemType(h, &mut v as *mut i32 as *mut _);
            println!("  GetSystemType rc={rc} raw={v} ({v:#x})");
            let mut v = -1i32;
            let rc = NvAPI_GPU_GetRamType(h, &mut v as *mut i32 as *mut _);
            println!("  GetRamType rc={rc} raw={v} ({v:#x})");
            let mut v = -1i32;
            let rc = NvAPI_GPU_GetFoundry(h, &mut v as *mut i32 as *mut _);
            println!("  GetFoundry rc={rc} raw={v} ({v:#x})");
            let mut v = -1i32;
            let rc = NvAPI_GPU_GetBusType(h, &mut v as *mut i32 as *mut _);
            println!("  GetBusType rc={rc} raw={v} ({v:#x})");
            let mut v = -1i32;
            let rc = NvAPI_GPU_GetArchInfo(h, &mut v as *mut i32 as *mut _);
            println!("  GetArchInfo rc={rc} raw={v} ({v:#x})");
            // bus_info() sub-calls — find which one hard-fails on Linux
            let mut v = -1i32;
            let rc = NvAPI_GPU_GetIRQ(h, &mut v as *mut i32 as *mut _);
            println!("  GetIRQ rc={rc} raw={v} ({v:#x})");
            let mut v = -1i32;
            let rc = NvAPI_GPU_GetBusId(h, &mut v as *mut i32 as *mut _);
            println!("  GetBusId rc={rc} raw={v} ({v:#x})");
            let mut v = -1i32;
            let rc = NvAPI_GPU_GetBusSlotId(h, &mut v as *mut i32 as *mut _);
            println!("  GetBusSlotId rc={rc} raw={v} ({v:#x})");
            // power-rail descriptor identities (rail 218 / id-0 rails)
            match gpu.power_rails() {
                Ok(rails) => {
                    for r in rails {
                        println!(
                            "  rail ch{}: id={} type={} name={} {}mW conf={:?}",
                            r.channel_bit, r.pwr_rail, r.channel_type, r.rail_name, r.pwr_mw, r.confidence
                        );
                    }
                }
                Err(e) => println!("  power_rails ERR = {e}"),
            }
        }
    }
}
