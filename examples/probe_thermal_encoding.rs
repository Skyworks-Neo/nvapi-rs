//! Live verification of the ClientThermalPoliciesSetStatus (0x34C0B13D)
//! temperature ENCODING: integer Celsius vs Q8 (celsius<<8).
//!
//! AmpereOC RE claims INTEGER C. Our V3 struct comment says "shifted 8 bits"
//! (Q8). The private target_temperature API (0xE097144F) is Q8-confirmed —
//! but this API may differ.
//!
//! Uses raw byte buffers so any magic can be tested: V2 (0x20038, 56B) per
//! HYDRA's 20-byte write, and V3 (0x33048, 1352B) per our default alias.
//!
//! Run: `cargo run --release --example probe_thermal_encoding`       (read only)
//!      `cargo run --release --example probe_thermal_encoding -- 85` (write, admin)

use nvapi::initialize;
use nvapi::sys::api::{NvAPI_EnumPhysicalGPUs, NvAPI_GPU_ClientThermalPoliciesGetStatus, NvAPI_GPU_ClientThermalPoliciesSetStatus};
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;

const GET_THERMAL_POLICIES: u32 = 0xE9C425A1;
const SET_THERMAL_POLICIES: u32 = 0x34C0B13D;

fn qi(id: u32) -> usize {
    nvapi::sys::nvapi_QueryInterface(id).expect("QI NULL")
}

type GetFn = unsafe extern "C" fn(NvPhysicalGpuHandle, *mut core::ffi::c_void) -> i32;
type SetFn = unsafe extern "C" fn(NvPhysicalGpuHandle, *const core::ffi::c_void) -> i32;

fn dump(label: &str, buf: &[u8], entry_off: usize) -> Option<u32> {
    println!("{label}: magic=0x{:X}", u32::from_le_bytes(buf[0..4].try_into().unwrap()));
    let mut first_temp: Option<u32> = None;
    // entries {policy_id, temp, pstate} 12B from entry_off
    let mut e = 0;
    while entry_off + 12 * (e + 1) <= buf.len() && e < 8 {
        let o = entry_off + 12 * e;
        let pid = u32::from_le_bytes(buf[o..o + 4].try_into().unwrap());
        let temp = u32::from_le_bytes(buf[o + 4..o + 8].try_into().unwrap());
        let ps = u32::from_le_bytes(buf[o + 8..o + 12].try_into().unwrap());
        if pid == 0 && temp == 0 && ps == 0 { break; }
        if e == 0 { first_temp = Some(temp); }
        println!("  entry[{e}]: policy={pid} temp_raw={temp} (0x{temp:X}){}",
            if (0o0..40000).contains(&temp) {
                format!("  -> int:{temp}C  Q8:{:.1}C", temp as f64 / 256.0)
            } else { String::new() });
        e += 1;
    }
    first_temp
}

fn main() {
    let target: Option<u32> = std::env::args().nth(1).and_then(|s| s.parse().ok());
    let _ = initialize();
    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS as usize];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    let gpu = handles[0];

    let _get: GetFn = unsafe { std::mem::transmute(qi(GET_THERMAL_POLICIES)) };
    let _set: SetFn = unsafe { std::mem::transmute(qi(SET_THERMAL_POLICIES)) };

    // Try V2 (56B, magic 0x20038) first — HYDRA's path
    for (label, magic, size, entry_off) in [
        ("V2(0x20038)", 0x20038u32, 56usize, 8usize),
        ("V3(0x33048)", 0x33048, 1352, 8),
    ] {
        let mut buf = vec![0u8; size];
        buf[0..4].copy_from_slice(&magic.to_le_bytes());
        let st = unsafe { _get(gpu, buf.as_mut_ptr() as *mut _) };
        println!("GetStatus {label} st={st}");
        if st == 0 {
            let before = dump("  current", &buf, entry_off);
            // optional SET test: patch entry[0] temp with INTEGER and read back
            if let (Some(t), Some(raw_before)) = (target, before) {
                let o = entry_off + 4;
                buf[o..o + 4].copy_from_slice(&t.to_le_bytes());
                let st2 = unsafe { _set(gpu, buf.as_ptr() as *const _) };
                println!("  SetStatus(int {t}) st={st2}");
                if st2 == 0 {
                    let mut rb = vec![0u8; size];
                    rb[0..4].copy_from_slice(&magic.to_le_bytes());
                    let st3 = unsafe { _get(gpu, rb.as_mut_ptr() as *mut _) };
                    if st3 == 0 {
                        let after = dump("  post-SET", &rb, entry_off);
                        if let Some(a) = after {
                            println!("  VERDICT: wrote {t}, read {a} -> {}",
                                if a == t { "INTEGER C" }
                                else if a == (t << 8) { "Q8 (x256)" }
                                else if a == raw_before { "driver clamped (unchanged)" }
                                else { "driver remapped" });
                        }
                    }
                    // restore: write back original
                    buf[o..o + 4].copy_from_slice(&raw_before.to_le_bytes());
                    let _ = unsafe { _set(gpu, buf.as_ptr() as *const _) };
                }
                return; // only test the first working version
            }
        }
    }
}
