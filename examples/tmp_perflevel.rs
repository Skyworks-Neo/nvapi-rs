fn main() {
    nvapi::initialize().expect("init");
    for (name, id) in [
        ("GetPowerMizerInfo 0x76bfa16b", 0x76bfa16bu32),
        ("SetPowerMizerInfo 0x50016c78", 0x50016c78),
        ("SetPerfLevel      0x75dd3e6a", 0x75dd3e6a),
    ] {
        match nvapi::sys::nvapi_QueryInterface(id) {
            Ok(p) if p != 0 => println!("{name}: RESOLVED"),
            _ => println!("{name}: NULL"),
        }
    }
}
