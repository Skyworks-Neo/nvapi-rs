//! Context probe: locate the known thermal windows inside the sweep map and
//! sanity-check two classic registers (PMC_BOOT_0, PTIMER) — READ-ONLY,
//! elevated.
//!   cargo test -p nvapi --test register_op_context_live -- --nocapture --ignored

use std::thread::sleep;
use std::time::Duration;

use nvapi::PhysicalGpu;
use nvapi::sys;
use nvapi::sys::gpu::register::undocumented::{
    NV_GPU_REGISTER_OP_DATA_V1, NV_GPU_REGISTER_OP_FLAG_32BIT, NV_GPU_REGISTER_OP_FLAG_READ,
    NV_GPU_REGISTER_OP_FLAG_TYPE_GLOBAL,
};

const R32G: u16 = NV_GPU_REGISTER_OP_FLAG_READ
    | NV_GPU_REGISTER_OP_FLAG_32BIT
    | NV_GPU_REGISTER_OP_FLAG_TYPE_GLOBAL;

fn rd(handle: sys::handles::NvPhysicalGpuHandle, offset: u32) -> Result<u64, sys::NvAPI_Status> {
    let mut data = NV_GPU_REGISTER_OP_DATA_V1::default();
    data.op_count = 1;
    data.op[0].flags = R32G;
    data.op[0].offset = offset;
    let rc = unsafe { sys::api::NvAPI_GPU_RegisterOp(handle, &mut data) };
    if rc == 0 {
        Ok(data.op[0].value)
    } else {
        Err(rc)
    }
}

fn cls(v: Result<u64, sys::NvAPI_Status>) -> String {
    match v {
        Err(rc) => format!("rc={rc}"),
        Ok(v) if v & 0xFFFF_0000 == 0xBADF_0000 => format!("poison({v:#x})"),
        Ok(v) => format!("{v:#x}"),
    }
}

#[test]
#[ignore = "live register context probe (elevated, read-only)"]
fn register_context() {
    nvapi::initialize().expect("init");
    let phys = &PhysicalGpu::enumerate().expect("enumerate")[0];
    let handle = *phys.handle();
    println!("== GPU 0: {} ==", phys.full_name().unwrap_or_default());

    // Classic identity: PMC_BOOT_0 (chip class + revision), envytools/nouveau.
    println!(
        "[PMC_BOOT_0 @0x0] {} (expect TU106 family: class 0x166 in high bits)",
        cls(rd(handle, 0x0))
    );

    // PTIMER family @0x9000 (NV50+): counter must tick between reads.
    let t0 = rd(handle, 0x9400);
    sleep(Duration::from_millis(120));
    let t1 = rd(handle, 0x9400);
    println!(
        "[PTIMER? @0x9400] {t0:?} -> {t1:?} (tick delta = {:?})",
        t1.ok().zip(t0.ok()).map(|(a, b)| a.wrapping_sub(b))
    );

    // LACT GDDR windows (0x9024C0/D0 + partition stride) and neighbors.
    println!("[GDDR FBPA windows] 0x9024C0 family (LACT GDDR6x/7 base):");
    for off in [0x9002_00u32, 0x9024_C0, 0x9024_D0, 0x9064_C0, 0x9064_D0] {
        println!("  {off:#010x}: {}", cls(rd(handle, off)));
    }

    // GB202 GDDR7 per-module base (gddr6.c): 0x9A24C0.
    println!("[GDDR7 FBPA (GB202 base) 0x9A24C0 family]:");
    for off in [0x9A24_C0u32, 0x9A24_D0] {
        println!("  {off:#010x}: {}", cls(rd(handle, off)));
    }

    // Blackwell aggregated hotspot + Ampere/Ada thermal-block memory junction.
    // Junction decode: bits 11:0, 1/32 °C per LSB (gddr6.c Ampere/Ada table).
    println!("[other thermal windows]:");
    for off in [0x00AD_0AA0u32, 0x0000_E2A8, 0x0000_EE50] {
        let v = rd(handle, off);
        let note = match &v {
            Ok(x) if off == 0x0000_E2A8 || off == 0x0000_EE50 => {
                format!("  junction? = {:.2} C", (x & 0xFFF) as f64 / 32.0)
            }
            Ok(x) if x & 0xFFFF_0000 == 0xBADF_0000 => "  (poison)".to_string(),
            _ => String::new(),
        };
        println!("  {off:#010x}: {}{note}", cls(v));
    }

    // Neighborhood of the GDDR window: 0x902400..0x902500 at dword steps,
    // compact 16-per-line class map.
    println!("[map 0x902400..0x902500] d=data z=zero p=poison -=rc!=0:");
    for row in 0..16u32 {
        let base = 0x9024_00 + row * 0x10;
        let mut line = String::new();
        for i in 0..16u32 {
            let v = rd(handle, base + i * 4);
            let ch = match v {
                Err(_) => '-',
                Ok(x) if x == 0 => 'z',
                Ok(x) if x & 0xFFFF_0000 == 0xBADF_0000 => 'p',
                Ok(_) => 'd',
            };
            line.push(ch);
        }
        println!("  {base:#010x}: {line}");
    }
}
