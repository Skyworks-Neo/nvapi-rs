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
// E7 raw_delta_forensics — offset_of-proven delta geometry (88+36i), full
//                         nonzero census, all-36-frame scan, optional
//                         self-restoring write arms → closure for audit ①/⑥
//
// Results also land as JSON under ../reverse/xocd/ for machine diffing.
//
// GEOMETRY ERRATUM (2026-10-05): e1/e1b printed an "nvapioc" column at
// 60+36i assuming a 40-byte header; the real nvstruct is 4 (version) + 32
// (ClockMask<8>) + 32 (unknown) → points@68, entry stride 36,
// freqDeltaKHz@entry+20 ⇒ delta(i) = 88+36i. That is exactly the family
// e1c saw retained ([88,124,160,196]) and identical to xOCD's
// 124+36*(i-1) for i>=1 — the two "competing" geometries were the same
// offsets. e7 proves the layout at runtime with offset_of! and re-parses
// every candidate family in one pass.

#![allow(unused_must_use)]

use core::ptr;
use nvapi::Kilohertz2Delta;
use nvapi::PhysicalGpu;
use nvapi::sys::api::{
    NvAPI_GPU_ClientPowerPoliciesGetInfoPrivate, NvAPI_GPU_ClientTgpWattGetStatus,
    NvAPI_GPU_ClockClientClkVfPointsGetControl, NvAPI_GPU_ClockClkDomainsGetControl,
    NvAPI_GPU_ClockClkPropTopRelsGetControl, NvAPI_GPU_ClockClkPropTopRelsGetInfo,
};
use nvapi::sys::gpu::clock::undocumented::{
    NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL2, NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_CONTROL,
    NV_GPU_CLOCK_CLIENT_CLK_PROP_TOP_RELS_INFO, NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_CONTROL_V1,
    NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL, clk_top_rels_control, clk_top_rels_info,
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

/// E7 — raw delta forensics on the PROVEN struct geometry. Read-only
/// phase always available; the write arms need `#[ignore]` + the env gate.
///
/// PHASE A (GET-only):
///   A0  runtime offset proof from the real nvstruct (`offset_of!`) —
///       takes every manual offset arithmetic out of the loop;
///   A1  full 9248B raw table hex + head;
///   A2  nonzero census: every value present in the table with the full
///       list of absolute offsets that carry it (36-stride RLE) — this
///       catches EVERY slot any previous experiment poked, whatever the
///       family (60+36i, 68+36i, 88+36i, 124+36i, …);
///   A3  all-36-frames scan (f=0..35): nonzero runs per frame, so no
///       candidate family can hide behind a wrong label;
///   A4  named-family column table for points 0..7.
///
/// PHASE B (`NVOC_ALLOW_VF_WRITE_PROBE=1`, both arms self-restoring with a
/// byte-identical verify before continuing):
///   B1  API arm — `set_vfp_table(point 5, +80 MHz)`, the exact path the
///       CLI's set-public-vftable writes use; raw GET diff afterwards
///       prints every changed absolute offset;
///   B2  raw arm — patch the proven delta slot (88+36*5) to 55555 directly,
///       raw SET, retention check, restore.
///
/// Run (read-only):
///   cargo test -p nvapi --test xocd_gap_probe_live e7 -- --ignored --nocapture --test-threads=1
/// Run (with the write arms):
///   NVOC_ALLOW_VF_WRITE_PROBE=1 cargo test -p nvapi --test xocd_gap_probe_live e7 -- --ignored --nocapture --test-threads=1
/// Artifacts: reverse/xocd/e7-report.txt, e7-raw-table.hex, e7-report.json
#[test]
#[ignore]
fn e7_raw_delta_forensics() {
    use core::mem::{offset_of, size_of};
    use std::collections::BTreeMap;

    let mut report = String::new();
    macro_rules! say {
        ($($t:tt)*) => {{
            let s = format!($($t)*);
            eprintln!("{s}");
            report.push_str(&s);
            report.push('\n');
        }};
    }

    // ---- A0: runtime struct proof --------------------------------------
    let ctl_size = size_of::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL>();
    let pts_off = offset_of!(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL, points);
    let entry_stride = size_of::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_CONTROL_V1>();
    let delta_off = offset_of!(NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_CONTROL_V1, freqDeltaKHz);
    let delta_base = pts_off + delta_off;
    say!("===== E7 raw delta forensics =====");
    say!(
        "A0 struct proof: size={ctl_size} points@{pts_off} entry_stride={entry_stride} \
         freqDeltaKHz@entry+{delta_off} → delta(i) = {delta_base}+{entry_stride}*i"
    );
    assert_eq!(
        (ctl_size, pts_off, entry_stride, delta_off),
        (9248, 68, 36, 20),
        "struct geometry drifted from 4+32+32+255*36 — audit the nvstruct before trusting offsets"
    );

    let gpu = first_gpu();
    let info = gpu.vfp_info().expect("vfp_info");
    say!("vfp info mask[0]: {:#010x}", info.mask.mask.mask[0]);

    let get_raw = |gpu: &PhysicalGpu| -> (i32, Vec<u8>) {
        let mut t = NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL {
            mask: info.mask.mask,
            ..Default::default()
        };
        let st = unsafe {
            NvAPI_GPU_ClockClientClkVfPointsGetControl(*gpu.handle(), ptr::from_mut(&mut t).cast())
        };
        let b =
            unsafe { core::slice::from_raw_parts(ptr::from_ref(&t).cast::<u8>(), size_of_val(&t)) }
                .to_vec();
        (st, b)
    };
    let set_raw = |gpu: &PhysicalGpu, b: &[u8]| -> i32 {
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
    let rd = |b: &[u8], abs: usize| -> i32 {
        u32::from_le_bytes(b[abs..abs + 4].try_into().unwrap()) as i32
    };

    let (st, base) = get_raw(&gpu);
    say!("A1 raw GET: status={st:?} bytes={}", base.len());
    assert_eq!(st, 0, "GET rejected");
    say!("A1 head[160]: {}", hex_head(&base, 160));

    // ---- A2: nonzero census (value → all absolute offsets) -------------
    let mut census: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
    for o in (0..base.len().saturating_sub(3)).step_by(4) {
        let v = rd(&base, o);
        if v != 0 {
            census.entry(v).or_default().push(o);
        }
    }
    say!("A2 nonzero census ({} distinct values):", census.len());
    for (v, offs) in &census {
        say!("  {v}: n={} {}", offs.len(), rle36(offs));
    }

    // ---- A3: all-36-frames scan ----------------------------------------
    say!("A3 all frames f=0..35 (abs f+36k) — nonzero RLE, zeros elided:");
    for f in 0..36usize {
        let mut vals: Vec<i32> = Vec::new();
        let mut o = f;
        while o + 4 <= base.len() {
            vals.push(rd(&base, o));
            o += 36;
        }
        let mut desc: Vec<String> = Vec::new();
        let mut k = 0usize;
        while k < vals.len() {
            let v = vals[k];
            let s = k;
            let mut e = k;
            while e + 1 < vals.len() && vals[e + 1] == v {
                e += 1;
            }
            if v != 0 {
                desc.push(if s == e {
                    format!("k{s}={v}")
                } else {
                    format!("k{s}..{e}={v}")
                });
            }
            k = e + 1;
        }
        if desc.is_empty() {
            say!("  f={f:>2}: all zero ({} dwords)", vals.len());
        } else {
            say!("  f={f:>2} (abs {f}+36k): {}", desc.join(", "));
        }
    }

    // ---- A4: named-family columns --------------------------------------
    say!("A4 named families for points 0..7:");
    say!("   pt  ours(delta@88+36i)  old60(60+36i)  type(68+36i)  xocd(124+36i)");
    for i in 0..8usize {
        say!(
            "  {i:>3} {:>18} {:>14} {:>13} {:>15}",
            rd(&base, delta_base + 36 * i),
            rd(&base, 60 + 36 * i),
            rd(&base, 68 + 36 * i),
            rd(&base, 124 + 36 * i)
        );
    }

    // ---- PHASE B (mutating, opt-in) ------------------------------------
    let di = delta_base + 36 * 5; // delta slot of point 5
    let mut json = String::new();
    if std::env::var("NVOC_ALLOW_VF_WRITE_PROBE").as_deref() == Ok("1") {
        say!(
            "B0 target point 5: delta abs {di} currently {}",
            rd(&base, di)
        );

        // B1 — API arm (the exact CLI write path)
        let st = gpu.set_vfp_table(
            &info,
            core::iter::once((5usize, Kilohertz2Delta(160_000))),
            core::iter::empty::<(usize, Kilohertz2Delta)>(),
        );
        say!("B1 set_vfp_table(p5, Kilohertz2Delta(160000)) → {st:?}");
        match &st {
            Ok(()) => {
                let (st2, after) = get_raw(&gpu);
                assert_eq!(st2, 0, "B1 readback GET rejected");
                let d = diff4(&base, &after);
                say!("B1 raw GET diff ({} dwords): {}", d.len(), fmt_diff(&d));
                say!(
                    "B1 delta slot {di}: {} (before {})",
                    rd(&after, di),
                    rd(&base, di)
                );
                json = format!(
                    ",\"b1_changed\":[{}],\"b1_slot_after\":{}",
                    d.iter()
                        .map(|(o, _, b)| format!("[{o},{b}]"))
                        .collect::<Vec<_>>()
                        .join(","),
                    rd(&after, di)
                );
            }
            Err(e) => {
                say!("B1 SET rejected ({e:?}) — table unchanged; skipping restore");
                say!("   (-137 InvalidUserPrivilege just means: run elevated)");
            }
        }
        if st.is_ok() {
            assert_eq!(set_raw(&gpu, &base), 0, "B1 restore SET failed");
            let (_, back) = get_raw(&gpu);
            let same = back == base;
            say!(
                "B1 restore verify: {}",
                if same {
                    "OK (byte-identical)"
                } else {
                    "MISMATCH — inspect diff before continuing!"
                }
            );
            assert!(same, "B1 restore mismatch");
        }

        // B2 — raw arm at the proven delta slot
        let mut m = base.clone();
        m[di..di + 4].copy_from_slice(&55_555u32.to_le_bytes());
        let st = set_raw(&gpu, &m);
        say!("B2 raw patch {di}=55555 → SET {st:?}");
        if st == 0 {
            let (_, after) = get_raw(&gpu);
            say!("B2 retention at {di}: {}", rd(&after, di));
            let d = diff4(&base, &after);
            say!("B2 raw GET diff ({} dwords): {}", d.len(), fmt_diff(&d));

            assert_eq!(set_raw(&gpu, &base), 0, "B2 restore SET failed");
            let (_, back) = get_raw(&gpu);
            let same = back == base;
            say!(
                "B2 restore verify: {}",
                if same {
                    "OK (byte-identical)"
                } else {
                    "MISMATCH!"
                }
            );
            assert!(same, "B2 restore mismatch");
        } else {
            say!("B2 SET rejected ({st}) — table unchanged; skipping restore");
        }
    } else {
        say!("phase B skipped: mutating probe — set NVOC_ALLOW_VF_WRITE_PROBE=1 to run");
    }

    // ---- artifacts ------------------------------------------------------
    let hex: String = base.iter().map(|b| format!("{b:02x}")).collect();
    write_json("e7-raw-table.hex", &hex);
    write_json("e7-report.txt", &report);
    let j = format!(
        "{{\"e\":\"e7\",\"size\":{ctl_size},\"points_off\":{pts_off},\"stride\":{entry_stride},\
         \"delta_off\":{delta_off},\"delta_base\":{delta_base},\"census\":{{{}}}{}}}",
        census
            .iter()
            .map(|(v, o)| format!(
                "{v}:[{}]",
                o.iter().map(usize::to_string).collect::<Vec<_>>().join(",")
            ))
            .collect::<Vec<_>>()
            .join(","),
        json
    );
    write_json("e7-report.json", &j);
}

/// RLE a sorted offset list along a 36-byte stride: runs of >=3 become
/// `start+36k k=0..n`, shorter groups are listed literally. Capped.
fn rle36(offs: &[usize]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < offs.len() {
        let start = offs[i];
        let mut n = 1;
        while i + n < offs.len() && offs[i + n] == start + 36 * n {
            n += 1;
        }
        if n >= 3 {
            out.push(format!("{start}+36k k=0..{}", n - 1));
        } else {
            for o in &offs[i..i + n] {
                out.push(o.to_string());
            }
        }
        i += n;
    }
    if out.len() > 24 {
        let extra = out.len() - 24;
        out.truncate(24);
        out.push(format!("…+{extra} more"));
    }
    format!("[{}]", out.join(", "))
}

/// Changed dwords between two byte images (absolute offset, before, after).
fn diff4(before: &[u8], after: &[u8]) -> Vec<(usize, i32, i32)> {
    let mut v = Vec::new();
    let n = before.len().min(after.len());
    let mut o = 0;
    while o + 4 <= n {
        let a = u32::from_le_bytes(before[o..o + 4].try_into().unwrap()) as i32;
        let b = u32::from_le_bytes(after[o..o + 4].try_into().unwrap()) as i32;
        if a != b {
            v.push((o, a, b));
        }
        o += 4;
    }
    v
}

fn fmt_diff(d: &[(usize, i32, i32)]) -> String {
    if d.is_empty() {
        return "(none — the write did not change the returned table)".to_string();
    }
    let mut s: Vec<String> = d
        .iter()
        .take(40)
        .map(|(o, a, b)| format!("abs {o}: {a}→{b}"))
        .collect();
    if d.len() > 40 {
        s.push(format!("…+{} more", d.len() - 40));
    }
    s.join(", ")
}
