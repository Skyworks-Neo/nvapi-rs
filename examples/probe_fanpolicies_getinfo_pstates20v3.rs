//! Live probe: ClientFanPoliciesGetInfo (0x52B76D12, struct 0x2004C/76B)
//! and the EVGA-corroborated pstates20 v3 clock-offset variant (0x31CF8/7416B).
//!
//! 1. GetInfo: dump the 72 opaque bytes as dwords for field decoding.
//! 2. pstates20 v3: feed a zeroed 7416-byte buffer stamped 0x31CF8 to
//!    NvAPI_GPU_GetPstates20 (0x6FF81213, raw QueryInterface call) and see
//!    whether the modern driver accepts the version-3 magic.

use nvapi::initialize;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::nvapi_QueryInterface;

fn main() {
    initialize().expect("initialize");

    let gpus = nvapi::PhysicalGpu::enumerate().expect("enumerate");
    println!("{} GPU(s)", gpus.len());
    for gpu in &gpus {
        let handle: NvPhysicalGpuHandle = *gpu.handle();
        let name = gpu.full_name().unwrap_or_default();
        println!("--- {name} ---");

        // 1. ClientFanPoliciesGetInfo
        match gpu.fan_policy_info() {
            Ok(info) => {
                println!("GetInfo(0x2004C) OK, magic=0x{:08X}", info.version);
                for (i, chunk) in info.data.chunks_exact(4).enumerate() {
                    let dw = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    if dw != 0 {
                        println!("  +{:02} (dword {:2}): 0x{:08X} ({})", i * 4, i, dw, dw);
                    }
                }
            }
            Err(e) => println!("GetInfo(0x2004C) failed: {:?}", e),
        }

        // 2. pstates20 v3 variant 0x31CF8 (7416B) via raw GetPstates20
        type GetPstates20Fn = unsafe extern "C" fn(NvPhysicalGpuHandle, *mut u8) -> i32;
        let f: GetPstates20Fn = unsafe {
            match nvapi_QueryInterface(0x6FF81213) {
                Ok(p) => std::mem::transmute(p),
                Err(_) => {
                    println!("GetPstates20 (0x6FF81213) UNRESOLVED");
                    continue;
                }
            }
        };
        let mut buf = vec![0u8; 7416];
        buf[..4].copy_from_slice(&0x31CF8u32.to_le_bytes());
        let st = unsafe { f(handle, buf.as_mut_ptr()) };
        if st == 0 {
            println!("pstates20 v3 (0x31CF8/7416B) ACCEPTED — nonzero dwords:");
            let mut shown = 0;
            for (i, chunk) in buf.chunks_exact(4).enumerate() {
                let dw = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                if dw != 0 && i > 0 && shown < 64 {
                    println!("  +{:04}: 0x{:08X} ({})", i * 4, dw, dw);
                    shown += 1;
                }
            }
            if shown == 0 {
                println!("  (all zero beyond the magic — driver accepted but filled nothing)");
            }
        } else {
            println!("pstates20 v3 (0x31CF8/7416B) rejected: status {st}");
        }
    }
}
