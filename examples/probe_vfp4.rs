//! Seed GetStatus's request header (+4..+132) with GetInfo's returned mask.
use nvapi::sys::api::NvAPI_EnumPhysicalGPUs;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::nvapi_QueryInterface;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;

type RawFn = unsafe extern "C" fn(usize, *mut u8) -> i32;

fn main() {
    let _ = nvapi::initialize();
    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    let h = handles[0].as_ptr() as usize;
    let status: RawFn = unsafe { std::mem::transmute(nvapi_QueryInterface(0x7FEE9032).expect("r")) };
    let info: RawFn = unsafe { std::mem::transmute(nvapi_QueryInterface(0x8895B510).expect("r")) };

    // 1. GetInfo
    let mut gi = vec![0u8; 0x78604 + 0x1000];
    gi[0..4].copy_from_slice(&0x78604u32.to_le_bytes());
    let st = unsafe { info(h, gi.as_mut_ptr()) };
    assert_eq!(st, 0, "GetInfo failed");
    println!("GetInfo ok; mask region +4..0x30:");
    for o in (4..52usize).step_by(16) {
        println!("  +{o}: {}", gi[o..o+16].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
    }

    // 2. GetStatus: try seeding the first 128 bytes of header with GetInfo's mask
    for copy_len in [44usize, 128] {
        let mut buf = vec![0u8; 85016 + 0x1000];
        buf[0..4].copy_from_slice(&85016u32.to_le_bytes());
        buf[4..4 + copy_len.min(gi.len() - 4)].copy_from_slice(&gi[4..4 + copy_len.min(gi.len() - 4)]);
        let st = unsafe { status(h, buf.as_mut_ptr()) };
        let nz = buf.iter().filter(|&&b| b != 0).count();
        println!("GetStatus hdr=GetInfo[4..{}]: st={st}, nonzero={nz}", 4 + copy_len);
        if st == 0 && nz > 4 {
            let first = buf.iter().position(|&b| b != 0).unwrap_or(0);
            println!("  first nonzero @+{first}; dump: {}", buf[first..first + 0x40].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
        }
    }
}
// appended at runtime? no-op
