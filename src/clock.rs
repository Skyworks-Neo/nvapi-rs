use crate::gpu::VfpInfo;
use crate::sys;
use crate::sys::gpu::{clock, power};
use crate::sys::types::ClockMask;
use crate::types::{
    Kilohertz, Kilohertz2Delta, KilohertzDelta, Microvolts, Percentage, Percentage1000, Range,
    RawConversion,
};
use log::trace;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fmt;

pub use sys::gpu::clock::PublicClockId as ClockDomain;
pub use sys::gpu::clock::private::ClockDomainId;
pub use sys::gpu::clock::private::{PerfLimitId, VfPointType};
pub use sys::gpu::power::private::{PerfFlags, PowerTopologyChannelId};

impl RawConversion for clock::NV_GPU_CLOCK_FREQUENCIES {
    type Target = BTreeMap<ClockDomain, Kilohertz>;
    type Error = Infallible;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok(ClockDomain::values()
            .filter(|&c| c != ClockDomain::Undefined)
            .map(|id| (id, &self.domain[id.raw() as usize]))
            .filter(|&(_, clock)| clock.bIsPresent.get())
            .map(|(id, clock)| (id, Kilohertz(clock.frequency)))
            .collect())
    }
}

/// Effective (actually-running) clocks from GetAllClocks V2
/// (`NvAPI_GPU_GetAllClocks`, RTSS `NV_GPU_CLOCK_INFO_V2`). The
/// `extendedDomain[]` effective frequency for each present public domain
/// (Graphics/Memory/Processor). Distinct from [`ClockFrequencies`] (the
/// GetAllClockFrequencies base/boost/current table).
pub type EffectiveClocks = BTreeMap<ClockDomain, Kilohertz>;

/// All 32 effective clock domains from GetAllClocks V2 (RTSS
/// `NV_GPU_CLOCK_INFO_V2.extendedDomain[]`), keyed by [`ClockDomainId`].
/// Superset of [`EffectiveClocks`] (which only carries the 4 *public* clocks
/// Graphics/Memory/Processor/Video): this additionally exposes the internal
/// fabric clocks — Gpc, **Xbar (crossbar)**, Sys, Hub, Host, Disp, Hotclk,
/// Gpc2/Xbar2/Sys2/Hub2, Pciegen, etc. — i.e. everything GPU-Z's "crossbar
/// clock" and similar readings come from. Only domains with a non-zero
/// `effective_frequency` are included.
///
/// **MOBILE/LAPTOP GAP — important.** On desktop GPUs `extended_domain[]`
/// is populated and this returns the full fabric table. On at least one
/// mobile GPU (RTX 4060 Laptop / R610.74) the driver returns `extended_domain[]`
/// ALL-ZERO — only `domain[]` base clocks come back — so `all_clocks()`
/// yields an empty map and `get-status` shows no fabric domains. The
/// base clocks (the `clocks`/`GetAllClockFrequencies` table) ARE returned
/// on mobile; only the V2 effective-fabric extension is absent. This is a
/// driver/GPU behavior, not a parse error. The private ClockClient
/// `MEASURE_FREQ` (`get-clk-domain-freq`) is the mobile-side workaround —
/// it reads every controllable domain's physical clock directly.
///
/// **ARCHITECTURE-DEPENDENT LABELS — important caveat.** The
/// [`ClockDomainId`] enum names (Gpc/Xbar/Sys/Hub/…) come from the RTSS
/// (RivaTuner) NDA header and are a *historical* convention. The first four
/// indices are dual-aliased in that header — `GPC≡NV=0, XBAR≡G=1, SYS≡S=2,
/// HUB≡R=3` — reflecting that the NV/G/S/R names were the pre-Maxwell
/// convention and GPC/XBAR/SYS/HUB the Maxwell+ convention. Because
/// `NV_GPU_CLOCK_INFO_V2` is undocumented/NDA, **the physical clock living at
/// each index SHIFTED across GPU architectures**, so on modern GPUs (Ada/
/// Hopper/Blackwell) the value at e.g. index 3 (labelled "Hub" here) may be
/// what GPU-Z calls "Xbar", and other fabric labels may be similarly rotated.
/// The `effective_frequency` VALUES are correct (verified for Gpc/Graphics and
/// M/Memory); only the human-readable LABELS are arch-stale. Tools like GPU-Z
/// carry their own arch-specific index→name remap; consumers wanting exact
/// parity with GPU-Z should treat these labels as advisory, not authoritative,
/// for non-public domains.
///
/// Observed on an RTX 4060 Laptop (Ada), under load:
///   Gpc(0)≈2200MHz tracks the Graphics core clock (2145MHz reported by
///     GetAllClockFrequencies); M(4)≈7993MHz tracks Memory (8000MHz).
///   The fabric clocks that SCALE with load (Xbar(1) 71→1984, Sys(2) 249→1894,
///     Host(5) 825→1350, Msd(21) 246→1970) are the dynamically-clocked fabric
///     domains; the constant ones (Hub(3)=450, Disp(6)=675, Pwr(20)=540,
///     Utils(22)=108) are fixed clocks. GPU-Z's "crossbar clock" most likely
///     corresponds to whichever fabric domain scales with the core on a given
///     architecture — verify against GPU-Z rather than trusting the enum label.
///
/// Additionally, `Pciegen` (index 31) is NOT a clock frequency — **its value
/// IS the current PCIe link generation number** (1=Gen1 … 5=Gen5). It is
/// stored in the same `effective_frequency` field as the clock kHz values but
/// is a raw integer: observed 1 at idle (Gen1 link power-saving) rising to 4
/// under load (Gen4) on an RTX 4060 Laptop. This is an alternative
/// NVAPI-sourced way to read the current PCIe gen (vs NVML
/// `nvmlDeviceGetCurrPcieLinkGeneration`). NB: the `Kilohertz` wrapper is
/// semantically wrong here; consumers should interpret index 31 as a gen
/// number, not kHz.
///
/// `Msd` (index 21) is a memory sub-domain clock (most likely the GDDR
/// controller/PHY serdes) — observed scaling ~1/4 of the effective memory rate
/// (1970 MHz at 8000 MHz memory on RTX 4060). Treat the MSD label as advisory.
pub type AllClocks = BTreeMap<ClockDomainId, Kilohertz>;

/// Extract all 32 clock domains from a raw GetAllClocks V2 result (companion
/// to `NV_GPU_CLOCK_INFO_V2`'s `EffectiveClocks` conversion, which only reads
/// the 4 public domains). Returns every present domain keyed by
/// [`ClockDomainId`].
pub fn all_clocks_from_raw(raw: &clock::private::NV_GPU_CLOCK_INFO_V2) -> AllClocks {
    ClockDomainId::values()
        .map(|id| {
            (
                id,
                raw.extended_domain[id.raw() as usize].effective_frequency,
            )
        })
        // Skip Pciegen(31): its effective_frequency holds the current PCIe link
        // generation NUMBER (1..5), not a kHz clock — including it would render
        // a misleading "Pciegen 0.001 MHz" in the All Clocks list. It is read
        // via the dedicated pcie_link_gen status field instead.
        .filter(|(id, _)| *id != ClockDomainId::Pciegen)
        .filter(|(_, freq)| *freq != 0)
        .map(|(id, freq)| (id, Kilohertz(freq)))
        .collect()
}

impl RawConversion for clock::private::NV_GPU_CLOCK_INFO_V2 {
    type Target = EffectiveClocks;
    type Error = Infallible;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok(ClockDomain::values()
            .filter(|&c| c != ClockDomain::Undefined)
            .map(|id| (id, &self.extended_domain[id.raw() as usize]))
            .filter(|(_, d)| d.effective_frequency != 0)
            .map(|(id, d)| (id, Kilohertz(d.effective_frequency)))
            .collect())
    }
}

impl RawConversion for clock::private::NV_USAGES_INFO {
    type Target = BTreeMap<crate::pstate::UtilizationDomain, Percentage>;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        self.usages
            .iter()
            .enumerate()
            .filter(|&(_, usage)| usage.bIsPresent.get())
            .map(|(i, usage)| {
                crate::pstate::UtilizationDomain::from_raw(i as _)
                    .and_then(|i| Percentage::from_raw(usage.percentage).map(|p| (i, p)))
            })
            .collect()
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct VfpMask {
    pub mask: ClockMask,
}

impl RawConversion for clock::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_CLOCK {
    type Target = VfPointType;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        VfPointType::from_raw(self.clock_type as i32)
    }
}

impl RawConversion for clock::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO {
    type Target = VfpMask;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);

        Ok(VfpMask { mask: self.mask })
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct ClockTable {
    pub delta_points: BTreeMap<ClockDomain, Vec<(usize, KilohertzDelta)>>,
}

impl ClockTable {
    pub fn from_raw(
        raw: &clock::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL,
        info: &VfpInfo,
    ) -> crate::Result<Self> {
        Ok(Self {
            delta_points: info
                .domains
                .domains
                .iter()
                .map(|d| {
                    info.index(d.domain, &raw.points[..])
                        .map(|(i, p)| p.convert_raw().map(|p| (i, p)))
                        .collect::<Result<_, _>>()
                        .map(|p| (d.domain, p))
                })
                .collect::<Result<_, _>>()?,
        })
    }
}

impl RawConversion for clock::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_CONTROL_V1 {
    type Target = KilohertzDelta;
    type Error = sys::ArgumentRangeError;

    #[allow(non_snake_case)]
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        let clock::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_CONTROL_V1 {
            clock_type: _,
            freqDeltaKHz,
            rsvd: _,
            padding: _,
        } = *self;
        Ok(freqDeltaKHz.into())
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct ClockRange {
    pub domain: ClockDomain,
    pub range: Range<Kilohertz2Delta>,
    pub vfp_index: Range<usize>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct ClockDomainInfo {
    pub domains: Vec<ClockRange>,
}

impl ClockDomainInfo {
    pub fn get(&self, domain: ClockDomain) -> Option<&ClockRange> {
        self.domains.iter().find(|d| d.domain == domain)
    }
}

impl RawConversion for clock::private::NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_INFO_ENTRY {
    type Target = ClockRange;
    type Error = sys::ArgumentRangeError;

    #[allow(non_snake_case)]
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        match *self {
            clock::private::NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_INFO_ENTRY {
                disabled: 0,
                clockType,
                rangeMax,
                rangeMin,
                vfpIndexMin,
                vfpIndexMax,
                unknown0: _,
                unknown1: _,
                padding: _,
            } => Ok(ClockRange {
                domain: ClockDomain::from_raw(clockType)?,
                range: Range {
                    max: Kilohertz2Delta(rangeMax),
                    min: Kilohertz2Delta(rangeMin),
                },
                vfp_index: Range {
                    min: vfpIndexMin as usize,
                    max: vfpIndexMax as usize,
                },
            }),
            _ => Err(sys::ArgumentRangeError),
        }
    }
}

impl RawConversion for clock::private::NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_INFO {
    type Target = ClockDomainInfo;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        let domains = self
            .mask
            .index(&self.entries[..])
            .map(|(_i, v)| v)
            .filter(|v| v.disabled == 0)
            .map(RawConversion::convert_raw)
            .collect::<Result<_, _>>()?;
        Ok(ClockDomainInfo { domains })
    }
}

// --- Blackwell XBar ClockClient clock-domain family -----------------------
// (reverse/melonvolt/xbar.txt — Loong0x00 LACT #1147). The 4 NV2080 RM
// commands wrapped via private NVAPI IDs (escape 0x07000109, same 0x0700_01xx
// family as VoltRails). All live-verified on Ada 4060 Laptop / R575.74.

/// One controllable clock-domain entry from the private ClockClient
/// GetControl (RM 0x2080901b, ID 0xF58938F5). `domain` is the
/// [`ClockDomainId`] (its value also = the controllable-mask bit position =
/// the MEASURE_FREQ `domain_index`). The article's XBAR domain is
/// [`ClockDomainId::Xbar`] = 1 (mask bit 0x2).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
#[allow(nonstandard_style)] // kHz suffix matches the sys-layer field naming
pub struct ClkDomainControlEntry {
    /// mask bit position (== domain index)
    pub bit: u32,
    /// record type byte (live 0x0A=10 on GPC/XBAR/SYS/MCLK on Ada 4060 Laptop)
    pub entry_type: u8,
    /// Whether an offset can be WRITTEN to this domain through the
    /// SetControl family — derived purely from the record TYPE byte, NOT
    /// a driver capability bit, and says NOTHING about readability: the
    /// driver-side remap (sub_18015BB30/BD20) maps protocol ↔ internal
    /// types and the per-record switch only copies value dwords for a
    /// fixed internal-type set (V2 magic 0x261A4 → protocol {1,3..=10,15};
    /// V1 0x10964 → {1,3..=9}). Type 0x02 (Disp/Host on several
    /// generations, M on A100) is marshalled by NEITHER — SetControl
    /// silently drops it — yet those clocks may still be READABLE via
    /// MEASURE_FREQ/GetAllClocks (A100 M reads fine; conversely Pascal has
    /// marshalled domains whose measure is RM-rejected). Readability and
    /// writability are independent per domain.
    pub value_modifiable: bool,
    /// The record's value dwords. V2 reads them from rec+268..296 (8
    /// dwords); the V1 fallback fills slots 0..4 from rec+44..60. Slot
    /// semantics are driver-opaque — per the article slot 0 is the signed
    /// frequency offset (kHz) and neighbors are range/voltage terms, but
    /// only an A/B SET + MEASURE_FREQ experiment confirms which is which.
    pub values_kHz: [i32; 8],
}

impl ClkDomainControlEntry {
    /// Protocol types whose value dwords the V2 (magic 0x261A4) protocol
    /// marshals in both GET and SET. The protocol→internal remap
    /// (sub_18015BD20) is identity+1, and the per-record switch only
    /// handles internal {2,4,5,6,7,8,9,0xA,0xB,0x10} → protocol
    /// {1,3,4,5,6,7,8,9,10,15}. Protocol 2 (e.g. Disp bit 6 → internal 3)
    /// is marshalled by NEITHER version — the driver silently drops it
    /// (live-confirmed: disp SET readback never matches).
    pub fn v2_marshalable(entry_type: u8) -> bool {
        matches!(entry_type, 1 | 3..=10 | 15)
    }

    /// Protocol types whose value dwords the V1 (magic 0x10964) protocol
    /// marshals (internal set {2,4,5,6,7,8,9,0xA} remapped to protocol).
    pub fn v1_marshalable(entry_type: u8) -> bool {
        matches!(entry_type, 1 | 3 | 4 | 5 | 6 | 7 | 8 | 9)
    }
}

impl ClkDomainControlEntry {
    /// The typed clock-domain id for this entry's bit, or `None` if the bit
    /// doesn't map to a known [`ClockDomainId`] (e.g. an unnamed NDA domain).
    pub fn domain(&self) -> Option<ClockDomainId> {
        ClockDomainId::from_raw(self.bit as i32).ok()
    }
}

/// Read-only snapshot of the controllable clock-domain block from GetControl.
/// `mask` is the controllable-domain bitmask; `entries` one per set bit.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ClockDomainControl {
    /// controllable-domain bitmask (live 0xFF on Ada 4060 Laptop; includes
    /// XBAR bit 0x2 — XBAR-as-controllable is NOT Blackwell-only)
    pub mask: u32,
    pub entries: Vec<ClkDomainControlEntry>,
}

/// Physical clock measurement from MEASURE_FREQ (RM 0x20809006, ID
/// 0xFB8F61EC). Windows returns raw {counter, timestamp}, NOT the article's
/// direct kHz; the medium layer samples twice and computes
/// `freq = Δcounter / Δtimestamp_ns × 1e9 Hz`, reported here as MHz.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ClockDomainFreq {
    /// the measured domain
    pub domain: ClockDomainId,
    /// physical frequency in MHz (two-sample Δcounter/Δt)
    pub freq_mhz: f64,
}

/// Detailed single-domain MEASURE_FREQ result — the raw protocol fields of
/// the SECOND sample alongside the computed frequency, for counter-scaling
/// calibration (Pascal's M counter unit ≠ cycles) and protocol forensics.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ClockDomainFreqDetail {
    pub domain: ClockDomainId,
    /// physical frequency in MHz (two-sample Δcounter/Δt)
    pub freq_mhz: f64,
    /// which protocol form succeeded: 1 = V1 magic 0x10020 (u32 counter),
    /// 2 = V2 magic 0x20020 (u64 counter)
    pub protocol: u8,
    /// second sample's raw cycle counter (read-modify-write value)
    pub counter: u64,
    /// second sample's raw QPC nanosecond timestamp
    pub timestamp_ns: u64,
    /// second sample's extra dword (+24 out; semantics driver-opaque)
    pub extra: u32,
}

/// One V/F curve point from the private ClockClient V/F-POINTS GetStatus
/// (RM 0x20809062, ID 0x7FEE9032, 488B type-08 records). Records are
/// INDEXED BY VOLTAGE; units live-calibrated against the public GPC VFP
/// curve (`get-vfp`) on R610.74: voltage µV @rec+0x58 (450000 = 450 mV =
/// public point #0), default MHz @rec+0x24, current MHz @rec+0x64
/// (= default + applied offset — 300 = 210 + 90 matched a live +90 MHz
/// public OC).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
#[allow(nonstandard_style)] // uV/mhz suffixes match the sys-layer field naming
pub struct ClkVfPointPrivate {
    /// bank (0 or 1) the record came from
    pub bank: u8,
    /// point index within the bank (0..2048)
    pub index: u16,
    /// record type byte (live 0x08 = V/F curve point on R610.74)
    pub record_type: u8,
    /// point voltage (µV — the V/F grid axis)
    pub voltage_uV: u32,
    /// default frequency at this voltage (MHz)
    pub freq_default_mhz: u32,
    /// current/effective frequency (MHz; = default + applied offset)
    pub freq_current_mhz: u32,
}

/// Read-only snapshot of the private ClockClient V/F-POINTS read path:
/// GetInfo (0x8895B510) point masks + GetStatus (0x7FEE9032) records.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ClkVfPointsPrivate {
    /// present-point masks, 2048 bits per bank: bank0 = masks[0..4],
    /// bank1 = masks[4..8]
    pub masks: [u64; 8],
    /// V/F points the driver filled (record type != 0), bank-major order
    pub points: Vec<ClkVfPointPrivate>,
    /// contiguous same-type runs of [`points`] — bank 0 packs multiple
    /// domains back-to-back, so segmentation is what makes the table
    /// plottable (one curve per type-8 segment)
    pub segments: Vec<ClkVfSegment>,
}

/// One contiguous same-record-type run inside the V/F-points table.
/// Attribution (live A/B on a 4060 Laptop, R610.74 — bank 0):
/// type-8 run 1 (127 pts) = GPC curve, type-7 run = mem pstate bins,
/// type-8 run 2 (127 pts) = XBAR curve, then HOST curve + its pstate
/// list. Segment ORDER is stable per GPU/driver but the domain each run
/// belongs to must be A/B'd (offset one domain, watch which segment's
/// `freq_current_mhz` shifts).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
#[allow(nonstandard_style)] // uV/mhz suffixes match the sys-layer field naming
pub struct ClkVfSegment {
    /// 0 or 1
    pub bank: u8,
    /// record type: 8 (13/18 on other drivers) = V/F curve point,
    /// 7 = pstate frequency bin
    pub record_type: u8,
    /// "vf_curve" (type 8/13/18) or "pstate_bins" (type 7) — plotting hint
    pub kind: ClkVfSegmentKind,
    /// EMPIRICAL domain attribution (advisory), by ordinal within the bank:
    /// vf_curve #1=GPC, #2=XBAR, #3=HOST; pstate_bins #1=Mem, #2=Host.
    /// Live A/B on an RTX 4060 Laptop / R610.74; the ordinal order is
    /// stable per driver but another GPU may pack domains differently —
    /// confirm by offsetting one domain and watching which segment's
    /// per-point current/default values shift.
    pub domain_hint: ClkVfDomainHint,
    /// index of the first point (within the bank)
    pub start_index: u16,
    /// index of the last point (within the bank), inclusive
    pub end_index: u16,
    /// points in the run
    pub count: u16,
    /// voltage axis range (µV)
    pub voltage_uV_min: u32,
    pub voltage_uV_max: u32,
    /// default-frequency range (MHz)
    pub freq_default_mhz_min: u32,
    pub freq_default_mhz_max: u32,
}

/// Advisory domain attribution for a [`ClkVfSegment`] — see
/// [`ClkVfSegment::domain_hint`].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum ClkVfDomainHint {
    Gpc,
    Xbar,
    Host,
    Mem,
    #[default]
    Unknown,
}

impl ClkVfDomainHint {
    /// lowercase slug used in CLI/JSON output ("gpc", "xbar", …)
    pub fn as_str(self) -> &'static str {
        match self {
            ClkVfDomainHint::Gpc => "gpc",
            ClkVfDomainHint::Xbar => "xbar",
            ClkVfDomainHint::Host => "host",
            ClkVfDomainHint::Mem => "mem",
            ClkVfDomainHint::Unknown => "unknown",
        }
    }
}

impl fmt::Display for ClkVfDomainHint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Plotting hint for a [`ClkVfSegment`].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum ClkVfSegmentKind {
    /// V/F curve: voltage-indexed frequency points — plot freq vs voltage
    VfCurve,
    #[default]
    /// pstate frequency bins — a discrete list, not a curve
    PstateBins,
}

// ---------------------------------------------------------------------------
// Mode-1 (reverse-volt) scaling model: effect = C(def) × (delta − D0)
//
// Empirically established by staircase calibration (probe_c_stair) on three
// live datasets — CMP 170HX (A100 core) GPC, RTX 4060 Laptop GPC, RTX 4060
// Laptop XBAR — see memory/vfp-mode1-c-calibration. Key facts:
//
// * C is NOT a function of voltage or SKU: it is keyed to the point's
//   DEFAULT FREQUENCY and near-universal across generations and domains
//   (def 1470 MHz → C 0.3375 on all three datasets; the D0 +25/−25 flip at
//   def≈1200 and the def≈1200 dip reproduce everywhere).
// * All true C are exact k/400 (RM appears to store C as /400 fixed-point).
// * Effects quantize to the curve grid Q (15 MHz, doubling to 30 MHz at
//   each domain's P0 ceiling).
// * XBAR deviates ~+20% from the GPC prior in the 500–900 MHz def band
//   (per-domain modulation below ~900); the table below is GPC.
// * Single-point NEGATIVE offsets are clamped by the backward slope cap
//   (60 MHz/point on Ada) — the same applies to the public kHz offset;
//   real negative moves need range writes.
// ---------------------------------------------------------------------------

/// One band of the universal mode-1 prior g(def). Bands are inclusive on
/// both ends and cover the union of observed default frequencies; gaps
/// between bands were never observed live.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ClkVfGPrior {
    /// inclusive default-frequency band lower bound (MHz)
    pub def_mhz_lo: u32,
    /// inclusive default-frequency band upper bound (MHz)
    pub def_mhz_hi: u32,
    /// slope C: MHz of curve lift per mode-1 delta unit (exact k/400)
    pub c_mhz_per_delta: f64,
    /// deadzone D0 in delta units (negative = immediate response)
    pub d0_delta: f64,
}

/// Universal GPC prior g(def), merged from the CMP-170HX/4060/TU104 GPC
/// datasets (~20 def bands match exactly across Turing/Ampere/Ada,
/// including the 0.275 dip at 1200). C is generation-stable; **D0 is
/// generation-specific** — Turing's stock curve sits ~45 MHz below the
/// delta-0 baseline (D0 ≈ −75..−400), so treat prior D0 as an
/// Ada/Ampere approximation and refine per GPU. Two ±1-grid generation
/// deviations exist (def 1755-1830: Ada 0.435 vs Turing 0.44; def 1845:
/// 0.45 vs 0.46). Below ~900 MHz XBAR/HOST run hot — see
/// [`CLK_VF_FABRIC_OVERRIDES`]; validate with
/// [`crate::gpu::PhysicalGpu::clk_vf_calibrate_private`].
pub const CLK_VF_G_PRIOR: &[ClkVfGPrior] = &[
    // def_lo def_hi  C        D0
    ClkVfGPrior { def_mhz_lo: 200, def_mhz_hi: 330, c_mhz_per_delta: 0.0800, d0_delta: -8.0 },
    ClkVfGPrior { def_mhz_lo: 345, def_mhz_hi: 480, c_mhz_per_delta: 0.1125, d0_delta: 0.0 },
    ClkVfGPrior { def_mhz_lo: 495, def_mhz_hi: 510, c_mhz_per_delta: 0.1250, d0_delta: 0.0 },
    ClkVfGPrior { def_mhz_lo: 540, def_mhz_hi: 570, c_mhz_per_delta: 0.1600, d0_delta: 20.0 },
    ClkVfGPrior { def_mhz_lo: 600, def_mhz_hi: 690, c_mhz_per_delta: 0.1550, d0_delta: -17.0 },
    ClkVfGPrior { def_mhz_lo: 720, def_mhz_hi: 745, c_mhz_per_delta: 0.1750, d0_delta: 0.0 },
    // TU104-observed band
    ClkVfGPrior { def_mhz_lo: 750, def_mhz_hi: 765, c_mhz_per_delta: 0.1875, d0_delta: -240.0 },
    ClkVfGPrior { def_mhz_lo: 765, def_mhz_hi: 870, c_mhz_per_delta: 0.2025, d0_delta: -9.0 },
    ClkVfGPrior { def_mhz_lo: 915, def_mhz_hi: 915, c_mhz_per_delta: 0.2500, d0_delta: 15.0 },
    ClkVfGPrior { def_mhz_lo: 930, def_mhz_hi: 945, c_mhz_per_delta: 0.2575, d0_delta: 21.0 },
    ClkVfGPrior { def_mhz_lo: 975, def_mhz_hi: 1005, c_mhz_per_delta: 0.2625, d0_delta: 18.0 },
    ClkVfGPrior { def_mhz_lo: 1020, def_mhz_hi: 1040, c_mhz_per_delta: 0.2700, d0_delta: 19.0 },
    // main OC band: long 0.30 plateau, D0 +25
    ClkVfGPrior { def_mhz_lo: 1050, def_mhz_hi: 1185, c_mhz_per_delta: 0.3000, d0_delta: 25.0 },
    // reproducible dip (all datasets)
    ClkVfGPrior { def_mhz_lo: 1200, def_mhz_hi: 1215, c_mhz_per_delta: 0.2700, d0_delta: -14.0 },
    // 0.30 again, D0 flips to −25
    ClkVfGPrior { def_mhz_lo: 1230, def_mhz_hi: 1365, c_mhz_per_delta: 0.3000, d0_delta: -25.0 },
    ClkVfGPrior { def_mhz_lo: 1380, def_mhz_hi: 1395, c_mhz_per_delta: 0.3225, d0_delta: -20.0 },
    ClkVfGPrior { def_mhz_lo: 1410, def_mhz_hi: 1410, c_mhz_per_delta: 0.3250, d0_delta: -21.0 },
    ClkVfGPrior { def_mhz_lo: 1440, def_mhz_hi: 1440, c_mhz_per_delta: 0.3300, d0_delta: -20.0 },
    // def 1470 → 0.3375: identical on CMP 170HX, 4060 GPC and 4060 XBAR
    ClkVfGPrior { def_mhz_lo: 1470, def_mhz_hi: 1530, c_mhz_per_delta: 0.3375, d0_delta: -19.0 },
    ClkVfGPrior { def_mhz_lo: 1545, def_mhz_hi: 1620, c_mhz_per_delta: 0.3875, d0_delta: -7.0 },
    ClkVfGPrior { def_mhz_lo: 1635, def_mhz_hi: 1665, c_mhz_per_delta: 0.3950, d0_delta: -9.0 },
    ClkVfGPrior { def_mhz_lo: 1710, def_mhz_hi: 1710, c_mhz_per_delta: 0.4100, d0_delta: -4.0 },
    ClkVfGPrior { def_mhz_lo: 1725, def_mhz_hi: 1740, c_mhz_per_delta: 0.4150, d0_delta: -6.0 },
    ClkVfGPrior { def_mhz_lo: 1755, def_mhz_hi: 1830, c_mhz_per_delta: 0.4350, d0_delta: -8.0 },
    ClkVfGPrior { def_mhz_lo: 1845, def_mhz_hi: 1920, c_mhz_per_delta: 0.4500, d0_delta: -8.0 },
    ClkVfGPrior { def_mhz_lo: 1950, def_mhz_hi: 1950, c_mhz_per_delta: 0.4650, d0_delta: -6.0 },
    ClkVfGPrior { def_mhz_lo: 1965, def_mhz_hi: 1980, c_mhz_per_delta: 0.4700, d0_delta: -7.0 },
    ClkVfGPrior { def_mhz_lo: 1995, def_mhz_hi: 2025, c_mhz_per_delta: 0.4900, d0_delta: -2.0 },
    ClkVfGPrior { def_mhz_lo: 2040, def_mhz_hi: 2055, c_mhz_per_delta: 0.4850, d0_delta: -7.0 },
    ClkVfGPrior { def_mhz_lo: 2070, def_mhz_hi: 2100, c_mhz_per_delta: 0.4975, d0_delta: -6.0 },
    ClkVfGPrior { def_mhz_lo: 2115, def_mhz_hi: 2145, c_mhz_per_delta: 0.5125, d0_delta: -3.0 },
    ClkVfGPrior { def_mhz_lo: 2160, def_mhz_hi: 2175, c_mhz_per_delta: 0.5275, d0_delta: -3.0 },
    ClkVfGPrior { def_mhz_lo: 2205, def_mhz_hi: 2235, c_mhz_per_delta: 0.5375, d0_delta: -3.0 },
    ClkVfGPrior { def_mhz_lo: 2265, def_mhz_hi: 2265, c_mhz_per_delta: 0.5475, d0_delta: -3.0 },
    ClkVfGPrior { def_mhz_lo: 2280, def_mhz_hi: 2340, c_mhz_per_delta: 0.5575, d0_delta: -2.0 },
    ClkVfGPrior { def_mhz_lo: 2355, def_mhz_hi: 2415, c_mhz_per_delta: 0.5775, d0_delta: -1.0 },
    ClkVfGPrior { def_mhz_lo: 2430, def_mhz_hi: 2445, c_mhz_per_delta: 0.5625, d0_delta: -5.0 },
    // domain-ceiling band: Q doubles to 30 MHz here (observed on both the
    // 4060 GPC ceiling 2640 and XBAR ceiling 2490)
    ClkVfGPrior { def_mhz_lo: 2460, def_mhz_hi: 2595, c_mhz_per_delta: 0.6000, d0_delta: -25.0 },
    ClkVfGPrior { def_mhz_lo: 2600, def_mhz_hi: 2700, c_mhz_per_delta: 0.6250, d0_delta: -11.0 },
];

/// Domain family for the mode-1 prior: the skeleton is universal, but the
/// 500–1830 MHz def band has family-specific values — GPC (graphics)
/// differs from the fabric domains, and XBAR and HOST track each other
/// point-for-point (live-verified on a 4060: XBAR idx128-255 and HOST
/// idx259-385 agree at every shared def, including the 0.4125 dip at 1830).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum ClkVfDomainClass {
    /// GPC (graphics) — the base [`CLK_VF_G_PRIOR`] table
    #[default]
    Graphics,
    /// XBAR / HOST (fabric) — overrides from [`CLK_VF_FABRIC_OVERRIDES`]
    /// where measured, base table elsewhere
    Fabric,
}

/// Fabric-family overrides on top of the base table (XBAR+HOST agree
/// point-for-point; bands absent here fall through to [`CLK_VF_G_PRIOR`]).
pub const CLK_VF_FABRIC_OVERRIDES: &[ClkVfGPrior] = &[
    // def_lo def_hi  C        D0
    ClkVfGPrior { def_mhz_lo: 510, def_mhz_hi: 570, c_mhz_per_delta: 0.2000, d0_delta: 62.0 },
    ClkVfGPrior { def_mhz_lo: 765, def_mhz_hi: 795, c_mhz_per_delta: 0.2250, d0_delta: 25.0 },
    ClkVfGPrior { def_mhz_lo: 825, def_mhz_hi: 900, c_mhz_per_delta: 0.2400, d0_delta: 19.0 },
    ClkVfGPrior { def_mhz_lo: 915, def_mhz_hi: 945, c_mhz_per_delta: 0.2500, d0_delta: 15.0 },
    ClkVfGPrior { def_mhz_lo: 1530, def_mhz_hi: 1620, c_mhz_per_delta: 0.3500, d0_delta: -18.0 },
    ClkVfGPrior { def_mhz_lo: 1635, def_mhz_hi: 1710, c_mhz_per_delta: 0.3975, d0_delta: -8.0 },
    ClkVfGPrior { def_mhz_lo: 1725, def_mhz_hi: 1825, c_mhz_per_delta: 0.4250, d0_delta: -3.0 },
    // reproducible fabric dip at 1830 (both XBAR and HOST)
    ClkVfGPrior { def_mhz_lo: 1830, def_mhz_hi: 1840, c_mhz_per_delta: 0.4125, d0_delta: -14.0 },
    // HOST ceiling 2250 (Q stays 15 there — no ceiling doubling on HOST)
    ClkVfGPrior { def_mhz_lo: 2250, def_mhz_hi: 2320, c_mhz_per_delta: 0.5700, d0_delta: -1.0 },
];

/// Look up the universal mode-1 prior (C, D0) for a point with this
/// default frequency. Pure — no driver IO. Measured bands first; def
/// values outside the table fall back to the first-order rule
/// **C ≈ def/4000** (live-observed across Pascal–Ada: 999 MHz → 0.25,
/// 1822 → 0.4587, 1890 → 0.4526; the k/400 grid implies k ≈ def/10),
/// accurate to ~±10% — refine with a sparse sweep. D0 prior is 0: at
/// stock it measures ≈0 on every generation; a large fitted |D0| means
/// the curve already carried offsets when calibrated (calibrate from
/// stock). For XBAR/HOST points prefer [`clk_vf_g_prior_class`] with
/// [`ClkVfDomainClass::Fabric`].
pub fn clk_vf_g_prior(def_mhz: u32) -> Option<(f64, f64)> {
    if def_mhz < 200 {
        return None;
    }
    Some(
        CLK_VF_G_PRIOR
            .iter()
            .find(|e| def_mhz >= e.def_mhz_lo && def_mhz <= e.def_mhz_hi)
            .map(|e| (e.c_mhz_per_delta, e.d0_delta))
            .unwrap_or_else(|| {
                // piecewise refinement of the rule (fit over all exact
                // bands): K = def − 4000C flips sign near def 1250 —
                // high band (def−72)/4000 (≈0.96·def/4000), low band
                // (def+30)/4000, mid zone plain def/4000 (the def≈1200
                // dip sits inside measured bands anyway)
                if def_mhz >= 1450 {
                    ((def_mhz - 72) as f64 / 4000.0, 0.0)
                } else if def_mhz <= 1100 {
                    ((def_mhz + 30) as f64 / 4000.0, 0.0)
                } else {
                    (def_mhz as f64 / 4000.0, 0.0)
                }
            }),
    )
}

/// Class-aware prior: fabric domains (XBAR/HOST) take overrides from
/// [`CLK_VF_FABRIC_OVERRIDES`] first, then fall through to the base table.
pub fn clk_vf_g_prior_class(
    def_mhz: u32,
    class: ClkVfDomainClass,
) -> Option<(f64, f64)> {
    match class {
        ClkVfDomainClass::Graphics => clk_vf_g_prior(def_mhz),
        ClkVfDomainClass::Fabric => CLK_VF_FABRIC_OVERRIDES
            .iter()
            .chain(CLK_VF_G_PRIOR.iter())
            .find(|e| def_mhz >= e.def_mhz_lo && def_mhz <= e.def_mhz_hi)
            .map(|e| (e.c_mhz_per_delta, e.d0_delta)),
    }
}

/// Predicted curve lift (MHz) of a mode-1 `delta` at this default
/// frequency, per the prior for `class` (Graphics for GPC, Fabric for
/// XBAR/HOST). Negative results clamp to 0 (backward moves are
/// slope-capped anyway).
pub fn clk_vf_effect_for_delta(
    def_mhz: u32,
    delta: i32,
    class: ClkVfDomainClass,
) -> Option<f64> {
    let (c, d0) = clk_vf_g_prior_class(def_mhz, class)?;
    Some(((delta as f64 - d0) * c).max(0.0))
}

/// Mode-1 `delta` that lifts a point with this default frequency by
/// `target_mhz`, per the prior for `class`. Positive targets only —
/// negative single-point offsets are clamped by the backward slope cap and
/// need range writes instead. Refine with a measured table from
/// [`crate::gpu::PhysicalGpu::clk_vf_calibrate_private`] when the prior is
/// off (new silicon, unmeasured def bands).
pub fn clk_vf_delta_for_target(
    def_mhz: u32,
    target_mhz: f64,
    class: ClkVfDomainClass,
) -> Option<i32> {
    if !(target_mhz.is_finite() && target_mhz > 0.0) {
        return None;
    }
    let (c, d0) = clk_vf_g_prior_class(def_mhz, class)?;
    if c <= 0.0 {
        return None;
    }
    let delta = target_mhz / c + d0;
    Some(delta.clamp(0.0, 1000.0) as i32)
}

/// One staircase-calibration sample: (mode-1 delta, measured effect MHz),
/// where effect = STATUS current MHz − default MHz.
pub type ClkVfStairSample = (i64, i64);

/// Result of the exact staircase fit.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ClkVfStairFit {
    /// slope C (MHz per delta unit); snapped to the nearest k/400 inside
    /// the feasible interval when one exists
    pub c: f64,
    /// exact feasible lower bound on C
    pub c_lo: f64,
    /// exact feasible upper bound on C
    pub c_hi: f64,
    /// deadzone D0 in delta units (B/C); negative = immediate response
    pub d0: f64,
}

/// Exact staircase fit over saturation-trimmed samples. Each sample gives
/// the linear constraint `E_i ≤ C·d_i − B < E_i + Q` (B = C·D0); pairwise
/// subtraction eliminates B and yields exact rational bounds on C:
/// `C·(d_i − d_j) ∈ (E_i − E_j − Q, E_i − E_j + Q)`. The point estimate is
/// the interval midpoint, snapped to the nearest k/400 inside the interval
/// (every true C observed so far is k/400). Returns `None` when no (C, B)
/// satisfies all samples (inconsistent staircase — usually load flutter).
pub fn clk_vf_stair_fit(samples: &[ClkVfStairSample], q_mhz: i64) -> Option<ClkVfStairFit> {
    if samples.len() < 2 {
        return None;
    }
    let q = q_mhz as f64;
    let mut lo = 0.0f64;
    let mut hi = f64::INFINITY;
    for i in 0..samples.len() {
        for j in 0..samples.len() {
            if i == j {
                continue;
            }
            let (di, ei) = samples[i];
            let (dj, ej) = samples[j];
            let dd = (di - dj) as f64;
            let de = (ei - ej) as f64;
            if dd > 0.0 {
                lo = lo.max((de - q) / dd);
                hi = hi.min((de + q) / dd);
            } else if dd < 0.0 {
                lo = lo.max((de + q) / dd);
                hi = hi.min((de - q) / dd);
            }
        }
    }
    if !(lo < hi) {
        return None;
    }
    let mut c = (lo + hi) / 2.0;
    let snapped = (c * 400.0).round() / 400.0;
    if snapped >= lo && snapped <= hi {
        c = snapped;
    }
    let mut b_hi = f64::MAX;
    let mut b_lo = f64::MIN;
    for &(d, e) in samples {
        let x = c * d as f64 - e as f64;
        b_hi = b_hi.min(x);
        b_lo = b_lo.max(x - q);
    }
    if !(b_lo < b_hi) {
        return None;
    }
    Some(ClkVfStairFit { c, c_lo: lo, c_hi: hi, d0: (b_lo + b_hi) / 2.0 / c })
}

/// Per-point outcome of a sparse mode-1 calibration sweep.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct ClkVfCalPoint {
    /// 0 or 1
    pub bank: u8,
    /// point index within the bank
    pub idx: u16,
    /// default frequency (MHz, Pascal type-1 already halved)
    pub def_mhz: u32,
    /// grid voltage (mV; 0 when the record lacks the field)
    pub volt_mv: u32,
    pub kind: ClkVfCalKind,
}

/// What the ladder measured at one point.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClkVfCalKind {
    /// staircase fit succeeded; compare `fit.c` against
    /// [`clk_vf_g_prior`] to validate the prior or measure domain modulation
    Fitted {
        fit: ClkVfStairFit,
        /// detected effect quantum (15 MHz, or 30 at domain ceilings)
        q_mhz: i64,
        /// samples used after trimming
        n_used: usize,
    },
    /// flat response across the ladder — pinned at the P0 ceiling or below
    /// the pstate floor (real information, but not a C)
    Pinned { flat_effect_mhz: i64 },
    /// STATUS current-frequency field empty (all Pascal type-1 records):
    /// mode-1 writes DO take effect there, but the effect must be measured
    /// (MEASURE_FREQ) rather than read from STATUS — source not yet wired
    CurAbsent,
    /// samples mutually inconsistent (load flutter mid-ladder)
    Unstable,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct VfPoint<T> {
    pub frequency: T,
    pub voltage: Microvolts,
}

impl<T: Default + PartialEq> VfPoint<T> {
    pub fn is_empty(&self) -> bool {
        self.voltage.0 == 0 && self.frequency == Default::default()
    }
}

impl<T> VfPoint<T> {
    pub fn from_entry<U>(e: VfPoint<U>) -> Self
    where
        T: From<U>,
    {
        VfPoint {
            frequency: e.frequency.into(),
            voltage: e.voltage,
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct VfpEntry<K> {
    pub point_type: VfPointType,
    pub current: VfPoint<K>,
    pub default: VfPoint<K>,
    pub overclocked: VfPoint<K>,
}

impl<K: Default> Default for VfpEntry<K> {
    fn default() -> Self {
        Self {
            point_type: VfPointType::Prog,
            current: Default::default(),
            default: Default::default(),
            overclocked: Default::default(),
        }
    }
}

impl<T> VfpEntry<T> {
    pub fn from_entry<K>(e: VfpEntry<K>) -> Self
    where
        T: From<K>,
    {
        VfpEntry {
            point_type: e.point_type,
            current: VfPoint::from_entry(e.current),
            default: VfPoint::from_entry(e.default),
            overclocked: VfPoint::from_entry(e.overclocked),
        }
    }
}

impl<T: Default + PartialEq> VfpEntry<T> {
    pub fn configured(&self) -> &VfPoint<T> {
        match self.overclocked.is_empty() {
            false => &self.overclocked,
            true => &self.current,
        }
    }

    pub fn default(&self) -> Option<&VfPoint<T>> {
        match self.default.is_empty() {
            false => Some(&self.default),
            true => None,
        }
    }
}

impl<T: Default> From<VfPoint<T>> for VfpEntry<T> {
    fn from(current: VfPoint<T>) -> Self {
        Self {
            point_type: VfPointType::Prog,
            current,
            default: Default::default(),
            overclocked: Default::default(),
        }
    }
}

impl RawConversion for power::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINT {
    type Target = VfPoint<u32>;
    type Error = Infallible;

    #[allow(non_snake_case)]
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok(VfPoint {
            frequency: self.freq_kHz,
            voltage: Microvolts(self.voltage_uV),
        })
    }
}

impl RawConversion for power::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_STATUS_V1 {
    type Target = VfPoint<u32>;
    type Error = sys::ArgumentRangeError;

    #[allow(non_snake_case)]
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        let power::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_STATUS_V1 {
            clock_type: _,
            point,
            unknown: _,
        } = *self;
        point.convert_raw().map_err(Into::into)
    }
}

impl RawConversion for power::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_STATUS_V3 {
    type Target = VfpEntry<u32>;
    type Error = sys::ArgumentRangeError;

    #[allow(non_snake_case)]
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        let power::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_STATUS_V3 {
            clock_type,
            point,
            point_default,
            point_overclocked,
            ..
        } = *self;
        Ok(VfpEntry {
            point_type: VfPointType::from_raw(clock_type as i32)?,
            current: point.convert_raw()?,
            default: point_default.convert_raw()?,
            overclocked: point_overclocked.convert_raw()?,
        })
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct VfpCurve {
    pub points: BTreeMap<ClockDomain, Vec<(usize, VfpEntry<Kilohertz>)>>,
}

impl VfpCurve {
    pub fn from_raw_v3(
        raw: &power::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_V3,
        info: &VfpInfo,
    ) -> crate::Result<Self> {
        Ok(Self {
            points: info
                .domains
                .domains
                .iter()
                .map(|d| {
                    info.index(d.domain, &raw.entries[..])
                        .map(|(i, p)| p.convert_raw().map(|p| (i, VfpEntry::from_entry(p))))
                        .collect::<Result<Vec<_>, _>>()
                        .map(|p| (d.domain, p))
                })
                .collect::<Result<_, _>>()?,
        })
    }

    pub fn from_raw_v1(
        raw: &power::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_V1,
        info: &VfpInfo,
    ) -> crate::Result<Self> {
        Ok(Self {
            points: info
                .domains
                .domains
                .iter()
                .map(|d| {
                    info.index(d.domain, &raw.entries[..])
                        .map(|(i, p)| p.convert_raw().map(|p| (i, VfPoint::from_entry(p).into())))
                        .collect::<Result<Vec<_>, _>>()
                        .map(|p| (d.domain, p))
                })
                .collect::<Result<_, _>>()?,
        })
    }

    pub fn from_raw(
        raw: &power::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS,
        info: &VfpInfo,
    ) -> crate::Result<Self> {
        Self::from_raw_v3(raw, info)
    }
}

impl RawConversion for power::private::NV_GPU_CLIENT_VOLT_RAILS_STATUS_V1 {
    type Target = Microvolts;
    type Error = sys::ArgumentRangeError;

    #[allow(non_snake_case)]
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        match *self {
            power::private::NV_GPU_CLIENT_VOLT_RAILS_STATUS_V1 {
                version: _,
                flags: 0,
                ref zero,
                value_uV,
                ref unknown,
            } if zero.all_zero() && unknown.all_zero() => Ok(Microvolts(value_uV)),
            _ => Err(sys::ArgumentRangeError),
        }
    }
}

impl RawConversion for power::private::NV_GPU_CLIENT_VOLT_RAILS_CONTROL_V1 {
    type Target = Percentage;
    type Error = sys::ArgumentRangeError;

    #[allow(non_snake_case)]
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        match *self {
            power::private::NV_GPU_CLIENT_VOLT_RAILS_CONTROL {
                version: _,
                percent,
                ref unknown,
            } if unknown.all_zero() => Percentage::from_raw(percent),
            _ => Err(sys::ArgumentRangeError),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct PowerInfoEntry {
    pub policy_id: power::private::NV_GPU_CLIENT_POWER_POLICIES_POLICY_ID,
    pub range: Range<Percentage1000>,
    pub default_limit: Percentage1000,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct PowerInfo {
    pub valid: bool,
    pub entries: Vec<PowerInfoEntry>,
}

impl RawConversion for power::private::NV_GPU_CLIENT_POWER_POLICIES_INFO_ENTRY_V1 {
    type Target = PowerInfoEntry;
    type Error = sys::ArgumentRangeError;

    #[allow(non_snake_case)]
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        let power::private::NV_GPU_CLIENT_POWER_POLICIES_INFO_ENTRY_V1 {
            policy_id,
            min_power,
            def_power,
            max_power,
            ..
        } = *self;
        Ok(PowerInfoEntry {
            policy_id,
            range: Range {
                min: Percentage1000(min_power),
                max: Percentage1000(max_power),
            },
            default_limit: Percentage1000(def_power),
        })
    }
}

impl RawConversion for power::private::NV_GPU_CLIENT_POWER_POLICIES_INFO_ENTRY_V2 {
    type Target = PowerInfoEntry;
    type Error = sys::ArgumentRangeError;

    #[allow(non_snake_case)]
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        let power::private::NV_GPU_CLIENT_POWER_POLICIES_INFO_ENTRY_V2 {
            policy_id,
            min_power,
            def_power,
            max_power,
            ..
        } = *self;
        Ok(PowerInfoEntry {
            policy_id,
            range: Range {
                min: Percentage1000(min_power),
                max: Percentage1000(max_power),
            },
            default_limit: Percentage1000(def_power),
        })
    }
}

impl RawConversion for power::private::NV_GPU_CLIENT_POWER_POLICIES_INFO {
    type Target = PowerInfo;
    type Error = sys::ArgumentRangeError;

    #[allow(non_snake_case)]
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok(PowerInfo {
            valid: self.valid != 0,
            entries: self
                .entries()
                .iter()
                .map(RawConversion::convert_raw)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl RawConversion for power::private::NV_GPU_CLIENT_POWER_TOPOLOGY_STATUS_ENTRY {
    type Target = (PowerTopologyChannelId, Percentage1000);
    type Error = sys::ArgumentRangeError;

    #[allow(non_snake_case)]
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        let power::private::NV_GPU_CLIENT_POWER_TOPOLOGY_STATUS_ENTRY {
            channel,
            power,
            unknown0: _,
            unknown1: _,
        } = *self;
        Ok((channel.try_into()?, Percentage1000(power)))
    }
}

impl RawConversion for power::private::NV_GPU_CLIENT_POWER_TOPOLOGY_STATUS {
    type Target = BTreeMap<PowerTopologyChannelId, Percentage1000>;
    type Error = sys::ArgumentRangeError;

    #[allow(non_snake_case)]
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        self.entries()
            .iter()
            .map(RawConversion::convert_raw)
            .collect()
    }
}

impl RawConversion for power::private::NV_GPU_CLIENT_POWER_POLICIES_STATUS_ENTRY_V1 {
    type Target = Percentage1000;
    type Error = sys::ArgumentRangeError;

    #[allow(non_snake_case)]
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        let power::private::NV_GPU_CLIENT_POWER_POLICIES_STATUS_ENTRY_V1 {
            policy_id: _,
            power_target,
            ..
        } = *self;
        Ok(Percentage1000(power_target))
    }
}

impl RawConversion for power::private::NV_GPU_CLIENT_POWER_POLICIES_STATUS_ENTRY_V2 {
    type Target = Percentage1000;
    type Error = sys::ArgumentRangeError;

    #[allow(non_snake_case)]
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        let power::private::NV_GPU_CLIENT_POWER_POLICIES_STATUS_ENTRY_V2 {
            policy_id: _,
            power_target,
            ..
        } = *self;
        Ok(Percentage1000(power_target))
    }
}

impl RawConversion for power::private::NV_GPU_CLIENT_POWER_TOPOLOGY_INFO {
    type Target = Vec<PowerTopologyChannelId>;
    type Error = sys::ArgumentRangeError;

    #[allow(non_snake_case)]
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        self.channels()
            .iter()
            .copied()
            .map(|raw| raw.try_into())
            .collect()
    }
}

impl RawConversion for power::private::NV_GPU_CLIENT_POWER_POLICIES_STATUS {
    type Target = Vec<Percentage1000>;
    type Error = sys::ArgumentRangeError;

    #[allow(non_snake_case)]
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        self.entries[..self.count as usize]
            .iter()
            .map(RawConversion::convert_raw)
            .collect()
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum ClockLockValue {
    Frequency(Kilohertz),
    Voltage(Microvolts),
}

impl ClockLockValue {
    pub fn value(&self) -> u32 {
        match self {
            ClockLockValue::Frequency(v) => v.0,
            ClockLockValue::Voltage(v) => v.0,
        }
    }

    pub fn voltage(&self) -> Option<Microvolts> {
        match self {
            &ClockLockValue::Voltage(v) => Some(v),
            _ => None,
        }
    }

    pub fn frequency(&self) -> Option<Kilohertz> {
        match self {
            &ClockLockValue::Frequency(v) => Some(v),
            _ => None,
        }
    }

    pub fn from_raw(
        raw: &clock::private::NV_GPU_PERF_CLIENT_LIMITS_ENTRY,
    ) -> Result<Option<Self>, sys::ArgumentRangeError> {
        Ok(match clock::private::ClockLockMode::from_raw(raw.mode)? {
            clock::private::ClockLockMode::None => None,
            clock::private::ClockLockMode::ManualVoltage => {
                Some(ClockLockValue::Voltage(Microvolts(raw.value)))
            }
            clock::private::ClockLockMode::ManualFrequency => {
                Some(ClockLockValue::Frequency(Kilohertz(raw.value)))
            }
            // PstateSelect (mode 1) is a P-State pin, not a freq/voltage lock;
            // it's not a VFP lock value, so report None (no lock to reset here).
            clock::private::ClockLockMode::PstateSelect => None,
            // ClockLockMode is #[non_exhaustive]; any future mode isn't a
            // freq/voltage VFP lock either, so treat it as "no lock to reset".
            _ => None,
        })
    }
}

impl fmt::Display for ClockLockValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClockLockValue::Voltage(v) => fmt::Display::fmt(v, f),
            ClockLockValue::Frequency(v) => fmt::Display::fmt(v, f),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct ClockLockEntry {
    pub limit: PerfLimitId,
    pub lock_value: Option<ClockLockValue>,
    pub clock: ClockDomain,
}

impl RawConversion for clock::private::NV_GPU_PERF_CLIENT_LIMITS_ENTRY {
    type Target = ClockLockEntry;
    type Error = sys::ArgumentRangeError;

    #[allow(non_snake_case)]
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        let clock::private::NV_GPU_PERF_CLIENT_LIMITS_ENTRY {
            id,
            mode: _,
            value: _,
            clock_id,
            ..
        } = *self;
        Ok(ClockLockEntry {
            limit: id.try_into()?,
            clock: clock_id.try_into()?,
            lock_value: ClockLockValue::from_raw(self)?,
        })
    }
}

impl RawConversion for clock::private::NV_GPU_PERF_CLIENT_LIMITS {
    type Target = Vec<ClockLockEntry>;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        if self.flags != 0 {
            Err(sys::ArgumentRangeError)
        } else {
            self.entries()
                .iter()
                .map(RawConversion::convert_raw)
                .collect()
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct PerfInfo {
    pub max_unknown: u32,
    pub limits: PerfFlags,
}

impl RawConversion for power::private::NV_GPU_PERF_POLICIES_INFO_PARAMS {
    type Target = PerfInfo;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        // TODO: check padding
        Ok(PerfInfo {
            max_unknown: self.maxUnknown,
            limits: PerfFlags::from_bits(self.limitSupport).ok_or(sys::ArgumentRangeError)?,
        })
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct PerfStatus {
    pub unknown: u32,
    pub limits: PerfFlags,
}

impl RawConversion for power::private::NV_GPU_PERF_POLICIES_STATUS_PARAMS {
    type Target = PerfStatus;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        // TODO: check padding
        match *self {
            power::private::NV_GPU_PERF_POLICIES_STATUS_PARAMS {
                flags: 0,
                limits,
                zero0: 0,
                unknown,
                zero1: 0,
                ..
            } => Ok(PerfStatus {
                unknown,
                limits: PerfFlags::from_bits(limits).ok_or(sys::ArgumentRangeError)?,
            }),
            _ => Err(sys::ArgumentRangeError),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct VoltageEntry {
    pub voltage: Microvolts,
}

impl RawConversion for power::private::NV_VOLT_TABLE_ENTRY {
    type Target = VoltageEntry;
    type Error = Infallible;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok(VoltageEntry {
            voltage: Microvolts(self.voltage_uV),
        })
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct VoltageTable {
    pub flags: u32,
    pub entries: Vec<VoltageEntry>,
}

impl RawConversion for power::private::NV_VOLT_TABLE {
    type Target = VoltageTable;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok(VoltageTable {
            flags: self.flags,
            entries: self
                .entries()
                .iter()
                .map(RawConversion::convert_raw)
                .collect::<Result<_, _>>()?,
        })
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct VoltageStatus {
    pub flags: u32,
    pub unknown0: u32,
    pub voltage: Microvolts,
    pub count: u32,
}

impl RawConversion for power::private::NV_VOLT_STATUS {
    type Target = VoltageStatus;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok(VoltageStatus {
            flags: self.flags,
            count: self.count,
            unknown0: self.unknown,
            voltage: Microvolts(self.value_uV),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::gpu::clock::private::NV_GPU_CLOCK_INFO_V2;

    /// Universal prior: def 1470 → C 0.3375 must hold (identical on all
    /// live datasets), gaps between bands return None, fabric overrides
    /// take precedence in their bands, and the delta→effect→delta
    /// prediction round-trips within one grid step.
    #[test]
    fn clk_vf_prior_lookup_and_prediction() {
        let (c, _) = clk_vf_g_prior(1470).unwrap();
        assert!((c - 0.3375).abs() < 1e-9);
        let (c, _) = clk_vf_g_prior(1050).unwrap();
        assert!((c - 0.30).abs() < 1e-9);
        // outside the measured table: piecewise first-order rule
        let (c, d0) = clk_vf_g_prior(340).unwrap();
        assert!((c - 0.0925).abs() < 1e-9); // (340+30)/4000
        assert!(d0.abs() < 1e-9);
        assert!(clk_vf_g_prior(100).is_none()); // below any real curve

        // fabric overrides: def 840 is 0.24 for XBAR/HOST (base 0.2025)
        let (g, _) = clk_vf_g_prior_class(840, ClkVfDomainClass::Graphics).unwrap();
        let (f, _) = clk_vf_g_prior_class(840, ClkVfDomainClass::Fabric).unwrap();
        assert!((g - 0.2025).abs() < 1e-9);
        assert!((f - 0.2400).abs() < 1e-9);
        // outside override bands fabric falls through to the base table
        let (f, _) = clk_vf_g_prior_class(1470, ClkVfDomainClass::Fabric).unwrap();
        assert!((f - 0.3375).abs() < 1e-9);

        let d = clk_vf_delta_for_target(1300, 90.0, ClkVfDomainClass::Graphics).unwrap();
        let e = clk_vf_effect_for_delta(1300, d, ClkVfDomainClass::Graphics).unwrap();
        assert!((e - 90.0).abs() <= 15.0 * 1.01, "round-trip {e}");
        assert!(clk_vf_delta_for_target(1300, -5.0, ClkVfDomainClass::Graphics).is_none());
        // fallback-rule prediction also yields a delta now
        assert!(clk_vf_delta_for_target(340, 90.0, ClkVfDomainClass::Graphics).is_some());
    }

    /// Synthetic CMP-shaped staircase (C=0.30, D0=25, Q=15): the exact
    /// pairwise fit must recover 0.30 with the true value inside [lo, hi].
    /// Ladder starts at d=50 — below D0 the effect clamps to 0, and the
    /// real calibrator trims that leading flat before fitting (a clamped
    /// (0,0) sample would contradict the pure-floor constraint model).
    #[test]
    fn clk_vf_stair_fit_recovers_exact_c() {
        let samples: Vec<ClkVfStairSample> = [50i64, 100, 150, 200, 250, 300, 350, 400]
            .iter()
            .map(|&d| {
                let x = 0.30 * (d as f64 - 25.0);
                (d, 15 * (x.max(0.0) / 15.0).floor() as i64)
            })
            .collect();
        let fit = clk_vf_stair_fit(&samples, 15).unwrap();
        assert!((fit.c - 0.30).abs() < 1e-9, "c={}", fit.c);
        assert!(fit.c_lo <= 0.30 && fit.c_hi >= 0.30);
        assert!((fit.d0 - 25.0).abs() <= 25.0, "d0={}", fit.d0);
        // inconsistent samples must fail closed
        assert!(clk_vf_stair_fit(&[(0, 0), (100, 90), (200, 0)], 15).is_none());
    }

    /// Build a GetAllClocks V2 buffer with a fabricated per-domain frequency
    /// map, then confirm `all_clocks_from_raw` surfaces every present (non-zero)
    /// domain keyed by `ClockDomainId` — including the internal fabric clocks
    /// (Xbar/crossbar, Sys, Hub, …) that `EffectiveClocks` deliberately omits.
    #[test]
    fn all_clocks_surfaces_all_32_domains() {
        let mut raw = NV_GPU_CLOCK_INFO_V2::default();
        // Populate a handful of domains with plausible fabric-clock kHz. Index
        // by the ClockDomainId discriminant (Gpc=0, Xbar=1, …, Pciegen=31).
        let samples: &[(ClockDomainId, u32)] = &[
            (ClockDomainId::Gpc, 2_100_000),  // Graphics (0)
            (ClockDomainId::Xbar, 1_800_000), // Crossbar (1)
            (ClockDomainId::Sys, 900_000),
            (ClockDomainId::Hub, 600_000),
            (ClockDomainId::M, 7_500_000), // Memory (4)
            (ClockDomainId::Host, 100_000),
            (ClockDomainId::Disp, 67_500),
            (ClockDomainId::Hotclk, 0),      // zero -> dropped
            (ClockDomainId::Pciegen, 8_000), // PCIe gen NUMBER -> filtered out
        ];
        for (dom, freq) in samples {
            raw.extended_domain[dom.raw() as usize].effective_frequency = *freq;
        }

        let all = all_clocks_from_raw(&raw);

        // Zero-frequency domains are dropped; the rest are keyed by ClockDomainId.
        assert_eq!(all.get(&ClockDomainId::Gpc), Some(&Kilohertz(2_100_000)));
        assert_eq!(all.get(&ClockDomainId::Xbar), Some(&Kilohertz(1_800_000)));
        assert_eq!(all.get(&ClockDomainId::M), Some(&Kilohertz(7_500_000)));
        // Pciegen is filtered out (its value is a PCIe gen number, not kHz).
        assert!(!all.contains_key(&ClockDomainId::Pciegen));
        // Hotclk was zero -> absent.
        assert!(!all.contains_key(&ClockDomainId::Hotclk));
        // The 4-public-clock EffectiveClocks conversion would only return
        // Gpc(0)/M(4); all_clocks additionally returns the fabric clocks.
        assert!(all.contains_key(&ClockDomainId::Xbar));
        assert!(all.contains_key(&ClockDomainId::Sys));
        assert!(all.contains_key(&ClockDomainId::Hub));
    }

    /// `all_clocks_from_raw` over an all-zero buffer yields an empty map (no
    /// domains present), never panics on the 32-element array indexing.
    #[test]
    fn all_clocks_empty_when_nothing_present() {
        let raw = NV_GPU_CLOCK_INFO_V2::default();
        assert!(all_clocks_from_raw(&raw).is_empty());
    }
}
