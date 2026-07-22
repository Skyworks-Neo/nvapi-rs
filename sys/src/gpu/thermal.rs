use crate::prelude_::*;

nvenum! {
    /// Used in NV_GPU_THERMAL_SETTINGS
    pub enum NV_THERMAL_TARGET / ThermalTarget {
        NVAPI_THERMAL_TARGET_NONE / None = 0,
        /// GPU core temperature requires NvPhysicalGpuHandle
        NVAPI_THERMAL_TARGET_GPU / Gpu = 1,
        /// GPU memory temperature requires NvPhysicalGpuHandle
        NVAPI_THERMAL_TARGET_MEMORY / Memory = 2,
        /// GPU power supply temperature requires NvPhysicalGpuHandle
        NVAPI_THERMAL_TARGET_POWER_SUPPLY / PowerSupply = 4,
        /// GPU board ambient temperature requires NvPhysicalGpuHandle
        NVAPI_THERMAL_TARGET_BOARD / Board = 8,
        /// Visual Computing Device Board temperature requires NvVisualComputingDeviceHandle
        NVAPI_THERMAL_TARGET_VCD_BOARD / VcdBoard = 9,
        /// Visual Computing Device Inlet temperature requires NvVisualComputingDeviceHandle
        NVAPI_THERMAL_TARGET_VCD_INLET / VcdInlet = 10,
        /// Visual Computing Device Outlet temperature requires NvVisualComputingDeviceHandle
        NVAPI_THERMAL_TARGET_VCD_OUTLET / VcdOutlet = 11,
        NVAPI_THERMAL_TARGET_ALL / All = 15,
        NVAPI_THERMAL_TARGET_UNKNOWN / Unknown = -1,
    }
}

nvenum_display! {
    ThermalTarget => {
        Gpu = "Core",
        Memory = "Memory",
        PowerSupply = "VRM",
        VcdBoard = "VCD Board",
        VcdInlet = "VCD Inlet",
        VcdOutlet = "VCD Outlet",
        _ = _,
    }
}

nvenum! {
    /// NV_GPU_THERMAL_SETTINGS
    pub enum NV_THERMAL_CONTROLLER / ThermalController {
        NVAPI_THERMAL_CONTROLLER_NONE / None = 0,
        NVAPI_THERMAL_CONTROLLER_GPU_INTERNAL / GpuInternal = 1,
        NVAPI_THERMAL_CONTROLLER_ADM1032 / ADM1032 = 2,
        NVAPI_THERMAL_CONTROLLER_MAX6649 / MAX6649 = 3,
        NVAPI_THERMAL_CONTROLLER_MAX1617 / MAX1617 = 4,
        NVAPI_THERMAL_CONTROLLER_LM99 / LM99 = 5,
        NVAPI_THERMAL_CONTROLLER_LM89 / LM89 = 6,
        NVAPI_THERMAL_CONTROLLER_LM64 / LM64 = 7,
        NVAPI_THERMAL_CONTROLLER_ADT7473 / ADT7473 = 8,
        NVAPI_THERMAL_CONTROLLER_SBMAX6649 / SBMAX6649 = 9,
        NVAPI_THERMAL_CONTROLLER_VBIOSEVT / VBIOSEVT = 10,
        NVAPI_THERMAL_CONTROLLER_OS / OS = 11,
        NVAPI_THERMAL_CONTROLLER_UNKNOWN / Unknown = -1,
    }
}

nvenum_display! {
    ThermalController => {
        GpuInternal = "Internal",
        _ = _,
    }
}

pub const NVAPI_MAX_THERMAL_SENSORS_PER_GPU: usize = 3;

nvstruct! {
    /// Used in NvAPI_GPU_GetThermalSettings()
    pub struct NV_GPU_THERMAL_SETTINGS_V1 {
        /// structure version
        pub version: NvVersion,
        /// number of associated thermal sensors
        pub count: u32,
        pub sensor: Array<[NV_GPU_THERMAL_SETTINGS_SENSOR; NVAPI_MAX_THERMAL_SENSORS_PER_GPU]>,
    }
}

nvstruct! {
    /// Anonymous struct in NV_GPU_THERMAL_SETTINGS
    pub struct NV_GPU_THERMAL_SETTINGS_SENSOR {
        /// internal, ADM1032, MAX6649...
        pub controller: NV_THERMAL_CONTROLLER,
        /// The min default temperature value of the thermal sensor in degree Celsius
        pub defaultMinTemp: i32,
        /// The max default temperature value of the thermal sensor in degree Celsius
        pub defaultMaxTemp: i32,
        /// The current temperature value of the thermal sensor in degree Celsius
        pub currentTemp: i32,
        /// Thermal sensor targeted @ GPU, memory, chipset, powersupply, Visual Computing Device, etc.
        pub target: NV_THERMAL_TARGET,
    }
}

nvversion! { NV_GPU_THERMAL_SETTINGS_V1(1) }
nvversion! { @=NV_GPU_THERMAL_SETTINGS NV_GPU_THERMAL_SETTINGS_V1(2) } // the only v2 difference is the _SENSOR struct uses i32 instead of u32 fields

nvapi! {
    pub type GPU_GetThermalSettingsFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, sensorIndex: u32, pThermalSettings: *mut NV_GPU_THERMAL_SETTINGS) -> NvAPI_Status;

    /// This function retrieves the thermal information of all thermal sensors or specific thermal sensor associated with the selected GPU.
    ///
    /// Thermal sensors are indexed 0 to NVAPI_MAX_THERMAL_SENSORS_PER_GPU-1.
    /// - To retrieve specific thermal sensor info, set the sensorIndex to the required thermal sensor index.
    /// - To retrieve info for all sensors, set sensorIndex to NVAPI_THERMAL_TARGET_ALL.
    pub unsafe fn NvAPI_GPU_GetThermalSettings;
}

nvapi! {
    pub type GPU_GetCurrentThermalLevelFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, pThermalLevel: *mut u32) -> NvAPI_Status;

    /// Returns current thermal level (0=cool, 3=hot). Kepler-era API.
    pub unsafe fn NvAPI_GPU_GetCurrentThermalLevel;
}

nvapi! {
    pub type GPU_GetCurrentFanSpeedLevelFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, pFanSpeedLevel: *mut u32) -> NvAPI_Status;

    /// Returns current fan speed level (0=slow, 7=fast). Kepler-era API.
    pub unsafe fn NvAPI_GPU_GetCurrentFanSpeedLevel;
}

/// Undocumented API
pub mod private {
    use crate::prelude_::*;

    pub const NVAPI_MAX_THERMAL_INFO_ENTRIES: usize = 4;

    nvenum! {
        pub enum NV_GPU_CLIENT_THERMAL_POLICIES_POLICY_ID / ThermalPolicyId {
            NV_GPU_CLIENT_THERMAL_POLICIES_POLICY_ID_DEFAULT / Default = 1,
        }
    }

    nvenum_display! {
        ThermalPolicyId => {
            Default = "GPU Thermal Policy",
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_THERMAL_POLICIES_INFO_ENTRY_V2 {
            pub policy_id: NV_GPU_CLIENT_THERMAL_POLICIES_POLICY_ID,
            pub unknown: u32,
            pub minTemp: i32,
            pub defaultTemp: i32,
            pub maxTemp: i32,
            pub defaultFlags: u32,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_THERMAL_POLICIES_INFO_V2 {
            pub version: NvVersion,
            pub count: u8,
            pub flags: u8,
            pub padding: Padding<[u8; 2]>,
            pub entries: Array<[NV_GPU_CLIENT_THERMAL_POLICIES_INFO_ENTRY_V2; NVAPI_MAX_THERMAL_INFO_ENTRIES]>,
        }
    }

    impl NV_GPU_CLIENT_THERMAL_POLICIES_INFO_V2 {
        pub fn entries(&self) -> &[NV_GPU_CLIENT_THERMAL_POLICIES_INFO_ENTRY_V2] {
            &self.entries[..self.count as usize]
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_THERMAL_POLICY_INFO_V3 {
            pub policy_id: NV_GPU_CLIENT_THERMAL_POLICIES_POLICY_ID,
            pub flags: u32,
            pub unknown: u32,
            pub minTemp: i32,
            pub defaultTemp: i32,
            pub maxTemp: i32,
            pub defaultFlags: u32,
            pub padding0: Padding<[u32; 16]>,
            pub pff_curve: NV_GPU_CLIENT_PFF_CURVE_V1,
            pub padding1: Padding<[u32; 49]>,
        }
    }

    impl NV_GPU_CLIENT_THERMAL_POLICY_INFO_V3 {
        pub fn has_pff(&self) -> bool {
            self.flags == 1
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_THERMAL_POLICIES_INFO_V3 {
            pub version: NvVersion,
            pub flags: u8,
            pub count: u8,
            pub padding: Padding<[u8; 2]>,
            pub entries: Array<[NV_GPU_CLIENT_THERMAL_POLICY_INFO_V3; NVAPI_MAX_THERMAL_INFO_ENTRIES]>,
        }
    }

    impl NV_GPU_CLIENT_THERMAL_POLICIES_INFO_V3 {
        pub fn entries(&self) -> &[NV_GPU_CLIENT_THERMAL_POLICY_INFO_V3] {
            &self.entries[..self.count as usize]
        }

        pub fn valid(&self) -> bool {
            self.flags & 1 != 0
        }
    }

    nvversion! { NV_GPU_CLIENT_THERMAL_POLICIES_INFO_V2(2) }
    nvversion! { @=NV_GPU_CLIENT_THERMAL_POLICIES_INFO NV_GPU_CLIENT_THERMAL_POLICIES_INFO_V3(3) = 1400 }

    nvapi! {
        pub unsafe fn NvAPI_GPU_ClientThermalPoliciesGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pThermalInfo: *mut NV_GPU_CLIENT_THERMAL_POLICIES_INFO) -> NvAPI_Status;
    }

    pub const NVAPI_MAX_THERMAL_LIMIT_ENTRIES: usize = 4;

    nvstruct! {
        pub struct NV_GPU_CLIENT_THERMAL_POLICIES_STATUS_ENTRY_V2 {
            pub policy_id: NV_GPU_CLIENT_THERMAL_POLICIES_POLICY_ID,
            /// shifted 8 bits
            pub temp_limit_C: u32,
            pub pstate: crate::gpu::pstate::NV_GPU_PERF_PSTATE_ID,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_THERMAL_POLICIES_STATUS_V2 {
            pub version: NvVersion,
            pub count: u32,
            pub entries: Array<[NV_GPU_CLIENT_THERMAL_POLICIES_STATUS_ENTRY_V2; NVAPI_MAX_THERMAL_LIMIT_ENTRIES]>,
        }
    }

    impl NV_GPU_CLIENT_THERMAL_POLICIES_STATUS_V2 {
        pub fn entries(&self) -> &[NV_GPU_CLIENT_THERMAL_POLICIES_STATUS_ENTRY_V2] {
            &self.entries[..self.count as usize]
        }
    }

    nvstruct! {
        #[derive(Default)]
        pub struct NV_GPU_CLIENT_THERMAL_POLICY_STATUS_V3 {
            pub policy_id: NV_GPU_CLIENT_THERMAL_POLICIES_POLICY_ID,
            pub flags: u32,
            /// shifted 8 bits
            ///
            /// aka iT0X
            pub temp_limit_C: u32,
            /// aka bRemoveTdpLimit
            pub remove_tdp_limit: BoolU32,
            pub padding0: Padding<[u32; 17]>,
            pub pff_curve: NV_GPU_CLIENT_PFF_CURVE_V1, // 92-8
            /// aka uiT{1,2,3}OCY
            pub pff_freqs: [u32; 3], // 152-8 ~ 160-8
            pub padding1: Padding<[u32; 45]>,
        }
    }

    impl NV_GPU_CLIENT_THERMAL_POLICY_STATUS_V3 {
        pub fn has_pff(&self) -> bool {
            self.flags == 1
        }

        pub fn set_pff(&mut self, enabled: bool) {
            self.flags = if enabled { 1 } else { 0 }
        }

        pub fn pff_freqs(&self) -> &[u32] {
            let count = self.pff_curve.points().len();
            &self.pff_freqs[..count]
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLIENT_THERMAL_POLICIES_STATUS_V3 {
            pub version: NvVersion,
            pub count: u32,
            pub entries: Array<[NV_GPU_CLIENT_THERMAL_POLICY_STATUS_V3; NVAPI_MAX_THERMAL_LIMIT_ENTRIES]>,
        }
    }

    impl NV_GPU_CLIENT_THERMAL_POLICIES_STATUS_V3 {
        pub fn entries(&self) -> &[NV_GPU_CLIENT_THERMAL_POLICY_STATUS_V3] {
            &self.entries[..self.count as usize]
        }
    }

    nvversion! { NV_GPU_CLIENT_THERMAL_POLICIES_STATUS_V2(2) }
    nvversion! { @=NV_GPU_CLIENT_THERMAL_POLICIES_STATUS NV_GPU_CLIENT_THERMAL_POLICIES_STATUS_V3(3) = 1352 }

    nvapi! {
        pub unsafe fn NvAPI_GPU_ClientThermalPoliciesGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pThermalLimit: *mut NV_GPU_CLIENT_THERMAL_POLICIES_STATUS) -> NvAPI_Status;
    }

    nvapi! {
        pub unsafe fn NvAPI_GPU_ClientThermalPoliciesSetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pThermalLimit: *const NV_GPU_CLIENT_THERMAL_POLICIES_STATUS) -> NvAPI_Status;
    }

    nvstruct! {
        #[derive(Default)]
        pub struct NV_GPU_CLIENT_PFF_CURVE_POINT_V1 {
            pub enabled: BoolU32,
            /// uiT{1,2,3}Y
            pub uiT_Y: u32,
            /// iT{1,2,3}X
            pub temp: u32,
            pub padding: Padding<[u32; 2]>,
        }
    }

    nvstruct! {
        #[derive(Default)]
        pub struct NV_GPU_CLIENT_PFF_CURVE_V1 {
            pub points: Array<[NV_GPU_CLIENT_PFF_CURVE_POINT_V1; 3]>,
        }
    }

    impl NV_GPU_CLIENT_PFF_CURVE_V1 {
        pub fn points(&self) -> &[NV_GPU_CLIENT_PFF_CURVE_POINT_V1] {
            let count = self.points.iter().take_while(|p| p.enabled.get()).count();
            &self.points[..count]
        }
    }

    nvstruct! {
        pub struct NV_GPU_THERMAL_SENSORS_V1 {
            pub version: NvVersion,
            pub mask: i32,
            pub values: [i32; 40],
        }
    }

    impl NV_GPU_THERMAL_SENSORS_V1 {
        /// Decode a raw sensor slot value into degrees Celsius.
        ///
        /// The driver encodes temperatures as `celsius * 256`; we preserve the
        /// sub-degree precision (two decimals) rather than truncating to an
        /// integer, so per-module readings that differ by a fraction of a
        /// degree are still distinguishable.
        pub fn decode(value: i32) -> Option<f32> {
            let v = value as f32 / 256.0;
            (v > 0.0 && v < 255.0).then_some(v)
        }

        /// Temperature at a fixed `values` index, if present and valid.
        pub fn get_temp(&self, index: usize) -> Option<f32> {
            self.values.get(index).copied().and_then(Self::decode)
        }

        /// All valid thermal readings in this result, as `(index, celsius)`.
        pub fn sensors(&self) -> Vec<(usize, f32)> {
            self.values
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(i, v)| Self::decode(v).map(|t| (i, t)))
                .collect()
        }

        /// Historical best-effort hotspot reading (index 9).
        ///
        /// NOTE: the actual mapping of `values` indices to physical sensors
        /// (core / hotspot / memory / ...) is GPU- and driver-dependent and
        /// is NOT fixed at 9. Prefer iterating via `sensors()` and matching
        /// against known readings.
        pub fn hotspot(&self) -> Option<f32> {
            self.get_temp(9)
        }

        /// Historical best-effort VRAM reading (index 15). See `hotspot()`.
        pub fn vram(&self) -> Option<f32> {
            self.get_temp(15)
        }
    }

    nvversion! { @=NV_GPU_THERMAL_SENSORS NV_GPU_THERMAL_SENSORS_V1(2) }

    nvapi! {
        pub unsafe fn NvAPI_GPU_GetThermalSensors(hPhysicalGPU: NvPhysicalGpuHandle, pSensors: *mut NV_GPU_THERMAL_SENSORS) -> NvAPI_Status;
    }

    // GPS (GPU Power Steering) thermal limit (Kepler-era, undocumented)

    nvstruct! {
        pub struct NV_GPU_GPS_THERMAL_LIMIT_V1 {
            pub version: NvVersion,
            pub flags: u32,
            pub thermal_limit: u32,
        }
    }

    nvversion! { @=NV_GPU_GPS_THERMAL_LIMIT NV_GPU_GPS_THERMAL_LIMIT_V1(1) }

    nvapi! {
        pub unsafe fn NvAPI_GPS_GetThermalLimit(hPhysicalGPU: NvPhysicalGpuHandle, pThermalLimit: *mut NV_GPU_GPS_THERMAL_LIMIT) -> NvAPI_Status;
    }

    nvapi! {
        pub unsafe fn NvAPI_GPS_SetThermalLimit(hPhysicalGPU: NvPhysicalGpuHandle, pThermalLimit: *const NV_GPU_GPS_THERMAL_LIMIT) -> NvAPI_Status;
    }

    // Thermal table (Kepler-era, undocumented)

    nvstruct! {
        pub struct NV_GPU_THERMAL_TABLE_ENTRY_V1 {
            pub temperature: i32,
            pub fan_speed: u32,
        }
    }

    nvstruct! {
        pub struct NV_GPU_THERMAL_TABLE_V1 {
            pub version: NvVersion,
            pub count: u32,
            pub entries: Array<[NV_GPU_THERMAL_TABLE_ENTRY_V1; 20]>,
        }
    }

    nvversion! { @=NV_GPU_THERMAL_TABLE NV_GPU_THERMAL_TABLE_V1(1) }

    nvapi! {
        pub unsafe fn NvAPI_GPU_GetThermalTable(hPhysicalGPU: NvPhysicalGpuHandle, pThermalTable: *mut NV_GPU_THERMAL_TABLE) -> NvAPI_Status;
    }
}
