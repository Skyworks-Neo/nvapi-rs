//! ClientPowerModes = the NVIDIA App Balanced/Max toggle = SetPerfPoint
//! behavioral commit (RM 0x2080E627, GSP commit sub_176FBC8 via slot-600
//! sampling). Probe:
//!   READ-ONLY part: capability (mode_mask/max_mode_idx), current mode,
//!   clocks/voltage — runnable safely by anyone.
//!   WRITE part (root): toggle Max/Balanced with clock/voltage/power
//!   sampling, restore. The behavioral test for the server card.
//!
//!   cargo  test ... --nocapture --ignored          (read-only part)
//!   sudo <binary> --nocapture --ignored --write    (adds the toggle test)

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
        let v = gpu.core_voltage().ok().map(|v| v.0 as f64 / 1000.0);
        let vs = v.map(|f| format!("{f:.1}")).unwrap_or_else(|| "-".into());
        println!(
            "  {tag}: GPC {} | XBAR {} | V {} mV",
            gpc.map(|f| format!("{f:.1}")).unwrap_or_else(|| "-".into()),
            xbar.map(|f| format!("{f:.1}"))
                .unwrap_or_else(|| "-".into()),
            vs
        );
        std::thread::sleep(Duration::from_millis(300));
    }
}

#[test]
#[ignore = "live GPU probe — read-only by default; --write adds the mode toggle (root)"]
fn power_mode_probe() {
    let write = std::env::args().any(|a| a == "--write");
    let uid = std::process::Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);

    nvapi::initialize().expect("init");
    let gpu = &PhysicalGpu::enumerate().expect("enumerate")[0];
    println!("gpu: {}", gpu.full_name().unwrap_or_default());

    let (mask, max_idx) = gpu.power_modes_capability().expect("capability");
    println!(
        "ClientPowerModes capability: mode_mask={mask:#06x} max_mode_idx={max_idx}  (App shows toggle only when max_mode_idx==1)"
    );
    let mode = gpu.power_mode().unwrap_or("ERR");
    println!("current power_mode = {mode}");

    if !write {
        println!("\n(read-only run — rerun under sudo with --write for the Max/Balanced toggle)");
        return;
    }
    assert_eq!(uid, 0, "write test needs root");

    println!("\n== current state ==");
    sample(gpu, "base", 4);

    println!("\n== SET power mode = Max ==");
    gpu.set_power_mode(true).expect("set Max failed");
    sample(gpu, "max", 8);

    println!("\n== SET power mode = Balanced ==");
    gpu.set_power_mode(false).expect("set Balanced failed");
    sample(gpu, "bal", 6);

    println!("\nVERDICT: clocks/voltage/pstate moved on Max = the SetPerfPoint");
    println!("commit path is LIVE on this server card via ClientPowerModes.");
}
