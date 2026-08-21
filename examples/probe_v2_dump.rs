//! V2 (magic 0x261A4, 24996B) GET_CONTROL full record dump for GPC(bit0) and
//! XBAR(bit1). Records @+292+772*idx; type-0xB value dwords at rec+268..296.
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

    let mut buf = vec![0u8; 0x61A4];
    buf[0..4].copy_from_slice(&0x261A4u32.to_le_bytes());
    buf[8..12].copy_from_slice(&0xFFu32.to_le_bytes());
    let st = unsafe { f(h, buf.as_mut_ptr()) };
    println!("V2 GET_CONTROL st={st}");
    if st != 0 { return; }

    for (name, idx) in [("GPC", 0usize), ("XBAR", 1), ("SYS", 2), ("MCLK", 4)] {
        let rec = 292 + 772 * idx;
        let typ = u32::from_le_bytes(buf[rec..rec+4].try_into().unwrap());
        println!("{name:5} rec@+{rec} type={typ}");
        // dump dwords rec+260..rec+300 (the type-0xB value area)
        for base in (260..300usize).step_by(16) {
            let off = rec + base;
            let dws: Vec<String> = (0..4usize)
                .map(|i| format!("+{}={}", base + i*4,
                    i32::from_le_bytes(buf[off+i*4..off+i*4+4].try_into().unwrap())))
                .collect();
            println!("   {}", dws.join("  "));
        }
    }
}
