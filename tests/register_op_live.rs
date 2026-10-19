//! Live smoke test for the `NvAPI_GPU_RegisterOp` (0x2EB3C140) read lane.
//!   cargo test -p nvapi --test register_op_live -- --nocapture --ignored
//!
//! READ-ONLY probe of the register windows LACT drives (GDDR clamshell
//! `0x900200`, GDDR6x/7 status `0x9024D0`, Blackwell hotspot `0xAD0AA0`).
//! The point is marshaling validation: an Ok value or a call-level status
//! other than -9 proves the driver parsed our version stamp (6152 | 1<<16).
//!
//! Findings (TU106 RTX 2070, 610.47 Windows, full matrix in
//! docs/reverse-engineering/nvapi/registerop-0x2eb3c140-handler-r61047.md):
//! the kernel lane needs an ELEVATED process — non-admin reads return
//! call-level NVAPI_ERROR (-1) for every offset; elevated, the GDDR6
//! windows decode to `0xBADF****` poison (no readable per-chip sensors on
//! GDDR6) and the Blackwell hotspot register is poison off-Blackwell.

use nvapi::PhysicalGpu;
use nvapi::hi::Gpu;
use nvapi::sys;
use nvapi::sys::gpu::register::undocumented::{
    NV_GPU_REGISTER_OP_DATA_V1, NV_GPU_REGISTER_OP_FLAG_32BIT, NV_GPU_REGISTER_OP_FLAG_READ,
    NV_GPU_REGISTER_OP_FLAG_TYPE_GLOBAL,
};

/// Raw single read that surfaces the per-op status even when the call fails.
fn raw_read(
    handle: sys::handles::NvPhysicalGpuHandle,
    offset: u32,
) -> (sys::NvAPI_Status, (u16, u64)) {
    let mut data = NV_GPU_REGISTER_OP_DATA_V1::default();
    data.op_count = 1;
    data.op[0].flags = NV_GPU_REGISTER_OP_FLAG_READ
        | NV_GPU_REGISTER_OP_FLAG_32BIT
        | NV_GPU_REGISTER_OP_FLAG_TYPE_GLOBAL;
    data.op[0].offset = offset;
    let rc = unsafe { sys::api::NvAPI_GPU_RegisterOp(handle, &mut data) };
    (rc, (data.op[0].status, data.op[0].value))
}

#[test]
#[ignore = "live GPU read-only register probe"]
fn register_op_reads() {
    nvapi::initialize().expect("init");
    for (idx, phys) in PhysicalGpu::enumerate()
        .expect("enumerate")
        .into_iter()
        .enumerate()
    {
        let gpu_name = phys.full_name().unwrap_or_default();
        let handle = *phys.handle();
        let gpu = Gpu::new(phys);
        println!("== GPU {idx}: {gpu_name} ==");

        let ram = gpu.ram_type();
        println!("  ram_type = {ram:?}");

        for (reg_name, offset) in [
            ("gddr_clamshell_0x900200", 0x0090_0200u32),
            ("gddr_status_p0_0x9024D0", 0x0090_24D0),
            ("blackwell_hotspot_0xAD0AA0", 0x00AD_0AA0),
        ] {
            let (rc, (op_status, value)) = raw_read(handle, offset);
            println!("  {reg_name}: rc={rc} op_status={op_status} value={value:#x}");
        }

        match ram {
            Ok(ram) => match gpu.vram_chip_temps(ram) {
                Ok(chips) => println!("  vram_chips = {chips:?}"),
                Err(e) => println!("  vram_chips = ERR {e}"),
            },
            Err(_) => println!("  vram_chips skipped (ram_type unreadable)"),
        }
        match gpu.blackwell_hotspot_register_c() {
            Ok(temp) => println!("  blackwell_hotspot = {temp} C"),
            Err(e) => println!("  blackwell_hotspot = ERR {e}"),
        }
    }
}
