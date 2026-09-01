// Volta (V100/GV100) VoltRails family probe — WHY is get-volt-rail-info
// "Supported: no" on this GPU?
//
// Production path: VoltVoltRailsGetInfo (0x2C73AFDC) with the V2 stamp
// ((2<<16)|6220) → IncompatibleStructVersion → map_legacy_struct_version
// → NotSupported → CLI "Supported: no". The known-legacy stamp is V1
// ((1<<16)|0xACC = 68300, 2764B) whose layout is unmapped.
//
// This probe sweeps candidate INFO stamps and, on the first accepted one,
// hexdumps the response head — distinguishing "struct-version gate, older
// layout exists (worth mapping)" from "family truly absent on Volta".
// Read-only (GetInfo only).
//
// Run: cargo test -p nvapi --test volta_voltrails_probe -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;
use nvapi::sys::api::NvAPI_GPU_VoltVoltRailsGetInfo;
use nvapi::sys::gpu::power::undocumented::NV_GPU_VOLT_RAILS_INFO;
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

#[test]
#[ignore]
fn volta_voltrails_stamp_sweep() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    eprintln!("GPU: {:?}", gpu.full_name());

    // (raw version stamp, tag). magic = (version<<16) | sizeof.
    let candidates: &[(u32, &str)] = &[
        ((2 << 16) | 6220, "V2 size6220 (production stamp)"),
        ((1 << 16) | 0xACC, "V1 size2764 (documented legacy)"),
        ((1 << 16) | 6220, "V1 size6220 (hybrid)"),
        ((3 << 16) | 6220, "V3 size6220 (Blackwell-era guess)"),
    ];
    for &(stamp, tag) in candidates {
        // oversized zeroed buffer: handlers that version-gate reject before
        // writing; handlers that fill, fill into it
        let mut info = unsafe {
            let b = Box::<NV_GPU_VOLT_RAILS_INFO>::new_zeroed();
            let mut b = b.assume_init();
            b.version = NvVersion::with_version(stamp);
            b
        };
        let st = unsafe {
            NvAPI_GPU_VoltVoltRailsGetInfo(*gpu.handle(), ptr::from_mut(&mut *info).cast())
        };
        eprintln!("GetInfo stamp={stamp:#x} ({tag}): status={:#x}", st as i32);
        if st == 0 {
            // rail mask = second dword (after version)
            let mask = u32::from_le_bytes(info.rest[0..4].try_into().unwrap());
            eprintln!("  ACCEPTED — rail_mask=0x{mask:08X}");
            hexdump(&info.rest, 0x100);
        }
    }
}

use core::ptr;
