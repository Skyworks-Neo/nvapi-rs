//! A/B comparison of the ClockClientClkVfPoints GetStatus V1 vs V3 layouts
//! on the same GPU. V1 (magic 0x21C28, 7208B): header 68 (version+mask[8]
//! u32+pad[8] u32), entries @+0x44 stride 28 `{clock_type@+0, freq_kHz@+4,
//! voltage_uV@+8, pad[16]}` — IDA-verified against the R610.74 impl
//! converter sub_180200190 (`lea rdx,[user+0x48]; mov [rdx-4],type;
//! mov [rdx],freq; mov [rdx+4],volt`). V3 (0x35B0C, 88844B): header 104,
//! entries stride 348 with current/default pairs.
//!
//! History: an earlier revision of this probe used HDR=36 (missing the
//! 32-byte pad), which shifted its 28-byte grid 32≡4 (mod 28) off the true
//! entry grid — every "entry i" it printed was actually entry i−1's data
//! mis-sliced, producing the phantom `region` dword at +4 and the
//! "+1 shift at the core→mem boundary" artifact. This revision is anchored
//! at +68 per the disassembly.
//!
//! Run: `cargo run --release --example probe_vfp_v1v3_diff`

use nvapi::initialize;
use nvapi::sys::api::{NvAPI_EnumPhysicalGPUs, NvAPI_GPU_ClockClientClkVfPointsGetInfo, NvVersion, NvAPI_GPU_ClockClientClkVfPointsGetStatus};
use nvapi::sys::gpu::clock::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::nvapi::VersionedStruct;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use std::ptr;

/// V1 truth: version(4) + mask 8×u32(32) + pad 8×u32(32) = 68; stride 28.
const HDR_V1: usize = 68;
const S1: usize = 28;
/// V3 truth: version(4) + mask 8×u32(32) + pad 0x44(68) = 104; stride 348.
const HDR_V3: usize = 4 + 32 + 0x44;
const S3: usize = 348;

fn main() {
    let _ = initialize();
    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS as usize];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    let gpu = handles[0];

    // 1. GetInfo — mask builder (V1, 6188B)
    let mut info = Box::new(unsafe { std::mem::zeroed::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO>() });
    *info.nvapi_version_mut() = NvVersion::with_struct::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO>(1);
    let st = unsafe { NvAPI_GPU_ClockClientClkVfPointsGetInfo(gpu, ptr::from_mut(&mut *info)) };
    println!("GetInfo st={st} magic=0x{:X}", info.version.data);
    if st != 0 { return; }
    let mask_ptr = ptr::from_ref(&info.mask.mask) as *const u8;

    // 2. GetStatus V1 (7208B) — ver2 magic 0x21C28 (ver1 0x11C28 returns -9 live)
    let mut v1 = vec![0u8; 7208];
    v1[0..4].copy_from_slice(&0x21C28u32.to_le_bytes());
    unsafe { std::ptr::copy_nonoverlapping(mask_ptr, v1.as_mut_ptr().add(4), 32); }
    let st1 = unsafe { NvAPI_GPU_ClockClientClkVfPointsGetStatus(gpu, v1.as_mut_ptr() as *mut _) };
    println!("GetStatus V1(ver2) st={st1}");
    if st1 != 0 { println!("  V1 failed: {st1}"); return; }

    // 3. GetStatus V3 (88844B, ver3 0x35B0C)
    let mut v3 = vec![0u8; 88844];
    v3[0..4].copy_from_slice(&0x35B0Cu32.to_le_bytes());
    unsafe { std::ptr::copy_nonoverlapping(mask_ptr, v3.as_mut_ptr().add(4), 32); }
    let st3 = unsafe { NvAPI_GPU_ClockClientClkVfPointsGetStatus(gpu, v3.as_mut_ptr() as *mut _) };
    println!("GetStatus V3(ver3) st={st3}");
    if st3 != 0 { println!("  V3 failed: {st3}"); return; }

    let dw = |buf: &[u8], off: usize| u32::from_le_bytes(buf[off..off + 4].try_into().unwrap());

    // 4. first points from each, corrected layout
    println!("-- first V1 entries (type@+0, freq@+4, volt@+8) --");
    let mut shown = 0;
    for i in 0..255 {
        let o = HDR_V1 + S1 * i;
        if o + S1 > v1.len() { break; }
        let (ct, f, v) = (dw(&v1, o), dw(&v1, o + 4), dw(&v1, o + 8));
        if ct == 0 && f == 0 && v == 0 { continue; }
        println!("  V1[{}] type={} freq={}kHz volt={}uV", i, ct, f, v);
        shown += 1;
        if shown >= 5 { break; }
    }
    println!("-- first V3 entries (cur@+4/+8, def@+12/+16) --");
    shown = 0;
    for i in 0..255 {
        let o = HDR_V3 + S3 * i;
        if o + S3 > v3.len() { break; }
        let (ct, f, v) = (dw(&v3, o), dw(&v3, o + 4), dw(&v3, o + 8));
        if ct == 0 && f == 0 && v == 0 { continue; }
        println!("  V3[{}] type={} cur=({}kHz,{}uV) def=({},{})", i, ct, f, v,
            dw(&v3, o + 12), dw(&v3, o + 16));
        shown += 1;
        if shown >= 5 { break; }
    }

    // 5. cross-check: V1 (freq@+4/volt@+8) vs V3 current pair, per index
    let mut agree = 0usize;
    let mut disagree = 0usize;
    let mut first_disagree: Option<usize> = None;
    for i in 0..255 {
        let (o1, o3) = (HDR_V1 + S1 * i, HDR_V3 + S3 * i);
        if o1 + S1 > v1.len() || o3 + S3 > v3.len() { break; }
        let (v1_f, v1_v) = (dw(&v1, o1 + 4), dw(&v1, o1 + 8));
        let (v3_f, v3_v) = (dw(&v3, o3 + 4), dw(&v3, o3 + 8));
        if v1_f == 0 && v1_v == 0 { continue; }
        if v1_f == v3_f && v1_v == v3_v { agree += 1; } else {
            disagree += 1;
            if first_disagree.is_none() {
                first_disagree = Some(i);
                println!("first disagree @[{i}]: V1=({v1_f},{v1_v}) V3=({v3_f},{v3_v})");
            }
        }
    }
    println!("cross-check: {agree} agree, {disagree} disagree");

    // 6. clock_type census: the core(0)/mem(1) region tag
    let mut types: std::collections::BTreeMap<u32, (usize, usize, usize)> = Default::default();
    for i in 0..255 {
        let o = HDR_V1 + S1 * i;
        if o + S1 > v1.len() { break; }
        let (ct, f, v) = (dw(&v1, o), dw(&v1, o + 4), dw(&v1, o + 8));
        if ct == 0 && f == 0 && v == 0 { continue; }
        match types.get_mut(&ct) {
            Some((n, lo, hi)) => { *n += 1; *lo = (*lo).min(i); *hi = (*hi).max(i); }
            None => { types.insert(ct, (1, i, i)); }
        }
    }
    for (ct, (n, lo, hi)) in &types {
        println!("clock_type={ct}: {n} entries, idx {lo}..{hi}");
    }
}
