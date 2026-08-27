use crate::sys;
use crate::sys::gpu::{cooler, thermal};
use crate::sys::types::counted;
use crate::types::{Celsius, CelsiusShifted, Kilohertz, Percentage, Range, RawConversion, Rpm};
use log::trace;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fmt;

fn convert_entries<T, U>(entries: &[T], label: &(impl fmt::Debug + ?Sized)) -> Result<U, T::Error>
where
    T: RawConversion,
    U: FromIterator<T::Target>,
{
    trace!("convert_raw({:#?})", label);
    entries.iter().map(RawConversion::convert_raw).collect()
}

pub use sys::gpu::cooler::undocumented::{FanArbiterInfoFlags, FanCoolerId};
pub use sys::gpu::thermal::undocumented::ThermalPolicyId;
pub use sys::gpu::thermal::{ThermalController, ThermalTarget};

#[derive(Debug, Copy, Clone)]
pub struct Sensor {
    pub controller: ThermalController,
    pub default_temperature_range: Range<Celsius>,
    pub current_temperature: Celsius,
    pub target: ThermalTarget,
}

impl RawConversion for thermal::NV_GPU_THERMAL_SETTINGS_SENSOR {
    type Target = Sensor;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok(Sensor {
            controller: ThermalController::try_from(self.controller)?,
            default_temperature_range: Range {
                min: Celsius(self.defaultMinTemp),
                max: Celsius(self.defaultMaxTemp),
            },
            current_temperature: Celsius(self.currentTemp),
            target: ThermalTarget::try_from(self.target)?,
        })
    }
}

impl RawConversion for thermal::NV_GPU_THERMAL_SETTINGS {
    type Target = Vec<Sensor>;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        counted(&*self.sensor, self.count as usize)
            .iter()
            .map(RawConversion::convert_raw)
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct ThermalInfo {
    pub policy: ThermalPolicyId,
    pub unknown: u32,
    pub pff: Option<PffCurve>,
    pub temperature_range: Range<CelsiusShifted>,
    pub default_temperature: CelsiusShifted,
    pub default_flags: u32,
}

impl RawConversion for thermal::undocumented::NV_GPU_CLIENT_THERMAL_POLICIES_INFO_ENTRY_V2 {
    type Target = ThermalInfo;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok(ThermalInfo {
            policy: self.policy_id.try_into()?,
            unknown: self.unknown,
            temperature_range: Range {
                min: CelsiusShifted(self.minTemp),
                max: CelsiusShifted(self.maxTemp),
            },
            default_temperature: CelsiusShifted(self.defaultTemp),
            default_flags: self.defaultFlags,
            pff: None,
        })
    }
}

impl RawConversion for thermal::undocumented::NV_GPU_CLIENT_THERMAL_POLICY_INFO_V3 {
    type Target = ThermalInfo;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok(ThermalInfo {
            policy: self.policy_id.try_into()?,
            unknown: self.unknown,
            temperature_range: Range {
                min: CelsiusShifted(self.minTemp),
                max: CelsiusShifted(self.maxTemp),
            },
            default_temperature: CelsiusShifted(self.defaultTemp),
            default_flags: self.defaultFlags,
            pff: if self.has_pff() {
                Some(self.pff_curve.convert_raw()?)
            } else {
                None
            },
        })
    }
}

impl RawConversion for thermal::undocumented::NV_GPU_CLIENT_THERMAL_POLICIES_INFO_V2 {
    type Target = Vec<ThermalInfo>;
    type Error = sys::ArgumentRangeError;
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        convert_entries(self.entries(), self)
    }
}

impl RawConversion for thermal::undocumented::NV_GPU_CLIENT_THERMAL_POLICIES_INFO_V3 {
    type Target = Vec<ThermalInfo>;
    type Error = sys::ArgumentRangeError;
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        convert_entries(self.entries(), self)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct ThermalLimit {
    pub policy: ThermalPolicyId,
    pub value: CelsiusShifted,
    pub remove_tdp_limit: bool,
    pub pff: Option<PffStatus>,
}

impl ThermalLimit {
    pub fn to_raw(&self) -> thermal::undocumented::NV_GPU_CLIENT_THERMAL_POLICY_STATUS_V3 {
        let mut entry = thermal::undocumented::NV_GPU_CLIENT_THERMAL_POLICY_STATUS_V3 {
            policy_id: self.policy.into(),
            temp_limit_C: self.value.0 as _,
            remove_tdp_limit: self.remove_tdp_limit.into(),
            ..Default::default()
        };
        if let Some(pff) = &self.pff {
            entry.set_pff(true);
            let (curve, values) = pff.to_raw();
            entry.pff_curve = curve;
            entry.pff_freqs = values;
        }
        entry
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct PffStatus {
    pub curve: PffCurve,
    pub values: Vec<Kilohertz>,
}

impl PffStatus {
    pub fn points<'a>(&'a self) -> impl Iterator<Item = PffPoint> + 'a {
        self.curve
            .points
            .iter()
            .copied()
            .zip(self.values.iter().copied())
            .map(|(point, value)| PffPoint {
                x: point.x,
                y: value,
            })
    }

    pub fn curve(&self) -> PffCurve {
        self.points().collect()
    }

    pub fn to_raw(&self) -> (thermal::undocumented::NV_GPU_CLIENT_PFF_CURVE_V1, [u32; 3]) {
        let mut values = [0u32; 3];
        for (dest, src) in values.iter_mut().zip(&self.values) {
            *dest = src.0 as _;
        }
        (self.curve.to_raw(), values)
    }
}

impl RawConversion for thermal::undocumented::NV_GPU_CLIENT_THERMAL_POLICIES_STATUS_ENTRY_V2 {
    type Target = ThermalLimit;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok(ThermalLimit {
            policy: self.policy_id.try_into()?,
            value: CelsiusShifted(self.temp_limit_C as _),
            remove_tdp_limit: false,
            pff: None,
        })
    }
}

impl RawConversion for thermal::undocumented::NV_GPU_CLIENT_THERMAL_POLICY_STATUS_V3 {
    type Target = ThermalLimit;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok(ThermalLimit {
            policy: self.policy_id.try_into()?,
            value: CelsiusShifted(self.temp_limit_C as _),
            remove_tdp_limit: self.remove_tdp_limit.get(),
            pff: match self.has_pff() {
                true => Some(PffStatus {
                    curve: self.pff_curve.convert_raw()?,
                    values: self
                        .pff_freqs()
                        .iter()
                        .map(|&c| Kilohertz(c as _))
                        .collect(),
                }),
                false => None,
            },
        })
    }
}

impl RawConversion for thermal::undocumented::NV_GPU_CLIENT_THERMAL_POLICIES_STATUS_V2 {
    type Target = Vec<ThermalLimit>;
    type Error = sys::ArgumentRangeError;
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        convert_entries(self.entries(), self)
    }
}

impl RawConversion for thermal::undocumented::NV_GPU_CLIENT_THERMAL_POLICIES_STATUS_V3 {
    type Target = Vec<ThermalLimit>;
    type Error = sys::ArgumentRangeError;
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        convert_entries(self.entries(), self)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct PffPoint {
    pub x: CelsiusShifted,
    pub y: Kilohertz,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct PffCurve {
    pub points: Vec<PffPoint>,
}

impl RawConversion for thermal::undocumented::NV_GPU_CLIENT_PFF_CURVE_POINT_V1 {
    type Target = PffPoint;
    type Error = Infallible;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok(PffPoint {
            x: CelsiusShifted(self.temp as _),
            y: Kilohertz(self.uiT_Y),
        })
    }
}

impl PffPoint {
    pub fn to_raw(&self) -> thermal::undocumented::NV_GPU_CLIENT_PFF_CURVE_POINT_V1 {
        thermal::undocumented::NV_GPU_CLIENT_PFF_CURVE_POINT_V1 {
            enabled: true.into(),
            temp: self.x.0 as _,
            uiT_Y: self.y.0,
            padding: Default::default(),
        }
    }
}

impl RawConversion for thermal::undocumented::NV_GPU_CLIENT_PFF_CURVE_V1 {
    type Target = PffCurve;
    type Error = Infallible;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok(PffCurve {
            points: self
                .points()
                .iter()
                .map(RawConversion::convert_raw)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl PffCurve {
    pub fn to_raw(&self) -> thermal::undocumented::NV_GPU_CLIENT_PFF_CURVE_V1 {
        let mut curve = thermal::undocumented::NV_GPU_CLIENT_PFF_CURVE_V1::default();
        for (dest, src) in curve.points.iter_mut().zip(&self.points) {
            *dest = src.to_raw();
        }
        curve
    }
}

impl FromIterator<PffPoint> for PffCurve {
    fn from_iter<T: IntoIterator<Item = PffPoint>>(iter: T) -> Self {
        Self {
            points: Vec::from_iter(iter),
        }
    }
}

impl fmt::Display for PffPoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}@{}", self.x, self.y)
    }
}

impl fmt::Display for PffCurve {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, p) in self.points.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            fmt::Display::fmt(p, f)?;
        }
        Ok(())
    }
}

pub use sys::gpu::cooler::undocumented::{
    CoolerControl, CoolerController, CoolerPolicy, CoolerTarget, CoolerType,
};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct Cooler {
    pub info: CoolerInfo,
    pub status: CoolerStatus,
    pub control: CoolerSettings,
    pub unknown: u32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct CoolerInfo {
    pub kind: CoolerType,
    pub controller: CoolerController,
    pub target: CoolerTarget,
    pub control: CoolerControl,
    pub default_level_range: Option<Range<Percentage>>,
    pub default_policy: CoolerPolicy,
    pub tach_range: Option<Range<Rpm>>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct CoolerStatus {
    pub current_level: Percentage,
    pub current_level_range: Range<Percentage>,
    pub active: bool,
    pub current_tach: Option<Rpm>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct CoolerSettings {
    pub policy: CoolerPolicy,
    pub level: Option<Percentage>,
}

impl CoolerSettings {
    pub fn new(level: Option<Percentage>) -> Self {
        Self {
            policy: match level {
                Some(..) => CoolerPolicy::Manual,
                None => CoolerPolicy::TemperatureContinuous,
            },
            level,
        }
    }

    pub fn to_raw(
        &self,
        cooler_id: FanCoolerId,
    ) -> cooler::undocumented::NV_GPU_CLIENT_FAN_COOLER_CONTROL_V1 {
        let mut raw = cooler::undocumented::NV_GPU_CLIENT_FAN_COOLER_CONTROL_V1 {
            cooler_id: cooler_id.into(),
            level: self.level.unwrap_or_default().0,
            ..Default::default()
        };
        raw.set_manual(match (self.policy, self.level) {
            (_, None) => false,
            (
                CoolerPolicy::Performance
                | CoolerPolicy::TemperatureDiscrete
                | CoolerPolicy::TemperatureContinuous,
                _,
            ) => true,
            _ => false,
        });
        raw
    }
}

impl RawConversion for cooler::undocumented::NV_GPU_GETCOOLER_SETTING_V1 {
    type Target = Cooler;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok(Cooler {
            info: CoolerInfo {
                kind: CoolerType::try_from(self.type_)?,
                target: CoolerTarget::try_from(self.target)?,
                controller: CoolerController::try_from(self.controller)?,
                control: CoolerControl::try_from(self.controlType)?,
                default_policy: CoolerPolicy::try_from(self.defaultPolicy)?,
                default_level_range: Some(Range {
                    min: Percentage::from_raw(self.defaultMinLevel)?,
                    max: Percentage::from_raw(self.defaultMaxLevel)?,
                }),
                tach_range: None,
            },
            status: CoolerStatus {
                current_level_range: Range {
                    min: Percentage::from_raw(self.currentMinLevel)?,
                    max: Percentage::from_raw(self.currentMaxLevel)?,
                },
                current_level: Percentage::from_raw(self.currentLevel)?,
                active: cooler::undocumented::CoolerActivityLevel::try_from(self.active)?.get(),
                current_tach: None,
            },
            control: CoolerSettings {
                policy: CoolerPolicy::try_from(self.currentPolicy)?,
                level: Some(Percentage::from_raw(self.currentLevel)?),
            },
            unknown: 0,
        })
    }
}

impl RawConversion for cooler::undocumented::NV_GPU_GETCOOLER_SETTING_V3 {
    type Target = Cooler;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        let mut cooler = self.v1.convert_raw()?;
        if self.tachometer.bSupported.get() {
            cooler.info.tach_range = Some(Range {
                min: Rpm(self.tachometer.minSpeedRPM),
                max: Rpm(self.tachometer.maxSpeedRPM),
            });
            cooler.status.current_tach = Some(Rpm(self.tachometer.speedRPM));
        }
        Ok(cooler)
    }
}

impl RawConversion for cooler::undocumented::NV_GPU_GETCOOLER_SETTING_V4 {
    type Target = Cooler;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        let mut cooler = self.v3.convert_raw()?;
        cooler.unknown = self.unknown;
        Ok(cooler)
    }
}

impl RawConversion for cooler::undocumented::NV_GPU_GETCOOLER_SETTINGS_V4 {
    type Target = Vec<Cooler>;
    type Error = sys::ArgumentRangeError;
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        convert_entries(self.coolers(), self)
    }
}

impl RawConversion for cooler::undocumented::NV_GPU_GETCOOLER_SETTINGS_V3 {
    type Target = Vec<Cooler>;
    type Error = sys::ArgumentRangeError;
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        convert_entries(self.coolers(), self)
    }
}

impl RawConversion for cooler::undocumented::NV_GPU_GETCOOLER_SETTINGS_V1 {
    type Target = Vec<Cooler>;
    type Error = sys::ArgumentRangeError;
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        convert_entries(self.coolers(), self)
    }
}

impl RawConversion for cooler::undocumented::NV_GPU_CLIENT_FAN_COOLER_INFO_V1 {
    type Target = (FanCoolerId, CoolerInfo);
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok((
            self.cooler_id.try_into()?,
            CoolerInfo {
                controller: CoolerController::Internal,
                kind: CoolerType::Fan,
                target: CoolerTarget::GPU,
                control: CoolerControl::Variable,
                default_policy: CoolerPolicy::None,
                default_level_range: None,
                tach_range: match self.tach_supported.get() {
                    true => Some(Range {
                        min: Rpm(self.tach_min_rpm),
                        max: Rpm(self.tach_max_rpm),
                    }),
                    false => None,
                },
            },
        ))
    }
}

impl RawConversion for cooler::undocumented::NV_GPU_CLIENT_FAN_COOLERS_INFO_V1 {
    type Target = BTreeMap<FanCoolerId, CoolerInfo>;
    type Error = sys::ArgumentRangeError;
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        convert_entries(self.coolers(), self)
    }
}

impl RawConversion for cooler::undocumented::NV_GPU_CLIENT_FAN_COOLER_STATUS_V1 {
    type Target = (FanCoolerId, CoolerStatus);
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok((
            self.cooler_id.try_into()?,
            CoolerStatus {
                active: self.level != 0,
                current_level: Percentage::from_raw(self.level)?,
                current_level_range: Range {
                    min: Percentage::from_raw(self.level_minimum)?,
                    max: Percentage::from_raw(self.level_maximum)?,
                },
                current_tach: Some(Rpm(self.tach_rpm)),
            },
        ))
    }
}

impl RawConversion for cooler::undocumented::NV_GPU_CLIENT_FAN_COOLERS_STATUS_V1 {
    type Target = BTreeMap<FanCoolerId, CoolerStatus>;
    type Error = sys::ArgumentRangeError;
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        convert_entries(self.coolers(), self)
    }
}
impl RawConversion for cooler::undocumented::NV_GPU_CLIENT_FAN_COOLERS_CONTROL_V1 {
    type Target = BTreeMap<FanCoolerId, CoolerSettings>;
    type Error = sys::ArgumentRangeError;
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        convert_entries(self.coolers(), self)
    }
}

impl RawConversion for cooler::undocumented::NV_GPU_CLIENT_FAN_COOLER_CONTROL_V1 {
    type Target = (FanCoolerId, CoolerSettings);
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok((
            self.cooler_id.try_into()?,
            match self.manual() {
                true => CoolerSettings {
                    policy: CoolerPolicy::Manual,
                    level: Some(Percentage::from_raw(self.level)?),
                },
                false => CoolerSettings {
                    policy: CoolerPolicy::TemperatureContinuous,
                    level: None,
                },
            },
        ))
    }
}

impl RawConversion for cooler::undocumented::NV_GPU_SETCOOLER_LEVEL_COOLER {
    type Target = CoolerSettings;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok(CoolerSettings {
            level: Some(Percentage::from_raw(self.currentLevel)?),
            policy: CoolerPolicy::try_from(self.currentPolicy)?,
        })
    }
}

impl RawConversion for cooler::undocumented::NV_GPU_SETCOOLER_LEVEL {
    type Target = Vec<CoolerSettings>;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        self.cooler.iter().map(RawConversion::convert_raw).collect()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct CoolerPolicyLevel {
    pub level_id: u32,
    pub current_level: u32,
    pub default_level: u32,
}

impl RawConversion for cooler::undocumented::NV_GPU_COOLER_POLICY_LEVEL {
    type Target = CoolerPolicyLevel;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok(CoolerPolicyLevel {
            level_id: self.levelId,
            current_level: self.currentLevel,
            default_level: self.defaultLevel,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CoolerPolicyTable {
    pub policy: CoolerPolicy,
    pub levels: Vec<CoolerPolicyLevel>,
}

impl RawConversion for cooler::undocumented::NV_GPU_COOLER_POLICY_TABLE {
    type Target = CoolerPolicyTable;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok(CoolerPolicyTable {
            policy: CoolerPolicy::try_from(self.policy)?,
            levels: self
                .policyCoolerLevel
                .iter()
                .map(RawConversion::convert_raw)
                .collect::<Result<_, _>>()?,
        })
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct FanArbiterInfo {
    pub flags: FanArbiterInfoFlags,
}

impl RawConversion for cooler::undocumented::NV_GPU_CLIENT_FAN_ARBITER_INFO_V1 {
    type Target = (u32, FanArbiterInfo);
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok((
            self.arbiter_index,
            FanArbiterInfo {
                flags: self.flags.try_into()?,
            },
        ))
    }
}

impl RawConversion for cooler::undocumented::NV_GPU_CLIENT_FAN_ARBITERS_INFO_V1 {
    type Target = BTreeMap<u32, FanArbiterInfo>;
    type Error = sys::ArgumentRangeError;
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        convert_entries(self.arbiters(), self)
    }
}
impl RawConversion for cooler::undocumented::NV_GPU_CLIENT_FAN_ARBITERS_STATUS_V1 {
    type Target = BTreeMap<u32, FanArbiterStatus>;
    type Error = sys::ArgumentRangeError;
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        convert_entries(self.arbiters(), self)
    }
}
impl RawConversion for cooler::undocumented::NV_GPU_CLIENT_FAN_ARBITERS_CONTROL_V1 {
    type Target = BTreeMap<u32, FanArbiterControl>;
    type Error = sys::ArgumentRangeError;
    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        convert_entries(self.arbiters(), self)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct FanArbiterStatus {
    pub fan_stopped: bool,
}

impl RawConversion for cooler::undocumented::NV_GPU_CLIENT_FAN_ARBITER_STATUS_V1 {
    type Target = (u32, FanArbiterStatus);
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok((
            self.unknown0,
            FanArbiterStatus {
                fan_stopped: self.fan_stop_active(),
            },
        ))
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct FanArbiterControl {
    pub stop_fan: bool,
}

impl RawConversion for cooler::undocumented::NV_GPU_CLIENT_FAN_ARBITER_CONTROL_V1 {
    type Target = (u32, FanArbiterControl);
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        Ok((
            self.arbiter_index,
            FanArbiterControl {
                stop_fan: self
                    .flags()
                    .contains(cooler::undocumented::FanArbiterControlFlags::FAN_STOP),
            },
        ))
    }
}

/// Per-channel thermal metadata for ONE channel, decoded from the GetInfo
/// capability descriptor. The numeric fields (`scaling` / `offset_sw` /
/// `offset_hw` / `min_temp` / `max_temp`) are surfaced as opaque `i32`
/// pass-through — their exact fixed-point semantics are not documented and
/// are exposed for research. `ch_type` 255 means the channel exists but is
/// not one of the 5 standard thermal types.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ChannelInfo {
    /// `NV_GPU_THERMAL_THERM_CHANNEL_TYPE` (0=GPU_AVG, 1=GPU_MAX, 2=BOARD,
    /// 3=MEMORY, 4=PWR_SUPPLY, 255=unclassified).
    pub ch_type: u32,
    pub ch_class: u32,
    pub rel_loc: u32,
    pub tgt_gpu: u32,
    /// Fixed-point scaling factor (semantics undocumented; research use).
    pub scaling: i32,
    /// Software offset (semantics undocumented; research use).
    pub offset_sw: i32,
    /// Hardware offset (semantics undocumented; research use).
    pub offset_hw: i32,
    /// Sensor reporting range lower bound (fixed-point; research use).
    pub min_temp: i32,
    /// Sensor reporting range upper bound (fixed-point; research use).
    pub max_temp: i32,
    pub is_temp_sim_supported: u8,
    pub flags: u8,
    /// `data.device.thermDevIdx`: the physical thermal device this channel
    /// reads from. Channels sharing this value read the same sensor.
    pub therm_dev_idx: u8,
    /// `data.device.thermDevProvIdx`: provider index within the device. For a
    /// `(dev, 1)` channel the driver has already applied `offset_hw` to its
    /// STATUS reading; the matching `(dev, 0)` channel has not.
    pub therm_dev_prov_idx: u8,
}

impl From<&thermal::undocumented::NV_GPU_THERMAL_THERM_CHANNEL_INFO_V1> for ChannelInfo {
    fn from(c: &thermal::undocumented::NV_GPU_THERMAL_THERM_CHANNEL_INFO_V1) -> Self {
        ChannelInfo {
            ch_type: c.ch_type,
            ch_class: c.ch_class,
            rel_loc: c.rel_loc,
            tgt_gpu: c.tgt_gpu,
            scaling: c.scaling,
            offset_sw: c.offset_sw,
            offset_hw: c.offset_hw,
            min_temp: c.min_temp,
            max_temp: c.max_temp,
            is_temp_sim_supported: c.is_temp_sim_supported,
            flags: c.flags,
            therm_dev_idx: c.therm_dev_idx(),
            therm_dev_prov_idx: c.therm_dev_prov_idx(),
        }
    }
}

/// Thermal-channel capability descriptor from the undocumented
/// `NvAPI_GPU_ThermChannelGetInfo` (0x0bc8163d).
///
/// `primary` gives the authoritative channel index for each of the 5 thermal
/// types (GPU_AVG, GPU_MAX=hotspot, BOARD, MEMORY=vram, PWR_SUPPLY); `None`
/// where that type is not exposed (e.g. desktop consumer cards typically have
/// BOARD/MEMORY/PWR_SUPPLY absent). `primary_info` additionally exposes that
/// channel's metadata (ch_type / offsets / range).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ThermalChannelInfo {
    /// Bitmask of populated channel slots (which of the 32 channel records are valid).
    pub channel_mask: u32,
    /// Primary channel index per thermal type, indexed by
    /// `NV_GPU_THERMAL_THERM_CHANNEL_TYPE`:
    /// `[GPU_AVG, GPU_MAX(hotspot), BOARD, MEMORY(vram), PWR_SUPPLY]`.
    /// `None` where the type is unavailable or the channel bit is not set.
    pub primary: [Option<u8>; thermal::undocumented::NV_GPU_THERMAL_THERM_CHANNEL_TYPE_MAX],
    /// Per-channel metadata, indexed by channel number (0..32); `None` where
    /// the channel bit is not set in `channel_mask`.
    pub channels: Vec<Option<ChannelInfo>>,
}

impl ThermalChannelInfo {
    /// Hot spot (GPU_MAX) primary channel index, if available.
    pub fn hotspot_index(&self) -> Option<u8> {
        self.primary
            [thermal::undocumented::NV_GPU_THERMAL_THERM_CHANNEL_TYPE_GPU_MAX.repr() as usize]
    }

    /// VRAM (MEMORY) primary channel index, if available.
    pub fn memory_index(&self) -> Option<u8> {
        self.primary
            [thermal::undocumented::NV_GPU_THERMAL_THERM_CHANNEL_TYPE_MEMORY.repr() as usize]
    }

    /// Metadata for a given channel index, if that channel is populated.
    pub fn channel_info(&self, channel: usize) -> Option<&ChannelInfo> {
        self.channels.get(channel).and_then(|c| c.as_ref())
    }

    /// Metadata for a thermal type's primary channel, if present.
    pub fn primary_info(&self, ty: usize) -> Option<&ChannelInfo> {
        self.primary
            .get(ty)
            .copied()
            .flatten()
            .map(|i| i as usize)
            .and_then(|i| self.channel_info(i))
    }
}

impl RawConversion for thermal::undocumented::NV_GPU_THERMAL_THERM_CHANNEL_INFO {
    type Target = ThermalChannelInfo;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        // Mirror the raw struct's mask-aware primary_index() into the typed array.
        let mut primary = [None; thermal::undocumented::NV_GPU_THERMAL_THERM_CHANNEL_TYPE_MAX];
        for (ty, slot) in primary.iter_mut().enumerate() {
            if let Some(idx) = self.primary_index(ty) {
                *slot = Some(idx as u8);
            }
        }
        // Decode per-channel metadata for every populated channel.
        let channels = self
            .channel
            .iter()
            .enumerate()
            .map(|(i, c)| {
                if self.channel_mask & (1u32 << i) != 0 {
                    Some(ChannelInfo::from(c))
                } else {
                    None
                }
            })
            .collect();
        Ok(ThermalChannelInfo {
            channel_mask: self.channel_mask,
            primary,
            channels,
        })
    }
}

/// Live thermal-channel readings from the STATUS half of the ThermChannel
/// pair (ID 0x65fe3aad, `channel[32]` layout). Indexed DIRECTLY by channel
/// number (matching GetInfo's priChIdx), so `temps` holds one Celsius reading
/// per channel whose bit is set in the `channel_mask` passed in. Decode is
/// celsius*256 (sub-degree precision preserved).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ThermalChannelStatus {
    pub channel_mask: u32,
    /// `(channel_index, celsius)` for each populated channel.
    pub temps: Vec<(usize, f32)>,
}

impl ThermalChannelStatus {
    /// Temperature at a specific channel index (e.g. `priChIdx[GPU_MAX]`).
    pub fn get(&self, channel: usize) -> Option<f32> {
        self.temps
            .iter()
            .find(|(i, _)| *i == channel)
            .map(|(_, t)| *t)
    }
}

impl RawConversion for thermal::undocumented::NV_GPU_THERMAL_THERM_CHANNEL_STATUS {
    type Target = ThermalChannelStatus;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        let temps = self
            .channel
            .iter()
            .enumerate()
            .filter_map(|(i, &v)| {
                thermal::undocumented::NV_GPU_THERMAL_THERM_CHANNEL_STATUS_PARAMS_V2::decode(v)
                    .map(|t| (i, t))
            })
            .collect();
        Ok(ThermalChannelStatus {
            channel_mask: self.channel_mask,
            temps,
        })
    }
}
