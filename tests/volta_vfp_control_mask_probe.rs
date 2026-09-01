// V100/GV100: CONTROL head-mask single-bit probe — READ-ONLY stride oracle.
//
// Evidence so far: all-zero input → all-zero response (stock); marker
// garbage in rest[0..0x20) → -1 (validated head); from +0x40 the driver
// echoes payload verbatim (user-copyable region). The head is therefore a
// byte-per-bit mask (legacy-INFO style: byte i = bit i). If the R610
// "fill from driver state" semantic holds, mask bit k alone should make
// GetControl materialize record k at its slot offset — offset deltas
// between consecutive k give the record stride WITHOUT any write.
//
// Run: cargo test -p nvapi --test volta_vfp_control_mask_probe -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;
use nvapi::sys::api::NvAPI_GPU_ClockClkVfPointsGetControl;
use nvapi::sys::gpu::clock::undocumented::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE;
use nvapi::sys::nvapi::NvVersion;

const SNAPSHOT_MAGIC: u32 = 82976;
const SCAN: usize = 256 * 1024;

fn get_with(gpu: &PhysicalGpu, input: &[u8]) -> Option<Vec<u8>> {
    let mut ctrl = unsafe {
        let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE>::new_zeroed();
        let mut b = b.assume_init();
        b.version = NvVersion::with_version(SNAPSHOT_MAGIC);
        b
    };
    ctrl.rest[..input.len()].copy_from_slice(input);
    let st = unsafe {
        NvAPI_GPU_ClockClkVfPointsGetControl(*gpu.handle(), ptr::from_mut(&mut *ctrl).cast())
    };
    if st != 0 {
        eprintln!("    -> rejected: {:#x}", st as i32);
        return None;
    }
    Some(ctrl.rest.to_vec())
}

#[test]
#[ignore]
fn volta_vfp_control_mask_single_bit() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    eprintln!("GPU: {:?}", gpu.full_name());

    let mut hits: Vec<(usize, Vec<(usize, u32)>)> = Vec::new();

    for k in 0..0x20usize {
        let mut input = vec![0u8; SCAN];
        input[k] = 1;
        eprint!("  rest[{k:#04x}]=1: ");
        let Some(resp) = get_with(gpu, &input) else {
            continue;
        };
        let nz: Vec<(usize, u32)> = (0..SCAN / 4)
            .map(|i| {
                (
                    i * 4,
                    u32::from_le_bytes(resp[i * 4..i * 4 + 4].try_into().unwrap()),
                )
            })
            .filter(|(_, v)| *v != 0)
            .collect();
        if nz.is_empty() {
            eprintln!("accepted, response all-zero");
        } else {
            eprintln!("accepted, {} nonzero words:", nz.len());
            for (off, v) in nz.iter().take(12) {
                eprintln!("    +{off:06x}: 0x{v:08X} ({v})");
            }
            hits.push((k, nz));
        }
    }

    eprintln!("=== summary: mask byte -> first record offset ===");
    for (k, nz) in &hits {
        eprintln!("  byte {k:#04x} (bit {}) -> first +{:#06x}", k, nz[0].0);
    }
    if hits.len() >= 2 {
        eprintln!("=== stride candidates (consecutive accepted bits) ===");
        for w in hits.windows(2) {
            eprintln!(
                "  bit {} (+{:#06x}) -> bit {} (+{:#06x}): delta bytes {}",
                w[0].0,
                w[0].1[0].0,
                w[1].0,
                w[1].1[0].0,
                w[1].1[0].0 as isize - w[0].1[0].0 as isize
            );
        }
    }
}

use core::ptr;
