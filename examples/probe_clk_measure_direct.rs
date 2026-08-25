//! Live probe for the DIRECT clock-frequency measure path (ID 0x527FC458),
//! green-curve-main's XBar/SYS measurement API — distinct from the
//! counter-based `ClockCounterMeasureAvgFreq` (0xFB8F61EC) already wrapped as
//! `clk_domain_freq`. This probe exercises `clk_domain_freq_direct` across the
//! known measure domains and, for each one that succeeds, compares the direct
//! kHz against the two-sample Δcounter/Δt MHz the counter variant computes.
//! Agreement is independent third-party corroboration that the 12-byte direct
//! struct (magic 0x0001000C) and the 65,568-byte counter struct (0x10020)
//! read the same underlying RM clock state through different sub-families.
//!
//! Run: `cargo run --release -p nvapi --example probe_clk_measure_direct`

use nvapi::PhysicalGpu;

fn main() {
    if let Err(e) = nvapi::initialize() {
        eprintln!("nvapi init failed: {e}");
        return;
    }

    let gpus = PhysicalGpu::enumerate().unwrap_or_default();
    let gpu = match gpus.into_iter().next() {
        Some(g) => g,
        None => {
            eprintln!("no NVIDIA GPU handle returned by EnumPhysicalGPUs");
            let _ = nvapi::unload();
            return;
        }
    };
    let name = gpu.full_name().unwrap_or_default();
    println!("GPU: {name}");
    println!();

    // Sequential domain INDEX shared by both measure sub-families
    // (green-curve self_test_clk_domain_survey, RTX 5070 / 610.88):
    //   GPC=0, XBAR=1, SYS=2, MCLK=4.  VIDEO (entry 4) has NO measure domain.
    let domains: &[(u32, &str)] = &[
        (0, "GPC"),
        (1, "XBAR"),
        (2, "SYS"),
        (4, "MCLK"),
    ];

    println!(
        "{:<6} {:>14} {:>16} {:>10}",
        "domain", "direct_kHz(0x527FC458)", "counter_MHz(0xFB8F61EC)", "agree?"
    );
    println!("{:-<50}", "");

    for &(bit, label) in domains {
        let direct = gpu.clk_domain_freq_direct(bit);
        let counter = gpu.clk_domain_freq(bit);

        let (dkhz, dstatus): (u32, String) = match direct {
            Ok(d) => (d.freq_khz, "ok".to_string()),
            Err(ref e) => (0, e.to_string()),
        };
        let (cmhz, cstatus): (f64, String) = match counter {
            Ok(c) => (c.freq_mhz, "ok".to_string()),
            Err(ref e) => (0.0, e.to_string()),
        };

        let agree = if dstatus == "ok" && cstatus == "ok" && dkhz > 0 {
            let direct_mhz = dkhz as f64 / 1000.0;
            // direct is one instantaneous sample; counter averages over ~50 ms.
            // Treat <2% drift as agreement (DVFS idle/boost swing).
            if direct_mhz > 0.0 && cmhz > 0.0 {
                let drift = (direct_mhz - cmhz).abs() / direct_mhz.max(cmhz);
                if drift < 0.02 {
                    "yes"
                } else {
                    "drift"
                }
            } else {
                "?"
            }
        } else {
            "n/a"
        };

        println!(
            "{:<6} {:>14}   {:>14.3}   {:>8}",
            label,
            if dstatus == "ok" {
                format!("{dkhz}")
            } else {
                dstatus
            },
            if cstatus == "ok" { cmhz } else { 0.0 },
            agree
        );
    }

    let _ = nvapi::unload();
}


