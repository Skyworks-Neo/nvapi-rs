//! Live Msd dual-interface sampler: GetAllClocks[21] vs direct measure
//! (0x527FC458 +4=21) side by side, ~24 samples at 0.5 s — run UNDER an
//! NVENC encode (ffmpeg -f lavfi -i testsrc -c:v h264_nvenc -f null -) to
//! settle whether the GetAllClocks Msd slot really tracks the video clock on
//! RTX PRO 6000 Blackwell / 580.173.02, or only the direct interface does.
//! READ-ONLY.
//! cargo test -p nvapi --test pro6000_msd_load_live -- --nocapture --ignored

use nvapi::PhysicalGpu;

#[test]
#[ignore = "live GPU read-only Msd sampler (pair with an NVENC load)"]
fn msd_dual_interface_sampler() {
    nvapi::initialize().expect("init");
    let gpu = &PhysicalGpu::enumerate().expect("enumerate")[0];
    let hi = gpu;

    println!(" t  | GClocks[21] | direct(21) | direct(20) | direct(1) | direct(4)");
    println!("----+-------------+------------+------------+-----------+----------");
    for i in 0..24 {
        let all = hi.all_clocks().ok();
        let at = |bits: u32| -> String {
            all.as_ref()
                .and_then(|m| m.iter().find(|(id, _)| id.repr() == bits as i32))
                .map(|(_, khz)| format!("{:.1}", khz.0 as f64 / 1000.0))
                .unwrap_or_else(|| "-".into())
        };
        let dm = |bits: u32| -> String {
            // PhysicalGpu returns Ok(freq_khz) directly; 0 = refused/unmeasurable
            match hi.clk_domain_freq_direct(bits) {
                Ok(v) if v.freq_khz > 0 => format!("{:.1}", v.freq_khz as f64 / 1000.0),
                _ => "-".into(),
            }
        };
        println!(
            "{i:3} | {at21:>11} | {d21:>10} | {d20:>10} | {d1:>9} | {d4:>9}",
            at21 = at(21),
            d21 = dm(21),
            d20 = dm(20),
            d1 = dm(1),
            d4 = dm(4),
        );
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
