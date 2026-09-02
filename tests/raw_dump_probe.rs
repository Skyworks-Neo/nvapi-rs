// Probe: does clk_vf_points_private_raw actually attach raw records?
// Run with: cargo test -p nvapi --test raw_dump_probe -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;

#[test]
#[ignore]
fn raw_records_probe() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    for (i, gpu) in gpus.iter().enumerate() {
        println!("--- GPU {i} {} ---", gpu.full_name().unwrap_or_default());
        match gpu.clk_vf_points_private_raw() {
            Ok(vfp) => {
                println!(
                    "points={} raw_records={}",
                    vfp.points.len(),
                    vfp.raw_records.len()
                );
                if let Some(r) = vfp.raw_records.first() {
                    println!(
                        "first record bank{} idx{} len={}",
                        r.bank,
                        r.index,
                        r.bytes.len()
                    );
                }
            }
            Err(e) => println!("err: {e:?}"),
        }
    }
}
