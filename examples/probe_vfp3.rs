//! GetInfo magic 0x78604 (493060, from sub_1802169A0's `*a2 == 493060`) and
//! GetStatus V1 85016 with header seeds (user +4..+132 is marshalled into the
//! RM request — likely point/request masks).
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

    println!("--- GetInfo 0x78604 ---");
    let mut buf = vec![0u8; 0x78604 + 0x1000];
    buf[0..4].copy_from_slice(&0x78604u32.to_le_bytes());
    let st = unsafe { info(h, buf.as_mut_ptr()) };
    println!("st={st}, nonzero={}", buf.iter().filter(|&&b| b != 0).count());
    if st == 0 {
        println!("  +0..0x60: {}", buf[0..0x60].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
    }

    println!("--- GetStatus 85016, header variants ---");
    for (label, seed) in [("hdr=0xCC", 0xCCu8), ("hdr=0xFF", 0xFF)] {
        let mut buf = vec![0u8; 85016 + 0x1000];
        buf[0..4].copy_from_slice(&85016u32.to_le_bytes());
        for b in buf[4..132].iter_mut() { *b = seed; }
        let st = unsafe { status(h, buf.as_mut_ptr()) };
        let nz = buf.iter().filter(|&&b| b != 0).count();
        println!("{label}: st={st}, nonzero={nz}");
        if st == 0 && nz > 4 {
            // which areas got written?
            let hdr_echo: Vec<String> = (4..36usize).step_by(4).map(|o| format!("{:08x}", u32::from_le_bytes(buf[o..o+4].try_into().unwrap()))).collect();
            println!("  hdr echo +4..36: {}", hdr_echo.join(" "));
            let first_nz = buf.iter().position(|&b| b != 0).unwrap_or(0);
            println!("  first nonzero @+{first_nz}");
            if first_nz >= 700 {
                let off = first_nz;
                println!("  bytes @first..+0x30: {}", buf[off..off+0x30].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
            }
        }
    }
}
