//! Live recon for the NvpwrControl-derived Blackwell laptop surfaces —
//! READ-ONLY (GET-only, nothing written).
//!   1. ClockClkProp TopRels GetInfo/GetControl (GPC→XBAR U16.16 ratio;
//!      ≤40-series trees are expected EMPTY — that baseline is the point)
//!   2. ClockDomains V2 control block: record type bytes (Ada 0x0A vs
//!      Blackwell 0x0F probe), Blackwell freq/MSVDD anchors, unique-populated
//!      entry (XBAR discovery heuristic)
//!   3. Direct measure 0x527FC458 with +4 ∈ {0..4} — captures the 50-series
//!      index-vs-mask dispute baseline (40-series: 1=XBAR, 2=SYS)
//!
//! Run with:
//!   cargo test -p nvapi --test blackwell_recon_live -- --nocapture --ignored
//!   (under GPU load for meaningful measure values)
//!
//! Optional: NVOC_GPU_IDX=<n> selects the GPU (default 0).
//!
//! See docs/reverse-engineering/nvapi/nvpwrcontrol-blackwell-tuner-audit.md
//! §4 for the 50-series verdict protocol.

use nvapi::PhysicalGpu;
use nvapi::sys::gpu::clock::undocumented::{
    NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE_FREQ_DIRECT_V1,
    NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL_V2, NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_CONTROL_V1,
    NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_INFO_V1, clk_ctrl_entry_v2, clk_ctrl_entry_v2_blackwell,
};
use nvapi::sys::nvapi::NvVersion;
use nvapi::{initialize, sys};

fn box_zeroed<T>() -> Box<T> {
    unsafe {
        let b = Box::<T>::new_zeroed();
        b.assume_init()
    }
}

#[test]
#[ignore = "live GPU read-only recon"]
fn blackwell_recon() {
    let idx: usize = std::env::var("NVOC_GPU_IDX")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    initialize().expect("NvAPI init failed");
    let gpus = PhysicalGpu::enumerate().expect("enumerate failed");
    let gpu = gpus.get(idx).expect("gpu index out of range");
    println!("gpu: {}", gpu.full_name().unwrap_or_default());

    // ---- 1. TopRels GET_INFO ----------------------------------------------
    println!(
        "\n== TopRels GetInfo (0xE826E4F0, magic {:#x}) ==",
        nvapi::sys::gpu::clock::undocumented::clk_top_rels_info::MAGIC
    );
    let mut info = box_zeroed::<NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_INFO_V1>();
    info.version =
        NvVersion::with_version(nvapi::sys::gpu::clock::undocumented::clk_top_rels_info::MAGIC);
    let st = unsafe { sys::api::NvAPI_GPU_ClockClkPropTopRelsGetInfo(*gpu.handle(), &mut *info) };
    println!("status: {st:?}");
    if st == 0 {
        println!(
            "relations: count={} gpc_xbar_records={:?}",
            info.relation_count(),
            info.find_gpc_xbar_records()
        );
        for i in 0..255usize {
            if info.relation_masked(i) != Some(true) {
                continue;
            }
            let ratio = info.relation_payload(i).unwrap_or(0);
            println!(
                "  rec {i}: enum={} bytes={:?} payload={ratio:#x} ({:.4}) payload2={:#x}",
                info.relation_enum(i).unwrap_or(0),
                info.relation_raw_bytes(i),
                f64::from(ratio) / 65536.0,
                info.relation_payload2(i).unwrap_or(0),
            );
        }
    }

    // ---- 2. TopRels GET_CONTROL -------------------------------------------
    println!(
        "\n== TopRels GetControl (0xCBFF71D0, magic {:#x}) ==",
        nvapi::sys::gpu::clock::undocumented::clk_top_rels_control::MAGIC
    );
    let mut ctrl = box_zeroed::<NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_CONTROL_V1>();
    ctrl.version =
        NvVersion::with_version(nvapi::sys::gpu::clock::undocumented::clk_top_rels_control::MAGIC);
    ctrl.seed_mask();
    let st =
        unsafe { sys::api::NvAPI_GPU_ClockClkPropTopRelsGetControl(*gpu.handle(), &mut *ctrl) };
    println!("status: {st:?}");
    if st == 0 {
        match ctrl.ratio_raw() {
            Some((off, raw)) => println!(
                "ratio: raw={raw:#x} ({:.4}) @abs {off:#x}",
                f64::from(raw) / 65536.0
            ),
            None => println!("ratio: UNRESOLVED (ambiguous / not populated)"),
        }
    }

    // ---- 3. ClockDomains V2 control block ---------------------------------
    println!("\n== ClockDomains V2 GetControl (0xF58938F5, magic 0x261A4) ==");
    let mut dom = box_zeroed::<NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL_V2>();
    dom.version = NvVersion::new(size_of::<NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL_V2>(), 2);
    dom.set_mask(0xFF);
    // Same entry as the V1 stamp — the handler dispatches on the version
    // dword, so the V2 block is passed through a pointer cast.
    let st = unsafe {
        sys::api::NvAPI_GPU_ClockClkDomainsGetControl(
            *gpu.handle(),
            std::ptr::from_mut(&mut *dom).cast(),
        )
    };
    println!("status: {st:?}");
    if st == 0 {
        println!("controllable mask: {:#x}", dom.controllable_mask());
        for bit in 0..32u32 {
            let Some(t) = dom.record_type(bit) else {
                continue;
            };
            if t == 0 {
                continue;
            }
            let gen = match dom.record_type_blackwell(bit) {
                Some(true) => " BW(0x0F)",
                Some(false) => " ada(0x0A)",
                None => "",
            };
            println!(
                "  dom {bit:2}: type={t:#04x}{gen} freq_khz={:?} msvdd_uv={:?}",
                dom.bw_freq_khz(bit),
                dom.bw_msvdd_uv(bit),
            );
        }
        println!(
            "unique populated entry (XBAR heuristic): {:?}",
            dom.find_unique_populated_entry()
        );
        let _ = clk_ctrl_entry_v2::BASE;
        let _ = clk_ctrl_entry_v2_blackwell::TYPE_BLACKWELL;
    }

    // ---- 4. Direct measure sweep (index-vs-mask dispute) -------------------
    println!("\n== Direct measure 0x527FC458, +4 = 0..4 ==");
    for d in 0u32..=4 {
        let mut m = NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE_FREQ_DIRECT_V1 {
            version: NvVersion::new(
                size_of::<NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE_FREQ_DIRECT_V1>(),
                1,
            ),
            domain_index: d,
            freq_khz: 0,
        };
        let st = unsafe { sys::api::NvAPI_GPU_ClockClkDomainsMeasureFreq(*gpu.handle(), &mut m) };
        println!(
            "  +4={d}: status={st:?} freq={:.3} MHz",
            f64::from(m.freq_khz) / 1000.0
        );
    }
    println!("\n(done — nothing was written; audit §4.3 has the 50-series verdict protocol)");
}
