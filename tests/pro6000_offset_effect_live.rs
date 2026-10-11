//! Root-required offset-EFFECT probe for the RTX PRO 6000 Blackwell server
//! card. Root SET succeeds (values stick in the records) but the user
//! reports slots 2/3/4 have zero behavioral effect. This probe separates
//! "record holds a value" from "the clock actually moves", using NEGATIVE
//! offsets (a down-offset cannot be clamped by any max):
//!
//!   xbar slot2 -100 MHz  -> XBAR direct-measure should drop ~2182 -> ~2082
//!   gpc  slot2 -100 MHz  -> GPC direct-measure should drop ~2422 -> ~2322
//!
//! Spawns an NVENC load (ffmpeg h264_nvenc, if installed) so the clocks sit
//! at a stable loaded point; NVOC_NO_LOAD=1 to rely on an external load.
//! Every write is restored to its snapshot value and re-verified.
//!
//!   sudo -E cargo test -p nvapi --test pro6000_offset_effect_live -- --nocapture --ignored

use nvapi::PhysicalGpu;
use std::process::{Child, Command};
use std::time::Duration;

fn euid() -> u32 {
    std::process::Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

struct LoadGuard(Option<Child>);

impl Drop for LoadGuard {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn spawn_load() -> LoadGuard {
    if std::env::var("NVOC_NO_LOAD").is_ok() {
        println!("load: skipped (NVOC_NO_LOAD) — provide your own load in another terminal");
        return LoadGuard(None);
    }
    match Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=3840x2160:rate=120",
            "-c:v",
            "h264_nvenc",
            "-preset",
            "p7",
            "-t",
            "120",
            "-f",
            "null",
            "/dev/null",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => {
            println!("load: ffmpeg h264_nvenc 4K120 spawned (pid {})", child.id());
            std::thread::sleep(Duration::from_secs(3));
            LoadGuard(Some(child))
        }
        Err(_) => {
            println!("load: ffmpeg unavailable — running without load (NVOC_NO_LOAD to silence)");
            LoadGuard(None)
        }
    }
}

fn sample(gpu: &PhysicalGpu, label: &str, n: usize) {
    for _ in 0..n {
        let gpc = gpu
            .clk_domain_freq_direct(0)
            .ok()
            .map(|v| v.freq_khz as f64 / 1000.0);
        let xbar = gpu
            .clk_domain_freq_direct(1)
            .ok()
            .map(|v| v.freq_khz as f64 / 1000.0);
        let uproc = gpu
            .clk_domain_freq_direct(20)
            .ok()
            .map(|v| v.freq_khz as f64 / 1000.0);
        println!(
            "  {label}: GPC {} | XBAR {} | uProc {}",
            fmt(gpc),
            fmt(xbar),
            fmt(uproc)
        );
        std::thread::sleep(Duration::from_millis(300));
    }
}

fn fmt(v: Option<f64>) -> String {
    v.map(|f| format!("{f:.1}")).unwrap_or_else(|| "-".into())
}

fn slot_value(gpu: &PhysicalGpu, bit: u32, slot: usize) -> Option<i32> {
    let ctl = gpu.clk_domains_control().ok()?;
    ctl.entries
        .iter()
        .find(|e| e.bit == bit)
        .and_then(|e| e.values_kHz.get(slot).copied())
}

#[test]
#[ignore = "live GPU WRITE probe — root required; restores every write"]
fn offset_effect_probe() {
    let uid = euid();
    assert_eq!(
        uid, 0,
        "run under sudo — the SET path needs RM admin (-137 otherwise)"
    );

    nvapi::initialize().expect("init");
    let gpu = &PhysicalGpu::enumerate().expect("enumerate")[0];
    println!("gpu: {}", gpu.full_name().unwrap_or_default());
    let _load = spawn_load();

    println!("\n== baseline (loaded) ==");
    sample(gpu, "base ", 4);

    // ---- XBAR slot2 -100 MHz --------------------------------------------
    println!("\n== XBAR slot2 = -100 MHz ==");
    let before = slot_value(gpu, 1, 2).unwrap_or(0);
    println!("  snapshot slot2 = {before}");
    gpu.set_clk_domain_offset(1, -100_000, 2, false)
        .expect("xbar slot2 SET failed under root — unexpected");
    println!("  write ok, readback slot2 = {:?}", slot_value(gpu, 1, 2));
    sample(gpu, "xbar-", 8);
    gpu.set_clk_domain_offset(1, before, 2, false)
        .expect("xbar restore failed");
    println!("  restored slot2 = {:?}", slot_value(gpu, 1, 2));

    // ---- GPC slot2 -100 MHz ----------------------------------------------
    println!("\n== GPC slot2 = -100 MHz ==");
    let before = slot_value(gpu, 0, 2).unwrap_or(0);
    println!("  snapshot slot2 = {before}");
    gpu.set_clk_domain_offset(0, -100_000, 2, false)
        .expect("gpc slot2 SET failed under root — unexpected");
    println!("  write ok, readback slot2 = {:?}", slot_value(gpu, 0, 2));
    sample(gpu, "gpc-", 8);
    gpu.set_clk_domain_offset(0, before, 2, false)
        .expect("gpc restore failed");
    println!("  restored slot2 = {:?}", slot_value(gpu, 0, 2));

    // ---- final integrity --------------------------------------------------
    println!("\n== final: all fabric slots must be back to snapshot ==");
    for bit in [0u32, 1] {
        let vals: Vec<Option<i32>> = (0..8).map(|s| slot_value(gpu, bit, s)).collect();
        println!("  domain {bit}: {vals:?}");
    }

    println!("\nVERDICT GUIDE:");
    println!("  XBAR dropped ~100 MHz while written  -> offsets ARE effective here;");
    println!("    the earlier 'no effect' was clamp/policy (e.g. TopRels 0.9 ratio on up-offsets)");
    println!("  XBAR unchanged while record holds -100000 -> GSP accepts-but-ignores:");
    println!("    the record table is detached from the clock tree on this SKU (GSP RE next)");
}
