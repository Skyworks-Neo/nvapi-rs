// V100/GV100: CONTROL record field map — READ-ONLY.
//
// Established: CONTROL mask = bitfield (bit r = byte r/8 bit r%8) over the
// same 132-point space as LEGACY STATUS; record r lives at +0x60 + r*0x44
// (68B); stock fill = flags dword = 1 at record base only. This probe
// selects records AND fills their windows with per-word markers: offsets
// the driver rewrites = owned fields (the mode/value candidates), offsets
// echoed = passthrough. Finishes with an all-zero GET that must be
// all-zero again (asserts driver state untouched — pure scratch echo).
//
// Run: cargo test -p nvapi --test volta_vfp_control_record_map -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;
use nvapi::sys::api::NvAPI_GPU_ClockClkVfPointsGetControl;
use nvapi::sys::gpu::clock::undocumented::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE;
use nvapi::sys::nvapi::NvVersion;

const SNAPSHOT_MAGIC: u32 = 82976;
const REC_BASE: usize = 0x60;
const REC_STRIDE: usize = 0x44;
const SCAN: usize = 64 * 1024; // covers 132*0x44+0x60 = 0x22E0 many times over

fn set_bit(mask: &mut [u8], r: usize) {
    mask[r / 8] |= 1 << (r % 8);
}

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

/// markers for record r's window; returns (input, expected marker words)
fn marker_window(r: usize) -> (Vec<u8>, Vec<(usize, u32)>) {
    let mut input = vec![0u8; SCAN];
    set_bit(&mut input, r);
    let mut exp = Vec::new();
    for w in 0..REC_STRIDE / 4 {
        let off = REC_BASE + r * REC_STRIDE + w * 4;
        let v = 0xC0DE_0000u32 + (r * 0x100 + w) as u32;
        input[off..off + 4].copy_from_slice(&v.to_le_bytes());
        exp.push((off, v));
    }
    (input, exp)
}

fn diff_record(resp: &[u8], exp: &[(usize, u32)], r: usize) {
    let base = REC_BASE + r * REC_STRIDE;
    eprintln!(
        "  record {r} window +{base:05x}..+{:05x}:",
        base + REC_STRIDE
    );
    for (off, m) in exp {
        let d = u32::from_le_bytes(resp[*off..*off + 4].try_into().unwrap());
        let tag = if d == *m { "echo" } else { "OWNED" };
        if tag == "OWNED" {
            eprintln!(
                "    +{:05x} (rec+{:02x}): driver=0x{d:08X} ({d})  <== {tag}",
                off,
                off - base
            );
        }
    }
    // anything nonzero outside the window?
    for i in 0..SCAN / 4 {
        let off = i * 4;
        if off >= base && off < base + REC_STRIDE {
            continue;
        }
        let d = u32::from_le_bytes(resp[off..off + 4].try_into().unwrap());
        if d != 0 {
            eprintln!("    OUTSIDE +{off:05x}: 0x{d:08X} ({d})");
        }
    }
}

#[test]
#[ignore]
fn volta_vfp_control_record_field_map() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    eprintln!("GPU: {:?}", gpu.full_name());

    // staged single records
    for r in [0usize, 1, 127, 128, 131] {
        eprintln!("=== single record {r} ===");
        let (input, exp) = marker_window(r);
        let Some(resp) = get_with(gpu, &input) else {
            continue;
        };
        diff_record(&resp, &exp, r);
    }

    // full sweep: every present bit + markers in every window
    eprintln!("=== full sweep: bits 0..=131, all windows marked ===");
    let mut input = vec![0u8; SCAN];
    let mut exp: Vec<(usize, u32)> = Vec::new();
    for r in 0..=131usize {
        set_bit(&mut input, r);
        for w in 0..REC_STRIDE / 4 {
            let off = REC_BASE + r * REC_STRIDE + w * 4;
            let v = 0xC0DE_0000u32 + (r * 0x100 + w) as u32;
            input[off..off + 4].copy_from_slice(&v.to_le_bytes());
            exp.push((off, v));
        }
    }
    let Some(resp) = get_with(gpu, &input) else {
        return;
    };

    // aggregate: which rec-relative offsets are driver-owned, per record
    let mut owned_by_offset: std::collections::BTreeMap<usize, Vec<(u32, u32)>> =
        std::collections::BTreeMap::new();
    for (off, m) in &exp {
        let d = u32::from_le_bytes(resp[*off..*off + 4].try_into().unwrap());
        if d != *m {
            let r = (off - REC_BASE) / REC_STRIDE;
            owned_by_offset
                .entry((off - REC_BASE) % REC_STRIDE)
                .or_default()
                .push((r as u32, d));
        }
    }
    eprintln!("owned rec-relative offsets (across 132 records):");
    for (rel, vals) in &owned_by_offset {
        let uniq: std::collections::BTreeSet<u32> = vals.iter().map(|(_, d)| *d).collect();
        eprintln!(
            "  rec+{rel:#04x}: owned in {}/132 records, distinct values {:?}",
            vals.len(),
            uniq.iter().take(8).collect::<Vec<_>>()
        );
    }

    // per-record fill: first owned value per record (the flags field?)
    eprintln!("first 20 records, rec+0 driver value:");
    for r in 0..20usize {
        let off = REC_BASE + r * REC_STRIDE;
        let d = u32::from_le_bytes(resp[off..off + 4].try_into().unwrap());
        eprintln!("  record {r}: rec+0 = 0x{d:08X} ({d})");
    }

    // state-untouched assertion: all-zero input must still return all-zero
    eprintln!("=== post-probe stock check (must be all-zero) ===");
    let zeros = vec![0u8; SCAN];
    match get_with(gpu, &zeros) {
        Some(resp) => {
            let nz = resp.chunks_exact(4).filter(|c| c != &[0; 4]).count();
            eprintln!("nonzero words after all probes: {nz} (expect 0)");
            assert_eq!(nz, 0, "driver state changed — INVESTIGATE BEFORE ANY WRITE");
        }
        None => panic!("stock GET failed after probes"),
    }
}

use core::ptr;
