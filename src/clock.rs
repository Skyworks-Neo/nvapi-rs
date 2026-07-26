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
pub type AllClocks = BTreeMap<ClockDomainId, Kilohertz>;

/// Extract all 32 clock domains from a raw GetAllClocks V2 result (companion
/// to `NV_GPU_CLOCK_INFO_V2`'s `EffectiveClocks` conversion, which only reads
/// the 4 public domains). Returns every present domain keyed by
/// [`ClockDomainId`].
pub fn all_clocks_from_raw(
    raw: &clock::private::NV_GPU_CLOCK_INFO_V2,
) -> AllClocks {
    ClockDomainId::values()
        .map(|id| (id, raw.extended_domain[id.raw() as usize].effective_frequency))
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
            _ => return Err(sys::ArgumentRangeError),
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
            (ClockDomainId::Gpc, 2_100_000),      // Graphics (0)
            (ClockDomainId::Xbar, 1_800_000),     // Crossbar (1)
            (ClockDomainId::Sys, 900_000),
            (ClockDomainId::Hub, 600_000),
            (ClockDomainId::M, 7_500_000),        // Memory (4)
            (ClockDomainId::Host, 100_000),
            (ClockDomainId::Disp, 67_500),
            (ClockDomainId::Hotclk, 0),           // zero -> dropped
            (ClockDomainId::Pciegen, 8_000),
        ];
        for (dom, freq) in samples {
            raw.extended_domain[dom.raw() as usize].effective_frequency = *freq;
        }

        let all = all_clocks_from_raw(&raw);

        // Zero-frequency domains are dropped; the rest are keyed by ClockDomainId.
        assert_eq!(all.get(&ClockDomainId::Gpc), Some(&Kilohertz(2_100_000)));
        assert_eq!(all.get(&ClockDomainId::Xbar), Some(&Kilohertz(1_800_000)));
        assert_eq!(all.get(&ClockDomainId::M), Some(&Kilohertz(7_500_000)));
        assert_eq!(all.get(&ClockDomainId::Pciegen), Some(&Kilohertz(8_000)));
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
