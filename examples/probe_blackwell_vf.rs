//! Blackwell-variant V/F protocol discrimination — tests whether the
//! pre-R610 "Blackwell" layouts that aiup/LACT documented (GetStatus
//! 0x11C28 with a NumClocks domain selector @+0x14; GetControl 0x12420
//! with 72-byte entries @+0x20 and single-bit masks) exist at all on the
//! CURRENT driver, or whether the R610.74 unified layout (GetControl
//! entries @+0x44 stride 36, flag@+0, delta@+0x14; GetStatus 8-dword mask
//! covering +0x14) is all there is — on ANY arch.
//!
//! Stages (all read-only unless --write is passed):
//!  1. Public GetControl 9248 raw scan — nonzero dword census in the
//!     entry region, classified by grid hypothesis (36B@0x44 vs 72B@0x20).
//!  2. GetStatus V1 magic/NumClocks matrix: ver1 0x11C28 (expect -9),
//!     ver2 0x21C28 with dword@+0x14 = 0 / 15 / 0xFFFFFFFF — if +0x14 is
//!     really mask dword 4 (R610 shape), 15 vs 0 must not change the
//!     curve; if it were a domain selector (R570 shape), it would.
//!  3. (--write) Definitive grid probe: set a +90 MHz mode-0 offset on a
//!     mid-curve point via the PRIVATE family (snapshot-RMW, restore
//!     guaranteed), then diff two public GetControl raw buffers. The
//!     changed dwords' absolute offsets reveal the true entry grid:
//!     R610 predicts the delta at 0x58+36*i, aiup-Blackwell at
//!     0x20+72*i. Restore to 0 afterwards either way.
//!
//! Run: cargo run --release --example probe_blackwell_vf [-- --write] [-- --point 40]

use nvapi::initialize;
use nvapi::sys::api::{
    NvAPI_EnumPhysicalGPUs, NvAPI_GPU_ClockClientClkVfPointsGetControl,
    NvAPI_GPU_ClockClientClkVfPointsGetInfo, NvAPI_GPU_ClockClientClkVfPointsGetStatus,
    NvAPI_GPU_ClockClkVfPointsGetControl as PrivGetControl,
    NvAPI_GPU_ClockClkVfPointsGetInfo as PrivGetInfo,
    NvAPI_GPU_ClockClkVfPointsSetControl as PrivSetControl, NvVersion,
};
use nvapi::sys::gpu::clock::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO;
use nvapi::sys::gpu::clock::private::{
    clk_vfp_control, NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE,
    NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE,
};
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::nvapi::VersionedStruct;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use std::ptr;

fn dw(buf: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(buf[off..off + 4].try_into().unwrap())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let do_write = args.iter().any(|a| a == "--write");
    let point: usize = args
        .iter()
        .position(|a| a == "--point")
        .and_then(|p| args.get(p + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(40);

    let _ = initialize();
    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS as usize];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    let gpu = handles[0];

    // ---- public GetInfo (6188): point mask seed ----
    let mut info = Box::new(unsafe { std::mem::zeroed::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO>() });
    *info.nvapi_version_mut() = NvVersion::with_struct::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO>(1);
    let st = unsafe { NvAPI_GPU_ClockClientClkVfPointsGetInfo(gpu, ptr::from_mut(&mut *info)) };
    if st != 0 { println!("public GetInfo failed: {st}"); return; }
    let mask_ptr = ptr::from_ref(&info.mask.mask) as *const u8;
    let n_points = info.mask.mask.iter().map(|m| m.count_ones()).sum::<u32>();
    println!("public GetInfo ok, mask bits = {n_points}");

    // ---- stage 1: public GetControl 9248 raw scan ----
    let ctrl_get = |magic: u32| -> (i32, Vec<u8>) {
        let mut buf = vec![0u8; 9248];
        buf[0..4].copy_from_slice(&magic.to_le_bytes());
        unsafe { std::ptr::copy_nonoverlapping(mask_ptr, buf.as_mut_ptr().add(4), 32); }
        let st = unsafe { NvAPI_GPU_ClockClientClkVfPointsGetControl(gpu, buf.as_mut_ptr() as *mut _) };
        (st, buf)
    };
    let (st, base) = ctrl_get(0x22420);
    println!("\n[1] public GetControl(0x22420) st={st}");
    if st == 0 {
        let mut nz = 0;
        let mut grid36 = 0;
        let mut grid72 = 0;
        let mut other = 0;
        for off in (68..9244).step_by(4) {
            let v = dw(&base, off);
            if v == 0 { continue; }
            nz += 1;
            let rel36 = (off - 68) % 36;
            let on72 = off >= 32 && (off - 32) % 72 == 0;
            if rel36 == 0 || rel36 == 20 { grid36 += 1; }
            if on72 { grid72 += 1; }
            if !(rel36 == 0 || rel36 == 20) && !on72 { other += 1; }
        }
        println!("    nonzero dwords in entry region: {nz}");
        println!("    fit R610 grid (0x44+36i, flag@+0/delta@+0x14): {grid36}");
        println!("    fit aiup-Blackwell grid (0x20+72i):            {grid72}");
        println!("    fit neither: {other}");
        if nz > 0 && nz <= 24 {
            for off in (68..9244).step_by(4) {
                if dw(&base, off) != 0 {
                    println!("      @+0x{off:X} (abs {off}) = {:#x}", dw(&base, off));
                }
            }
        }
    }

    // ---- stage 2: GetStatus V1 magic / NumClocks matrix ----
    println!("\n[2] GetStatus V1 magic matrix");
    for (label, magic, dword14) in [
        ("ver1 0x11C28, +0x14=0", 0x11C28u32, 0u32),
        ("ver2 0x21C28, +0x14=0", 0x21C28, 0),
        ("ver2 0x21C28, +0x14=15 (NumClocks?)", 0x21C28, 15),
        ("ver2 0x21C28, +0x14=0xFFFFFFFF (mask dw4)", 0x21C28, 0xFFFF_FFFF),
    ] {
        let mut buf = vec![0u8; 7208];
        buf[0..4].copy_from_slice(&magic.to_le_bytes());
        unsafe { std::ptr::copy_nonoverlapping(mask_ptr, buf.as_mut_ptr().add(4), 32); }
        buf[0x14..0x18].copy_from_slice(&dword14.to_le_bytes());
        let st = unsafe { NvAPI_GPU_ClockClientClkVfPointsGetStatus(gpu, buf.as_mut_ptr() as *mut _) };
        let first = if st == 0 {
            format!(" pt0=({},{})", dw(&buf, 68 + 4), dw(&buf, 68 + 8))
        } else { String::new() };
        println!("    {label:42} st={st}{first}");
    }

    // ---- stage 3: controlled private write + public diff (definitive) ----
    if do_write {
        println!("\n[3] grid discrimination via private set (+90 MHz on point {point}) + public diff");
        // private GetInfo + snapshot (Default stamps the accepted magic)
        let mut pinfo = Box::new(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE::default());
        let st = unsafe { PrivGetInfo(gpu, ptr::from_mut(&mut *pinfo).cast()) };
        if st != 0 { println!("    private GetInfo failed: {st}"); return; }
        let mut snap: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE> =
            Box::new(unsafe { std::mem::zeroed() });
        snap.version = NvVersion::with_version(clk_vfp_control::MAGIC);
        snap.seed_masks_from_info(&pinfo);
        let st = unsafe { PrivGetControl(gpu, ptr::from_mut(&mut *snap).cast()) };
        if st != 0 { println!("    private GetControl failed: {st}"); return; }
        // remember original record for restore
        let orig_type = snap.record_type(0, point).unwrap_or(8);
        let orig_mode = snap.mode(0, point).unwrap_or(0);
        let orig_value = snap.value(0, point).unwrap_or(0);
        println!("    original rec: type={orig_type} mode={orig_mode} value={orig_value}");

        // apply mode-0 +90000 kHz on bank0 point
        let _ = snap.set_mask_bit(0, point);
        let _ = snap.set_record_type(0, point, 8);
        let _ = snap.set_absolute(0, point, 90_000);
        let st = unsafe { PrivSetControl(gpu, ptr::from_ref(&*snap).cast()) };
        println!("    private SetControl(+90000) st={st}");

        if st == 0 {
            let (_st2, after) = ctrl_get(0x22420);
            let mut diffs = Vec::new();
            for off in (4..9244).step_by(4) {
                if dw(&base, off) != dw(&after, off) {
                    diffs.push((off, dw(&base, off), dw(&after, off)));
                }
            }
            println!("    changed dwords: {}", diffs.len());
            for (off, was, now) in diffs.iter().take(12) {
                let rel36 = off.checked_sub(68).map(|r| r % 36);
                let on72 = *off >= 32 && (*off - 32) % 72 == 0;
                println!("      @+0x{off:04X} ({off:5}): {was:#x} -> {now:#x}   [mod36={rel36:?}{}]",
                    if on72 { ", on72grid" } else { "" });
            }
            if let Some(&(off, _, _)) = diffs.first() {
                let idx36 = off.checked_sub(0x58).map(|r| r / 36);
                let idx72 = off.checked_sub(0x20).map(|r| r / 72);
                println!("    delta-slot verdict: 0x58+36*i ⇒ i={idx36:?} (want {point}); 0x20+72*i ⇒ i={idx72:?}");
            }
        }

        // restore: patch back the original record, and zero our point if it
        // was previously absent (mode-0 value 0 like probe_c_stair cleanup)
        let _ = snap.set_mask_bit(0, point);
        let _ = snap.set_record_type(0, point, orig_type);
        if orig_type == 8 || orig_type == 6 {
            if orig_mode == 0 { let _ = snap.set_absolute(0, point, orig_value); }
            else { let _ = snap.set_delta(0, point, orig_value as i16); }
        } else {
            let _ = snap.set_absolute(0, point, 0);
        }
        let st = unsafe { PrivSetControl(gpu, ptr::from_ref(&*snap).cast()) };
        println!("    restore st={st}");
        // verify restore via public read
        let (_, re) = ctrl_get(0x22420);
        let same = (4..9244).step_by(4).all(|o| dw(&base, o) == dw(&re, o));
        println!("    public buffer restored to baseline: {same}");
    } else {
        println!("\n[3] skipped (pass --write for the private-set grid discrimination)");
    }

    // ---- stage 4: PUBLIC SetControl single-point delta (first live
    // exercise of the 9248 SET path) — writes via the crate's own struct,
    // predicts the delta to reappear at 0x58+36*point on GetControl, then
    // restores and verifies. Also checks whether the public GetStatus V3
    // current pair moves (public deltas should; private ones do not —
    // stage 3 showed the private path writes below the user-offset layer).
    if do_write {
        println!("\n[4] public SetControl single-point (+90 MHz @ point {point}) round-trip");
        use nvapi::sys::api::NvAPI_GPU_ClockClientClkVfPointsSetControl;
        use nvapi::sys::gpu::clock::private::{
            NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL as CtrlStruct,
        };
        let predict = 0x58 + 36 * point;

        let mut ctrl = Box::new(unsafe { std::mem::zeroed::<CtrlStruct>() });
        use nvapi::sys::nvapi::VersionedStruct as _;
        *ctrl.nvapi_version_mut() = NvVersion::with_version(0x0001_2420);
        // SINGLE-BIT mask: the SET validates the type-class flag of every
        // masked entry against the RM snapshot — seeding the full GetInfo
        // mask drags in the fixed(mem) points and fails -1. Zeroed mask +
        // one bit = only our point is validated/written.
        ctrl.mask.set_bit(point);
        ctrl.points[point].freqDeltaKHz = 90_000;
        let st = unsafe { NvAPI_GPU_ClockClientClkVfPointsSetControl(gpu, ptr::from_ref(&*ctrl).cast()) };
        println!("    public SetControl st={st}");

        if st == 0 {
            let (_, after) = ctrl_get(0x22420);
            let got = dw(&after, predict);
            println!("    GetControl delta @+0x{predict:X} (predicted slot) = {got} (want 90000)");
            for off in (4..9244).step_by(4) {
                if dw(&base, off) != dw(&after, off) && off != predict {
                    println!("    UNEXPECTED diff @+0x{off:X}: {:#x} -> {:#x}", dw(&base, off), dw(&after, off));
                }
            }
            // does the public V3 GetStatus current pair move?
            let mut v3 = vec![0u8; 88844];
            v3[0..4].copy_from_slice(&0x35B0Cu32.to_le_bytes());
            unsafe { std::ptr::copy_nonoverlapping(mask_ptr, v3.as_mut_ptr().add(4), 32); }
            let st3 = unsafe { NvAPI_GPU_ClockClientClkVfPointsGetStatus(gpu, v3.as_mut_ptr() as *mut _) };
            if st3 == 0 {
                let o = 104 + 348 * point;
                println!("    V3[{point}] cur=({},{}) def=({},{})",
                    dw(&v3, o + 4), dw(&v3, o + 8), dw(&v3, o + 12), dw(&v3, o + 16));
            }
            // restore: delta 0
            ctrl.points[point].freqDeltaKHz = 0;
            let st = unsafe { NvAPI_GPU_ClockClientClkVfPointsSetControl(gpu, ptr::from_ref(&*ctrl).cast()) };
            println!("    restore st={st}");
            let (_, re) = ctrl_get(0x22420);
            let same = (4..9244).step_by(4).all(|o| dw(&base, o) == dw(&re, o));
            println!("    public buffer restored to baseline: {same}");
        }
    }
}
