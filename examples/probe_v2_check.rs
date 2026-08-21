//! Decide why V2 (0x26154) GET_CONTROL returns -9 live despite the impl-DLL
//! handler accepting both magics: repeat calls, order variations, mask
//! variations. Read-only.
use nvapi::sys::api::NvAPI_EnumPhysicalGPUs;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::nvapi_QueryInterface;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;

type RawFn = unsafe extern "C" fn(usize, *mut u8) -> i32;

fn main() {
    nvapi::initialize();
    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    let h = handles[0].as_ptr() as usize;

    let f: RawFn = unsafe {
        std::mem::transmute(nvapi_QueryInterface(0xF58938F5).expect("resolve"))
    };

    for &(magic, mask) in &[
        (0x261A4u32, 0xFFu32),
        (0x261A4, 0x17),
        (0x261A4, 0xFFFFFFFF),
        (0x10964, 0xFF),
        (0x261A4, 0xFF), // repeat after a V1 success
    ] {
        let mut buf = vec![0u8; 0x61A4 + 0x1000];
        buf[0..4].copy_from_slice(&magic.to_le_bytes());
        buf[8..12].copy_from_slice(&mask.to_le_bytes());
        let st = unsafe { f(h, buf.as_mut_ptr()) };
        // nonzero-dword census to see if anything was written
        let nz = buf.iter().filter(|&&b| b != 0).count();
        println!("magic=0x{magic:X} mask=0x{mask:X} -> st={st}, nonzero bytes={nz}");
        if st == 0 && magic == 0x261A4 {
            // dump record 0 (base +292) and record 1 (+292+772)
            for (i, off) in [292usize, 292 + 772, 292 + 2 * 772].iter().enumerate() {
                println!("  V2 rec[{i}] @+{off}: {}", buf[*off..*off + 0x20]
                    .iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
            }
        }
    }
}
