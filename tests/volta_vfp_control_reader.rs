// V100: validate the production clk_vf_control_private reader on the
// legacy layout (snapshot-magic GET + 17B mask seed + rec+0x24 decode).
// Read-only. Expect 132 points, all values 0 at stock.
//
// Run: cargo test -p nvapi --test volta_vfp_control_reader -- --nocapture --ignored

#![allow(unused_must_use)]

#[test]
#[ignore]
fn volta_vfp_control_reader_legacy() {
    nvapi::initialize().expect("init");
    let gpus = nvapi::PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    eprintln!("GPU: {:?}", gpu.full_name());

    let ctrl = gpu.clk_vf_control_private().expect("control read failed");
    eprintln!("points: {}", ctrl.points.len());
    let nonzero: Vec<_> = ctrl.points.iter().filter(|p| p.value != 0).collect();
    eprintln!("nonzero values: {}", nonzero.len());
    for p in &ctrl.points[..5.min(ctrl.points.len())] {
        eprintln!(
            "  bank {} idx {}: mode {} value {}",
            p.bank, p.index, p.mode, p.value
        );
    }
    for p in nonzero.iter().take(10) {
        eprintln!(
            "  NONZERO bank {} idx {}: mode {} value {}",
            p.bank, p.index, p.mode, p.value
        );
    }
    assert_eq!(ctrl.points.len(), 132, "expected 132 legacy control points");
    assert!(nonzero.is_empty(), "stock control should be all-zero");
}
