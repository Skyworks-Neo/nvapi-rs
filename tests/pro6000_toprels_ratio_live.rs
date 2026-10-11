//! TopRels GPC:XBAR ratio walk v2 — RAW sys calls with the PROVEN minimal
//! seed: +4 = 0xFF ONLY (blackwell_recon_live seed A, rc=0, ratio 0xE660
//! @abs 0x68). The hi-level top_rels_control_seeded() uses the 616.92-era
//! full seeding (seed_mask: +4 + rec0 + 32B@+8) which this driver rejects
//! (-104); recon seed B already showed the fuller seeding failing.
//!
//! Walk 0.9 -> 0.95 -> 1.0 -> 0.9 under NVENC load; direct-measure GPC/XBAR.
//! Root required; restores the original ratio at the end.
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

const RATIO_OFF: usize = 0x64; // rest-relative; absolute 0x68 in the buffer

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

fn seeded() -> Box<NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_CONTROL> {
    unsafe {
        let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_CONTROL>::new_zeroed();
        let mut b = b.assume_init();
        b.version = NvVersion::with_version(0x1075C);
        b.rest[0..4].copy_from_slice(&0xFFu32.to_le_bytes()); // seed A ONLY
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
        panic!("baseline GET failed — paste me");
    };
    println!(
        "current ratio raw = {original:#x} ({:.4})",
        original as f64 / 65536.0
    );
    let _load = spawn_load();

    println!("\n== baseline @ {original:#x} ==");
    sample(gpu, "r0.90", 4);

    for raw in [0xF333u32, 0x10000] {
        // 0.95, 1.0
        println!(
            "\n== SET ratio raw {raw:#x} ({:.4}) ==",
            raw as f64 / 65536.0
        );
        match set_ratio(gpu, raw) {
            Some(rb) => println!("  landed = {rb:#x} ({:.4})", rb as f64 / 65536.0),
            None => {
                println!("  SET/GET failed — restoring original");
                set_ratio(gpu, original);
                continue;
            }
        }
        sample(gpu, &format!("r{:.2}", raw as f64 / 65536.0), 6);
    }

    println!("\n== restore {original:#x} ==");
    match set_ratio(gpu, original) {
        Some(rb) => println!("  restore ok, landed = {rb:#x}"),
        None => println!("  restore FAILED — MANUAL CHECK: blackwell_recon_live GetControl"),
    }
    sample(gpu, "r0.90", 3);

    println!("\nVERDICT GUIDE:");
    println!("  XBAR tracked 0.95/1.0 -> RATIO PATH LIVE: XBAR OC via TopRels SET_CONTROL;");
    println!("    dead slot offsets were ratio-clamped — retest with headroom");
    println!("  landed moves but clock pinned -> fourth inert face; GSP RE continues");
    println!("  SET refused -> paste rc; face gated beyond the seed fix");
}
