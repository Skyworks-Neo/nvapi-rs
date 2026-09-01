// V100/GV100: does the private V/F-POINTS CONTROL (GetControl 0xDA025C3E)
// legacy stamp (0x16004, the R391-era variant) work — i.e. is the write-
// path readback surface present under the V1-era table? Read-only.
//
// Run: cargo test -p nvapi --test volta_vfp_control_legacy -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;
use nvapi::sys::api::NvAPI_GPU_ClockClkVfPointsGetControl;
use nvapi::sys::gpu::clock::undocumented::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE;
use nvapi::sys::nvapi::NvVersion;

#[test]
#[ignore]
fn volta_vfp_control_legacy() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    eprintln!("GPU: {:?}", gpu.full_name());

    // legacy CONTROL stamp (R391-era; see private-vfp-legacy-stamp-fallback)
    const CONTROL_LEGACY: u32 = 0x16004;
    let mut ctrl = unsafe {
        let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE>::new_zeroed();
        let mut b = b.assume_init();
        b.version = NvVersion::with_version(CONTROL_LEGACY);
        b
    };
    // seed the legacy INFO header (mask bytes) like the production reader does
    // — the legacy header is 20B of mask; copy a generous prefix
    let header = [0u8; 32];
    {
        let rest = &mut ctrl.rest;
        let n = header.len().min(rest.len());
        rest[..n].copy_from_slice(&header[..n]);
    }
    let st = unsafe {
        NvAPI_GPU_ClockClkVfPointsGetControl(*gpu.handle(), ptr::from_mut(&mut *ctrl).cast())
    };
    eprintln!("GetControl[LEGACY 0x16004] status={:#x}", st as i32);
    if st != 0 {
        eprintln!("legacy CONTROL rejected — write-path readback surface ABSENT");
        return;
    }

    // census: nonzero u32s (the per-point mode/value override table)
    let nz: Vec<(usize, u32)> = (0..ctrl.rest.len() / 4)
        .map(|i| {
            (
                i * 4,
                u32::from_le_bytes(ctrl.rest[i * 4..i * 4 + 4].try_into().unwrap()),
            )
        })
        .filter(|(_, v)| *v != 0)
        .collect();
    eprintln!("nonzero u32: {}/{}", nz.len(), ctrl.rest.len() / 4);
    for (off, v) in nz.iter().take(60) {
        eprintln!("  +{off:04x}: 0x{v:08X} ({v})");
    }
}

use core::ptr;
