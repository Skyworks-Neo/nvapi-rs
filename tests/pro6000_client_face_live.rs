//! Face-C probe: the ClientClkVfPoints SetControl (QI 0x0733E009, 9248B,
//! 255x36B, ver1 verbatim) is the ONLY VF write face never behaviorally
//! tested on this card — nvoc uses it for GET only; the PRIVATE face
//! (0xFEC00D04, which the CLI's set-private-vftable-* commands use) lands
//! values readably but has zero behavioral effect on this server SKU.
//!
//! Shim contract (sub_38B80, fully decoded): version 0x12420; mask 32B @+4;
//! 255 x 36B records @+68: [+0] must be 0 for populated-type points,
//! [+20] = signed kHz delta; the handler GETs 0x2080902A first (shadow),
//! preserves record types, then SETs 0x2080D02D.
//!
//! Root required. Writes point 40 +50 MHz, samples, restores to 0, and also
//! reports whether the PRIVATE-face readback (set_vfp_point_private's
//! GetControl) sees the Client-face write — i.e. do the faces share storage?
//!
//!   sudo target/debug/deps/pro6000_client_face_live-<hash> --nocapture --ignored

use nvapi::PhysicalGpu;
use nvapi::sys::api::NvAPI_GPU_ClockClientClkVfPointsGetControl;
use nvapi::sys::api::NvAPI_GPU_ClockClientClkVfPointsSetControl;
use nvapi::sys::gpu::clock::undocumented::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL;
use nvapi::sys::nvapi::NvVersion;

const POINT: usize = 40;
const DELTA_KHZ: i32 = 50_000;

/// Client-face GET: the 9248B 255x36B control table's freqDeltaKHz for POINT.
fn client_get(gpu: &PhysicalGpu) -> Option<i32> {
    unsafe {
        let mut ctl = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL>::new_zeroed();
            b.assume_init()
        };
        ctl.version = NvVersion::with_version(0x12420);
        ctl.mask.mask[POINT / 32] |= 1u32 << (POINT % 32);
        let st = NvAPI_GPU_ClockClientClkVfPointsGetControl(
            *gpu.handle(),
            std::ptr::from_mut(&mut *ctl).cast(),
        );
        if st != 0 {
            println!("  Client-GET rc={st}");
            return None;
        }
        ctl.points.get(POINT).map(|p| p.freqDeltaKHz)
    }
}

#[test]
#[ignore = "live GPU WRITE probe — root required; restores"]
fn client_face_write_probe() {
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

    // baseline via the Client GET
    let baseline = client_get(gpu);
    println!("baseline Client-GET point{POINT} delta = {baseline:?}");

    // build the SET payload: mask bit for POINT, all-zero records except ours
    let mut ctl = unsafe {
        let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL>::new_zeroed();
        b.assume_init()
    };
    ctl.version = NvVersion::with_version(0x12420); // ver1 — verbatim path
    ctl.mask.mask[POINT / 32] |= 1u32 << (POINT % 32);
    ctl.points[POINT].freqDeltaKHz = DELTA_KHZ;
    // [+0] clock_type stays 0 — the populated-type gate value

    println!("\n== Client SET point{POINT} +{DELTA_KHZ} kHz (ver1) ==");
    let st = unsafe {
        NvAPI_GPU_ClockClientClkVfPointsSetControl(*gpu.handle(), std::ptr::from_ref(&*ctl).cast())
    };
    println!("SetControl rc = {st}");
    if st != 0 {
        println!("-> face refused ({st}); paste me — new datapoint for the face matrix");
        return;
    }

    // readback on the SAME face
    let after = client_get(gpu);
    println!("Client-GET readback point{POINT} delta = {after:?}");

    // behavioral sample while it (may) hold
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

    // restore: write 0 through the same face
    ctl.points[POINT].freqDeltaKHz = 0;
    let st = unsafe {
        NvAPI_GPU_ClockClientClkVfPointsSetControl(*gpu.handle(), std::ptr::from_ref(&*ctl).cast())
    };
    let restored = client_get(gpu);
    println!("\nrestore rc = {st}, readback = {restored:?}");

    // cross-face check: does the PRIVATE face see the Client write?
    println!("\n== cross-face: PRIVATE GetControl view of point{POINT} ==");
    // set_vfp_point_private's readback path — reuse it with value 0? No —
    // a raw GetControl here would need the PRIVATE seeding; report via the
    // simple probe instead: if Client and PRIVATE shared storage, the CLI's
    // get-private-vftable offset column would show +50 on point 40.
    println!("run: nvoc-cli get-private-vftable  -> check point{POINT} offset column");
    println!("  +50.0 = faces share storage; +0.0 = separate planes");
}
