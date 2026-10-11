//! TopRels GPC:XBAR ratio path — the one live-confirmed Blackwell-specific
//! policy knob (ratio record 0xE660=0.9 @rec+0x68, GetControl works on this
//! card, SetControl 0xEF3D20EA never tested). If the ratio is writable AND
//! behavioral, XBAR tracking moves off 0.9x GPC — and the "dead" xbar slot
//! offsets may have been ratio-clamped all along.
//!
//! Sequence (root; auto-restore on mismatch is built into the helper):
//!   0.9 baseline  -> 0.95 -> 1.0 -> back to 0.9, direct-measure GPC/XBAR
//!   at each step under an auto-spawned NVENC load.
//!
//!   sudo target/debug/deps/pro6000_toprels_ratio_live-<hash> --nocapture --ignored

use nvapi::PhysicalGpu;
use std::process::{Child, Command};
use std::time::Duration;

struct LoadGuard(Option<Child>);

impl Drop for LoadGuard {
    fn drop(&mut self) {
        if let Some(c) = self.0.as_mut() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

fn spawn_load() -> LoadGuard {
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
        Ok(c) => {
            println!("load: ffmpeg h264_nvenc spawned (pid {})", c.id());
            std::thread::sleep(Duration::from_secs(3));
            LoadGuard(Some(c))
        }
        Err(_) => {
            println!("load: ffmpeg unavailable — external load only");
            LoadGuard(None)
        }
    }
}

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
        let live = match (xbar, gpc) {
            (Some(x), Some(g)) if g > 0.0 => format!("ratio={:.4}", x / g),
            _ => "ratio=-".into(),
        };
        println!(
            "  {tag}: GPC {} | XBAR {} | {live}",
            gpc.map(|f| format!("{f:.1}")).unwrap_or_else(|| "-".into()),
            xbar.map(|f| format!("{f:.1}"))
                .unwrap_or_else(|| "-".into())
        );
        std::thread::sleep(Duration::from_millis(300));
    }
}

#[test]
#[ignore = "live GPU WRITE probe — root required; auto-restores"]
fn toprels_ratio_walk() {
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

    let current = gpu
        .top_rels_ratio()
        .map(|(_off, raw)| format!("raw={raw:#x} ({:.4})", raw as f64 / 65536.0))
        .unwrap_or_else(|e| format!("ERR {e}"));
    println!("current TopRels GPC:XBAR ratio = {current}");
    let _load = spawn_load();

    println!("\n== baseline @ 0.9 ==");
    sample(gpu, "r0.90", 4);

    for step in [0.95f64, 1.0] {
        println!("\n== SET ratio {step} ==");
        match gpu.set_top_rels_ratio(step) {
            Ok(rb) => println!("  SET ok, readback ratio = {rb:.4}"),
            Err(e) => {
                println!("  SET ERR: {e}");
                continue;
            }
        }
        sample(gpu, &format!("r{:.2}", step), 6);
    }

    println!("\n== restore 0.9 ==");
    match gpu.set_top_rels_ratio(0.9) {
        Ok(rb) => println!("  restore ok, readback = {rb:.4}"),
        Err(e) => println!("  restore ERR: {e} — MANUAL CHECK via top_rels_ratio"),
    }
    sample(gpu, "r0.90", 3);

    println!("\nVERDICT GUIDE:");
    println!("  XBAR tracked 0.95/1.0 steps  -> RATIO PATH IS LIVE: XBAR OC via TopRels");
    println!("    SET_CONTROL works where the slot/point faces are store-only;");
    println!("    then retest slot2 offsets WITH ratio headroom (clamp theory)");
    println!("  readback moved but clock pinned -> ratio stored, engine ignores:");
    println!("    fourth inert face — GSP RE continues");
    println!("  SET refused (-x)             -> paste status; face gated differently");
}
