//! Diagnostic sweep for the RegisterOp (0x2EB3C140) kernel lane on the local
//! machine — READ-ONLY. Separates "channel dead" (every offset fails) from
//! "window absent" (only the LACT GDDR/hotspot offsets fail).
//!   cargo test -p nvapi --test register_op_sweep_live -- --nocapture --ignored

use nvapi::PhysicalGpu;
use nvapi::sys;
use nvapi::sys::gpu::register::undocumented::{
    NV_GPU_REGISTER_OP_DATA_V1, NV_GPU_REGISTER_OP_FLAG_32BIT, NV_GPU_REGISTER_OP_FLAG_64BIT,
    NV_GPU_REGISTER_OP_FLAG_READ, NV_GPU_REGISTER_OP_FLAG_TYPE_20,
    NV_GPU_REGISTER_OP_FLAG_TYPE_GLOBAL,
};

fn raw_read(
    handle: sys::handles::NvPhysicalGpuHandle,
    offset: u32,
    flags: u16,
) -> (sys::NvAPI_Status, u16, u64) {
    let mut data = NV_GPU_REGISTER_OP_DATA_V1::default();
    data.op_count = 1;
    data.op[0].flags = flags;
    data.op[0].offset = offset;
    let rc = unsafe { sys::api::NvAPI_GPU_RegisterOp(handle, &mut data) };
    (rc, data.op[0].status, data.op[0].value)
}

#[test]
#[ignore = "live GPU read-only register sweep"]
fn register_op_sweep() {
    nvapi::initialize().expect("init");
    let phys = &PhysicalGpu::enumerate().expect("enumerate")[0];
    let handle = *phys.handle();
    println!("== GPU 0: {} ==", phys.full_name().unwrap_or_default());

    let r32g = NV_GPU_REGISTER_OP_FLAG_READ
        | NV_GPU_REGISTER_OP_FLAG_32BIT
        | NV_GPU_REGISTER_OP_FLAG_TYPE_GLOBAL;
    let r64g = NV_GPU_REGISTER_OP_FLAG_READ
        | NV_GPU_REGISTER_OP_FLAG_64BIT
        | NV_GPU_REGISTER_OP_FLAG_TYPE_GLOBAL;
    let r32t20 = NV_GPU_REGISTER_OP_FLAG_READ
        | NV_GPU_REGISTER_OP_FLAG_32BIT
        | NV_GPU_REGISTER_OP_FLAG_TYPE_20;

    println!("-- flag variants on 0x9024D0 --");
    for (name, flags) in [
        ("READ|32|GLOBAL", r32g),
        ("READ|64|GLOBAL", r64g),
        ("READ|32|TYPE20", r32t20),
    ] {
        let (rc, st, v) = raw_read(handle, 0x0090_24D0, flags);
        println!("  {name}: rc={rc} op_status={st} value={v:#x}");
    }

    println!("-- offset scatter (READ|32|GLOBAL) --");
    for offset in [
        0x0090_0200u32, // GDDR clamshell
        0x0090_24C0,    // GDDR temp data p0
        0x0090_24D0,    // GDDR temp status p0
        0x0090_64D0,    // GDDR temp status p1
        0x00AD_0AA0,    // Blackwell hotspot
        0x0000_0000,    // scatter
        0x0000_1000,
        0x0010_0000,
        0x0080_0000,
        0x0200_0000,
        0x1000_0000,
        0x1B00_0000,
    ] {
        let (rc, st, v) = raw_read(handle, offset, r32g);
        println!("  {offset:#010x}: rc={rc} op_status={st} value={v:#x}");
    }
}
