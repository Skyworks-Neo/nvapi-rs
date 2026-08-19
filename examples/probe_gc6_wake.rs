//! Verify the GC6 force-wake wrappers can actually wake a GCOFF dGPU.
//!
//! Run: `cargo run --release -p nvapi --example probe_gc6_wake`
//!
//! On a 610 mobile driver with aggressive GC6/GCOFF, the dGPU may be powered
//! down at idle. This probe:
//!   1. enumerates GPUs (NvAPI_Initialize + Gpu::enumerate — fails if GCOFF'd
//!      hard enough that no device is visible),
//!   2. for the first GPU: queries GC6 state, calls force_gc6_exit, queries
//!      GC6 state again, then tries the struct-based gc6_force_wake too.
//! State decode: 3 = D0/active (awake), 2 = GC6/idle (down), 0 = OK/no report.
//!
//! If the GPU is so deep in GCOFF that even enumerate fails, the probe prints
//! that and exits — which itself confirms the problem (initialize-time
//! NvidiaDeviceNotFound). In that case, manually wake the GPU once (any GPU
//! load), re-run, and the wake calls should then keep it awake for follow-up
//! overclock ops.

fn decode_state(s: u32) -> &'static str {
    match s {
        0 => "OK/no-report",
        2 => "GC6/IDLE (powered down)",
        3 => "D0/ACTIVE (awake)",
        4 => "UNKNOWN",
        _ => "OTHER(?)",
    }
}

fn main() {
    use nvapi::PhysicalGpu;
    println!("== NvAPI_Initialize ==");
    match nvapi::initialize() {
        Ok(_) => println!("  ok (dGPU visible to enumerator)"),
        Err(e) => {
            println!(
                "  FAILED: {:?}\n  (dGPU is hard-GCOFF — even enumerate can't see it.",
                e
            );
            println!("   Manually wake it once with any GPU load, then re-run.)");
            return;
        }
    }

    println!("\n== enumerate ==");
    let gpus = PhysicalGpu::enumerate().expect("enumerate failed");
    if gpus.is_empty() {
        println!("  no physical GPUs found");
        return;
    }
    let gpu = &gpus[0];
    println!("  GPU[0] enumerated ok");

    println!("\n== GC6 state (query, cmd=0) ==");
    match gpu.gc6_query_state() {
        Ok(s) => println!("  state = {} ({})", s, decode_state(s)),
        Err(e) => println!("  ERR: {:?}", e),
    }

    println!("\n== force_gc6_exit (one-shot wake, 0x55590CB2) ==");
    match gpu.force_gc6_exit() {
        Ok(_) => println!("  ok"),
        Err(e) => println!("  ERR: {:?}", e),
    }

    println!("\n== GC6 state after force_gc6_exit ==");
    match gpu.gc6_query_state() {
        Ok(s) => println!("  state = {} ({})", s, decode_state(s)),
        Err(e) => println!("  ERR: {:?}", e),
    }

    println!("\n== gc6_force_wake (struct cmd=2, 0xD387D414) ==");
    match gpu.gc6_force_wake() {
        Ok(s) => println!("  ok, post-state = {} ({})", s, decode_state(s)),
        Err(e) => println!("  ERR: {:?}", e),
    }

    println!("\n== sanity: a real op after wake (pstate levels, domain 0) ==");
    // If the wake worked, this op (which fails with -220 when GCOFF) should succeed.
    match gpu.pstate_levels_domain(0) {
        Ok(Some(info)) => println!("  ok, {} pstate entries read", info.pstates.len()),
        Ok(None) => println!("  ok but empty (driver returned no entries)"),
        Err(e) => println!("  ERR: {:?}", e),
    }
}
