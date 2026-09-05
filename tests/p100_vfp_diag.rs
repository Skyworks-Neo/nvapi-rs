// Diagnostic for Volta/TCC P100 private V/F-POINTS:
//  1. raw driver-model status (TCC repro of NvAPI_GetDriverModel -6)
//  2. EVERY mask-present bank0 record (incl. the type-0 ones the reader
//     skips) → explains reset=168 vs read=160
//  3. the public VFP curve's voltage grid → is the private grid index
//     aligned with it (user report: private voltage fields read 0)?
// Run with: cargo test --test p100_vfp_diag -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;
use nvapi::sys::api::{NvAPI_GPU_ClockClkVfPointsGetInfo, NvAPI_GPU_ClockClkVfPointsGetStatus};
use nvapi::sys::gpu::clock::undocumented::{
    NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE,
    NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE,
};
use nvapi::sys::nvapi::NvVersion;

/// mask dword for `idx` in `bank`; rest offset = abs - 4.
fn mask_bit(
    info: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE,
    bank: usize,
    idx: usize,
) -> bool {
    let base = if bank == 0 { 4 } else { 0x34304 };
    let off = base + 4 * (idx >> 5) - 4;
    let dword = u32::from_le_bytes(info.rest[off..off + 4].try_into().unwrap());
    dword & (1 << (idx & 31)) != 0
}

#[test]
#[ignore]
fn p100_private_vfp_dump() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    eprintln!("=== {} GPU(s) ===", gpus.len());
    for (i, gpu) in gpus.iter().enumerate() {
        eprintln!(
            "--- GPU {} {:?} / {:?} ---",
            i,
            gpu.short_name(),
            gpu.full_name()
        );

        // 1. driver model — expect -6 (NVIDIA_DEVICE_NOT_FOUND) on TCC
        match gpu.driver_model() {
            Ok(dm) => eprintln!("driver_model: {:?}", dm),
            Err(e) => eprintln!("driver_model: Err {:?}", e),
        }

        // 2. GetInfo (masks)
        let mut info = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE>::new_zeroed();
            let mut b = b.assume_init();
            b.version =
                NvVersion::with_version(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE::MAGIC);
            b
        };
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetInfo(*gpu.handle(), core::ptr::from_mut(&mut *info).cast())
        };
        eprintln!("GetInfo: status={:#x}", st as i32);
        if st != 0 {
            continue;
        }

        // 3. GetStatus (records)
        let mut status = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE>::new_zeroed();
            let mut b = b.assume_init();
            b.version = NvVersion::with_version(2000388u32);
            b
        };
        status.rest[..128].copy_from_slice(&info.rest[..128]); // seed masks
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetStatus(
                *gpu.handle(),
                core::ptr::from_mut(&mut *status).cast(),
            )
        };
        eprintln!("GetStatus: status={:#x}", st as i32);
        if st != 0 {
            continue;
        }

        for bank in 0..2usize {
            let rec_base = if bank == 0 { 772 } else { 1000964 };
            let mut present = 0usize;
            let mut typed = 0usize;
            let mut type0 = Vec::new();
            for idx in 0..2048usize {
                if !mask_bit(&info, bank, idx) {
                    continue;
                }
                present += 1;
                let typ = status.rest[rec_base + 488 * idx - 4];
                if typ != 0 {
                    typed += 1;
                } else {
                    type0.push(idx);
                }
            }
            eprintln!(
                "bank{bank}: present={present} typed={typed} type0_count={} type0_idx={:?}",
                type0.len(),
                type0
            );

            // dump every present record: idx, type, freq_def, freq_cur, volt
            for idx in 0..2048usize {
                if !mask_bit(&info, bank, idx) {
                    continue;
                }
                let rec = rec_base + 488 * idx - 4;
                let typ = status.rest[rec];
                let rd = |off: usize| -> u32 {
                    u32::from_le_bytes(status.rest[rec + off..rec + off + 4].try_into().unwrap())
                };
                eprintln!(
                    "  b{bank} #{idx:4}: type={typ:3} freq_def={:6} freq_cur={:6} uV@58={:9} uV@68={:9}",
                    rd(0x24),
                    rd(0x64),
                    rd(0x58),
                    rd(0x68),
                );
            }
        }

        // 4. public VFP curve for grid comparison
        match gpu.vfp_info() {
            Ok(vi) => match gpu.vfp_curve(&vi) {
                Ok(c) => {
                    let total: usize = c.points.values().map(|v| v.len()).sum();
                    eprintln!("public vfp_curve: {total} points");
                    for (domain, entries) in &c.points {
                        eprintln!("  domain {domain:?}: {} points", entries.len());
                        for (i, (idx, p)) in entries.iter().enumerate().take(90) {
                            eprintln!(
                                "  pub #{i:3} (idx {idx}): def={:6} cur={:6} volt={}",
                                p.default.frequency.0, p.current.frequency.0, p.default.voltage.0
                            );
                        }
                    }
                }
                Err(e) => eprintln!("public vfp_curve: Err {:?}", e),
            },
            Err(e) => eprintln!("public vfp_info: Err {:?}", e),
        }
    }
}
