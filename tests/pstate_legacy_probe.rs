// One-shot: which PerfPstatesGetInfoPrivate layout fires on this driver —
// V4 / legacy V3 (0x31A38) / legacy V1 (0x119C8) — plus, for the legacy
// view that lands, the per-pstate record header and the decoded SUB-TABLE
// (one 68B entry per ClkDomains bit, GPC first). READ-ONLY.
//
// Run: cargo test -p nvapi --test pstate_legacy_probe -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;
use nvapi::sys::api::NvAPI_GPU_PerfPstatesGetInfoPrivate;
use nvapi::sys::gpu::clock::undocumented::{
    PERF_PSTATES_INFO_PRIVATE_V1_LEGACY_LEN, PERF_PSTATES_INFO_PRIVATE_V1_LEGACY_MAGIC,
    PERF_PSTATES_INFO_PRIVATE_V3_LEGACY_LEN, PERF_PSTATES_INFO_PRIVATE_V3_LEGACY_MAGIC,
    perf_pstates_legacy_domain_clock, perf_pstates_legacy_mask, perf_pstates_legacy_record,
};
use nvapi::sys::handles::NvPhysicalGpuHandle;

/// V100 ClkDomains record names per bit (get-private-freq-domain-info).
const DOMAIN_NAMES: [&str; 10] = [
    "Gpc", "Xbar", "Mem", "Sys", "M", "Msd", "Disp", "Hotclk", "Pclk0", "Host",
];

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
        if mask & (1 << bit) == 0 {
            continue;
        }
        let (ty, min, max, pstate) = perf_pstates_legacy_record(&buf, bit);
        eprintln!("{tag}: record bit {bit}: P{pstate} type {ty} header min {min} max {max} kHz");
        eprintln!("{tag}:   sub-table (ClkDomains bit → nominal/live/max kHz):");
        for (domain, name) in DOMAIN_NAMES.iter().enumerate() {
            match perf_pstates_legacy_domain_clock(&buf, bit, domain) {
                Some((nominal, live, mx)) => eprintln!(
                    "{tag}:     bit{domain:<2} {:<7} {nominal:>7} / {live:>7} / {mx:>7} kHz",
                    name
                ),
                None => eprintln!("{tag}:     bit{domain:<2} {:<7} — absent —", name),
            }
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
