// One-shot: which PerfPstatesGetInfoPrivate layout fires on this driver —
// V4 / legacy V3 (0x31A38) / legacy V1 (0x119C8) — plus a per-68B-entry
// dump of the present pstate records (header + sub-table region) for the
// legacy view that lands. READ-ONLY.
//
// Run: cargo test -p nvapi --test pstate_legacy_probe -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;
use nvapi::sys::api::NvAPI_GPU_PerfPstatesGetInfoPrivate;
use nvapi::sys::gpu::clock::undocumented::{
    PERF_PSTATES_INFO_PRIVATE_V1_LEGACY_LEN, PERF_PSTATES_INFO_PRIVATE_V1_LEGACY_MAGIC,
    PERF_PSTATES_INFO_PRIVATE_V3_LEGACY_LEN, PERF_PSTATES_INFO_PRIVATE_V3_LEGACY_MAGIC,
    perf_pstates_legacy_mask, perf_pstates_legacy_record,
};
use nvapi::sys::handles::NvPhysicalGpuHandle;

/// Dump one legacy record: header summary, then the record body as
/// ENTRY_STRIDE-byte rows with every nonzero dword annotated.
fn dump_record(buf: &[u8], bit: u32) {
    let (ty, min, max, pstate) = perf_pstates_legacy_record(buf, bit);
    eprintln!("P{pstate} type {ty} header min {min} max {max} kHz");
    let base = 72 + 2252 * bit as usize;
    const ENTRY_STRIDE: usize = 68;
    const N_ENTRIES: usize = 16;
    eprintln!("  body as {N_ENTRIES} × {ENTRY_STRIDE}B entries (base +72):");
    for k in 0..N_ENTRIES {
        let ebase = base + 72 + k * ENTRY_STRIDE;
        let dws: Vec<(usize, u32)> = (0..ENTRY_STRIDE / 4)
            .map(|i| {
                (
                    i * 4,
                    u32::from_ne_bytes(buf[ebase + i * 4..ebase + i * 4 + 4].try_into().unwrap()),
                )
            })
            .filter(|(_, v)| *v != 0)
            .collect();
        if dws.is_empty() {
            continue;
        }
        let fields = dws
            .iter()
            .map(|(o, v)| format!("+{o}:{v} ({v:#x})"))
            .collect::<Vec<_>>()
            .join("  ");
        eprintln!("  entry[{k:2}] @+{:4}: {}", 72 + k * ENTRY_STRIDE, fields);
    }
}

fn probe(tag: &str, len: usize, magic: u32) -> bool {
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    let mut buf = vec![0u8; len];
    buf[..4].copy_from_slice(&magic.to_ne_bytes());
    let handle: NvPhysicalGpuHandle = unsafe { std::mem::transmute(gpu.handle().as_ptr()) };
    let st = unsafe { NvAPI_GPU_PerfPstatesGetInfoPrivate(handle, buf.as_mut_ptr().cast()) };
    eprintln!("{tag}: status {st} ({len} B, magic 0x{magic:X})");
    if st != 0 {
        return false;
    }
    let mask = perf_pstates_legacy_mask(&buf);
    eprintln!("{tag}: table_version byte @8 = {:#x}", buf[8]);
    eprintln!("{tag}: mask = 0x{mask:08X}");
    for bit in 0..32u32 {
        if mask & (1 << bit) != 0 {
            eprintln!("{tag}: record for mask bit {bit}:");
            dump_record(&buf, bit);
        }
    }
    true
}

#[test]
#[ignore]
fn pstate_legacy_probe() {
    nvapi::initialize().expect("init");
    if probe(
        "V3",
        PERF_PSTATES_INFO_PRIVATE_V3_LEGACY_LEN,
        PERF_PSTATES_INFO_PRIVATE_V3_LEGACY_MAGIC,
    ) {
        return;
    }
    probe(
        "V1",
        PERF_PSTATES_INFO_PRIVATE_V1_LEGACY_LEN,
        PERF_PSTATES_INFO_PRIVATE_V1_LEGACY_MAGIC,
    );
}
