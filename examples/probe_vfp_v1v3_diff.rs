//! A/B comparison of the ClockClientClkVfPoints GetStatus V1 vs V3 layouts
//! on the same GPU. Two independent third-party OC tools (AmpereOC for 30-series,
//! HYDRA 2.2B PRO for 50-series) BOTH drive the V1 path (magic 0x1C28, 7208B,
//! 28-byte entry stride: clock_type/freq_kHz/voltage_uV/unknown[4]). nvapi-rs
//! defaults to V3 (0x15B0C, 88588B, 348-byte entries with default+overclocked
//! point pairs) and only falls back to V1 if V3 fails.
//!
//! This probe runs BOTH unconditionally and dumps the first few decoded points
//! from each, plus a nonzero-dword census, to see whether V1 carries data V3
//! loses (or vice-versa) on Ada — and to validate the 28-byte stride.
//!
//! Run: `cargo run --release --example probe_vfp_v1v3_diff`

use nvapi::initialize;
use nvapi::sys::api::{NvAPI_EnumPhysicalGPUs, NvAPI_GPU_ClockClientClkVfPointsGetInfo, NvVersion, NvAPI_GPU_ClockClientClkVfPointsGetStatus};
use nvapi::sys::gpu::clock::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO;
use nvapi::sys::gpu::power::private::{NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_V1, NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS};
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::nvapi::VersionedStruct;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use std::ptr;

fn dump_v1(buf: &[u8]) {
    // header: version(4) + mask(32) = +36; entries @ +36, stride 28
    const HDR: usize = 36;
    const STRIDE: usize = 28;
    let mut nz = 0usize;
    for c in buf[4..].chunks(4).skip(1) { if c.iter().any(|&b| b != 0) { nz += 1; } }
    println!("  V1: size={} nonzero_dwords={} magic=0x{:X}", buf.len(), nz, u32::from_le_bytes(buf[0..4].try_into().unwrap()));
    let mut shown = 0;
    for i in 0..255 {
        let off = HDR + STRIDE * i;
        if off + STRIDE > buf.len() { break; }
        let e = &buf[off..off + STRIDE];
        let ct = u32::from_le_bytes(e[0..4].try_into().unwrap());
        let _rsv = u32::from_le_bytes(e[4..8].try_into().unwrap());   // @+4 reserved
        let freq = u32::from_le_bytes(e[8..12].try_into().unwrap());   // @+8
        let volt = u32::from_le_bytes(e[12..16].try_into().unwrap()); // @+12
        let unk = u32::from_le_bytes(e[16..20].try_into().unwrap());
        if ct == 0 && freq == 0 && volt == 0 && unk == 0 { continue; }
        if shown < 6 {
            let mhz = freq as f64 / 1000.0;
            let mv = volt as f64 / 1000.0;
            println!("    [{}] type={} freq={}kHz({:.1}MHz) volt={}uV({:.1}mV)",
                i, ct, freq, mhz, volt, mv);
            shown += 1;
        }
    }
}

fn dump_v3(buf: &[u8]) {
    // header: version(4) + mask(32) + unknown[0x44] = +140; entries @ +140, stride 348
    const HDR: usize = 4 + 32 + 0x44;
    const STRIDE: usize = 348;
    let mut nz = 0usize;
    for c in buf[4..].chunks(4).skip(1) { if c.iter().any(|&b| b != 0) { nz += 1; } }
    println!("  V3: size={} nonzero_dwords={} magic=0x{:X}", buf.len(), nz, u32::from_le_bytes(buf[0..4].try_into().unwrap()));
    let mut shown = 0;
    for i in 0..255 {
        let off = HDR + STRIDE * i;
        if off + STRIDE > buf.len() { break; }
        let e = &buf[off..off + STRIDE];
        let ct = u32::from_le_bytes(e[0..4].try_into().unwrap());
        let f0 = u32::from_le_bytes(e[4..8].try_into().unwrap());
        let v0 = u32::from_le_bytes(e[8..12].try_into().unwrap());
        if ct == 0 && f0 == 0 && v0 == 0 { continue; }
        if shown < 6 {
            let fd = u32::from_le_bytes(e.get(20..24).unwrap_or(&[0;4]).try_into().unwrap());
            let f1 = u32::from_le_bytes(e.get(24..28).unwrap_or(&[0;4]).try_into().unwrap());
            let v1 = u32::from_le_bytes(e.get(28..32).unwrap_or(&[0;4]).try_into().unwrap());
            println!("    [{}] type={} cur=({}kHz,{}uV) def=({},{}) oc=({},{})",
                i, ct, f0, v0, fd, v1, f1, v1);
            shown += 1;
        }
    }
}

/// Scan every V1 entry's 28 bytes for non-padding dwords beyond freq/volt.
fn scan_v1_full(buf: &[u8]) {
    const HDR: usize = 36;
    const S: usize = 28;
    let mut nonzero_tail = 0usize; // entries with nonzero dwords[4..6] (padding region)
    let mut nonzero_rsv = 0usize;  // entries with nonzero reserved@dword[1]
    let mut first_tail: Option<usize> = None;
    for i in 0..255 {
        let o = HDR + S * i;
        if o + S > buf.len() { break; }
        let dw = |j: usize| u32::from_le_bytes(buf[o+j*4..o+j*4+4].try_into().unwrap());
        let freq = dw(2); let volt = dw(3);
        if freq == 0 && volt == 0 && dw(0) == 0 { continue; } // empty
        if dw(1) != 0 { nonzero_rsv += 1; }
        // tail = dwords[4],[5],[6]
        let tail = [dw(4), dw(5), dw(6)];
        if tail.iter().any(|&v| v != 0) {
            nonzero_tail += 1;
            if first_tail.is_none() {
                println!("  V1[{}] tail non-zero: dword[4]={:#X} [5]={:#X} [6]={:#X} (freq={} volt={})",
                    i, tail[0], tail[1], tail[2], freq, volt);
            }
            if first_tail.is_none() { first_tail = Some(i); }
        }
    }
    println!("  V1 scan: {} entries with non-zero reserved@dword[1], {} with non-zero tail padding[4..6]",
        nonzero_rsv, nonzero_tail);
    if nonzero_rsv > 0 {
        // re-scan printing ALL entries with non-zero reserved
        for i in 0..255 {
            let o = HDR + S * i;
            if o + S > buf.len() { break; }
            let dw = |j: usize| u32::from_le_bytes(buf[o+j*4..o+j*4+4].try_into().unwrap());
            if dw(2) == 0 && dw(3) == 0 && dw(0) == 0 { continue; }
            if dw(1) != 0 {
                println!("    V1[{}] dword[1]={:#X}({}) freq={} volt={}", i, dw(1), dw(1), dw(2), dw(3));
            }
        }
    }
}

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
    let info_bytes = unsafe { std::slice::from_raw_parts(ptr::from_ref(&*info).cast::<u8>(), 6188) };
    let mask_ptr = unsafe { (ptr::from_ref(&info.mask.mask) as *const u8) };

    // 2. GetStatus V1 (7208B) — only ver2 (0x21C28) accepted (ver1 0x1C28 fails -9)
    let mut v1 = vec![0u8; 7208];
    v1[0..4].copy_from_slice(&0x21C28u32.to_le_bytes());
    unsafe { std::ptr::copy_nonoverlapping(mask_ptr, v1.as_mut_ptr().add(4), 32); }
    let st1 = unsafe { NvAPI_GPU_ClockClientClkVfPointsGetStatus(gpu, v1.as_mut_ptr() as *mut _) };
    println!("GetStatus V1(ver2) st={st1}");
    if st1 == 0 { dump_v1(&v1); scan_v1_full(&v1); } else { println!("  V1 failed: {st1}"); }

    // 3. GetStatus V3 (88844B, ver3 0x35B0C)
    let mut v3 = vec![0u8; 88844];
    v3[0..4].copy_from_slice(&0x35B0Cu32.to_le_bytes());
    unsafe { std::ptr::copy_nonoverlapping(mask_ptr, v3.as_mut_ptr().add(4), 32); }
    let st3 = unsafe { NvAPI_GPU_ClockClientClkVfPointsGetStatus(gpu, v3.as_mut_ptr() as *mut _) };
    println!("GetStatus V3(ver3) st={st3}");
    if st3 == 0 { dump_v3(&v3); } else { println!("  V3 failed: {st3}"); }

    // 4. compare V1[128..131] (the dword[1]=1 outliers) with V3 same indices
    if st1 == 0 && st3 == 0 {
        const HDR_V3: usize = 4 + 32 + 0x44; const S3: usize = 348;
        const HDR_V1: usize = 36; const S1: usize = 28;
        for i in [126, 127, 128, 129, 130, 131, 132, 133, 134] {
            let o1 = HDR_V1 + S1*i; let o3 = HDR_V3 + S3*i;
            if o3+S3 > v3.len() { break; }
            let v3_ct = u32::from_le_bytes(v3[o3..o3+4].try_into().unwrap());
            let v3_f0 = u32::from_le_bytes(v3[o3+4..o3+8].try_into().unwrap());
            let v3_v0 = u32::from_le_bytes(v3[o3+8..o3+12].try_into().unwrap());
            let v3_fd = u32::from_le_bytes(v3.get(o3+20..o3+24).unwrap_or(&[0;4]).try_into().unwrap());
            let v3_vd = u32::from_le_bytes(v3.get(o3+24..o3+28).unwrap_or(&[0;4]).try_into().unwrap());
            println!("  idx[{}] V1=(flag=1,freq={},volt={}) V3=(type={},cur=({},{})def=({},{}))",
                i,
                u32::from_le_bytes(v1[o1+8..o1+12].try_into().unwrap()),
                u32::from_le_bytes(v1[o1+12..o1+16].try_into().unwrap()),
                v3_ct, v3_f0, v3_v0, v3_fd, v3_vd);
        }
    }

    // 5. cross-check: do V1 (corrected layout) and V3 agree per-point?
    if st1 == 0 && st3 == 0 {
        const HDR_V1: usize = 36; const S1: usize = 28;
        const HDR_V3: usize = 4 + 32 + 0x44; const S3: usize = 348;
        let mut agree = 0usize; let mut disagree = 0usize;
        for i in 0..255 {
            let o1 = HDR_V1 + S1*i; let o3 = HDR_V3 + S3*i;
            if o1+S1 > v1.len() || o3+S3 > v3.len() { break; }
            let v1_freq = u32::from_le_bytes(v1[o1+8..o1+12].try_into().unwrap());
            let v1_volt = u32::from_le_bytes(v1[o1+12..o1+16].try_into().unwrap());
            let v3_freq = u32::from_le_bytes(v3[o3+4..o3+8].try_into().unwrap());
            let v3_volt = u32::from_le_bytes(v3[o3+8..o3+12].try_into().unwrap());
            if v1_freq==0 && v1_volt==0 { continue; }
            if v1_freq==v3_freq && v1_volt==v3_volt { agree+=1; } else { disagree+=1; }
        }
        println!("cross-check: {agree} agree, {disagree} disagree");
    }
}

