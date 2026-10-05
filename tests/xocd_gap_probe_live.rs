// Read-only probes for the xOCD gap-audit E-matrix
// (docs/reverse-engineering/nvapi/xocd-oc-tool-audit.md §12).
// ZERO writes — every test below is GET-only. Run on the adjudication
// machine (4060L preferred; E1 also meaningful on P100):
//
//   cargo test -p nvapi --test xocd_gap_probe_live -- --nocapture --ignored
//   (single experiment: ... -- --nocapture --ignored e3_power_channels)
//
// E1 vf_points_geometry — public 9248B boost table under BOTH candidate
//                         geometries (nvapioc base-40/delta+20 vs xOCD
//                         base-100/delta+24) → adjudicates audit ①/⑥
// E2 (volt rails)       — SEE tests/volt_rails_raw_dump.rs (already dumps
//                         the rail status; audit ② is adjudicated by
//                         re-parsing that dump in both slot orders)
// E3 power_channels     — 0x67F31384 info v4 + 0x8B3E7343 control with
//                         geometry detection → audit ⑤ (OCP mA vs TGP
//                         mW) + the 0x10A4C stride question
// E4 clk_domains        — V2 control records under the xOCD semantic map
// E5 top_rels           — info gate + ratio resolution → audit ③
// E6 boost_locks        — PerfClientLimits 7-domain table → gap #6
//
// Results also land as JSON under ../reverse/xocd/ for machine diffing.

#![allow(unused_must_use)]

use core::ptr;
use nvapi::PhysicalGpu;
use nvapi::sys::api::{
    NvAPI_GPU_ClientPowerPoliciesGetInfoPrivate, NvAPI_GPU_ClientTgpWattGetStatus,
    NvAPI_GPU_ClockClientClkVfPointsGetControl, NvAPI_GPU_ClockClkDomainsGetControl,
    NvAPI_GPU_ClockClkPropTopRelsGetControl, NvAPI_GPU_ClockClkPropTopRelsGetInfo,
};
use nvapi::sys::gpu::clock::undocumented::{
    NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL2, NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_CONTROL,
    NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_INFO, NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL,
    clk_top_rels_control, clk_top_rels_info,
};
use nvapi::sys::gpu::power::undocumented::{
    NV_GPU_CLIENT_POWER_CHANNELS_INFO, NV_GPU_CLIENT_TGP_WATT_STATUS_10A4C_V1,
};
use nvapi::sys::nvapi::NvVersion;

fn hex_head(buf: &[u8], n: usize) -> String {
    buf.iter()
        .take(n)
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn write_json(name: &str, json: &str) {
    let path = format!("../reverse/xocd/{name}");
    match std::fs::write(&path, json) {
        Ok(()) => eprintln!("snapshot written: {path}"),
        Err(e) => eprintln!("snapshot write failed ({e:?}) — JSON follows:\n{json}"),
    }
}

fn first_gpu() -> PhysicalGpu {
    nvapi::initialize().expect("init");
    let mut gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.remove(0);
    eprintln!("gpu: {:?}", gpu.full_name());
    gpu
}

/// E1 — public V/F boost-table geometry adjudication (audit ①/⑥). Prints
/// the first N entries under BOTH candidate geometries; the operator marks
/// which column matches the live curve (cross-check `get-public-vftable`).
#[test]
#[ignore]
fn e1_vf_points_geometry() {
    let gpu = first_gpu();
    let info = gpu.vfp_info().expect("vfp_info");
    let mut raw = NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL::default();
    raw.mask = info.mask.mask;
    let st = unsafe {
        NvAPI_GPU_ClockClientClkVfPointsGetControl(*gpu.handle(), ptr::from_mut(&mut raw).cast())
    };
    eprintln!(
        "vf points control: status={st:?} magic={:#x}",
        raw.version.data
    );
    assert_eq!(st, 0, "GetControl rejected");
    let bytes = unsafe {
        core::slice::from_raw_parts(
            ptr::from_ref(&raw).cast::<u8>(),
            std::mem::size_of_val(&raw),
        )
    };
    let d =
        |abs: usize| -> i32 { u32::from_le_bytes(bytes[abs..abs + 4].try_into().unwrap()) as i32 };
    eprintln!("payload head[128]: {}", hex_head(bytes, 128));
    eprintln!();
    eprintln!(
        "{:>3} {:>16} {:>16} {:>12} {:>12}",
        "pt", "nvapioc:a60+36i", "xocd:a124+36i", "nvapioc/2", "xocd/2"
    );
    for i in 0..16usize {
        // nvapioc: points base 40, delta at entry+20 → abs 40+36i+20
        let nv = d(40 + 36 * i + 20);
        // xOCD: entries base 100 stride 36 delta@+24, point i≥1 → abs
        // 100+36*(i-1)+24; point 0 anchor printed at its own slot.
        let xo = if i == 0 {
            d(124)
        } else {
            d(100 + 36 * (i - 1) + 24)
        };
        eprintln!("{i:>3} {nv:>16} {xo:>16} {:>12} {:>12}", nv / 2, xo / 2);
    }
    let json = format!(
        "{{\"e\":\"e1\",\"magic\":{},\"nvapioc_deltas\":[{}],\"xocd_deltas\":[{}],\"head128\":\"{}\"}}",
        raw.version.data,
        (0..32)
            .map(|i| d(40 + 36 * i + 20).to_string())
            .collect::<Vec<_>>()
            .join(","),
        (0..32)
            .map(|i| d(100 + 36 * i + 24).to_string())
            .collect::<Vec<_>>()
            .join(","),
        hex_head(bytes, 128)
    );
    write_json("e1-vf-points-geometry.json", &json);
}

/// E3 — PowerChannels info v4 + control geometry detection (audit ⑤ and
/// the 0x10A4C stride question). GET-only.
#[test]
#[ignore]
fn e3_power_channels() {
    let gpu = first_gpu();

    let mut ibuf: Vec<u8> = vec![0u8; std::mem::size_of::<NV_GPU_CLIENT_POWER_CHANNELS_INFO>()];
    let ver =
        <NV_GPU_CLIENT_POWER_CHANNELS_INFO as nvapi::sys::nvapi::StructVersion>::NVAPI_VERSION;
    ibuf[..4].copy_from_slice(&ver.data.to_ne_bytes());
    let st = unsafe {
        NvAPI_GPU_ClientPowerPoliciesGetInfoPrivate(*gpu.handle(), ibuf.as_mut_ptr() as *mut _)
    };
    let _ = &st;
    eprintln!("info v4 (stamp 264816): status={st:?}");
    if st == 0 {
        let info = unsafe { &*(ibuf.as_ptr() as *const NV_GPU_CLIENT_POWER_CHANNELS_INFO) };
        eprintln!("info mask: {:#010x}", info.mask);
        for bit in 0..15u32 {
            if info.mask & (1 << bit) == 0 {
                continue;
            }
            eprintln!(
                "  ch[{bit:2}] id=({},{}) min/def/max={:?}",
                info.channel_id(bit as usize).map(|x| x.0).unwrap_or(0xFFFF),
                info.channel_id(bit as usize).map(|x| x.1).unwrap_or(0xFFFF),
                info.channel_range(bit as usize)
            );
        }
    }

    let mut cbuf: Vec<u8> =
        vec![0u8; std::mem::size_of::<NV_GPU_CLIENT_TGP_WATT_STATUS_10A4C_V1>()];
    cbuf[..4].copy_from_slice(&0x0010_0A4Cu32.to_ne_bytes());
    let st =
        unsafe { NvAPI_GPU_ClientTgpWattGetStatus(*gpu.handle(), cbuf.as_mut_ptr() as *mut _) };
    eprintln!("control 0x10A4C: status={st:?}");
    if st == 0 {
        let ctrl = unsafe { &*(cbuf.as_ptr() as *const NV_GPU_CLIENT_TGP_WATT_STATUS_10A4C_V1) };
        eprintln!("mask: {:#010x}", ctrl.mask);
        eprintln!(
            "compact values (xOCD 40B@28): {:?}",
            (0..6)
                .map(|i| ctrl.channel_value_compact(i).unwrap_or(0xFFFF_FFFF))
                .collect::<Vec<_>>()
        );
        eprintln!(
            "r465 values (136B@1756):      {:?}",
            (0..6)
                .map(|i| ctrl.power_mw(i).unwrap_or(0xFFFF_FFFF))
                .collect::<Vec<_>>()
        );
        eprintln!(
            "payload head[96]: {}",
            hex_head(ctrl.payload.get(0..96).unwrap_or(&[]), 96)
        );
        let json = format!(
            "{{\"e\":\"e3\",\"compact\":[{}],\"r465\":[{}],\"head96\":\"{}\"}}",
            (0..6)
                .map(|i| ctrl
                    .channel_value_compact(i)
                    .unwrap_or(0xFFFF_FFFF)
                    .to_string())
                .collect::<Vec<_>>()
                .join(","),
            (0..6)
                .map(|i| ctrl.power_mw(i).unwrap_or(0xFFFF_FFFF).to_string())
                .collect::<Vec<_>>()
                .join(","),
            hex_head(ctrl.payload.get(0..96).unwrap_or(&[]), 96)
        );
        write_json("e3-power-channels.json", &json);
    }
}

/// E4 — ClkDomains V2 control records under the xOCD semantic map (typed
/// freq/volt slot dispatch per record type). GET-only.
#[test]
#[ignore]
fn e4_clk_domains_semantics() {
    let gpu = first_gpu();
    let mut probe = NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL2::default();
    let mut filled = false;
    for &m in &[0x3FFu32, 0xFFu32] {
        probe = NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL2::default();
        probe.set_mask(m);
        let st = unsafe {
            NvAPI_GPU_ClockClkDomainsGetControl(*gpu.handle(), ptr::from_mut(&mut probe).cast())
        };
        if st == 0 && probe.version.data == 0x261A4 {
            filled = true;
            break;
        }
        eprintln!("seed {m:#x}: status={st:?} magic={:#x}", probe.version.data);
    }
    assert!(filled, "no V2 control block accepted");
    eprintln!(
        "{:>3} {:>6} {:>16} {:>14} {:>14}",
        "bit", "type", "slots(freq,volt)", "freq_kHz", "volt_uV"
    );
    for bit in 0..32u32 {
        let Some(ty) = probe.record_type(bit) else {
            continue;
        };
        if ty == 0 {
            continue;
        }
        let slots = probe.xocd_semantic_slots(bit, false);
        let (f, v) = slots.unwrap_or((usize::MAX, usize::MAX));
        let freq = if f != usize::MAX {
            probe.value(bit, f)
        } else {
            None
        };
        let volt = if v != usize::MAX {
            probe.value(bit, v)
        } else {
            None
        };
        eprintln!(
            "{bit:>3} {ty:#>6} {:>16} {:>14} {:>14}",
            format_args!("({},{})", f, v),
            freq.map(|x| x.to_string()).unwrap_or_else(|| "-".into()),
            volt.map(|x| x.to_string()).unwrap_or_else(|| "-".into()),
        );
    }
}

/// E5 — TopRels edge identity (audit ③): which relation record carries the
/// ratio and what its raw value is. GET-only.
#[test]
#[ignore]
fn e5_top_rels() {
    let gpu = first_gpu();

    let mut info = NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_INFO::zeroed();
    info.version = NvVersion::with_version(clk_top_rels_info::MAGIC);
    let st = unsafe {
        NvAPI_GPU_ClockClkPropTopRelsGetInfo(*gpu.handle(), ptr::from_mut(&mut info).cast())
    };
    eprintln!("info ({:#x}): status={st:?}", clk_top_rels_info::MAGIC);
    if st == 0 {
        eprintln!("gpc_xbar records: {:?}", info.find_gpc_xbar_records());
    }

    let mut ctrl = NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_CONTROL::zeroed();
    ctrl.seed_mask();
    ctrl.version = NvVersion::with_version(clk_top_rels_control::MAGIC);
    let st = unsafe {
        NvAPI_GPU_ClockClkPropTopRelsGetControl(*gpu.handle(), ptr::from_mut(&mut ctrl).cast())
    };
    eprintln!(
        "control ({:#x}): status={:?}",
        clk_top_rels_control::MAGIC,
        st
    );
    if st == 0 {
        eprintln!("ratio resolution: {:?}", ctrl.ratio_raw());
        eprintln!("rec0 tag: {:?}", ctrl.record_tag(0));
        eprintln!(
            "rec0+0x68 raw: {:?}",
            ctrl.u32_at(clk_top_rels_control::REC_RATIO)
        );
        eprintln!("payload head[128]: {}", hex_head(&ctrl.rest, 128));
        // sibling ratio-window dwords — any hit outside rec0+0x68 weakens
        // the "record 0 is THE ratio" assumption.
        for off in (0..ctrl.rest.len().saturating_sub(3)).step_by(4) {
            let v = u32::from_le_bytes(ctrl.rest[off..off + 4].try_into().unwrap());
            if (45_000..80_000).contains(&v) {
                eprintln!(
                    "  ratio-window dword @abs {:#x}: {v} ({:.4})",
                    off + 4,
                    v as f64 / 65536.0
                );
            }
        }
        let json = format!(
            "{{\"e\":\"e5\",\"rec0_tag\":{},\"rec0_ratio\":{},\"gpc_xbar\":[{}]}}",
            ctrl.record_tag(0)
                .map(|x| x.to_string())
                .unwrap_or_else(|| "null".into()),
            ctrl.u32_at(clk_top_rels_control::REC_RATIO)
                .map(|x| x.to_string())
                .unwrap_or_else(|| "null".into()),
            info.find_gpc_xbar_records()
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        write_json("e5-top-rels.json", &json);
    }
}

/// E6 — PerfClientLimits 7-domain lock table presence + decode (gap #6).
/// GET-only.
#[test]
#[ignore]
fn e6_boost_locks() {
    let gpu = first_gpu();
    match gpu.boost_lock_snapshot() {
        Ok(entries) => {
            for e in &entries {
                eprintln!(
                    "id={} mode={} value={} clock_range_lock={} voltage_lock={}",
                    e.id,
                    e.mode,
                    e.value,
                    e.is_clock_range_lock(),
                    e.is_voltage_lock()
                );
            }
            let json = format!(
                "{{\"e\":\"e6\",\"entries\":[{}]}}",
                entries
                    .iter()
                    .map(|e| format!(
                        "{{\"id\":{},\"mode\":{},\"value\":{}}}",
                        e.id, e.mode, e.value
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            write_json("e6-boost-locks.json", &json);
        }
        Err(e) => eprintln!("boost_lock_snapshot: Err {e:?}"),
    }
}
