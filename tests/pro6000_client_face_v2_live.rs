//! Face-C probe v2. First run: point40 write returned -1 — the shim gate
//! (sub_38B80) rejects [+0]=0 when the point's shadow type is EMPTY-class
//! (0/2/4 demand [+0]==1; populated 1/3/5/6/7 demand [+0]==0; types outside
//! 0..7 -> -180). So face-C only writes VALUES on populated-type points.
//!
//! v2: census all 255 shadow types via Client GET, then write +50 MHz on
//! every populated point found (one SET, batch), readback, clock samples,
//! restore to 0. Root required; everything restored.
//!
//!   sudo target/debug/deps/pro6000_client_face_v2_live-<hash> --nocapture --ignored

use nvapi::PhysicalGpu;
use nvapi::sys::api::{
    NvAPI_GPU_ClockClientClkVfPointsGetControl, NvAPI_GPU_ClockClientClkVfPointsSetControl,
};
use nvapi::sys::gpu::clock::undocumented::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL;
use nvapi::sys::nvapi::NvVersion;
use std::ptr;

const DELTA_KHZ: i32 = 50_000;

fn client_table(gpu: &PhysicalGpu) -> Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL> {
    unsafe {
        let mut ctl = {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL>::new_zeroed();
            b.assume_init()
        };
        ctl.version = NvVersion::with_version(0x12420);
        // seed the full 256-bit mask space
        for d in ctl.mask.mask.iter_mut() {
            *d = u32::MAX;
        }
        let st = NvAPI_GPU_ClockClientClkVfPointsGetControl(
            *gpu.handle(),
            ptr::from_mut(&mut *ctl).cast(),
        );
        assert_eq!(st, 0, "Client-GET rc={st}");
        ctl
    }
}

#[test]
#[ignore = "live GPU WRITE probe — root required; restores"]
fn client_face_v2() {
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

    let table = client_table(gpu);

    // census the shadow types
    let mut census: std::collections::BTreeMap<u32, Vec<usize>> = Default::default();
    for (i, p) in table.points.iter().enumerate() {
        census.entry(p.clock_type).or_default().push(i);
    }
    println!("== shadow type census (255 points) ==");
    for (ty, pts) in &census {
        println!(
            "  type {ty}: {} points {:?}{}",
            pts.len(),
            if pts.len() > 12 {
                &pts[..12]
            } else {
                pts.as_slice()
            },
            if pts.len() > 12 { "..." } else { "" }
        );
    }

    // populated class = {1,3,5,6,7} per the shim switch
    let populated: Vec<usize> = census
        .iter()
        .filter(|(ty, _)| [1, 3, 5, 6, 7].contains(ty))
        .flat_map(|(_, pts)| pts.iter().copied())
        .collect();
    println!(
        "populated-class points: {} -> {:?}",
        populated.len(),
        populated
    );

    if populated.is_empty() {
        println!("\nNO populated points — face-C has no writable value surface on this card.");
        println!("(empty-class claim write is a no-op per the shim: writes only the type byte)");
        return;
    }

    // build the batch SET: [+0]=0 (populated gate), [+20]=+50MHz, mask = populated set
    let mut ctl = {
        let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL>::new_zeroed();
        unsafe { b.assume_init() }
    };
    ctl.version = NvVersion::with_version(0x12420);
    for &i in &populated {
        ctl.mask.mask[i / 32] |= 1u32 << (i % 32);
        ctl.points[i].freqDeltaKHz = DELTA_KHZ;
    }
    let st = unsafe {
        NvAPI_GPU_ClockClientClkVfPointsSetControl(*gpu.handle(), ptr::from_ref(&*ctl).cast())
    };
    println!(
        "\n== Client SET {} points +{DELTA_KHZ} kHz (ver1): rc = {st} ==",
        populated.len()
    );
    if st != 0 {
        println!("face refused — paste me");
        return;
    }

    // readback
    let rd = client_table(gpu);
    let landed: Vec<i32> = populated
        .iter()
        .map(|&i| rd.points[i].freqDeltaKHz)
        .collect();
    println!("readback deltas: {:?}...", &landed[..landed.len().min(8)]);

    // behavioral sample
    for _ in 0..6 {
        let gpc = gpu
            .clk_domain_freq_direct(0)
            .ok()
            .map(|v| v.freq_khz as f64 / 1000.0);
        let xbar = gpu
            .clk_domain_freq_direct(1)
            .ok()
            .map(|v| v.freq_khz as f64 / 1000.0);
        println!(
            "  live: GPC {} | XBAR {}",
            gpc.map(|f| format!("{f:.1}")).unwrap_or_else(|| "-".into()),
            xbar.map(|f| format!("{f:.1}"))
                .unwrap_or_else(|| "-".into())
        );
        std::thread::sleep(std::time::Duration::from_millis(300));
    }

    // restore: zero the deltas through the same face
    for &i in &populated {
        ctl.points[i].freqDeltaKHz = 0;
    }
    let st = unsafe {
        NvAPI_GPU_ClockClientClkVfPointsSetControl(*gpu.handle(), ptr::from_ref(&*ctl).cast())
    };
    let rd = client_table(gpu);
    let after: Vec<i32> = populated
        .iter()
        .map(|&i| rd.points[i].freqDeltaKHz)
        .collect();
    println!(
        "restore rc = {st}, readback = {:?}...",
        &after[..after.len().min(8)]
    );
    println!(
        "\n(cross-check: nvoc-cli get-private-vftable offset column — populated points +50 = faces share storage)"
    );
}
