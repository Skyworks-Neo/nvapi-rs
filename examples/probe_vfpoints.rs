//! Live probe of the private XBAR V/F-points family (article xbar.txt:151-167,
//! "NV2080_CTRL_CMD_CLK_VF_POINTS_GET_STATUS"; XBAR bank @0x4c40, 127 records,
//! stride 0x98). Read-only diagnostic: the effective V/F curve — the
//! verification companion to SET_CONTROL (CONTROL is byte-identical before/
//! after an offset write; the recomputed curve appears only in STATUS).
use nvapi::sys::api::NvAPI_EnumPhysicalGPUs;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::nvapi_QueryInterface;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;

type RawFn = unsafe extern "C" fn(usize, *mut u8) -> i32;

fn resolve(id: u32, label: &str) -> Option<RawFn> {
    match nvapi_QueryInterface(id) {
        Ok(ptr) if ptr != 0 => {
            println!("0x{id:08X} {label:34} -> RESOLVED");
            Some(unsafe { std::mem::transmute::<usize, RawFn>(ptr) })
        }
        _ => { println!("0x{id:08X} {label:34} -> NULL"); None }
    }
}

fn main() {
    nvapi::initialize();
    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    if count == 0 { println!("no GPU"); return; }
    let h = handles[0].as_ptr() as usize;
    println!("GPU handle = 0x{h:X}\n");

    const VF_STATUS_SIZE: usize = 0x98208;
    let status_fn = resolve(0x7fee9032, "ClockClkVfPointsGetStatus");
    let info_fn = resolve(0x8895b510, "ClockClkVfPointsGetInfo");
    let ctrl_fn = resolve(0xda025c3e, "ClockClkVfPointsGetControl");

    if let Some(f) = info_fn {
        println!("\n--- GET_INFO (discovery) ---");
        for &sz in &[0x100usize, 0x200, 0x1000] {
            for v in 1u32..=4 {
                let mut buf = vec![0u8; sz];
                let magic = (sz as u32) | (v << 16);
                buf[0..4].copy_from_slice(&magic.to_le_bytes());
                let st = unsafe { f(h, buf.as_mut_ptr()) };
                if st == 0 {
                    println!("  GET_INFO OK: size=0x{:X} v{} (m0x{:X})", sz, v, magic);
                    println!("  +0..0x20: {}", buf[0..0x20].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
                }
            }
        }
    }

    if let Some(f) = status_fn {
        println!("\n--- GET_STATUS (effective V/F curve, READ-ONLY) ---");
        for v in 1u32..=4 {
            let mut buf = vec![0u8; VF_STATUS_SIZE];
            let magic = (VF_STATUS_SIZE as u32) | (v << 16);
            buf[0..4].copy_from_slice(&magic.to_le_bytes());
            let st = unsafe { f(h, buf.as_mut_ptr()) };
            println!("  v{} (m0x{:X}): st={}", v, magic, st);
            if st == 0 {
                let base = 0x4c40;
                println!("  XBAR bank @0x{:X}:", base);
                for i in 0..3 {
                    let off = base + i * 0x98;
                    if off + 0x20 <= buf.len() {
                        println!("    rec[{}] @0x{:X}: {}", i, off, buf[off..off+0x20].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
                    }
                }
                println!("  +0..0x20 (head): {}", buf[0..0x20].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
                break;
            }
        }
    }

    if let Some(f) = ctrl_fn {
        println!("\n--- GET_CONTROL (writable V/F points) ---");
        for v in 1u32..=4 {
            let mut buf = vec![0u8; VF_STATUS_SIZE];
            let magic = (VF_STATUS_SIZE as u32) | (v << 16);
            buf[0..4].copy_from_slice(&magic.to_le_bytes());
            let st = unsafe { f(h, buf.as_mut_ptr()) };
            println!("  v{} (m0x{:X}): st={}", v, magic, st);
            if st == 0 { break; }
        }
    }
}
