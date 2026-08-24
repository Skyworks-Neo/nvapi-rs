use crate::prelude_::*;

nvapi! {
    pub type GPU_GetTachReadingFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, pValue: *mut u32) -> NvAPI_Status;

    /// This API retrieves the fan speed tachometer reading for the specified physical GPU.
    pub unsafe fn NvAPI_GPU_GetTachReading;
}

/// Undocumented API
pub mod private {
    use crate::prelude_::*;

    pub const NVAPI_MIN_COOLER_LEVEL: usize = 0;
    pub const NVAPI_MAX_COOLER_LEVEL: usize = 100;
    pub const NVAPI_MAX_COOLER_LEVELS: usize = 24;
    pub const NVAPI_MAX_COOLERS_PER_GPU: usize = 3;
    pub const NVAPI_MAX_COOLERS_PER_GPU_VER2: usize = 20;
    pub const NVAPI_MAX_COOLERS_PER_GPU_VER3: usize = NVAPI_MAX_COOLERS_PER_GPU_VER2;
    pub const NVAPI_MAX_COOLERS_PER_GPU_VER4: usize = NVAPI_MAX_COOLERS_PER_GPU_VER3;

    nvenum! {
        pub enum NV_COOLER_TYPE / CoolerType {
            NVAPI_COOLER_TYPE_NONE / None = 0,
            NVAPI_COOLER_TYPE_FAN / Fan = 1,
            NVAPI_COOLER_TYPE_WATER / Water = 2,
            NVAPI_COOLER_TYPE_LIQUID_NO2 / LiquidNO2 = 3,
        }
    }

    nvenum_display! {
        CoolerType => _
    }

    nvenum! {
        pub enum NV_COOLER_CONTROLLER / CoolerController {
            NVAPI_COOLER_CONTROLLER_NONE / None = 0,
            NVAPI_COOLER_CONTROLLER_ADI / ADI = 1,
            NVAPI_COOLER_CONTROLLER_INTERNAL / Internal = 2,
        }
    }

    nvenum_display! {
        CoolerController => _
    }

    nvenum! {
        pub enum NV_COOLER_POLICY / CoolerPolicy {
            NVAPI_COOLER_POLICY_NONE / None = 0,
            /// Manual adjustment of cooler level. Gets applied right away independent of temperature or performance level.
            NVAPI_COOLER_POLICY_MANUAL / Manual = 1,
            /// GPU performance controls the cooler level.
            NVAPI_COOLER_POLICY_PERF / Performance = 2,
            /// Discrete thermal levels control the cooler level.
            NVAPI_COOLER_POLICY_TEMPERATURE_DISCRETE / TemperatureDiscrete = 4,
            /// Cooler level adjusted at continuous thermal levels.
            NVAPI_COOLER_POLICY_TEMPERATURE_CONTINUOUS / TemperatureContinuous = 8,
            /// Hybrid of performance and temperature levels.
            NVAPI_COOLER_POLICY_HYBRID / Hybrid = 9, // are you sure this isn't just a bitmask?
            /// Fan turns off at idle, default of MSI Gaming X
            NVAPI_COOLER_POLICY_TEMPERATURE_CONTINUOUS_SW / TemperatureContinuousSoftware = 16,
            /// Apparently a default of some GPUs
            NVAPI_COOLER_POLICY_DEFAULT / Default = 32,
        }
    }

    nvenum_display! {
        CoolerPolicy => {
            TemperatureDiscrete = "Thermal (Discrete)",
            TemperatureContinuous = "Thermal",
            TemperatureContinuousSoftware = "Thermal (Silent)",
            _ = _,
        }
    }

    nvenum! {
        pub enum NV_COOLER_TARGET / CoolerTarget {
            NVAPI_COOLER_TARGET_NONE / None = 0,
            NVAPI_COOLER_TARGET_GPU / GPU = 1,
            NVAPI_COOLER_TARGET_MEMORY / Memory = 2,
            NVAPI_COOLER_TARGET_POWER_SUPPLY / PowerSupply = 4,
            /// This cooler cools all of the components related to its target gpu.
            NVAPI_COOLER_TARGET_ALL / All = 7,
        }
    }

    nvenum_display! {
        CoolerTarget => {
            GPU = "Core",
            PowerSupply = "VRM",
            _ = _,
        }
    }

    nvenum! {
        pub enum NV_COOLER_CONTROL / CoolerControl {
            NVAPI_COOLER_CONTROL_NONE / None = 0,
            /// ON/OFF
            NVAPI_COOLER_CONTROL_TOGGLE / Toggle = 1,
            /// Suppports variable control.
            NVAPI_COOLER_CONTROL_VARIABLE / Variable = 2,
        }
    }

    nvenum_display! {
        CoolerControl => _
    }

    nvenum! {
        pub enum NV_COOLER_ACTIVITY_LEVEL / CoolerActivityLevel {
            NVAPI_INACTIVE / Inactive = 0,
            NVAPI_ACTIVE / Active = 1,
        }
    }

    impl CoolerActivityLevel {
        pub fn get(&self) -> bool {
            match *self {
                CoolerActivityLevel::Active => true,
                CoolerActivityLevel::Inactive => false,
            }
        }
    }

    nvstruct! {
        pub struct NV_GPU_GETCOOLER_SETTING_V1 {
            /// type of cooler - FAN, WATER, LIQUID_NO2...
            pub type_: NV_COOLER_TYPE,
            /// internal, ADI...
            pub controller: NV_COOLER_CONTROLLER,
            /// the min default value % of the cooler
            pub defaultMinLevel: u32,
            /// the max default value % of the cooler
            pub defaultMaxLevel: u32,
            /// the current allowed min value % of the cooler
            pub currentMinLevel: u32,
            /// the current allowed max value % of the cooler
            pub currentMaxLevel: u32,
            /// the current value % of the cooler
            pub currentLevel: u32,
            /// cooler control policy - auto-perf, auto-thermal, manual, hybrid...
            pub defaultPolicy: NV_COOLER_POLICY,
            /// cooler control policy - auto-perf, auto-thermal, manual, hybrid...
            pub currentPolicy: NV_COOLER_POLICY,
            /// cooling target - GPU, memory, chipset, powersupply, canoas...
            pub target: NV_COOLER_TARGET,
            /// toggle or variable
            pub controlType: NV_COOLER_CONTROL,
            /// is the cooler active - fan spinning...
            pub active: NV_COOLER_ACTIVITY_LEVEL,
        }
    }

    nvstruct! {
        pub struct NV_GPU_GETCOOLER_SETTINGS_V1 {
            pub version: NvVersion,
            pub count: u32,
            pub cooler: Array<[NV_GPU_GETCOOLER_SETTING_V1; NVAPI_MAX_COOLERS_PER_GPU]>,
        }
    }

    impl NV_GPU_GETCOOLER_SETTINGS_V1 {
        pub fn coolers(&self) -> &[NV_GPU_GETCOOLER_SETTING_V1] {
            &self.cooler[..self.count as usize]
        }
    }

    nvstruct! {
        pub struct NV_COOLER_TACHOMETER {
            /// current tachometer reading in RPM
            pub speedRPM: u32,
            /// cooler supports tach function?
            pub bSupported: BoolU32,
            /// Maximum RPM corresponding to 100% defaultMaxLevel
            pub maxSpeedRPM: u32,
            /// Minimum RPM corresponding to 100% defaultMinLevel
            pub minSpeedRPM: u32,
        }
    }

    nvstruct! {
        pub struct NV_GPU_GETCOOLER_SETTING_V3 {
            pub v1: NV_GPU_GETCOOLER_SETTING_V1,
            /// cooler tachometer info
            pub tachometer: NV_COOLER_TACHOMETER,
        }
    }

    nvinherit! { struct NV_GPU_GETCOOLER_SETTING_V3(v1: NV_GPU_GETCOOLER_SETTING_V1) }

    nvstruct! {
        pub struct NV_GPU_GETCOOLER_SETTINGS_V3 {
            /// structure version
            pub version: NvVersion,
            /// number of associated coolers with the selected GPU
            pub count: u32,
            pub cooler: Array<[NV_GPU_GETCOOLER_SETTING_V3; NVAPI_MAX_COOLERS_PER_GPU_VER3]>,
        }
    }

    impl NV_GPU_GETCOOLER_SETTINGS_V3 {
        pub fn coolers(&self) -> &[NV_GPU_GETCOOLER_SETTING_V3] {
            &self.cooler[..self.count as usize]
        }
    }

    nvstruct! {
        pub struct NV_GPU_GETCOOLER_SETTING_V4 {
            pub v3: NV_GPU_GETCOOLER_SETTING_V3,
            pub unknown: u32,
        }
    }

    nvinherit! { struct NV_GPU_GETCOOLER_SETTING_V4(v3: NV_GPU_GETCOOLER_SETTING_V3) }

    nvstruct! {
        pub struct NV_GPU_GETCOOLER_SETTINGS_V4 {
            pub version: NvVersion,
            pub count: u32,
            pub cooler: Array<[NV_GPU_GETCOOLER_SETTING_V4; NVAPI_MAX_COOLERS_PER_GPU_VER4]>,
        }
    }

    impl NV_GPU_GETCOOLER_SETTINGS_V4 {
        pub fn coolers(&self) -> &[NV_GPU_GETCOOLER_SETTING_V4] {
            &self.cooler[..self.count as usize]
        }
    }

    nvversion! { NV_GPU_GETCOOLER_SETTINGS_V1(1) = 152 }
    nvversion! { NV_GPU_GETCOOLER_SETTINGS_V3(3) = 1288 }
    nvversion! { @=NV_GPU_GETCOOLER_SETTINGS NV_GPU_GETCOOLER_SETTINGS_V4(4) = 1368 }

    nvapi! {
        pub type GPU_GetCoolerSettingsFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, coolerIndex: u32, pCoolerInfo: *mut NV_GPU_GETCOOLER_SETTINGS) -> NvAPI_Status;

        /// Undocumented function.
        /// Retrieves the cooler information of all coolers or a specific cooler associated with the selected GPU.
        ///
        /// Coolers are indexed 0 to NVAPI_MAX_COOLERS_PER_GPU-1.
        /// To retrieve specific cooler info set the coolerIndex to the appropriate cooler index.
        /// To retrieve info for all cooler set coolerIndex to NVAPI_COOLER_TARGET_ALL.
        pub unsafe fn NvAPI_GPU_GetCoolerSettings;
    }

    nvstruct! {
        pub struct NV_GPU_SETCOOLER_LEVEL_COOLER {
            /// the new value % of the cooler
            pub currentLevel: u32,
            /// the new cooler control policy - auto-perf, auto-thermal, manual, hybrid...
            pub currentPolicy: NV_COOLER_POLICY,
        }
    }

    nvstruct! {
        pub struct NV_GPU_SETCOOLER_LEVEL_V1 {
            pub version: NvVersion,
            pub cooler: Array<[NV_GPU_SETCOOLER_LEVEL_COOLER; NVAPI_MAX_COOLERS_PER_GPU]>,
        }
    }

    nvversion! { @=NV_GPU_SETCOOLER_LEVEL NV_GPU_SETCOOLER_LEVEL_V1(1) }

    nvapi! {
        pub type GPU_SetCoolerLevelsFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, coolerIndex: u32, pCoolerLevels: *const NV_GPU_SETCOOLER_LEVEL) -> NvAPI_Status;

        /// Undocumented function.
        /// Set the cooler levels for all coolers or a specific cooler associated with the selected GPU.
        ///
        /// Coolers are indexed 0 to NVAPI_MAX_COOLERS_PER_GPU-1. Every cooler level with non-zero currentpolicy gets applied.
        ///
        /// The new level should be in the range of minlevel and maxlevel retrieved from GetCoolerSettings API or between
        /// and NVAPI_MIN_COOLER_LEVEL to MAX_COOLER_LEVEL.
        /// To set level for a specific cooler set the coolerIndex to the appropriate cooler index.
        /// To set level for all coolers set coolerIndex to NVAPI_COOLER_TARGET_ALL.
        ///
        /// NOTE: To lock the fan speed independent of the temperature or performance changes set the cooler currentPolicy to
        /// NVAPI_COOLER_POLICY_MANUAL else set it to the current policy retrieved from the GetCoolerSettings API.
        ///
        /// nvapioc (reverse/nvapioc-master) corroborates the policy encoding from
        /// the field's consumers: policy 32 (= NVAPI_COOLER_POLICY_DEFAULT) restores
        /// the driver fan curve, policy 1 (= MANUAL) applies `level` as a fixed
        /// duty in percent.
        pub unsafe fn NvAPI_GPU_SetCoolerLevels;
    }

    nvapi! {
        pub type GPU_RestoreCoolerSettingsFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, coolerIndex: *const u32, coolerCount: u32) -> NvAPI_Status;

        /// Undocumented function.
        /// Restore the modified cooler settings to NVIDIA defaults.
        ///
        /// pCoolerIndex: Array containing absolute cooler indexes to restore. Pass NULL restore all coolers.
        ///
        /// coolerCount: Number of coolers to restore.
        pub unsafe fn NvAPI_GPU_RestoreCoolerSettings;
    }

    nvstruct! {
        pub struct NV_GPU_COOLER_POLICY_LEVEL {
            /// level indicator for a policy
            pub levelId: u32,
            /// new cooler level for the selected policy level indicator.
            pub currentLevel: u32,
            /// default cooler level for the selected policy level indicator.
            pub defaultLevel: u32,
        }
    }

    nvstruct! {
        pub struct NV_GPU_COOLER_POLICY_TABLE_V1 {
            /// structure version
            pub version: NvVersion,
            /// selected policy to update the cooler levels for, example NVAPI_COOLER_POLICY_PERF
            pub policy: NV_COOLER_POLICY,
            pub policyCoolerLevel: Array<[NV_GPU_COOLER_POLICY_LEVEL; NVAPI_MAX_COOLER_LEVELS]>,
        }
    }

    nvversion! { @=NV_GPU_COOLER_POLICY_TABLE NV_GPU_COOLER_POLICY_TABLE_V1(1) }

    nvapi! {
        pub type GPU_GetCoolerPolicyTableFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, coolerIndex: u32, pCoolerTable: *mut NV_GPU_COOLER_POLICY_TABLE, count: *mut u32) -> NvAPI_Status;

        /// Undocumented function.
        /// Retrieves the table of cooler and policy levels for the selected policy. Supported only for NVAPI_COOLER_POLICY_PERF.
        pub unsafe fn NvAPI_GPU_GetCoolerPolicyTable;
    }

    nvapi! {
        pub type GPU_SetCoolerPolicyTableFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, coolerIndex: u32, pCoolerTable: *const NV_GPU_COOLER_POLICY_TABLE, count: u32) -> NvAPI_Status;

        /// Undocumented function.
        /// Restore the modified cooler settings to NVIDIA defaults. Supported only for NVAPI_COOLER_POLICY_PERF.
        ///
        /// pCoolerTable: Updated table of policy levels and associated cooler levels. Every non-zero policy level gets updated.
        ///
        /// count: Number of valid levels in the policy table.
        pub unsafe fn NvAPI_GPU_SetCoolerPolicyTable;
    }

    nvapi! {
        pub type GPU_RestoreCoolerPolicyTableFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, coolerIndex: *const u32, coolerCount: u32, policy: NV_COOLER_POLICY) -> NvAPI_Status;

        /// Undocumented function.
        /// Restores the perf table policy levels to the defaults.
        ///
        /// pCoolerIndex: Array containing absolute cooler indexes to restore. Pass NULL restore all coolers.
        ///
        /// coolerCount: Number of coolers to restore.
        pub unsafe fn NvAPI_GPU_RestoreCoolerPolicyTable;
    }

    nvbits! {
        pub enum NV_FAN_ARBITER_INFO_FLAGS / FanArbiterInfoFlags {
            /// Supports full fan stop
            NV_FAN_ARBITER_INFO_FLAGS_FAN_STOP / FAN_STOP = 1,
            /// Fan stop is enabled by default
            NV_FAN_ARBITER_INFO_FLAGS_FAN_STOP_DEFAULT / FAN_STOP_DEFAULT = 2,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_FAN_ARBITER_INFO_V1 {
            pub unknown: u32,
            pub flags: NV_FAN_ARBITER_INFO_FLAGS,
            pub arbiter_index: u32,
            pub padding: Padding<[u32; 40/4-3]>,
        }
    }

    impl NV_GPU_CLIENT_FAN_ARBITER_INFO_V1 {
        pub fn flags(&self) -> FanArbiterInfoFlags {
            FanArbiterInfoFlags::from_bits_truncate(self.flags)
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_FAN_ARBITERS_INFO_V1 {
            pub version: NvVersion,
            pub count: u32,
            pub padding: Padding<[u32; 28/4]>,
            pub arbiters: Array<[NV_GPU_CLIENT_FAN_ARBITER_INFO_V1; 32]>, // offset 36
        }
    }

    impl NV_GPU_CLIENT_FAN_ARBITERS_INFO_V1 {
        pub fn arbiters(&self) -> &[NV_GPU_CLIENT_FAN_ARBITER_INFO_V1] {
            &self.arbiters[..self.count as usize]
        }
    }

    nvversion! { @=NV_GPU_CLIENT_FAN_ARBITERS_INFO NV_GPU_CLIENT_FAN_ARBITERS_INFO_V1(1) = 1316 }

    nvapi! {
        pub type GPU_ClientFanArbitersGetInfoFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, arbiter: *mut NV_GPU_CLIENT_FAN_ARBITERS_INFO) -> NvAPI_Status;

        pub unsafe fn NvAPI_GPU_ClientFanArbitersGetInfo;
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_FAN_ARBITER_STATUS_V1 {
            pub unknown0: u32,
            pub unknown1: u32,
        }
    }

    impl NV_GPU_CLIENT_FAN_ARBITER_STATUS_V1 {
        pub fn fan_stop_active(&self) -> bool {
            self.unknown1 != 0
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_FAN_ARBITERS_STATUS_V1 {
            pub version: NvVersion,
            pub count: u32,
            pub padding: Padding<[u32; 28/4]>,
            pub arbiters: Array<[NV_GPU_CLIENT_FAN_ARBITER_STATUS_V1; 32]>, // offset 36
        }
    }

    impl NV_GPU_CLIENT_FAN_ARBITERS_STATUS_V1 {
        pub fn arbiters(&self) -> &[NV_GPU_CLIENT_FAN_ARBITER_STATUS_V1] {
            &self.arbiters[..self.count as usize]
        }
    }

    nvversion! { @=NV_GPU_CLIENT_FAN_ARBITERS_STATUS NV_GPU_CLIENT_FAN_ARBITERS_STATUS_V1(1) = 292 }

    nvapi! {
        pub type GPU_ClientFanArbitersGetStatusFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, arbiter: *mut NV_GPU_CLIENT_FAN_ARBITERS_STATUS) -> NvAPI_Status;

        pub unsafe fn NvAPI_GPU_ClientFanArbitersGetStatus;
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_FAN_ARBITER_CONTROL_V1 {
            pub arbiter_index: u32,
            pub flags: NV_FAN_ARBITER_CONTROL_FLAGS,
        }
    }

    nvbits! {
        pub enum NV_FAN_ARBITER_CONTROL_FLAGS / FanArbiterControlFlags {
            /// Fan stop enabled
            NV_FAN_ARBITER_CONTROL_FLAGS_FAN_STOP / FAN_STOP = 1,
        }
    }

    impl NV_GPU_CLIENT_FAN_ARBITER_CONTROL_V1 {
        pub fn flags(&self) -> FanArbiterControlFlags {
            FanArbiterControlFlags::from_bits_truncate(self.flags)
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_FAN_ARBITERS_CONTROL_V1 {
            pub version: NvVersion,
            pub count: u32,
            pub padding: Padding<[u32; 28/4]>,
            pub arbiters: Array<[NV_GPU_CLIENT_FAN_ARBITER_CONTROL_V1; 32]>, // offset 36
        }
    }

    impl NV_GPU_CLIENT_FAN_ARBITERS_CONTROL_V1 {
        pub fn arbiters(&self) -> &[NV_GPU_CLIENT_FAN_ARBITER_CONTROL_V1] {
            &self.arbiters[..self.count as usize]
        }
    }

    nvversion! { @=NV_GPU_CLIENT_FAN_ARBITERS_CONTROL NV_GPU_CLIENT_FAN_ARBITERS_CONTROL_V1(1) = 292 }

    nvapi! {
        pub type GPU_ClientFanArbitersGetControlFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, arbiter: *mut NV_GPU_CLIENT_FAN_ARBITERS_CONTROL) -> NvAPI_Status;

        pub unsafe fn NvAPI_GPU_ClientFanArbitersGetControl;
    }

    nvapi! {
        pub type GPU_ClientFanArbitersSetControlFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, arbiter: *const NV_GPU_CLIENT_FAN_ARBITERS_CONTROL) -> NvAPI_Status;

        pub unsafe fn NvAPI_GPU_ClientFanArbitersSetControl;
    }

    nvenum! {
        pub enum NV_GPU_CLIENT_FAN_COOLERS_COOLER_ID / FanCoolerId {
            NV_GPU_CLIENT_FAN_COOLERS_COOLER_ID_NONE / None = 0,
            NV_GPU_CLIENT_FAN_COOLERS_COOLER_ID_1 / Cooler1 = 1,
            NV_GPU_CLIENT_FAN_COOLERS_COOLER_ID_2 / Cooler2 = 2,
        }
    }

    nvenum_display! {
        FanCoolerId => {
            Cooler1 = "Fan1",
            Cooler2 = "Fan2",
            _ = _,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_FAN_COOLER_INFO_V1 {
            pub cooler_id: NV_GPU_CLIENT_FAN_COOLERS_COOLER_ID,
            pub tach_supported: BoolU32,
            pub tach_min_rpm: u32,
            pub tach_max_rpm: u32,
            pub padding: Padding<[u32; 8]>,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_FAN_COOLERS_INFO_V1 {
            pub version: NvVersion,
            pub flags: u32,
            pub count: u32,
            pub padding: Padding<[u32; 8]>,
            pub coolers: Array<[NV_GPU_CLIENT_FAN_COOLER_INFO_V1; 32]>, // offset 44
        }
    }

    impl NV_GPU_CLIENT_FAN_COOLERS_INFO_V1 {
        pub fn valid(&self) -> bool {
            self.flags & 1 != 0
        }

        pub fn coolers(&self) -> &[NV_GPU_CLIENT_FAN_COOLER_INFO_V1] {
            &self.coolers[..self.count as usize]
        }
    }

    nvversion! { @=NV_GPU_CLIENT_FAN_COOLERS_INFO NV_GPU_CLIENT_FAN_COOLERS_INFO_V1(1) = 0x62c }

    nvapi! {
        pub type GPU_ClientFanCoolersGetInfoFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, coolers: *mut NV_GPU_CLIENT_FAN_COOLERS_INFO) -> NvAPI_Status;

        pub unsafe fn NvAPI_GPU_ClientFanCoolersGetInfo;
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_FAN_COOLER_STATUS_V1 {
            pub cooler_id: NV_GPU_CLIENT_FAN_COOLERS_COOLER_ID,
            pub tach_rpm: u32,
            pub level_minimum: u32,
            pub level_maximum: u32,
            pub level: u32,
            pub padding: Padding<[u32; 8]>,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_FAN_COOLERS_STATUS_V1 {
            pub version: NvVersion,
            pub count: u32,
            pub padding: Padding<[u32; 8]>,
            pub coolers: Array<[NV_GPU_CLIENT_FAN_COOLER_STATUS_V1; 32]>,
        }
    }

    impl NV_GPU_CLIENT_FAN_COOLERS_STATUS_V1 {
        pub fn coolers(&self) -> &[NV_GPU_CLIENT_FAN_COOLER_STATUS_V1] {
            &self.coolers[..self.count as usize]
        }
    }

    nvversion! { @=NV_GPU_CLIENT_FAN_COOLERS_STATUS NV_GPU_CLIENT_FAN_COOLERS_STATUS_V1(1) = 0x6a8 }

    nvapi! {
        pub type GPU_ClientFanCoolersGetStatusFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, coolers: *mut NV_GPU_CLIENT_FAN_COOLERS_STATUS) -> NvAPI_Status;

        pub unsafe fn NvAPI_GPU_ClientFanCoolersGetStatus;
    }

    nvstruct! {
        #[derive(Default)]
        pub struct NV_GPU_CLIENT_FAN_COOLER_CONTROL_V1 {
            pub cooler_id: NV_GPU_CLIENT_FAN_COOLERS_COOLER_ID,
            pub level: u32,
            pub flags: u32,
            pub padding: Padding<[u32; 8]>,
        }
    }

    impl NV_GPU_CLIENT_FAN_COOLER_CONTROL_V1 {
        pub fn manual(&self) -> bool {
            self.flags & 1 != 0
        }

        pub fn set_manual(&mut self, manual: bool) {
            self.flags = self.flags & 0xfffffffe | if manual { 1 } else { 0 }
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_FAN_COOLERS_CONTROL_V1 {
            pub version: NvVersion,
            pub flags: u32,
            pub count: u32,
            pub padding: Padding<[u32; 8]>,
            pub coolers: Array<[NV_GPU_CLIENT_FAN_COOLER_CONTROL_V1; 32]>,
        }
    }

    impl NV_GPU_CLIENT_FAN_COOLERS_CONTROL_V1 {
        pub fn valid(&self) -> bool {
            self.flags & 1 != 0
        }

        pub fn set_valid(&mut self, valid: bool) {
            self.flags = self.flags & 0xfffffffe | if valid { 1 } else { 0 }
        }

        pub fn coolers(&self) -> &[NV_GPU_CLIENT_FAN_COOLER_CONTROL_V1] {
            &self.coolers[..self.count as usize]
        }
    }

    nvversion! { @=NV_GPU_CLIENT_FAN_COOLERS_CONTROL NV_GPU_CLIENT_FAN_COOLERS_CONTROL_V1(1) = 0x5ac }

    nvapi! {
        pub type GPU_ClientFanCoolersGetControlFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, coolers: *mut NV_GPU_CLIENT_FAN_COOLERS_CONTROL) -> NvAPI_Status;

        pub unsafe fn NvAPI_GPU_ClientFanCoolersGetControl;
    }

    nvapi! {
        pub type GPU_ClientFanCoolersSetControlFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, coolers: *const NV_GPU_CLIENT_FAN_COOLERS_CONTROL) -> NvAPI_Status;

        pub unsafe fn NvAPI_GPU_ClientFanCoolersSetControl;
    }

    // One user-editable temperature→RPM point of the CURVE table. Layout RE'd
    // byte-for-byte from GPUMon.exe setFanCurve/pollFanCurve and cross-checked
    // against the nvapi64_impl.dll ObjInfo handler for the ClientFanPolicies
    // {Get,Set}Control pair. The driver's Set handler enforces strict
    // monotonicity across all three dword lanes (temp, reserved, rpm) plus
    // per-point ordering — a non-increasing curve returns -5.
    nvstruct! {
        pub struct NV_GPU_CLIENT_FAN_POLICIES_POINT_V1 {
            /// input temperature, Q8.8 fixed-point (celsius × 256) — the
            /// GPUMon dialog stores `temp << 8` here and reads it back as
            /// `(x + 128) >> 8`.
            pub temp_q8: u32,
            /// reserved lane — GPUMon never writes it (kept from the GET
            /// snapshot); the driver still requires it monotonic.
            pub reserved: u32,
            /// target fan speed, Q16 scaled (RPM × 65536/100); GPUMon reads
            /// it back as `(x * 100 + 32768) / 65536`.
            pub rpm_q16: u32,
        }
    }

    // One fan-curve slot (temperature→RPM points) in the control table.
    // 52 bytes — the per-curve stride GPUMon and the impl handler share.
    nvstruct! {
        pub struct NV_GPU_CLIENT_FAN_POLICIES_CURVE_V1 {
            /// curve slot index (byte @ slot+0, abs +20)
            pub index: u8,
            pub padding: Padding<[u8; 3]>,
            /// three monotonic (temp, rpm) points, each 12 bytes
            pub points: Array<[NV_GPU_CLIENT_FAN_POLICIES_POINT_V1; 3]>,
            pub tail: Padding<[u8; 12]>,
        }
    }

    /// Undocumented client fan-policy curve table (structure magic `0x200DC`;
    /// legacy sibling `0x10038`). RE'd from GPUMon.exe (`setFanCurve`, pane
    /// "DialogFanCurve") and nvapi64_impl.dll — both GET and SET marshal the
    /// same table through RM escape `0x07000198`. To change one curve you GET
    /// a snapshot, edit the slot (`+20 + 52·k`), then SET it back (RMW).
    /// `count` ≤ 4 curves; "Next Curve" in GPUMon just cycles `(idx+1) % count`.
    #[repr(C)]
    #[derive(Copy, Clone, Debug)]
    pub struct NV_GPU_CLIENT_FAN_POLICIES_CONTROL_V1 {
        /// structure magic — `0x200DC`
        pub version: u32,
        /// curve count (byte; driver rejects > 4)
        pub count: u8,
        pub header: Padding<[u8; 15]>,
        /// up to 4 curve slots, each 52 bytes
        pub curves: Array<[NV_GPU_CLIENT_FAN_POLICIES_CURVE_V1; 4]>,
    }

    unsafe impl zerocopy::AsBytes for NV_GPU_CLIENT_FAN_POLICIES_CONTROL_V1 {
        fn only_derive_is_allowed_to_implement_this_trait()
        where
            Self: Sized,
        {
        }
    }
    unsafe impl zerocopy::FromBytes for NV_GPU_CLIENT_FAN_POLICIES_CONTROL_V1 {
        fn only_derive_is_allowed_to_implement_this_trait()
        where
            Self: Sized,
        {
        }
    }

    /// Versionless alias used by the NVAPI function signatures (this NDA
    /// structure is magic-numbered, not size-versioned like public NVAPI V1s).
    pub type NV_GPU_CLIENT_FAN_POLICIES_CONTROL = NV_GPU_CLIENT_FAN_POLICIES_CONTROL_V1;

    impl NV_GPU_CLIENT_FAN_POLICIES_CONTROL_V1 {
        /// The `0x200DC` structure magic for `ClientFanPolicies{Get,Set}Control`.
        pub const MAGIC: u32 = 0x200DC;

        pub fn new() -> Self {
            Self {
                version: Self::MAGIC,
                count: 0,
                header: Padding { data: [0u8; 15] },
                curves: Padding {
                    data: [NV_GPU_CLIENT_FAN_POLICIES_CURVE_V1 {
                        index: 0,
                        padding: Padding { data: [0u8; 3] },
                        points: Padding {
                            data: [NV_GPU_CLIENT_FAN_POLICIES_POINT_V1 {
                                temp_q8: 0,
                                reserved: 0,
                                rpm_q16: 0,
                            }; 3],
                        },
                        tail: Padding { data: [0u8; 12] },
                    }; 4],
                },
            }
        }
    }

    nvapi! {
        pub type GPU_ClientFanPoliciesGetControlFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *mut NV_GPU_CLIENT_FAN_POLICIES_CONTROL) -> NvAPI_Status;

        /// Undocumented. Fills the fan-curve table (version `0x200DC`).
        pub unsafe fn NvAPI_GPU_ClientFanPoliciesGetControl;
    }

    nvapi! {
        pub type GPU_ClientFanPoliciesSetControlFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *const NV_GPU_CLIENT_FAN_POLICIES_CONTROL) -> NvAPI_Status;

        /// Undocumented. Writes the fan-curve table (version `0x200DC`).
        pub unsafe fn NvAPI_GPU_ClientFanPoliciesSetControl;
    }

    // ------------------------------------------------------------------
    // FanPolicy whole-block reset family (NDA). RE'd from GPUMon.exe
    // `GPUHandle::resetFanCurve` (sub_140030830): GET the full 0x14AC-byte
    // fan-policy block, write `1 << curve_index` into the bitmask dword at
    // +0x04 and set the flag bit0 at +0x08, SET it back — the driver
    // restores that curve slot to factory. This is GPUMon's NVAPI fan
    // reset, NOT the public RestoreCoolerSettings (which the driver rejects
    // with NOT_SUPPORTED on GPUs whose user-mode cooler table isn't
    // exposed, e.g. desktop 3060/2070; NVML's SetDefaultFanSpeed_v2 goes
    // through a separate RM arbiter channel and works there).
    //
    // Struct layout (magic 0x214AC = size 0x14AC | version 2):
    //   +0x00 u32  magic 0x214AC
    //   +0x04 u32  reset bitmask: bit N set = reset curve slot N
    //   +0x08 u32  flag dword: bit0 set by GPUMon's reset (meaning: apply)
    //   +0x0C..    opaque driver-filled policy data (memset from +0x0C,
    //              0x14A4 bytes; GET fills the rest first)
    // Cross-checked against nvapi64_impl.dll's 0x214AC handler.
    // ------------------------------------------------------------------
    pub const NV_GPU_FAN_POLICY_CONTROL_SIZE: usize = 0x14AC;
    pub const NV_GPU_FAN_POLICY_CONTROL_MAGIC: u32 = 0x214AC;
    /// Byte offset of the per-curve reset bitmask within the FanPolicy block.
    pub const NV_GPU_FAN_POLICY_OFF_RESET_MASK: usize = 0x04;
    /// Byte offset of the flag dword (bit0 = apply/reset marker).
    pub const NV_GPU_FAN_POLICY_OFF_FLAGS: usize = 0x08;

    nvapi! {
        pub type GPU_FanPolicyGetControlFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, pPolicy: *mut u8) -> NvAPI_Status;

        /// Undocumented (NDA 0x0FE87B7F). Fills the full fan-policy block
        /// (magic 0x214AC, 0x14AC bytes). RE'd from GPUMon resetFanCurve.
        pub unsafe fn NvAPI_GPU_FanPolicyGetControl;
    }

    nvapi! {
        pub type GPU_FanPolicySetControlFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, pPolicy: *const u8) -> NvAPI_Status;

        /// Undocumented (NDA 0x2B2A2A45). Writes the full fan-policy block;
        /// a set bit in the +0x04 bitmask resets that curve slot to factory.
        /// RE'd from GPUMon resetFanCurve.
        pub unsafe fn NvAPI_GPU_FanPolicySetControl;
    }

    // ------------------------------------------------------------------
    // Private FanCoolers family (NDA). RE'd from GPUMon.exe
    // `GPUHandle::setFanSim` (sub_140030F40): the private cooler info +
    // control path used for RPM-direct fan simulation. DISTINCT from the
    // public ClientFanCoolers family (0xFB85B01E etc.) — different IDs,
    // different structures, richer data (per-cooler type + min/max RPM).
    //
    // Four IDs (all already in nvid.rs):
    //   FanCoolerGetInfo     0x65CE5BFC  struct 0x108A8 (2216B)
    //   FanCoolerGetStatus   0x3CC2D181  struct 0x210A8 (4264B)
    //   FanCoolerGetControl  0xCF86B990  struct 0x210AC (4268B)
    //   FanCoolerSetControl  0xEB44E8AA  struct 0x210AC (4268B)
    //
    // Info struct (0x108A8 = version 1, size 0x8A8 = 2216B):
    //   +0x00 u32  magic 0x108A8
    //   +0x04 u32  32-bit cooler presence MASK (bit i = cooler i exists;
    //              NOT a count — popcount it. GPUMon pollFanSpeed iterates
    //              bits, so a GPU with 2 fans can report bits 0,1,2 set).
    //
    // Control struct (0x210AC, per-cooler stride 33 dword = 0x84):
    //   +0x00 u32  magic 0x210AC
    //   +0x04 u32  cooler mask (copied from info+0x04)
    //   entry[k] fields at dword[33*k + N]:
    //     dword 11  u32 cooler type (0=active, 1=pwm, 2=pwm-tach)
    //     dword 20  u32 min (driver scale — on some GPUs this is the
    //               normalized 0..65536 duty scale, NOT physical RPM)
    //     dword 21  u32 max (driver scale)
    //     dword 22  u32 enable bitmask (bit0 = simulation active)
    //     dword 23  u32 level (RPM mode: ((v-min)<<16)/(max-min) —
    //               effectively a 0..65536 duty scale; pwm-tach: raw)
    //     dword 24  u32 min PWM, dword 25 u32 max PWM
    //     dword 26/27  PWM enable/level
    //     dword 30/31  tach enable/level
    //
    // Status struct (0x210A8, per-cooler stride 33 dword — same indexing):
    //   dword 10   u32 tach-type (0=active → dword 19 is current RPM)
    //   dword 19   u32 current speed (driver scale, same unit as min/max)
    //   dword 24   u32 current PWM level (×100/65536 → percent)
    //
    // IMPORTANT: the "RPM" fields are in the DRIVER's scale. On the 2070
    // desktop the scale is the normalized 0..65536 duty grid (set v =
    // v/65536 × 100% duty), on other GPUs it may be raw RPM. The SET
    // value and the physical result are two different numbers by design;
    // surface min/max/current so the caller can see the actual grid.
    // ------------------------------------------------------------------
    pub const NV_GPU_FAN_COOLER_INFO_MAGIC: u32 = 0x108A8;
    pub const NV_GPU_FAN_COOLER_INFO_SIZE: usize = 0x8A8;
    pub const NV_GPU_FAN_COOLER_STATUS_MAGIC: u32 = 0x210A8;
    pub const NV_GPU_FAN_COOLER_STATUS_SIZE: usize = 0x10A8;
    pub const NV_GPU_FAN_COOLER_CONTROL_MAGIC: u32 = 0x210AC;
    pub const NV_GPU_FAN_COOLER_CONTROL_SIZE: usize = 0x10AC;
    /// Per-cooler entry stride (33 dword = 0x84 bytes). Field addressing is
    /// `dword[33*cooler + field_idx]` straight from the struct base — the
    /// magic/count header occupies entry0's first two dword slots and the
    /// driver tolerates that overlap (GPUMon's exact arithmetic).
    pub const NV_GPU_FAN_COOLER_ENTRY_STRIDE: usize = 0x84;
    /// Byte offset of the first entry (= 0; fields index from struct base).
    pub const NV_GPU_FAN_COOLER_ENTRY0_BASE: usize = 0x00;
    // Per-cooler field offsets (dword index × 4, from struct base +
    // cooler * 0x84). RE'd from GPUMon v19[33*v4 + N]:
    pub const NV_GPU_FAN_COOLER_OFF_TYPE: usize = 11 * 4; // dword 11
    pub const NV_GPU_FAN_COOLER_OFF_MIN_RPM: usize = 20 * 4; // dword 20
    pub const NV_GPU_FAN_COOLER_OFF_MAX_RPM: usize = 21 * 4; // dword 21
    pub const NV_GPU_FAN_COOLER_OFF_ENABLE: usize = 22 * 4; // dword 22
    pub const NV_GPU_FAN_COOLER_OFF_LEVEL: usize = 23 * 4; // dword 23
    pub const NV_GPU_FAN_COOLER_OFF_MIN_PWM: usize = 24 * 4; // dword 24
    pub const NV_GPU_FAN_COOLER_OFF_MAX_PWM: usize = 25 * 4; // dword 25
    pub const NV_GPU_FAN_COOLER_OFF_PWM_ENABLE: usize = 26 * 4; // dword 26
    pub const NV_GPU_FAN_COOLER_OFF_PWM_LEVEL: usize = 27 * 4; // dword 27
    pub const NV_GPU_FAN_COOLER_OFF_TACH_ENABLE: usize = 30 * 4; // dword 30
    pub const NV_GPU_FAN_COOLER_OFF_TACH_LEVEL: usize = 31 * 4; // dword 31
    // Status-struct field offsets (dword index × 4, same 33-dword stride):
    pub const NV_GPU_FAN_COOLER_OFF_ST_TYPE: usize = 10 * 4; // dword 10 (tach kind)
    pub const NV_GPU_FAN_COOLER_OFF_ST_CURRENT: usize = 19 * 4; // dword 19 (current speed)
    pub const NV_GPU_FAN_COOLER_OFF_ST_PWM: usize = 24 * 4; // dword 24 (current PWM Q16)

    nvapi! {
        pub type GPU_FanCoolerGetInfoFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut u8) -> NvAPI_Status;

        /// Undocumented (NDA 0x65CE5BFC). Fills the private cooler info
        /// struct (magic 0x108A8): per-cooler type + min/max RPM range.
        /// RE'd from GPUMon setFanSim.
        pub unsafe fn NvAPI_GPU_FanCoolerGetInfo;
    }

    nvapi! {
        pub type GPU_FanCoolerGetControlFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *mut u8) -> NvAPI_Status;

        /// Undocumented (NDA 0xCF86B990). Fills the private cooler control
        /// struct (magic 0x210AC): per-cooler enable/level snapshot.
        /// RE'd from GPUMon setFanSim (RMW baseline).
        pub unsafe fn NvAPI_GPU_FanCoolerGetControl;
    }

    nvapi! {
        pub type GPU_FanCoolerGetStatusFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, pStatus: *mut u8) -> NvAPI_Status;

        /// Undocumented (NDA 0x3CC2D181). Fills the private cooler status
        /// struct (magic 0x210A8): per-cooler current speed (dword 19,
        /// driver scale) + current PWM (dword 24, Q16). RE'd from GPUMon
        /// pollFanSpeed.
        pub unsafe fn NvAPI_GPU_FanCoolerGetStatus;
    }

    nvapi! {
        pub type GPU_FanCoolerSetControlFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *const u8) -> NvAPI_Status;

        /// Undocumented (NDA 0xEB44E8AA). Writes the private cooler control
        /// struct. RE'd from GPUMon setFanSim (RMW write-back).
        pub unsafe fn NvAPI_GPU_FanCoolerSetControl;
    }
}
