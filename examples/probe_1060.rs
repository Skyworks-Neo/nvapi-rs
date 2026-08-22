//! Dump raw 1060B GetControl records for GPC (idx 0) and XBAR (idx 127)
//! on bank 0, and diff the non-zero fields. The per-domain scaling
//! factor should appear as a field difference between the two records.
use nvapi::initialize;
use nvapi::sys::api::{
    NvAPI_EnumPhysicalGPUs, NvAPI_GPU_ClockClkVfPointsGetControl,
    NvAPI_GPU_ClockClkVfPointsGetInfo,
};
use nvapi::sys::gpu::clock::private::*;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use std::ptr;

fn main() {
    let _ = initialize();
    let mut h = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut n = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut h, &mut n) };
    let gpu = h[0];

    let mut info = Box::new(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1::default());
    unsafe { NvAPI_GPU_ClockClkVfPointsGetInfo(gpu, ptr::from_mut(&mut *info).cast()) };

    let mut ctrl: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE> =
        Box::new(unsafe { std::mem::zeroed() });
    ctrl.version = nvapi::sys::api::NvVersion::with_version(clk_vfp_control::MAGIC);
    ctrl.seed_masks_from_info(&info);
    unsafe { NvAPI_GPU_ClockClkVfPointsGetControl(gpu, ptr::from_mut(&mut *ctrl).cast()) };

    // Dump 1060B records for GPC (idx 0), XBAR (idx 127), and HOST (idx 259)
    for (label, idx) in [("GPC", 0usize), ("XBAR", 127), ("HOST", 259)] {
        let rec_base = clk_vfp_control::REC1 + clk_vfp_control::STRIDE * idx;
        let off = rec_base - 4; // convert to rest[] offset
        eprintln!("=== {label} (idx {idx}) @ rest+0x{off:X} ===");

        // Dump in dword rows
        for row in (0..1060usize).step_by(32) {
            let start = off + row;
            if start + 32 > ctrl.rest.len() { break; }
            let dws: Vec<u32> = (0..8usize)
                .map(|k| u32::from_le_bytes(
                    ctrl.rest[start + k*4..start + k*4 + 4].try_into().unwrap_or([0;4])
                ))
                .collect();
            let any_nonzero = dws.iter().any(|&v| v != 0);
            if any_nonzero {
                eprintln!("  +0x{row:04X}: {}", dws.iter()
                    .map(|v| format!("{v:08X}"))
                    .collect::<Vec<_>>()
                    .join(" "));
            }
        }
        eprintln!();
    }

    // Also dump a raw byte comparison of the first 128 bytes per record
    eprintln!("=== Byte-level diff (first 256 bytes) ===");
    eprintln!("offset | GPC            | XBAR           | HOST");
    let gpc_off = clk_vfp_control::REC1 - 4;
    let xbar_off = clk_vfp_control::REC1 + clk_vfp_control::STRIDE * 127 - 4;
    let host_off = clk_vfp_control::REC1 + clk_vfp_control::STRIDE * 259 - 4;
    for i in (0..256usize).step_by(4) {
        let g = u32::from_le_bytes(ctrl.rest[gpc_off+i..gpc_off+i+4].try_into().unwrap_or([0;4]));
        let x = u32::from_le_bytes(ctrl.rest[xbar_off+i..xbar_off+i+4].try_into().unwrap_or([0;4]));
        let h = u32::from_le_bytes(ctrl.rest[host_off+i..host_off+i+4].try_into().unwrap_or([0;4]));
        if g != 0 || x != 0 || h != 0 {
            let diff = if g == x && x == h { "" } else { " <== DIFF" };
            eprintln!("  +0x{i:04X}: {g:08X} | {x:08X} | {h:08X}{diff}");
        }
    }
}
