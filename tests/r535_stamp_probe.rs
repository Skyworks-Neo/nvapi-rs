// R535-era stamp whitelist probe — READ-ONLY. Walks the GetInfo/GetStatus
// stamp ladders discovered by IDA (nvapi64_39135/53878/58241: the stamps
// are struct sizes in bytes and each branch's whitelist is the ABI) and
// prints the live status per stamp, plus a canonical-record peek when the
// R535 canonical stamp (300164) is accepted.
//
// Run: cargo test -p nvapi --test r535_stamp_probe -- --nocapture --ignored

#![allow(unused_must_use)]

use core::ptr;
use nvapi::PhysicalGpu;
use nvapi::sys::api::{NvAPI_GPU_ClockClkVfPointsGetInfo, NvAPI_GPU_ClockClkVfPointsGetStatus};
use nvapi::sys::gpu::clock::undocumented::{
    NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE,
    NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE,
};
use nvapi::sys::nvapi::NvVersion;

#[test]
#[ignore]
fn r535_stamp_whitelist_probe() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    eprintln!("GPU: {:?} handle={:?}", gpu.full_name(), *gpu.handle());

    // middle-layer chain check: does clk_vf_points_private_raw() land on a
    // working stamp where the raw ladder above succeeds?
    match gpu.clk_vf_points_private_raw() {
        Ok(v) => eprintln!(
            "middle clk_vf_points_private_raw: OK ({} points, {} raw records)",
            v.points.len(),
            v.raw_records.len()
        ),
        Err(e) => eprintln!("middle clk_vf_points_private_raw: ERR {e}"),
    }

    // ---- GetInfo ladder (IDA whitelist R535: {83996, 157692, 249844, 369796}) ----
    for magic in [0x78604, 369_796, 249_844, 157_692, 83_996] {
        let mut info = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE>::new_zeroed();
            let mut b = b.assume_init();
            b.version = NvVersion::with_version(magic);
            b
        };
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetInfo(*gpu.handle(), ptr::from_mut(&mut *info).cast())
        };
        eprintln!("INFO  {magic:>8} (0x{magic:X}): {st:#x}");
    }

    // ---- GetStatus ladder (IDA whitelist R535: {85016, 158200, 214652, 300164}) ----
    // canonical (300164) needs the 512-bit mask windows seeded; every other
    // stamp takes the 128B +4 seed.
    let info_stamp = 83_996u32; // the accepted legacy GetInfo stamp
    let mut info = unsafe {
        let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE>::new_zeroed();
        let mut b = b.assume_init();
        b.version = NvVersion::with_version(info_stamp);
        b
    };
    let st = unsafe {
        NvAPI_GPU_ClockClkVfPointsGetInfo(*gpu.handle(), ptr::from_mut(&mut *info).cast())
    };
    eprintln!("INFO  seed-provider {info_stamp}: {st:#x}");
    assert_eq!(st, 0, "seed-provider GetInfo must succeed for the ladder");

    for magic in [2_000_388, 1_525_252, 300_164, 214_652, 158_200, 85_016] {
        let mut status = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE>::new_zeroed();
            let mut b = b.assume_init();
            b.version = NvVersion::with_version(magic);
            b
        };
        if magic == 300_164 {
            status.seed_canonical_header(&info);
        } else {
            info.seed_status_header(&mut status);
        }
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetStatus(*gpu.handle(), ptr::from_mut(&mut *status).cast())
        };
        eprintln!("STATUS {magic:>8} (0x{magic:X}): {st:#x}");

        // canonical peek: first four present points' nonzero dwords.
        // NB: word offsets are printed ABSOLUTE (record base + 4*k) — the
        // buffer's rest[] slice starts at struct+4 (version dword), so
        // rest-relative dumps skew every offset by -4 vs the documented
        // record layout.
        if magic == 300_164 && st == 0 {
            let mut shown = 0;
            for idx in 0..512usize {
                if status.canonical_point_present(0, idx) != Some(true) || shown >= 4 {
                    continue;
                }
                shown += 1;
                let rec_abs = 580 + idx * 292; // clk_vfp_status_canonical::REC1
                let base_rest = rec_abs - 4;
                let words: Vec<String> = (0..292 / 4)
                    .filter_map(|w| {
                        let off = base_rest + w * 4;
                        let v = u32::from_le_bytes(status.rest[off..off + 4].try_into().unwrap());
                        (v != 0).then(|| format!("+{:03X}:0x{v:08X}", w * 4))
                    })
                    .collect();
                eprintln!(
                    "  rec[{idx}] type={} {}",
                    status.canonical_type(0, idx).unwrap_or(0),
                    words.join(" ")
                );
            }
        }
    }
}
