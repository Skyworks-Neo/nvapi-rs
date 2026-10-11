//! TopRels ratio + ClkDomains slot offset COMBINED probe.
//!
//! Envelope (v4): the ratio is a live ceiling — engine applies
//! XBAR = ratio x GPC clamped to a ~884 MHz frequency floor; anything above
//! the devinit 0.9 lands in the record but the clock pins at 0.9x.
//! This probe tests whether the ClkDomains slot-offset plane (xbar slot2)
//! feeds the clock WITHIN the live window: set ratio 0.7 (XBAR ~1700), then
//! slot2 +100 MHz (expect ~1800 if the slot plane feeds; unchanged if the
//! slot plane is inert independent of ratio). Also probes slot2 -100 MHz in
//! the same window. Root; everything restored.
//!
//! sudo target/debug/deps/pro6000_toprels_ratio_live-<hash> --nocapture --ignored

use std::ptr;

use nvapi::PhysicalGpu;
use nvapi::sys::api::{
    NvAPI_GPU_ClockClkPropTopRelsGetControl, NvAPI_GPU_ClockClkPropTopRelsSetControl,
};
use nvapi::sys::gpu::clock::undocumented::NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_CONTROL;
use nvapi::sys::nvapi::NvVersion;
use std::process::{Child, Command};
use std::time::Duration;

const RATIO_OFF: usize = 0x64;

struct LoadGuard(Option<Child>);

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
            "150",
            "-f",
            "null",
            "/dev/null",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => {
            println!("load: ffmpeg spawned (pid {})", c.id());
            std::thread::sleep(Duration::from_secs(3));
            LoadGuard(Some(c))
        }
        Err(_) => {
            println!("load: ffmpeg unavailable — external load only");
            LoadGuard(None)
        }
    }
}

impl Drop for LoadGuard {
    fn drop(&mut self) {
        if let Some(c) = self.0.as_mut() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

fn seeded() -> Box<NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_CONTROL> {
    unsafe {
        let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_CONTROL>::new_zeroed();
        let mut b = b.assume_init();
        b.version = NvVersion::with_version(0x1075C);
        b.rest[0..4].copy_from_slice(&0xFFu32.to_le_bytes());
        b
    }
}

fn get_ratio(gpu: &PhysicalGpu) -> Option<u32> {
    unsafe {
        let mut ctl = seeded();
        let st =
            NvAPI_GPU_ClockClkPropTopRelsGetControl(*gpu.handle(), ptr::from_mut(&mut *ctl).cast());
        if st != 0 {
            println!("  GET rc={st}");
            return None;
        }
        Some(u32::from_le_bytes(
            ctl.rest[RATIO_OFF..RATIO_OFF + 4].try_into().ok()?,
        ))
    }
}

fn set_ratio(gpu: &PhysicalGpu, raw: u32) -> Option<u32> {
    unsafe {
        let mut ctl = seeded();
        ctl.rest[RATIO_OFF..RATIO_OFF + 4].copy_from_slice(&raw.to_le_bytes());
        let st =
            NvAPI_GPU_ClockClkPropTopRelsSetControl(*gpu.handle(), ptr::from_ref(&*ctl).cast());
        println!("  ratio SET rc={st}");
        if st != 0 {
            return None;
        }
        get_ratio(gpu)
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
        let volt = gpu
            .core_voltage()
            .ok()
            .map(|v| format!("{:.1} mV", v.0 as f64 / 1000.0));
        let live = match (xbar, gpc) {
            (Some(x), Some(g)) if g > 0.0 => format!("ratio={:.4}", x / g),
            _ => "ratio=-".into(),
        };
        println!(
            "  {tag}: GPC {} | XBAR {} | {live} | V {}",
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
#[ignore = "live GPU WRITE probe — root required; restores ratio and slot offsets"]
fn ratio_plus_slot_combo() {
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
    let _load = spawn_load();

    // ---- step 1: ratio 0.7 ------------------------------------------------
    println!("\n== 1. ratio = 0.7 ==");
    let Some(rb) = set_ratio(gpu, 0xB333) else {
        println!("ratio SET failed — abort");
        return;
    };
    println!("  ratio landed = {rb:#x}");
    sample(gpu, "r0.70", 3);

    // ---- step 2: xbar slot2 +100 MHz inside the live window ---------------
    println!("\n== 2. xbar slot2 = +100 MHz (inside the 0.7 window) ==");
    let before = slot_value(gpu, 1, 2).unwrap_or(0);
    match gpu.set_clk_domain_offset(1, 100_000, 2, false) {
        Ok(_) => println!("  slot2 SET ok, readback = {:?}", slot_value(gpu, 1, 2)),
        Err(e) => println!("  slot2 SET ERR: {e}"),
    }
    sample(gpu, "slot+100", 6);
    gpu.set_clk_domain_offset(1, before, 2, false)
        .expect("slot2 restore failed");
    println!("  restored slot2 = {:?}", slot_value(gpu, 1, 2));

    // ---- step 3: xbar slot2 -100 MHz (same window) ------------------------
    println!("\n== 3. xbar slot2 = -100 MHz (same window) ==");
    gpu.set_clk_domain_offset(1, -100_000, 2, false)
        .expect("slot2 SET failed");
    sample(gpu, "slot-100", 6);
    gpu.set_clk_domain_offset(1, before, 2, false)
        .expect("slot2 restore failed");
    println!("  restored slot2 = {:?}", slot_value(gpu, 1, 2));

    // ---- step 4: restore ratio 0.9 ----------------------------------------
    println!("\n== 4. restore ratio 0.9 ==");
    match set_ratio(gpu, 0xE660) {
        Some(rb) => println!("  ratio landed = {rb:#x}"),
        None => println!("  ratio restore FAILED — MANUAL CHECK"),
    }
    sample(gpu, "final", 3);

    println!("\nVERDICT GUIDE:");
    println!("  XBAR moved with slot offsets inside the 0.7 window  -> the slot plane");
    println!("    FEEDS the clock but was ceiling-masked at 0.9; combined control =");
    println!("    ratio (ceiling) + slot offset (trim) — a working XBAR control pair");
    println!("  XBAR unmoved at +/-100 MHz inside the window         -> slot plane is");
    println!("    inert independent of ratio; only the ratio itself is behavioral");
}
