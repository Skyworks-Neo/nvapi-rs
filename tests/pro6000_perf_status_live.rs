//! PerfPoliciesGetStatus RAW dump — attributing NVAPI perf-flag bit 0x20.
//! GetInfo says the SUPPORTED set is 0x3F (six bits incl. NVIDIA's
//! "UNKNOWN_32"); the host parser (sub_6E830) maps RM response mask@+32
//! positions identity-style onto the six NVAPI bits, so bit 0x20 = RM reason
//! position 5. This probe dumps the RAW STATUS response at idle and under an
//! auto-spawned NVENC load — whichever raw position lights alongside the
//! known bits attributes the reason empirically. READ-ONLY.
//!
//!   sudo target/debug/deps/pro6000_perf_status_live-<hash> --nocapture --ignored

use nvapi::PhysicalGpu;
use nvapi::sys::api::NvAPI_GPU_PerfPoliciesGetStatus;
use nvapi::sys::gpu::power::undocumented::NV_GPU_PERF_POLICIES_STATUS_PARAMS;
use nvapi::sys::nvapi::NvVersion;
use std::process::{Child, Command};

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
            "90",
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
            std::thread::sleep(std::time::Duration::from_secs(3));
            LoadGuard(Some(c))
        }
        Err(_) => LoadGuard(None),
    }
}

/// version dword of the STATUS struct = (1<<16) | 0x550
const STATUS_MAGIC: u32 = 0x1_0550;

fn dump(tag: &str, gpu: &PhysicalGpu) {
    unsafe {
        let mut st = {
            let b = Box::<NV_GPU_PERF_POLICIES_STATUS_PARAMS>::new_zeroed();
            b.assume_init()
        };
        st.version = NvVersion::with_version(STATUS_MAGIC);
        let rc = NvAPI_GPU_PerfPoliciesGetStatus(*gpu.handle(), ptr::from_mut(&mut *st).cast());
        println!("== {tag} (rc={rc}) ==");
        if rc != 0 {
            return;
        }
        // known header fields
        println!(
            "  flags={:#x} limits={:#x} unknown(policies)={:#x}",
            st.flags, st.limits.value, st.unknown
        );
        // raw dword scan over the padding tail: every nonzero dword
        // (offset base = 4 ver + 4 flags + 8 timer + 4 limits + 4 zero0
        //  + 4 unknown + 4 zero1 + 24 timers = 56)
        for (i, v) in st.padding.data.iter().enumerate() {
            if *v != 0 {
                println!("  +{:#05x}: {v:#010x} ({v})", 56 + i * 4);
            }
        }
    }
}

use std::ptr;

#[test]
#[ignore = "live GPU READ-ONLY probe — pairs with a load for bit attribution"]
fn perf_status_raw() {
    nvapi::initialize().expect("init");
    let gpu = &PhysicalGpu::enumerate().expect("enumerate")[0];
    println!("gpu: {}", gpu.full_name().unwrap_or_default());

    println!("\n===== IDLE =====");
    dump("idle-1", gpu);
    std::thread::sleep(std::time::Duration::from_millis(500));
    dump("idle-2", gpu);

    let _load = spawn_load();
    println!("\n===== LOAD =====");
    for i in 0..3 {
        dump(&format!("load-{i}"), gpu);
        std::thread::sleep(std::time::Duration::from_millis(400));
    }
    println!("\n(read the loaded dump: whichever reason-mask position lights");
    println!(" alongside bit 0x20 in limitSupport names UNKNOWN_32's reason)");
}
