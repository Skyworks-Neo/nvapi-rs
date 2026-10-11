//! TopRels ENVELOPE probe — the GPC:XBAR edge has FIVE ratio slots:
//! rec0  = 0xE660 (0.900, active — our walks wrote this; down works, up clamped)
//! rec10 = 0xCCC0 (0.800)
//! rec11 = 0x20000 (2.000)
//! rec12 = 0x13330 (1.200)
//! rec13 = 0x18000 (1.500)
//! Question: is the up-clamp governed by the envelope slots? Tests, in order:
//! 1. rec10 0xCCC0 -> 0xF333 (0.95): does the engine track rec10?
//! 2. restore rec10
//! 3. rec0 -> 0xF333 with rec11 (max) raised 0x20000 -> 0x30000 (3.0):
//! does lifting the envelope max unclamp rec0's upside?
//! 4. full restore.
//! sudo target/debug/deps/pro6000_toprels_envelope_live-<hash> --nocapture --ignored

use std::ptr;

use nvapi::PhysicalGpu;
use nvapi::sys::api::{
    NvAPI_GPU_ClockClkPropTopRelsGetControl, NvAPI_GPU_ClockClkPropTopRelsSetControl,
};
use nvapi::sys::gpu::clock::undocumented::NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_CONTROL;
use nvapi::sys::nvapi::NvVersion;
use std::process::{Child, Command};
use std::time::Duration;

const REC_BASE: usize = 0x60; // rest-relative record 0 base (rec i: +0x108*i)
const REC_RATIO: usize = 4; // ratio dword within a record

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
        Err(_) => LoadGuard(None),
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
        // d0 = bits 0-7 (base relations) + bits 10-13 (envelope records);
        // d1 (bits 32-63) would be out of range and -1s
        b.rest[0..4].copy_from_slice(&0x3CFFu32.to_le_bytes());
        b
    }
}

/// GET the full control block, returning the raw ratio dwords for the five slots.
fn get_all(gpu: &PhysicalGpu) -> Option<[u32; 5]> {
    unsafe {
        let mut ctl = seeded();
        let st =
            NvAPI_GPU_ClockClkPropTopRelsGetControl(*gpu.handle(), ptr::from_mut(&mut *ctl).cast());
        if st != 0 {
            println!("  GET rc={st}");
            return None;
        }
        let rd = |rec: usize| {
            let base = REC_BASE + 0x108 * rec + REC_RATIO;
            u32::from_le_bytes(ctl.rest[base..base + 4].try_into().unwrap())
        };
        Some([rd(0), rd(10), rd(11), rd(12), rd(13)])
    }
}

/// SET by rebuilding the block from `slots` and submitting.
fn set_all(gpu: &PhysicalGpu, slots: [u32; 5]) -> Option<()> {
    unsafe {
        let mut ctl = seeded();
        for (rec, raw) in [
            (0usize, slots[0]),
            (10, slots[1]),
            (11, slots[2]),
            (12, slots[3]),
            (13, slots[4]),
        ] {
            let base = REC_BASE + 0x108 * rec + REC_RATIO;
            ctl.rest[base..base + 4].copy_from_slice(&raw.to_le_bytes());
        }
        let st =
            NvAPI_GPU_ClockClkPropTopRelsSetControl(*gpu.handle(), ptr::from_ref(&*ctl).cast());
        println!("  SET rc={st}");
        (st == 0).then_some(())
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
#[ignore = "live GPU WRITE probe — root required; restores all five slots"]
fn toprels_envelope_probe() {
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

    let Some(snapshot) = get_all(gpu) else {
        panic!("baseline GET failed");
    };
    println!("slots [rec0, rec10, rec11, rec12, rec13] = {snapshot:#x?}");

    // ---- 1. rec10 0.8 -> 0.95 ---------------------------------------------
    println!("\n== 1. rec10: 0.8 -> 0.95 (does the engine track rec10?) ==");
    let mut s = snapshot;
    s[1] = 0xF333;
    set_all(gpu, s);
    if let Some(now) = get_all(gpu) {
        println!("  slots now = {now:#x?}");
    }
    sample(gpu, "rec10=0.95", 6);
    set_all(gpu, snapshot);
    println!("  rec10 restored");

    // ---- 2. rec0 0.95 with rec11 (max?) lifted to 3.0 ---------------------
    println!("\n== 2. rec0 -> 0.95 with rec11 (2.0 -> 3.0) ==");
    let mut s = snapshot;
    s[0] = 0xF333;
    s[2] = 0x30000;
    set_all(gpu, s);
    if let Some(now) = get_all(gpu) {
        println!("  slots now = {now:#x?}");
    }
    sample(gpu, "rec0-up", 6);
    set_all(gpu, snapshot);
    println!("  restored");

    // ---- final -------------------------------------------------------------
    println!("\n== final state ==");
    if let Some(now) = get_all(gpu) {
        println!("slots = {now:#x?} (snapshot was {snapshot:#x?})");
    }
    sample(gpu, "final", 3);

    println!("\nVERDICT GUIDE:");
    println!("  step1 XBAR -> 0.95x           : rec10 is a LIVE second control (two knobs!)");
    println!("  step1 clock pinned            : rec10 is not engine-tracked either");
    println!("  step2 XBAR -> 0.95x           : envelope max gates the active ratio —");
    println!("    lift via rec11 = the up-side unlock");
    println!("  step2 clock pinned            : 0.9 ceiling is engine-side policy,");
    println!("    not envelope-encoded — registry levers are the remaining path");
}
