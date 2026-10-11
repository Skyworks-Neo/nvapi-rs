//! TopRels census — READ-ONLY. Answers: how many control slots does the
//! TopRels face actually populate on RTX PRO 6000 Blackwell, which topology
//! edges carry control payloads (ratio-like dwords), and what tags they use.
//!
//!   cargo test -p nvapi --test pro6000_toprels_census_live -- --nocapture --ignored

use nvapi::PhysicalGpu;
use nvapi::sys::api::{
    NvAPI_GPU_ClockClkPropTopRelsGetControl, NvAPI_GPU_ClockClkPropTopRelsGetInfo,
};
use nvapi::sys::gpu::clock::undocumented::{
    NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_CONTROL_V1,
    NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_INFO_V1, clk_top_rels_control, clk_top_rels_info,
};
use nvapi::sys::nvapi::NvVersion;

fn box_zeroed<T>() -> Box<T> {
    unsafe {
        let b = Box::<T>::new_zeroed();
        b.assume_init()
    }
}

#[test]
#[ignore = "live GPU READ-ONLY census"]
fn toprels_census() {
    nvapi::initialize().expect("init");
    let gpu = &PhysicalGpu::enumerate().expect("enumerate")[0];
    println!("gpu: {}", gpu.full_name().unwrap_or_default());

    // ---- GetInfo: topology edges ------------------------------------------
    println!("\n== GetInfo: relation table ==");
    let mut info = box_zeroed::<NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_INFO_V1>();
    info.version = NvVersion::with_version(clk_top_rels_info::MAGIC);
    let st = unsafe {
        nvapi::sys::api::NvAPI_GPU_ClockClkPropTopRelsGetInfo(
            *gpu.handle(),
            std::ptr::from_mut(&mut *info).cast(),
        )
    };
    println!("GetInfo rc = {st}");
    if st == 0 {
        println!("relation_count = {}", info.relation_count());
        for i in 0..255usize {
            if info.relation_masked(i) != Some(true) {
                continue;
            }
            println!(
                "  rel {i}: enum={} raw={:?} payload={:#x} payload2={:#x}",
                info.relation_enum(i).unwrap_or(0),
                info.relation_raw_bytes(i),
                info.relation_payload(i).unwrap_or(0),
                info.relation_payload2(i).unwrap_or(0)
            );
        }
    }

    // ---- GetControl census at increasing seeds ----------------------------
    // seed combos: (dword0@+4, mask dword1@+8, dword2@+12) — the 256-bit
    // relation mask spans +4..+36; bits 10..13 (the envelope records) live
    // in dword0's high half
    for (d0, d1, d2) in [
        (0xFFu32, 0u32, 0u32),
        (0x3CFF, 0, 0),      // rel 0-7 + 10-13 in dword0
        (0xFFFF, 0, 0),      // rel 0-15
        (0xFFFF_FFFF, 0, 0), // all dword0
        (0xFF, 0x3C00, 0),   // rel 0-7 + 42-45 (dword1 high bits)
        (0xFFFF_FFFF, 0xFFFF_FFFF, 0xFFFF_FFFF),
    ] {
        println!("\n== GetControl seed d0={d0:#x} d1={d1:#x} d2={d2:#x} ==");
        let mut ctl = box_zeroed::<NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_CONTROL_V1>();
        ctl.version = NvVersion::with_version(clk_top_rels_control::MAGIC);
        ctl.rest[0..4].copy_from_slice(&d0.to_le_bytes());
        ctl.rest[4..8].copy_from_slice(&d1.to_le_bytes());
        ctl.rest[8..12].copy_from_slice(&d2.to_le_bytes());
        let st = unsafe {
            NvAPI_GPU_ClockClkPropTopRelsGetControl(
                *gpu.handle(),
                std::ptr::from_mut(&mut *ctl).cast(),
            )
        };
        println!("rc = {st}");
        if st != 0 {
            continue;
        }
        // records at rest 0x60 + i*0x108 (abs 0x64 + ...); dump any populated one
        let mut populated = 0;
        for i in 0..16usize {
            let base = 0x60 + i * 0x108;
            let dwords: Vec<u32> = (0..0x108 / 4)
                .filter_map(|d| {
                    let o = base + d * 4;
                    if o + 4 <= ctl.rest.len() {
                        Some(u32::from_le_bytes(ctl.rest[o..o + 4].try_into().unwrap()))
                    } else {
                        None
                    }
                })
                .collect();
            if dwords.iter().any(|&v| v != 0) {
                populated += 1;
                let tag = ctl.rest.get(base + 0x64).copied().unwrap_or(0);
                println!(
                    "  rec {i}: tag={tag:#x} nonzero_dwords={:?}",
                    dwords
                        .iter()
                        .enumerate()
                        .filter(|(_, v)| **v != 0)
                        .map(|(d, v)| (d, v))
                        .take(12)
                        .collect::<Vec<_>>()
                );
                // ratio-slot dword (rec+4) explicitly
                println!(
                    "    rec+4 (ratio slot) = {:#x}",
                    dwords.get(1).unwrap_or(&0)
                );
            }
        }
        println!("populated records: {populated}");
    }

    println!("\n(read: populated records x tags x ratio-slot dwords = the full");
    println!(" control-slot inventory; any second ratio-bearing edge = a new trim knob)");
}
