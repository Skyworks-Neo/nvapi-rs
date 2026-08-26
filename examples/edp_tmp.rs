fn main() {
    let _ = nvapi::initialize();
    let gpus = nvapi::PhysicalGpu::enumerate().unwrap();
    let st = gpus[0].enable_dynamic_pstates(0);
    println!("EnableDynamicPstates(0) -> {:?}", st);
}
