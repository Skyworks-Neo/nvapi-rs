//! Live probe for NvAPI_GPU_EnableOverclockedPstates (0xB23B70EE) — the
//! extended memory-OC-range unlock HYDRA 2.2B PRO brands as
//! "EnableMemoryOcExtendedRange" — plus the PSTATES20 V2 OV-array SET path
//! (HYDRA "NvApiSetOverVoltageOffset": numVoltages@+7316,
//! voltages[0].voltDelta_uV@+7332 on the 7416B V2 struct).
//!
//! Run: `cargo run --release --example probe_ocp_extended_range [-- <ov_uV>]`
//! Elevated runs redirect via PROBE_OUT=<file> (UAC drops console handles).
//!
//! Signature RE (HYDRA nvapioc.cpp, export @0x180002740): the real nvapi64
//! interface is 2-arg — fn(hPhysicalGPU, enable: NvU32) — evidenced by
//! `movzx edx, dl` before the indirect call (C# bool marshalling fingerprint)
//! and the log string "NvAPI_GPU_EnableOverclockedPstates(..., enabled ? 1 : 0)".
//! nvapi-rs previously declared it 1-arg, which leaves RDX undefined.
//!
//! The probe reads current pstate deltas as a baseline, then toggles enable=1
//! and enable=0 (restoring default) without applying any clock offsets.
//! On 50-series, enable=1 is claimed to widen the SetPstates20 memory-domain
//! offset clamp beyond the stock VBIOS range; on older GPUs expect
//! NotSupported / NoImplementation — the run still validates that the 2-arg
//! call reaches the driver without ill effect.

use nvapi::PhysicalGpu;

fn main() {
    if let Ok(path) = std::env::var("PROBE_OUT") {
        use std::io::Write;
        let f = std::fs::File::create(&path).unwrap();
        let mut w = std::io::BufWriter::new(f);
        run(&mut |s: String| {
            let _ = writeln!(w, "{s}");
            let _ = w.flush();
        });
        return;
    }
    run(&mut |s: String| println!("{s}"));
}

fn run(log: &mut dyn FnMut(String)) {
    nvapi::initialize().expect("NvAPI_Initialize failed");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");

    for gpu in &gpus {
        log(format!("GPU: {}", gpu.full_name().unwrap_or_default()));

        if let Ok(pstates) = gpu.pstates() {
            for ps in &pstates.pstates {
                for c in &ps.clocks {
                    log(format!(
                        "  baseline pstate {:?} domain {:?}: delta {:?} (editable {})",
                        ps.id,
                        c.domain(),
                        c.frequency_delta(),
                        c.editable()
                    ));
                }
            }
        }

        match gpu.enable_overclocked_pstates(true) {
            Ok(()) => log("  EnableOverclockedPstates(enable=1) -> OK".into()),
            Err(e) => log(format!("  EnableOverclockedPstates(enable=1) -> Err {e:?}")),
        }
        match gpu.enable_overclocked_pstates(false) {
            Ok(()) => log("  EnableOverclockedPstates(enable=0) -> OK  [restore]".into()),
            Err(e) => log(format!(
                "  EnableOverclockedPstates(enable=0) -> Err {e:?}  [restore]"
            )),
        }

        // PSTATES20 V2 OV-array path: a neutral 0 uV write verifies the SET
        // path end-to-end (needs admin; expect InvalidUserPrivilege otherwise).
        if let Some(delta) = std::env::args().nth(1).and_then(|s| s.parse::<i32>().ok()) {
            match gpu.set_overvolt(nvapi::MicrovoltsDelta(delta)) {
                Ok(()) => log(format!("  set_overvolt({delta} uV) -> OK")),
                Err(e) => log(format!("  set_overvolt({delta} uV) -> Err {e:?}")),
            }
            if let Ok(p) = gpu.pstates() {
                for ov in &p.overvolt {
                    log(format!("  post-GET overvolt: {ov:?}"));
                }
            }
        }
    }
}
