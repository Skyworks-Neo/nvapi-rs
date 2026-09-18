//! Live read-only power-policy state snapshot, mapped onto the RM kernel
//! object model (NvpwrControl 616.92 audit §2 — see the doc block in
//! `sys/src/gpu/power.rs`):
//!   LOWER / MAX ceiling / F7-side current + the F7=min(C+A,U) formula
//!   annotation and a stock-vs-enforced classification. GET-only.
//!
//! Run: cargo test -p nvapi --test power_policy_state_live -- --ignored --nocapture
//! Optional: NVOC_GPU_IDX=<n> selects the GPU (default 0).

use nvapi::{PhysicalGpu, initialize};

#[test]
#[ignore = "live GPU read-only"]
fn power_policy_state() {
    let idx: usize = std::env::var("NVOC_GPU_IDX")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    initialize().expect("NvAPI init failed");
    let gpus = PhysicalGpu::enumerate().expect("enumerate failed");
    let gpu = gpus.get(idx).expect("gpu index out of range");
    println!("gpu: {}", gpu.full_name().unwrap_or_default());

    let range = match gpu.tgp_watt_range() {
        Ok(Some(r)) => r,
        Ok(None) => {
            println!("tgp_watt_range: not supported on this platform");
            return;
        }
        Err(e) => {
            println!("tgp_watt_range: {e}");
            return;
        }
    };
    println!(
        "policy[{idx}]: LOWER={:?} mW  default(slider)={:?} mW  MAX={:?} mW",
        range.min_mw, range.default_mw, range.max_mw,
    );

    match gpu.tgp_watt_status() {
        Ok(Some(st)) => {
            println!("F7-side current: {:?} mW", st.current_mw);
            match (st.current_mw, range.max_mw) {
                (Some(0), _) => println!(
                    "state: NO LIVE REQUEST (current=0 — desktop/slider-unused path;\n         the TGP request slot only populates after a TGP-watt SET)"
                ),
                (Some(cur), Some(max)) if cur >= max => println!(
                    "state: AT-CEILING — kernel stock coherent shape (base==UPPER, F7==UPPER;\n         F7 = min(C+A, U) with A=0 ⇒ F7=U)"
                ),
                (Some(cur), Some(max)) => println!(
                    "state: ENFORCED {delta_mw} mW below MAX (slider/DB carving; F7 = min(C+A, U) < U)",
                    delta_mw = max - cur
                ),
                _ => println!("state: indeterminate (missing range/current value)"),
            }
        }
        Ok(None) => println!("F7-side current: reset sentinel / not requested"),
        Err(e) => println!("tgp_watt_status: {e}"),
    }

    match gpu.power_limit() {
        Ok(pl) => println!("public power limit: {pl:?}"),
        Err(e) => println!("public power limit: {e}"),
    }
    println!("\n(done — nothing was written; ceilings have no user-mode exit, audit §6)");
}
