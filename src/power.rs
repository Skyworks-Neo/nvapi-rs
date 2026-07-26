use crate::sys;
use crate::sys::gpu::power;
use crate::types::RawConversion;
use log::trace;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub use sys::gpu::power::private::{PowerMonitorChannelType, PowerRail};

/// Per-channel capability descriptor (decoded from
/// `NV_GPU_POWER_MONITOR_POWER_CHANNEL_INFO_V2`). Research-grade: most fields
/// are passed through opaquely; `channel_type` and `pwr_rail` are decoded enums.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerChannelInfo {
    pub pwr_device_mask: u32,
    pub pwr_offset_mw: i32,
    pub pwr_limit_mw: u32,
    /// Decoded channel type (`None` if the raw value is not a known variant).
    pub channel_type: Option<PowerMonitorChannelType>,
    /// Decoded power rail (`None` if the raw value is not a known variant).
    pub pwr_rail: Option<PowerRail>,
    /// Raw channel type (c_int) — kept for unknown-variant diagnostics.
    pub channel_type_raw: i32,
    /// Raw power rail (c_int) — kept for unknown-variant diagnostics.
    pub pwr_rail_raw: i32,
    pub volt_fixed_uv: u32,
    pub pwr_corr_slope: u32,
    pub curr_corr_slope: u32,
    pub curr_corr_offset_ma: i32,
}

impl From<&power::private::NV_GPU_POWER_MONITOR_POWER_CHANNEL_INFO_V2> for PowerChannelInfo {
    fn from(c: &power::private::NV_GPU_POWER_MONITOR_POWER_CHANNEL_INFO_V2) -> Self {
        PowerChannelInfo {
            pwr_device_mask: c.pwr_device_mask,
            pwr_offset_mw: c.pwr_offset_mw,
            pwr_limit_mw: c.pwr_limit_mw,
            channel_type: PowerMonitorChannelType::from_raw(c.channel_type).ok(),
            pwr_rail: PowerRail::from_raw(c.pwr_rail).ok(),
            channel_type_raw: c.channel_type,
            pwr_rail_raw: c.pwr_rail,
            volt_fixed_uv: c.volt_fixed_uv,
            pwr_corr_slope: c.pwr_corr_slope,
            curr_corr_slope: c.curr_corr_slope,
            curr_corr_offset_ma: c.curr_corr_offset_ma,
        }
    }
}

/// Power-monitor capability/topology descriptor (decoded from the INFO half of
/// the PowerMonitor pair, ID 0xC12EB19E). `supported` gates whether the STATUS
/// half will yield live readings on this GPU/driver.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PowerMonitorInfo {
    /// Whether GetStatus is expected to return live readings on this GPU.
    pub supported: bool,
    pub sampling_period_ms: u32,
    pub sample_count: u32,
    pub channel_mask: u32,
    pub total_gpu_power_channel_mask: u32,
    /// Channel index carrying total GPU power (if any).
    pub total_gpu_channel_idx: Option<u8>,
    /// `(channel_index, descriptor)` for each populated channel.
    pub channels: Vec<(usize, PowerChannelInfo)>,
}

impl RawConversion for power::private::NV_GPU_POWER_MONITOR_GET_INFO {
    type Target = PowerMonitorInfo;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        let channels = self
            .channels()
            .map(|(i, c)| (i, PowerChannelInfo::from(c)))
            .collect();
        Ok(PowerMonitorInfo {
            supported: self.b_supported.get(),
            sampling_period_ms: self.sampling_period_ms,
            sample_count: self.sample_count,
            channel_mask: self.channel_mask,
            total_gpu_power_channel_mask: self.total_gpu_power_channel_mask,
            total_gpu_channel_idx: {
                let idx = self.total_gpu_channel_idx as usize;
                (idx < power::private::NV_GPU_POWER_MONITOR_POWER_CHANNELS_MAX)
                    .then_some(self.total_gpu_channel_idx)
            },
            channels,
        })
    }
}

/// Per-channel live reading (decoded from
/// `NV_GPU_POWER_MONITOR_POWER_CHANNEL_STATUS_V2`). Average/min/max power in
/// milliwatts, current in milliamps, voltage in microvolts, energy in
/// milli-Joules.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PowerChannelStatus {
    pub pwr_avg_mw: u32,
    pub pwr_min_mw: u32,
    pub pwr_max_mw: u32,
    pub curr_ma: u32,
    pub volt_uv: u32,
    pub energy_mj: u64,
}

impl From<&power::private::NV_GPU_POWER_MONITOR_POWER_CHANNEL_STATUS_V2> for PowerChannelStatus {
    fn from(c: &power::private::NV_GPU_POWER_MONITOR_POWER_CHANNEL_STATUS_V2) -> Self {
        PowerChannelStatus {
            pwr_avg_mw: c.pwr_avg_mw(),
            pwr_min_mw: c.pwr_min_mw(),
            pwr_max_mw: c.pwr_max_mw(),
            curr_ma: c.curr_ma(),
            volt_uv: c.volt_uv(),
            energy_mj: c.energy_mj(),
        }
    }
}

/// Power-monitor live readings (decoded from the STATUS half, ID 0xF40238EF).
/// `total_gpu_power_mw` is the board total; `channels` carries per-rail wattage
/// for each channel whose bit was set in the `channel_mask` passed to GetStatus.
/// Each channel's `rail` is filled by merging the INFO descriptor (same index);
/// it is `None` until [`PowerMonitorStatus::merge_info`] is called.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PowerMonitorStatus {
    pub channel_mask: u32,
    pub total_gpu_power_mw: u32,
    /// `(channel_index, rail, reading)` for each populated channel.
    pub channels: Vec<(usize, Option<PowerRail>, PowerChannelStatus)>,
}

impl PowerMonitorStatus {
    /// Reading at a specific channel index, if populated.
    pub fn get(&self, channel: usize) -> Option<&PowerChannelStatus> {
        self.channels
            .iter()
            .find(|(i, _, _)| *i == channel)
            .map(|(_, _, s)| s)
    }

    /// Attach each channel's rail label from the INFO descriptor (matched by
    /// channel index). Callers that have both halves should invoke this so the
    /// per-channel entries carry human-readable rail names.
    pub fn merge_info(&mut self, info: &PowerMonitorInfo) {
        for (idx, rail, _) in &mut self.channels {
            *rail = info
                .channels
                .iter()
                .find(|(c, _)| c == idx)
                .and_then(|(_, d)| d.pwr_rail);
        }
    }
}

impl RawConversion for power::private::NV_GPU_POWER_MONITOR_GET_STATUS {
    type Target = PowerMonitorStatus;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:#?})", self);
        let channels = (0..power::private::NV_GPU_POWER_MONITOR_POWER_CHANNELS_MAX)
            .filter_map(|i| {
                self.channel(i).map(|c| (i, None, PowerChannelStatus::from(c)))
            })
            .collect();
        Ok(PowerMonitorStatus {
            channel_mask: self.channel_mask,
            total_gpu_power_mw: self.total_gpu_power_mw,
            channels,
        })
    }
}
