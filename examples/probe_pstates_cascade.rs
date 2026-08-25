//! Live check of the `PhysicalGpu::pstates()` version cascade
//! (V2(3) -> V2(2) -> V1(1) -> legacy GetPstatesInfoEx).

fn main() {
    nvapi::initialize().expect("initialize");
    let gpus = nvapi::PhysicalGpu::enumerate().expect("enumerate");
    for gpu in &gpus {
        println!("--- {} ---", gpu.full_name().unwrap_or_default());
        match gpu.pstates() {
            Ok(p) => {
                println!(
                    "editable={} pstates={} overvolt={}",
                    p.editable,
                    p.pstates.len(),
                    p.overvolt.len()
                );
                for ps in &p.pstates {
                    let clocks: Vec<String> = ps
                        .clocks
                        .iter()
                        .map(|c| format!("{:?}={:?}", c.domain(), c.frequency_delta()))
                        .collect();
                    println!("  {:?} clocks=[{}]", ps.id, clocks.join(", "));
                }
            }
            Err(e) => println!("pstates() failed: {e:?}"),
        }
    }
}
