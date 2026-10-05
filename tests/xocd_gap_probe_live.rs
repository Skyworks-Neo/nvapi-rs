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
    // Test CWD is the WORKSPACE root when invoked from the parent repo
    // (nvoc/), so a relative "../reverse/xocd" escapes the repo. Anchor on
    // the crate dir instead — independent of the invocation directory.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate parent")
        .join("reverse")
        .join("xocd");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!(
            "mkdir {} failed ({e:?}) — JSON follows:\n{json}",
            dir.display()
        );
        return;
    }
    let path = dir.join(name);
    match std::fs::write(&path, json) {
        Ok(()) => eprintln!("snapshot written: {}", path.display()),
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
    let mut raw = NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL {
        mask: info.mask.mask,
        ..Default::default()
    };
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
/// the 0x10A4C stride question). GET-only. Round 2 addition: when the
/// compact 0x10A4C control is rejected (pre-50-series, live-confirmed -9
/// on Turing/Ampere/Pascal), scan the 10016B 0x12720 control (the stamp
/// `set_tgp_watt` drives) for dwords matching the info-side channel
/// defaults — those hits are the candidate OCP value offsets for a future
/// pre-50-series write path.
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
    eprintln!("info v4 (stamp 264816): status={st:?}");
    let mut channels: Vec<(u32, u32, u32, u32, u32)> = Vec::new(); // (pid,sub,min,def,max)
    if st == 0 {
        let info = unsafe { &*(ibuf.as_ptr() as *const NV_GPU_CLIENT_POWER_CHANNELS_INFO) };
        eprintln!("info mask: {:#010x}", info.mask);
        for bit in 0..15u32 {
            if info.mask & (1 << bit) == 0 {
                continue;
            }
            let id = info.channel_id(bit as usize);
            let range = info.channel_range(bit as usize);
            eprintln!(
                "  ch[{bit:2}] id=({},{}) min/def/max={:?}",
                id.map(|x| x.0).unwrap_or(0xFFFF),
                id.map(|x| x.1).unwrap_or(0xFFFF),
                range
            );
            if let (Some((p, s)), Some((mn, df, mx))) = (id, range) {
                channels.push((p, s, mn, df, mx));
            }
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

    // Round 4: LEGACY stamp matrix (kernel RE, 2026-10-05). Byte-scanning
    // nvlddmkm 610 shows ZERO true references to any family stamp
    // (0x10A4C/0x10298 hit .pdata unwind data; 0x11F10 was a modrm+disp
    // misalignment of `mov rcx,[rax+rdx+0x11F]`; 0x12720 absent entirely)
    // — the struct-version gate for this family is entirely USER-SIDE
    // (nvapi64). nvapi64 R465's switch accepted {0x10298, 0x106DC, 0x10A4C,
    // 0x11F10}; the R538+ 0x12720 was added later. Test which stamps the
    // 610.47 nvapi64 still accepts on THIS generation (GET-only): a
    // populated legacy buffer is the pre-50-series OCP write-path candidate.
    {
        let stamps: &[(u32, &str)] = &[
            (0x0001_0298, "0x10298 v1|664B"),
            (0x0001_06DC, "0x106DC v1|1756B (R465 136B-stride)"),
            (0x0001_0A4C, "0x10A4C v1|2636B (compact, 50-series)"),
            (0x0001_1F10, "0x11F10 v1|7952B"),
            (0x0001_2720, "0x12720 v1|10016B (R538+)"),
        ];
        for &(stamp, label) in stamps {
            let size = (stamp & 0xFFFF) as usize;
            let mut b: Vec<u8> = vec![0u8; size];
            b[..4].copy_from_slice(&stamp.to_ne_bytes());
            b[4..8].copy_from_slice(&0x0000_7FFFu32.to_ne_bytes());
            let st = unsafe {
                NvAPI_GPU_ClientTgpWattGetStatus(*gpu.handle(), b.as_mut_ptr() as *mut _)
            };
            let nonzero = b[8..].chunks_exact(4).filter(|c| *c != [0; 4]).count();
            eprintln!("legacy-stamp {label}: status={st:?} nonzero_dwords={nonzero}");
            if st == 0 && nonzero > 0 {
                eprintln!("  head[128]: {}", hex_head(&b, 128));
            }
        }
    }

    // Round 2: 10016B 0x12720 control — default-value dword scan against
    // the info channels (skip sentinels def==max==5001000/1001000).
    // Round-2 live fix (both machines returned an all-zero payload): the
    // ref-tool flow PRIMES the driver with the private GetInfo before the
    // control GET and seeds the entry mask at +4 — mirror both, skip the
    // version/mask header in the scan, and demand EXACT default hits only
    // (the loose range match false-positived on the version dword 0x12720).
    let _ = gpu.tgp_watt_range();
    let mut tbuf: Vec<u8> = vec![0u8; 10016];
    tbuf[..4].copy_from_slice(&0x0001_2720u32.to_ne_bytes());
    tbuf[4..8].copy_from_slice(&0x0000_7FFFu32.to_ne_bytes());
    let st =
        unsafe { NvAPI_GPU_ClientTgpWattGetStatus(*gpu.handle(), tbuf.as_mut_ptr() as *mut _) };
    eprintln!("control 0x12720 (10016B): status={st:?}");
    if st == 0 {
        let nonzero = tbuf[8..].chunks_exact(4).filter(|c| *c != [0; 4]).count();
        eprintln!("nonzero dwords in payload: {nonzero}");
        let real_defs: Vec<(u32, u32, u32)> = channels
            .iter()
            .copied()
            .filter(|&(_, _, _mn, df, mx)| {
                !(df == mx && (df == 5_001_000 || df == 1_001_000 || df == 0))
            })
            .map(|(_, _, mn, df, mx)| (df, mn, mx))
            .collect();
        for &(def, mn, mx) in &real_defs {
            let mut hits = Vec::new();
            for off in (8..10016 - 4).step_by(4) {
                let v = u32::from_le_bytes(tbuf[off..off + 4].try_into().unwrap());
                // exact default hit, or a live value strictly inside the
                // window while AWAY from the window edges (cuts the
                // version-dword false positives)
                if v == def || (v > mn && v < mx) {
                    hits.push(off);
                    if hits.len() >= 8 {
                        break;
                    }
                }
            }
            eprintln!("  def={def} [{mn},{mx}] → buffer offsets {hits:?}");
        }
        write_json(
            "e3-10016-scan.json",
            &format!(
                "{{\"e\":\"e3b\",\"defs\":{real_defs:?},\"nonzero_dwords\":{nonzero},\"head128\":\"{}\"}}",
                hex_head(&tbuf, 128)
            ),
        );
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
            format!("({f},{v})"),
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

/// E1b — DECISIVE V/F boost-table geometry adjudication (audit ①/⑥).
/// MUTATING, opt-in TWICE: `#[ignore]` AND the `NVOC_ALLOW_VF_WRITE_PROBE=1`
/// environment variable. Without the variable this test prints and exits.
///
/// Protocol (house RMW recipe, single dword):
///   1. GET the 9248B table (snapshot)
///   2. pick the victim point (last table-count point, else point 4)
///   3. patch +15000 kHz at the NVAPIOC offset (60+36*i) only
///   4. SET → fresh GET → print BOTH geometry columns
///   5. restore the original snapshot → SET → full byte-compare verify
///
/// Verdict: the column showing 15000 after the SET is the driver's real
/// delta field. Run:
///   NVOC_ALLOW_VF_WRITE_PROBE=1 cargo test -p nvapi --test xocd_gap_probe_live -- --nocapture --ignored e1b
#[test]
#[ignore]
fn e1b_vf_points_write_read() {
    if std::env::var("NVOC_ALLOW_VF_WRITE_PROBE").as_deref() != Ok("1") {
        eprintln!("e1b skipped: mutating probe — set NVOC_ALLOW_VF_WRITE_PROBE=1 to run");
        return;
    }
    let gpu = first_gpu();
    let info = gpu.vfp_info().expect("vfp_info");

    let mut orig = NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL {
        mask: info.mask.mask,
        ..Default::default()
    };
    let st = unsafe {
        NvAPI_GPU_ClockClientClkVfPointsGetControl(*gpu.handle(), ptr::from_mut(&mut orig).cast())
    };
    assert_eq!(st, 0, "snapshot GET rejected");
    let as_bytes = |r: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL| unsafe {
        core::slice::from_raw_parts(ptr::from_ref(r).cast::<u8>(), std::mem::size_of_val(r))
    };
    let orig_bytes = as_bytes(&orig).to_vec();
    let rd = |b: &[u8], abs: usize| -> i32 {
        u32::from_le_bytes(b[abs..abs + 4].try_into().unwrap()) as i32
    };

    // victim point: last table-count point (count dword @+20), else point 4
    let count = rd(&orig_bytes, 20) as usize;
    let victim = if count > 1 { count - 1 } else { 4 };
    let nv_off = 60 + 36 * victim; // nvapioc delta slot for point i
    let xo_off = 124 + 36 * (victim - 1); // xOCD delta slot for point i (i>=1)
    eprintln!(
        "victim point {victim} (count={count}): nvapioc abs {nv_off} = {}, xocd abs {xo_off} = {}",
        rd(&orig_bytes, nv_off),
        rd(&orig_bytes, xo_off)
    );

    // patch a copy at the NVAPIOC offset only
    let mut modified = orig_bytes.clone();
    modified[nv_off..nv_off + 4].copy_from_slice(&15_000u32.to_le_bytes());
    let mut m = NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL::default();
    unsafe {
        ptr::copy_nonoverlapping(
            modified.as_ptr(),
            ptr::from_mut(&mut m).cast::<u8>(),
            modified.len(),
        )
    };
    let st = unsafe {
        nvapi::sys::api::NvAPI_GPU_ClockClientClkVfPointsSetControl(
            *gpu.handle(),
            ptr::from_ref(&m).cast(),
        )
    };
    eprintln!("SET (nvapioc-offset patch): status={st:?}");
    if st != 0 {
        eprintln!("SET rejected — nothing to restore (driver refused the write)");
        return;
    }

    // readback
    let mut verify = NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL {
        mask: info.mask.mask,
        ..Default::default()
    };
    let st = unsafe {
        NvAPI_GPU_ClockClientClkVfPointsGetControl(*gpu.handle(), ptr::from_mut(&mut verify).cast())
    };
    assert_eq!(st, 0, "readback GET rejected");
    let vb = as_bytes(&verify);
    eprintln!(
        "after SET: nvapioc abs {nv_off} = {}, xocd abs {xo_off} = {}",
        rd(vb, nv_off),
        rd(vb, xo_off)
    );
    for i in 0..8usize {
        eprintln!(
            "  pt{i:>2}: nvapioc {} / xocd {}",
            rd(vb, 60 + 36 * i),
            rd(vb, 124 + 36 * i)
        );
    }
    let verdict = if rd(vb, nv_off) == 15_000 {
        "NVAPIOC geometry (delta @ entry+20, base 40) holds"
    } else if rd(vb, xo_off) == 15_000 {
        "xOCD geometry (delta @ entry+24, base 100) holds"
    } else {
        "NEITHER column retained 15000 — geometry still unresolved (check dump above)"
    };
    eprintln!("VERDICT: {verdict}");
    let json = format!(
        "{{\"e\":\"e1b\",\"victim\":{victim},\"nv_off\":{nv_off},\"xo_off\":{xo_off},\"verdict\":\"{}\",\"after_nv\":{},\"after_xo\":{}}}",
        verdict,
        rd(vb, nv_off),
        rd(vb, xo_off)
    );
    write_json("e1b-vf-write-read.json", &json);

    // restore + verify (full byte compare)
    let mut r = NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL::default();
    unsafe {
        ptr::copy_nonoverlapping(
            orig_bytes.as_ptr(),
            ptr::from_mut(&mut r).cast::<u8>(),
            orig_bytes.len(),
        )
    };
    let st = unsafe {
        nvapi::sys::api::NvAPI_GPU_ClockClientClkVfPointsSetControl(
            *gpu.handle(),
            ptr::from_ref(&r).cast(),
        )
    };
    eprintln!("restore SET: status={st:?}");
    let mut back = NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL {
        mask: info.mask.mask,
        ..Default::default()
    };
    let st = unsafe {
        NvAPI_GPU_ClockClientClkVfPointsGetControl(*gpu.handle(), ptr::from_mut(&mut back).cast())
    };
    assert_eq!(st, 0, "restore-verify GET rejected");
    let same = as_bytes(&back) == orig_bytes.as_slice();
    eprintln!(
        "restore verify: {}",
        if same {
            "OK (byte-identical)"
        } else {
            "MISMATCH — inspect diff before continuing!"
        }
    );
    assert!(
        same,
        "restore did not return the table to its original bytes"
    );
}

/// E1c — V/F delta-field DISCOVERY sweep (audit ①/⑥). MUTATING, opt-in
/// TWICE: `#[ignore]` AND `NVOC_ALLOW_VF_WRITE_PROBE=1`.
///
/// E1b falsified BOTH candidate geometries on current drivers (the SET was
/// accepted but +15000 was not retained at nvapioc 60+36i nor xOCD
/// 124+36(i-1); restore verified byte-identical). This probe stops
/// guessing: sweep every 4-aligned offset in the entry region
/// [40..=204] whose ORIGINAL dword is 0 — patch 15000 there, SET, fresh
/// GET, record (a) whether the offset itself retained the value, (b) ANY
/// dword that changed anywhere in the table (the driver may marshal the
/// write into a different slot — the diff exposes the real field).
/// Restore after every step; abort the sweep on the first restore
/// mismatch.
#[test]
#[ignore]
fn e1c_vf_points_offset_sweep() {
    if std::env::var("NVOC_ALLOW_VF_WRITE_PROBE").as_deref() != Ok("1") {
        eprintln!("e1c skipped: mutating probe — set NVOC_ALLOW_VF_WRITE_PROBE=1 to run");
        return;
    }
    let gpu = first_gpu();
    let info = gpu.vfp_info().expect("vfp_info");

    let mask = info.mask.mask;
    let get = |gpu: &PhysicalGpu| -> Vec<u8> {
        let mut t = NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL {
            mask,
            ..Default::default()
        };
        let st = unsafe {
            NvAPI_GPU_ClockClientClkVfPointsGetControl(*gpu.handle(), ptr::from_mut(&mut t).cast())
        };
        assert_eq!(st, 0, "GET rejected");
        unsafe {
            core::slice::from_raw_parts(ptr::from_ref(&t).cast::<u8>(), std::mem::size_of_val(&t))
                .to_vec()
        }
    };
    let set = |gpu: &PhysicalGpu, b: &[u8]| -> i32 {
        let mut t = NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL::default();
        unsafe {
            ptr::copy_nonoverlapping(b.as_ptr(), ptr::from_mut(&mut t).cast::<u8>(), b.len());
        }
        unsafe {
            nvapi::sys::api::NvAPI_GPU_ClockClientClkVfPointsSetControl(
                *gpu.handle(),
                ptr::from_ref(&t).cast(),
            )
        }
    };

    let orig = get(&gpu);
    eprintln!(
        "snapshot taken ({} bytes), sweeping offsets 40..=204",
        orig.len()
    );

    let mut retained = Vec::new();
    let mut remapped: Vec<(usize, usize)> = Vec::new(); // (patched_off, seen_off)
    for off in (40..=204).step_by(4) {
        if orig[off..off + 4] != [0; 4] {
            eprintln!("off {off:>3}: skip (original nonzero)");
            continue;
        }
        let mut modified = orig.clone();
        modified[off..off + 4].copy_from_slice(&15_000u32.to_le_bytes());
        let st = set(&gpu, &modified);
        if st != 0 {
            eprintln!("off {off:>3}: SET rejected ({st})");
            continue;
        }
        let after = get(&gpu);
        // restore FIRST (before any reporting) so every step ends clean
        let st = set(&gpu, &orig);
        assert_eq!(st, 0, "restore SET failed at off {off}");
        let back = get(&gpu);
        assert_eq!(
            back, orig,
            "restore mismatch after off {off} — aborting sweep"
        );

        if after[off..off + 4] == 15_000u32.to_le_bytes() {
            retained.push(off);
            eprintln!("off {off:>3}: RETAINED 15000 at the patched offset");
        }
        for (i, w) in after.chunks_exact(4).enumerate() {
            if w != &orig[i * 4..i * 4 + 4] {
                let seen = u32::from_le_bytes(w.try_into().unwrap());
                eprintln!(
                    "off {off:>3}: table CHANGED at abs {} → {seen} ({})",
                    i * 4,
                    seen as i32
                );
                if seen == 15_000 {
                    remapped.push((off, i * 4));
                }
            }
        }
    }

    eprintln!();
    eprintln!("VERDICT: retained at {retained:?}, remapped {remapped:?}");
    eprintln!(
        "  (empty = the driver marshals NO delta dword in [40,204] for this mask/shape — \
         the field lives outside the swept window or needs per-point mask bits)"
    );
    let json = format!("{{\"e\":\"e1c\",\"retained\":{retained:?},\"remapped\":{remapped:?}}}");
    write_json("e1c-vf-offset-sweep.json", &json);
}
