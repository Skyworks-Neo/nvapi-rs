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

    // Probe sets: domain:slot pairs. Slot 2/3 proven dead on this SKU
    // (prior run) — the open question is whether the server GSP policy
    // still attaches the LEGACY planes 0 (freq) / 1 (volt). Override via
    // NVOC_PROBE_SETS="domain:slot,domain:slot".
    let sets: Vec<(u32, usize)> = std::env::var("NVOC_PROBE_SETS")
        .unwrap_or_else(|_| "1:0,0:0".into())
        .split(',')
        .filter_map(|p| {
            let (d, s) = p.trim().split_once(':')?;
            Some((d.parse().ok()?, s.parse().ok()?))
        })
        .collect();

    for (domain, slot) in sets {
        println!("\n== domain {domain} slot{slot} = -100 MHz ==");
        let before = slot_value(gpu, domain, slot).unwrap_or(0);
        println!("  snapshot slot{slot} = {before}");
        gpu.set_clk_domain_offset(domain, -100_000, slot as u32, false)
            .unwrap_or_else(|e| panic!("domain {domain} slot{slot} SET failed: {e}"));
        println!("  write ok, readback = {:?}", slot_value(gpu, domain, slot));
        sample(gpu, &format!("d{domain}s{slot}-"), 8);
        gpu.set_clk_domain_offset(domain, before, slot as u32, false)
            .unwrap_or_else(|e| panic!("domain {domain} slot{slot} restore failed: {e}"));
        println!(
            "  restored slot{slot} = {:?}",
            slot_value(gpu, domain, slot)
        );
    }

    // ---- final integrity --------------------------------------------------
    println!("\n== final: all fabric slots must be back to snapshot ==");
    for bit in [0u32, 1] {
        let vals: Vec<Option<i32>> = (0..8).map(|s| slot_value(gpu, bit, s)).collect();
        println!("  domain {bit}: {vals:?}");
    }

    println!("\nVERDICT GUIDE:");
    println!("  clock dropped ~100 MHz while written  -> THAT plane is attached on this SKU;");
    println!("    nvoc's Blackwell slot map must become platform-skewed (server = legacy planes)");
    println!("  every plane unchanged while records hold values -> GSP accepts-but-ignores:");
    println!("    the record table is detached from the clock tree on this SKU (GSP RE next)");
}
