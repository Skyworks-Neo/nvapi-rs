//! One-shot probe: do the dGPU force-wake candidate IDs resolve on the live
//! 610 mobile driver? Only IDs that return non-NULL via nvapi_QueryInterface are
//! worth RE'ing for struct/signature. NULL (or NoImplementation) ones are dead
//! ends on this driver.
//!
//! Run: `cargo run --release -p nvapi --example probe_wake_ids`
//!
//! Background (see memory: deepidlestate-not-wake-rtd3-wake-leads):
//! GPUMonCmd -wake is just an internal flag; DeepIdleState 0x1AAD16B4/0x568A2292
//! is unimplemented (NoImpl -200) and a scalar-policy setter, not a wake. The
//! three leads here came out of the DeepIdle RE + the YOFOO key table:
//!   0x9A60640C  HWFS control (hwfsControlEscData, struct-write, escape 0x07000190)
//!               — NVIDIA's internal force/sleep path; most promising.
//!   0x55590CB2  ForceGC6Exit (YOFOO nvGpu.spec; name says force-exit GC6).
//!   0xD387D414  GC6Control (YOFOO nvGpu.spec; generic GC6 control).
//! Plus reference points: a known-good resolver (TGP-watt GET) and known-dead
//! IDs (DeepIdle, phantom targettemp).

use nvapi::sys::nvapi_QueryInterface;

fn probe(id: u32, label: &str) {
    let r = nvapi_QueryInterface(id);
    match r {
        Ok(ptr) => println!("0x{:08X} {:30} -> 0x{:016X}  RESOLVED", id, label, ptr),
        Err(e) => println!("0x{:08X} {:30} -> NULL  ({:?})", id, label, e),
    }
}

fn main() {
    // NOTE: nvapi_QueryInterface only walks nvapi64.dll's function-pointer
    // table — it does NOT need a powered GPU. We deliberately do NOT call
    // NvAPI_Initialize here, because on a 610 mobile driver with the dGPU in
    // GCOFF, Initialize returns NvidiaDeviceNotFound. QI resolution is the
    // question we actually want answered ("is this ID implemented on this
    // driver?"), and that is independent of GPU power state.
    match nvapi::initialize() {
        Ok(_) => println!("(NvAPI_Initialize ok — dGPU is currently powered)\n"),
        Err(e) => println!("(NvAPI_Initialize failed: {:?} — dGPU likely GCOFF; \
                            QI probe continues since ID resolution is driver-table-only)\n", e),
    }

    println!("=== dGPU force-wake candidates ===");
    probe(0x9a60640c, "HWFS control (struct-write)");
    probe(0x55590cb2, "ForceGC6Exit");
    probe(0xd387d414, "GC6Control");

    println!("\n=== GC6 adjacent (read-side, for context) ===");
    probe(0xc118ed82, "GetGC6Statistics");
    probe(0xf6f0454e, "GetGCXWakeUpReasonInfo");
    probe(0x7bf85571, "Diag_GetGC6DebugInfo");
    probe(0x0191a35e, "queryRTD3");

    println!("\n=== reference: known-good resolver ===");
    probe(0x8b3e7343, "TGP-watt GET (works)");

    println!("\n=== reference: known-dead / unimplemented ===");
    probe(0x1aad16b4, "GetDeepIdleState (NoImpl)");
    probe(0x568a2292, "SetDeepIdleState (NoImpl)");
    probe(0xe0765b6f, "phantom targettemp SET");
}
