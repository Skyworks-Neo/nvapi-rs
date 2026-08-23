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

impl PowerMonitorChannel {
    /// Human-friendly name for this channel's rail via the three-layer
    /// merged resolver ([`rail_display_name`]): AmpereOC precise NVIDIA
    /// naming → RTSS full enum naming → GPU-Z market naming. Zero
    /// contradictions across sources (verified rail-by-rail).
    pub fn friendly_name(&self) -> Option<&'static str> {
        rail_display_name(self.pwr_rail)
    }
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
    /// How `pwr_mw` was obtained — see [`Confidence`]. Drives downstream
    /// rendering (Measured=plain, Inferred=`~`, Ambiguous=`?`, Unavailable
    /// =omitted). Replaces the earlier `isolated: bool`; callers that only
    /// need the trusted/untrusted split can use `confidence.is_trusted()`.
    pub confidence: Confidence,
    /// Secondary private GetStatus readings for this channel (offset, value),
    /// when the channel had >1 private offset (e.g. type=8 channels exposing
    /// both an instantaneous and an averaged/peak slot). The primary reading
    /// is in `pwr_mw` (lowest-offset private slot); these are the rest, kept
    /// so downstream can surface them rather than silently `max()`-ing them
    /// away. Empty for Inferred/Ambiguous channels (single attributed offset).
    pub aux_readings: Vec<(usize, u32)>,
    /// GPU-Z-equivalent friendly name for this rail (e.g. "Board", "Chip",
    /// "MVDDC", "PWR_SRC", "16-Pin"), from the semantic map
    /// [`gpu_z_rail_name`]. `None` when no GPU-Z equivalent is known (the
    /// rail is NVAPI-specific; use `rail_name` to label it). Owned `String`
    /// so the struct remains `Deserialize`-able (a `&'static str` would not
    /// survive deserialization from JSON).
    pub gpuz_name: Option<String>,
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
/// different rail sets and orderings; both decode correctly here). Each
/// reading carries a [`Confidence`] tier so callers can render trustworthy
/// (Measured) vs inferred (`~`) vs ambiguous (`?`) values distinctly.
pub type PowerRails = Vec<PowerRailReading>;

/// Build the `channel_bit -> (pwr_rail, channel_type)` map from a GetInfo
/// descriptor table. The descriptor records are found by signature scan
/// (channel_type 1..=8 + plausible PowerRail); the Nth descriptor found maps
/// to the Nth set bit of `channel_mask` (the channels are enumerated in
/// ascending bit order by the driver).
pub fn descriptor_rail_map(info: &PowerMonitorInfo) -> Vec<(u32, u32, u32)> {
    // Ordered set bits of the channel mask — descriptor N corresponds to bit N.
    let bits: Vec<u32> = (0..32)
        .filter(|i| info.channel_mask & (1 << i) != 0)
        .collect();
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
pub fn extract_channel_mw(nz: &[(usize, u32)], baseline: &std::collections::HashSet<usize>) -> u32 {
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

/// Per-channel intermediate state used by [`disambiguate_power_rails`].
struct DisChan {
    bit: u32,
    pwr_rail: u32,
    channel_type: u32,
    /// This bit's GetStatus nonzero `(offset, value)` slots.
    nz: Vec<(usize, u32)>,
    /// Private offsets (nonzero here, absent from every other bit).
    private: Vec<(usize, u32)>,
    confidence: Confidence,
    pwr_mw: u32,
    /// Secondary private readings beyond the primary (lowest-offset) one.
    aux_readings: Vec<(usize, u32)>,
}

/// Resolve per-rail power readings from a GetInfo descriptor map + per-bit
/// GetStatus samples, assigning each channel a [`Confidence`] tier.
///
/// Algorithm (adversarially reviewed — see commit history for the bug list it
/// closes): ownership is monotonic via `resolved_owner`; values are computed
/// only after ownership settles. For each still-ambiguous channel we look for a
/// candidate offset that (a) isn't baseline, (b) isn't already owned, (c) has
/// every other claimant already resolved, (d) agrees with all claimants on the
/// value within 25% (else it's sensor cross-talk, not clean ownership), and
/// (e) whose GPU-Z label (if known) matches the descriptor rail. The first
/// channel to satisfy these for an offset claims it; the worklist iterates to a
/// fixed point. This never synthesizes a reading from no signal (Turing, where
/// the baseline swallows everything, stays all-Ambiguous — the safe outcome).
pub fn disambiguate_power_rails(
    rail_map: &[(u32, u32, u32)],
    per_bit: &[(u32, Vec<(usize, u32)>)],
) -> PowerRails {
    use std::collections::{HashMap, HashSet};
    let n = rail_map.len();
    let all_sets: Vec<HashSet<usize>> = per_bit
        .iter()
        .map(|(_, nz)| nz.iter().map(|(o, _)| *o).collect())
        .collect();

    // Baseline = offsets present in EVERY non-empty buffer (an empty buffer is
    // an unreadable channel, not evidence for the shared baseline).
    let baseline: HashSet<usize> = {
        let nonempty: Vec<&HashSet<usize>> = all_sets.iter().filter(|s| !s.is_empty()).collect();
        if nonempty.is_empty() {
            HashSet::new()
        } else {
            let mut iter = nonempty.iter();
            let first = (*iter.next().unwrap()).clone();
            iter.fold(first, |acc, s| acc.intersection(s).copied().collect())
        }
    };

    // Per-channel private offsets + initial confidence.
    let mut chans: Vec<DisChan> = (0..n)
        .map(|i| {
            let my = all_sets.get(i).cloned().unwrap_or_default();
            let other_union: HashSet<usize> = (0..n)
                .filter(|&j| j != i)
                .flat_map(|j| all_sets.get(j).into_iter().flat_map(|s| s.iter().copied()))
                .collect();
            let private: Vec<(usize, u32)> = per_bit
                .get(i)
                .map(|(_, nz)| {
                    nz.iter()
                        .filter(|(o, _)| my.contains(o) && !other_union.contains(o))
                        .copied()
                        .collect()
                })
                .unwrap_or_default();
            let confidence = if my.is_empty() {
                Confidence::Unavailable
            } else if !private.is_empty() {
                Confidence::Measured
            } else {
                Confidence::Ambiguous
            };
            DisChan {
                bit: rail_map[i].0,
                pwr_rail: rail_map[i].1,
                channel_type: rail_map[i].2,
                nz: per_bit.get(i).map(|(_, v)| v.clone()).unwrap_or_default(),
                private: private.clone(),
                confidence,
                pwr_mw: 0,
                aux_readings: Vec::new(),
            }
        })
        .collect();

    // Phase 1: seed ownership from Measured channels' private offsets. First
    // claimant wins (private offsets are exclusive by definition, so there is
    // no real contention — but two Measured channels could in principle both
    // list an offset if the per-bit buffers were noisy; first-wins is safe).
    let mut resolved_owner: HashMap<usize, usize> = HashMap::new();
    for (i, c) in chans.iter().enumerate() {
        if c.confidence == Confidence::Measured {
            for &(o, _) in &c.private {
                resolved_owner.entry(o).or_insert(i);
            }
        }
    }

    // Phase 2: iterate disambiguation to a fixed point. Order doesn't affect
    // the final ownership map (monotonic), only how many passes it takes.
    let mut changed = true;
    while changed {
        changed = false;
        // Process channels with FEWER candidate offsets first (more private
        // offsets ≈ more constrained) — stable by original index.
        let mut order: Vec<usize> = (0..n)
            .filter(|&i| chans[i].confidence == Confidence::Ambiguous)
            .collect();
        order.sort_by_key(|&i| {
            let nonbase = chans[i]
                .nz
                .iter()
                .filter(|(o, _)| !baseline.contains(o))
                .count();
            (std::cmp::Reverse(nonbase), i)
        });

        for i in order {
            // Collect candidate offsets for channel i.
            let mut candidates: Vec<(usize, u32)> = Vec::new();
            for &(o, v) in &chans[i].nz {
                if baseline.contains(&o) {
                    continue;
                }
                if resolved_owner.contains_key(&o) {
                    continue; // already owned — never leak a resolved offset
                }
                // claimants = OTHER bits whose buffer also contains o.
                let claimants: Vec<usize> = (0..n)
                    .filter(|&j| j != i && all_sets.get(j).map(|s| s.contains(&o)).unwrap_or(false))
                    .collect();
                // Need every other claimant to already be resolved (have a
                // value), so o is uniquely attributable to i.
                let all_resolved = claimants.iter().all(|&j| {
                    matches!(
                        chans[j].confidence,
                        Confidence::Measured | Confidence::Inferred
                    )
                });
                if !all_resolved {
                    continue;
                }
                // Value consistency: if claimants disagree on o by >25%, this
                // is sensor cross-talk (one read the rail, another the board
                // sum), not clean ownership — reject.
                let mut vals: Vec<u32> = vec![v];
                for &j in &claimants {
                    if let Some(vj) = chans[j]
                        .nz
                        .iter()
                        .find(|(oo, _)| *oo == o)
                        .map(|(_, vv)| *vv)
                    {
                        vals.push(vj);
                    }
                }
                let mx = *vals.iter().max().unwrap_or(&0);
                let mn = *vals.iter().min().unwrap_or(&0);
                if mn > 0 && (mx as f64) / (mn as f64) > 1.25 {
                    continue;
                }
                // Semantic gate: if o has a known GPU-Z label, i's descriptor
                // rail must match it (else we'd mislabel the rail). Unlabeled
                // offsets pass through unchecked.
                if let Some(label) = gpuz_offset_label(o) {
                    if !rail_matches_label(chans[i].pwr_rail, label) {
                        continue;
                    }
                }
                candidates.push((o, v));
            }

            if !candidates.is_empty() {
                // Prefer the largest-value candidate (the dominant reading);
                // tie-break by lowest offset (the record's primary slot).
                candidates.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                let (o, v) = candidates[0];
                resolved_owner.insert(o, i);
                chans[i].pwr_mw = v;
                chans[i].confidence = Confidence::Inferred;
                changed = true;
            }
        }
    }

    // Phase 3: compute values for channels not set during the loop.
    for c in chans.iter_mut() {
        match c.confidence {
            Confidence::Measured => {
                // Primary = lowest-offset private slot; rest are aux readings
                // (don't `max()` them away — e.g. type=8 instant + avg slots).
                let mut private_sorted = c.private.clone();
                private_sorted.sort_by_key(|(o, _)| *o);
                c.pwr_mw = private_sorted.first().map(|(_, v)| *v).unwrap_or(0);
                c.aux_readings = private_sorted[1..].to_vec();
            }
            Confidence::Inferred => {
                // pwr_mw already set during disambiguation.
            }
            Confidence::Ambiguous => {
                c.pwr_mw = extract_channel_mw(&c.nz, &baseline);
            }
            Confidence::Unavailable => {
                c.pwr_mw = 0;
            }
        }
    }

    // Phase 4: build PowerRailReading (gpuz_name populated here — the earlier
    // build site omitted it, which silently no-op'd the semantic gate).
    chans
        .into_iter()
        .map(|c| PowerRailReading {
            channel_bit: c.bit,
            pwr_rail: c.pwr_rail,
            rail_name: power_rail_name_owned(c.pwr_rail),
            channel_type: c.channel_type,
            pwr_mw: c.pwr_mw,
            confidence: c.confidence,
            aux_readings: c.aux_readings,
            gpuz_name: gpu_z_rail_name(c.pwr_rail).map(|s| s.to_string()),
        })
        .collect()
}

/// GPU-Z-equivalent friendly name for an NVAPI `pwr_rail` value, or `None`
/// when no GPU-Z equivalent is known. This is a SEMANTIC map (NVAPI rail →
/// GPU-Z sensor label), validated by cross-referencing load/idle readings
/// against GPU-Z on RTX 4060 Laptop + desktop Turing:
///   - `InputTotalBoard` (245) ≈ GPU-Z "Board Power" / the 16-pin input on
///     laptops. (GPU-Z "Board Power" includes PCIe-slot overhead, so this is
///     close but not identical at idle.)
///   - `InputNvvdd` (246) / `InputNvvdd1` (226) ≈ GPU-Z "GPU Chip Power"
///     (the core input rail).
///   - `InputFbvdd` (247) ≈ GPU-Z "MVDDC" (memory input rail).
///   - `InputPwrSrcPp` (241) ≈ GPU-Z "PWR_SRC".
///   - `InputPex12v1` (222) / `InputPex12v` (255) ≈ GPU-Z "PCIe slot" power.
///   - `InputTotalBoard2` (223) ≈ GPU-Z "Board Power" (a second board-total
///     channel, same semantic as 245; present on some SKUs alongside 245).
///   - `OutputNvvdd` (1) ≈ GPU-Z "Chip" output regulator.
/// Other rails (PEX3V3, Misc, Ext12v connectors, …) have no clean GPU-Z
/// single-rail equivalent → `None` (label by `rail_name` instead).
pub fn gpu_z_rail_name(rail: u32) -> Option<&'static str> {
    match rail {
        245 | 223 => Some("Board"), // InputTotalBoard / InputTotalBoard2
        246 | 226 => Some("Chip"),  // InputNvvdd / InputNvvdd1 (core in)
        247 => Some("MVDDC"),       // InputFbvdd (memory in)
        241 => Some("PWR_SRC"),     // InputPwrSrcPp
        222 | 255 => Some("PCIe"),  // InputPex12v1 / InputPex12v (slot)
        1 => Some("Chip-out"),      // OutputNvvdd
        _ => None,
    }
}

/// Human channel names for the PowerMonitor rail space, cross-referenced
/// from AmpereOC's "Power Statistics" panel (sub_140068074): it renders 12
/// fixed slots — Total Board Power Draw, Framebuffer VDD, NVIDIA GPU VDD,
/// 12v PCIE 8-pin #1/#2/#3, 12v PEX Rail, PWR PP SRC, SRAM, MISC #0..#3 —
/// each fed by one PowerMonitor channel (its GPU-object slots sit at a
/// uniform 168-byte pitch, matching the GetInfo descriptor size). AmpereOC
/// confirms these rails are reported in **milliwatts**. Use alongside
/// [`gpu_z_rail_name`]: this table names the AmpereOC view of the rail
/// space (incl. the unnamed-gap rails 218..=240), that names the GPU-Z
/// view of the confirmed subset.
pub fn ampereoc_rail_name(rail: u32) -> Option<&'static str> {
    match rail {
        245 | 223 => Some("Total Board Power Draw"), // TBPD  = InputTotalBoard(2)
        246 | 226 => Some("NVIDIA GPU VDD"),         // NVVDD = InputNvvdd(1)
        247 => Some("Framebuffer VDD"),              // FBVDD = InputFbvdd
        241 => Some("PWR PP SRC"),                   // PWRPPCSRC = InputPwrSrcPp
        222 | 255 => Some("12v PEX Rail"),           // PEX   = InputPex12v1 / InputPex12v
        228 => Some("12v PCIE 8-pin #1"),            // InputExt12v8pin2
        229 => Some("12v PCIE 8-pin #2"),            // InputExt12v8pin3
        230 => Some("12v PCIE 8-pin #3"),            // InputExt12v8pin4
        11 => Some("SRAM"),                          // OutputSram
        232 => Some("MISC #0"),                      // InputMisc0
        233 => Some("MISC #1"),                      // InputMisc1
        234 => Some("MISC #2"),                      // InputMisc2
        235 => Some("MISC #3"),                      // InputMisc3
        _ => None,
    }
}

/// Milliwatt display unit for PowerMonitor values, per AmpereOC's Power
/// Statistics panel (all 12 slots rendered in mW). Applies to both the
/// GetStatus live values and any averaged/min/max fields.
pub const POWER_MONITOR_UNIT_MW: &str = "mW";

/// Full RTSS `NV_GPU_POWER_CHANNEL_POWER_RAIL` enum naming — the complete
/// coverage layer (0-11 outputs, 218..=255 inputs), promoted from the
/// gpu_readonly test helper. Programming-style names; used as the
/// fallback layer of [`rail_display_name`] so every known rail value
/// resolves to *something*.
pub fn rtss_rail_name(rail: u32) -> Option<&'static str> {
    Some(match rail {
        0 => "Unknown",
        1 => "OutputNvvdd",
        2 => "OutputFbvdd",
        3 => "OutputFbvddq",
        4 => "OutputFbvddQ",
        5 => "OutputPexvdd",
        6 => "OutputA3v3",
        7 => "Output3v3nv",
        8 => "OutputTotalGpu",
        9 => "OutputFbvddqGpu",
        10 => "OutputFbvddqMem",
        11 => "OutputSram",
        222 => "InputPex12v1",
        223 => "InputTotalBoard2",
        224 => "InputHighVolt0",
        225 => "InputHighVolt1",
        226 => "InputNvvdd1",
        227 => "InputNvvdd2",
        228 => "InputExt12v8pin2",
        229 => "InputExt12v8pin3",
        230 => "InputExt12v8pin4",
        231 => "InputExt12v8pin5",
        232 => "InputMisc0",
        233 => "InputMisc1",
        234 => "InputMisc2",
        235 => "InputMisc3",
        236 => "InputUsbc0",
        237 => "InputUsbc1",
        238 => "InputFan0",
        239 => "InputFan1",
        240 => "InputSram",
        241 => "InputPwrSrcPp",
        242 => "Input3v3Pp",
        243 => "Input3v3Main",
        244 => "Input3v3Aon",
        245 => "InputTotalBoard",
        246 => "InputNvvdd",
        247 => "InputFbvdd",
        248 => "InputFbvddq",
        249 => "InputFbvddQ",
        250 => "InputExt12v8pin0",
        251 => "InputExt12v8pin1",
        252 => "InputExt12v6pin0",
        253 => "InputExt12v6pin1",
        254 => "InputPex3v3",
        255 => "InputPex12v",
        _ => return None,
    })
}

/// Three-layer merged rail naming — the definitive resolver. The three
/// sources cover disjoint gaps with ZERO contradictions on overlap
/// (verified rail-by-rail: GPU-Z market names vs AmpereOC NVIDIA names
/// are synonyms, e.g. MVDDC≡Framebuffer VDD, Chip≡NVIDIA GPU VDD):
///
/// 1. [`ampereoc_rail_name`] — AmpereOC "Power Statistics" panel names:
///    most precise NVIDIA semantics (TBPD/FBVDD/NVVDD/8pin/PEX/…), but
///    only the 14 rails its panel renders. Values confirmed mW.
/// 2. [`rtss_rail_name`] — full RTSS enum naming, complete coverage of
///    every defined rail value (programming-style, no prose).
/// 3. [`gpu_z_rail_name`] — GPU-Z market names; only adds "Chip-out"
///    beyond layer 1's coverage (kept for stable GPU-Z-facing output).
///
/// Returns `None` only for genuinely undefined rail values (e.g. the
/// unnamed gap 218..=221 observed on some SKUs).
pub fn rail_display_name(rail: u32) -> Option<&'static str> {
    ampereoc_rail_name(rail)
        .or_else(|| rtss_rail_name(rail))
        .or_else(|| gpu_z_rail_name(rail))
}

/// Confidence tier for a per-rail power reading, expressing how the value
/// was obtained. Drives downstream rendering (CLI/pynvoc/TUI suffix the rail
/// name: Measured=plain, Inferred=`~`, Ambiguous=`?`, Unavailable=omitted).
///
/// - [`Confidence::Measured`]: the channel had ≥1 genuinely PRIVATE GetStatus
///   offset (nonzero in this channel's per-bit buffer only). Trustworthy.
/// - [`Confidence::Inferred`]: no private offset, but topology disambiguation
///   (the channel is the unique un-owned claimant of a shared offset) plus a
///   semantic + value-consistency check attributed a shared offset to it.
///   Display with a `~` marker — best-effort, usually right.
/// - [`Confidence::Ambiguous`]: disambiguation found no clean candidate; the
///   value is the largest non-baseline reading (a full-board view that may be
///   a duplicate of another rail). Display with a `?` marker.
/// - [`Confidence::Unavailable`]: the channel exists in GetInfo but GetStatus
///   did not populate it (empty buffer). Omit from display.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Confidence {
    /// GetStatus did not populate this channel (empty per-bit buffer).
    #[default]
    Unavailable,
    /// No clean disambiguation; value is a full-board-view fallback.
    Ambiguous,
    /// Unique-claimant topology + semantic + value-consistency attribution.
    Inferred,
    /// ≥1 genuinely private GetStatus offset — trustworthy per-channel read.
    Measured,
}

impl Confidence {
    /// `true` for tiers a caller may display as a per-channel value without a
    /// caveat marker (Measured; Inferred is shown but flagged `~`).
    pub fn is_trusted(self) -> bool {
        matches!(self, Confidence::Measured)
    }
}

/// GPU-Z-confirmed GetStatus byte-offset → rail-label map, for the semantic
/// cross-check during disambiguation. This is the ONLY place a hardcoded
/// offset appears, and it is used solely as a soft gate (reject a candidate
/// whose descriptor rail contradicts the offset's known GPU-Z label), never
/// as the extraction mechanism. Offsets not in this table return `None` and
/// are allowed through the gate unchecked. Per-GPU validated on RTX 4060
/// Laptop; unknown on other SKUs (the soft gate is a no-op there).
const GPUZ_OFFSET_LABELS: &[(usize, &str)] = &[
    (0x14, "Chip"),     // GPU Chip Power Draw
    (0x2C, "MVDDC"),    // MVDDC Power Draw
    (0xE0, "Chip-out"), // core sub-channel (≈ GPU Chip output)
    (0xEC, "Chip-out"), // core sub-channel (≈ GPU Chip output)
    (0x98, "PWR_SRC"),  // PWR_SRC Power Draw
                        // +0x08 (Board), +0x44 (16-pin) are baseline/ch0 slots handled elsewhere;
                        // +0x14C ≈ +0x2C duplicate, left ungated (no unique label).
];

/// Look up the GPU-Z rail label for a GetStatus byte offset, or `None` when
/// the offset has no confirmed GPU-Z equivalent. Used only by the
/// disambiguation semantic gate — see [`GPUZ_OFFSET_LABELS`].
pub fn gpuz_offset_label(offset: usize) -> Option<&'static str> {
    GPUZ_OFFSET_LABELS
        .binary_search_by_key(&offset, |e| e.0)
        .ok()
        .map(|i| GPUZ_OFFSET_LABELS[i].1)
}

/// `true` when a descriptor `pwr_rail` is semantically compatible with a GPU-Z
/// offset label (via [`gpu_z_rail_name`]). Rails with no known GPU-Z name
/// (`None`) are accepted unchecked so the gate doesn't starve unnamed SKUs.
pub fn rail_matches_label(rail: u32, label: &str) -> bool {
    gpu_z_rail_name(rail).map(|r| r == label).unwrap_or(true)
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

#[cfg(test)]
mod rail_name_tests {
    use super::*;

    #[test]
    fn merged_resolver_zero_blind_spots() {
        // Every rail the scan-gate accepts must resolve through the merged
        // resolver: outputs 1..=11, inputs 222..=255 (plus 0 = Unknown).
        for rail in 0u32..=11 {
            assert!(rail_display_name(rail).is_some(), "output rail {rail} unnamed");
        }
        for rail in 222u32..=255 {
            assert!(rail_display_name(rail).is_some(), "input rail {rail} unnamed");
        }
    }

    #[test]
    fn merged_resolver_layer_priority() {
        // Layer 1 (AmpereOC precise) wins on its 14 rails.
        assert_eq!(rail_display_name(245), Some("Total Board Power Draw"));
        assert_eq!(rail_display_name(247), Some("Framebuffer VDD"));
        assert_eq!(rail_display_name(11), Some("SRAM"));
        assert_eq!(rail_display_name(228), Some("12v PCIE 8-pin #1"));
        // Layer 2 (RTSS full enum) covers what layer 1 misses.
        assert_eq!(rail_display_name(243), Some("Input3v3Main"));
        assert_eq!(rail_display_name(238), Some("InputFan0"));
        // Layer 3 (GPU-Z) adds Chip-out on top of layers 1-2 gaps... note
        // rail 1 is covered by RTSS as OutputNvvdd before GPU-Z is reached.
        assert_eq!(rail_display_name(1), Some("OutputNvvdd"));
        assert_eq!(gpu_z_rail_name(1), Some("Chip-out")); // still available directly
        // Undefined gap rails return None.
        assert_eq!(rail_display_name(218), None);
    }

    #[test]
    fn no_contradictions_between_sources() {
        // Wherever two sources both name a rail they must agree in meaning.
        // Verified pairs (synonyms, not conflicts):
        //   GPU-Z "Board"        == AmpereOC "Total Board Power Draw"  (245|223)
        //   GPU-Z "Chip"         == AmpereOC "NVIDIA GPU VDD"         (246|226)
        //   GPU-Z "MVDDC"        == AmpereOC "Framebuffer VDD"        (247)
        //   GPU-Z "PWR_SRC"      == AmpereOC "PWR PP SRC"             (241)
        //   GPU-Z "PCIe"         == AmpereOC "12v PEX Rail"           (222|255)
        // The RTSS layer always names the same rail value, so a None/Some
        // divergence between ampereoc and gpu_z tables would be a bug:
        for rail in [245u32, 223, 246, 226, 247, 241, 222, 255] {
            let a = ampereoc_rail_name(rail);
            let g = gpu_z_rail_name(rail);
            assert!(
                a.is_some() && g.is_some(),
                "rail {rail} lost a source name: ampereoc={a:?} gpuz={g:?}"
            );
        }
    }
}
