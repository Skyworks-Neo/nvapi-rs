//! Live read-only test of the V/F-points GetControl (0xDA025C3E): seed bank
//! masks from GetInfo, GET, dump the first 1060B records of both banks
//! (mode@+36, value@+56, flag@+96). This is the RMW snapshot source for the
//! future SetControl (0xFEC00D04) write path. READ-ONLY.
use nvapi::initialize;
use nvapi::sys::api::NvAPI_EnumPhysicalGPUs;
use nvapi::sys::gpu::clock::private::*;
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use std::ptr;

fn main() {
    let _ = initialize();
    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut count = 0u32;
    unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    let gpu = handles[0];

    use nvapi::sys::api::{
        NvAPI_GPU_ClockClkVfPointsGetControl, NvAPI_GPU_ClockClkVfPointsGetInfo,
    };

    let mut info = Box::new(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1::default());
    let st = unsafe { NvAPI_GPU_ClockClkVfPointsGetInfo(gpu, ptr::from_mut(&mut *info).cast()) };
    println!("GetInfo st={st}");
    assert_eq!(st, 0);

    let mut ctrl = Box::new(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE_V1::default());
    println!("control magic: 0x{:X}", ctrl.version.data);
    ctrl.seed_masks_from_info(&info);
    let st = unsafe {
        NvAPI_GPU_ClockClkVfPointsGetControl(gpu, ptr::from_mut(&mut *ctrl).cast())
    };
    println!("GetControl st={st}");
    if st != 0 {
        return;
    }

    for (bank, rec_base) in [(0usize, clk_vfp_control::REC1), (1, clk_vfp_control::REC2)] {
        let mut shown = 0;
        for idx in 0..clk_vfp_control::POINTS {
            let present = info.point_present(bank, idx) == Some(true);
            if !present {
                continue;
            }
            let typ = ctrl.record_type(bank, idx).unwrap_or(0);
            if typ == 0 {
                continue;
            }
            println!(
                "bank{bank} rec[{idx}] type={typ} mode={:?} value={:?} flag_byte={:?}",
                ctrl.mode(bank, idx),
                ctrl.value(bank, idx),
                {
                    let abs = rec_base + clk_vfp_control::STRIDE * idx + clk_vfp_control::FLAG;
                    ctrl.rest.get(abs - 4).copied()
                },
            );
            shown += 1;
            if shown >= 8 {
                break;
            }
        }
        println!("bank{bank}: {shown} records shown");
    }
}
