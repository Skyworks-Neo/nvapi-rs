//! Live probe for NvAPI_GPU_SetExtendedThermalSimulationMode (0x95E71AB6)
//! — temperature-simulation interface RE'd from GPUMon.exe + ThermSpyPremium.
//!
//! Signature: fn(hGpu, flags: u32, enable: u32, temperature: i32) -> status
//! Requires VBIOS "Secured Overrides" table with <Temp faking allowed> enabled.
//! On Ada mobile (no VBIOS override): expect NotSupported.
//!
//! Run: `cargo run --release --example probe_temp_sim [temperature_C]`

use nvapi::PhysicalGpu;

fn main() {
    let target: i32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(65); // default 65C if given

    nvapi::initialize().expect("NvAPI_Initialize failed");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = &gpus[0];
    println!("GPU: {}", gpu.full_name().unwrap_or_default());

    // 1. read current sim state (baseline)
    match gpu.temp_sim() {
        Ok((enabled, temp)) => println!("baseline temp_sim: enabled={enabled} temp={temp}C"),
        Err(e) => println!("GetThermalSimulationMode: {e:?}"),
    }

    // 2. try set sim (enable=1, temp=target)
    match gpu.set_temp_sim(target) {
        Ok(()) => println!("set_temp_sim({target}C) -> OK"),
        Err(e) => println!("set_temp_sim({target}C) -> Err {e:?}"),
    }

    // 3. read back
    match gpu.temp_sim() {
        Ok((enabled, temp)) => println!("post-set temp_sim: enabled={enabled} temp={temp}C"),
        Err(e) => println!("post-set GetThermalSimulationMode: {e:?}"),
    }

    // 4. disable
    match gpu.disable_temp_sim() {
        Ok(()) => println!("disable_temp_sim -> OK  [restore]"),
        Err(e) => println!("disable_temp_sim -> Err {e:?}  [restore]"),
    }
}
