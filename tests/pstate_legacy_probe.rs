// One-shot: which PerfPstatesGetInfoPrivate layout fires on this driver —
// V4 / legacy V3 (0x31A38) / legacy V1 (0x119C8) — plus the raw decoded
// records of whichever legacy view lands. READ-ONLY.
//
// Run: cargo test -p nvapi --test pstate_legacy_probe -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;
use nvapi::sys::api::NvAPI_GPU_PerfPstatesGetInfoPrivate;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::gpu::clock::undocumented::{
    perf_pstates_legacy_mask, perf_pstates_legacy_record,
    PERF_PSTATES_INFO_PRIVATE_V1_LEGACY_LEN, PERF_PSTATES_INFO_PRIVATE_V1_LEGACY_MAGIC,
    PERF_PSTATES_INFO_PRIVATE_V3_LEGACY_LEN, PERF_PSTATES_INFO_PRIVATE_V3_LEGACY_MAGIC,
};

fn probe(tag: &str, len: usize, magic: u32) -> bool {
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    let mut buf = vec![0u8; len];
    buf[..4].copy_from_slice(&magic.to_ne_bytes());
    let handle: NvPhysicalGpuHandle =
        unsafe { std::mem::transmute(gpu.handle().as_ptr()) };
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
            let (ty, min, max, pstate) = perf_pstates_legacy_record(&buf, bit);
            eprintln!(
                "{tag}: bit {bit:2} → P{pstate} type {ty} min {min} kHz max {max} kHz"
            );
            // Nonzero-dword scan of this record's first 1024 bytes (header
            // + sub-table region) to locate the real clock fields.
            let base = 72 + 2252 * bit as usize;
            let mut run_start: Option<usize> = None;
            for off in 0..1024usize {
                let dw = u32::from_ne_bytes(buf[base + off * 4..base + off * 4 + 4].try_into().unwrap());
                let nz = dw != 0;
                match (run_start, nz) {
                    (None, true) => run_start = Some(off),
                    (Some(s), false) => {
                        eprintln!("{tag}:   record +{}..+{} dwords nonzero", s * 4, off * 4);
                        run_start = None;
                    }
                    _ => {}
                }
                if nz && (off < 16 || off % 17 == 3 || off % 17 == 0 || (36..44).contains(&(off % 17))) {
                    eprintln!(
                        "{tag}:   +{:4} (dword {:3}): {:#010X} ({})",
                        off * 4, off, dw, dw
                    );
                }
            }
            if let Some(s) = run_start {
                eprintln!("{tag}:   record +{}..+1024 dwords nonzero", s * 4);
            }
        }
    }
    true
}

#[test]
#[ignore]
fn pstate_legacy_probe() {
    nvapi::initialize().expect("init");
    if probe("V3", PERF_PSTATES_INFO_PRIVATE_V3_LEGACY_LEN, PERF_PSTATES_INFO_PRIVATE_V3_LEGACY_MAGIC) {
        return;
    }
    probe("V1", PERF_PSTATES_INFO_PRIVATE_V1_LEGACY_LEN, PERF_PSTATES_INFO_PRIVATE_V1_LEGACY_MAGIC);
}
