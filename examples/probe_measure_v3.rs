//! Raw V3 batch MEASURE_FREQ probe — print the driver's status for the
//! 0x30038 magic and, if OK, the per-entry {counter, timestamp, extra}.
use nvapi::initialize;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use nvapi::sys::api::{NvAPI_EnumPhysicalGPUs, NvAPI_GPU_ClockCounterMeasureAvgFreq};
use nvapi::sys::gpu::clock::private::NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE3;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use std::ptr;

fn main() {
    let _ = initialize();
    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    let gpu = handles[0];

    let domains = [0u32, 1, 2, 4, 5];
    let mut m = NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE3::default();
    m.version = nvapi::sys::api::NvVersion::new(0x178, 3);
    m.set_count(domains.len() as u8);
    for (i, &d) in domains.iter().enumerate() {
        m.set_entry(i, d, 0, 0).unwrap();
    }
    let st = unsafe { NvAPI_GPU_ClockCounterMeasureAvgFreq(gpu, ptr::from_mut(&mut m).cast()) };
    println!("V3 st={st}");
    // RMW second sample: seed each entry with the FIRST call's raw output
    let mut m2 = NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE3::default();
    m2.version = nvapi::sys::api::NvVersion::new(0x178, 3);
    m2.set_count(domains.len() as u8);
    for (i, &d) in domains.iter().enumerate() {
        m2.set_entry(i, d, 0, 0).unwrap();
        m2.entries[24 * i..24 * i + 24].copy_from_slice(&m.entries[24 * i..24 * i + 24]);
    }
    std::thread::sleep(std::time::Duration::from_millis(50));
    let st2 = unsafe { NvAPI_GPU_ClockCounterMeasureAvgFreq(gpu, ptr::from_mut(&mut m2).cast()) };
    println!("V3 second-sample st={st2}");
    // candidate per-entry u32 fields: +4 (extra), +8/+12 (q1 halves), +16 low, +20
    for i in 0..domains.len() {
        let f = |mm: &NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE3, k: usize| {
            let off = 24 * i + k - 4;
            u32::from_le_bytes(mm.entries[off..off + 4].try_into().unwrap())
        };
        let ts = |mm: &NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE3| {
            let off = 24 * i + 16 - 4;
            u64::from_le_bytes(mm.entries[off..off + 8].try_into().unwrap())
        };
        let dt = (ts(&m2) as f64 - ts(&m) as f64).max(1.0);
        let cands: Vec<f64> = [4usize, 8, 12, 16, 20]
            .iter()
            .map(|&k| {
                let d = f(&m2, k) as f64 - f(&m, k) as f64;
                d / dt * 1e9 / 1e6
            })
            .collect();
        println!(
            "domain {}: MHz candidates (+4,+8,+12,+16,+20) = {:.1} {:.1} {:.1} {:.1} {:.1}",
            domains[i], cands[0], cands[1], cands[2], cands[3], cands[4]
        );
    }
    // also try the documented-baseline size as the magic's high word
    // variant probe: some drivers want 0x30038 verbatim; ours stamps
    // 3<<16|0x38 = 0x30038 — same. Sanity-print the stamped dword:
    println!("stamped magic: 0x{:X}", m.version.data);
}
