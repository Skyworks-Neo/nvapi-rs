//! Public OC face probe — the faces we NEVER tested on this server card:
//!   1. Pstates20 SET (public SetPstates20 RMW, "RTX 50 core path"):
//!      P0 GPC +50 MHz -> readback -> restore. GET/SET cascade ver2/ver3.
//!   2. NVML GPC/MEM VF offsets (RM 0x2080D031 on 580): read -> write
//!      +50/-50 -> readback -> restore 0.
//! Each step self-verifies (the helpers assert readback) and restores.
//!   sudo target/debug/deps/pro6000_public_oc_live-<hash> --nocapture --ignored

use nvapi::PhysicalGpu;

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

use std::time::Duration;

#[test]
#[ignore = "live GPU WRITE probe — root required; restores every write"]
fn public_oc_faces() {
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

    // ---- 1. Pstates20 SET: P0 GPC +50 MHz ---------------------------------
    println!("\n== 1. Pstates20 SET: P0 GPC +50 MHz ==");
    match gpu.set_pstate_clock_offset(0, nvapi::ClockDomain::Graphics, 50_000) {
        Ok(rb) => println!("  SET ok, readback delta = {rb} kHz"),
        Err(e) => println!("  SET ERR: {e}"),
    }
    sample(gpu, "p20+50", 6);
    println!("  restore: P0 GPC offset -> 0");
    match gpu.set_pstate_clock_offset(0, nvapi::ClockDomain::Graphics, 0) {
        Ok(rb) => println!("  restore ok, readback = {rb} kHz"),
        Err(e) => println!("  restore ERR: {e} — MANUAL CHECK via get-pstates"),
    }
    sample(gpu, "p20-restore", 3);

    println!("\n(NVML VF-offset face 0x2080D031 is tested separately via ctypes —");
    println!(" nvapi-rs does not wrap NVML writes)");

    println!("\nVERDICT GUIDE:");
    println!("  Pstates20 moved GPC 50 MHz -> PUBLIC OC face live: nvoc's standard OC");
    println!("    path works on this server card; only the PRIVATE faces are inert");
    println!("  refused/ignored            -> server SKU locks even the public faces");
}
