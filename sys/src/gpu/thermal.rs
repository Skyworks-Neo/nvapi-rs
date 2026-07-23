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

    // ------------------------------------------------------------------
    // Thermal Channel capability descriptor (the INFO half of the
    // ThermChannel pair). NDA-developer-SDK private API; identity +
    // struct layout confirmed by RTSS (RivaTuner) source
    // (temp/NVAPIInterface.h) and nvapi64_impl.dll RE (nvid.rs comment).
    // The STATUS half is `NvAPI_GPU_GetThermalSensors` (0x65fe3aad) above.
    //
    // The point of this call: it returns a `priChIdx[5]` LUT giving the
    // authoritative primary channel index per thermal type
    // (GPU_AVG=0, GPU_MAX=1 hotspot, BOARD=2, MEMORY=3, PWR_SUPPLY=4).
    // Feeding that index to the STATUS read yields the true hotspot /
    // memory temperature — replacing the hi-layer positional heuristic.
    // ------------------------------------------------------------------

    nvenum! {
        /// Thermal channel type (RTSS `NV_GPU_THERMAL_THERM_CHANNEL_TYPE`).
        /// Indexes the `priChIdx[5]` LUT returned by GetInfo.
        pub enum NV_GPU_THERMAL_THERM_CHANNEL_TYPE / ThermChannelType {
            NV_GPU_THERMAL_THERM_CHANNEL_TYPE_GPU_AVG / GpuAvg = 0,
            /// Hot spot (max) temperature.
            NV_GPU_THERMAL_THERM_CHANNEL_TYPE_GPU_MAX / GpuMax = 1,
            NV_GPU_THERMAL_THERM_CHANNEL_TYPE_BOARD / Board = 2,
            /// VRAM / memory temperature.
            NV_GPU_THERMAL_THERM_CHANNEL_TYPE_MEMORY / Memory = 3,
            NV_GPU_THERMAL_THERM_CHANNEL_TYPE_PWR_SUPPLY / PwrSupply = 4,
            NV_GPU_THERMAL_THERM_CHANNEL_TYPE_INVALID / Invalid = 255,
        }
    }

    nvstruct! {
        /// One thermal-channel info record (RTSS `NV_GPU_THERMAL_THERM_CHANNEL_INFO_V1`).
        /// 84 bytes; the `is_temp_sim_supported`+`flags` pair is followed by a
        /// 2-byte align pad before the 4-byte `offset_hw` (same layout idiom as
        /// `NV_GPU_CLIENT_THERMAL_POLICIES_INFO_V2`).
        pub struct NV_GPU_THERMAL_THERM_CHANNEL_INFO_V1 {
            pub ch_class: u32,
            pub ch_type: u32,
            pub rel_loc: u32,
            pub tgt_gpu: u32,
            pub scaling: i32,
            pub offset_sw: i32,
            pub min_temp: i32,
            pub max_temp: i32,
            pub is_temp_sim_supported: u8,
            pub flags: u8,
            pub padding0: Padding<[u8; 2]>,
            pub offset_hw: i32,
            pub rsvd0: Padding<[u8; 28]>,
            /// RTSS union { device[2] | rsvd[16] } — raw 16 bytes.
            pub data: Padding<[u8; 16]>,
        }
    }

    /// Number of thermal channels the params struct reserves room for.
    pub const NV_GPU_THERMAL_THERM_CHANNEL_MAX: usize = 32;
    /// Number of primary-channel LUT entries (`priChIdx`).
    pub const NV_GPU_THERMAL_THERM_CHANNEL_TYPE_MAX: usize = 5;

    nvstruct! {
        /// Thermal-channel capability params (RTSS
        /// `NV_GPU_THERMAL_THERM_CHANNEL_INFO_PARAMS_V2`). 2736 bytes.
        /// On success the driver fills `channel_mask` (which of 32 channel
        /// slots are populated), per-channel records, and `pri_ch_idx`
        /// (the primary channel index for each of the 5 thermal types).
        pub struct NV_GPU_THERMAL_THERM_CHANNEL_INFO_PARAMS_V2 {
            pub version: NvVersion,
            pub channel_mask: u32,
            pub rsvd: Padding<[u8; 32]>,
            pub channel: Array<[NV_GPU_THERMAL_THERM_CHANNEL_INFO_V1; NV_GPU_THERMAL_THERM_CHANNEL_MAX]>,
            /// Primary channel index per type, indexed by
            /// `NV_GPU_THERMAL_THERM_CHANNEL_TYPE`
            /// (0=GPU_AVG, 1=GPU_MAX/hotspot, 2=BOARD, 3=MEMORY, 4=PWR_SUPPLY).
            pub pri_ch_idx: [u8; NV_GPU_THERMAL_THERM_CHANNEL_TYPE_MAX],
            pub padding1: Padding<[u8; 3]>,
        }
    }

    impl NV_GPU_THERMAL_THERM_CHANNEL_INFO_PARAMS_V2 {
        /// Primary channel index for a thermal type, if that channel is
        /// actually populated (index in range AND its bit set in `channel_mask`).
        pub fn primary_index(&self, ty: usize) -> Option<usize> {
            self.pri_ch_idx
                .get(ty)
                .copied()
                .map(|i| i as usize)
                .filter(|&i| i < NV_GPU_THERMAL_THERM_CHANNEL_MAX && self.channel_mask & (1u32 << i) != 0)
        }

        /// Hot spot (GPU_MAX) primary channel index.
        pub fn hotspot_index(&self) -> Option<usize> {
            self.primary_index(NV_GPU_THERMAL_THERM_CHANNEL_TYPE_GPU_MAX as usize)
        }

        /// VRAM (MEMORY) primary channel index.
        pub fn memory_index(&self) -> Option<usize> {
            self.primary_index(NV_GPU_THERMAL_THERM_CHANNEL_TYPE_MEMORY as usize)
        }

        /// The per-channel info record for a thermal type's primary channel,
        /// if present. Use to read `ch_type`/`offset_sw`/`offset_hw`/`scaling`/
        /// `min_temp`/`max_temp` for that type's sensor.
        pub fn primary_info(&self, ty: usize) -> Option<&NV_GPU_THERMAL_THERM_CHANNEL_INFO_V1> {
            self.primary_index(ty)
                .and_then(|i| self.channel.get(i))
        }
    }

    nvversion! { @=NV_GPU_THERMAL_THERM_CHANNEL_INFO NV_GPU_THERMAL_THERM_CHANNEL_INFO_PARAMS_V2(2) = 2736 }

    nvapi! {
        /// Undocumented (NDA-private, ID 0x0bc8163d). Thermal-channel capability
        /// descriptor. Pair with `NvAPI_GPU_ThermChannelGetStatus` for live temps.
        pub unsafe fn NvAPI_GPU_ThermChannelGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_THERMAL_THERM_CHANNEL_INFO) -> NvAPI_Status;
    }

    // ------------------------------------------------------------------
    // Thermal Channel STATUS (the live-reading half of the ThermChannel pair;
    // same QueryInterface ID 0x65fe3aad). RTSS (RivaTuner) source names this
    // `NvAPI_GPU_ThermChannelGetStatus` and passes
    // `NV_GPU_THERMAL_THERM_CHANNEL_STATUS_PARAMS_V2` (168 bytes, version magic
    // (2<<16)|168 = 131240). The caller sets `channel_mask` (copied from
    // GetInfo); on success `channel[i]` is the temperature for channel `i`
    // (celsius*256), for each bit set in `channel_mask`. Index `i` with
    // GetInfo's `priChIdx[type]`: `channel[priChIdx[GPU_MAX]]` = hot-spot temp,
    // `channel[priChIdx[MEMORY]]` = VRAM temp.
    //
    // History: this ID was previously wrapped with a `values[40]` positional
    // layout (`NV_GPU_THERMAL_SENSORS_V1`, same 168 bytes / magic). That layout
    // is now removed — the two are the same 168-byte payload, and
    // `channel[k] == values[k+8]` (the values[] array has an 8-element header
    // region). The channel[32] layout is kept because it is indexed directly by
    // GetInfo's priChIdx and carries clear INVALID(255) semantics.
    // ------------------------------------------------------------------

    nvstruct! {
        /// Thermal-channel live readings (RTSS
        /// `NV_GPU_THERMAL_THERM_CHANNEL_STATUS_PARAMS_V2`). 168 bytes. The
        /// caller sets `channel_mask` (from GetInfo); on success `channel[i]`
        /// holds the temperature for channel `i`, encoded celsius*256, for each
        /// bit set in `channel_mask`. Index `i` with GetInfo's `priChIdx[type]`.
        pub struct NV_GPU_THERMAL_THERM_CHANNEL_STATUS_PARAMS_V2 {
            pub version: NvVersion,
            pub channel_mask: u32,
            pub rsvd: Padding<[u8; 32]>,
            /// Temperature per channel (celsius*256), indexed by channel number.
            /// Read `channel[priChIdx[type]]` for the authoritative reading.
            pub channel: [i32; NV_GPU_THERMAL_THERM_CHANNEL_MAX],
        }
    }

    impl NV_GPU_THERMAL_THERM_CHANNEL_STATUS_PARAMS_V2 {
        /// Decode a channel slot into degrees Celsius, if valid (>0, <255C).
        pub fn decode(value: i32) -> Option<f32> {
            let v = value as f32 / 256.0;
            (v > 0.0 && v < 255.0).then_some(v)
        }

        /// Temperature at a channel index (e.g. `priChIdx[GPU_MAX]`).
        pub fn get_temp(&self, channel: usize) -> Option<f32> {
            self.channel.get(channel).copied().and_then(Self::decode)
        }
    }

    nvversion! { @=NV_GPU_THERMAL_THERM_CHANNEL_STATUS NV_GPU_THERMAL_THERM_CHANNEL_STATUS_PARAMS_V2(2) = 168 }

    nvapi! {
        /// Undocumented (NDA-private, ID 0x65fe3aad). Thermal-channel live
        /// readings (the STATUS half of the ThermChannel pair). Pass GetInfo's
        /// `channel_mask`; read `channel[priChIdx[type]]` for each type's temp.
        pub unsafe fn NvAPI_GPU_ThermChannelGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pStatus: *mut NV_GPU_THERMAL_THERM_CHANNEL_STATUS) -> NvAPI_Status;
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
