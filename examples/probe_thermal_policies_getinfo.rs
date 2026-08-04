//! Probe the PRIVATE ClientThermalPolicies GetInfo (ID 0x2F69F8E5) to confirm
//! the per-GPU target-temp policy index discovery, RE'd from GPUMonCmd
//! `GPUHandle::queryTargetTemperature` (sub_14002C410).
//!
//! Run: `cargo run --release -p nvapi --example probe_thermal_policies_getinfo`
//!
//! What it confirms:
//! - 0x2F69F8E5 resolves + the call returns OK with version magic 0x33D38.
//! - dword[2] packing: LOBYTE = GPS (target-temp) index, BYTE1 = acoustics.
//!   On this laptop GPS idx should be 2 (= the 87C wall); on a desktop GPS would
//!   be 0xFF and acoustics would carry the writable slot.
//! - VBIOS min/default/max (Q8) for the chosen index.

use nvapi::sys::api::{NvAPI_GPU_ClientThermalPoliciesGetInfo, NvAPI_GPU_ClientThermalPoliciesPrivateGetInfo};
use nvapi::sys::gpu::thermal::private::{NV_GPU_CLIENT_THERMAL_POLICIES_INFO, NV_GPU_CLIENT_THERMAL_POLICIES_PRIVATE_INFO};
use nvapi::sys::nvapi::StructVersion;
use nvapi::PhysicalGpu;

fn main() {
    nvapi::initialize().expect("NvAPI_Initialize failed");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.into_iter().next().expect("no GPU");

    let mut info = NV_GPU_CLIENT_THERMAL_POLICIES_PRIVATE_INFO::default();
    info.version =
        <NV_GPU_CLIENT_THERMAL_POLICIES_PRIVATE_INFO as StructVersion>::NVAPI_VERSION;

    // Debug: struct size + computed magic (GPUMon uses 0x33D58 = v3 | 15704).
    println!(
        "rust struct size = {} bytes, version magic = 0x{:X}",
        std::mem::size_of_val(&info),
        info.version.data
    );

    let status = unsafe { NvAPI_GPU_ClientThermalPoliciesPrivateGetInfo(*gpu.handle(), &mut info) };
    println!("PRIVATE GetInfo (0x2F69F8E5) status = {} (0 = OK)", status);

    // Compare against the documented GetInfo (0x0D258BB5, V3 struct 1400 B) to
    // isolate whether the -9 is handle-related or specific to the private ID.
    let mut doc = NV_GPU_CLIENT_THERMAL_POLICIES_INFO::default();
    doc.version = <NV_GPU_CLIENT_THERMAL_POLICIES_INFO as StructVersion>::NVAPI_VERSION;
    let doc_status =
        unsafe { NvAPI_GPU_ClientThermalPoliciesGetInfo(*gpu.handle(), &mut doc) };
    println!(
        "documented GetInfo (0x0D258BB5) status = {} (0 = OK), count = {}",
        doc_status, doc.count
    );

    if status != 0 {
        return;
    }

    let gps = info.gps_policy_index();
    let acoustic = info.acoustics_policy_index();
    println!(
        "dword[2] packed -> GPS idx = {:?}, acoustics idx = {:?}",
        gps, acoustic
    );
    let chosen = info.target_temp_policy_index();
    println!(
        "GPUMon-style chosen target-temp index = {:?}",
        chosen
    );

    if let Some(idx) = chosen {
        match info.target_temp_range(idx) {
            Some((min, default, max)) => {
                println!(
                    "VBIOS range for idx {} -> min={:.1} C, default={:.1} C, max={:.1} C",
                    idx, min, default, max
                );
            }
            None => println!("range read out of bounds for idx {}", idx),
        }
    }

    // Sanity: compare to the live target-temp policy scan (get-temp-thresholds).
    println!("\n=== live target-temp policy scan (for cross-check) ===");
    match gpu.target_temperature_policies() {
        Ok(policies) => {
            for (i, c) in policies {
                let tag = if Some(i as u8) == chosen {
                    "  <- chosen by GetInfo"
                } else {
                    ""
                };
                println!("  idx {} = {:.1} C{}", i, c, tag);
            }
        }
        Err(e) => println!("scan failed: {:?}", e),
    }

    // Dump every non-zero dword (index : value) so the full struct contents are
    // visible for RE. Q8 temperatures show up as celsius*256.
    println!("\n=== full non-zero dword dump (idx : value / Q8 temp) ===");
    let bytes = info.payload_bytes();
    let mut shown = 0;
    for i in (0..bytes.len() / 4).step_by(1) {
        let off = i * 4;
        if off + 4 > bytes.len() {
            break;
        }
        let v = u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap());
        if v != 0 {
            let q8 = v as i32 as f32 / 256.0;
            let q8tag = if q8 > 30.0 && q8 < 200.0 {
                format!("  (Q8 {:.1} C?)", q8)
            } else {
                String::new()
            };
            println!("  dword[{:4}] (byte {:5}): 0x{:08X} ({}){}", i + 1, off + 4, v, v, q8tag);
            shown += 1;
            if shown >= 80 {
                println!("  ... (truncated)");
                break;
            }
        }
    }
}
