//! R465 (462.96) PerfPstatesGetInfoPrivate layout probe — READ-ONLY.
//!
//! The V4 (0x432D0, 275152 B) layout was RE'd on R610; 462.96 was never in
//! the audited branch set. get-pstate-lock decodes garbage there (min fields
//! carry pstate ids 8/5/3/2), so dump every accepted magic's raw record for
//! byte-level comparison.
//!
//! Run: cargo test -p nvapi --test pstates_private_r465_probe -- --nocapture --ignored

use std::io::Write as _;

use nvapi::PhysicalGpu;
use nvapi::sys::api::NvAPI_GPU_PerfPstatesGetInfoPrivate;

#[test]
#[ignore]
fn pstates_private_r465_dump() {
    nvapi::initialize().expect("init");
    let gpu = PhysicalGpu::enumerate()
        .expect("enumerate")
        .into_iter()
        .next()
        .expect("no gpu");
    eprintln!("GPU: {:?} handle={:?}", gpu.full_name(), *gpu.handle());

    let dump_dir = std::path::Path::new(r"D:\git-repo\nvoc\.zcode\pstate-dumps");
    std::fs::create_dir_all(dump_dir).expect("mkdir");

    // (tag, magic, len) — V4 native + the V4-alternate 0x832D0 the audit
    // mentions, then the legacy V3/V1 pair.
    for (tag, magic, len) in [
        ("v4_432d0", 0x432D0u32, 275152usize),
        ("v4_832d0", 0x832D0, 550000),
        ("v3_319c8", 0x319C8, 203208),
        ("v1_119c8", 0x119C8, 72136),
    ] {
        let mut buf = vec![0u8; len];
        buf[..4].copy_from_slice(&magic.to_ne_bytes());
        let status = unsafe {
            NvAPI_GPU_PerfPstatesGetInfoPrivate(*gpu.handle(), buf.as_mut_ptr() as *mut _)
        };
        eprintln!(
            "== {tag}: magic {magic:#x} len {len} -> status {status:#x} ({})",
            status as i32
        );
        if status != 0 {
            continue;
        }
        // Driver may rewrite the version dword — log what came back.
        eprintln!(
            "   returned version dword: {:#x}, +4 mask: {:#x}, +8 table_version byte: {:#x}",
            u32::from_ne_bytes(buf[..4].try_into().unwrap()),
            u32::from_ne_bytes(buf[4..8].try_into().unwrap()),
            buf[8]
        );
        let path = dump_dir.join(format!("{tag}.bin"));
        std::fs::write(&path, &buf).expect("write dump");
        eprintln!("   dumped {} bytes to {}", buf.len(), path.display());

        // Structured peek: first 48 non-zero dwords after the header.
        let mut shown = 0;
        for off in (8..0x400).step_by(4) {
            let dw = u32::from_ne_bytes(buf[off..off + 4].try_into().unwrap());
            if dw != 0 {
                eprintln!("   +{off:#07x}: {dw:#010x} ({dw})");
                shown += 1;
                if shown >= 48 {
                    break;
                }
            }
        }
        // Slot table @0x2114 / freq min @0x22C8 / max @0x22F0, strides
        // 0x2090 / 0x9C — only meaningful for the V4 layout, but harmless
        // to print for the others (context for the diff).
        for slot in 0..6usize {
            let slot_off = 0x2114 + slot * 0x2090;
            if slot_off + 4 > buf.len() {
                break;
            }
            let row: Vec<String> = (0..4usize)
                .map(|d| {
                    let min_off = 0x22C8 + slot * 0x2090 + d * 0x9C;
                    let max_off = 0x22F0 + slot * 0x2090 + d * 0x9C;
                    if max_off + 4 > buf.len() {
                        return "-".to_string();
                    }
                    let mn = u32::from_ne_bytes(buf[min_off..min_off + 4].try_into().unwrap());
                    let mx = u32::from_ne_bytes(buf[max_off..max_off + 4].try_into().unwrap());
                    format!("{mn}/{mx}")
                })
                .collect();
            eprintln!(
                "   slot{slot} pstate_dw@0x2114={:#x} min/max per domain: {}",
                u32::from_ne_bytes(buf[slot_off..slot_off + 4].try_into().unwrap()),
                row.join(" ")
            );
        }
        // Legacy record view (72 + 2252*bit): {type, min, max, pstate}
        for bit in 0..8usize {
            let base = 72 + 2252 * bit;
            if base + 16 > buf.len() {
                break;
            }
            let dw = |o: usize| u32::from_ne_bytes(buf[o..o + 4].try_into().unwrap());
            eprintln!(
                "   legacy bit{bit}: type={:#x} min={} max={:#x} pstate={}",
                dw(base),
                dw(base + 4),
                dw(base + 8),
                buf[base + 12]
            );
        }
        let _ = std::io::stderr().flush();
    }
}
