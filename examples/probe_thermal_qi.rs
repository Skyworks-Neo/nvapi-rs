//! One-shot probe: does nvoc's process resolve the PRIVATE targettemp IDs?
//!
//! Run: `cargo run --release -p nvapi --example probe_thermal_qi`
//!
//! Expected (if hypothesis holds): 0xE097144F (SET) and 0xC4554575 (GET-prime)
//! both resolve to non-NULL pointers, because they sit in nvapi64.dll's
//! STATIC table off_1804DD000 — no EnableRID74954 gate needed. The documented
//! 0x34C0B13D/0xE9C425A1/0x0D258BB5 should also resolve. The phantom 0xE0765B6F
//! (wrong earlier attribution) should be NULL.

use nvapi::sys::nvapi_QueryInterface;

fn probe(id: u32, label: &str) {
    let r = nvapi_QueryInterface(id);
    match r {
        Ok(ptr) => println!("0x{:08X} {:28} -> 0x{:016X}  RESOLVED", id, label, ptr),
        Err(e) => println!("0x{:08X} {:28} -> NULL  ({:?})", id, label, e),
    }
}

fn main() {
    nvapi::initialize().expect("NvAPI_Initialize failed");

    println!("=== ClientThermalPolicies family (nvamsi static table off_1804DD000) ===");
    probe(0x0d258bb5, "GetInfo (doc V3)");
    probe(0xc4554575, "GET-prime (PRIVATE)");      // targettemp GET-prime
    probe(0xe097144f, "SET (PRIVATE targettemp)"); // <-- the real SET
    probe(0x34c0b13d, "SetStatus (doc V3)");
    probe(0xe9c425a1, "GetStatus (doc V3)");

    println!("\n=== wrong-attribution phantom (should be NULL) ===");
    probe(0xe0765b6f, "phantom SET (was wrong)");
    probe(0x2f7a429d, "phantom GetInfo (was wrong)");

    println!("\n=== TGP-watt (sanity, known to work) ===");
    probe(0x8b3e7343, "TGP-watt GET");
    probe(0xaffc2279, "TGP-watt SET");
}
