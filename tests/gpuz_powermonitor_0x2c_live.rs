// GPU-Z 2.71 audit E-⑪ probe (gpuz-sensor-audit.md §12): verify the
// channel-ID × 0x2C status-buffer indexing rule GPU-Z 2.71 uses
// (sub_654390: `imul ecx, [eax + idx*4], 0x2C` into the 0x1059C status
// buffer) against our current ch0/+0x44 + heuristic-offset model.
//
// Read-only. Run on a machine with a live GPU:
//   cargo test -p nvapi --release --test gpuz_powermonitor_0x2c_live -- --ignored --nocapture
//
// Expected backfill: if the driver accepts v1|0x59C and the live values sit
// on 0x2C*idx boundaries (ch0 == +0x44 slot), the 0x2C rule generalizes and
// `disambiguate_power_rails` can drop its heuristics (audit §10 ⑪).

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;

#[test]
#[ignore]
fn power_monitor_0x2c_indexing_probe() {
    use nvapi::sys::api::NvAPI_GPU_PowerMonitorGetStatus;

    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    eprintln!("=== GPU-Z E-11: PowerMonitor 0x2C indexing probe ===");

    // 0x59C = 1436 bytes — GPU-Z's exact status-buffer size (stamp 0x1059C).
    const GPUZ_STATUS_SIZE: usize = 0x59C;
    let mut buf = vec![0u8; GPUZ_STATUS_SIZE];
    // Stamp the header like GPU-Z's sub_623A60: +0x00 = ver1|0x59C.
    buf[0..4].copy_from_slice(&0x0001_059Cu32.to_le_bytes());

    let st =
        unsafe { NvAPI_GPU_PowerMonitorGetStatus(*gpu.handle(), buf.as_mut_ptr().cast()) as i32 };
    eprintln!("GetStatus v1|0x59C: status={st:#x} ({st})");
    if st != 0 {
        eprintln!("  driver refused GPU-Z's exact stamp — 0x2C rule cannot apply here");
        return;
    }

    let words =
        unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u32, GPUZ_STATUS_SIZE / 4) };
    eprintln!(
        "  header words: {:#010x} {:#010x} {:#010x} {:#010x}",
        words[0], words[1], words[2], words[3]
    );

    eprintln!("  nonzero dwords at 0x2C-aligned offsets:");
    for (i, &w) in words.iter().enumerate() {
        if w != 0 && i * 4 % 0x2C == 0 {
            eprintln!("    +{:#05x} = {:#010x} ({})", i * 4, w, w);
        }
    }
    eprintln!("  known heuristic offsets for comparison:");
    for off in [0x14usize, 0x2C, 0x44, 0x80, 0x98, 0xE0, 0xEC, 0x14C] {
        let i = off / 4;
        if i < words.len() {
            eprintln!("    +{off:#05x} = {:#010x}", words[i]);
        }
    }
    eprintln!("  Verdict guide: ch0 at +0x44 must equal the matching 0x2C*idx slot for");
    eprintln!("  the 0x2C rule to hold; per-rail descriptor offsets (GPUZ_OFFSET_LABELS)");
    eprintln!("  should fall on 0x2C*idx boundaries for their channel index.");
}
