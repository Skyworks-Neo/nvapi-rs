//! Slot + VF-point write matrix at the devinit ratio 0.9 — xbar (bit 1):
//! A. ClkDomains slot2 (+/-100 MHz), slot3 (+/-100 mV), slot4 (+/-100 raw)
//! B. PRIVATE VF table xbar region (points 180..189): mode0 freq +50 MHz
//! per point, then mode1 raw 1000 per point — each write self-verifies
//! landing (the helper asserts readback == input).
//! Clock/voltage sampled around every write; everything restored.
//!
//! sudo target/debug/deps/pro6000_slot_vf_matrix_live-<hash> --nocapture --ignored

use nvapi::PhysicalGpu;
use std::time::Duration;

fn sample(gpu: &PhysicalGpu, tag: &str, n: usize) {
    for _ in 0..n {
        let gpc = gpu
            .clk_domain_freq_direct(0)
            .ok()
            .map(|v| v.freq_khz as f64 / 1000.0);
        let xbar = gpu
            .clk_domain_freq_direct(1)
            .ok()
            .map(|v| v.freq_khz as f64 / 1000.0);
        let volt = gpu
            .core_voltage()
            .ok()
            .map(|v| format!("{:.1}", v.0 as f64 / 1000.0));
        println!(
            "  {tag}: GPC {} | XBAR {} | V {} mV",
            gpc.map(|f| format!("{f:.1}")).unwrap_or_else(|| "-".into()),
            xbar.map(|f| format!("{f:.1}"))
                .unwrap_or_else(|| "-".into()),
            volt.unwrap_or_else(|| "-".into())
        );
        std::thread::sleep(Duration::from_millis(300));
    }
}

fn slot_value(gpu: &PhysicalGpu, bit: u32, slot: usize) -> Option<i32> {
    let ctl = gpu.clk_domains_control().ok()?;
    ctl.entries
        .iter()
        .find(|e| e.bit == bit)
        .and_then(|e| e.values_kHz.get(slot).copied())
}

#[test]
#[ignore = "live GPU WRITE probe — root required; restores everything"]
fn slot_vf_matrix() {
    let uid = std::process::Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    assert_eq!(uid, 0, "run under sudo");

    nvapi::initialize().expect("init");
    let gpu = &PhysicalGpu::enumerate().expect("enumerate")[0];
    println!("gpu: {}", gpu.full_name().unwrap_or_default());

    let ratio = gpu
        .top_rels_ratio()
        .map(|(_, raw)| format!("{raw:#x}"))
        .unwrap_or_else(|e| format!("ERR {e}"));
    println!("ratio (untouched this run): {ratio}");

    // ---- A. ClkDomains slot walk on xbar ----------------------------------
    for (slot, unit, step) in [
        (2usize, "MHz", 100_000i32),
        (3, "mV", 100_000),
        (4, "raw", 100_000),
    ] {
        for sign in [1i32, -1] {
            let v = step * sign;
            println!("\n== A: xbar slot{slot} = {sign:+}{step} {unit} ==");
            let before = slot_value(gpu, 1, slot).unwrap_or(0);
            match gpu.set_clk_domain_offset(1, v, slot as u32, false) {
                Ok(_) => println!(
                    "  SET ok, readback = {:?} (was {before})",
                    slot_value(gpu, 1, slot)
                ),
                Err(e) => {
                    println!("  SET ERR: {e}");
                    continue;
                }
            }
            sample(gpu, &format!("s{slot}{v:+}"), 4);
            gpu.set_clk_domain_offset(1, before, slot as u32, false)
                .expect("restore failed");
            println!("  restored = {:?}", slot_value(gpu, 1, slot));
        }
    }

    // ---- B1. VF table xbar region, mode0 freq +50 MHz ----------------------
    println!("\n== B1: VF points 180..189 mode0 freq +50 MHz ==");
    let mut landed = Vec::new();
    for idx in 180..190 {
        match gpu.set_vfp_point_private(0, idx, true, 50_000) {
            Ok(v) => landed.push((idx, v)),
            Err(e) => println!("  point{idx} ERR: {e}"),
        }
    }
    println!("  landed (idx, kHz): {landed:?}");
    sample(gpu, "vf+50", 6);
    match gpu.reset_vfp_private(0, None) {
        Ok(n) => println!("  restore: reset {n} points"),
        Err(e) => println!("  restore ERR: {e}"),
    }
    sample(gpu, "vf-restored", 3);

    // ---- B2. VF table xbar region, mode1 raw 1000 --------------------------
    println!("\n== B2: VF points 180..189 mode1 raw 1000 ==");
    let mut landed = Vec::new();
    for idx in 180..190 {
        match gpu.set_vfp_point_private(0, idx, false, 1000) {
            Ok(v) => landed.push((idx, v)),
            Err(e) => println!("  point{idx} ERR: {e}"),
        }
    }
    println!("  landed (idx, raw): {landed:?}");
    sample(gpu, "vf-raw", 6);
    match gpu.reset_vfp_private(0, None) {
        Ok(n) => println!("  restore: reset {n} points"),
        Err(e) => println!("  restore ERR: {e}"),
    }
    sample(gpu, "vf-restored", 3);

    println!("\nVERDICT GUIDE:");
    println!("  any slot step moved XBAR/V          -> that plane is live at 0.9;");
    println!("  VF mode0 landed but clocks pinned   -> points land, engine inert;");
    println!("  VF mode1 landed (i16) but garbled   -> i16 truncation confirmed live;");
    println!("  everything restored to +0.0         -> matrix clean");
}
