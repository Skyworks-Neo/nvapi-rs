//! RegisterOp lane cartography (0x2EB3C140) — READ-ONLY, requires elevation.
//!   cargo test -p nvapi --test register_op_map_live -- --nocapture --ignored
//!
//! Answers three questions with live numbers:
//! 1. Batch semantics — does one bad op poison a 256-op packet, or does the
//!    kernel report per-op status and continue? (decides the sweep cost model)
//! 2. Throughput — singles vs 256-op batches, wall clock.
//! 3. Window map — coarse sweep of the 32-bit offset space, then edge
//!    refinement around the valid/invalid boundary and the known windows.

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

/// One raw batch call; returns (call_rc, per-op (status, value) slices).
fn raw_batch(
    handle: sys::handles::NvPhysicalGpuHandle,
    offsets: &[u32],
) -> (sys::NvAPI_Status, Vec<(u16, u64)>) {
    let mut data = NV_GPU_REGISTER_OP_DATA_V1::default();
    data.op_count = offsets.len() as u32;
    for (slot, &offset) in data.op.iter_mut().zip(offsets).take(256) {
        slot.flags = R32G;
        slot.offset = offset;
    }
    let rc = unsafe { sys::api::NvAPI_GPU_RegisterOp(handle, &mut data) };
    let out = data
        .op
        .iter()
        .take(offsets.len())
        .map(|op| (op.status, op.value))
        .collect();
    (rc, out)
}

fn value_class(v: u64) -> &'static str {
    if v & 0xFFFF_0000 == 0xBADF_0000 {
        "poison"
    } else if v == 0 {
        "zero"
    } else {
        "data"
    }
}

#[test]
#[ignore = "live GPU read-only register cartography (elevated)"]
fn register_op_map() {
    nvapi::initialize().expect("init");
    let phys = &PhysicalGpu::enumerate().expect("enumerate")[0];
    let handle = *phys.handle();
    println!("== GPU 0: {} ==", phys.full_name().unwrap_or_default());

    // -- 1. Batch semantics: valid + beyond-gate op in one packet ----------
    let (rc, ops) = raw_batch(handle, &[0x0000_0000, 0x0200_0000, 0x0000_1000]);
    println!("[batch semantics] rc={rc}");
    for (i, (st, v)) in ops.iter().enumerate() {
        println!(
            "  op{i}: status={st:#06x} value={v:#x} ({})",
            value_class(*v)
        );
    }

    // -- 2. Throughput ------------------------------------------------------
    let t = Instant::now();
    let n = 512;
    for _ in 0..n {
        let _ = raw_batch(handle, &[0x0000_0000]);
    }
    let single_us = t.elapsed().as_micros() as f64 / n as f64;
    println!("[throughput] single-op call: {single_us:.1} us/call");

    let t = Instant::now();
    let batches = 8;
    for _ in 0..batches {
        let _ = raw_batch(handle, &[0x0000_0000; 256]);
    }
    let batch_us = t.elapsed().as_micros() as f64 / batches as f64;
    println!(
        "[throughput] 256-op batch: {batch_us:.1} us/call = {:.2} us/op",
        batch_us / 256.0
    );

    // -- 3. Coarse window map: 0x0..0x0800_0000 step 0x4_0000 (512 reads) ---
    let step = 0x4_0000u32;
    let limit = 0x0800_0000u32;
    let offsets: Vec<u32> = (0..limit).step_by(step as usize).collect();
    let t = Instant::now();
    let mut results = Vec::new();
    for chunk in offsets.chunks(256) {
        let (rc, ops) = raw_batch(handle, chunk);
        results.push((rc, chunk.to_vec(), ops));
    }
    let map_ms = t.elapsed().as_millis();
    println!(
        "[map] {} offsets in {map_ms} ms — rc/value classes per 0x{:x}-stride region:",
        offsets.len(),
        step
    );
    // Aggregate into runs with the same (rc-class, value-class) signature.
    let mut run_start = 0u32;
    let mut run_sig = String::new();
    let mut run_count = 0usize;
    let mut flush = |start: u32, sig: &str, count: usize| {
        if count > 0 {
            println!("  {start:#010x} x{count:<4} {sig}");
        }
    };
    for (rc, chunk, ops) in &results {
        for (i, off) in chunk.iter().enumerate() {
            let (st, v) = &ops[i];
            let sig = format!(
                "rc={rc} opst={st:#06x} {}",
                if *rc == 0 {
                    value_class(*v).to_string()
                } else {
                    "-".into()
                }
            );
            if sig == run_sig && *off == run_start + step * run_count as u32 {
                run_count += 1;
            } else {
                flush(run_start, &run_sig, run_count);
                run_start = *off;
                run_sig = sig;
                run_count = 1;
            }
        }
    }
    flush(run_start, &run_sig, run_count);

    // -- 4. Edge refinement: fine steps around 0x100_0000..0x200_0000 -------
    println!("[edge] step 0x1000 across 0x00F0_0000..0x0210_0000:");
    let edge: Vec<u32> = (0x00F0_0000u32..0x0210_0000).step_by(0x1000).collect();
    let mut edge_runs: Vec<(u32, String, usize)> = Vec::new();
    for chunk in edge.chunks(256) {
        let (rc, ops) = raw_batch(handle, chunk);
        for (i, off) in chunk.iter().enumerate() {
            let (st, v) = &ops[i];
            let sig = format!(
                "rc={rc} opst={st:#06x} {}",
                if rc == 0 {
                    value_class(*v).to_string()
                } else {
                    "-".into()
                }
            );
            match edge_runs.last_mut() {
                Some((start, s, c)) if *s == sig && *start + 0x1000 * (*c as u32) == *off => {
                    *c += 1;
                }
                _ => edge_runs.push((*off, sig, 1)),
            }
        }
    }
    for (start, sig, count) in &edge_runs {
        println!("  {start:#010x} x{count:<4} {sig}");
    }
}
