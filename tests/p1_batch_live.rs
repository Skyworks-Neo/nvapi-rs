//! Live dump for the nvClocks.spec P1 batch (2026-09-06 audit) — READ-ONLY.
//! Prints raw nonzero dwords + decoded interpretation for every surface so
//! field semantics can be confirmed before deciding what gets surfaced.
//!
//! Run with:
//!   cargo test -p nvapi --test p1_batch_live -- --nocapture --ignored
//!
//! Optional: NVOC_GPU_IDX=<n> selects the GPU (default 0).
#![allow(unused_must_use)]

use nvapi::PhysicalGpu;
use nvapi::sys::gpu::clock::undocumented::{
    NV_GPU_CLOCK_CLK_PROP_REGIMES_INFO, NV_GPU_PUBLIC_CLOCK_INFO,
};
use nvapi::sys::nvapi::StructVersion;

fn nonzero_dwords(tag: &str, bytes: &[u8], limit: usize) {
    // bytes = struct tail starting at struct offset 4; absolute struct
    // offset of bytes[k] is k + 4.
    let mut shown = 0;
    for k in 0..bytes.len() / 4 {
        let v = u32::from_le_bytes(bytes[k * 4..k * 4 + 4].try_into().unwrap());
        if v != 0 {
            println!("    {tag} +{:#x}: {v:#010x} ({v})", (k + 1) * 4);
            shown += 1;
            if shown >= limit {
                println!("    {tag} ... (more nonzero truncated)");
                break;
            }
        }
    }
    if shown == 0 {
        println!("    {tag}: (all zero)");
    }
}

#[test]
#[ignore]
fn p1_batch_live_dump() {
    nvapi::initialize().expect("NvAPI init failed");
    let gpus = PhysicalGpu::enumerate().expect("enumerate failed");
    let idx: usize = std::env::var("NVOC_GPU_IDX")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let gpu = gpus.get(idx).expect("gpu index out of range");
    println!("=== GPU {idx}: {} ===", gpu.full_name().unwrap_or_default());

    // ---------------- ADC (seed source + ★P1 live status) ----------------
    println!("\n########## 0x68789E2A ClockAdcDevicesGetInfo (seed source) ##########");
    let adc_mask = match gpu.adc_devices_info() {
        Ok(None) => {
            println!("  no ADC devices exposed (mask 0)");
            0
        }
        Ok(Some(info)) => {
            println!("  mask: {:#010x}", info.mask);
            for d in &info.entries {
                let name = d
                    .name
                    .iter()
                    .take_while(|&&b| b != 0)
                    .cloned()
                    .collect::<Vec<_>>();
                println!(
                    "  dev[{}] type={} chan={:#x} name={:?}",
                    d.bit,
                    d.device_type,
                    d.channel,
                    String::from_utf8_lossy(&name)
                );
            }
            info.mask
        }
        Err(e) => {
            println!("  ERROR: {e}");
            0
        }
    };

    println!("\n########## 0x43D9B26A ClockAdcDevicesGetStatus (★P1 live) ##########");
    if adc_mask == 0 {
        println!("  skipped (no ADC mask)");
    } else {
        for sweep in 0..3 {
            match gpu.adc_devices_status() {
                Ok(None) => println!("  sweep{sweep}: none"),
                Ok(Some(st)) => {
                    println!("  sweep{sweep} mask: {:#010x}", st.mask);
                    for e in &st.entries {
                        println!(
                            "  ch[{}] state={:+} value_uv={} ({} mV) vfp_id={}/{} /{} fmt={} value2={:#010x}",
                            e.bit,
                            e.state,
                            e.value_uv,
                            e.value_uv / 1000,
                            e.vf_point_id_a,
                            e.reserved,
                            e.vf_point_id_b,
                            e.value_format,
                            e.value2
                        );
                    }
                }
                Err(e) => println!("  sweep{sweep} ERROR: {e}"),
            }
            if sweep != 2 {
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
        println!(
            "  (value_uv/vfp_id moving across sweeps = live telemetry; vfp_id = VF-table voltage-point index)"
        );
    }

    // ---------------- ClkPropRegimes (P1 pair) ----------------
    println!("\n########## 0xCF08E934 ClockClkPropRegimesGetInfo (P1) ##########");
    let reg_mask = match gpu.clk_prop_regimes_info() {
        Ok(None) => {
            println!("  empty regime mask");
            0
        }
        Ok(Some(info)) => {
            println!(
                "  availability@+4: {} mask: {:#010x}",
                info.availability, info.mask
            );
            for e in &info.entries {
                println!(
                    "  regime[{}] status={:+} type={:#x} value={:#010x} ({})",
                    e.bit, e.status, e.regime_type, e.value, e.value
                );
            }
            info.mask
        }
        Err(e) => {
            println!("  ERROR: {e}");
            0
        }
    };

    println!("\n########## 0x4F11EAA4 ClockClkPropRegimesGetControl (P1) ##########");
    if reg_mask == 0 {
        println!("  skipped (no regime mask)");
    } else {
        match gpu.clk_prop_regimes_control() {
            Ok(None) => println!("  none"),
            Ok(Some(ctl)) => {
                println!("  mask: {:#010x}", ctl.mask);
                for e in &ctl.entries {
                    println!(
                        "  regime[{}] status={:+} value={:#010x} ({})",
                        e.bit, e.status, e.value, e.value
                    );
                }
            }
            Err(e) => println!("  ERROR: {e}"),
        }
    }

    // ---------------- ClkDomainFreqsEnum (P1, MHz) ----------------
    println!("\n########## 0x40BDDDB36 ClockClkDomainFreqsEnum (P1, MHz) ##########");
    for sel in 0u8..8 {
        match gpu.clk_domain_freqs_enum(sel) {
            Ok(fe) => {
                let f = &fe.freqs_mhz;
                let preview = if f.len() > 12 {
                    format!("{:?} ... {:?}", &f[..8], &f[f.len() - 4..])
                } else {
                    format!("{f:?}")
                };
                println!(
                    "  sel={sel}: {} pts  min={:?} max={:?}  {preview}",
                    f.len(),
                    f.first(),
                    f.last()
                );
            }
            Err(e) => println!("  sel={sel}: ERROR {e}"),
        }
    }

    // ---------------- PublicClockInfo (P1) ----------------
    println!("\n########## 0x1B46D4CC GetPublicClockInfo (P1) ##########");
    match gpu.public_clock_info() {
        Ok(pc) => {
            println!("  count@+4: {}", pc.count);
            let zones: [(&str, usize); 4] = [
                ("type1", 8),
                ("type4", 0x38),
                ("type2", 0x5C),
                ("type8", 0x68),
            ];
            for (name, off) in zones {
                let k = (off - 8) / 12;
                if let Some(s) = pc.slots.get(k) {
                    println!(
                        "  {name} zone @+{off:#x} slot[{k}]: value={} flag={} max={}",
                        s.value, s.flag, s.max
                    );
                }
            }
            let non_default: Vec<_> = pc
                .slots
                .iter()
                .filter(|s| s.value != 32 || s.flag != 0 || s.max != 100)
                .collect();
            println!("  non-default slots: {}", non_default.len());
            for s in non_default {
                println!(
                    "  slot[{}] @+{:#x}: value={} flag={} max={}",
                    s.index,
                    8 + s.index * 12,
                    s.value,
                    s.flag,
                    s.max
                );
            }
        }
        Err(e) => println!("  ERROR: {e}"),
    }

    // ---------------- LockedClockModeStatus (P1) ----------------
    println!("\n########## 0xC4733F19 GetLockedClockModeStatus (P1) ##########");
    match gpu.locked_clock_mode_status() {
        Ok(lm) => println!(
            "  mode_mask: {:#010x} (bits on: {:?})",
            lm.mode_mask,
            (0..4)
                .filter(|b| lm.mode_mask & (1 << b) != 0)
                .collect::<Vec<_>>()
        ),
        Err(e) => println!("  ERROR: {e}"),
    }

    println!("\n(done — no SET call was made; every surface above is read-only)");
}

/// Raw sys-level dump companion: re-reads the two structurally-opaque
/// surfaces (Regimes info, PublicClockInfo) straight through the FFI and
/// prints every nonzero dword so unknown header fields stay visible.
#[test]
#[ignore]
fn p1_raw_header_dump() {
    use nvapi::sys::api;

    nvapi::initialize().expect("NvAPI init failed");
    let gpus = PhysicalGpu::enumerate().expect("enumerate failed");
    let gpu = gpus.first().expect("no gpu");
    let h = *gpu.handle();

    println!("=== raw header dump: RegimesInfo 0x10A8C ===");
    let mut info = NV_GPU_CLOCK_CLK_PROP_REGIMES_INFO {
        version: <NV_GPU_CLOCK_CLK_PROP_REGIMES_INFO as StructVersion>::NVAPI_VERSION,
        ..Default::default()
    };
    let st = unsafe { api::NvAPI_GPU_ClockClkPropRegimesGetInfo(h, &mut info as *mut _ as *mut _) };
    println!("  status: {st:?}");
    nonzero_dwords("regimes_info", &info.rest, 40);

    println!("=== raw header dump: PublicClockInfo 0x10188 (preset 32/0/100) ===");
    let mut pc = NV_GPU_PUBLIC_CLOCK_INFO {
        version: <NV_GPU_PUBLIC_CLOCK_INFO as StructVersion>::NVAPI_VERSION,
        ..Default::default()
    };
    pc.preset_defaults();
    let st = unsafe { api::NvAPI_GPU_GetPublicClockInfo(h, &mut pc as *mut _ as *mut _) };
    println!("  status: {st:?}");
    for k in 0..32usize {
        let o = 4 + k * 12;
        let trip = (
            u32::from_le_bytes(pc.rest[o..o + 4].try_into().unwrap()),
            u32::from_le_bytes(pc.rest[o + 4..o + 8].try_into().unwrap()),
            u32::from_le_bytes(pc.rest[o + 8..o + 12].try_into().unwrap()),
        );
        if trip != (32, 0, 100) {
            println!("    slot[{k}] @+{:#x}: {trip:?}", o + 4);
        }
    }
}
