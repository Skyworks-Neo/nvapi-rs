//! Full 488B record dump from GetStatus (magic 2000388, GetInfo-seeded) —
//! first records of bank1, with dword decoding to identify freq/voltage fields.
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
    let mut gi = vec![0u8; 0x78604 + 0x1000];
    gi[0..4].copy_from_slice(&0x78604u32.to_le_bytes());
    assert_eq!(unsafe { info(h, gi.as_mut_ptr()) }, 0);
    let mut buf = vec![0u8; 2000388 + 0x1000];
    buf[0..4].copy_from_slice(&2000388u32.to_le_bytes());
    buf[4..132].copy_from_slice(&gi[4..132]);
    assert_eq!(unsafe { status(h, buf.as_mut_ptr()) }, 0);
    // bank1: first 6 present records (mask bit set)
    let mut shown = 0;
    for i in 0..2048usize {
        let md = u32::from_le_bytes(gi[4 + 4*(i>>5)..8 + 4*(i>>5)].try_into().unwrap());
        if md & (1 << (i & 31)) == 0 { continue; }
        let rec = 772 + 488 * i;
        println!("bank1 rec[{i}] @+{rec} type={}:",
            u8::from_le_bytes([buf[rec]]));
        for base in (0..488usize).step_by(32) {
            let dws: Vec<String> = (0..8usize).map(|k| {
                let o = rec + base + k*4;
                if o + 4 > buf.len() { return String::new(); }
                format!("{:08x}", u32::from_le_bytes(buf[o..o+4].try_into().unwrap()))
            }).collect();
            if dws.iter().any(|d| d != "00000000") {
                println!("  +{base:03}: {}", dws.join(" "));
            }
        }
        shown += 1;
        if shown >= 6 { break; }
    }
    // bank2 first 3
    let mut shown2 = 0;
    for i in 0..2048usize {
        let md = u32::from_le_bytes(gi[0x34304 + 4*(i>>5)..0x34304 + 4*(i>>5) + 4].try_into().unwrap());
        if md & (1 << (i & 31)) == 0 { continue; }
        let rec = 1000964 + 488 * i;
        println!("bank2 rec[{i}] @+{rec} type={}", u8::from_le_bytes([buf[rec]]));
        for base in (0..488usize).step_by(32) {
            let dws: Vec<String> = (0..8usize).map(|k| {
                let o = rec + base + k*4;
                if o + 4 > buf.len() { return String::new(); }
                format!("{:08x}", u32::from_le_bytes(buf[o..o+4].try_into().unwrap()))
            }).collect();
            if dws.iter().any(|d| d != "00000000") {
                println!("  +{base:03}: {}", dws.join(" "));
            }
        }
        shown2 += 1;
        if shown2 >= 3 { break; }
    }
}
