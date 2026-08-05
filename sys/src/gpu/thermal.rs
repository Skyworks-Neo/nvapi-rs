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

    impl NV_GPU_THERMAL_THERM_CHANNEL_INFO_V1 {
        /// `data.device.thermDevIdx` (byte 0 of the union): identifies the
        /// physical thermal device (sensor group) this channel reads from.
        /// Two channels sharing the same `therm_dev_idx` read the same physical
        /// sensor — distinguished by `therm_dev_prov_idx`.
        pub fn therm_dev_idx(&self) -> u8 {
            self.data.data[0]
        }

        /// `data.device.thermDevProvIdx` (byte 1 of the union): the provider
        /// index within the device. A `(dev, 1)` channel's STATUS reading has
        /// `offset_hw` already applied by the driver; the matching `(dev, 0)`
        /// reading has not (even when its own `offset_hw` is non-zero).
        pub fn therm_dev_prov_idx(&self) -> u8 {
            self.data.data[1]
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
                .filter(|&i| {
                    i < NV_GPU_THERMAL_THERM_CHANNEL_MAX && self.channel_mask & (1u32 << i) != 0
                })
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
            self.primary_index(ty).and_then(|i| self.channel.get(i))
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

    // ------------------------------------------------------------------
    // Mobile GPU target-temperature wall ("targettemp" / 温度墙) — the PRIVATE
    // ClientThermalPolicies RMW pair, RE'd from the ref-tool CLI.
    //
    // the ref-tool CLI `-targettemp:<C>` (setTargetTemperature, sub_140013090) does:
    //   1. memset a stack buffer (984 B effective); dword0 = 0x203D8 (version
    //      magic: struct version 2, size 984), dword1 = mask = 1 << policy_index.
    //   2. GET-prime 0xC4554575 fills the buffer with current policy state.
    //   3. Write target = (int)(celsius * 256.0) at dword (15*policy_index + 7).
    //   4. SET 0xE097144F applies the patched buffer.
    //
    // This is NOT the documented ClientThermalPoliciesSetStatus (0x34C0B13D),
    // which returns OK on mobile but silently does not persist. The private
    // pair is what actually writes the wall (nvidia-smi 87->82 confirmed).
    // Both IDs sit in nvapi64.dll's static table off_1804DD000 and resolve in
    // nvoc's process (probe-confirmed: SET -> 0x7FFE90A12750 on RTX 4060 Laptop).
    //
    // Version magic: the ref tool writes dword0 = 0x203D8 = version 2 | size 984.
    // the ref tool's stack buffer is _DWORD v30[248] (992 B) but only the first 984 B
    // (header 8 + memset 0x3D0=976) are used; the magic's size field (984) is
    // what the driver validates. Each policy entry is 15 dwords (60 B); the
    // target-temp Q8 field is entry dword 7. The rest of each entry is opaque
    // (GET-filled) and must be preserved by the RMW.
    // ------------------------------------------------------------------

    /// Number of target-temp policy entries the 984-byte buffer can hold.
    /// 984 B = 8 B header + N*(60 B); floor((984-8)/60) = 16, but the ref tool only
    /// ever uses index 0..3 — keep 16 to cover the full buffer.
    pub const NV_GPU_CLIENT_THERMAL_TARGET_ENTRIES_MAX: usize = 16;

    nvstruct! {
        /// Target-temperature control read-modify-write buffer (RE'd from the ref tool;
        /// NDA). dword0 = version (0x203D8 = v2|984), dword1 = mask = (1<<idx).
        /// Per-entry target temp (celsius*256) at dword (15*index + 7). The bulk
        /// of the buffer is opaque — GET fills it, the caller patches one entry's
        /// temp field, SET applies.
        pub struct NV_GPU_CLIENT_THERMAL_TARGET_STATUS_V2 {
            pub version: NvVersion,
            /// Bitmask selecting which policy entry to read/write (1 << index).
            pub mask: u32,
            /// Opaque policy table (raw; GET-filled, preserve on RMW).
            pub payload: Padding<[u8; 984 - 8]>,
        }
    }

    impl NV_GPU_CLIENT_THERMAL_TARGET_STATUS_V2 {
        /// Access the raw opaque payload bytes (for diagnostics/dumps).
        pub fn payload_bytes(&self) -> &[u8] {
            &self.payload.data[..]
        }

        /// Per-entry stride in BYTES = 15 dwords (the ref-tool CLI: v30[15*idx + 7]).
        const ENTRY_STRIDE_BYTES: usize = 15 * 4;
        /// Byte offset WITHIN `payload` of entry 0's target-temp field.
        /// Buffer dword (15*idx + 7); payload starts at buffer byte 8, so
        /// idx-0 base = 7*4 - 8 = 20.
        const TEMP_BASE_PAYLOAD_OFF: usize = 7 * 4 - 8;

        fn temp_off(&self, index: usize) -> Option<usize> {
            Self::TEMP_BASE_PAYLOAD_OFF.checked_add(Self::ENTRY_STRIDE_BYTES.checked_mul(index)?)
        }

        /// Read the target temp for `index`, decoded to degrees Celsius.
        /// Returns None if the index is out of range.
        pub fn target_temp_c(&self, index: usize) -> Option<f32> {
            let off = self.temp_off(index)?;
            self.payload.get(off..off + 4).map(|b| {
                let q8 = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                q8 as f32 / 256.0
            })
        }

        /// Write the target temp for `index` (encoded celsius*256) and set the
        /// mask bit for that entry.
        pub fn set_target_temp_c(&mut self, index: usize, celsius: f32) {
            if let Some(off) = self.temp_off(index) {
                if let Some(slot) = self.payload.get_mut(off..off + 4) {
                    let q8 = (celsius * 256.0) as i32 as u32;
                    slot.copy_from_slice(&q8.to_le_bytes());
                    self.mask |= 1u32 << index;
                }
            }
        }
    }

    nvversion! { @=NV_GPU_CLIENT_THERMAL_TARGET_STATUS NV_GPU_CLIENT_THERMAL_TARGET_STATUS_V2(2) = 984 }

    nvapi! {
        /// Undocumented (NDA, ID 0xC4554575). Fills the target-temp control
        /// buffer (the GET-prime half of setTargetTemperature). Pair with
        /// SetStatus for the read-modify-write.
        pub unsafe fn NvAPI_GPU_ClientThermalTargetGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pStatus: *mut NV_GPU_CLIENT_THERMAL_TARGET_STATUS) -> NvAPI_Status;
    }

    nvapi! {
        /// Undocumented (NDA, ID 0xE097144F). Applies the target-temp control
        /// buffer (the SET half of setTargetTemperature). Caller writes the
        /// target temp into the active policy entry first (celsius*256).
        pub unsafe fn NvAPI_GPU_ClientThermalTargetSetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pStatus: *const NV_GPU_CLIENT_THERMAL_TARGET_STATUS) -> NvAPI_Status;
    }

    // ---- Private ClientThermalPolicies GetInfo (ID 0x2F69F8E5) -------------
    // RE'd from the ref-tool CLI sub_14002C410 (GPUHandle::queryTargetTemperature).
    // This is the PRIVATE sibling of the documented ClientThermalPoliciesGetInfo
    // (0x0D258BB5) — same family split as the target-temp GET/SET pair. the ref tool
    // resolves 0x2F69F8E5 (not 0x0D258BB5) and passes version magic 0x33D58
    // (= v3 | size 15704). The documented path is a different, smaller struct.
    //
    // Layout (from queryTargetTemperature, little-endian dword indexing):
    //   dword[0]      = version magic 0x33D58
    //   dword[2]      = packed policy indices: LOBYTE = GPS (target-temp) idx,
    //                   BYTE1 = acoustics idx; 0xFF = invalid. If GPS invalid but
    //                   acoustics valid, the driver wants acoustics (desktop case
    //                   — there the writable slot is AcousticCurr, not GpsCurr).
    //   For the chosen index `idx`, min/default/max (Q8 celsius, /256) at:
    //   dword[231*idx + 232 / 233 / 234]. Entry stride 231 dwords (924 B).
    //   (Live-verified RTX 4060 Laptop: GPS idx 2, range [75, 87], default 87.)

    /// Total size of the private thermal-policy GetInfo buffer (version magic
    /// 0x33D58 = v3 | 15704, from the ref-tool CLI sub_14002C410 v9[0]=212312).
    pub const NV_GPU_CLIENT_THERMAL_POLICIES_PRIVATE_INFO_SIZE: usize = 15704;

    nvstruct! {
        /// Raw private ClientThermalPolicies GetInfo buffer (ID 0x2F69F8E5,
        /// magic 0x33D58). Kept as opaque bytes — only the fields RE'd from
        /// the ref tool's queryTargetTemperature are decoded by the accessors below.
        pub struct NV_GPU_CLIENT_THERMAL_POLICIES_PRIVATE_INFO_V3 {
            pub version: NvVersion,
            pub payload: Padding<[u8; NV_GPU_CLIENT_THERMAL_POLICIES_PRIVATE_INFO_SIZE - 4]>,
        }
    }

    impl NV_GPU_CLIENT_THERMAL_POLICIES_PRIVATE_INFO_V3 {
        /// Packed dword[2]: LOBYTE = GPS (target-temp) index, BYTE1 = acoustics
        /// index. 0xFF = not exposed by the VBIOS.
        fn packed_indices(&self) -> u32 {
            // dword[2] = bytes 8..12 of the buffer (dword0=version @0..4,
            // dword1 @4..8, dword2 @8..12). Payload starts after the version
            // dword (byte 4), so dword2 is payload bytes 4..8.
            u32::from_le_bytes(self.payload.data[4..8].try_into().unwrap())
        }

        /// The GPS (target-temp) policy index, or None if the VBIOS doesn't
        /// expose one (0xFF) — caller should then try acoustics.
        pub fn gps_policy_index(&self) -> Option<u8> {
            let b = self.packed_indices() as u8;
            (b != 0xFF).then_some(b)
        }

        /// The acoustics policy index (the desktop fallback), or None (0xFF).
        pub fn acoustics_policy_index(&self) -> Option<u8> {
            let b = (self.packed_indices() >> 8) as u8;
            (b != 0xFF).then_some(b)
        }

        /// The policy index the ref tool itself chooses for target-temp control:
        /// GPS index if exposed, else acoustics (desktop), else None. This is
        /// the per-GPU discovery value that replaces hardcoding idx 2.
        pub fn target_temp_policy_index(&self) -> Option<u8> {
            self.gps_policy_index()
                .or_else(|| self.acoustics_policy_index())
        }

        /// min/default/max target temp (celsius) for entry `idx`, Q8-decoded
        /// (/256). Mirrors dword[231*idx + 232/233/234]. None if out of range.
        pub fn target_temp_range(&self, idx: u8) -> Option<(f32, f32, f32)> {
            const STRIDE: usize = 231;
            const MIN_OFF: usize = 232; // default @ +1, max @ +2
            let base = STRIDE.checked_mul(idx as usize)?.checked_add(MIN_OFF)?;
            // Each dword index is relative to the buffer start (incl. version);
            // payload starts right after the version dword (byte 4), so the
            // byte offset into payload = dword_index*4 - 4.
            let read = |dword_index: usize| -> Option<f32> {
                let byte_off = dword_index.checked_mul(4)?.checked_sub(4)?;
                let end = byte_off.checked_add(4)?;
                let bytes = self.payload.data.get(byte_off..end)?;
                let v = u32::from_le_bytes(bytes.try_into().ok()?) as i32 as f32;
                Some(v / 256.0)
            };
            Some((read(base)?, read(base + 1)?, read(base + 2)?))
        }

        pub fn payload_bytes(&self) -> &[u8] {
            &self.payload.data[..]
        }
    }

    nvversion! { @=NV_GPU_CLIENT_THERMAL_POLICIES_PRIVATE_INFO NV_GPU_CLIENT_THERMAL_POLICIES_PRIVATE_INFO_V3(3) = 15704 }

    nvapi! {
        /// Undocumented (NDA, ID 0x2F69F8E5). PRIVATE ClientThermalPolicies
        /// GetInfo — returns the ~15.7 KB policy table the ref tool's
        /// queryTargetTemperature reads to find the target-temp policy index
        /// (GPS lobte, acoustics fallback) and its VBIOS min/default/max range.
        /// NOT the documented 0x0D258BB5 (different, smaller struct).
        pub unsafe fn NvAPI_GPU_ClientThermalPoliciesPrivateGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_CLIENT_THERMAL_POLICIES_PRIVATE_INFO) -> NvAPI_Status;
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
