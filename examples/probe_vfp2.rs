//! Probe the private V/F-points GetStatus (0x7FEE9032) with the 6 IDA-derived
//! accepted magics (sub_180217330): 85016, 158200, 214652, 300164, 1525252,
//! 2000388. The article's 0x98208 was the RM param size, not an NVAPI magic —
//! hence the earlier -9. User struct: two banks of 2048 points × 488B records
//! (bank1 @+772, bank2 @+1000964; total 2000388 bytes).
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
    let f: RawFn = unsafe { std::mem::transmute(nvapi_QueryInterface(0x7FEE9032).expect("resolve")) };
    let g: Option<RawFn> = nvapi_QueryInterface(0x8895B510).ok()
        .filter(|p| *p != 0).map(|p| unsafe { std::mem::transmute(p) });

    println!("--- GetInfo (0x8895B510) with same magics ---");
    if let Some(g) = g {
        for &m in &[85016u32, 158200, 214652, 300164, 1525252, 2000388] {
            let mut buf = vec![0u8; (m as usize) + 0x1000];
            buf[0..4].copy_from_slice(&m.to_le_bytes());
            let st = unsafe { g(h, buf.as_mut_ptr()) };
            println!("  0x{m:X} ({m}): st={st}");
            if st == 0 {
                println!("    +0..0x30: {}", buf[0..0x30].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
            }
        }
    }

    println!("--- GetStatus (0x7FEE9032) ---");
    for &m in &[85016u32, 158200, 214652, 300164, 1525252, 2000388] {
        let mut buf = vec![0u8; (m as usize) + 0x1000];
        buf[0..4].copy_from_slice(&m.to_le_bytes());
        let st = unsafe { f(h, buf.as_mut_ptr()) };
        let nz = buf.iter().filter(|&&b| b != 0).count();
        println!("  0x{m:X} ({m}): st={st}, nonzero={nz}");
        if st == 0 {
            // header + first records of bank1
            println!("    hdr +0..0x40: {}", buf[0..0x40].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
            for i in 0..2usize {
                let off = 772 + 488 * i;
                println!("    bank1 rec[{i}] @+{off}: {}", buf[off..off+0x30].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
            }
            break;
        }
    }
}
