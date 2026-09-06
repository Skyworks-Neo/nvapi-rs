//! Live dump for the nvClocks.spec P2 batch (2026-09-06 audit) — READ-ONLY.
//! NAFLL devices, ClkFreqController envelope, HWFS config, thermal slowdown.
//!
//! Run with:
//!   cargo test -p nvapi --test p2_batch_live -- --nocapture --ignored
#![allow(unused_must_use)]

use nvapi::PhysicalGpu;

#[test]
#[ignore]
fn p2_batch_live_dump() {
    nvapi::initialize().expect("NvAPI init failed");
    let gpus = PhysicalGpu::enumerate().expect("enumerate failed");
    let gpu = gpus.first().expect("no gpu");
    println!("=== GPU: {} ===", gpu.full_name().unwrap_or_default());

    // ---------------- NAFLL devices (P2) ----------------
    println!("\n########## 0x2BC9F805 ClockNafllDevicesGetInfo (P2) ##########");
    let nafll_mask = match gpu.nafll_devices_info() {
        Ok(None) => {
            println!("  no NAFLL devices (mask 0)");
            0
        }
        Ok(Some(info)) => {
            println!("  mask: {:#010x}", info.mask);
            for d in &info.entries {
                println!(
                    "  nafll[{}] type={} domain={} steps={} mode={} base_mhz={} b1c={:#x}",
                    d.bit, d.device_type, d.domain_id, d.steps, d.mode, d.base_mhz, d.b1c
                );
            }
            info.mask
        }
        Err(e) => {
            println!("  ERROR: {e}");
            0
        }
    };

    println!("\n########## 0xAFA4113C ClockNafllDevicesGetStatus (P2) ##########");
    if nafll_mask == 0 {
        println!("  skipped");
    } else {
        match gpu.nafll_devices_status() {
            Ok(None) => println!("  none"),
            Ok(Some(st)) => {
                for e in &st.entries {
                    let preview: Vec<_> = e.table.iter().copied().take(10).collect();
                    println!(
                        "  nafll[{}] tail={:#04x} ladder({} entries): {:?}{}",
                        e.bit,
                        e.tail_flag,
                        e.table.len(),
                        preview,
                        if e.table.len() > 10 { "..." } else { "" }
                    );
                }
                println!("  (static across samples = VF/oltage ladder, unit unclosed)");
            }
            Err(e) => println!("  ERROR: {e}"),
        }
    }

    // ---------------- ClkFreqController (P2) ----------------
    println!("\n########## 0x58F4F4C1 ClockClkFreqControllerGetInfo (P2) ##########");
    match gpu.clk_freq_controller_info() {
        Ok(None) => println!("  none"),
        Ok(Some(info)) => {
            println!("  mask: {:#010x}", info.mask);
            for e in &info.entries {
                println!(
                    "  fctrl[{}] supported={} type={} steps={} max_khz={} offset_range=[{}, {}]",
                    e.bit, e.supported, e.ctype, e.steps, e.max_khz, e.min_offset, e.max_offset
                );
            }
        }
        Err(e) => println!("  ERROR: {e}"),
    }

    // ---------------- HWFS (P2) ----------------
    println!("\n########## 0x14277C24 ThermalHwFsGetInfo (P2) ##########");
    for sel in 0u32..6 {
        match gpu.hwfs_control_get(sel) {
            Ok(h) => println!(
                "  sel={sel}: out_byte={:#04x} out_a={:#010x} out_b={} out_c={}",
                h.out_byte, h.out_a, h.out_b, h.out_c
            ),
            Err(e) => println!("  sel={sel}: ERROR {e}"),
        }
    }

    // ---------------- Thermal slowdown (P2) ----------------
    println!("\n########## 0x6683EE65 GetThermalSlowdownState (P2) ##########");
    match gpu.thermal_slowdown_state() {
        Ok(s) => println!(
            "  state: {s} ({})",
            if s == 0 {
                "normal"
            } else if s == 0xFFFF {
                "THERMALLY SLOWED"
            } else {
                "unknown"
            }
        ),
        Err(e) => println!("  ERROR: {e}"),
    }

    println!("\n(done — read-only; no SET call was made)");
}
