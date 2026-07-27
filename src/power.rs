//! PowerMonitor v4 — per-channel / per-rail power descriptor + live readings.
//!
//! `NvAPI_GPU_PowerMonitorGetInfo` (0xC12EB19E) / `GetStatus` (0xF40238EF).
//! Pre-wrap / research: the deployed driver's **v4 GetInfo** layout (6312 B)
//! carries a per-channel descriptor table (channel_type + PowerRail identity +
//! Q12 scaling fields), and GetStatus returns live per-channel values. Both
//! were long thought unsupported — that was a probe bug (feeding GetStatus's
//! accepted magics to GetInfo). With the correct per-IID magics both return Ok.
//!
//! **STATUS: pre-wrap, units UNCONFIRMED.** Under load the channel-0 value
//! tracks NVML board power at ~0.95×, but at idle the ratio collapses to
//! ~0.15–0.54, so the raw values cannot yet be converted to W/A reliably —
//! the per-channel type-specific scaling (slope/offset from the descriptors)
//! must be cross-validated against GPU-Z first. Do NOT surface decoded power
//! to the UI yet; the values here are deliberately raw.

use crate::sys::gpu::power::private::NV_GPU_POWER_MONITOR_GET_INFO_V4;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

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

/// Decode the variable-stride per-channel descriptor table from a GetInfo v4
/// buffer by signature scan: a descriptor is recognized by `(channel_type in
/// 1..=8)` immediately followed by a plausible `pwr_rail`. Record length varies
/// with channel_type (type 5/7 carry VF-estimation LUT tables), so this scans
/// rather than stepping a fixed stride — mirroring the proven probe logic in
/// `core/tests/gpu_readonly.rs::nvapi_power_monitor_raw`.
///
/// `status_bytes` is the raw GetStatus v1|392 buffer (the live per-channel
/// values). Status values are best-effort positionally matched to descriptors;
/// matching is intentionally loose because the status record stride is also
/// irregular and not yet fully decoded.
pub fn power_monitor_from_raw(info: &NV_GPU_POWER_MONITOR_GET_INFO_V4, status_bytes: &[u8]) -> PowerMonitor {
    let desc = info.descriptors_bytes();
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
