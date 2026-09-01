// V100/GV100: V/F-POINTS CONTROL layout probe — READ-ONLY echo mapping.
//
// R610 semantics (IDA sub_180215FC0): GetControl with a snapshot magic
// internally allocates the full table, fills it from current driver state,
// then COPIES THE USER'S MASKS AND RECORDS OVER IT before returning. A
// fully-marker-filled input is REJECTED with -1 (validation), so phase 1
// bisects how much of the buffer head must stay zero for acceptance —
// that boundary IS the validated region (mask/head). Phase 2 echoes only
// past that boundary and diffs: echoed words = the user-copyable region
// (record fields the driver preserves), cleared words = driver-owned.
// No driver state is modified (GetControl is a read).
//
// Run: cargo test -p nvapi --test volta_vfp_control_echo -- --nocapture --ignored

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

fn marker_buf() -> Vec<u8> {
    let mut markers = vec![0u8; SCAN];
    for (i, chunk) in markers.chunks_mut(4).enumerate() {
        let v = 0x5A5A_0000u32 + i as u32;
        chunk.copy_from_slice(&v.to_le_bytes());
    }
    markers
}

#[test]
#[ignore]
fn volta_vfp_control_echo_map() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    eprintln!("GPU: {:?}", gpu.full_name());

    let markers = marker_buf();

    // Phase 1: how much head must be zero? (mask/head validated region)
    eprintln!("=== phase 1: prefix-zero bisection (zeros[0..P] + markers[P..]) ===");
    let mut accepted_p = None;
    for p in [
        0x04usize, 0x08, 0x10, 0x14, 0x18, 0x20, 0x40, 0x60, 0x80, 0x100, 0x200,
    ] {
        let mut input = markers.clone();
        input[..p].fill(0);
        eprint!("  P={p:#06x}: ");
        if get_with(gpu, &input).is_some() {
            eprintln!("    ACCEPTED");
            if accepted_p.is_none() {
                accepted_p = Some(p);
            }
        }
    }

    let p = match accepted_p {
        Some(p) => p,
        None => {
            eprintln!("no prefix accepted — even 0x200 zeros rejected; aborting");
            return;
        }
    };
    eprintln!("minimal accepted prefix: {p:#x} bytes");

    // Phase 2: echo diff at that prefix
    eprintln!("=== phase 2: echo diff with zeros[0..{p:#x}] ===");
    let mut input = markers.clone();
    input[..p].fill(0);
    let Some(resp) = get_with(gpu, &input) else {
        return;
    };

    let mut echoed = 0usize;
    let mut cleared = 0usize;
    let mut other = 0usize;
    let mut runs: Vec<(usize, usize, u8)> = Vec::new(); // (start_word, len, kind)
    for i in p / 4..SCAN / 4 {
        let off = i * 4;
        let m = u32::from_le_bytes(markers[off..off + 4].try_into().unwrap());
        let r = u32::from_le_bytes(resp[off..off + 4].try_into().unwrap());
        let kind = if r == m {
            echoed += 1;
            b'E' // echoed back verbatim
        } else if r == 0 {
            cleared += 1;
            b'.' // driver cleared our marker
        } else {
            other += 1;
            b'X' // driver wrote its own value
        };
        match runs.last_mut() {
            Some((_, len, k)) if *k == kind => *len += 1,
            _ => runs.push((i, 1, kind)),
        }
    }
    eprintln!(
        "words (from +{p:#x}): echoed {echoed}, cleared {cleared}, other {other} (total {})",
        SCAN / 4 - p / 4
    );
    eprintln!("=== region runs (start_byte kind len_words) — E=echo .=cleared X=driver-value ===");
    let mut shown = 0usize;
    for (start, len, kind) in &runs {
        if *len < 2 && *kind != b'X' {
            continue;
        }
        eprintln!("  +{:06x} {} x{}", start * 4, *kind as char, len);
        shown += 1;
        if shown > 80 {
            eprintln!("  ... (truncated)");
            break;
        }
    }

    // X words: driver-derived values are the interesting ones
    eprintln!("=== X-word details (first 40) ===");
    let mut shown = 0usize;
    for (start, len, kind) in &runs {
        if *kind != b'X' {
            continue;
        }
        for i in *start..*start + *len {
            let off = i * 4;
            let r = u32::from_le_bytes(resp[off..off + 4].try_into().unwrap());
            eprintln!("  +{off:06x}: driver={r:#x} ({r})");
            shown += 1;
            if shown > 40 {
                return;
            }
        }
    }
}

use core::ptr;
