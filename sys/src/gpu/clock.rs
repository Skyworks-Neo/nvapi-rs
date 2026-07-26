use crate::prelude_::*;

pub const NVAPI_MAX_GPU_CLOCKS: usize = 32;
pub const NVAPI_MAX_GPU_PUBLIC_CLOCKS: usize = 32;
pub const NVAPI_MAX_GPU_PERF_CLOCKS: usize = 32;
pub const NVAPI_MAX_GPU_PERF_VOLTAGES: usize = 16;
pub const NVAPI_MAX_GPU_PERF_PSTATES: usize = 16;

nvenum! {
    /// An index into NV_GPU_CLOCK_FREQUENCIES.domain[]
    pub enum NV_GPU_PUBLIC_CLOCK_ID / PublicClockId {
        NVAPI_GPU_PUBLIC_CLOCK_GRAPHICS / Graphics = 0,
        NVAPI_GPU_PUBLIC_CLOCK_MEMORY / Memory = 4,
        NVAPI_GPU_PUBLIC_CLOCK_PROCESSOR / Processor = 7,
        NVAPI_GPU_PUBLIC_CLOCK_VIDEO / Video = 8,
        NVAPI_GPU_PUBLIC_CLOCK_UNDEFINED / Undefined = NVAPI_MAX_GPU_PUBLIC_CLOCKS,
    }
}

nvenum_display! {
    PublicClockId => _
}

nvstruct! {
    /// Used in [NvAPI_GPU_GetAllClockFrequencies]\(\)
    pub struct NV_GPU_CLOCK_FREQUENCIES_V1 {
        /// Structure version
        pub version: NvVersion,
        /// These bits are reserved for future use.
        ///
        /// `bits:2` is [NV_GPU_CLOCK_FREQUENCIES_CLOCK_TYPE]. Used to specify the type of clock to be returned.
        pub reserved: u32,
        pub domain: Array<[NV_GPU_CLOCK_FREQUENCIES_DOMAIN; NVAPI_MAX_GPU_PUBLIC_CLOCKS]>,
    }
}

impl NV_GPU_CLOCK_FREQUENCIES_V1 {
    pub fn clock_type(&self) -> NV_GPU_CLOCK_FREQUENCIES_CLOCK_TYPE {
        (self.reserved & 3) as _
    }

    pub fn set_clock_type(&mut self, value: NV_GPU_CLOCK_FREQUENCIES_CLOCK_TYPE) {
        self.reserved = (value as u32) & 3;
    }
}

nvversion! { NV_GPU_CLOCK_FREQUENCIES_V1(1) }
nvversion! { NV_GPU_CLOCK_FREQUENCIES_V1(2) }
nvversion! { @=NV_GPU_CLOCK_FREQUENCIES NV_GPU_CLOCK_FREQUENCIES_V1(3) }

nvenum! {
    /// Used in [NvAPI_GPU_GetAllClockFrequencies]\(\)
    pub enum NV_GPU_CLOCK_FREQUENCIES_CLOCK_TYPE / ClockFrequencyType {
        NV_GPU_CLOCK_FREQUENCIES_CURRENT_FREQ / Current = 0,
        NV_GPU_CLOCK_FREQUENCIES_BASE_CLOCK / Base = 1,
        NV_GPU_CLOCK_FREQUENCIES_BOOST_CLOCK / Boost = 2,
        NV_GPU_CLOCK_FREQUENCIES_CLOCK_TYPE_NUM / Count = 3,
    }
}

nvenum_display! {
    ClockFrequencyType => _
}

nvstruct! {
    pub struct NV_GPU_CLOCK_FREQUENCIES_DOMAIN {
        /// Set if this domain is present on this GPU
        pub bIsPresent: BoolU32,
        /// Clock frequency (kHz)
        pub frequency: u32,
    }
}

nvapi! {
    pub type GPU_GetAllClockFrequenciesFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, pClkFreqs: *mut NV_GPU_CLOCK_FREQUENCIES) -> NvAPI_Status;

    /// This function retrieves the [NV_GPU_CLOCK_FREQUENCIES] structure for the specified physical GPU.
    ///
    /// For each clock domain:
    /// - bIsPresent is set for each domain that is present on the GPU
    /// - frequency is the domain's clock freq in kHz
    ///
    /// Each domain's info is indexed in the array.  For example:
    /// `clkFreqs.domain[NVAPI_GPU_PUBLIC_CLOCK_MEMORY]` holds the info for the MEMORY domain.
    pub unsafe fn NvAPI_GPU_GetAllClockFrequencies;
}

/// Undocumented API
pub mod private {
    use crate::prelude_::*;

    // undocumented constants
    pub const NVAPI_MAX_USAGES_PER_GPU: usize = 8;
    pub const NVAPI_MAX_CLOCKS_PER_GPU: usize = 288;

    nvstruct! {
        pub struct NV_USAGES_INFO_USAGE {
            pub bIsPresent: BoolU32,
            /// % 0 to 100 usage
            pub percentage: u32,
            pub unknown: [u32; 2],
        }
    }

    nvstruct! {
        pub struct NV_USAGES_INFO_V1 {
            pub version: NvVersion,
            pub flags: u32,
            /// (core_usage, memory_usage, video_engine_usage), probably indexed by NV_GPU_UTILIZATION_DOMAIN_ID
            pub usages: Array<[NV_USAGES_INFO_USAGE; NVAPI_MAX_USAGES_PER_GPU]>,
        }
    }

    nvversion! { @=NV_USAGES_INFO NV_USAGES_INFO_V1(1) }

    nvapi! {
        pub type GPU_GetUsagesFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, pUsagesInfo: *mut NV_USAGES_INFO) -> NvAPI_Status;

        /// Undocumented function. Probably deprecated and replaced with NvAPI_GPU_GetDynamicPstatesInfoEx()
        pub unsafe fn NvAPI_GPU_GetUsages;
    }

    nvstruct! {
        pub struct NV_CLOCKS_INFO_V1 {
            pub version: NvVersion,
            pub clocks: Array<[u32; NVAPI_MAX_CLOCKS_PER_GPU]>,
        }
    }

    nvversion! { @=NV_CLOCKS_INFO NV_CLOCKS_INFO_V1(1) }

    nvapi! {
        pub type GPU_GetAllClocksFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, pClocksInfo: *mut NV_CLOCKS_INFO) -> NvAPI_Status;

        /// Undocumented function. Probably deprecated and replaced with [NvAPI_GPU_GetAllClockFrequencies()](super::NvAPI_GPU_GetAllClockFrequencies)
        ///
        /// ```
        /// memory_clock = clocks[8] * 0.001f;
        ///
        /// if clocks[30] != 0 {
        /// core_clock = clocks[30] * 0.0005f
        /// shader_clock = clocks[30] * 0.001f
        /// } else {
        /// core_clock = clocks[0] * 0.001f
        /// shader_clock = clocks[14] * 0.001f
        /// }
        /// ```
        pub unsafe fn NvAPI_GPU_GetAllClocks;
    }

    // ------------------------------------------------------------------
    // GetAllClocks V2 — the "effective clocks" layout (same function ID
    // 0x1bd69f49, different struct). RTSS (RivaTuner) source names this
    // `NV_GPU_CLOCK_INFO_V2` and reads `extendedDomain[GRAPHICS/MEMORY/
    // PROCESSOR].effectiveFrequency` for the effective core/memory clocks
    // (the actually-running, boosted clocks — distinct from the
    // GetAllClockFrequencies base/boost/current table).
    // ------------------------------------------------------------------

    nvenum! {
        /// Clock domain id (RTSS `NV_GPU_CLOCK_DOMAIN_ID`). Indexes the
        /// `domain[]` / `extended_domain[]` arrays. Only GRAPHICS(0)/MEMORY(4)/
        /// PROCESSOR(7) are read for effective clocks; the rest are research.
        /// (RTSS aliases some domains to the same value — e.g. NV==GPC==0 —
        /// those aliases are omitted; Rust enums can't repeat discriminants.)
        pub enum NV_GPU_CLOCK_DOMAIN_ID / ClockDomainId {
            NV_GPU_CLOCK_DOMAIN_GPC / Gpc = 0,
            NV_GPU_CLOCK_DOMAIN_XBAR / Xbar = 1,
            NV_GPU_CLOCK_DOMAIN_SYS / Sys = 2,
            NV_GPU_CLOCK_DOMAIN_HUB / Hub = 3,
            NV_GPU_CLOCK_DOMAIN_M / M = 4,
            NV_GPU_CLOCK_DOMAIN_HOST / Host = 5,
            NV_GPU_CLOCK_DOMAIN_DISP / Disp = 6,
            NV_GPU_CLOCK_DOMAIN_HOTCLK / Hotclk = 7,
            NV_GPU_CLOCK_DOMAIN_PCLK0 / Pclk0 = 8,
            NV_GPU_CLOCK_DOMAIN_PCLK1 / Pclk1 = 9,
            NV_GPU_CLOCK_DOMAIN_BYPCLK / Bypclk = 10,
            NV_GPU_CLOCK_DOMAIN_XCLK / Xclk = 11,
            NV_GPU_CLOCK_DOMAIN_VPV / Vpv = 12,
            NV_GPU_CLOCK_DOMAIN_VPS / Vps = 13,
            NV_GPU_CLOCK_DOMAIN_GPUCACHECLK / Gpucacheclk = 14,
            NV_GPU_CLOCK_DOMAIN_GPC2 / Gpc2 = 15,
            NV_GPU_CLOCK_DOMAIN_XBAR2 / Xbar2 = 16,
            NV_GPU_CLOCK_DOMAIN_SYS2 / Sys2 = 17,
            NV_GPU_CLOCK_DOMAIN_HUB2 / Hub2 = 18,
            NV_GPU_CLOCK_DOMAIN_LEG / Leg = 19,
            NV_GPU_CLOCK_DOMAIN_PWR / Pwr = 20,
            NV_GPU_CLOCK_DOMAIN_MSD / Msd = 21,
            NV_GPU_CLOCK_DOMAIN_UTILS / Utils = 22,
            NV_GPU_CLOCK_DOMAIN_COLD_NV / ColdNv = 23,
            NV_GPU_CLOCK_DOMAIN_COLD_HOTCLK / ColdHotclk = 24,
            NV_GPU_CLOCK_DOMAIN_LTC2 / Ltc2 = 25,
            NV_GPU_CLOCK_DOMAIN_2D / TwoD = 26,
            NV_GPU_CLOCK_DOMAIN_3D / ThreeD = 27,
            NV_GPU_CLOCK_DOMAIN_HOST1X / Host1x = 28,
            NV_GPU_CLOCK_DOMAIN_DISP0 / Disp0 = 29,
            NV_GPU_CLOCK_DOMAIN_DISP1 / Disp1 = 30,
            NV_GPU_CLOCK_DOMAIN_PCIEGEN / Pciegen = 31,
        }
    }

    nvenum_display! {
        ClockDomainId => _
    }

    nvstruct! {
        /// Per-domain clock entry (RTSS `NV_GPU_CLOCK_INFO_DOMAIN`). The
        /// `flags` word packs: `bIsPresent:1 | bDrivingDDR:1 | bSetClock:1 |
        /// pstateUsage:2 | reserved:27` (RTSS C bitfield). `frequency` is kHz.
        pub struct NV_GPU_CLOCK_INFO_DOMAIN {
            pub frequency: u32,
            pub flags: u32,
        }
    }

    impl NV_GPU_CLOCK_INFO_DOMAIN {
        /// Bit 0: this domain is present on the GPU.
        pub fn is_present(&self) -> bool {
            self.flags & 1 != 0
        }
        /// Bit 1: driving DDR memory.
        pub fn is_driving_ddr(&self) -> bool {
            self.flags & 2 != 0
        }
        /// Bit 2: clock is set (not default).
        pub fn is_set_clock(&self) -> bool {
            self.flags & 4 != 0
        }
        /// Bits 3..4: P-state usage (0..3, semantics undocumented; research).
        pub fn pstate_usage(&self) -> u32 {
            (self.flags >> 3) & 3
        }
    }

    nvstruct! {
        /// Per-domain effective-clock entry (RTSS inline struct inside
        /// `NV_GPU_CLOCK_INFO_V2.extendedDomain[]`). `effective_frequency` is
        /// the actually-running frequency in kHz; `ratio_domain`/`ratio`
        /// relate it to a parent domain (research semantics).
        pub struct NV_GPU_CLOCK_INFO_EXTENDED_DOMAIN {
            pub effective_frequency: u32,
            pub ratio_domain: NV_GPU_CLOCK_DOMAIN_ID,
            pub ratio: u32,
            pub reserved: Padding<[u32; 4]>,
        }
    }

    nvstruct! {
        /// GetAllClocks V2 "effective clocks" params (RTSS
        /// `NV_GPU_CLOCK_INFO_V2`). `domain[]` holds per-domain presence +
        /// base frequency; `extended_domain[]` holds the effective (running)
        /// frequency per domain. 32 entries each (`NVAPI_MAX_GPU_CLOCKS`).
        pub struct NV_GPU_CLOCK_INFO_V2 {
            pub version: NvVersion,
            pub domain: Array<[NV_GPU_CLOCK_INFO_DOMAIN; super::NVAPI_MAX_GPU_CLOCKS]>,
            pub extended_domain: Array<[NV_GPU_CLOCK_INFO_EXTENDED_DOMAIN; super::NVAPI_MAX_GPU_CLOCKS]>,
        }
    }

    nvversion! { @=NV_GPU_CLOCK_EFFECTIVE_INFO NV_GPU_CLOCK_INFO_V2(2) }

    // Note: GetAllClocks (ID 0x1bd69f49) is FFI-bound once above with the V1
    // `NV_CLOCKS_INFO` pointer type. The V2 effective-clocks layout uses the
    // SAME function ID — callers pass a `*mut NV_GPU_CLOCK_INFO_V2` (cast to
    // the V1 pointer type at the call site), since the driver only sees a
    // version-tagged buffer. No separate FFI binding is needed.

    pub type NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_CONTROL_PROG_V1 = i32;

    nvstruct! {
        pub struct NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_CONTROL_V1 {
            pub clock_type: u32,
            pub rsvd: Padding<[u32; 4]>,
            /// offsetFrequencyKhz
            pub freqDeltaKHz: NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_CONTROL_PROG_V1,
            pub padding: Padding<[u32; 3]>,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_V1 {
            pub version: NvVersion,
            pub mask: ClockMask,
            pub unknown: Padding<[u32; 8]>,
            pub points: Array<[NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_CONTROL_V1; 255]>,
        }
    }

    nvversion! { NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_V1(1) = 9248 }
    nvversion! { @=NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_V1(2) }

    nvapi! {
        /// Pascal and later
        pub unsafe fn NvAPI_GPU_ClockClientClkVfPointsGetControl(hPhysicalGPU: NvPhysicalGpuHandle, pClockTable: *mut NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Pascal and later
        pub unsafe fn NvAPI_GPU_ClockClientClkVfPointsSetControl(hPhysicalGPU: NvPhysicalGpuHandle, pClockTable: *const NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL) -> NvAPI_Status;
    }

    nvstruct! {
        pub struct NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_INFO_ENTRY {
            pub disabled: u32,
            pub clockType: super::NV_GPU_PUBLIC_CLOCK_ID,
            pub unknown0: Padding<[u32; 8]>,
            pub rangeMax: i32,
            pub rangeMin: i32,
            pub vfpIndexMin: u8,
            pub vfpIndexMax: u8,
            pub padding: Padding<[u8; 2]>,
            pub unknown1: Padding<[u32; 5]>,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_INFO_V1 {
            pub version: NvVersion,
            pub mask: ClockMask<1>,
            pub zero: Padding<[u32; 8]>,
            pub entries: Array<[NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_INFO_ENTRY; 32]>,
        }
    }

    nvversion! { @=NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_INFO NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_INFO_V1(1) = 2344 }

    nvapi! {
        /// Pascal only
        pub unsafe fn NvAPI_GPU_ClockClientClkDomainsGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pClockRanges: *mut NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_INFO) -> NvAPI_Status;
    }

    nvenum! {
        pub enum NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_TYPE / VfPointType {
            NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_TYPE_PROG / Prog = 0,
            NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_TYPE_FIXED / Fixed = 1,
            NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_TYPE_DYN / Dyn = 2,
        }
    }

    nvenum_display! {
        VfPointType => {
            Prog = "Prog",
            Fixed = "Fixed",
            Dyn = "Dyn",
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_CLOCK {
            pub clock_type: u32,
            pub b_voltage_based: u8,
            pub rsvd: Padding<[u8; 19]>,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_V1 {
            pub version: NvVersion,
            pub mask: ClockMask,
            pub unknown: Padding<[u32; 8]>,
            pub clocks: Array<[NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_CLOCK; 255]>,
        }
    }

    nvversion! { @=NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_V1(1) = 6188 }

    nvapi! {
        /// Pascal and later
        pub unsafe fn NvAPI_GPU_ClockClientClkVfPointsGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pClockMasks: *mut NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO) -> NvAPI_Status;
    }

    nvenum! {
        pub enum NV_GPU_CLOCK_LOCK_MODE / ClockLockMode {
            NVAPI_GPU_CLOCK_LOCK_NONE / None = 0,
            NVAPI_GPU_CLOCK_LOCK_MANUAL_FREQUENCY / ManualFrequency = 2,
            NVAPI_GPU_CLOCK_LOCK_MANUAL_VOLTAGE / ManualVoltage = 3,
        }
    }

    nvenum! {
        pub enum NV_PERF_CLIENT_LIMIT_ID / PerfLimitId {
            NV_PERF_CLIENT_LIMIT_ID_GPU / Gpu = 0,
            NV_PERF_CLIENT_LIMIT_ID_GPU_UNKNOWN / GpuLowerbound = 1,
            NV_PERF_CLIENT_LIMIT_ID_MEMORY / Memory = 2,
            NV_PERF_CLIENT_LIMIT_ID_MEMORY_UNKNOWN / MemoryLowerbound = 3,
            NV_PERF_CLIENT_LIMIT_ID_UNKNOWN_4 / Unknown_4 = 4,
            NV_PERF_CLIENT_LIMIT_ID_UNKNOWN_5 / Unknown_5 = 5,
            NV_PERF_CLIENT_LIMIT_ID_VOLTAGE / Voltage = 6,
        }
    }

    nvenum_display! {
        PerfLimitId => {
            Gpu = "GPU Core Upperbound",
            GpuLowerbound = "GPU Core Lowerbound",
            Memory = "Memory Upperbound",
            MemoryLowerbound = "Memory Lowerbound",
            _ = _,
        }
    }

    nvstruct! {
        pub struct NV_GPU_PERF_CLIENT_LIMITS_ENTRY {
            pub id: NV_PERF_CLIENT_LIMIT_ID, // entry index
            pub b: u32, // 0
            pub mode: NV_GPU_CLOCK_LOCK_MODE, // 0 = default, 3 = manual voltage
            pub d: u32, // 0
            /// voltage uV or freq kHz depending on `id`
            pub value: u32, // 0 unless set explicitly, seems to always get set on the last/highest entry only
            pub clock_id: super::NV_GPU_PUBLIC_CLOCK_ID,
        }
    }

    nvstruct! {
        // 2-030c: 0C 03 02 00 00 00 00 00 01 00 00 00 06 00 00 00
        pub struct NV_GPU_PERF_CLIENT_LIMITS_V2 {
            pub version: NvVersion,
            pub flags: u32, // unknown, only see 0
            pub count: u32,
            pub entries: Array<[NV_GPU_PERF_CLIENT_LIMITS_ENTRY; 0x20]>,
        }
    }

    impl NV_GPU_PERF_CLIENT_LIMITS_V2 {
        pub fn entries(&self) -> &[NV_GPU_PERF_CLIENT_LIMITS_ENTRY] {
            &self.entries[..self.count as usize]
        }
    }

    nvversion! { @=NV_GPU_PERF_CLIENT_LIMITS NV_GPU_PERF_CLIENT_LIMITS_V2(2) = 0x30c }

    nvapi! {
        /// Pascal only
        pub unsafe fn NvAPI_GPU_PerfClientLimitsGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pClockLocks: *mut NV_GPU_PERF_CLIENT_LIMITS) -> NvAPI_Status;
    }

    nvapi! {
        /// Pascal only
        pub unsafe fn NvAPI_GPU_PerfClientLimitsSetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pClockLocks: *const NV_GPU_PERF_CLIENT_LIMITS) -> NvAPI_Status;
    }
}
