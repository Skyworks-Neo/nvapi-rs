//! TopRels ratio walk v3 — DOWN-walk + transition-triggered application test.
//!
//! v2 result: ver1 SET lands (0xF333/0x10000 readback exact) but XBAR stays
//! at ~0.898x GPC under stable load. Two hypotheses:
//!   H1  upside clamp — lowering the ratio moves the clock (user suggestion)
//!   H2  transition-applied — the ratio is only re-evaluated when GPC
//!       re-clocks; stable-load GPC never transitions, so the stored ratio
//!       is never re-read. Test: set 0.8, kill the load (GPC downclocks),
//!       respawn (GPC re-upclocks) — if XBAR returns at 0.8x, the path is
//!       LIVE but transition-applied.
//!
//!   sudo target/debug/deps/pro6000_toprels_ratio_live-<hash> --nocapture --ignored

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

fn spawn_load() -> (LoadGuard, bool) {
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
            "180",
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
            (LoadGuard(Some(c)), true)
        }
        Err(_) => (LoadGuard(None), false),
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
        println!("  SET rc={st}");
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
#[ignore = "live GPU WRITE probe — root required; restores the original ratio"]
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

    let Some(original) = get_ratio(gpu) else {
        panic!("baseline GET failed");
    };
    println!(
        "current ratio raw = {original:#x} ({:.4})",
        original as f64 / 65536.0
    );
    let (load1, had1) = spawn_load();

    // ---- H1: DOWN-walk 0.8, 0.7 ------------------------------------------
    for raw in [0xCCCCu32, 0xB333] {
        let r = raw as f64 / 65536.0;
        println!("\n== H1 DOWN: SET ratio {r:.2} ==");
        match set_ratio(gpu, raw) {
            Some(rb) => println!("  landed = {rb:#x} ({:.4})", rb as f64 / 65536.0),
            None => {
                println!("  failed — restoring");
                set_ratio(gpu, original);
                continue;
            }
        }
        sample(gpu, &format!("r{r:.2}"), 6);
    }

    // ---- H2: transition-triggered application ----------------------------
    println!("\n== H2: ratio 0.8 + GPC transition (load drop -> respawn) ==");
    if let Some(rb) = set_ratio(gpu, 0xCCCC) {
        println!("  ratio landed = {rb:#x}");
    }
    sample(gpu, "pre-drop", 2);
    drop(load1);
    if had1 {
        println!("  -- load dropped, GPC downclocking --");
    }
    sample(gpu, "downclock", 6);
    println!("  -- respawning load, GPC re-upclocks --");
    {
        let (_g2, _) = spawn_load();
        sample(gpu, "reup", 8);
    }

    // ---- restore ---------------------------------------------------------
    println!("\n== restore {original:#x} ==");
    match set_ratio(gpu, original) {
        Some(rb) => println!("  restore ok, landed = {rb:#x}"),
        None => println!("  restore FAILED — MANUAL CHECK via blackwell_recon_live GET"),
    }
    sample(gpu, "final", 3);

    println!("\nVERDICT GUIDE:");
    println!("  DOWN-walk moved XBAR                     -> H1: path live, upside-clamped;");
    println!("    slot offsets were clamped by the ratio — raise ratio first, then slots");
    println!("  XBAR returned at 0.8x after transition   -> H2: transition-applied;");
    println!("    writes need a GPC re-clock to take effect (toggle load/pstate)");
    println!("  neither                                  -> record inert on this build");
}
