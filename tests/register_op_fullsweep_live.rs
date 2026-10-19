//! Full-sweep feasibility probe — READ-ONLY, elevated.
//!   cargo test -p nvapi --test register_op_fullsweep_live -- --nocapture --ignored
//!
//! Sweeps the whole valid window (0x0..0x100_0000, dword granularity, 4.19M
//! reads) in 256-op batches; on a failed batch falls back to singles so one
//! bad slot cannot hide the rest. Classifies every dword:
//!   data (real value) / zero / poison (0xBADF****) / fail (rc!=0).
//! Output: histogram, per-64KiB-region data profile, and the run list of
//! data-bearing stretches (what "readable windows" means in practice).

use std::time::Instant;

use nvapi::PhysicalGpu;
use nvapi::sys;
use nvapi::sys::gpu::register::undocumented::{
    NV_GPU_REGISTER_OP_DATA_V1, NV_GPU_REGISTER_OP_FLAG_32BIT, NV_GPU_REGISTER_OP_FLAG_READ,
    NV_GPU_REGISTER_OP_FLAG_TYPE_GLOBAL,
};

const R32G: u16 = NV_GPU_REGISTER_OP_FLAG_READ
    | NV_GPU_REGISTER_OP_FLAG_32BIT
    | NV_GPU_REGISTER_OP_FLAG_TYPE_GLOBAL;

const CLASS_ZERO: u8 = 1;
const CLASS_POISON: u8 = 2;
const CLASS_FAIL: u8 = 3;
const CLASS_DATA: u8 = 4;

fn raw_single(handle: sys::handles::NvPhysicalGpuHandle, offset: u32) -> (sys::NvAPI_Status, u64) {
    let mut data = NV_GPU_REGISTER_OP_DATA_V1::default();
    data.op_count = 1;
    data.op[0].flags = R32G;
    data.op[0].offset = offset;
    let rc = unsafe { sys::api::NvAPI_GPU_RegisterOp(handle, &mut data) };
    (rc, data.op[0].value)
}

fn classify(v: u64) -> u8 {
    if v == 0 {
        CLASS_ZERO
    } else if v & 0xFFFF_0000 == 0xBADF_0000 {
        CLASS_POISON
    } else {
        CLASS_DATA
    }
}

#[test]
#[ignore = "live full-window register sweep (elevated, read-only)"]
fn fullsweep_run() {
    nvapi::initialize().expect("init");
    let phys = &PhysicalGpu::enumerate().expect("enumerate")[0];
    let handle = *phys.handle();
    println!("== GPU 0: {} ==", phys.full_name().unwrap_or_default());

    const LIMIT: u32 = 0x100_0000;
    let slots = (LIMIT / 4) as usize;
    let mut class = vec![0u8; slots];
    let mut data_vals: Vec<(u32, u64)> = Vec::new();

    let t = Instant::now();
    let mut failed_batches = 0usize;
    let mut singles = 0usize;
    let mut batch_buf = [0u32; 256];

    for chunk_start in (0u32..LIMIT).step_by(256 * 4) {
        for (i, slot) in batch_buf.iter_mut().enumerate() {
            *slot = chunk_start + (i as u32) * 4;
        }
        let mut data = NV_GPU_REGISTER_OP_DATA_V1::default();
        data.op_count = 256;
        for (slot_in, &off) in data.op.iter_mut().zip(batch_buf.iter()) {
            slot_in.flags = R32G;
            slot_in.offset = off;
        }
        let rc = unsafe { sys::api::NvAPI_GPU_RegisterOp(handle, &mut data) };
        if rc == 0 {
            for (i, op) in data.op.iter().enumerate() {
                let idx = ((chunk_start + (i as u32) * 4) / 4) as usize;
                class[idx] = classify(op.value);
                if class[idx] == CLASS_DATA {
                    data_vals.push((chunk_start + (i as u32) * 4, op.value));
                }
            }
        } else {
            failed_batches += 1;
            for &off in &batch_buf {
                singles += 1;
                let (rc1, v) = raw_single(handle, off);
                class[(off / 4) as usize] = if rc1 == 0 {
                    let c = classify(v);
                    if c == CLASS_DATA {
                        data_vals.push((off, v));
                    }
                    c
                } else {
                    CLASS_FAIL
                };
            }
        }
    }
    let elapsed = t.elapsed();

    let (mut n_zero, mut n_poison, mut n_fail, mut n_pending) = (0usize, 0usize, 0usize, 0usize);
    for &c in &class {
        match c {
            CLASS_ZERO => n_zero += 1,
            CLASS_POISON => n_poison += 1,
            CLASS_FAIL => n_fail += 1,
            0 => n_pending += 1,
            _ => {}
        }
    }
    let n_data = slots - n_zero - n_poison - n_fail - n_pending;
    println!(
        "[hist] total {slots} dwords: data {n_data} | zero {n_zero} | poison {n_poison} | fail {n_fail} | pending {n_pending} \
         (failed batches {failed_batches}, fallback singles {singles})"
    );
    println!(
        "[time] full sweep: {:.2?} ({:.3} us/op)",
        elapsed,
        elapsed.as_micros() as f64 / slots as f64
    );

    // Per-64KiB region profile: only regions carrying any data.
    println!("[regions 0x10000] data dwords / region size — regions with data:");
    const REGION: usize = 0x1_0000 / 4;
    for (ri, region) in class.chunks(REGION).enumerate() {
        let (mut d, mut p, mut z, mut f) = (0usize, 0usize, 0usize, 0usize);
        for &c in region {
            match c {
                CLASS_ZERO => z += 1,
                CLASS_POISON => p += 1,
                CLASS_FAIL => f += 1,
                CLASS_DATA => d += 1,
                _ => {}
            }
        }
        if d > 0 {
            println!(
                "  {:#08x}: data {d:<6} ({:.1}%)  poison {p:<6} zero {z:<6} fail {f}",
                ri * 0x1_0000,
                100.0 * d as f64 / region.len() as f64
            );
        }
    }

    // Run-length list of data-bearing stretches (first 100 runs).
    println!("[data runs] (start..end bytes) — first 100:");
    let mut runs: Vec<(u32, u32)> = Vec::new();
    let mut cur: Option<(u32, u32)> = None;
    for (i, &c) in class.iter().enumerate() {
        if c == CLASS_DATA {
            let off = (i as u32) * 4;
            match &mut cur {
                Some((_, end)) if *end == off => *end = off + 4,
                Some(done) => {
                    runs.push(*done);
                    cur = Some((off, off + 4));
                }
                None => cur = Some((off, off + 4)),
            }
        }
    }
    if let Some(done) = cur.take() {
        runs.push(done);
    }
    for (start, end) in runs.iter().take(100) {
        let sample = data_vals
            .iter()
            .find(|(off, _)| *off >= *start && *off < *end)
            .map(|(_, v)| format!("{v:#x}"))
            .unwrap_or_default();
        println!(
            "  {start:#010x}..{end:#010x} ({:>6} B) sample {sample}",
            end - start
        );
    }
    if runs.len() > 100 {
        println!("  … {} more runs", runs.len() - 100);
    }
    println!(
        "[total] {} data runs, {} data dwords",
        runs.len(),
        data_vals.len()
    );
}
