//! Distinguish "driver wrote real zeros" from "field untouched": pre-fill the
//! V2 buffer with 0xCC and see which bytes survive.
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
    let f: RawFn = unsafe { std::mem::transmute(nvapi_QueryInterface(0xF58938F5).expect("resolve")) };

    let mut buf = vec![0xCCu8; 0x61A4];
    buf[0..4].copy_from_slice(&0x261A4u32.to_le_bytes());
    buf[8..12].copy_from_slice(&0xFFu32.to_le_bytes());
    let st = unsafe { f(h, buf.as_mut_ptr()) };
    println!("V2 (0xCC prefill) st={st}");

    // header area
    println!("hdr +0..44: {}", buf[0..44].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
    for (name, idx) in [("GPC", 0usize), ("XBAR", 1), ("MCLK", 4), ("Disp(bit6)", 6)] {
        let rec = 292 + 772 * idx;
        println!("{name:10} rec@+{rec}: rec+0..0x10: {}", buf[rec..rec+16].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
        // value area: which bytes changed from 0xCC?
        let area = &buf[rec+260..rec+300];
        let changed: Vec<usize> = area.iter().enumerate().filter(|(_, &b)| b != 0xCC).map(|(i, _)| 260 + i).collect();
        println!("           value area +260..300 changed offsets: {:?}", changed);
        let dws: Vec<String> = (0..8usize).map(|i| format!("+{}={}", 268+i*4,
            i32::from_le_bytes(buf[rec+268+i*4..rec+268+i*4+4].try_into().unwrap()))).collect();
        println!("           dwords: {}", dws.join("  "));
    }
}
