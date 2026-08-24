use crate::{allowable_result, allowable_result_fallback};
use once_cell::sync::OnceCell;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn collect_domain<T: Copy, U: From<T>>(
    points: &BTreeMap<ClockDomain, Vec<(usize, T)>>,
    domain: ClockDomain,
) -> BTreeMap<usize, U> {
    points
        .get(&domain)
        .map(|d| d.iter().map(|&(i, e)| (i, e.into())).collect())
        .unwrap_or_default()
}

use nvapi::{
    self, BaseVoltage, ClockEntry, ClockFrequencyType, ClockRange, ClockTable, PStates, PffStatus,
    PowerInfoEntry, Sensor, ThermalInfo, ThermalLimit, ThermalPolicyId, VfpCurve, VfpEntry,
    VfpInfo,
};
pub use nvapi::{
    AllClocks, ArchInfo, Bus, BusInfo, BusType, Celsius, ClockDomain, ClockFrequencies,
    ClockLockEntry, ClockLockValue, ComputeCapabilities, ConnectedIdsFlags, CoolerControl,
    CoolerController, CoolerInfo, CoolerPolicy, CoolerSettings, CoolerStatus, CoolerTarget,
    CoolerType, DisplayId, DriverModel, EccErrors, EffectiveClocks, FanArbiterControl,
    FanArbiterStatus, FanCoolerId, FanCurve, FanCurvePoint, Foundry, GpuType, Kibibytes, Kilohertz,
    KilohertzDelta, MemoryInfo, Microvolts, MicrovoltsDelta, PState, PStateNativeLock,
    PciIdentifiers, Percentage, PerfFreqCap, PerfFreqCapEntry, PerfInfo, PerfLimitId, PerfStatus,
    PerformanceDecreaseReason, PffCurve, PffPoint, PhysicalGpu, PowerMonitor, PowerRails,
    PowerTopologyChannelId, RamMaker, RamType, Range, Rpm, SystemType, ThermalChannelInfo,
    ThermalChannelStatus, ThermalController, ThermalTarget, UtilizationDomain, Utilizations,
    Vendor, VfPointType, VoltageDomain, VoltageStatus, VoltageTable,
};

pub struct Gpu {
    gpu: PhysicalGpu,
    vfp_info: OnceCell<VfpInfo>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct GpuInfo {
    pub id: usize,
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    pub uuid: Option<String>,
    pub name: String,
    pub codename: String,
    pub bios_version: String,
    pub driver_model: Option<DriverModel>,
    pub bus: BusInfo,
    pub memory: Option<MemoryInfo>,
    pub system_type: SystemType,
    pub gpu_type: GpuType,
    pub arch: ArchInfo,
    pub ram_type: RamType,
    pub ram_maker: RamMaker,
    pub ram_bus_width: u32,
    pub physical_frame_buffer: Kibibytes,
    pub virtual_frame_buffer: Kibibytes,
    pub ram_bank_count: u32,
    pub ram_partition_count: u32,
    pub foundry: Foundry,
    pub core_count: u32,
    pub shader_pipe_count: u32,
    pub shader_sub_pipe_count: u32,
    pub ecc: EccInfo,
    /// Static compute/PhysX/framebuffer capability flags
    /// (`NvAPI_GPU_GetComputeCapabilities`). Despite the name the bits are PhysX/compute/
    /// framebuffer oriented, not virtualization. One-shot descriptor (zero = no caps / not
    /// reported), appropriate for `get-info`. See [ComputeCapabilities].
    pub compute_capabilities: ComputeCapabilities,
    pub base_clocks: ClockFrequencies,
    pub boost_clocks: ClockFrequencies,
    pub sensors: Vec<SensorDesc>,
    pub coolers: BTreeMap<FanCoolerId, CoolerInfo>,
    pub perf: PerfInfo,
    pub sensor_limits: Vec<SensorLimit>,
    pub power_limits: Vec<PowerLimit>,
    pub pstate_limits: BTreeMap<PState, BTreeMap<ClockDomain, PStateLimit>>,
    // TODO: pstate base_voltages
    pub overvolt_limits: Vec<OvervoltLimit>,
    pub vfp_limits: BTreeMap<ClockDomain, VfpRange>,
    pub connected_displays: Vec<DisplayId>,
}

impl GpuInfo {
    pub fn vendor(&self) -> Option<Vendor> {
        self.bus.vendor().ok().flatten()
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct EccInfo {
    pub enabled_by_default: bool,
    pub info: nvapi::EccStatus,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct EccStatus {
    pub enabled: bool,
    pub errors: EccErrors,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct VfpRange {
    pub range: Range<KilohertzDelta>,
}

impl From<ClockRange> for VfpRange {
    fn from(c: ClockRange) -> Self {
        VfpRange {
            range: Range::range_from(c.range),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct GpuStatus {
    pub pstate: PState,
    pub clocks: ClockFrequencies,
    /// Effective (actually-running) clocks from GetAllClocks V2
    /// (`NV_GPU_CLOCK_INFO_V2`). `None` where the driver doesn't support the
    /// V2 layout. Distinct from `clocks` (the GetAllClockFrequencies table).
    pub effective_clocks: Option<EffectiveClocks>,
    /// All 32 effective clock domains from GetAllClocks V2 (superset of
    /// `effective_clocks`): includes the internal fabric clocks — Gpc,
    /// **Xbar (crossbar)**, Sys, Hub, Host, Disp, Hotclk, Gpc2/Xbar2/Sys2/Hub2,
    /// Pciegen, etc. `None` where the driver doesn't support the V2 layout.
    pub all_clocks: Option<AllClocks>,
    /// Per-channel / per-rail power from PowerMonitor v4 GetInfo + v1 GetStatus
    /// (IDs 0xC12EB19E / 0xF40238EF). `None` where the GPU/driver doesn't
    /// expose PowerMonitor. **Pre-wrap / research: raw values, units
    /// unconfirmed** — see `nvapi::power` for the validation status.
    pub power_monitor: Option<PowerMonitor>,
    /// Per-rail power readings (Board / Chip / MVDDC / PWR_SRC / …, whichever
    /// the GPU exposes), via PowerMonitor GetStatus v1|392 with per-bit
    /// isolation + topology disambiguation. `None` where PowerMonitor isn't
    /// exposed. Each reading carries a `Confidence` tier (Measured/Inferred/
    /// Ambiguous/Unavailable); units confirmed (raw ÷ 1000 = W).
    pub power_rails: Option<PowerRails>,
    pub memory: Option<MemoryInfo>,
    pub pcie_lanes: Option<u32>,
    pub ecc: EccStatus,
    pub voltage: Option<Microvolts>,
    pub voltage_domains: Option<VoltageStatus>,
    pub voltage_step: Option<VoltageStatus>,
    pub voltage_table: Option<VoltageTable>,
    pub tachometer: Option<u32>,
    pub utilization: Utilizations,
    pub power: BTreeMap<PowerTopologyChannelId, Percentage>,
    /// `(descriptor, celsius)` thermal readings with sub-degree precision.
    pub sensors: Vec<(SensorDesc, f32)>,
    pub coolers: BTreeMap<FanCoolerId, CoolerStatus>,
    pub perf: PerfStatus,
    /// Reason(s) the GPU is currently below peak performance
    /// (`NvAPI_GPU_GetPerfDecreaseInfo`): a bitset of thermal/power/battery/
    /// API/insufficient-power flags. Empty (`NONE`) when running at full speed.
    pub performance_decrease: PerformanceDecreaseReason,
    /// Fan arbiter status/control from `NvAPI_GPU_ClientFanArbiters*`. The
    /// status reports whether each fan is currently stopped (zero-RPM); the
    /// control reports whether the driver is permitted to stop it.
    pub fan_arbiter_status: BTreeMap<u32, FanArbiterStatus>,
    pub fan_arbiter_control: BTreeMap<u32, FanArbiterControl>,
    /// Legacy single-value levels from `NvAPI_GPU_GetCurrent*Level`. These are
    /// older aggregate indices (0-based) superseded by the per-cooler
    /// `coolers` map above, but some tools still read them.
    pub current_thermal_level: Option<u32>,
    pub current_fan_speed_level: Option<u32>,
    pub vfp: Option<VfpTable>,
    pub vfp_locks: BTreeMap<PerfLimitId, ClockLockValue>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct GpuSettings {
    pub voltage_boost: Option<Percentage>,
    pub sensor_limits: Vec<SensorThrottle>,
    pub power_limits: Vec<Percentage>,
    pub coolers: BTreeMap<FanCoolerId, CoolerSettings>,
    pub vfp: Option<VfpDeltas>,
    pub pstate_deltas: BTreeMap<PState, BTreeMap<ClockDomain, KilohertzDelta>>,
    pub overvolt: Vec<MicrovoltsDelta>,
    pub vfp_locks: BTreeMap<PerfLimitId, ClockLockEntry>,
}

impl Gpu {
    pub fn new(gpu: PhysicalGpu) -> Self {
        Gpu {
            gpu,
            vfp_info: OnceCell::new(),
        }
    }

    pub fn into_inner(self) -> PhysicalGpu {
        self.gpu
    }

    pub fn inner(&self) -> &PhysicalGpu {
        &self.gpu
    }

    pub fn id(&self) -> usize {
        self.gpu.handle().as_ptr() as _
    }

    pub fn enumerate() -> nvapi::Result<Vec<Self>> {
        PhysicalGpu::enumerate()
            .map_err(Into::into)
            .map(|v| v.into_iter().map(Gpu::new).collect())
    }

    pub fn info(&self) -> nvapi::Result<GpuInfo> {
        let pstates = allowable_result(self.gpu.pstates())?;
        let (pstates, ov) = match pstates {
            Ok(PStates {
                editable: _editable,
                pstates,
                overvolt,
            }) => (pstates, overvolt),
            Err(..) => (Default::default(), Default::default()),
        };

        Ok(GpuInfo {
            id: self.id(),
            uuid: allowable_result(self.gpu.uuid())?.ok(),
            name: self.gpu.full_name()?,
            codename: self.gpu.short_name()?,
            bios_version: self.gpu.vbios_version_string()?,
            driver_model: allowable_result(self.gpu.driver_model())?.ok(),
            bus: allowable_result_fallback(self.gpu.bus_info(), Default::default())?,
            memory: allowable_result(self.gpu.memory_info())?.ok(),
            ecc: EccInfo {
                enabled_by_default: allowable_result_fallback(
                    self.gpu
                        .ecc_configuration()
                        .map(|(_enabled, enabled_by_default)| enabled_by_default),
                    false,
                )?,
                info: allowable_result_fallback(self.gpu.ecc_status(), Default::default())?,
            },
            compute_capabilities: allowable_result_fallback(
                self.gpu.compute_capabilities(),
                Default::default(),
            )?,
            system_type: allowable_result_fallback(self.gpu.system_type(), SystemType::Unknown)?,
            gpu_type: allowable_result_fallback(self.gpu.gpu_type(), GpuType::Unknown)?,
            arch: allowable_result_fallback(self.gpu.architecture(), Default::default())?,
            ram_type: allowable_result_fallback(self.gpu.ram_type(), RamType::Unknown)?,
            ram_maker: allowable_result_fallback(self.gpu.ram_maker(), RamMaker::Unknown)?,
            ram_bus_width: allowable_result_fallback(self.gpu.ram_bus_width(), 0)?,
            physical_frame_buffer: allowable_result_fallback(
                self.gpu.physical_frame_buffer_size(),
                Kibibytes(0),
            )?,
            virtual_frame_buffer: allowable_result_fallback(
                self.gpu.virtual_frame_buffer_size(),
                Kibibytes(0),
            )?,
            ram_bank_count: allowable_result_fallback(self.gpu.ram_bank_count(), 0)?,
            ram_partition_count: allowable_result_fallback(self.gpu.ram_partition_count(), 0)?,
            foundry: allowable_result_fallback(self.gpu.foundry(), Foundry::Unknown)?,
            core_count: self.gpu.core_count()?,
            shader_pipe_count: self.gpu.shader_pipe_count()?,
            shader_sub_pipe_count: self.gpu.shader_sub_pipe_count()?,
            base_clocks: self.gpu.clock_frequencies(ClockFrequencyType::Base)?,
            boost_clocks: self.gpu.clock_frequencies(ClockFrequencyType::Boost)?,
            sensors: match allowable_result(self.gpu.thermal_settings(None))? {
                Ok(s) => s.into_iter().map(From::from).collect(),
                Err(..) => Default::default(),
            },
            coolers: allowable_result(self.gpu.cooler_info())?
                .unwrap_or_else(|_e| Default::default()),
            perf: self.gpu.perf_info()?,
            sensor_limits: match allowable_result(self.gpu.thermal_limit_info())? {
                Ok(l) => l.into_iter().map(From::from).collect(),
                Err(..) => Default::default(),
            },
            power_limits: match allowable_result(self.gpu.power_limit_info())? {
                Ok(p) => p.entries.into_iter().map(From::from).collect(),
                Err(..) => Default::default(),
            },
            pstate_limits: pstates
                .into_iter()
                .map(|p| {
                    (
                        p.id,
                        p.clocks
                            .into_iter()
                            .map(|p| (p.domain(), p.into()))
                            .collect(),
                    )
                })
                .collect(),
            overvolt_limits: ov.into_iter().map(From::from).collect(),
            vfp_limits: match allowable_result(self.gpu.vfp_ranges())? {
                Ok(l) => l
                    .domains
                    .into_iter()
                    .map(|v| (v.domain, v.into()))
                    .collect(),
                Err(..) => Default::default(),
            },
            connected_displays: allowable_result(
                self.gpu.display_ids_connected(ConnectedIdsFlags::empty()),
            )?
            .unwrap_or_else(|_| Default::default()),
        })
    }

    fn vfp_info(&self) -> nvapi::Result<nvapi::Result<&VfpInfo>> {
        allowable_result(self.vfp_info.get_or_try_init(|| self.gpu.vfp_info()))
    }

    pub fn status(&self) -> nvapi::Result<GpuStatus> {
        let vfp_info = self.vfp_info()?;

        // Thermal sensors via the RTSS ThermChannel pair (unified layout):
        // `NvAPI_GPU_ThermChannelGetInfo` (0x0bc8163d) returns a `priChIdx[5]`
        // LUT naming the authoritative primary channel per type (GPU_AVG=0,
        // GPU_MAX=1=hotspot, BOARD=2, MEMORY=3=VRAM, PWR_SUPPLY=4) plus
        // per-channel metadata; `NvAPI_GPU_ThermChannelGetStatus` (0x65fe3aad,
        // channel[32] layout, called with GetInfo's channel_mask) returns the
        // live temp at each channel index. `channel[priChIdx[type]]` is the
        // authoritative reading for that type.
        //
        // Best-effort: pre-Pascal GPUs may not expose GetInfo; on failure we
        // fall back to the documented `thermal_settings` (Core only) below.
        // Verified on Pascal/Turing/Ampere laptop + desktop GPUs: GetInfo
        // returns OK (e.g. 1080Ti channel_mask=0x03, 2070 0x7c00ff,
        // priChIdx GPU_AVG=0 / GPU_MAX=1 on all).
        //
        // Sensor ordering matters: positional consumers (nvoc-python/TUI/GUI)
        // take `sensors.first()` as the core temperature, so Core MUST be
        // emitted first.
        let therm_info = allowable_result(self.thermal_channel_info())?.ok();
        let therm_status = match therm_info.as_ref() {
            Some(i) if i.channel_mask != 0 => {
                allowable_result(self.thermal_channel_status(i.channel_mask))?.ok()
            }
            _ => None,
        };

        // Build the sensor list from the RTSS channel data. Sensors are
        // identified by their `channel_type` (GPU_AVG/GPU_MAX/BOARD/MEMORY/
        // PWR_SUPPLY, or 255=unclassified) — there is no free-form name; the
        // type IS the classification. Core (GPU_AVG) is emitted first so
        // positional consumers (sensors.first()) still see the core temp.
        let mut extra_sensors: Vec<(SensorDesc, f32)> = Vec::new();

        // ThermalTarget per standard channel type.
        let type_target: [ThermalTarget; 5] = [
            ThermalTarget::Gpu, // GPU_AVG (core)
            ThermalTarget::Gpu, // GPU_MAX (hot spot)
            ThermalTarget::Board,
            ThermalTarget::Memory,
            ThermalTarget::PowerSupply,
        ];
        if let (Some(info), Some(status)) = (therm_info.as_ref(), therm_status.as_ref()) {
            // Standard primary channels first, in type order (Core, then Hot
            // Spot, Board, Memory, Power Supply).
            for ty in 0..type_target.len() {
                let Some(idx) = info.primary.get(ty).copied().flatten() else {
                    continue;
                };
                let Some(temp) = status.get(idx as usize) else {
                    continue;
                };
                let target = type_target[ty];
                extra_sensors.push((
                    sensor_desc_for_channel(target, idx as u32, info.channel_info(idx as usize)),
                    temp,
                ));
            }

            // Remaining populated channels (ch_type=255, unclassified — e.g.
            // per-VRAM-module hotspots on desktop cards). Emitted in ascending
            // channel order, all uniformly unclassified (distinguished only by
            // their channel index). Target defaults to Gpu.
            for &(idx, _) in status.temps.iter() {
                let is_primary = info
                    .primary
                    .iter()
                    .any(|p| p.map(|p| p as usize) == Some(idx));
                if is_primary {
                    continue;
                }
                let temp = status.get(idx).unwrap_or(0.0);
                extra_sensors.push((
                    sensor_desc_for_channel(ThermalTarget::Gpu, idx as u32, info.channel_info(idx)),
                    temp,
                ));
            }

            // Sensor pairing: RTSS exposes two channels per physical sensor —
            // `(thermDevIdx, 0)` (raw) and `(thermDevIdx, 1)` (with `offset_hw`
            // applied by the driver). Mark each `ProvIdx==1` channel with the
            // index of its `ProvIdx==0` sibling so the display can annotate it.
            for (desc, _) in extra_sensors.iter_mut() {
                let Some(chan) = desc.channel_num else {
                    continue;
                };
                let Some(ci) = info.channel_info(chan as usize) else {
                    continue;
                };
                if ci.therm_dev_prov_idx != 1 {
                    continue;
                }
                // Find a populated channel with the same device and ProvIdx==0.
                if let Some((sibling, _)) = info.channels.iter().enumerate().find(|(i, c)| {
                    *i as u32 != chan
                        && c.as_ref().is_some_and(|c| {
                            c.therm_dev_idx == ci.therm_dev_idx && c.therm_dev_prov_idx == 0
                        })
                }) {
                    desc.same_sensor_as = Some(sibling as u32);
                }
            }
        }

        Ok(GpuStatus {
            pstate: self.gpu.current_pstate()?,
            clocks: self.gpu.clock_frequencies(ClockFrequencyType::Current)?,
            effective_clocks: self.gpu.effective_clocks().ok(),
            all_clocks: self.gpu.all_clocks().ok(),
            power_monitor: self.gpu.power_monitor_v4().ok(),
            power_rails: self.gpu.power_rails().ok(),
            memory: allowable_result(self.gpu.memory_info())?.ok(),
            pcie_lanes: match self.gpu.bus_type() {
                Ok(BusType::PciExpress) => {
                    allowable_result_fallback(self.gpu.pcie_lanes().map(Some), None)?
                }
                _ => None,
            },
            ecc: EccStatus {
                enabled: allowable_result_fallback(
                    self.gpu
                        .ecc_configuration()
                        .map(|(enabled, _enabled_by_default)| enabled),
                    false,
                )?,
                errors: allowable_result_fallback(self.gpu.ecc_errors(), Default::default())?,
            },
            voltage: allowable_result(self.gpu.core_voltage())?.ok(),
            voltage_domains: allowable_result(self.gpu.voltage_domains_status())?.ok(),
            voltage_step: allowable_result(self.gpu.voltage_step())?.ok(),
            voltage_table: allowable_result(self.gpu.voltage_table())?.ok(),
            tachometer: allowable_result(self.gpu.tachometer())?.ok(),
            utilization: self.gpu.dynamic_pstates_info()?,
            power: self
                .gpu
                .power_usage(self.gpu.power_usage_channels()?)?
                .into_iter()
                .map(|(ch, power)| (ch, power.into()))
                .collect(),
            sensors: {
                // RTSS path: if we got authoritative channel data, Core is
                // already at extra_sensors[0] (emitted first above). Otherwise
                // fall back to the documented thermal_settings (Core only) for
                // GPUs that don't expose GetInfo.
                let mut sensors: Vec<(SensorDesc, f32)> = Vec::new();
                if extra_sensors.is_empty() {
                    if let Ok(s) = allowable_result(self.gpu.thermal_settings(None))? {
                        for s in s {
                            let desc: SensorDesc = From::from(s);
                            let temp = s.current_temperature.0 as f32;
                            sensors.push((desc, temp));
                        }
                    }
                }
                sensors.extend(extra_sensors);
                sensors
            },
            coolers: allowable_result(self.gpu.cooler_status())?
                .unwrap_or_else(|_e| Default::default()),
            perf: self.gpu.perf_status()?,
            // Best-effort: these undocumented/legacy calls fail on many drivers
            // or on Optimus/secondary GPUs; degrade to empty/default rather
            // than aborting the whole status read.
            performance_decrease: allowable_result_fallback(
                self.gpu.performance_decrease(),
                PerformanceDecreaseReason::NONE,
            )?,
            fan_arbiter_status: allowable_result_fallback(
                self.gpu.fan_arbiter_status(),
                Default::default(),
            )?,
            fan_arbiter_control: allowable_result_fallback(
                self.gpu.fan_arbiter_control(),
                Default::default(),
            )?,
            current_thermal_level: allowable_result(self.gpu.current_thermal_level())?.ok(),
            current_fan_speed_level: allowable_result(self.gpu.current_fan_speed_level())?.ok(),
            vfp: match &vfp_info {
                Ok(info) => allowable_result(self.gpu.vfp_curve(info))?
                    .map(From::from)
                    .ok(),
                Err(..) => None,
            },
            vfp_locks: match allowable_result(self.gpu.vfp_locks(PerfLimitId::values()))? {
                Ok(l) => l
                    .into_iter()
                    .filter_map(|lock| lock.lock_value.map(|value| (lock.limit, value)))
                    .collect(),
                Err(..) => Default::default(),
            },
        })
    }

    pub fn settings(&self) -> nvapi::Result<GpuSettings> {
        let vfp_info = self.vfp_info()?;
        let pstates = allowable_result(self.gpu.pstates())?;
        let (pstates, ov) = match pstates {
            Ok(PStates {
                editable: _editable,
                pstates,
                overvolt,
            }) => (pstates, overvolt),
            Err(..) => (Default::default(), Default::default()),
        };

        Ok(GpuSettings {
            voltage_boost: allowable_result(self.gpu.core_voltage_boost())?.ok(),
            sensor_limits: match allowable_result(self.gpu.thermal_limit())? {
                Ok(l) => l
                    .into_iter()
                    .map(|l| SensorThrottle::from_limit(&l))
                    .collect(),
                Err(..) => Default::default(),
            },
            power_limits: match allowable_result(self.gpu.power_limit())? {
                Ok(l) => l.into_iter().map(|l| l.into()).collect(),
                Err(..) => Default::default(),
            },
            coolers: allowable_result(self.gpu.cooler_control())?
                .unwrap_or_else(|_e| Default::default()),
            vfp: match &vfp_info {
                Ok(info) => allowable_result(self.gpu.vfp_table(info))?
                    .map(From::from)
                    .ok(),
                Err(..) => None,
            },
            vfp_locks: match allowable_result(self.gpu.vfp_locks(PerfLimitId::values()))? {
                Ok(v) => v.into_iter().map(|lock| (lock.limit, lock)).collect(),
                Err(..) => Default::default(),
            },
            pstate_deltas: pstates
                .into_iter()
                .filter(|p| p.editable)
                .map(|p| {
                    (
                        p.id,
                        p.clocks
                            .into_iter()
                            .filter(|p| p.editable())
                            .map(|p| (p.domain(), p.frequency_delta().value))
                            .collect(),
                    )
                })
                .collect(),
            overvolt: ov
                .into_iter()
                .filter(|v| v.editable)
                .map(|v| v.voltage_delta.value)
                .collect(),
        })
    }

    pub fn set_voltage_boost(&self, boost: Percentage) -> nvapi::Result<()> {
        self.gpu.set_core_voltage_boost(boost).map_err(Into::into)
    }

    pub fn set_power_limits<I: IntoIterator<Item = Percentage>>(
        &self,
        limits: I,
    ) -> nvapi::Result<()> {
        // TODO: match against power_limit_info, use range.min/max from there if it matches (can get fraction of a percent!)
        self.gpu
            .set_power_limit(limits.into_iter().map(From::from))
            .map_err(Into::into)
    }

    /// Set the PPAB / Dynamic-Boost controller enable state (notebook dGPU↔CPU
    /// power coordination). `active = true` = "PPAB Enable" on. NDA-private
    /// ID 0x1504FC3D; raw boolean setter.
    pub fn set_dynamic_boost(&self, active: bool) -> nvapi::Result<()> {
        self.gpu.set_dynamic_boost(active).map_err(Into::into)
    }

    /// TGP-watts range (min/default/max mW) + active policy index (NDA
    /// 0x67F31384). `Ok(None)` where the driver doesn't expose it.
    pub fn tgp_watt_range(&self) -> nvapi::Result<Option<nvapi::TgpWattRange>> {
        self.gpu.tgp_watt_range().map_err(Into::into)
    }

    /// Set GPU TGP in watts (the watts-form TGP slider; read-modify-write over
    /// NDA 0x8B3E7343 GET + 0xBFF09E59 SET). Returns the mW actually written.
    pub fn set_tgp_watt(&self, watts: u32, policy_index: usize) -> nvapi::Result<u32> {
        self.gpu
            .set_tgp_watt(watts, policy_index)
            .map_err(Into::into)
    }

    /// Reset GPU TGP to rated/default (NDA triplet). Returns the default mW, if known.
    pub fn reset_tgp_watt(&self, policy_index: usize) -> nvapi::Result<Option<u32>> {
        self.gpu.reset_tgp_watt(policy_index).map_err(Into::into)
    }

    /// D-Notifier current state + the D1..D5 power-cap table (NDA 0x67F31384,
    /// the same private ClientPowerPoliciesGetInfo used by `tgp_watt_range`).
    /// `Ok(None)` where the driver doesn't expose the private interface.
    pub fn dnotify_info(&self) -> nvapi::Result<Option<nvapi::DNotifierInfo>> {
        self.gpu.dnotify_info().map_err(Into::into)
    }

    /// Set the D-Notifier (D0-notify) limit to a driver D-level code
    /// (-1=D1/Unlimited, 0..3=D2..D5). Raw two-arg setter (NDA 0x48E0847D).
    pub fn set_dnotify_limit(&self, didx: i32) -> nvapi::Result<()> {
        self.gpu.set_dnotify_limit(didx).map_err(Into::into)
    }

    /// Read-only snapshot of the private VoltRails family (the "melonVolt
    /// path": rail mask + per-rail control-offset entries + live per-rail
    /// voltages, via 0x2C73AFDC/0xA3070DB0/0x5D0634EE). `Ok(None)` where the
    /// driver doesn't expose the private interface.
    pub fn volt_rails(&self) -> nvapi::Result<Option<nvapi::VoltRails>> {
        match self.gpu.volt_rails() {
            Ok(v) => Ok(Some(v)),
            Err(nvapi::Error::Nvapi(e))
                if matches!(
                    e.status,
                    nvapi::Status::NotSupported
                        | nvapi::Status::NoImplementation
                        | nvapi::Status::ArgumentExceedMaxSize
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Write one rail's control-entry value (payload index 0; µV offset on
    /// type-3 entries) with the full melonVolt write protocol. `Ok(None)`
    /// where the driver doesn't expose the private family. Policy (type
    /// check, ±mV limits) is the caller's — see the core operation.
    #[allow(non_snake_case)] // uV suffix matches the sys-layer field naming
    pub fn set_volt_rail_value(&self, rail_bit: u32, value_uV: i32) -> nvapi::Result<Option<i32>> {
        match self.gpu.set_volt_rail_value(rail_bit, value_uV) {
            Ok(v) => Ok(Some(v)),
            Err(nvapi::Error::Nvapi(e))
                if matches!(
                    e.status,
                    nvapi::Status::NotSupported | nvapi::Status::NoImplementation
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    // --- Blackwell XBar ClockClient clock-domain family ---------------------
    // (reverse/melonvolt/xbar.txt — Loong0x00 LACT #1147). The 4 NV2080 RM
    // commands wrapped via private NVAPI IDs (escape 0x07000109).

    /// Controllable clock-domain block from the private ClockClient
    /// GetControl (RM 0x2080901b, ID 0xF58938F5). `Ok(None)` where the driver
    /// doesn't expose the private interface. The article's XBAR domain is
    /// bit 1 (`nvapi::ClockDomainId::Xbar`).
    pub fn clk_domains_control(&self) -> nvapi::Result<Option<nvapi::ClockDomainControl>> {
        match self.gpu.clk_domains_control() {
            Ok(v) => Ok(Some(v)),
            Err(nvapi::Error::Nvapi(e))
                if matches!(
                    e.status,
                    nvapi::Status::NotSupported
                        | nvapi::Status::NoImplementation
                        | nvapi::Status::ArgumentExceedMaxSize
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Physical clock for one domain from MEASURE_FREQ (RM 0x20809006,
    /// ID 0xFB8F61EC) via two-sample Δcounter/Δtimestamp. `domain_bit` is the
    /// sequential domain index (GPC=0, XBAR=1, SYS=2, MCLK=4). `Ok(None)`
    /// where the driver doesn't expose the private interface.
    pub fn clk_domain_freq(
        &self,
        domain_bit: u32,
    ) -> nvapi::Result<Option<nvapi::ClockDomainFreq>> {
        match self.gpu.clk_domain_freq(domain_bit) {
            Ok(v) => Ok(Some(v)),
            Err(nvapi::Error::Nvapi(e))
                if matches!(
                    e.status,
                    nvapi::Status::NotSupported | nvapi::Status::NoImplementation
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Detailed single-domain measure — frequency plus the second sample's
    /// raw {counter, timestamp, extra} and the accepted protocol form.
    /// `Ok(None)` where the driver refuses the domain.
    pub fn clk_domain_freq_detail(
        &self,
        domain_bit: u32,
    ) -> nvapi::Result<Option<nvapi::ClockDomainFreqDetail>> {
        match self.gpu.clk_domain_freq_detail(domain_bit) {
            Ok(v) => Ok(Some(v)),
            Err(nvapi::Error::Nvapi(e))
                if matches!(
                    e.status,
                    nvapi::Status::NotSupported | nvapi::Status::NoImplementation
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Write one V/F curve point via the private ClockClient V/F-POINTS
    /// SetControl (ID 0xFEC00D04). DANGEROUS: snapshots the full control
    /// block, patches one record, SETs, readbacks, restores on mismatch.
    /// `bank` 0 = V/F curve points, 1 = pstate-class; `idx` 0..2048.
    /// `freq_mode` = mode 0 (kHz freq OFFSET, max ~990 MHz) vs mode 1
    /// (reverse-volt lookup: delta → voltage shift → default freq lookup).
    /// Both modes produce identical curves after RM interpolation. Returns the
    /// retained value, or `Ok(None)` where the family is absent.
    pub fn set_vfp_point_private(
        &self,
        bank: usize,
        idx: usize,
        freq_mode: bool,
        value: u32,
    ) -> nvapi::Result<Option<u32>> {
        match self.gpu.set_vfp_point_private(bank, idx, freq_mode, value) {
            Ok(v) => Ok(Some(v)),
            Err(nvapi::Error::Nvapi(e))
                if matches!(
                    e.status,
                    nvapi::Status::NotSupported | nvapi::Status::NoImplementation
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Write a RANGE of V/F curve points with the same delta via the
    /// private SetControl — the analogue of the public
    /// `set-vfp-range-delta-mhz`. Patches `[start, end]` on `bank` in a
    /// single RMW cycle. `Ok(None)` where the family is absent.
    pub fn set_vfp_range_private(
        &self,
        bank: usize,
        start: usize,
        end: usize,
        delta_mhz: i16,
    ) -> nvapi::Result<Option<()>> {
        match self.gpu.set_vfp_range_private(bank, start, end, delta_mhz) {
            Ok(()) => Ok(Some(())),
            Err(nvapi::Error::Nvapi(e))
                if matches!(
                    e.status,
                    nvapi::Status::NotSupported | nvapi::Status::NoImplementation
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Like [`set_vfp_range_private`] but writes a per-point raw mode-1
    /// value (one RMW cycle). `deltas.len()` must equal `end - start + 1`.
    /// `Ok(None)` where the driver doesn't expose the private interface.
    pub fn set_vfp_range_per_point_private(
        &self,
        bank: usize,
        start: usize,
        end: usize,
        deltas: &[i16],
    ) -> nvapi::Result<Option<()>> {
        match self
            .gpu
            .set_vfp_range_per_point_private(bank, start, end, deltas)
        {
            Ok(()) => Ok(Some(())),
            Err(nvapi::Error::Nvapi(e))
                if matches!(
                    e.status,
                    nvapi::Status::NotSupported | nvapi::Status::NoImplementation
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Batch physical clocks for many domains via the V3 MEASURE_FREQ
    /// (magic 0x30038) — one RM round-trip per sample instead of one per
    /// domain. Per-domain V1/V2 fallback when the driver rejects the batch
    /// form. `Ok(None)` where the family is absent.
    pub fn clk_domain_freqs_batch(
        &self,
        domains: &[u32],
    ) -> nvapi::Result<Option<Vec<nvapi::ClockDomainFreq>>> {
        match self.gpu.clk_domain_freqs_batch(domains) {
            Ok(v) => Ok(Some(v)),
            Err(nvapi::Error::Nvapi(e))
                if matches!(
                    e.status,
                    nvapi::Status::NotSupported | nvapi::Status::NoImplementation
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Write a signed kHz offset into one clock-domain's control record via
    /// the private ClockClient SET_CONTROL (RM 0x2080d01c, ID 0xD14B69CF).
    /// DANGEROUS GPU clock write: snapshots the full GetControl block,
    /// version-gates (magic 0x261A4 V2), patches a copy, SETs, readbacks,
    /// restores on mismatch. `slot` picks the record's value dword (0-7,
    /// slot 0 = the article's signed frequency offset). If `temporary`, the
    /// snapshot is written back and verified restored before returning.
    /// `Ok(None)` where the driver doesn't expose the private interface.
    #[allow(non_snake_case)] // kHz suffix matches the sys-layer field naming
    pub fn set_clk_domain_offset(
        &self,
        domain_bit: u32,
        offset_kHz: i32,
        slot: u32,
        temporary: bool,
    ) -> nvapi::Result<Option<nvapi::ClkDomainControlEntry>> {
        match self
            .gpu
            .set_clk_domain_offset(domain_bit, offset_kHz, slot, temporary)
        {
            Ok(v) => Ok(Some(v)),
            Err(nvapi::Error::Nvapi(e))
                if matches!(
                    e.status,
                    nvapi::Status::NotSupported | nvapi::Status::NoImplementation
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// V/F curve points from the private ClockClient V/F-POINTS read path
    /// (GetInfo 0x8895B510 → GetStatus 0x7FEE9032, RM 0x20809061/0x20809062).
    /// Units live-calibrated vs the public GPC VFP curve. `Ok(None)` where
    /// the driver doesn't expose the private interface.
    pub fn clk_vf_points_private(&self) -> nvapi::Result<Option<nvapi::ClkVfPointsPrivate>> {
        match self.gpu.clk_vf_points_private() {
            Ok(v) => Ok(Some(v)),
            Err(nvapi::Error::Nvapi(e))
                if matches!(
                    e.status,
                    nvapi::Status::NotSupported | nvapi::Status::NoImplementation
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Sparse mode-1 (reverse-volt) V/F calibration over one domain segment
    /// of the private table — see the middle layer for the full contract
    /// (one DOMAIN per call: GPC 0-127, XBAR 128-255, …; every `pt_step`-th
    /// present point gets a mode-1 delta ladder, exact staircase fit,
    /// per-point restore). Compare results against the universal prior
    /// `nvapi::clk_vf_g_prior(def_mhz)`; cache per GPU + driver version.
    /// `Ok(None)` where the driver doesn't expose the private interface.
    #[allow(clippy::too_many_arguments)]
    pub fn clk_vf_calibrate_private(
        &self,
        idx_lo: usize,
        idx_hi: usize,
        pt_step: usize,
        d_step: i64,
        dmax: i64,
    ) -> nvapi::Result<Option<Vec<nvapi::ClkVfCalPoint>>> {
        match self
            .gpu
            .clk_vf_calibrate_private(idx_lo, idx_hi, pt_step, d_step, dmax)
        {
            Ok(v) => Ok(Some(v)),
            Err(nvapi::Error::Nvapi(e))
                if matches!(
                    e.status,
                    nvapi::Status::NotSupported | nvapi::Status::NoImplementation
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// P-State level table (present pstates + per-pstate min/max clock in kHz
    /// for the given clock-domain) from the private PerfPstatesGetInfo (NDA
    /// 0x7B30AE0D). Source of the ref tool's `-pstate` GET listing. `domain` selects
    /// the clock dimension (0=GPC/core by default). `Ok(None)` if not exposed.
    pub fn pstate_levels_domain(
        &self,
        domain: usize,
    ) -> nvapi::Result<Option<nvapi::PStateLevelsInfo>> {
        self.gpu.pstate_levels_domain(domain).map_err(Into::into)
    }

    /// P-State level table for the default (GPC/core) clock-domain.
    pub fn pstate_levels(&self) -> nvapi::Result<Option<nvapi::PStateLevelsInfo>> {
        self.gpu.pstate_levels().map_err(Into::into)
    }

    /// The set of P-State numbers currently locked (NDA 0x9962C97C). Empty when
    /// nothing is locked. `Ok(None)` if the driver doesn't expose it.
    pub fn pstate_lock_status(&self) -> nvapi::Result<Option<Vec<u8>>> {
        self.gpu.pstate_lock_status().map_err(Into::into)
    }

    /// Set the native NVAPI P-State lock (NDA 0x39442CFB, the the ref tool
    /// `-pstate:<index>` SETTER). See [`nvapi::PStateNativeLock`].
    pub fn set_pstate_native(&self, lock: nvapi::PStateNativeLock) -> nvapi::Result<()> {
        self.gpu.set_pstate_native(lock).map_err(Into::into)
    }

    /// Set the GPU frequency perf-cap (NDA 0x32CA4983, the ref tool
    /// `-gpuclk:<MHz>` SETTER). Clamps the perf max/min frequency to a cap
    /// value — NOT an offset, NOT a P-state lock. See [`nvapi::PerfFreqCap`].
    pub fn set_perf_freq_cap(&self, cap: nvapi::PerfFreqCap) -> nvapi::Result<()> {
        self.gpu.set_perf_freq_cap(cap).map_err(Into::into)
    }

    /// Read back the active GPU frequency perf-caps (NDA 0xEFCEDD1F). Returns
    /// one [`nvapi::PerfFreqCapEntry`] per active cap (max/min). Empty where
    /// the driver doesn't expose the private interface.
    pub fn perf_freq_caps(&self) -> nvapi::Result<Vec<nvapi::PerfFreqCapEntry>> {
        self.gpu.perf_freq_caps().map_err(Into::into)
    }

    /// Force the dGPU out of GC6 / GCOFF — the one-shot wake (NDA 0x55590CB2).
    /// Recommended first call before any overclock op on 610 mobile drivers
    /// where the dGPU aggressively enters GC6 and makes ops fail with -220.
    pub fn force_gc6_exit(&self) -> nvapi::Result<()> {
        self.gpu.force_gc6_exit().map_err(Into::into)
    }

    /// Query the dGPU GC6 power state (NDA 0xD387D414, cmd=0). Returns the
    /// driver state: 3 = D0/active (awake), 2 = GC6/idle (powered down),
    /// 0 = OK/no report. Use after [`force_gc6_exit`] to confirm the wake.
    pub fn gc6_query_state(&self) -> nvapi::Result<u32> {
        self.gpu.gc6_query_state().map_err(Into::into)
    }

    /// Force the dGPU awake via the GC6Control cmd=2 path (NDA 0xD387D414).
    /// Returns the post-wake state. Prefer [`Gpu::force_gc6_exit`] for a simpler
    /// wake; this is the struct-based superset (also supports query/sleep).
    pub fn gc6_force_wake(&self) -> nvapi::Result<u32> {
        self.gpu.gc6_force_wake().map_err(Into::into)
    }

    /// Read the target-temperature wall for one policy slot (private GET-prime
    /// 0xC4554575). `Ok(None)` if the driver doesn't expose that slot.
    pub fn target_temperature(&self, policy_index: usize) -> nvapi::Result<Option<f32>> {
        self.gpu
            .target_temperature(policy_index)
            .map_err(Into::into)
    }

    /// Scan every target-temp policy slot and return `(policy_index, celsius)`
    /// for each the driver exposes. Drives `get-temp-thresholds --nvapi`
    /// and per-GPU discovery of the "GPU Target Temperature" wall index (idx 2
    /// on RTX 4060 Laptop — matches nvidia-smi's value and NVML's GpsCurr
    /// channel).
    pub fn target_temperature_policies(&self) -> nvapi::Result<Vec<(usize, f32)>> {
        self.gpu.target_temperature_policies().map_err(Into::into)
    }

    /// Every target-temp policy slot with live current temp + VBIOS
    /// min/default/max range. Drives `get-temp-thresholds --nvapi`.
    pub fn target_temperature_policies_with_info(
        &self,
    ) -> nvapi::Result<Vec<nvapi::TargetTempPolicyEntry>> {
        self.gpu
            .target_temperature_policies_with_info()
            .map_err(Into::into)
    }

    /// Authoritative per-GPU target-temp policy index (private GetInfo
    /// 0x2F69F8E5): GPS index, else acoustics (desktop fallback), else None.
    /// Replaces hardcoding idx 2.
    pub fn target_temp_policy_index(&self) -> nvapi::Result<Option<usize>> {
        self.gpu.target_temp_policy_index().map_err(Into::into)
    }

    /// VBIOS min/default/max target temp (celsius) for one policy slot.
    pub fn target_temperature_info(
        &self,
        policy_index: usize,
    ) -> nvapi::Result<Option<(f32, f32, f32)>> {
        self.gpu
            .target_temperature_info(policy_index)
            .map_err(Into::into)
    }

    /// Set the target-temperature wall for one policy slot (private RMW:
    /// GET-prime 0xC4554575 + SET 0xE097144F). Persists on mobile GPUs. Caller
    /// picks the slot — only idx 2 is confirmed writable (the wall) on RTX 4060
    /// Laptop; other indices may reject or no-op.
    pub fn set_target_temperature(&self, celsius: f32, policy_index: usize) -> nvapi::Result<()> {
        self.gpu
            .set_target_temperature(celsius, policy_index)
            .map_err(Into::into)
    }

    pub fn set_sensor_limits<I: IntoIterator<Item = SensorThrottle>>(
        &self,
        limits: I,
    ) -> nvapi::Result<()> {
        self.gpu
            .thermal_limit_info()
            .map_err(Into::into)
            .and_then(|info| {
                self.gpu
                    .set_thermal_limit(
                        limits
                            .into_iter()
                            .zip(info.into_iter())
                            .map(|(limit, info)| limit.to_limit(info.policy, info.pff.as_ref())),
                    )
                    .map_err(Into::into)
            })
    }

    /// Thermal-channel capability descriptor (undocumented
    /// `NvAPI_GPU_ThermChannelGetInfo`). Best-effort; returns `Ok` with the
    /// descriptor or an error that should be tolerated by callers (some GPUs
    /// stub this call). See [`PhysicalGpu::thermal_channel_info`](nvapi::PhysicalGpu::thermal_channel_info).
    pub fn thermal_channel_info(&self) -> nvapi::Result<ThermalChannelInfo> {
        self.gpu.thermal_channel_info().map_err(Into::into)
    }

    /// Live thermal-channel readings (the STATUS half). `channel_mask` should
    /// come from [`Self::thermal_channel_info`]. Best-effort.
    pub fn thermal_channel_status(&self, channel_mask: u32) -> nvapi::Result<ThermalChannelStatus> {
        self.gpu
            .thermal_channel_status(channel_mask)
            .map_err(Into::into)
    }

    pub fn set_cooler_levels<I: IntoIterator<Item = (FanCoolerId, CoolerSettings)>>(
        &self,
        levels: I,
    ) -> nvapi::Result<()> {
        self.gpu.set_cooler(levels).map_err(Into::into)
    }

    pub fn reset_cooler_levels(&self) -> nvapi::Result<()> {
        self.gpu.restore_cooler_settings(&[]).map_err(Into::into)
    }

    pub fn set_vfp<
        I: Iterator<Item = (usize, KilohertzDelta)>,
        M: Iterator<Item = (usize, KilohertzDelta)>,
    >(
        &self,
        clock_deltas: I,
        mem_deltas: M,
    ) -> nvapi::Result<()> {
        let info = self.vfp_info()??;
        self.gpu
            .set_vfp_table(
                info,
                clock_deltas.map(|(i, d)| (i, d.into())),
                mem_deltas.map(|(i, d)| (i, d.into())),
            )
            .map_err(Into::into)
    }

    pub fn set_vfp_lock_voltage(&self, voltage: Option<Microvolts>) -> nvapi::Result<()> {
        self.gpu
            .set_vfp_locks([ClockLockEntry {
                limit: PerfLimitId::Voltage,
                clock: ClockDomain::Graphics,
                lock_value: voltage.map(ClockLockValue::Voltage),
            }])
            .map_err(Into::into)
    }

    pub fn set_vfp_lock(
        &self,
        domain: ClockDomain,
        frequency: Option<Kilohertz>,
    ) -> nvapi::Result<()> {
        let gpu = match domain {
            ClockDomain::Graphics => true,
            ClockDomain::Memory => false,
            _ => return Err(nvapi::sys::ArgumentRangeError.into()),
        };
        self.gpu
            .set_vfp_locks([
                ClockLockEntry {
                    limit: match gpu {
                        true => PerfLimitId::Gpu,
                        false => PerfLimitId::Memory,
                    },
                    clock: domain,
                    lock_value: frequency.map(ClockLockValue::Frequency),
                },
                ClockLockEntry {
                    limit: match gpu {
                        true => PerfLimitId::GpuLowerbound,
                        false => PerfLimitId::MemoryLowerbound,
                    },
                    clock: domain,
                    lock_value: frequency.map(ClockLockValue::Frequency),
                },
            ])
            .map_err(Into::into)
    }

    pub fn reset_vfp_lock(&self) -> nvapi::Result<()> {
        self.gpu
            .set_vfp_locks(self.gpu.vfp_locks(None)?.into_iter().map(|mut lock| {
                lock.lock_value = None;
                lock
            }))
            .map_err(Into::into)
    }

    pub fn reset_vfp(&self) -> nvapi::Result<()> {
        use std::iter;

        let info = self.vfp_info()??;
        self.gpu
            .set_vfp_table(info, iter::empty(), iter::empty())
            .map_err(Into::into)
    }

    // Driver-side ("OEM"/NVIDIA) OC Scanner control — the family MSI's
    // MSIOCScanner drives on drivers >= 455.00. The scan runs inside the
    // driver; these are thin start/stop/revert controls. See
    // `PhysicalGpu::oem_oc_scanner_start` for the RE provenance.
    pub fn oem_oc_scanner_start(&self) -> nvapi::Result<()> {
        self.gpu.oem_oc_scanner_start().map_err(Into::into)
    }

    pub fn oem_oc_scanner_stop(&self) -> nvapi::Result<()> {
        self.gpu.oem_oc_scanner_stop().map_err(Into::into)
    }

    pub fn oem_oc_scanner_revert(&self) -> nvapi::Result<()> {
        self.gpu.oem_oc_scanner_revert().map_err(Into::into)
    }

    /// Query the last OC scanner run status. Returns Ok(()) if idle/has-
    /// result, or an error status (busy/scanning, not-supported, etc.).
    /// Per-point results are not available through this call — they arrive
    /// via the Register callback (not yet wired in the hi layer).
    pub fn oem_oc_scanner_status(&self) -> nvapi::Result<()> {
        self.gpu.oem_oc_scanner_status().map_err(Into::into)
    }

    /// Force the GPU into a given P-State. `set_type` 0/1/2 all force-lock
    /// (live-tested 4060L); none release. nvapioc uses 2. To unlock, use
    /// SetPstateClientLimits or EnableDynamicPstates instead.
    pub fn set_force_pstate(&self, pstate: u32, set_type: u32) -> nvapi::Result<()> {
        self.gpu
            .set_force_pstate(pstate, set_type)
            .map_err(Into::into)
    }

    /// Restart the display driver (legacy "apply OC" trigger).
    pub fn restart_display_driver(&self) -> nvapi::Result<()> {
        self.gpu.restart_display_driver().map_err(Into::into)
    }

    /// Enable/disable dynamic pstate switching. enable=0 is the release
    /// for a force-locked pstate (SetForcePstate has no unlock of its own).
    pub fn enable_dynamic_pstates(&self, enable: u32) -> nvapi::Result<()> {
        self.gpu.enable_dynamic_pstates(enable).map_err(Into::into)
    }

    /// Battery Boost 2.0 enable/disable. Mobile-only. GPUMonCmd `-bb`.
    pub fn set_bb2_active(&self, enable: bool) -> nvapi::Result<()> {
        self.gpu.set_bb2_active(enable).map_err(Into::into)
    }

    /// Whisper Mode 2.0 enable/disable. Mobile-only. GPUMonCmd `-wm`.
    pub fn set_wm2_active(&self, enable: bool) -> nvapi::Result<()> {
        self.gpu.set_wm2_active(enable).map_err(Into::into)
    }

    /// Whisper Mode 2.0 acoustic mode (Quieter/Quiet/Balanced). GPUMonCmd `-wmMode`.
    pub fn set_wm2_mode(
        &self,
        mode: nvapi::sys::gpu::power::private::Wm2AcousticMode,
    ) -> nvapi::Result<()> {
        self.gpu.set_wm2_mode(mode).map_err(Into::into)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct OvervoltLimit {
    pub domain: VoltageDomain,
    pub voltage: Microvolts,
    pub range: Option<Range<MicrovoltsDelta>>,
}

impl From<BaseVoltage> for OvervoltLimit {
    fn from(v: BaseVoltage) -> Self {
        OvervoltLimit {
            domain: v.voltage_domain,
            voltage: v.voltage,
            range: if v.editable {
                Some(v.voltage_delta.range)
            } else {
                None
            },
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct PStateLimit {
    pub frequency_delta: Option<Range<KilohertzDelta>>,
    pub frequency: Range<Kilohertz>,
    pub voltage: Range<Microvolts>,
    pub voltage_domain: VoltageDomain,
}

impl From<ClockEntry> for PStateLimit {
    fn from(s: ClockEntry) -> Self {
        match s {
            ClockEntry::Range {
                domain: _,
                editable,
                frequency_delta,
                frequency_range,
                voltage_domain,
                voltage_range,
            } => PStateLimit {
                frequency_delta: if editable {
                    Some(frequency_delta.range)
                } else {
                    None
                },
                frequency: frequency_range,
                voltage: voltage_range,
                voltage_domain,
            },
            ClockEntry::Single {
                domain: _,
                editable,
                frequency_delta,
                frequency,
            } => PStateLimit {
                frequency_delta: if editable {
                    Some(frequency_delta.range)
                } else {
                    None
                },
                frequency: Range::from_scalar(frequency),
                voltage: Default::default(),
                voltage_domain: VoltageDomain::Undefined,
            },
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct PowerLimit {
    pub range: Range<Percentage>,
    pub default: Percentage,
}

impl From<PowerInfoEntry> for PowerLimit {
    fn from(info: PowerInfoEntry) -> Self {
        PowerLimit {
            range: Range::range_from(info.range),
            default: info.default_limit.into(),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct SensorLimit {
    pub range: Range<Celsius>,
    pub default: Celsius,
    pub flags: u32,
    pub throttle_curve: Option<PffCurve>,
}

impl From<ThermalInfo> for SensorLimit {
    fn from(info: ThermalInfo) -> Self {
        SensorLimit {
            range: Range::range_from(info.temperature_range),
            default: info.default_temperature.into(),
            flags: info.default_flags,
            throttle_curve: info.pff,
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Default, Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct SensorThrottle {
    pub value: Celsius,
    pub remove_tdp_limit: bool,
    pub curve: Option<PffCurve>,
}

impl SensorThrottle {
    pub fn to_limit(&self, policy: ThermalPolicyId, info: Option<&PffCurve>) -> ThermalLimit {
        ThermalLimit {
            policy,
            value: self.value.into(),
            remove_tdp_limit: self.remove_tdp_limit,
            pff: self.curve.as_ref().map(|pff| PffStatus {
                values: pff.points.iter().map(|p| p.y.into()).collect(),
                curve: match info {
                    Some(curve) => curve.clone(),
                    None => pff.clone(),
                },
            }),
        }
    }

    pub fn from_limit(limit: &ThermalLimit) -> Self {
        Self {
            value: limit.value.into(),
            remove_tdp_limit: limit.remove_tdp_limit,
            curve: limit.pff.as_ref().map(|pff| pff.curve()),
        }
    }

    pub fn from_default(info: SensorLimit) -> Self {
        Self {
            value: info.default,
            curve: info.throttle_curve.clone(),
            remove_tdp_limit: false,
        }
    }
}

impl From<Celsius> for SensorThrottle {
    fn from(value: Celsius) -> Self {
        Self {
            value,
            ..Default::default()
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct SensorDesc {
    /// Thermal controller. Always `GpuInternal` for every sensor NVAPI
    /// exposes here, so it is omitted from serialization as redundant.
    #[cfg_attr(
        feature = "serde",
        serde(skip, default = "SensorDesc::default_controller")
    )]
    pub controller: ThermalController,
    pub target: ThermalTarget,
    pub range: Range<Celsius>,
    /// The RTSS ThermChannel channel index this reading comes from. Channel
    /// 0 = GPU_AVG (Core), 1 = GPU_MAX (Hot Spot), etc. — indexed directly by
    /// GetInfo's priChIdx. `None` for documented sensors that don't come from
    /// the ThermChannel API.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub channel_num: Option<u32>,
    /// `NV_GPU_THERMAL_THERM_CHANNEL_TYPE` (0=GPU_AVG, 1=GPU_MAX, 2=BOARD,
    /// 3=MEMORY, 4=PWR_SUPPLY, 255=unclassified). Research metadata from GetInfo.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub channel_type: Option<u32>,
    /// Software offset (GetInfo `offsetSw`; semantics undocumented). Research use.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub offset_sw: Option<i32>,
    /// Hardware offset (GetInfo `offsetHw`; semantics undocumented). Research use.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub offset_hw: Option<i32>,
    /// Fixed-point scaling factor (GetInfo `scaling`; semantics undocumented).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub scaling: Option<i32>,
    /// Cross-reference set when this channel is the `thermDevProvIdx==1` half
    /// of a paired reading from the same physical sensor as another channel.
    /// The driver has already applied `offset_hw` to this channel's STATUS
    /// reading; the paired `(dev, 0)` channel has not. Display-only annotation.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub same_sensor_as: Option<u32>,
}

impl From<Sensor> for SensorDesc {
    fn from(sensor: Sensor) -> Self {
        SensorDesc {
            controller: sensor.controller,
            target: sensor.target,
            range: sensor.default_temperature_range,
            channel_num: None,
            channel_type: None,
            offset_sw: None,
            offset_hw: None,
            scaling: None,
            same_sensor_as: None,
        }
    }
}

impl SensorDesc {
    /// Controller value used when deserializing a `SensorDesc` whose
    /// `controller` field was skipped during serialization.
    fn default_controller() -> ThermalController {
        ThermalController::GpuInternal
    }
}

/// Build a `SensorDesc` for one RTSS thermal channel, attaching the channel
/// index and (when available) the GetInfo metadata fields for research. The
/// sensor is identified by its `channel_type` (set from the GetInfo record) —
/// there is no free-form name.
fn sensor_desc_for_channel(
    target: ThermalTarget,
    channel: u32,
    info: Option<&nvapi::ChannelInfo>,
) -> SensorDesc {
    let mut desc = SensorDesc {
        controller: ThermalController::GpuInternal,
        target,
        range: Range::default(),
        channel_num: Some(channel),
        channel_type: info.map(|c| c.ch_type),
        offset_sw: info.map(|c| c.offset_sw),
        offset_hw: info.map(|c| c.offset_hw),
        scaling: info.map(|c| c.scaling),
        same_sensor_as: None,
    };
    // If the GetInfo record carried a min/max range, surface it. These are in
    // the same celsius*256 fixed-point as the live readings (see `scaling`),
    // so decode to integer degrees (truncating divide, matching the /256
    // decode convention) for a sensible display.
    if let Some(c) = info {
        if c.min_temp != 0 || c.max_temp != 0 {
            desc.range = Range {
                min: Celsius(c.min_temp / 256),
                max: Celsius(c.max_temp / 256),
            };
        }
    }
    desc
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct VfpPoint {
    pub point_type: VfPointType,
    pub default_frequency: Kilohertz,
    pub frequency: Kilohertz,
    pub voltage: Microvolts,
}

impl VfpPoint {
    pub fn is_editable(&self) -> bool {
        self.point_type == VfPointType::Prog
    }
}

impl<T: Default + PartialEq + Copy> From<VfpEntry<T>> for VfpPoint
where
    Kilohertz: From<T>,
{
    fn from(v: VfpEntry<T>) -> Self {
        debug_assert!(v.configured().voltage == v.current.voltage);
        if !v.overclocked.is_empty() {
            debug_assert!(v.overclocked.voltage == v.current.voltage);
            debug_assert!(v.current.frequency == v.overclocked.frequency);
        }
        VfpPoint {
            point_type: v.point_type,
            default_frequency: v.default.frequency.into(),
            frequency: v.configured().frequency.into(),
            voltage: v.configured().voltage,
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct VfpTable {
    pub graphics: BTreeMap<usize, VfpPoint>,
    pub memory: BTreeMap<usize, VfpPoint>,
}

impl From<VfpCurve> for VfpTable {
    fn from(v: VfpCurve) -> Self {
        VfpTable {
            graphics: collect_domain(&v.points, ClockDomain::Graphics),
            memory: collect_domain(&v.points, ClockDomain::Memory),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct VfpDeltas {
    pub graphics: BTreeMap<usize, KilohertzDelta>,
    pub memory: BTreeMap<usize, KilohertzDelta>,
}

impl From<ClockTable> for VfpDeltas {
    fn from(c: ClockTable) -> Self {
        VfpDeltas {
            graphics: collect_domain(&c.delta_points, ClockDomain::Graphics),
            memory: collect_domain(&c.delta_points, ClockDomain::Memory),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct VfPoint {
    pub point_type: VfPointType,
    pub voltage: Microvolts,
    pub frequency: Kilohertz,
    pub delta: KilohertzDelta,
    pub default_frequency: Kilohertz,
}

impl VfPoint {
    pub fn new(point: VfpPoint, delta: KilohertzDelta) -> Self {
        VfPoint {
            point_type: point.point_type,
            voltage: point.voltage,
            frequency: point.frequency,
            default_frequency: point.default_frequency,
            delta,
        }
    }

    pub fn is_editable(&self) -> bool {
        self.point_type == VfPointType::Prog
    }
}
