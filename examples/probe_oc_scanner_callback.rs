//! Live probe: OC Scanner status-callback subscription (0x1CB41116, V1EX 0x100D8).
//! VelocityX protocol: subscribe -> start -> poll snapshot -> stop -> unsubscribe.
use nvapi::PhysicalGpu;
use nvapi::initialize;
fn main() {
    println!("initialize: {:?}", initialize());
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = &gpus[0];
    println!("gpu: {:?}", gpu.full_name().map(|s| s.to_string()));

    println!("subscribe: {:?}", gpu.oem_oc_scanner_subscribe());
    println!("start:     {:?}", gpu.oem_oc_scanner_start());
    for i in 0..3 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let (state, progress, s60, s64) = PhysicalGpu::oem_oc_scanner_last_update();
        println!("t{i}: state={state} progress={progress} st60=0x{s60:X} st64=0x{s64:X}");
    }
    println!("stop:      {:?}", gpu.oem_oc_scanner_stop());
    println!("status:    {:?}", gpu.oem_oc_scanner_status());
    println!("unsub:     {:?}", gpu.oem_oc_scanner_unsubscribe());
}
