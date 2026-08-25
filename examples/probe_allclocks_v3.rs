//! GetAllClocks-family availability probe — tests V1/V2/V3 struct variants
//! across BOTH IDs:
//!
//!   NvAPI_GPU_GetAllClocks         0x1BD69F49 (legacy, clocks[32] raw)
//!   NvAPI_GPU_GetAllClockFrequencies 0xDCB616C3 (modern per-domain)
//!
//! The V3 compact variant (magic 0x30108, 264B) was discovered in AmpereOC:
//! dword@+4 = mode selector (1 = base clocks, 2 = boost clocks), then 8
//! slots of {valid: u32, value: u32} at 32-byte stride from +8. AmpereOC
//! wraps it via 0xDCB616C3 (GetAllClockFrequencies), reading slot[0]
//! (core/shader) and slot[8]... in both modes for base/boost pairs.
//!
//! Desktop note: on Ada mobile (R610.74) several variants return -9; run the
//! same probe on a desktop to compare availability.
//!
//! Run: `cargo run --release --example probe_allclocks_v3`

use nvapi::initialize;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use nvapi::sys::api::NvAPI_EnumPhysicalGPUs;
use nvapi::sys::handles::NvPhysicalGpuHandle;

const GET_ALL_CLOCKS: u32 = 0x1BD69F49; // legacy
const GET_ALL_CLOCK_FREQ: u32 = 0xDCB616C3; // modern

fn main() {
    let _ = initialize();
    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    let gpu = handles[0];

    let qi = |id: u32| -> unsafe extern "C" fn(NvPhysicalGpuHandle, *mut core::ffi::c_void) -> i32 {
        unsafe { std::mem::transmute(nvapi::sys::nvapi_QueryInterface(id).expect("QI NULL")) }
    };

    let dump = |label: &str, buf: &[u8], slot_mode: bool| {
        if slot_mode {
            // V3 compact: mode@+4, slots {valid,value} 32B stride from +8
            let mode = u32::from_le_bytes(buf[4..8].try_into().unwrap());
            println!("{label}: st=0 mode={mode}");
            for slot in 0..8usize {
                let o = 8 + 32 * slot;
                if o + 8 > buf.len() {
                    break;
                }
                let valid = u32::from_le_bytes(buf[o..o + 4].try_into().unwrap());
                let value = u32::from_le_bytes(buf[o + 4..o + 8].try_into().unwrap());
                if valid != 0 || value != 0 {
                    println!(
                        "  slot[{slot}]: valid={valid} value={value} ({} MHz)",
                        value / 1000
                    );
                }
            }
        } else {
            // V1/V2: raw dword dump of plausible clock values
            println!("{label}: st=0");
            let nz: Vec<(usize, u32)> = (1..buf.len() / 4)
                .map(|i| {
                    (
                        i,
                        u32::from_le_bytes(buf[i * 4..i * 4 + 4].try_into().unwrap()),
                    )
                })
                .filter(|(_, v)| *v != 0 && *v < 100_000_000)
                .collect();
            for (i, v) in nz.iter().take(10) {
                println!("  dword[{i}] = {v}");
            }
        }
    };

    // ---- ID 1: GetAllClockFrequencies (0xDCB616C3) — the modern one ----
    let f1 = qi(GET_ALL_CLOCK_FREQ);
    for (label, magic, size, slot_mode) in [
        ("FreqV1(0x10088)", 0x10088u32, 136usize, false), // documented V1 (per-domain 8B)
        ("FreqV2(0x20108)", 0x20108, 264, false), // documented V2 (per-domain 8B, 32 domains)
        ("FreqV3compact(0x30108)", 0x30108, 264, true), // AmpereOC variant w/ mode selector
    ] {
        for mode in [1u32, 2u32] {
            let mut buf = vec![0u8; size];
            buf[0..4].copy_from_slice(&magic.to_le_bytes());
            if slot_mode {
                buf[4..8].copy_from_slice(&mode.to_le_bytes());
            }
            let st = unsafe { f1(gpu, buf.as_mut_ptr() as *mut _) };
            if slot_mode {
                println!("GetAllClockFrequencies {label} mode={mode} st={st}");
            } else {
                println!("GetAllClockFrequencies {label} st={st}");
            }
            if st == 0 {
                dump(&format!("  {label} mode={mode}"), &buf, slot_mode);
            }
        }
    }

    // ---- ID 2: GetAllClocks (0x1BD69F49) — legacy ----
    let f2 = qi(GET_ALL_CLOCKS);
    for (label, magic, size) in [
        ("LegacyV1(0x10084)", 0x10084u32, 132usize),
        ("LegacyV3compact(0x30108)", 0x30108, 264),
    ] {
        let mut buf = vec![0u8; size];
        buf[0..4].copy_from_slice(&magic.to_le_bytes());
        let st = unsafe { f2(gpu, buf.as_mut_ptr() as *mut _) };
        println!("GetAllClocks {label} st={st}");
        if st == 0 {
            dump(&format!("  {label}"), &buf, false);
        }
    }

    // ---- medium-layer API: base_boost_clocks (V3 compact via typed wrapper) ----
    let gpus = nvapi::PhysicalGpu::enumerate().expect("enumerate");
    let g = &gpus[0];
    println!("GPU: {}", g.full_name().unwrap_or_default());
    match g.base_boost_clocks(nvapi::BaseBoostMode::Base) {
        Ok((core, mem)) => println!(
            "base_boost_clocks(Base):  core={}kHz ({} MHz), mem={}kHz ({} MHz)",
            core,
            core / 1000,
            mem,
            mem / 1000
        ),
        Err(e) => println!("base_boost_clocks(Base): {e:?}"),
    }
    match g.base_boost_clocks(nvapi::BaseBoostMode::Boost) {
        Ok((core, mem)) => println!(
            "base_boost_clocks(Boost): core={}kHz ({} MHz), mem={}kHz ({} MHz)",
            core,
            core / 1000,
            mem,
            mem / 1000
        ),
        Err(e) => println!("base_boost_clocks(Boost): {e:?}"),
    }
}
