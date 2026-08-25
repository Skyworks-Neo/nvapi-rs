//! Live QI resolution probe for PNY VelocityX NVpower_wrapper.dll-discovered IDs.
use nvapi::initialize;
fn main() {
    let st = initialize();
    println!("initialize: {st:?}");
    let ids = [
        ("0xAD9A2E6D ClientPowerPoliciesSetInfo(NEW)", 0xAD9A2E6Du32),
        ("0xC03C31E8 MemoryInfoEx-private(NEW)", 0xC03C31E8),
        // siblings for context
        ("0xAD95F5ED ClientPowerPoliciesSetStatus", 0xAD95F5ED),
        ("0x34206D86 ClientPowerPoliciesGetInfo", 0x34206D86),
        ("0x0D258BB5 ClientThermalPoliciesGetInfo", 0x0D258BB5),
        ("0x34C0B13D ClientThermalPoliciesSetStatus", 0x34C0B13D),
        ("0xBC4AEE25 ClientStartOcScanner", 0xBC4AEE25),
        ("0xC28B73DE ClientStopOcScanner", 0xC28B73DE),
        ("0xCC727B22 ClientRevertOc", 0xCC727B22),
        ("0x593E8E72 ClientGetLastOcScannerResults", 0x593E8E72),
        ("0x210F1841 ClientGetOcConfig", 0x210F1841),
        (
            "0x1CB41116 ClientRegisterForOcScannerStatusUpdates",
            0x1CB41116,
        ),
        ("0xD7C61344 unload-internal", 0xD7C61344),
        ("0xE543C540 ClientFanPoliciesGetControl(ctrl)", 0xE543C540),
        ("0x0FE87B7F FanPolicyGetInfo(ctrl)", 0x0FE87B7F),
        ("0xAFFC2279 TGP-watt-SET(ctrl)", 0xAFFC2279),
        ("0x7B30AE0D queryPStateInfo(ctrl)", 0x7B30AE0D),
    ];
    for (name, id) in ids {
        match nvapi::sys::nvapi_QueryInterface(id) {
            Ok(p) if p != 0 => println!("{name}: RESOLVED 0x{p:X}"),
            _ => println!("{name}: NULL"),
        }
    }
}
// appended control probes
