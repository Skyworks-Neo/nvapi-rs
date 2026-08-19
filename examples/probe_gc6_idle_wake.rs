//! Controlled probe: independently verify BOTH GC6 wake interfaces can wake an
//! idle/GCOFF'd dGPU, each tested in its own clean idle->wake cycle so neither
//! benefits from the other's prior wake.
//!
//! Run: `cargo run --release -p nvapi --example probe_gc6_idle_wake -- <secs>`
//!
//! Two wake interfaces (both confirmed QI-resolved on the 610 driver):
//!   force_gc6_exit  (0x55590CB2) — fn(hGpu), escape 0x10000FC, no struct
//!   gc6_force_wake  (0xD387D414 cmd=2) — fn(hGpu,*12B magic 0x1000C), escape 0x70000ED
//!
//! Each wake interface is tested in isolation:
//!   idle -> read (confirm -220, i.e. truly GCOFF) -> wake -> read (verdict)
//! Plus a CONTROL cycle that does read->read (no wake) to prove plain GETs don't wake.
//!
//! Verdict per interface:
//!   read-after-idle = -220 AND read-after-wake = OK  => independently wakes.
//!   read-after-wake = -220                            => does NOT independently wake.

use nvapi::PhysicalGpu;
use std::env;
use std::time::Duration;

fn idle_then(secs: u64, label: &str) {
    println!(
        "\n  [idle {}s — no nvapi/nvml calls, let GCOFF take hold]",
        secs
    );
    std::thread::sleep(Duration::from_secs(secs));
    println!("  --- {} ---", label);
}

fn read(gpu: &PhysicalGpu, label: &str) -> bool {
    match gpu.pstates() {
        Ok(_) => {
            println!("    [{}] pstates() -> OK (powered)", label);
            true
        }
        Err(e) => {
            println!("    [{}] pstates() -> ERR {:?} (not powered)", label, e);
            false
        }
    }
}

fn main() {
    let secs: u64 = env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);

    println!("== initialize ==");
    match nvapi::initialize() {
        Ok(_) => {}
        Err(e) => {
            println!("  initialize failed: {:?} (GPU hard-GCOFF at init)", e);
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
    read(gpu, "post-enumerate");

    // ---------- CYCLE 1: CONTROL (plain GET does not wake) ----------
    println!("\n=== CYCLE 1: CONTROL — plain GET after idle (proves GET alone doesn't wake) ===");
    idle_then(secs, "1st read after idle");
    let _ = read(gpu, "control-1st");
    println!("  (2nd read immediately, no wake in between:)");
    let _ = read(gpu, "control-2nd");

    // ---------- CYCLE 2: force_gc6_exit ----------
    println!("\n=== CYCLE 2: force_gc6_exit (0x55590CB2) — independent wake test ===");
    idle_then(secs, "read after idle (expect -220)");
    let before = read(gpu, "pre-force_gc6_exit");
    match gpu.force_gc6_exit() {
        Ok(_) => println!("    force_gc6_exit -> ok"),
        Err(e) => println!("    force_gc6_exit -> ERR {:?}", e),
    }
    let after = read(gpu, "post-force_gc6_exit");
    verdict("force_gc6_exit", before, after);

    // ---------- CYCLE 3: gc6_force_wake ----------
    println!("\n=== CYCLE 3: gc6_force_wake (0xD387D414 cmd=2) — independent wake test ===");
    idle_then(secs, "read after idle (expect -220)");
    let before = read(gpu, "pre-gc6_force_wake");
    match gpu.gc6_force_wake() {
        Ok(s) => println!("    gc6_force_wake -> ok, state={}", s),
        Err(e) => println!("    gc6_force_wake -> ERR {:?}", e),
    }
    let after = read(gpu, "post-gc6_force_wake");
    verdict("gc6_force_wake", before, after);

    println!("\nDone.");
}

fn verdict(name: &str, before: bool, after: bool) {
    println!(
        "  VERDICT {}: before_wake={}, after_wake={}",
        name, before, after
    );
    if !before && after {
        println!("    => {} INDEPENDENTLY wakes a GCOFF'd dGPU.", name);
    } else if !after {
        println!("    => {} does NOT independently wake.", name);
    } else {
        println!("    => inconclusive (GPU was already powered before wake — idle too short?).");
    }
}
