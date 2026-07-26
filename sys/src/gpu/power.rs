/// Undocumented API
pub mod private {
    use crate::prelude_::*;

    nvstruct! {
        pub struct NV_GPU_CLIENT_VOLT_RAILS_STATUS_V1 {
            pub version: NvVersion,
            pub flags: u32,
            pub zero: Padding<[u32; 8]>,
            pub value_uV: u32,
            pub unknown: Padding<[u32; 8]>,
        }
    }

    nvversion! { @=NV_GPU_CLIENT_VOLT_RAILS_STATUS NV_GPU_CLIENT_VOLT_RAILS_STATUS_V1(1) = 76 }

    nvapi! {
        /// Pascal and later
        pub unsafe fn NvAPI_GPU_ClientVoltRailsGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pVoltageStatus: *mut NV_GPU_CLIENT_VOLT_RAILS_STATUS) -> NvAPI_Status;
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_VOLT_RAILS_CONTROL_V1 {
            pub version: NvVersion,
            /// uiDelta
            pub percent: u32, // apparently actually i32?
            pub unknown: Padding<[u32; 8]>,
        }
    }

    nvversion! { @=NV_GPU_CLIENT_VOLT_RAILS_CONTROL NV_GPU_CLIENT_VOLT_RAILS_CONTROL_V1(1) }

    nvapi! {
        /// Pascal and later
        pub unsafe fn NvAPI_GPU_ClientVoltRailsGetControl(hPhysicalGPU: NvPhysicalGpuHandle, pVoltboostPercent: *mut NV_GPU_CLIENT_VOLT_RAILS_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Pascal and later
        pub unsafe fn NvAPI_GPU_ClientVoltRailsSetControl(hPhysicalGPU: NvPhysicalGpuHandle, pVoltboostPercent: *const NV_GPU_CLIENT_VOLT_RAILS_CONTROL) -> NvAPI_Status;
    }

    nvstruct! {
        pub struct NV_GPU_CLOCK_CLIENT_CLK_VF_POINT {
            pub freq_kHz: u32,
            pub voltage_uV: u32,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_STATUS_V1 {
            pub clock_type: u32,
            pub point: NV_GPU_CLOCK_CLIENT_CLK_VF_POINT,
            pub unknown: Padding<[u32; 4]>,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_STATUS_V3 {
            pub clock_type: u32,
            pub point: NV_GPU_CLOCK_CLIENT_CLK_VF_POINT,
            pub point_default: NV_GPU_CLOCK_CLIENT_CLK_VF_POINT,
            pub unknown0: Padding<[u32; 8]>,
            /// overclockedFrequencyKhz and millivoltage
            pub point_overclocked: NV_GPU_CLOCK_CLIENT_CLK_VF_POINT,
            pub unknown: Padding<[u32; 348/4 - (7 + 8)]>,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_V1 {
            pub version: NvVersion,
            pub mask: ClockMask,
            pub unknown: Padding<[u32; 8]>,
            pub entries: Array<[NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_STATUS_V1; 255]>,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_V3 {
            pub version: NvVersion,
            pub mask: ClockMask,
            pub unknown: Padding<[u8; 0x44]>,
            pub entries: Array<[NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_STATUS_V3; 255]>,
        }
    }

    nvversion! { NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_V1(1) = 0x1c28 }
    nvversion! { NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_V1(2) = 0x1c28 }
    nvversion! { @=NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_V3(3) = 0x15b0c }

    nvapi! {
        /// Pascal and later
        pub unsafe fn NvAPI_GPU_ClockClientClkVfPointsGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pVfpCurve: *mut NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS) -> NvAPI_Status;
    }

    nvenum! {
        pub enum NV_GPU_CLIENT_POWER_POLICIES_POLICY_ID / PowerPolicyId {
            NV_GPU_CLIENT_POWER_POLICIES_POLICY_ID_DEFAULT / Default = 0,
        }
    }

    nvenum_display! {
        PowerPolicyId => {
            Default = "Board Power Limit",
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_POWER_POLICIES_INFO_ENTRY_V1 {
            pub policy_id: NV_GPU_CLIENT_POWER_POLICIES_POLICY_ID,
            pub b: u32,
            pub c: u32,
            pub min_power: u32,
            pub e: u32,
            pub f: u32,
            pub def_power: u32,
            pub h: u32,
            pub i: u32,
            pub max_power: u32,
            pub k: u32, // 0
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_POWER_POLICIES_INFO_V1 {
            pub version: NvVersion,
            pub valid: u8,
            pub count: u8,
            pub padding: Padding<[u8; 2]>,
            pub entries: Array<[NV_GPU_CLIENT_POWER_POLICIES_INFO_ENTRY_V1; 4]>,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_POWER_POLICIES_INFO_ENTRY_V2 {
            pub policy_id: NV_GPU_CLIENT_POWER_POLICIES_POLICY_ID,
            pub unknown0: Padding<[u32; 3]>,
            pub min_power: u32,
            pub unknown1: Padding<[u32; 2]>,
            pub def_power: u32,
            pub unknown2: Padding<[u32; 2]>,
            pub max_power: u32,
            pub padding: Padding<[u32; 560/4 - 11]>,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_POWER_POLICIES_INFO_V2 {
            pub version: NvVersion,
            pub valid: u8,
            pub count: u8,
            pub padding: Padding<[u8; 2]>,
            pub entries: Array<[NV_GPU_CLIENT_POWER_POLICIES_INFO_ENTRY_V2; 4]>,
        }
    }

    impl NV_GPU_CLIENT_POWER_POLICIES_INFO_V2 {
        pub fn entries(&self) -> &[NV_GPU_CLIENT_POWER_POLICIES_INFO_ENTRY_V2] {
            &self.entries[..self.count as usize]
        }
    }

    nvversion! { NV_GPU_CLIENT_POWER_POLICIES_INFO_V1(1) }
    nvversion! { @=NV_GPU_CLIENT_POWER_POLICIES_INFO NV_GPU_CLIENT_POWER_POLICIES_INFO_V2(2) = 2248 }

    nvapi! {
        pub unsafe fn NvAPI_GPU_ClientPowerPoliciesGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pPowerInfo: *mut NV_GPU_CLIENT_POWER_POLICIES_INFO) -> NvAPI_Status;
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_POWER_POLICIES_STATUS_ENTRY_V1 {
            pub policy_id: NV_GPU_CLIENT_POWER_POLICIES_POLICY_ID,
            pub b: u32,
            pub power_target: u32,
            pub d: u32,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_POWER_POLICIES_STATUS_V1 {
            pub version: NvVersion,
            pub count: u32,
            pub entries: Array<[NV_GPU_CLIENT_POWER_POLICIES_STATUS_ENTRY_V1; 4]>,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_POWER_POLICIES_STATUS_ENTRY_V2 {
            pub policy_id: NV_GPU_CLIENT_POWER_POLICIES_POLICY_ID,
            pub unknown: Padding<[u32; 1]>,
            pub flags: u32,
            pub power_target: u32,
            pub padding: Padding<[u32; 340/4 - 4]>,
        }
    }

    impl NV_GPU_CLIENT_POWER_POLICIES_STATUS_ENTRY_V2 {
        /// Unsure what this is but flag should be cleared for SetStatus, maybe?
        pub fn set_flag(&mut self, value: bool) {
            self.flags = self.flags & 0xfffffffe | if value { 1 } else { 0 }
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_POWER_POLICIES_STATUS_V2 {
            pub version: NvVersion,
            pub count: u32,
            pub entries: Array<[NV_GPU_CLIENT_POWER_POLICIES_STATUS_ENTRY_V2; 4]>,
        }
    }

    nvversion! { NV_GPU_CLIENT_POWER_POLICIES_STATUS_V1(1) }
    nvversion! { @=NV_GPU_CLIENT_POWER_POLICIES_STATUS NV_GPU_CLIENT_POWER_POLICIES_STATUS_V2(2) = 1368 }

    nvapi! {
        pub unsafe fn NvAPI_GPU_ClientPowerPoliciesGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pPowerStatus: *mut NV_GPU_CLIENT_POWER_POLICIES_STATUS) -> NvAPI_Status;
    }

    nvapi! {
        pub unsafe fn NvAPI_GPU_ClientPowerPoliciesSetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pPowerStatus: *const NV_GPU_CLIENT_POWER_POLICIES_STATUS) -> NvAPI_Status;
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_POWER_TOPOLOGY_INFO_V1 {
            pub version: NvVersion,
            pub valid: u8,
            pub count: u8,
            pub padding: Padding<[u8; 2]>,
            pub channels: Array<[NV_GPU_CLIENT_POWER_TOPOLOGY_CHANNEL_ID; 4]>,
        }
    }

    impl NV_GPU_CLIENT_POWER_TOPOLOGY_INFO_V1 {
        pub fn channels(&self) -> &[NV_GPU_CLIENT_POWER_TOPOLOGY_CHANNEL_ID] {
            &self.channels[..self.count as usize]
        }
    }

    nvversion! { @=NV_GPU_CLIENT_POWER_TOPOLOGY_INFO NV_GPU_CLIENT_POWER_TOPOLOGY_INFO_V1(1) = 24 }

    nvapi! {
        pub unsafe fn NvAPI_GPU_ClientPowerTopologyGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pPowerTopo: *mut NV_GPU_CLIENT_POWER_TOPOLOGY_INFO) -> NvAPI_Status;
    }

    nvenum! {
        pub enum NV_GPU_CLIENT_POWER_TOPOLOGY_CHANNEL_ID / PowerTopologyChannelId {
            NV_GPU_CLIENT_POWER_TOPOLOGY_CHANNEL_ID_TOTAL_GPU_POWER / TotalGpuPower = 0,
            NV_GPU_CLIENT_POWER_TOPOLOGY_CHANNEL_ID_NORMALIZED_TOTAL_POWER / NormalizedTotalPower = 1,
        }
    }

    nvenum_display! {
        PowerTopologyChannelId => {
            TotalGpuPower = "Total Power",
            NormalizedTotalPower = "Normalized Power",
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_POWER_TOPOLOGY_STATUS_ENTRY {
            pub channel: NV_GPU_CLIENT_POWER_TOPOLOGY_CHANNEL_ID,
            pub unknown0: u32,
            pub power: u32,
            pub unknown1: u32,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_POWER_TOPOLOGY_STATUS_V1 {
            pub version: NvVersion,
            pub count: u32,
            pub entries: Array<[NV_GPU_CLIENT_POWER_TOPOLOGY_STATUS_ENTRY; 4]>,
        }
    }

    impl NV_GPU_CLIENT_POWER_TOPOLOGY_STATUS_V1 {
        pub fn entries(&self) -> &[NV_GPU_CLIENT_POWER_TOPOLOGY_STATUS_ENTRY] {
            &self.entries[..self.count as usize]
        }
    }

    nvversion! { @=NV_GPU_CLIENT_POWER_TOPOLOGY_STATUS NV_GPU_CLIENT_POWER_TOPOLOGY_STATUS_V1(1) = 72 }

    nvapi! {
        pub unsafe fn NvAPI_GPU_ClientPowerTopologyGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pPowerTopo: *mut NV_GPU_CLIENT_POWER_TOPOLOGY_STATUS) -> NvAPI_Status;
    }

    nvbits! {
        pub enum NV_GPU_PERF_FLAGS / PerfFlags {
            NV_GPU_PERF_FLAGS_POWER_LIMIT / POWER_LIMIT = 1,
            NV_GPU_PERF_FLAGS_THERMAL_LIMIT / THERMAL_LIMIT = 2,
            /// Reliability voltage
            NV_GPU_PERF_FLAGS_VOLTAGE_REL_LIMIT / VOLTAGE_REL_LIMIT = 4,
            /// Operating voltage
            NV_GPU_PERF_FLAGS_VOLTAGE_OP_LIMIT / VOLTAGE_OP_LIMIT = 8,
            /// GPU utilization
            NV_GPU_PERF_FLAGS_NO_LOAD_LIMIT / NO_LOAD_LIMIT = 16,
            /// Never seen this
            NV_GPU_PERF_FLAGS_UNKNOWN_32 / UNKNOWN_32 = 32,
        }
    }

    nvenum_display! {
        PerfFlags => {
            POWER_LIMIT = "Power",
            THERMAL_LIMIT = "Temperature",
            VOLTAGE_REL_LIMIT = "Reliability Voltage",
            VOLTAGE_OP_LIMIT = "Operating Voltage",
            NO_LOAD_LIMIT = "No Load",
            UNKNOWN_32 = "Unknown32",
            _ = _,
        }
    }

    nvstruct! {
        pub struct NV_GPU_PERF_POLICIES_INFO_PARAMS_V1 {
            pub version: NvVersion,
            pub maxUnknown: u32,
            pub limitSupport: NV_GPU_PERF_FLAGS,
            pub padding: Padding<[u32; 16]>,
        }
    }

    nvversion! { @=NV_GPU_PERF_POLICIES_INFO_PARAMS NV_GPU_PERF_POLICIES_INFO_PARAMS_V1(1) = 76 }

    nvapi! {
        pub unsafe fn NvAPI_GPU_PerfPoliciesGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pPerfInfo: *mut NV_GPU_PERF_POLICIES_INFO_PARAMS) -> NvAPI_Status;
    }

    nvstruct! {
        pub struct NV_GPU_PERF_POLICIES_STATUS_PARAMS_V1 {
            pub version: NvVersion,
            pub flags: u32,
            /// nanoseconds
            pub timer: u64,
            /// - 1 = power limit
            /// - 2 = temp limit
            /// - 4 = voltage limit
            /// - 8 = only got with 15 in driver crash
            /// - 16 = no-load limit
            pub limits: NV_GPU_PERF_FLAGS,
            pub zero0: u32,
            /// - 1 on load
            /// - 3 in low clocks
            /// - 7 in idle
            pub unknown: u32,
            pub zero1: u32,
            /// nanoseconds
            pub timers: [u64; 3],
            pub padding: Padding<[u32; 326]>,
        }
    }

    nvversion! { @=NV_GPU_PERF_POLICIES_STATUS_PARAMS NV_GPU_PERF_POLICIES_STATUS_PARAMS_V1(1) = 0x550 }

    nvapi! {
        pub unsafe fn NvAPI_GPU_PerfPoliciesGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pPerfStatus: *mut NV_GPU_PERF_POLICIES_STATUS_PARAMS) -> NvAPI_Status;
    }

    nvstruct! {
        pub struct NV_VOLT_STATUS_V1 {
            pub version: NvVersion,
            pub flags: u32,
            /// unsure
            pub count: u32,
            pub unknown: u32,
            pub value_uV: u32,
            pub buf1: Padding<[u32; 30]>,
        }
    }

    nvversion! { @=NV_VOLT_STATUS NV_VOLT_STATUS_V1(1) = 140 }

    nvapi! {
        /// Maxwell only
        pub unsafe fn NvAPI_GPU_GetVoltageDomainsStatus(hPhysicalGPU: NvPhysicalGpuHandle, pVoltStatus: *mut NV_VOLT_STATUS) -> NvAPI_Status;
    }

    nvapi! {
        /// Maxwell only
        pub unsafe fn NvAPI_GPU_GetVoltageStep(hPhysicalGPU: NvPhysicalGpuHandle, pVoltStep: *mut NV_VOLT_STATUS) -> NvAPI_Status;
    }

    nvstruct! {
        pub struct NV_VOLT_TABLE_ENTRY {
            pub voltage_domain: u32,
            pub voltage_uV: u32,
            pub unknown: Padding<[u32; 257]>,
        }
    }

    nvstruct! {
        pub struct NV_VOLT_TABLE_V1 {
            pub version: NvVersion,
            pub flags: u32,
            pub count: u32,
            pub entries: Array<[NV_VOLT_TABLE_ENTRY; 16]>,
        }
    }

    impl NV_VOLT_TABLE_V1 {
        pub fn entries(&self) -> &[NV_VOLT_TABLE_ENTRY] {
            &self.entries[..self.count as usize]
        }
    }

    nvversion! { @=NV_VOLT_TABLE NV_VOLT_TABLE_V1(1) = 0x40cc }

    nvapi! {
        /// Maxwell only
        pub unsafe fn NvAPI_GPU_GetVoltages(hPhysicalGPU: NvPhysicalGpuHandle, pVolts: *mut NV_VOLT_TABLE) -> NvAPI_Status;
    }

    // ------------------------------------------------------------------
    // PowerMonitor — per-channel / per-rail power monitoring (NDA-private).
    // IDs 0xC12EB19E (GetInfo) + 0xF40238EF (GetStatus). Reversed from RTSS
    // (RivaTuner) source `NVAPIInterface.h` + nvapi64_impl.dll handlers.
    // GetInfo returns a capability/topology descriptor (which of up to 32
    // power channels exist, each channel's type/rail/limit); GetStatus returns
    // the live per-channel wattage/current/voltage/energy plus a top-level
    // total GPU power. Best-effort only — the STATUS handler is stubbed
    // (returns -104 NVIDIA_DEVICE_NOT_FOUND) on some GPU/driver combos; probe
    // with GetInfo's `b_supported` before calling GetStatus.
    // ------------------------------------------------------------------

    /// Number of power channels the params structs reserve room for.
    pub const NV_GPU_POWER_MONITOR_POWER_CHANNELS_MAX: usize = 32;

    nvenum! {
        /// Power-monitor channel type (RTSS `NV_GPU_POWER_MONITOR_POWER_CHANNEL_TYPE`).
        /// Research semantics; opaque pass-through.
        pub enum NV_GPU_POWER_MONITOR_POWER_CHANNEL_TYPE / PowerMonitorChannelType {
            NV_GPU_POWER_MONITOR_POWER_CHANNEL_TYPE_DEFAULT / Default = 0,
            NV_GPU_POWER_MONITOR_POWER_CHANNEL_TYPE_SUMMATION / Summation = 1,
            NV_GPU_POWER_MONITOR_POWER_CHANNEL_TYPE_ESTIMATION / Estimation = 2,
            NV_GPU_POWER_MONITOR_POWER_CHANNEL_TYPE_SLOW / Slow = 3,
            NV_GPU_POWER_MONITOR_POWER_CHANNEL_TYPE_GEMINI_CORRECTION / GeminiCorrection = 4,
            NV_GPU_POWER_MONITOR_POWER_CHANNEL_TYPE_1X / OneX = 5,
            NV_GPU_POWER_MONITOR_POWER_CHANNEL_TYPE_SENSOR / Sensor = 6,
            NV_GPU_POWER_MONITOR_POWER_CHANNEL_TYPE_PSTATE_ESTIMATION_LUT / PstateEstimationLut = 7,
            NV_GPU_POWER_MONITOR_POWER_CHANNEL_TYPE_SENSOR_CLIENT_ALIGNED / SensorClientAligned = 8,
        }
    }

    nvenum_display! {
        PowerMonitorChannelType => _
    }

    nvenum! {
        /// Power rail a channel measures (RTSS `NV_GPU_POWER_CHANNEL_POWER_RAIL`).
        /// OUTPUT_* are on-GPU regulator outputs; INPUT_* are board input rails.
        pub enum NV_GPU_POWER_CHANNEL_POWER_RAIL / PowerRail {
            NV_GPU_POWER_CHANNEL_POWER_RAIL_UNKNOWN / Unknown = 0,
            // --- output rails (on-GPU regulator outputs) ---
            NV_GPU_POWER_CHANNEL_POWER_RAIL_OUTPUT_NVVDD / OutputNvvdd = 1,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_OUTPUT_FBVDD / OutputFbvdd = 2,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_OUTPUT_FBVDDQ / OutputFbvddq = 3,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_OUTPUT_FBVDD_Q / OutputFbvddQ = 4,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_OUTPUT_PEXVDD / OutputPexvdd = 5,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_OUTPUT_A3V3 / OutputA3v3 = 6,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_OUTPUT_3V3NV / Output3v3nv = 7,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_OUTPUT_TOTAL_GPU / OutputTotalGpu = 8,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_OUTPUT_FBVDDQ_GPU / OutputFbvddqGpu = 9,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_OUTPUT_FBVDDQ_MEM / OutputFbvddqMem = 10,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_OUTPUT_SRAM / OutputSram = 11,
            // --- input rails (board input) ---
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_PEX12V1 / InputPex12v1 = 222,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_TOTAL_BOARD2 / InputTotalBoard2 = 223,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_HIGH_VOLT0 / InputHighVolt0 = 224,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_HIGH_VOLT1 / InputHighVolt1 = 225,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_NVVDD1 / InputNvvdd1 = 226,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_NVVDD2 / InputNvvdd2 = 227,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_EXT12V_8PIN2 / InputExt12v8pin2 = 228,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_EXT12V_8PIN3 / InputExt12v8pin3 = 229,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_EXT12V_8PIN4 / InputExt12v8pin4 = 230,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_EXT12V_8PIN5 / InputExt12v8pin5 = 231,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_MISC0 / InputMisc0 = 232,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_MISC1 / InputMisc1 = 233,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_MISC2 / InputMisc2 = 234,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_MISC3 / InputMisc3 = 235,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_USBC0 / InputUsbc0 = 236,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_USBC1 / InputUsbc1 = 237,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_FAN0 / InputFan0 = 238,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_FAN1 / InputFan1 = 239,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_SRAM / InputSram = 240,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_PWR_SRC_PP / InputPwrSrcPp = 241,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_3V3_PP / Input3v3Pp = 242,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_3V3_MAIN / Input3v3Main = 243,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_3V3_AON / Input3v3Aon = 244,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_TOTAL_BOARD / InputTotalBoard = 245,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_NVVDD / InputNvvdd = 246,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_FBVDD / InputFbvdd = 247,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_FBVDDQ / InputFbvddq = 248,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_FBVDD_Q / InputFbvddQ = 249,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_EXT12V_8PIN0 / InputExt12v8pin0 = 250,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_EXT12V_8PIN1 / InputExt12v8pin1 = 251,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_EXT12V_6PIN0 / InputExt12v6pin0 = 252,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_EXT12V_6PIN1 / InputExt12v6pin1 = 253,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_PEX3V3 / InputPex3v3 = 254,
            NV_GPU_POWER_CHANNEL_POWER_RAIL_INPUT_PEX12V / InputPex12v = 255,
        }
    }

    nvenum_display! {
        PowerRail => _
    }

    nvstruct! {
        /// Per-channel capability descriptor (RTSS
        /// `NV_GPU_POWER_MONITOR_POWER_CHANNEL_INFO_V2`). The trailing `data`
        /// union is a 16-byte region whose layout depends on `channel_type`
        /// (1x / sensor / summation / pstate-estimation-LUT / …); kept as raw
        /// bytes for research, not decoded.
        pub struct NV_GPU_POWER_MONITOR_POWER_CHANNEL_INFO_V2 {
            pub pwr_device_mask: u32,
            pub pwr_offset_mw: i32,
            pub pwr_limit_mw: u32,
            pub channel_type: NV_GPU_POWER_MONITOR_POWER_CHANNEL_TYPE,
            pub pwr_rail: NV_GPU_POWER_CHANNEL_POWER_RAIL,
            pub volt_fixed_uv: u32,
            pub pwr_corr_slope: u32,
            pub curr_corr_slope: u32,
            pub curr_corr_offset_ma: i32,
            pub rsvd: Padding<[u8; 8]>,
            /// RTSS `data` union (16 bytes) — type-dispatched, raw.
            pub data: Padding<[u8; 16]>,
        }
    }

    nvstruct! {
        /// Per-channel relationship descriptor (RTSS
        /// `NV_GPU_POWER_MONITOR_POWER_CHANNEL_RELATIONSHIP_INFO_V3`).
        /// Research semantics; the trailing union is type-dispatched, kept raw.
        pub struct NV_GPU_POWER_MONITOR_POWER_CHANNEL_RELATIONSHIP_INFO_V3 {
            pub rel_type: u32,
            pub ch_idx: u8,
            pub rsvd0: Padding<[u8; 3]>,
            pub data: Padding<[u8; 32]>,
        }
    }

    nvstruct! {
        /// Power-monitor capability/topology params (RTSS
        /// `NV_GPU_POWER_MONITOR_GET_INFO_V2`). On success the driver fills
        /// `b_supported` (gate for GetStatus), `channel_mask` (which of 32
        /// channels exist), per-channel info + relationships, and
        /// `total_gpu_channel_idx` (the channel carrying total GPU power).
        pub struct NV_GPU_POWER_MONITOR_GET_INFO_V2 {
            pub version: NvVersion,
            pub b_supported: BoolU32,
            pub sampling_period_ms: u32,
            pub sample_count: u32,
            pub channel_mask: u32,
            pub ch_rel_mask: u32,
            pub total_gpu_power_channel_mask: u32,
            pub total_gpu_channel_idx: u8,
            pub rsvd: Padding<[u8; 8]>,
            pub channels: Array<[NV_GPU_POWER_MONITOR_POWER_CHANNEL_INFO_V2; NV_GPU_POWER_MONITOR_POWER_CHANNELS_MAX]>,
            pub ch_rels: Array<[NV_GPU_POWER_MONITOR_POWER_CHANNEL_RELATIONSHIP_INFO_V3; NV_GPU_POWER_MONITOR_POWER_CHANNELS_MAX]>,
        }
    }

    impl NV_GPU_POWER_MONITOR_GET_INFO_V2 {
        /// Iterate the populated channel info records (bits set in `channel_mask`).
        pub fn channels(&self) -> impl Iterator<Item = (usize, &NV_GPU_POWER_MONITOR_POWER_CHANNEL_INFO_V2)> {
            (0..NV_GPU_POWER_MONITOR_POWER_CHANNELS_MAX)
                .filter(move |&i| self.channel_mask & (1u32 << i) != 0)
                .filter_map(|i| self.channels.get(i).map(|c| (i, c)))
        }
    }

    nvversion! { @=NV_GPU_POWER_MONITOR_GET_INFO NV_GPU_POWER_MONITOR_GET_INFO_V2(1) }

    nvapi! {
        /// Undocumented (NDA-private, ID 0xC12EB19E). Power-monitor capability/
        /// topology descriptor (the INFO half). Probe `b_supported` before
        /// calling `NvAPI_GPU_PowerMonitorGetStatus`.
        pub unsafe fn NvAPI_GPU_PowerMonitorGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_POWER_MONITOR_GET_INFO) -> NvAPI_Status;
    }

    nvstruct! {
        /// Per-channel live reading (RTSS
        /// `NV_GPU_POWER_MONITOR_POWER_CHANNEL_STATUS_V2`, `#pragma pack(1)`).
        /// Average/min/max power in mW, current in mA, voltage in µV, energy in
        /// milli-Joules. Packed — read fields by copy, not by reference.
        #[repr(C, packed)]
        pub struct NV_GPU_POWER_MONITOR_POWER_CHANNEL_STATUS_V2 {
            pub pwr_avg_mw: u32,
            pub pwr_min_mw: u32,
            pub pwr_max_mw: u32,
            pub curr_ma: u32,
            pub volt_uv: u32,
            pub energy_mj: u64,
            pub rsvd: Padding<[u8; 16]>,
        }
    }

    impl NV_GPU_POWER_MONITOR_POWER_CHANNEL_STATUS_V2 {
        /// Average power (mW). Copies out of the packed struct.
        pub fn pwr_avg_mw(&self) -> u32 {
            self.pwr_avg_mw
        }
        /// Min power (mW).
        pub fn pwr_min_mw(&self) -> u32 {
            self.pwr_min_mw
        }
        /// Max power (mW).
        pub fn pwr_max_mw(&self) -> u32 {
            self.pwr_max_mw
        }
        /// Current (mA).
        pub fn curr_ma(&self) -> u32 {
            self.curr_ma
        }
        /// Voltage (µV).
        pub fn volt_uv(&self) -> u32 {
            self.volt_uv
        }
        /// Energy (mJ).
        pub fn energy_mj(&self) -> u64 {
            self.energy_mj
        }
    }

    nvstruct! {
        /// Power-monitor live readings (RTSS
        /// `NV_GPU_POWER_MONITOR_GET_STATUS_V2`). The caller sets `channel_mask`
        /// (copied from GetInfo); on success `channels[i]` holds the live
        /// reading for channel `i`, and `total_gpu_power_mw` the board total.
        pub struct NV_GPU_POWER_MONITOR_GET_STATUS_V2 {
            pub version: NvVersion,
            pub channel_mask: u32,
            pub total_gpu_power_mw: u32,
            pub rsvd: Padding<[u8; 16]>,
            pub channels: Array<[NV_GPU_POWER_MONITOR_POWER_CHANNEL_STATUS_V2; NV_GPU_POWER_MONITOR_POWER_CHANNELS_MAX]>,
        }
    }

    impl NV_GPU_POWER_MONITOR_GET_STATUS_V2 {
        /// Live reading for a channel index, if its bit is set in `channel_mask`.
        pub fn channel(&self, idx: usize) -> Option<&NV_GPU_POWER_MONITOR_POWER_CHANNEL_STATUS_V2> {
            (idx < NV_GPU_POWER_MONITOR_POWER_CHANNELS_MAX
                && self.channel_mask & (1u32 << idx) != 0)
                .then(|| ())
                .and_then(|_| self.channels.get(idx))
        }
    }

    nvversion! { @=NV_GPU_POWER_MONITOR_GET_STATUS NV_GPU_POWER_MONITOR_GET_STATUS_V2(1) }

    nvapi! {
        /// Undocumented (NDA-private, ID 0xF40238EF). Power-monitor live readings
        /// (the STATUS half). Pass GetInfo's `channel_mask`; read
        /// `total_gpu_power_mw` + per-channel `channels[i]`. Stubbed (-104) on
        /// some GPU/driver combos — gate on GetInfo's `b_supported`.
        pub unsafe fn NvAPI_GPU_PowerMonitorGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pStatus: *mut NV_GPU_POWER_MONITOR_GET_STATUS) -> NvAPI_Status;
    }
}
