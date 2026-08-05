//! Controlled long-lived probe: does force_gc6_exit independently wake an
//! idle/GCOFF dGPU, WITHOUT relying on dashboard polling keep-alive?
//!
//! Run: `cargo run --release -p nvapi --example probe_gc6_idle_wake -- <secs>`
//!
//! Background: GUI fails NVAPI ops with -220 after idle because the dGPU enters
//! GCOFF. Dashboard high-frequency polling (0.3s) suppresses GCOFF as a
//! "symptom-fix" (keeps the driver too busy to power down). This probe tests
//! whether force_gc6_exit / gc6_force_wake can wake the GPU on demand after it
//! has naturally powered down — the "root-cause fix" that would not need
//! continuous polling.
//!
//! Protocol (single process, no background polling — lets the GPU idle freely):
//!   1. initialize + enumerate (GPU likely awake right after enumerate).
//!   2. sleep <secs> with NO nvapi/nvml calls → let the dGPU enter GCOFF.
//!   3. probe a read op (pstates) — expect -220 if GCOFF took hold.
//!   4. call force_gc6_exit.
//!   5. immediately re-probe the read op — does it now succeed? (wake verdict)
//!   6. call gc6_force_wake (cmd=2), re-probe.
//!   7. optional: sleep again + re-probe to see if the wake persisted.
//!
//! Interpretation:
//!   - If step 3 fails (-220) but step 5 succeeds → force_gc6_exit DOES wake.
//!   - If step 5 still fails → force_gc6_exit alone is insufficient; you need
//!     either polling keep-alive or a different mechanism.

use nvapi::PhysicalGpu;
use std::env;

fn try_read(label: &str, gpu: &PhysicalGpu) {
    match gpu.pstates() {
        Ok(_) => println!("  [{}] pstates() -> OK (GPU powered)", label),
        Err(e) => println!("  [{}] pstates() -> ERR {:?} (GPU not powered?)", label, e),
    }
}

fn main() {
    let idle_secs: u64 = env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);

    println!("== initialize ==");
    match nvapi::initialize() {
        Ok(_) => {}
        Err(e) => {
            println!("  initialize failed: {:?} (GPU already hard-GCOFF)", e);
            return;
        }
    }
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    if gpus.is_empty() {
        println!("  no GPUs");
        return;
    }
    let gpu = &gpus[0];
    println!("  GPU[0] enumerated");

    println!("\n== step 1: read right after enumerate (should be powered) ==");
    try_read("post-enumerate", gpu);

    println!(
        "\n== step 2: sleeping {}s with NO nvapi/nvml calls (let GCOFF take hold) ==",
        idle_secs
    );
    std::thread::sleep(std::time::Duration::from_secs(idle_secs));

    println!("\n== step 3: read after idle (expect -220 if GCOFF'd) ==");
    try_read("post-idle", gpu);

    println!("\n== step 4: force_gc6_exit (0x55590CB2) ==");
    match gpu.force_gc6_exit() {
        Ok(_) => println!("  ok"),
        Err(e) => println!("  ERR {:?}", e),
    }

    println!("\n== step 5: read immediately after force_gc6_exit (WAKE VERDICT) ==");
    try_read("post-force_gc6_exit", gpu);

    println!("\n== step 6: gc6_force_wake (0xD387D414 cmd=2) ==");
    match gpu.gc6_force_wake() {
        Ok(s) => println!("  ok, state={}", s),
        Err(e) => println!("  ERR {:?}", e),
    }
    println!("\n== step 6b: read after gc6_force_wake ==");
    try_read("post-gc6_force_wake", gpu);

    println!("\n== step 7: sleep {}s again, then read (did wake persist?) ==", idle_secs);
    std::thread::sleep(std::time::Duration::from_secs(idle_secs));
    try_read("post-second-idle", gpu);

    println!("\n== step 7b: CONTROL — read AGAIN with no force_gc6_exit in between ==");
    println!("  (if this 2nd read is OK, then a plain failed GET woke the GPU and");
    println!("   force_gc6_exit is NOT special — any RM call would do. If still -220,");
    println!("   only force_gc6_exit wakes it.)");
    try_read("second-read-no-wake", gpu);

    println!("\n== step 8: force_gc6_exit + immediate read, no idle gap ==");
    let _ = gpu.force_gc6_exit();
    try_read("force_exit-then-read", gpu);

    println!("\nDone. Verdict rules:");
    println!("  - step3 ERR + step5 OK          => force_gc6_exit wakes (prima facie)");
    println!("  - step7b ERR (2nd read still -220) => confirms plain GET does NOT wake");
    println!("  => together: force_gc6_exit is the SPECIFIC wake, not any RM call.");
}

