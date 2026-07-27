//! PowerMonitor v4 — per-channel / per-rail power descriptor + live readings.
//!
//! `NvAPI_GPU_PowerMonitorGetInfo` (0xC12EB19E) / `GetStatus` (0xF40238EF).
//! The deployed driver's **v4 GetInfo** layout (6312 B) carries a per-channel
//! descriptor table (channel_type + PowerRail identity + Q12 scaling fields),
//! and GetStatus returns live per-channel values in **milliwatts** (units
//! confirmed by exact GPU-Z match: raw ÷ 1000 = W). Both were long thought
//! unsupported — that was a probe bug (feeding GetStatus's accepted magics to
//! GetInfo). With the correct per-IID magics both return Ok.
//!
//! Two surfaces:
//! - [`PowerRails`] — the 4 GPU-Z-confirmed named rails (Board/Chip/MVDDC/
//!   PWR_SRC) extracted from GetStatus by known offsets. Safe to display.
//! - [`PowerMonitor`] — the full decoded descriptor table (channel identity +
//!   scaling), research-grade; channel-0 carries a live status at +0x44.

use crate::sys::gpu::power::private::{
    NV_GPU_POWER_MONITOR_GET_INFO_V1_2728, NV_GPU_POWER_MONITOR_GET_INFO_V3_3240,
    NV_GPU_POWER_MONITOR_GET_INFO_V4,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Version-independent view of a PowerMonitor GetInfo result: the channel
/// mask + the raw descriptor bytes (owned, so it outlives the source struct).
/// The v1|2728 / v3|3240 / v4|6312 layouts share an identical header +
/// descriptor-offset format, so a descriptor reader only needs these two
/// fields regardless of which version the driver accepted.
pub struct PowerMonitorInfo {
    pub channel_mask: u32,
    pub descriptors: Vec<u8>,
}

macro_rules! impl_powermonitorinfo {
    ($ty:ty) => {
        impl From<&$ty> for PowerMonitorInfo {
            fn from(info: &$ty) -> Self {
                PowerMonitorInfo {
                    channel_mask: info.channel_mask,
                    descriptors: info.descriptors_bytes().to_vec(),
                }
            }
        }
    };
}
impl_powermonitorinfo!(NV_GPU_POWER_MONITOR_GET_INFO_V1_2728);
impl_powermonitorinfo!(NV_GPU_POWER_MONITOR_GET_INFO_V3_3240);
impl_powermonitorinfo!(NV_GPU_POWER_MONITOR_GET_INFO_V4);

/// A per-channel live reading from PowerMonitor GetStatus. **Raw values,
/// units unconfirmed** — fields are the observed slots (avg/min/max power,
/// current, voltage, energy) but their physical units are not yet validated.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PowerMonitorStatus {
    pub pwr_avg: u32,
    pub pwr_min: u32,
    pub pwr_max: u32,
    pub curr: u32,
    pub volt: u32,
    pub energy: u64,
}

/// One decoded PowerMonitor channel: its descriptor identity (type, rail,
/// scaling) from GetInfo v4, plus a best-effort matched live status from
/// GetStatus. `byte_offset` is the descriptor's position in the raw v4 buffer,
/// retained so the typed view correlates with the byte-dump probe.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PowerMonitorChannel {
    /// Byte offset of this channel's descriptor in the v4 GetInfo buffer.
    pub byte_offset: usize,
    /// `channel_type` (RTSS `NV_GPU_POWER_MONITOR_POWER_CHANNEL_TYPE`):
    /// 1=Summation, 2=Estimation, 3=Slow, 4=GeminiCorrection, 5=OneX,
    /// 6=Sensor, 7=PstateEstimationLut, 8=SensorClientAligned.
    pub channel_type: u32,
    /// `pwr_rail` (RTSS `NV_GPU_POWER_CHANNEL_POWER_RAIL`). May exceed the
    /// named enum range — see `PowerRail` gaps.
    pub pwr_rail: u32,
    /// Fixed voltage for the rail in µV (e.g. 3_300_000 for PEX 3.3 V), when
    /// the descriptor carries it.
    pub volt_fixed_uv: u32,
    /// Power-correction slope (Q12 fixed-point; observed 4096 = 1.0).
    pub pwr_corr_slope: u32,
    /// Best-effort matched live reading (None if GetStatus didn't populate it).
    pub raw_status: Option<PowerMonitorStatus>,
}

/// All decoded PowerMonitor channels from a GetInfo v4 + GetStatus pair.
/// A `Vec` (not a map) because the v4 descriptor body is variable-stride and
/// sparsely packed — channel index is a decoded field, not the position.
pub type PowerMonitor = Vec<PowerMonitorChannel>;

/// Plausible PowerRail value: the named outputs (0..=11), the named inputs
/// (218..=255), or 0. Used by the descriptor signature scan to avoid matching
/// unrelated adjacent words.
fn plausible_rail(r: u32) -> bool {
    r == 0 || r <= 11 || (218..=255).contains(&r)
}

/// Decode the variable-stride per-channel descriptor table from a GetInfo
/// buffer by signature scan: a descriptor is recognized by `(channel_type in
/// 1..=8)` immediately followed by a plausible `pwr_rail`. Record length varies
/// with channel_type (type 5/7 carry VF-estimation LUT tables), so this scans
/// rather than stepping a fixed stride — mirroring the proven probe logic in
/// `core/tests/gpu_readonly.rs::nvapi_power_monitor_raw`. Works identically
/// across the v1|2728 / v3|3240 / v4|6312 layouts (same header + descriptor
/// offsets); pass a [`PowerMonitorInfo`] built from whichever version the
/// driver accepted.
///
/// `status_bytes` is the raw GetStatus v1|392 buffer (the live per-channel
/// values). Status values are best-effort positionally matched to descriptors;
/// matching is intentionally loose because the status record stride is also
/// irregular and not yet fully decoded.
pub fn power_monitor_from_raw(info: &PowerMonitorInfo, status_bytes: &[u8]) -> PowerMonitor {
    let desc = info.descriptors.as_slice();
    let desc_words = desc.len() / 4;
    let w = |i: usize| -> u32 {
        if i < desc_words {
            u32::from_le_bytes(desc[i * 4..i * 4 + 4].try_into().unwrap_or([0; 4]))
        } else {
            0
        }
    };
    // Read a u32 from the STATUS buffer at a byte offset.
    let s = |off: usize| -> u32 {
        if off + 4 <= status_bytes.len() {
            u32::from_le_bytes(status_bytes[off..off + 4].try_into().unwrap_or([0; 4]))
        } else {
            0
        }
    };

    // NOTE on GetStatus matching: the GetStatus v1|392 buffer does NOT carry a
    // per-record channel_type signature like GetInfo does, so its per-channel
    // record stride cannot be recovered by the same signature scan. Only
    // channel 0's value is at a known, confirmed offset (+0x44 — the total GPU
    // power channel). The other channels' offsets are irregular and not yet
    // decoded; decoding them requires correlating the GetStatus nonzero slots
    // (+0x2C/+0x44/+0x80/+0x98/+0xE0/+0xEC/+0x14C observed) to channels under
    // controlled per-rail load. Until then only ch0 gets a live status; the
    // raw byte-dump probe (nvapi_power_monitor_raw) remains the source of truth
    // for all-channel status bytes.
    let mut channels = Vec::new();
    let mut i = 0usize; // word index into the descriptor region
    let mut desc_idx = 0usize;
    while i + 1 < desc_words {
        let ctype = w(i);
        let rail = w(i + 1);
        if (1..=8).contains(&ctype) && plausible_rail(rail) {
            // Descriptor header (offsets relative to the descriptor base):
            //   +0x00 pwr_device_mask, +0x04 channel_type, +0x08 pwr_rail,
            //   +0x0C volt_fixed_uv, +0x10 pwr_corr_slope, +0x14 curr_corr_slope.
            let base = i * 4;
            let volt_fixed = w(i + 2);
            let slope = w(i + 3);
            // Only channel 0's live status is at a confirmed GetStatus offset.
            let raw_status = (desc_idx == 0).then(|| PowerMonitorStatus {
                pwr_avg: s(0x44),
                ..Default::default()
            });
            desc_idx += 1;
            channels.push(PowerMonitorChannel {
                byte_offset: base,
                channel_type: ctype,
                pwr_rail: rail,
                volt_fixed_uv: volt_fixed,
                pwr_corr_slope: slope,
                raw_status,
            });
            i += 8; // skip past this descriptor's header to avoid re-matching
        } else {
            i += 1;
        }
    }
    channels
}

/// One rail's live power reading, labeled by its **descriptor identity**
/// (not by a hardcoded GetStatus offset). The rail identity comes from the
/// GetInfo descriptor table; the power value comes from a per-channel GetStatus
/// call. Units: milliwatts (confirmed by exact GPU-Z match: raw ÷ 1000 = W).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PowerRailReading {
    /// Bit index of this channel in GetInfo's `channel_mask` (0..32).
    pub channel_bit: u32,
    /// `pwr_rail` from the GetInfo descriptor
    /// (RTSS `NV_GPU_POWER_CHANNEL_POWER_RAIL`). May be an unnamed value
    /// (e.g. 218 on some Ada SKUs).
    pub pwr_rail: u32,
    /// Human-readable rail name from [`power_rail_name_owned`] (e.g.
    /// "InputTotalBoard", "InputPex12v1", "UNNAMED_218"). Pre-rendered so
    /// downstream consumers (CLI/pynvoc/TUI) need no nvapi-rs import to label.
    pub rail_name: String,
    /// `channel_type` from the GetInfo descriptor
    /// (1=Summation, 2=Estimation, 3=Slow, 4=GeminiCorrection, 5=OneX,
    /// 6=Sensor, 7=PstateEstimationLut, 8=SensorClientAligned).
    pub channel_type: u32,
    /// Live power draw on this rail, in milliwatts. 0 when GetStatus didn't
    /// populate the channel (rail present but currently unreadable).
    pub pwr_mw: u32,
}

impl PowerRailReading {
    /// Power in watts, or `None` if the rail reported 0.
    pub fn watts(&self) -> Option<f32> {
        (self.pwr_mw != 0).then(|| self.pwr_mw as f32 / 1000.0)
    }
}

/// All rail readings from a PowerMonitor GetInfo + per-bit GetStatus pass.
/// Keyed/identified by the descriptor's `pwr_rail` (via [`power_rail_name`]),
/// NOT by a fixed offset — so it's correct on every GPU regardless of how the
/// driver orders channels (the RTX 4060 Laptop and a desktop Turing expose
/// different rail sets and orderings; both decode correctly here).
pub type PowerRails = Vec<PowerRailReading>;

/// Build the `channel_bit -> (pwr_rail, channel_type)` map from a GetInfo
/// descriptor table. The descriptor records are found by signature scan
/// (channel_type 1..=8 + plausible PowerRail); the Nth descriptor found maps
/// to the Nth set bit of `channel_mask` (the channels are enumerated in
/// ascending bit order by the driver).
pub fn descriptor_rail_map(info: &PowerMonitorInfo) -> Vec<(u32, u32, u32)> {
    // Ordered set bits of the channel mask — descriptor N corresponds to bit N.
    let bits: Vec<u32> = (0..32).filter(|i| info.channel_mask & (1 << i) != 0).collect();
    let desc = info.descriptors.as_slice();
    let desc_words = desc.len() / 4;
    let w = |i: usize| -> u32 {
        if i < desc_words {
            u32::from_le_bytes(desc[i * 4..i * 4 + 4].try_into().unwrap_or([0; 4]))
        } else {
            0
        }
    };
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 1 < desc_words && out.len() < bits.len() {
        let ctype = w(i);
        let rail = w(i + 1);
        if (1..=8).contains(&ctype) && plausible_rail(rail) {
            let bit = bits[out.len()];
            out.push((bit, rail, ctype));
            i += 8; // skip past this descriptor's header
        } else {
            i += 1;
        }
    }
    out
}

/// Nonzero `(byte_offset, value)` u32 pairs in a GetStatus buffer.
pub fn nonzero_offsets(status_bytes: &[u8]) -> Vec<(usize, u32)> {
    let n = status_bytes.len() / 4;
    let mut out = Vec::new();
    for w in 0..n {
        let off = w * 4;
        if off + 4 > status_bytes.len() {
            break;
        }
        let v = u32::from_le_bytes(status_bytes[off..off + 4].try_into().unwrap_or([0; 4]));
        if v != 0 {
            out.push((off, v));
        }
    }
    out
}

/// Extract a single channel's power value (mW) from its per-bit GetStatus
/// nonzero-offset list, given the shared `baseline` offsets (those present in
/// every channel's buffer — header, accumulator, calibration offset, and
/// channel-0's slot).
///
/// The channel's OWN value is at an offset OUTSIDE the baseline. Take the
/// largest plausible such value (the rail power is the dominant reading; small
/// sub-fields and sentinels ≥ 10_000_000 like accumulators/version magics are
/// excluded). Channel 0 (total) has no non-baseline offset — its value IS the
/// +0x44 baseline slot — so fall back to the +0x44 value when present.
pub fn extract_channel_mw(
    nz: &[(usize, u32)],
    baseline: &std::collections::HashSet<usize>,
) -> u32 {
    let mut best: u32 = 0;
    for &(off, v) in nz {
        if baseline.contains(&off) {
            continue;
        }
        if v != 0 && v < 10_000_000 && v > best {
            best = v;
        }
    }
    if best == 0 {
        // Channel 0 (total): its value is the shared +0x44 slot.
        for &(off, v) in nz {
            if off == 0x44 {
                return v;
            }
        }
    }
    best
}

/// Name a `NV_GPU_POWER_CHANNEL_POWER_RAIL` value. Delegates to the
/// `PowerRail` enum (single source of truth for the RTSS rail names); unknown
/// values (not in the enum, e.g. Ada-private 218) return a rendered
/// `"UNNAMED_<n>"` string via the owned-return variant below.
pub fn power_rail_name(rail: u32) -> &'static str {
    use crate::sys::gpu::power::private::PowerRail;
    // Owned variants of the known enum values map to their Display name. We
    // can't return a borrow of PowerRail's Display (it's formatted, not
    // &'static), so use a static match derived from the enum's known values.
    // For values outside the enum, callers should use power_rail_name_owned.
    match PowerRail::from_raw(rail as i32) {
        Ok(r) => match r {
            PowerRail::Unknown => "Unknown",
            PowerRail::OutputNvvdd => "OutputNvvdd",
            PowerRail::OutputFbvdd => "OutputFbvdd",
            PowerRail::OutputFbvddq => "OutputFbvddq",
            PowerRail::OutputFbvddQ => "OutputFbvddQ",
            PowerRail::OutputPexvdd => "OutputPexvdd",
            PowerRail::OutputA3v3 => "OutputA3v3",
            PowerRail::Output3v3nv => "Output3v3nv",
            PowerRail::OutputTotalGpu => "OutputTotalGpu",
            PowerRail::OutputFbvddqGpu => "OutputFbvddqGpu",
            PowerRail::OutputFbvddqMem => "OutputFbvddqMem",
            PowerRail::OutputSram => "OutputSram",
            PowerRail::InputPex12v1 => "InputPex12v1",
            PowerRail::InputTotalBoard2 => "InputTotalBoard2",
            PowerRail::InputHighVolt0 => "InputHighVolt0",
            PowerRail::InputHighVolt1 => "InputHighVolt1",
            PowerRail::InputNvvdd1 => "InputNvvdd1",
            PowerRail::InputNvvdd2 => "InputNvvdd2",
            PowerRail::InputExt12v8pin2 => "InputExt12v8pin2",
            PowerRail::InputExt12v8pin3 => "InputExt12v8pin3",
            PowerRail::InputExt12v8pin4 => "InputExt12v8pin4",
            PowerRail::InputExt12v8pin5 => "InputExt12v8pin5",
            PowerRail::InputMisc0 => "InputMisc0",
            PowerRail::InputMisc1 => "InputMisc1",
            PowerRail::InputMisc2 => "InputMisc2",
            PowerRail::InputMisc3 => "InputMisc3",
            PowerRail::InputUsbc0 => "InputUsbc0",
            PowerRail::InputUsbc1 => "InputUsbc1",
            PowerRail::InputFan0 => "InputFan0",
            PowerRail::InputFan1 => "InputFan1",
            PowerRail::InputSram => "InputSram",
            PowerRail::InputPwrSrcPp => "InputPwrSrcPp",
            PowerRail::Input3v3Pp => "Input3v3Pp",
            PowerRail::Input3v3Main => "Input3v3Main",
            PowerRail::Input3v3Aon => "Input3v3Aon",
            PowerRail::InputTotalBoard => "InputTotalBoard",
            PowerRail::InputNvvdd => "InputNvvdd",
            PowerRail::InputFbvdd => "InputFbvdd",
            PowerRail::InputFbvddq => "InputFbvddq",
            PowerRail::InputFbvddQ => "InputFbvddQ",
            PowerRail::InputExt12v8pin0 => "InputExt12v8pin0",
            PowerRail::InputExt12v8pin1 => "InputExt12v8pin1",
            PowerRail::InputExt12v6pin0 => "InputExt12v6pin0",
            PowerRail::InputExt12v6pin1 => "InputExt12v6pin1",
            PowerRail::InputPex3v3 => "InputPex3v3",
            PowerRail::InputPex12v => "InputPex12v",
            // non_exhaustive / future variants — fall through to UNNAMED.
            _ => "UNNAMED",
        },
        Err(_) => "UNNAMED",
    }
}

/// Owned rail label for unknown values: `"UNNAMED_<n>"` when not in the enum.
pub fn power_rail_name_owned(rail: u32) -> String {
    use crate::sys::gpu::power::private::PowerRail;
    if PowerRail::from_raw(rail as i32).is_ok() {
        power_rail_name(rail).to_string()
    } else {
        format!("UNNAMED_{}", rail)
    }
}


