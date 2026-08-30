// Diagnostic: probe the legacy V1 (R391-era) FanPolicies family stamps.
// IDA (391.35 nvapi64.dll): GetInfo handler 0x180111F10 accepts stamp 0x1003C
// (60B), GetControl handler 0x180111650 accepts 0x10038 (56B). Both route
// through RM escape 0x07000038 cmd 0x20800532 (232B fanPolicyInfo internal
// table: +0 policy mask ≤16 bits, +4 active-policy index byte, then per-policy
// records with two flag bytes).
//
// V1 output layout (from handler decode):
//   INFO  60B: +0 stamp, +4 flags(bit0=has-entries), +8 count, +0xC entries[]
//             each 12B: {+0 policy_id?, +4 active(1 if idx==active), +8 flags
//             (bit0/bit1 from per-policy record bytes 13/14)}
//   CONTROL 56B: same family, per-policy flag fetch (no curve points).
//
// Run with: cargo test --test fan_policy_v1_probe -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;
use nvapi::sys::nvapi::NvVersion;

/// Dump nonzero dwords with byte-exact hex.
fn dump(label: &str, bytes: &[u8]) {
    eprintln!("  {} ({}B) nonzero dwords:", label, bytes.len());
    let mut any = false;
    for (i, chunk) in bytes.chunks(4).enumerate() {
        if chunk.len() < 4 {
            break;
        }
        let v = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        if v != 0 {
            any = true;
            eprintln!("    +{:02x}: {:#010x}", i * 4, v);
        }
    }
    if !any {
        eprintln!("    (all zero)");
    }
    eprintln!("  raw first 64B: {:02x?}", &bytes[..bytes.len().min(64)]);
}

#[test]
#[ignore]
fn fan_policy_v1_probe() {
    use nvapi::sys::api::{
        NvAPI_GPU_ClientFanPoliciesGetControl, NvAPI_GPU_ClientFanPoliciesGetInfo,
    };
    use nvapi::sys::gpu::cooler::undocumented::{
        NV_GPU_CLIENT_FAN_POLICIES_CONTROL, NV_GPU_CLIENT_FAN_POLICIES_INFO,
    };

    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    eprintln!("=== FanPolicies V1 probe ===");

    // --- GetInfo: V2 stamp (0x2004C) vs legacy V1 (0x1003C) ---
    for (label, magic) in [("V2 0x2004C", 0x2004Cu32), ("V1 0x1003C", 0x1003Cu32)] {
        let mut info = NV_GPU_CLIENT_FAN_POLICIES_INFO::new();
        info.version = NvVersion::with_version(magic).data;
        let st = unsafe { NvAPI_GPU_ClientFanPoliciesGetInfo(*gpu.handle(), &mut info) };
        eprintln!("GetInfo {:10}: status={:#x}", label, st as i32);
        if st == 0 {
            dump("info payload", &info.data);
        }
    }

    // --- GetControl: V2 stamp (0x200DC) vs legacy V1 (0x10038) ---
    for (label, magic) in [("V2 0x200DC", 0x200DCu32), ("V1 0x10038", 0x10038u32)] {
        let mut ctrl = NV_GPU_CLIENT_FAN_POLICIES_CONTROL::new();
        ctrl.version = NvVersion::with_version(magic).data;
        let st = unsafe { NvAPI_GPU_ClientFanPoliciesGetControl(*gpu.handle(), &mut ctrl) };
        eprintln!("GetControl {:10}: status={:#x}", label, st as i32);
        if st == 0 {
            // The struct is repr(C) with version:u32 first; view the whole
            // thing as bytes (V1 driver extent = 56B).
            let bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(
                    (&ctrl as *const _) as *const u8,
                    std::mem::size_of::<NV_GPU_CLIENT_FAN_POLICIES_CONTROL>(),
                )
            };
            dump("ctrl raw (V1 extent=56B)", bytes);
        }
    }
}
