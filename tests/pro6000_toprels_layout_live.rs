//! Read-only layout probe: pin the GetControl record-ratio offset per slot.
//! Candidates: abs 0x68 + i*0x108 (record base 0) vs 0xCC + i*0x108
//! (record base 0x64, within-record rec+0x68). Known live values after the
//! operator's 0.7 write: slot0 = 0xB333; GetInfo defaults slots 10-13 =
//! 0xCCC0/0x20000/0x13330/0x18000.
//!
//! cargo test --test pro6000_toprels_layout_live -- --nocapture --ignored

use nvapi::PhysicalGpu;
use nvapi::sys::gpu::clock::undocumented::{NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_CONTROL, clk_top_rels_control};
use nvapi::sys::nvapi::NvVersion;

const SLOTS: [usize; 5] = [0, 10, 11, 12, 13];

#[test]
#[ignore = "live GPU read probe"]
fn toprels_record_layout_dump() {
    nvapi::initialize().expect("init");
    let gpu = &PhysicalGpu::enumerate().expect("enumerate")[0];
    println!("gpu: {}", gpu.full_name().unwrap_or_default());

    for &slot in &SLOTS {
        let mut ctl = NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_CONTROL::zeroed();
        ctl.version = NvVersion::with_version(clk_top_rels_control::MAGIC);
        let d0 = 0xFFu32 | (1u32 << slot);
        ctl.rest[0..4].copy_from_slice(&d0.to_le_bytes());
        let st = unsafe {
            nvapi::sys::api::NvAPI_GPU_ClockClkPropTopRelsGetControl(
                *gpu.handle(),
                std::ptr::from_mut(&mut ctl).cast(),
            )
        };
        println!("slot {slot}: GET rc={st}");
        if st != 0 {
            continue;
        }
        // rest-relative = absolute - 4
        let cands = [
            ("rec0+0x68 (abs 0x68+i*0x108)", 0x68 + slot * 0x108),
            ("rec64+0x68 (abs 0xCC+i*0x108)", 0xCC + slot * 0x108),
            ("rec64+0x64 tag (abs 0xC8+i*0x108)", 0xC8 + slot * 0x108),
        ];
        for (name, abs) in cands {
            let off = abs - 4;
            let v = ctl
                .rest
                .get(off..off + 4)
                .map(|s| u32::from_le_bytes(s.try_into().unwrap()));
            println!("  {name}: abs {abs:#x} = {v:#x?}");
        }
        // the shipped helper must agree with the pinned plane
        println!("  api top_rels_ratio_at({slot}) = {:?}", gpu.top_rels_ratio_at(slot));
    }
}
