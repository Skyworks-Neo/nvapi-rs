//! End-to-end live test for the private target-temp (温度墙) interface.
//!
//! Run:
//!   `cargo run --release --example probe_target_temp`              # read idx 2
//!   `cargo run --release --example probe_target_temp -- <celsius>`  # write idx 2
//!   `cargo run --release --example probe_target_temp -- <C> <idx>`  # write custom idx
//!
//! Defaults to policy_index = 2 — the RTX 4060 Laptop "GPU Target Temperature"
//! policy (confirmed via nvidia-smi cross-check: idx 2 reads 87C = the wall).
//! idx 0/1/4 are other thermal policies (slowdown/etc), idx 3/5/6/7 invalid.
//!
//! Cross-check persistence with: `nvidia-smi -q -d TEMPERATURE` →
//! "GPU Target Temperature" should match the value written here.

use nvapi::PhysicalGpu;

fn main() {
    let arg: Option<f32> = std::env::args().nth(1).and_then(|s| s.parse().ok());
    let policy_index: usize = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2); // RTX 4060 Laptop target-temp policy index

    nvapi::initialize().expect("NvAPI_Initialize failed");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.into_iter().next().expect("no GPU");

    println!("policy_index = {}", policy_index);

    // 1. Read current wall.
    match gpu.target_temperature(policy_index) {
        Ok(Some(c)) => println!("current target temp = {:.2} C", c),
        Ok(None) => println!("current target temp = <index out of range>"),
        Err(e) => {
            println!("GET-prime failed: {:?}", e);
            return;
        }
    }

    // 2. If a target was given, SET it and re-GET to confirm.
    if let Some(target) = arg {
        println!("setting target temp = {:.2} C ...", target);
        match gpu.set_target_temperature(target, policy_index) {
            Ok(()) => println!("SET returned OK"),
            Err(e) => {
                println!("SET failed: {:?}", e);
                return;
            }
        }
        match gpu.target_temperature(policy_index) {
            Ok(Some(c)) => println!("post-SET target temp = {:.2} C  ({})", c,
                if (c - target).abs() < 1.0 { "PERSISTED ✓" } else { "MISMATCH ✗" }),
            Ok(None) => println!("post-GET: index out of range"),
            Err(e) => println!("post-GET failed: {:?}", e),
        }
        println!("\nverify with: nvidia-smi -q -d TEMPERATURE");
    }
}
