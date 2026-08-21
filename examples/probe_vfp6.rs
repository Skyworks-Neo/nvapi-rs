//! Try the LARGER GetStatus magics with the GetInfo-seeded header.
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

    for &m in &[2000388u32, 1525252, 300164, 214652, 158200] {
        let mut buf = vec![0u8; (m as usize) + 0x1000];
        buf[0..4].copy_from_slice(&m.to_le_bytes());
        buf[4..132].copy_from_slice(&gi[4..132]);
        let st = unsafe { status(h, buf.as_mut_ptr()) };
        // find first nonzero beyond +132
        let first = buf[132..].iter().position(|&b| b != 0).map(|p| p + 132);
        println!("m={m} (0x{m:X}): st={st}, first-nonzero-after-hdr={:?}",
            first.map(|f| { let d = &buf[f..(f + 48).min(buf.len())]; (f, d.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")) }));
    }
}
