//! A/B probe for the public boost-curve family (HYDRA 2.2B PRO cross-check):
//!
//!   GetInfo     0x507B4B59 (GetClockBoostMask)  -> our vfp_mask()
//!   GetControl  0x23F1B133 (GetClockBoostTable) -> our vfp_table_raw()
//!   GetStatus   0x21537AD4 (GetVFPCurve)        -> our vfp_curve()
//!
//! RESOLVED (live 4060L + nvapi64_impl.dll handler RE @0x1802071C0):
//! the rumored "+36..+68 unknown32 must echo GetInfo output" is FALSE —
//! the impl handler never reads those bytes, and GetInfo leaves them zero.
//! The only seed that matters is the point mask @+4..+36 (256-bit selector,
//! bittest loop), which our wrappers already set. Version check is
//! `(ver - 0x12420) & 0xFFFEFFFF == 0`: BOTH V1 (0x12420) and V2 (0x22420)
//! magics accepted for the 9248B control struct. Entry semantics: flag
//! @entry+0 must match driver-side type validity (types 0/2/4 -> 1,
//! 1/3/5/6/7 -> 0, mismatch -> -1), freqDeltaKHz @entry+20 (Kilohertz2
//! fixed point — the ×2 unit our gpu.rs /2 already handles).
//!
//! Run: `cargo run --release --example probe_boost_curve_echo`

use nvapi::initialize;
use nvapi::sys::api::{NvAPI_EnumPhysicalGPUs, NvVersion};
use nvapi::sys::gpu::clock::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL;
use nvapi::sys::gpu::clock::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO;
use nvapi::sys::gpu::power::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::nvapi::VersionedStruct;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use std::ptr;

fn nz(buf: &[u8]) -> usize {
    buf.chunks(4)
        .filter(|c| c.iter().any(|&b| b != 0))
        .count()
}

fn main() {
    let _ = initialize();
    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS as usize];
    let mut count = 0u32;
    use nvapi::sys::api::NvAPI_GPU_ClockClientClkVfPointsGetControl;
    use nvapi::sys::api::NvAPI_GPU_ClockClientClkVfPointsGetInfo;
    use nvapi::sys::api::NvAPI_GPU_ClockClientClkVfPointsGetStatus;

    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    let gpu = handles[0];

    // ---- 1. GetInfo (mask builder) ------------------------------------
    let mut info = Box::new(unsafe { std::mem::zeroed::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO>() });
    *info.nvapi_version_mut() =
        NvVersion::with_struct::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO>(1);
    let st = unsafe { NvAPI_GPU_ClockClientClkVfPointsGetInfo(gpu, ptr::from_mut(&mut *info)) };
    println!("GetInfo  st={st} magic=0x{:X}", info.version.data);
    if st != 0 {
        println!("GetInfo failed — nothing to seed from; abort");
        return;
    }
    let info_bytes =
        unsafe { std::slice::from_raw_parts(ptr::from_ref(&*info).cast::<u8>(), 6188) };
    println!("  info.mask  = {:08X?}", info.mask.mask.iter().map(|v| v).take(2).collect::<Vec<_>>());
    println!("  info.unknown32 (+36..+68) = {:08X?}", {
        let mut v = [0u32; 8];
        for (i, w) in v.iter_mut().enumerate() {
            *w = u32::from_le_bytes(info_bytes[36 + 4 * i..36 + 4 * i + 4].try_into().unwrap());
        }
        v
    });
    let echo: [u8; 32] = info_bytes[36..68].try_into().unwrap();

    // helper: build a control/status buffer with mask set, unknown zeroed or echoed
    let seed = |buf: *mut u8, mask: *const u8, do_echo: bool| unsafe {
        std::ptr::copy_nonoverlapping(mask, buf.add(4), 32); // mask @+4..+36
        if do_echo {
            std::ptr::copy_nonoverlapping(echo.as_ptr(), buf.add(36), 32); // unknown @+36..+68
        }
    };

    // ---- 2. GetControl A/B (+ ver1-magic variant per HYDRA RE) ----------
    // HYDRA uses ver1 magic 0x12420 on the 9248B table; our alias is ver2
    // (0x22420). Try mask-set vs zero, and ver2 vs ver1.
    for (do_echo, ver) in [(true, 2u32), (false, 2), (true, 1), (false, 1)] {
        let mut ctrl = Box::new(unsafe { std::mem::zeroed::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL>() });
        *ctrl.nvapi_version_mut() = NvVersion::with_version(ver << 16 | 9248);
        let st = unsafe { NvAPI_GPU_ClockClientClkVfPointsGetControl(gpu, ptr::from_mut(&mut *ctrl)) };
        let bytes = unsafe {
            std::slice::from_raw_parts(ptr::from_ref(&*ctrl).cast::<u8>(), 9248)
        };
        println!(
            "GetControl echo={do_echo:<5} st={st}  nonzero dwords={} (of {})  hdr36..68={:08X?}",
            nz(bytes),
            bytes.len() / 4,
            {
                let mut v = [0u32; 8];
                for (i, w) in v.iter_mut().enumerate() {
                    *w = u32::from_le_bytes(bytes[36 + 4 * i..40 + 4 * i].try_into().unwrap());
                }
                v
            }
        );
        if st == 0 {
            // dump first 2 entries (entries @+68, 36B stride): clock_type, rsvd, freqDelta @+20
            for e in 0..2 {
                let base = 68 + 36 * e;
                let d = |o: usize| u32::from_le_bytes(bytes[base + o..base + o + 4].try_into().unwrap());
                println!(
                    "    entry[{e}] clock_type={} freqDeltaKHz={} raw={:08X?}",
                    d(0),
                    d(20) as i32,
                    (0..36).step_by(4).map(d).collect::<Vec<_>>()
                );
            }
        }
    }

    // ---- 3. GetStatus A/B (V3 88588B our default; HYDRA uses V1 7208B) --
    use nvapi::sys::gpu::power::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_V1;
    for do_echo in [false, true] {
        let mut status = Box::new(unsafe { std::mem::zeroed::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS>() });
        *status.nvapi_version_mut() =
            NvVersion::with_struct::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS>(3);
        let size = std::mem::size_of::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS>();
        seed(ptr::from_mut(&mut *status).cast(), ptr::from_ref(&info.mask.mask).cast(), do_echo);
        let st = unsafe { NvAPI_GPU_ClockClientClkVfPointsGetStatus(gpu, ptr::from_mut(&mut *status)) };
        let bytes = unsafe { std::slice::from_raw_parts(ptr::from_ref(&*status).cast::<u8>(), size) };
        println!(
            "GetStatusV3 echo={do_echo:<5} st={st}  nonzero dwords={} (of {}) size={size}",
            nz(bytes),
            bytes.len() / 4
        );
    }
    for do_echo in [false, true] {
        let mut status = Box::new(unsafe { std::mem::zeroed::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_V1>() });
        *status.nvapi_version_mut() =
            NvVersion::with_struct::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_V1>(2);
        let size = std::mem::size_of::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_V1>();
        seed(ptr::from_mut(&mut *status).cast(), ptr::from_ref(&info.mask.mask).cast(), do_echo);
        let st = unsafe { NvAPI_GPU_ClockClientClkVfPointsGetStatus(gpu, ptr::from_mut(&mut *status).cast()) };
        let bytes = unsafe { std::slice::from_raw_parts(ptr::from_ref(&*status).cast::<u8>(), size) };
        println!(
            "GetStatusV1 echo={do_echo:<5} st={st}  nonzero dwords={} (of {}) size={size}",
            nz(bytes),
            bytes.len() / 4
        );
    }
}
