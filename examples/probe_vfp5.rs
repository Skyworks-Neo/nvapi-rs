//! Locate the record regions GetStatus actually wrote (beyond the header).
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
    let mut buf = vec![0u8; 85016 + 0x1000];
    buf[0..4].copy_from_slice(&85016u32.to_le_bytes());
    buf[4..132].copy_from_slice(&gi[4..132]);
    assert_eq!(unsafe { status(h, buf.as_mut_ptr()) }, 0);
    // nonzero ranges beyond +132
    let mut ranges = Vec::new();
    let mut start: Option<usize> = None;
    for (i, &b) in buf.iter().enumerate() {
        if b != 0 && start.is_none() && i > 132 { start = Some(i); }
        if b == 0 { if let Some(s) = start.take() { if i - s > 3 { ranges.push((s, i)); } } }
    }
    if let Some(s) = start { ranges.push((s, buf.len())); }
    println!("nonzero ranges beyond +132 (first 12):");
    for (s, e) in ranges.iter().take(12) {
        println!("  +{s}..+{e} ({}B): {}", e - s, buf[*s..(*s + 32).min(*e)].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
    }
    println!("total ranges: {}", ranges.len());
}
