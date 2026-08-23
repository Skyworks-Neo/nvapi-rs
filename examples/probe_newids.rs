//! Live QI resolution probe for the 5 NVIDIA-App-discovered novel IDs.
use nvapi::initialize;
fn main() {
    let _ = initialize();
    let ids = [
        ("0xF21C2D56 ClientPowerModesGetInfo", 0xF21C2D56u32),
        ("0x180A9468 ClientPowerModesGetControl", 0x180A9468),
        ("0x3CC8C552 ClientPowerModesSetControl", 0x3CC8C552),
        ("0x7E4A9B0B DRS GetProfileInfo-family", 0x7E4A9B0B),
        ("0xAFC4C83E CustomMode Calculate", 0xAFC4C83E),
    ];
    for (name, id) in ids {
        match nvapi::sys::nvapi_QueryInterface(id) {
            Ok(p) if p != 0 => println!("{name}: RESOLVED 0x{p:X}"),
            _ => println!("{name}: NULL"),
        }
    }
}
