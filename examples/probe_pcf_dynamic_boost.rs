//! Probe the PCF Dynamic-Boost controller table (2026-08-26 follow-up):
//! GET 0xC80068A1 reports `no` even with PPAB enabled. Static RE of R610.74
//! nvapi64 impl shows:
//!
//!   sub_180067050: pick platform entry idx from a 71836-magic table
//!                  (196B stride, first rec with byte0==1 && byte[95])
//!   GET 0xC80068A1: dump PCF table (via PCF_ControllerGetControl 0x93456591,
//!                  {mask, size=68744} + 32 x 100B records), then
//!                  active = rec[0]&0xFF==1 && rec[+60]!=2 && rec[+61]!=2
//!   SET 0x1504FC3D: rec[0]=1, rec[+60] = enable?1:2  (never touches +61)
//!
//! So byte +61 is a driver-side second status the client SET cannot write.
//! This probe dumps the whole controller record to see which byte is 2.
//!
//! Run: cargo run --release -p nvapi --example probe_pcf_dynamic_boost

use nvapi::initialize;
use nvapi::sys::api::NvAPI_PCF_DynamicBoostGetStatus;
use nvapi::sys::nvapi_QueryInterface;

#[repr(C)]
struct PcfControl {
    mask: u32,
    size: u32,
    records: [u8; 32 * 100],
}

fn main() {
    let _ = initialize();

    type PcfControllerGetControl = unsafe extern "system" fn(*mut PcfControl) -> i32;
    let get_control: Option<PcfControllerGetControl> = nvapi_QueryInterface(0x93456591)
        .ok()
        .and_then(|p| unsafe { std::mem::transmute(p) });
    let Some(get_control) = get_control else {
        println!("PCF_ControllerGetControl 0x93456591: NOT RESOLVED");
        return;
    };
    println!("PCF_ControllerGetControl 0x93456591: resolved");

    // Private lifecycle init (0xAD298D3F arg=1), same as medium does.
    let _ = nvapi::sys::api::private::NvAPI_GPU_PrivateLifecycleInit;
    {
        type LifecycleInit = unsafe extern "system" fn(u32) -> i32;
        let f: Option<LifecycleInit> = nvapi_QueryInterface(0xAD298D3F)
            .ok()
            .and_then(|p| unsafe { std::mem::transmute(p) });
        if let Some(f) = f {
            println!("PrivateLifecycleInit(1) st={}", unsafe { f(1) });
        }
    }

    for idx in 0u8..4 {
        let mut ctl = Box::new(PcfControl {
            mask: 1 << idx,
            size: 68744,
            records: [0u8; 3200],
        });
        let st = unsafe { get_control(&mut *ctl) };
        let rec = &ctl.records[100 * idx as usize..100 * (idx as usize + 1)];
        println!(
            "idx {idx}: st={st} rec[0]={:02X} +60={:02X} +61={:02X}",
            rec[0], rec[60], rec[61]
        );
        if rec[0] & 0xFF == 1 || rec.iter().any(|&b| b != 0) {
            println!("  full: {:02X?}", &rec[..100]);
        }
    }

    // The GET itself, for side-by-side.
    let mut active = nvapi::sys::types::BoolU32(0);
    let st = unsafe { NvAPI_PCF_DynamicBoostGetStatus(&mut active) };
    println!("PCF_DynamicBoostGetStatus st={st} active={}", active.0);
}
