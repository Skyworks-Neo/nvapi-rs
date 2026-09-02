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
        NV_GPU_CLOCK_FREQUENCIES_CLOCK_TYPE::with_repr((self.reserved & 3) as i32)
    }

    pub fn set_clock_type(&mut self, value: NV_GPU_CLOCK_FREQUENCIES_CLOCK_TYPE) {
        self.reserved = (value.repr() as u32) & 3;
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

// ------------------------------------------------------------------
// NvAPI_GPU_GetPerfClocks / SetPerfClocks (ID 0x1EA54A3B / 0x07BCF4AC).
// Fermi/Kepler-era per-pstate clock/voltage table. Layout from the
// vertminer wrapper (reverse/vertminer-nvidia-master/compat/nvapi/
// nvapi_vertminer.h): version-2 magic 0x22A74 = 10868 bytes; the first
// 12 dwords were field-mapped by live probing, the remaining 2705 dwords
// were never decoded (observed mostly-zero memory-domain clock fields).
// vertminer's SET wrapper is marked "error" — never observed working;
// modern (Pascal+) drivers likely reject both. Registered for
// completeness only.
// ------------------------------------------------------------------

nvstruct! {
    /// Per-pstate performance clocks table (Kepler-era, undocumented).
    /// 10868 bytes, version magic 0x22A74 (v2).
    pub struct NV_GPU_PERF_CLOCKS_V2 {
        pub version: NvVersion,
        /// Observed constant 4.
        pub val1: u32,
        /// Observed 2 or 0.
        pub val2: u32,
        /// Observed constant 2.
        pub val3: u32,
        /// Observed constant 3.
        pub val4: u32,
        pub pStateId: u32,
        /// Observed 0 or 2.
        pub val6: u32,
        /// Observed constant 4.
        pub val7: u32,
        /// Observed 0.
        pub val8: u32,
        /// Memory frequency kHz (observed 405000).
        pub memFreq1: u32,
        /// Memory frequency kHz (observed 405000).
        pub memFreq2: u32,
        /// Memory frequency minimum kHz (observed 101250).
        pub memFreqMin: u32,
        /// Undecoded tail (2705 dwords; mostly-zero memory-domain fields).
        pub pad: Array<[u32; 2705]>,
    }
}

nvversion! { @=NV_GPU_PERF_CLOCKS NV_GPU_PERF_CLOCKS_V2(2) }

nvapi! {
    pub type GPU_GetPerfClocksFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, numClocks: u32, pPerfClocks: *mut NV_GPU_PERF_CLOCKS) -> NvAPI_Status;

    /// Kepler-era per-pstate clock table GET. vertminer resolves 0x1EA54A3B
    /// with the 10868-byte V2 struct; expect NotSupported on Pascal+.
    pub unsafe fn NvAPI_GPU_GetPerfClocks;
}

nvapi! {
    pub type GPU_SetPerfClocksFn = extern "C" fn(hPhysicalGPU: NvPhysicalGpuHandle, numClocks: u32, pPerfClocks: *const NV_GPU_PERF_CLOCKS) -> NvAPI_Status;

    /// Kepler-era per-pstate clock table SET (0x07BCF4AC). vertminer's own
    /// wrapper is commented "// error" — no known working usage; bound for
    /// completeness.
    pub unsafe fn NvAPI_GPU_SetPerfClocks;
}

/// Undocumented API
pub mod undocumented {
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
        /// ```text
        /// memory_clock = clocks[8] * 0.001f;
        ///
        /// if clocks[30] != 0 {
        /// core_clock = clocks[30] * 0.0005f;
        /// shader_clock = clocks[30] * 0.001f;
        /// } else {
        /// core_clock = clocks[0] * 0.001f;
        /// shader_clock = clocks[14] * 0.001f;
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

    nvstruct! {
        /// GetAllClockFrequencies V3 "compact" variant (magic 0x30108, 264B),
        /// discovered in AmpereOC (sub_14005C998). `mode` selects the table:
        /// 1 = BASE clocks, 2 = BOOST clocks. 8 slots of {valid, value_kHz}
        /// at 32-byte stride: slot[0] = core (kHz), slot[1] = memory (kHz).
        /// The driver ORs status flags into `mode` on return
        /// (0x0800_0001 / 0x0900_0002 observed on Ada mobile).
        /// Live-verified 4060L: base 2175/8001 MHz, boost 2370/8001 MHz.
        pub struct NV_GPU_CLOCK_INFO_V3_COMPACT {
            pub version: NvVersion,
            pub mode: u32,
            pub slots: Array<[NV_GPU_CLOCK_INFO_V3_SLOT; 8]>,
        }
    }

    nvstruct! {
        pub struct NV_GPU_CLOCK_INFO_V3_SLOT {
            pub valid: u32,
            pub value_kHz: u32,
            pub reserved: Padding<[u32; 6]>,
        }
    }

    nvversion! { NV_GPU_CLOCK_INFO_V3_COMPACT(3) = 0x108 }

    // Note: GetAllClocks (ID 0x1bd69f49) is FFI-bound once above with the V1
    // `NV_CLOCKS_INFO` pointer type. The V2 effective-clocks layout uses the
    // SAME function ID — callers pass a `*mut NV_GPU_CLOCK_INFO_V2` (cast to
    // the V1 pointer type at the call site), since the driver only sees a
    // version-tagged buffer. No separate FFI binding is needed.

    pub type NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_CONTROL_PROG_V1 = i32;

    // nvapioc (reverse/nvapioc-master) navigates the V/F table by VOLTAGE,
    // not point index: GET the mask+curve, find the entry whose voltageUV
    // matches the requested mV, patch that entry's freqDeltaKHz. It also
    // multiplies the delta by 2 before SET and divides by 2 after GET —
    // on R610.74 our live round-trip shows plain kHz units (90000 delta →
    // exactly +90 MHz), so the ×2 is either a Pascal-era driver unit or
    // nvapioc's own CLI convention; do NOT copy it blindly.
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

    // IDA R610.74 impl (sub_180204C30, the 0x928 ClkDomainsGetInfo fill
    // loop): user entry i is 72 bytes at struct+0x28; the handler copies
    // RM record dwords @+80/+84 into entry+0x28/+0x2C (rangeMax/Min) and
    // TWO SEPARATE BYTES @+88/+89 into entry+0x30/+0x31 — MinerLamp's
    // `tempMax i32` reading of the same dword is wrong (only 2 bytes are
    // written, +0x32/+0x33 stay zero). The byte pair is confirmed to be
    // the domain's V/F-point index bounds: the public VfPoints SetControl
    // (sub_1802071C0) gates per-point scaling on
    // `point >= rec[88] && point <= rec[89]`. `disabled`@entry+0 is the
    // inverted RM type byte@+64: 1 = domain present without range info
    // (type 0), 0 = range + vfp-index fields filled (type 1); any other
    // type byte aborts the whole call with -180. clockType@entry+4 is
    // filled by sub_1801FF320 from the RM domain id @rec+68.
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
            /// Select a P-State (entry value = pstate number). RE'd from
            /// the ref tool setPState: entries with id 4/5 (Unknown_4/5) use mode 1
            /// to pin the active pstate.
            NVAPI_GPU_CLOCK_LOCK_PSTATE_SELECT / PstateSelect = 1,
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
            // nvapioc (reverse/nvapioc-master) drives this exact 780-byte
            // CLOCK_LOCK form as its "-cvolt/-mvolt" VOLTAGE LOCK: RMW the
            // GET, then for the entry with id==6 set mode=3 and value=µV
            // (0 to unlock). Corroborates mode 3 = manual voltage below.
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
            counted(&*self.entries, self.count as usize)
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

    // ------------------------------------------------------------------
    // PerfLimits family — GPU frequency perf-cap (NDA). RE'd byte-exact from
    // ref tool 2's `-gpuclk:<MHz>` (`GPUHandle::setGpcClock`). DISTINCT
    // from PerfClientLimits above (P-state lock, 780B): this is a 287KB
    // struct that clamps the perf max/min frequency to a cap value, not a
    // P-state/mode entry table. The medium GetInfo struct returns the entry
    // count; the large GetStatus/SetStatus structs share one layout.
    //
    // Three structs (all heap-backed in the high-level wrapper — too large for
    // a fixed-size `nvstruct!`):
    //   small  NV_GPU_PERF_CLIENT_LIMITS  magic 0x2030C 0x30C B (already wrapped above)
    //   medium NV_GPU_PERF_LIMITS_INFO    magic 0x1300C 0x300C B
    //   large  NV_GPU_PERF_LIMITS         magic 0x6642C 0x4642C B
    //
    // Large struct layout (from setGpcClock sub_140023FE0 + isPStateLocked
    // sub_14002C8E0):
    //   +0x00 u32  magic/size = 0x6642C
    //   +0x08 u32  count (entries; SET=2, GET fills)
    //   entry[k] @ +0x2C + k*0x464 (stride 0x464 = 1124 B):
    //     +0x00 (+0x2C)  type_marker u32  (SET entry0=0x58/entry1=0x5B; GET 0x5D=Pmax/0x49=Pmin)
    //     +0x30 (+0x5C)  enable u32       (2=apply cap, 0=reset)
    //     +0x58 (+0x84)  freq_kHz u32     (1000*MHz; entry0=max, entry1=min)
    //     +0x458(+0x484) locked u8        (GET only: non-zero = cap active)
    //   Medium struct: +0x00 magic 0x1300C, +0x08 count.
    // ------------------------------------------------------------------
    nvapi! {
        /// PerfLimits GetInfo (NDA 0xE63AE22B). Medium struct (magic 0x1300C);
        /// fills `count` at +0x08 — the entry count for the paired large
        /// GetStatus/SetStatus struct. RE'd from ref tool 2 isPStateLocked.
        pub unsafe fn NvAPI_GPU_PerfLimitsGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pPerfLimitsInfo: *mut u8) -> NvAPI_Status;
    }

    nvapi! {
        /// PerfLimits GetStatus (NDA 0xEFCEDD1F). Large struct (magic 0x6642C,
        /// 0x4642C B): reads back the current perf frequency caps. RE'd from
        /// ref tool 2 isPStateLocked.
        pub unsafe fn NvAPI_GPU_PerfLimitsGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pPerfLimits: *mut u8) -> NvAPI_Status;
    }

    nvapi! {
        /// PerfLimits SetStatus (NDA 0x32CA4983). Large struct (magic 0x6642C,
        /// 0x4642C B): sets the perf max/min frequency cap. RE'd from
        /// ref tool 2 `-gpuclk:<MHz>` (setGpcClock). MHz→kHz (×1000); -1=reset.
        pub unsafe fn NvAPI_GPU_PerfLimitsSetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pPerfLimits: *const u8) -> NvAPI_Status;
    }

    // ------------------------------------------------------------------
    // Driver-side OC Scanner family (NDA). RE'd from MSI's MSIOCScanner_x64
    // host (reverse/msiafterburner/Bundle/OCScanner): on drivers >= 455.00
    // the legacy user-mode scanner.dll is bypassed entirely — the host calls
    // ClientStartOcScanner and the DRIVER performs the scan, reporting
    // progress through the RegisterForOcScannerStatusUpdates callback.
    // Start/Stop/Revert all take the same 68-byte struct (magic 0x10044,
    // zeroed then version-stamped by the host; fields opaque). The register
    // call takes a 152-byte struct (magic 0x10098) whose qword at +0x50 is
    // the status callback function pointer. There is also
    // ClientGetLastOcScannerResults 0x593E8E72 (registered in nvid.rs,
    // layout unknown — not bound).
    // ------------------------------------------------------------------

    nvstruct! {
        /// Driver-side OC Scanner control (RE'd from MSIOCScanner; NDA).
        /// 68 bytes, version magic 0x10044 (v1). Fields beyond the version
        /// are opaque — the host zeroes the buffer and stamps the magic.
        pub struct NV_GPU_OC_SCANNER_CONTROL_V1 {
            pub version: NvVersion,
            pub pad: Array<[u8; 64]>,
        }
    }

    nvversion! { @=NV_GPU_OC_SCANNER_CONTROL NV_GPU_OC_SCANNER_CONTROL_V1(1) = 68 }

    nvstruct! {
        /// OC Scanner status-update registration (RE'd from nvapi64_impl.dll
        /// handler 0x180072470; NDA). 152 bytes, version magic 0x10098 (v1).
        /// Layout (IDA-verified): +0x30 = cookie (opaque u64), +0x50 =
        /// registration-validity field (NULL-checked; zeroed on RPC failure =
        /// unregister semantics), +0x78 = the callback fn pointer (driver
        /// calls this on status notifications). The callback receives a
        /// status struct: eventType(0/1)@+24, status byte@+28, flags@+32,
        /// and eventType 1 carries a ~9KB per-point payload starting at
        /// +0x6C.
        #[nv_unchecked]
        pub struct NV_GPU_OC_SCANNER_STATUS_UPDATE_PARM_V1 {
            pub version: NvVersion,
            pub pad0: Padding<[u8; 44]>,
            /// Cookie (offset 0x30). Opaque u64 passed through to the callback.
            pub cookie: u64,
            /// Registration-validity field (offset 0x50). NULL-checked by
            /// the driver; zeroed on RPC failure (unregister semantics).
            pub validity: u32,
            pub pad1: Padding<[u8; 36]>,
            /// Status callback fn pointer (offset 0x78). The driver calls
            /// this on status notifications. Exact signature not yet typed —
            /// placeholder no-arg; cast at the call site.
            pub callback: Option<unsafe extern "system" fn()>,
            pub pad2: Padding<[u8; 48]>,
        }
    }

    nvversion! { @=NV_GPU_OC_SCANNER_STATUS_UPDATE_PARM NV_GPU_OC_SCANNER_STATUS_UPDATE_PARM_V1(1) = 152 }

    pub type NV_OC_SCANNER_STATUS_CALLBACK = unsafe extern "system" fn(
        ctx: *mut std::os::raw::c_void,
        pStatus: *const NV_GPU_OC_SCANNER_STATUS,
    ) -> u32;

    nvstruct! {
        /// OC Scanner status payload handed to the Register callback. The
        /// driver owns the buffer; we only read through the pointer. Two
        /// independent RE sources agree on the layout:
        /// - nvapi64_impl handler 0x180072470 (MSI path): eventType (0/1) at
        ///   +24, status byte +28, flags +32; eventType 1 carries a ~9KB
        ///   per-point V/F payload starting at +0x6C.
    /// - PNY VelocityX `NVpower_wrapper.dll` (RE'd 2026-08-25, live
    ///   instruction-verified): the wrapper's callback reads state dword at
    ///   +0x48 (0 = idle, 1 = scanning, other = failed/finished), progress
    ///   at +0x50 (byte + dword mirror), and dwords at +0x60/+0x64 (last
    ///   dword is the HRESULT-ish code returned from the callback).
        pub struct NV_GPU_OC_SCANNER_STATUS_V1 {
            pub pad0: Padding<[u8; 0x48]>,
            /// +0x48: scanner state (VelocityX mapping: 0 = idle, 1 =
            /// scanning, other = failed/finished).
            pub state: u32,
            pub gap: Padding<[u8; 4]>,
            /// +0x50: progress (byte mirror + dword).
            pub progress: u32,
            pub pad1: Padding<[u8; 0x0C]>,
            /// +0x60: unknown status dword.
            pub status_0x60: u32,
            /// +0x64: unknown status dword (returned as the callback result
            /// by the VelocityX handler).
            pub status_0x64: u32,
            pub pad2: Padding<[u8; 4]>,
            /// +0x6C: per-point V/F payload on eventType-1 notifications
            /// (~9KB), opaque here.
            pub payload: Array<[u8; 0x2400]>,
        }
    }

    impl NV_GPU_OC_SCANNER_STATUS_V1 {
        /// VelocityX's derived 3-state mapping from the raw +0x48 dword:
        /// 0 → 0 (idle), 1 → 1 (scanning), other → 2 (failed/finished).
        pub fn scan_state(&self) -> u32 {
            match self.state {
                0 => 0,
                1 => 1,
                _ => 2,
            }
        }
    }

    pub type NV_GPU_OC_SCANNER_STATUS = NV_GPU_OC_SCANNER_STATUS_V1;

    nvstruct! {
        /// OC Scanner status-update registration — V1-EX variant (RE'd from
        /// PNY VelocityX `NVpower_wrapper.dll` Subscribe/Unsubscribe exports,
        /// 2026-08-25). 216 bytes, version magic 0x100D8 (v1|216B — the
        /// newer sibling of the MSI 0x10098/152B layout above). The callback
        /// fn pointer sits at +0x50; UNREGISTER = the same call with a NULL
        /// callback. The callback receives (ctx, pStatus: *const
        /// NV_GPU_OC_SCANNER_STATUS) and returns u32.
        #[nv_unchecked]
        pub struct NV_GPU_OC_SCANNER_STATUS_UPDATE_PARM_V1EX {
            pub version: NvVersion,
            pub pad0: Padding<[u8; 0x4C]>,
            /// +0x50: status callback (NULL = unregister).
            pub callback: Option<NV_OC_SCANNER_STATUS_CALLBACK>,
            pub tail: Array<[u8; 216 - 0x58]>,
        }
    }

    nvversion! { NV_GPU_OC_SCANNER_STATUS_UPDATE_PARM_V1EX(1) = 216 }

    nvapi! {
        /// Undocumented (NDA, ID 0xBC4AEE25). Start the DRIVER-side OC
        /// scanner (drivers >= 455.00). 68-byte control struct, magic
        /// 0x10044. Progress arrives via the RegisterForOcScannerStatusUpdates
        /// callback. The legacy path (NVIDIA's scanner.dll) is only used on
        /// pre-455 drivers or when forced.
        pub unsafe fn NvAPI_GPU_ClientStartOcScanner(hPhysicalGPU: NvPhysicalGpuHandle, pScanner: *mut NV_GPU_OC_SCANNER_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Undocumented (NDA, ID 0xC28B73DE). Stop the driver-side OC
        /// scanner. Same 68-byte control struct as the start call.
        pub unsafe fn NvAPI_GPU_ClientStopOcScanner(hPhysicalGPU: NvPhysicalGpuHandle, pScanner: *mut NV_GPU_OC_SCANNER_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Undocumented (NDA, ID 0xCC727B22). Revert the OC applied by the
        /// driver-side scanner (back to the pre-scan curve). Same 68-byte
        /// control struct.
        pub unsafe fn NvAPI_GPU_ClientRevertOc(hPhysicalGPU: NvPhysicalGpuHandle, pRevert: *mut NV_GPU_OC_SCANNER_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Undocumented (NDA, ID 0x1CB41116). Register a status callback
        /// for the driver-side OC scanner. 152-byte struct, magic 0x10098,
        /// callback fn pointer at +0x78 (cookie at +0x30, validity at +0x50).
        pub unsafe fn NvAPI_GPU_ClientRegisterForOcScannerStatusUpdates(hPhysicalGPU: NvPhysicalGpuHandle, pRegister: *mut NV_GPU_OC_SCANNER_STATUS_UPDATE_PARM) -> NvAPI_Status;
    }

    nvapi! {
        /// Undocumented (NDA, ID 0x593E8E72). Query the last OC scanner
        /// run status. Uses the SAME 68-byte control struct as Start
        /// (magic 0x10044). Per IDA (nvapi64_impl handler 0x180071B80):
        /// this is a STATUS-ONLY call — it returns an NVAPI status code
        /// describing the scanner state (OK = idle/has-result, busy/timeout
        /// = scanning, etc.) but does NOT write per-point results into the
        /// struct. Per-point result data flows through the Register callback
        /// (eventType 1, ~9KB payload) or the internal selector-2002 RPC.
        pub unsafe fn NvAPI_GPU_ClientGetLastOcScannerResults(hPhysicalGPU: NvPhysicalGpuHandle, pScanner: *mut NV_GPU_OC_SCANNER_CONTROL) -> NvAPI_Status;
    }

    nvstruct! {
        /// Background-scanner enable struct (RE'd R610.74 @0x1800717C0:
        /// 72B, magic 0x10048 — one step above the 0x10044 control family).
        /// Enable flag byte @+4; a 9-byte feature GUID @+10..21 =
        /// 0B 0A 0E 08 E8 72 9D D9 F3 (checked by the RPC, cmd id 7).
        pub struct NV_GPU_OC_BACKGROUND_SCANNER_CONTROL_V1 {
            pub version: NvVersion,
            pub enable: u8,
            pub pad_05: Padding<[u8; 5]>,
            pub feature_guid: [u8; 9],
            pub pad_1a: Padding<[u8; 53]>,
        }
    }

    nvversion! { @=NV_GPU_OC_BACKGROUND_SCANNER_CONTROL NV_GPU_OC_BACKGROUND_SCANNER_CONTROL_V1(1) = 72 }

    nvapi! {
        /// Undocumented (NDA, ID 0x06DC7CE8, @0x1800717C0). Enable the
        /// background OC scanner. 72-byte struct, magic 0x10048; reads the
        /// enable byte @+4 and validates the feature GUID @+10..21.
        pub unsafe fn NvAPI_GPU_ClientEnableBackgroundOcScanner(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *mut NV_GPU_OC_BACKGROUND_SCANNER_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Undocumented (NDA, ID 0xBE371D0A, @0x180073550). Query the last
        /// INCOMPLETE OC-scanner run's partial results. Same 68-byte control
        /// struct as GetLast (magic 0x10044); RPC cmd 13 (2→-104, 4→-191).
        pub unsafe fn NvAPI_GPU_GetLastIncompleteOcScannerResults(hPhysicalGPU: NvPhysicalGpuHandle, pScanner: *mut NV_GPU_OC_SCANNER_CONTROL) -> NvAPI_Status;
    }

    // ------------------------------------------------------------------
    // PerfPstatesGetInfoPrivate (NDA, ID 0x7B30AE0D) — the P-State level
    // table behind the ref tool's `-pstate` GET ("Level[N] P*.Max/P*.Min").
    //
    // RE'd from the ref tool `[GPUHandle::queryPStateInfo]` (thunk sub_140003A20).
    // Returns a 275152-byte struct with version magic 0x432D0 (v4 | size).
    // Decoded layout (byte offsets from the version dword at byte 0):
    //   valid-pstate bitmask ... dword 34 (byte 0x88), bit i set ⇔ P{i} exists
    //   table version       ... dword 35 low byte (byte 0x8C)
    //   slot table          ... base byte 0x2114, stride 0x2090; one entry per
    //                          present pstate, holding that pstate's NUMBER
    //                          (the slot order tracks the bitmask scan, NOT the
    //                          pstate number directly)
    //   freq table          ... indexed BY pstate number (0..31), stride 0x9C;
    //                          min_kHz @ 0x22C8, max_kHz @ 0x22F0 per pstate
    // Everything else is opaque. The decoded view (present pstates with their
    // min/max clocks) is built by the accessors below; the slot table is only
    // needed to enumerate WHICH pstates are present in driver order, but the
    // bitmask already encodes that, so we drive off the bitmask + freq table.
    // ------------------------------------------------------------------

    /// Max P-State index the struct reserves room for (bitmask is 32 bits).
    pub const NV_GPU_PERF_PSTATES_MAX: usize = 32;

    nvstruct! {
        /// Perf P-states info (RE'd from the ref tool; NDA). Opaque except for the
        /// bitmask/version header and the decoded accessors below.
        pub struct NV_GPU_PERF_PSTATES_INFO_PRIVATE_V4 {
            pub version: NvVersion,
            /// dwords 1..34 (opaque header). Bytes 4..0x88.
            pub hdr: Array<[u32; 33]>,
            /// Byte 0x88 (dword 34) = bitmask of present pstates (bit i ⇔ P{i}).
            pub pstate_mask: u32,
            /// Byte 0x8C (dword 35) low byte = table version (logged by the ref tool).
            pub table_version: u8,
            pub rsvd0: Padding<[u8; 3]>,
            /// Bytes 0x90..(then the slot + freq tables). Header above = 144 B.
            /// Total struct = 275152 B (the ref tool's memset clears 0x432CC bytes from
            /// v19[1], i.e. struct = 4 + 0x432CC = 0x432D0 = 275152; the version
            /// magic with_struct(4) yields exactly 0x432D0).
            pub payload: Array<[u8; 275152 - 144]>,
        }
    }

    impl NV_GPU_PERF_PSTATES_INFO_PRIVATE_V4 {
        // Freq table layout (RE'd from the ref tool queryPStateInfo loop):
        //   max_kHz byte offset = 0x22F0 + slot*0x2090 + domain*0x9C
        //   min_kHz byte offset = 0x22C8 + slot*0x2090 + domain*0x9C
        // where:
        //   - `slot` = the k-th set bit in `pstate_mask` (one slot per present
        //     pstate, in ascending bit order). NOT the pstate NUMBER — each slot
        //     is 0x2090 (8336) bytes apart.
        //   - `domain` = clock-domain index (0=GPC/core typically; the ref tool
        //     resolves it via the separate 0x57B5A5DF queryClockDomainInfo). Each
        //     domain is 0x9C (156) bytes apart — so the 4-dimensional view a
        //     P-State exposes (core max/min, memory, ...) is just domain 0..N.
        // A first pass wrongly used `pstate_number * 0x9C`, reading the wrong
        // domain at the wrong slot and producing implausible clocks.
        const FREQ_MIN_BASE: usize = 0x22C8;
        const FREQ_MAX_BASE: usize = 0x22F0;
        const SLOT_STRIDE: usize = 0x2090;
        const DOMAIN_STRIDE: usize = 0x9C;
        /// Slot table base (one real pstate number per set bitmask bit), stride
        /// 0x2090 bytes per slot. Slot k holds the REAL pstate number for the
        /// k-th set bit in `pstate_mask` — the bitmask bit position is NOT the
        /// pstate number (e.g. a GPU with P0/P3/P4/P5/P8 has bits 0,3,4,5,8 set
        /// but slot 0..4 hold pstate numbers 0,3,4,5,8 respectively).
        const SLOT_BASE: usize = 0x2114;

        fn payload_dword(&self, byte_off: usize) -> Option<u32> {
            // The typed header occupies the first 144 bytes; offset into the
            // payload by subtracting that.
            let off = byte_off.checked_sub(144)?;
            self.payload
                .get(off..off.checked_add(4)?)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        }

        /// Table version byte (the ref tool logs this as "P state version: 0x%X").
        pub fn table_version(&self) -> u8 {
            self.table_version
        }

        /// Min clock (kHz) for the given slot + clock-domain, if in bounds.
        fn min_khz_slot(&self, slot: usize, domain: usize) -> Option<u32> {
            let off = Self::FREQ_MIN_BASE
                .checked_add(slot * Self::SLOT_STRIDE)?
                .checked_add(domain * Self::DOMAIN_STRIDE)?;
            self.payload_dword(off)
        }

        /// Max clock (kHz) for the given slot + clock-domain, if in bounds.
        fn max_khz_slot(&self, slot: usize, domain: usize) -> Option<u32> {
            let off = Self::FREQ_MAX_BASE
                .checked_add(slot * Self::SLOT_STRIDE)?
                .checked_add(domain * Self::DOMAIN_STRIDE)?;
            self.payload_dword(off)
        }

        /// The decoded P-State entries: one per set bitmask bit, each carrying
        /// its REAL pstate number (read from the slot table) plus min/max clock
        /// in kHz for the given clock-domain. `domain` selects which dimension
        /// (0=GPC/core by default; the ref tool resolves it via 0x57B5A5DF).
        /// Mirrors the ref tool's queryPStateInfo loop.
        pub fn pstate_entries_domain(&self, domain: usize) -> Vec<PStateEntryRaw> {
            let mut out = Vec::new();
            for bit in 0u32..32 {
                if (self.pstate_mask >> bit) & 1 == 0 {
                    continue;
                }
                // Slot index = number of set bits already emitted (the ref tool's v10
                // counter, one slot per set bit, in ascending bit order).
                let slot = out.len();
                let pstate = self
                    .payload_dword(Self::SLOT_BASE + slot * Self::SLOT_STRIDE)
                    .map(|v| v as u8)
                    .unwrap_or(bit as u8);
                out.push(PStateEntryRaw {
                    pstate,
                    min_khz: self.min_khz_slot(slot, domain),
                    max_khz: self.max_khz_slot(slot, domain),
                });
            }
            out
        }

        /// Convenience: P-State entries for the default clock domain (0 = GPC /
        /// core). Same as [`pstate_entries_domain`](Self::pstate_entries_domain(0)).
        pub fn pstate_entries(&self) -> Vec<PStateEntryRaw> {
            self.pstate_entries_domain(0)
        }
    }

    /// Raw decoded P-State entry (kHz), before ergonomic conversion.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PStateEntryRaw {
        pub pstate: u8,
        pub min_khz: Option<u32>,
        pub max_khz: Option<u32>,
    }

    nvversion! { @=NV_GPU_PERF_PSTATES_INFO_PRIVATE NV_GPU_PERF_PSTATES_INFO_PRIVATE_V4(4) = 275152 }

    nvapi! {
        /// Undocumented (NDA, ID 0x7B30AE0D). Private PerfPstatesGetInfo — the
        /// P-State level table (present pstates + per-pstate min/max core clock
        /// in kHz). Source of the ref tool's `-pstate` GET listing. Returns a
        /// 275152-byte struct with version magic 0x432D0 (version 4).
        pub unsafe fn NvAPI_GPU_PerfPstatesGetInfoPrivate(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_PERF_PSTATES_INFO_PRIVATE) -> NvAPI_Status;
    }

    // ------------------------------------------------------------------
    // PerfPstatesGetInfoPrivate LEGACY fallback layouts (pre-V4 drivers).
    //
    // IDA nvapi64.dll 538.78, handler sub_1802E4570 (IID 0x7B30AE0D, escape
    // 0x07000048): the version check accepts EXACTLY three caller magic
    // dwords — 0x379C8 (native V3, 227784 B), 0x31A38 (V3, 203208 B) and
    // 0x119C8 (V1, 72136 B); anything else (incl. the V4 0x432D0/0x832D0
    // the R610-era ref tool and our V4 path send) → -9
    // INCOMPATIBLE_STRUCT_VERSION. So V4 is R610-only; older drivers speak
    // the legacy layouts and the caller degrades V4 → V3 → V1.
    //
    // The magic dword IS the raw struct size (high nibble = version,
    // 0x119C8 = 72136 …) — nvversion's `size | version<<16` encoding cannot
    // express that folding, so the magic is written raw into the buffer
    // (same pattern as ClientPStateLimitStatus's 0x10088 above).
    //
    // Shared record layout (from the driver's own marshal loops into both
    // legacy views): present bitmask @ +4, table version byte @ +8, then
    // one record per set mask bit at byte 72 + 2252*bit:
    //   +0  u32 clock-domain type (semantics live; V1 re-indexes by pstate
    //       number — multi-domain slots overwrite — V3 keeps every slot,
    //       so prefer V3)
    //   +4  u32 min_kHz
    //   +8  u32 max_kHz (bit0 = driver flag, masked off)
    //   +12 u8  pstate number
    // (V1 aggregates mask bits BY pstate number — bit p ⇔ P{p} present;
    //  V3's mask is the raw slot mask and the pstate number rides in each
    //  record. Iterating set bits and reading record.pstate decodes BOTH.)
    // ------------------------------------------------------------------

    /// V3 legacy version magic (= struct size 203208, high nibble = v3).
    pub const PERF_PSTATES_INFO_PRIVATE_V3_LEGACY_MAGIC: u32 = 0x31A38;
    /// V3 legacy buffer size (bytes).
    pub const PERF_PSTATES_INFO_PRIVATE_V3_LEGACY_LEN: usize = 203208;
    /// V1 legacy version magic (= struct size 72136, high nibble = v1).
    pub const PERF_PSTATES_INFO_PRIVATE_V1_LEGACY_MAGIC: u32 = 0x119C8;
    /// V1 legacy buffer size (bytes).
    pub const PERF_PSTATES_INFO_PRIVATE_V1_LEGACY_LEN: usize = 72136;

    /// Present-pstate bitmask in a legacy PerfPstatesGetInfoPrivate buffer.
    pub fn perf_pstates_legacy_mask(buf: &[u8]) -> u32 {
        u32::from_ne_bytes(buf[4..8].try_into().expect("4 bytes"))
    }

    /// Decode the legacy per-pstate record for mask bit `bit`:
    /// `(type, min_khz, max_khz, pstate)`; max bit0 (driver flag) masked off.
    pub fn perf_pstates_legacy_record(buf: &[u8], bit: u32) -> (u32, u32, u32, u8) {
        let base = 72 + 2252 * bit as usize;
        let dw = |o: usize| u32::from_ne_bytes(buf[o..o + 4].try_into().expect("4 bytes"));
        (dw(base), dw(base + 4), dw(base + 8) & !1, buf[base + 12])
    }

    /// Legacy V1 record SUB-TABLE (live V100 decode 2026-09-02): the record
    /// body packs one 68-byte clock-snapshot entry per ClkDomains record
    /// bit, GPC (bit 0) FIRST, in bit order: `+8 nominal_kHz, +12
    /// live/min_kHz, +16 max_kHz, +40 chained TYPE of the NEXT bit's
    /// record`. `0xFFFFFFFF` at +40 = absent domain (V100 bit9 Host).
    /// Anchors: entry0 +12 == the live SM clock (135 MHz idle, nvidia-smi
    /// same-instant), entry2 == MEM 877×3; the +40 sequence 5/4/5/5/5/2/4/2/2
    /// equals the get-private-freq-domain-info Type sequence of bits 1..9
    /// exactly.
    ///
    /// Returns `(nominal_khz, live_min_khz, max_khz)` for `domain_bit` of
    /// record `bit`, or `None` when the entry is absent/out of range.
    /// NOTE the values are domain-appropriate, not uniformly kHz: on V100
    /// the Pclk0 entry (bit 8) carries the PCIe GEN LEVEL (3 = Gen3, user
    /// confirmed — same "gen count" semantics as GetAllClocks domain 31),
    /// not a clock. Only the kHz clock domains (Gpc/Xbar/Mem/Sys/M/Msd…)
    /// should be consumed as frequencies.
    pub fn perf_pstates_legacy_domain_clock(
        buf: &[u8],
        record_bit: u32,
        domain_bit: usize,
    ) -> Option<(u32, u32, u32)> {
        let base = 72 + 2252 * record_bit as usize + 72 + 68 * domain_bit;
        if base + 44 > buf.len() {
            return None;
        }
        let dw = |o: usize| u32::from_ne_bytes(buf[o..o + 4].try_into().expect("4 bytes"));
        let tail_type = dw(base + 40);
        if tail_type == 0 || tail_type == u32::MAX {
            return None; // absent domain / padding
        }
        Some((dw(base + 8), dw(base + 12), dw(base + 16)))
    }

    // ------------------------------------------------------------------
    // ClientPStateLimitStatus (NDA, ID 0x9962C97C) — the "which P-States are
    // currently locked" view. RE'd from the ref tool's `[GPUHandle::pollPState]`
    // "get p state limit" branch (thunk sub_140003D60). the ref tool allocates a
    // 164-byte buffer but the driver's version magic 0x10088 reports size 136
    // (v1) — the tail is padding. Entries start at byte 8, each 2 bytes
    // {type:u8, pstate:u8}; type == 0x1A marks a pstate locked by
    // PerfClientLimitsSetStatus (0x39442CFB). the ref tool renders the locked set as
    // "P0.P3.P5".
    // ------------------------------------------------------------------

    nvstruct! {
        /// P-State limit-status (RE'd from the ref tool; NDA). Opaque except for the
        /// count + entry table decoded by the accessor below.
        pub struct NV_GPU_CLIENT_PSTATE_LIMIT_STATUS_V1 {
            pub version: NvVersion,
            /// Number of valid entries in `entries`.
            pub count: u32,
            /// Entry table: count × {type:u8, pstate:u8}, type==0x1A = locked.
            /// 164-byte buffer total (driver magic reports 136; tail is pad).
            pub entries: Array<[u8; 164 - 8]>,
        }
    }

    impl NV_GPU_CLIENT_PSTATE_LIMIT_STATUS_V1 {
        /// The set of P-State numbers currently locked, in entry order. Each
        /// entry is `{type:u8, pstate:u8}`; the ref tool's pollPState only renders
        /// type==0x1A, but on current drivers the locked entries carry other
        /// type codes (e.g. 0x7B/0x7E for a P0 max/min lock) — so we treat
        /// EVERY entry as a locked pstate (count is authoritative). Empty when
        /// nothing is locked (the cleared state).
        pub fn locked_pstates(&self) -> Vec<u8> {
            let n = (self.count as usize).min(self.entries.len() / 2);
            (0..n).map(|i| self.entries[i * 2 + 1]).collect()
        }
    }

    nvversion! { @=NV_GPU_CLIENT_PSTATE_LIMIT_STATUS NV_GPU_CLIENT_PSTATE_LIMIT_STATUS_V1(1) = 164 }

    nvapi! {
        /// Undocumented (NDA, ID 0x9962C97C). Returns the set of P-States
        /// currently locked via PerfClientLimitsSetStatus (0x39442CFB). The
        /// lightweight counterpart to the full PerfClientLimits status
        /// (0xE440B867, 780B). 164-byte struct, version magic 0x10088 (v1).
        pub unsafe fn NvAPI_GPU_ClientPStateLimitStatus(hPhysicalGPU: NvPhysicalGpuHandle, pStatus: *mut NV_GPU_CLIENT_PSTATE_LIMIT_STATUS) -> NvAPI_Status;
    }

    // ------------------------------------------------------------------
    // Rated-TDP control (NDA, ID 0xC9E9BB33). RE'd from the ref tool's
    // `[GPUHandle::clearRatedTdp]`/`[GPUHandle::setRatedTdp]` (the setPState
    // preamble + cmdPState index==0 path). 12-byte struct
    // {version: 0x1000C, dword1: 1, mode}: mode=0 clear, mode=3 enable rated
    // TDP (the "P0.TDP" level). NOT a P-State lock despite an earlier mislabel.
    // ------------------------------------------------------------------

    nvstruct! {
        pub struct NV_GPU_RATED_TDP_CONTROL_V1 {
            pub version: NvVersion,
            pub flags: u32,
            /// 0 = clear/disable, 3 = enable rated TDP.
            pub mode: u32,
        }
    }

    nvversion! { @=NV_GPU_RATED_TDP_CONTROL NV_GPU_RATED_TDP_CONTROL_V1(1) = 12 }

    nvapi! {
        /// Undocumented (NDA, ID 0xC9E9BB33). Rated-TDP control. 12-byte struct,
        /// version magic 0x1000C (v1). the ref tool calls this (mode 0) before every
        /// P-State/frequency lock via 0x39442CFB.
        pub unsafe fn NvAPI_GPU_ClientRatedTdpControl(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *const NV_GPU_RATED_TDP_CONTROL) -> NvAPI_Status;
    }

    // ------------------------------------------------------------------
    // Rated-TDP GET trio (RE'd R610.74; RM cmd 0x7000048, 0x81868 work
    // buffer, hGpu @buf+0x30, sub-cmd @buf+0x34). Readback halves of the
    // SET above.
    // ------------------------------------------------------------------

    nvstruct! {
        /// GetStatus output (36B, magic 0x10024). Fill order from the
        /// workbuf: +4 u32 (buf+0x38), +8 u8 (buf+0x3C), +12 u32 decoded
        /// (buf+0x40), then five mode dwords from the buf+0x48 array into
        /// +16, +32, +20, +24, +28 (each mapped 0-4).
        pub struct NV_GPU_RATED_TDP_STATUS_V1 {
            pub version: NvVersion,
            pub dword_04: u32,
            pub byte_08: u8,
            pub pad_09: Padding<[u8; 3]>,
            pub dword_0c: u32,
            pub mode_0: u32,
            pub mode_1: u32,
            pub mode_2: u32,
            pub mode_3: u32,
            pub mode_4: u32,
        }
    }

    nvversion! { @=NV_GPU_RATED_TDP_STATUS NV_GPU_RATED_TDP_STATUS_V1(1) = 36 }

    nvstruct! {
        /// GetInfo output (8B, magic 0x10008): single byte of capability.
        pub struct NV_GPU_RATED_TDP_INFO_V1 {
            pub version: NvVersion,
            pub capabilities: u8,
            pub pad: Padding<[u8; 3]>,
        }
    }

    nvversion! { @=NV_GPU_RATED_TDP_INFO NV_GPU_RATED_TDP_INFO_V1(1) = 8 }

    nvapi! {
        /// Rated-TDP control GET (0xED2BEA09 @0x1802A90F0): reuses the SET
        /// struct (12B, magic 0x1000C) — reads the mode dword @+4, fills the
        /// current mode @+8. Sub-cmd 0x207E004E.
        pub unsafe fn NvAPI_GPU_PerfRatedTdpGetControl(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *mut NV_GPU_RATED_TDP_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Rated-TDP info (0x87BD35EF @0x1802A93D0): 8B struct, magic
        /// 0x10008, fills one capability byte. Sub-cmd 0x207F000C.
        pub unsafe fn NvAPI_GPU_PerfRatedTdpGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_RATED_TDP_INFO) -> NvAPI_Status;
    }

    nvapi! {
        /// Rated-TDP status (0xFCBDF642 @0x1802A96A0): 36B struct, magic
        /// 0x10024. Sub-cmd 0x207F000D.
        pub unsafe fn NvAPI_GPU_PerfRatedTdpGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pStatus: *mut NV_GPU_RATED_TDP_STATUS) -> NvAPI_Status;
    }

    // ------------------------------------------------------------------
    // Blackwell XBar ClockClient clock-domain family
    // (reverse/melonvolt/xbar.txt — Loong0x00 LACT #1147).
    //
    // Wraps the 4 NV2080 RM commands the article drives on Linux via
    // /dev/nvidiactl NV20_SUBDEVICE_0:
    //   CLK_CLK_DOMAINS_GET_INFO (0x20809019)  → NvAPI_GPU_ClockClkDomainsGetInfo
    //   GET_CONTROL            (0x2080901b)  → NvAPI_GPU_ClockClkDomainsGetControl
    //   SET_CONTROL            (0x2080d01c)  → NvAPI_GPU_ClockClkDomainsSetControl
    //   CLK_MEASURE_FREQ        (0x20809006)  → NvAPI_GPU_ClockCounterMeasureAvgFreq
    // IDA-confirmed: each impl handler (nvapi64_impl_live.dll R575.74) writes the
    // article's exact RM cmd id into v6[13] and escapes via 0x07000109
    // (sub_180389320/4A0 — same 0x0700_01xx private family as VoltRails 0x07000191).
    // All 4 QI-resolve non-NULL; 3 GET paths live-verified on Ada 4060 Laptop.
    //
    // GetControl V1 (magic 0x10964) layout (IDA + live dump):
    //   +0  NvVersion magic      +8  controllable_mask (u32)
    //   +12..+99 opaque header (bytes/dwords)
    //   +100 32×72B per-domain records, BIT-SPARSE (record for domain bit N
    //        at +100+72*N). Each record: type u8 @+0 (live 0x0A), then 5 u32
    //        @+44..+60: offset_kHz(i32), range_min, range_max, applied, extra.
    // Live mask 0x000000FF = GPC(bit0)|XBAR(bit1)|SYS(bit2)|MCLK(bit4) —
    // XBARCLK IS controllable on Ada 4060 Laptop, NOT Blackwell-only.
    //
    // MeasureFreq V1 (magic 0x10020): +8 cycle_counter (u32, read-modify-write,
    // NOT direct kHz), +16 timestamp_ns (u64 QPC). Windows returns raw
    // {counter,timestamp}; sample twice and compute freq = Δcounter/Δt_ns × 1e9.
    // ------------------------------------------------------------------

    /// Byte offsets into the bit-sparse per-domain records of
    /// [`NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL_V1`] (all ABSOLUTE struct
    /// offsets; `rest` begins at +4, so a `rest`-relative index is `abs - 4`).
    pub mod clk_ctrl_entry {
        /// controllable domain mask (u32) absolute offset
        pub const MASK: usize = 8;
        /// first per-domain record base (absolute)
        pub const BASE: usize = 100;
        /// per-domain record stride
        pub const STRIDE: usize = 72;
        /// record+0: u8 type discriminator (live 0x0A=10)
        pub const TYPE: usize = 0;
        /// record+44: signed kHz offset (i32)
        pub const OFFSET_KHZ: usize = 44;
        /// record+48: range minimum (i32 kHz)
        pub const RANGE_MIN: usize = 48;
        /// record+52: range maximum (i32 kHz)
        pub const RANGE_MAX: usize = 52;
        /// record+56: applied value (i32 kHz)
        pub const APPLIED: usize = 56;
    }

    nvstruct! {
        /// Opaque versioned control block for the private ClockClient
        /// GetControl/SetControl (RM 0x2080901b / 0x2080d01c). Layout beyond
        /// the version + mask is driver-firmware-interpreted; accessors use the
        /// [`clk_ctrl_entry`] byte offsets. Total 0x964 = 2404 bytes.
        pub struct NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL_V1 {
            pub version: NvVersion,
            /// +4 .. +2404: mask@+8, header, 32×72B records @+100
            pub rest: [u8; 2400],
        }
    }

    nvversion! { @=NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL_V1(1) = 0x964 }

    impl NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL {
        /// Controllable-domain bitmask (u32 @+8). This is BOTH the input mask
        /// (which domains the caller asks the driver to fill records for) and
        /// the echoed output. Seed it with a broad mask before GET_CONTROL so
        /// the driver populates every controllable record; derive the TRUE
        /// controllable mask from [record_type] != 0 rather than trusting this
        /// echo (the driver echoes the seed, not the real controllable set).
        pub fn mask(&self) -> u32 {
            let off = clk_ctrl_entry::MASK - 4;
            u32::from_le_bytes(self.rest[off..off + 4].try_into().unwrap_or([0; 4]))
        }

        /// Seed the input mask at +8 (call before GET_CONTROL).
        pub fn set_mask(&mut self, mask: u32) {
            let off = clk_ctrl_entry::MASK - 4;
            self.rest[off..off + 4].copy_from_slice(&mask.to_le_bytes());
        }

        /// Read a u32 record field for `bit` at absolute offset `field_off`.
        fn record_u32(&self, bit: u32, field_off: usize) -> Option<u32> {
            let abs = clk_ctrl_entry::BASE
                .checked_add((bit as usize).checked_mul(clk_ctrl_entry::STRIDE)?)?
                .checked_add(field_off)?;
            let off = abs.checked_sub(4)?;
            let end = off.checked_add(4)?;
            let raw = self.rest.get(off..end)?;
            Some(u32::from_le_bytes(raw.try_into().ok()?))
        }

        /// Write a u32 record field for `bit` at absolute offset `field_off`.
        fn set_record_u32(&mut self, bit: u32, field_off: usize, value: u32) -> Option<()> {
            let abs = clk_ctrl_entry::BASE
                .checked_add((bit as usize).checked_mul(clk_ctrl_entry::STRIDE)?)?
                .checked_add(field_off)?;
            let off = abs.checked_sub(4)?;
            let end = off.checked_add(4)?;
            let dst = self.rest.get_mut(off..end)?;
            dst.copy_from_slice(&value.to_le_bytes());
            Some(())
        }

        /// Record type byte (u8 @record+0) for domain `bit`.
        pub fn record_type(&self, bit: u32) -> Option<u8> {
            let abs = clk_ctrl_entry::BASE
                .checked_add((bit as usize).checked_mul(clk_ctrl_entry::STRIDE)?)?
                .checked_add(clk_ctrl_entry::TYPE)?;
            self.rest.get(abs - 4).copied()
        }

        /// Signed kHz offset (i32 @record+44) for domain `bit`.
        pub fn offset_khz(&self, bit: u32) -> Option<i32> {
            self.record_u32(bit, clk_ctrl_entry::OFFSET_KHZ)
                .map(|v| v as i32)
        }

        /// Range minimum (i32 @record+48) for domain `bit`.
        pub fn range_min(&self, bit: u32) -> Option<i32> {
            self.record_u32(bit, clk_ctrl_entry::RANGE_MIN)
                .map(|v| v as i32)
        }

        /// Range maximum (i32 @record+52) for domain `bit`.
        pub fn range_max(&self, bit: u32) -> Option<i32> {
            self.record_u32(bit, clk_ctrl_entry::RANGE_MAX)
                .map(|v| v as i32)
        }

        /// Applied value (i32 @record+56) for domain `bit`.
        pub fn applied(&self, bit: u32) -> Option<i32> {
            self.record_u32(bit, clk_ctrl_entry::APPLIED)
                .map(|v| v as i32)
        }

        /// Write the signed kHz offset (i32 @record+44) for domain `bit`.
        pub fn set_offset_khz(&mut self, bit: u32, offset_khz: i32) -> Option<()> {
            self.set_record_u32(bit, clk_ctrl_entry::OFFSET_KHZ, offset_khz as u32)
        }

        /// Iterate (bit, type, offset_kHz, range_min, range_max, applied) for
        /// every domain the driver actually filled a record for (record type
        /// != 0). This derives the TRUE controllable set from filled records
        /// rather than trusting the echoed +8 mask (which is just the seed).
        pub fn entries(&self) -> impl Iterator<Item = (u32, u8, i32, i32, i32, i32)> + '_ {
            let this = self;
            (0..32u32).filter_map(move |bit| {
                let typ = this.record_type(bit).filter(|&t| t != 0)?;
                let off = this.offset_khz(bit).unwrap_or(0);
                let rmin = this.range_min(bit).unwrap_or(0);
                let rmax = this.range_max(bit).unwrap_or(0);
                let appl = this.applied(bit).unwrap_or(0);
                Some((bit, typ, off, rmin, rmax, appl))
            })
        }

        /// The true controllable mask: OR of every bit whose record the driver
        /// filled (record type != 0). Differs from [mask] when the seed was
        /// broader than the real controllable set.
        pub fn controllable_mask(&self) -> u32 {
            let mut m = 0u32;
            for bit in 0..32u32 {
                if self.record_type(bit).filter(|&t| t != 0).is_some() {
                    m |= 1 << bit;
                }
            }
            m
        }
    }

    nvstruct! {
        /// Private ClockClient MEASURE_FREQ params (RM 0x20809006). The driver
        /// returns a raw {counter, timestamp} pair — NOT a direct frequency.
        /// Sample twice and compute freq = (c2-c1)/(t2-t1) × 1e9 Hz. Magic
        /// 0x10020; +4 is the sequential domain INDEX (GPC=0, XBAR=1, SYS=2,
        /// MCLK=4 — validated by sub_18017A680's idx→mask table).
        pub struct NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE_V1 {
            pub version: NvVersion,
            pub domain_index: u32,
            /// +8 read-modify-write cycle counter (grows by freq×Δt)
            pub counter: u32,
            pub rsvd: u32,
            /// +16 QPC nanosecond timestamp
            pub timestamp_ns: u64,
            pub rsvd2: u32,
            /// explicit tail padding (align 8: 28 -> 32 bytes)
            pub rsvd3: u32,
        }
    }

    nvversion! { @=NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE_V1(1) = 0x20 }

    nvstruct! {
        /// V2 of the MEASURE_FREQ params (magic 131104 = 0x20020). Same
        /// call, but the cycle counter is a u64 (IDA sub_18021DC90: output
        /// writes a qword at +8). Older GPUs (Pascal observed) reject the
        /// V1 measure for some domains — the V2 form is the fallback.
        pub struct NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE_V2 {
            pub version: NvVersion,
            /// sequential domain INDEX (GPC=0, XBAR=1, SYS=2, MCLK=4);
            /// the u64 counter output overwrites this slot's upper half on
            /// return (IDA sub_18021DC90 V2 arm writes a qword at +8)
            pub domain_index: u32,
            /// +8 read-modify-write cycle counter (u64 on V2)
            pub counter: u64,
            /// +16 QPC nanosecond timestamp
            pub timestamp_ns: u64,
            /// +24 extra dword out
            pub extra: u32,
            /// explicit tail padding (align 8: 28 -> 32 bytes)
            pub rsvd: u32,
        }
    }

    nvversion! { @=NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE2 NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE_V2(2) = 0x20 }

    nvstruct! {
        /// Direct (non-counter) single-domain clock-frequency read, the
        /// green-curve-main (aufkrawall, MIT) MEASURE path. ID 0x527FC458 —
        /// a DIFFERENT sub-family from `ClockCounterMeasureAvgFreq` (0xFB8F61EC,
        /// which returns a {counter, timestamp} pair needing two samples).
        /// This struct is the driver's direct answer: caller supplies the
        /// version word + sequential domain INDEX, driver writes `freq_khz` at
        /// +8. Magic 0x0001000C = (1<<16)|0xC; 12 bytes / 3 dwords. Domain
        /// index encoding matches the counter variant (XBAR=1, SYS=2; the
        /// entry→measure-domain map is 1→1, 3→2 per green-curve's empirical
        /// differential-write identification on RTX 5070 / 610.88). VIDEO
        /// (entry 4) has NO measure domain — verify it via exact control-block
        /// readback instead.
        ///
        /// LIVE-VERIFIED on RTX 4060 Laptop / R610: under GPU load, all four
        /// measurable domains (GPC/XBAR/SYS/MCLK) agree with the counter-based
        /// `0xFB8F61EC` within <2%, confirming both IDs read the same RM clock
        /// state through different sub-families. NOTE: at IDLE / aggressive
        /// GCOFF, GPC and XBAR return transient anomalously-low or zero kHz
        /// (gate cycling) while SYS/MCLK stay stable — this is real hardware
        /// state, not an API error. For post-offset verification, read under
        /// load (green-curve also measures XBAR in the apply path, which is a
        /// load scenario); the counter variant's 50 ms window can SMOOTH gate
        /// transients or return 0 when Δcounter=0 across the sample, so the
        /// direct form is the more robust single-sample read under load.
        pub struct NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE_FREQ_DIRECT_V1 {
            pub version: NvVersion,
            /// +4 sequential domain INDEX in (GPC=0, XBAR=1, SYS=2, MCLK=4)
            pub domain_index: u32,
            /// +8 OUT: measured frequency in kHz (0 on refused/unmeasurable,
            /// or transient GCOFF gate-cycling at idle for GPC/XBAR)
            pub freq_khz: u32,
        }
    }

    nvversion! { @=NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE_FREQ_DIRECT NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE_FREQ_DIRECT_V1(1) = 0xC }

    /// V3 batch MEASURE_FREQ (magic 196984 = 0x30038; IDA sub_18021DC90
    /// V3 arm + disasm @0x18021DF03). One RM round-trip measures MANY
    /// domains: header 16B (magic@+0, count u8@+11), then `count` packed
    /// 24B entries from +16. Per entry the counter/timestamp qwords are
    /// SEED inputs and new-value outputs (read-modify-write, same as the
    /// single-domain forms); `extra` is output-only.
    pub mod clk_measure_v3 {
        /// magic 0x30178 (196984 decimal) = version 3 | size 0x178 = 376B
        /// = 16B header + 24B × 15 entries — the driver's FIXED capacity.
        pub const MAGIC: u32 = 0x30178;
        /// count u8
        pub const COUNT: usize = 11;
        /// first 24B entry (absolute)
        pub const ENTRIES: usize = 16;
        /// per-entry stride
        pub const STRIDE: usize = 24;
        /// max entries the internal 0x98240 buffer accommodates (far above
        /// any domain count in practice)
        pub const MAX_ENTRIES: usize = 15;
        /// entry+0: domain index u8
        pub const DOMAIN: usize = 0;
        /// entry+4: extra dword OUT
        pub const EXTRA: usize = 4;
        /// entry+8: cycle counter u64 (seed in / new value out)
        pub const COUNTER: usize = 8;
        /// entry+16: QPC timestamp ns u64 (seed in / new value out)
        pub const TIMESTAMP: usize = 16;
    }

    nvstruct! {
        /// V3 batch MEASURE_FREQ params — see [`clk_measure_v3`].
        pub struct NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE_V3 {
            pub version: NvVersion,
            /// +4 .. +16: reserved (count byte lives at +11)
            pub header: [u8; 12],
            /// +16 .. +376: 15 packed 24B entries
            pub entries: [u8; 360],
        }
    }

    // NOTE: no `= size` assert here — the magic's 0x38 is the DRIVER's
    // baseline size (header + 1 entry); the actual struct is sized for 32
    // entries and the handler validates only the magic dword.
    nvversion! { @=NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE3 NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE_V3(3) = 0x178 }

    impl NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE_V3 {
        fn ent_off(&self, i: usize, field_off: usize, len: usize) -> Option<usize> {
            if i >= clk_measure_v3::MAX_ENTRIES {
                return None;
            }
            let off = clk_measure_v3::ENTRIES + clk_measure_v3::STRIDE * i + field_off - 4;
            let end = off.checked_add(len)?;
            if end <= self.entries.len() {
                Some(off)
            } else {
                None
            }
        }

        /// number of entries (u8 @+11)
        pub fn count(&self) -> u8 {
            // +11 absolute = header[7]
            self.header[clk_measure_v3::COUNT - 4]
        }

        /// Set the entry count (u8 @+11).
        pub fn set_count(&mut self, n: u8) {
            self.header[clk_measure_v3::COUNT - 4] = n;
        }

        /// Program entry `i`: domain index + counter/timestamp seeds.
        pub fn set_entry(
            &mut self,
            i: usize,
            domain: u32,
            counter: u64,
            timestamp_ns: u64,
        ) -> Option<()> {
            let d = self.ent_off(i, clk_measure_v3::DOMAIN, 1)?;
            self.entries[d] = domain as u8;
            let c = self.ent_off(i, clk_measure_v3::COUNTER, 8)?;
            self.entries[c..c + 8].copy_from_slice(&counter.to_le_bytes());
            let t = self.ent_off(i, clk_measure_v3::TIMESTAMP, 8)?;
            self.entries[t..t + 8].copy_from_slice(&timestamp_ns.to_le_bytes());
            Some(())
        }

        /// Read entry `i`'s returned {counter, timestamp, extra}.
        pub fn entry(&self, i: usize) -> Option<(u64, u64, u32)> {
            let c = self.ent_off(i, clk_measure_v3::COUNTER, 8)?;
            let counter = u64::from_le_bytes(self.entries[c..c + 8].try_into().ok()?);
            let t = self.ent_off(i, clk_measure_v3::TIMESTAMP, 8)?;
            let ts = u64::from_le_bytes(self.entries[t..t + 8].try_into().ok()?);
            let e = self.ent_off(i, clk_measure_v3::EXTRA, 4)?;
            let extra = u32::from_le_bytes(self.entries[e..e + 4].try_into().ok()?);
            Some((counter, ts, extra))
        }
    }

    /// Byte offsets into the bit-sparse per-domain records of
    /// [`NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL_V2`] (absolute struct
    /// offsets; `rest` begins at +4).
    ///
    /// V2 is the REAL read/write path for the record types modern drivers
    /// report (protocol 0x0A — internal 0x0B via the sub_18015BB30/BD20
    /// remap). The V1 handler's per-record switch only marshals internal
    /// types {2,4,5,6,7,8,9,0xA}; internal 0x0B (protocol 0x0A) and 0x10
    /// exist ONLY in the V2 switch — V1 silently drops those records (the
    /// type dword is still written on GET, the value dwords never are).
    ///
    /// IDA (sub_1802091B0 GET / sub_18020BDF0 SET, nvapi64_impl R610.74):
    /// records at +292+772*bit; type-0x0B records carry 8 value dwords at
    /// rec+268..+296 (GET copies internal dwords[32..36,41..43] there; SET
    /// copies the same 8 back). Verified live: 0xCC-prefill shows the driver
    /// zeroing +268..299 for type-0x0A records while +260..267 and the
    /// type-0x02 record stay untouched.
    pub mod clk_ctrl_entry_v2 {
        /// controllable domain mask (u32) absolute offset (seeded input)
        pub const MASK: usize = 8;
        /// first per-domain record base (absolute)
        pub const BASE: usize = 292;
        /// per-domain record stride
        pub const STRIDE: usize = 772;
        /// record+0: u32 type discriminator (low byte; live 0x0A)
        pub const TYPE: usize = 0;
        /// record+268: first of 8 value dwords (type-0x0A records)
        pub const VALUES: usize = 268;
        /// number of value dwords
        pub const VALUE_COUNT: usize = 8;
    }

    nvstruct! {
        /// V2 control block for the private ClockClient GetControl/SetControl.
        /// Magic 0x261A4 = version 2 | size 0x61A4 = 24996 bytes. NOTE: an
        /// earlier reverse-engineering pass mis-transcribed the magic as
        /// 0x26154 — the handler's `cmp eax, 261A4h` (0x180209354) is
        /// authoritative; 0x26154 returns -9.
        pub struct NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL_V2 {
            pub version: NvVersion,
            /// +4 .. +24996: mask@+8, header, 32×772B records @+292
            pub rest: [u8; 24992],
        }
    }

    nvversion! { @=NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL2 NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL_V2(2) = 0x61a4 }

    impl NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL_V2 {
        /// Seeded input mask (u32 @+8). GET_CONTROL reads it to decide which
        /// records to fill and echoes it back.
        pub fn mask(&self) -> u32 {
            let off = clk_ctrl_entry_v2::MASK - 4;
            u32::from_le_bytes(self.rest[off..off + 4].try_into().unwrap_or([0; 4]))
        }

        /// Seed the input mask at +8 (call before GET_CONTROL). The driver
        /// rejects u32::MAX; 0xFF is accepted.
        pub fn set_mask(&mut self, mask: u32) {
            let off = clk_ctrl_entry_v2::MASK - 4;
            self.rest[off..off + 4].copy_from_slice(&mask.to_le_bytes());
        }

        fn rec_off(&self, bit: u32, field_off: usize, len: usize) -> Option<usize> {
            let abs = clk_ctrl_entry_v2::BASE
                .checked_add((bit as usize).checked_mul(clk_ctrl_entry_v2::STRIDE)?)?
                .checked_add(field_off)?;
            let off = abs.checked_sub(4)?;
            let end = off.checked_add(len)?;
            if end <= self.rest.len() {
                Some(off)
            } else {
                None
            }
        }

        /// Record type low byte (u32 @rec+0) for domain `bit`.
        pub fn record_type(&self, bit: u32) -> Option<u8> {
            self.rec_off(bit, clk_ctrl_entry_v2::TYPE, 4)
                .and_then(|off| self.rest.get(off).copied())
        }

        /// Value dword `i` (0..8, at rec+268+4*i) for domain `bit`.
        pub fn value(&self, bit: u32, i: usize) -> Option<i32> {
            if i >= clk_ctrl_entry_v2::VALUE_COUNT {
                return None;
            }
            self.rec_off(bit, clk_ctrl_entry_v2::VALUES + 4 * i, 4)
                .and_then(|off| {
                    self.rest
                        .get(off..off + 4)
                        .and_then(|s| s.try_into().ok())
                        .map(u32::from_le_bytes)
                        .map(|v| v as i32)
                })
        }

        /// Write value dword `i` (0..8) for domain `bit`.
        pub fn set_value(&mut self, bit: u32, i: usize, v: i32) -> Option<()> {
            if i >= clk_ctrl_entry_v2::VALUE_COUNT {
                return None;
            }
            let off = self.rec_off(bit, clk_ctrl_entry_v2::VALUES + 4 * i, 4)?;
            self.rest[off..off + 4].copy_from_slice(&(v as u32).to_le_bytes());
            Some(())
        }

        /// The true controllable mask: OR of bits whose record the driver
        /// filled (record type != 0).
        pub fn controllable_mask(&self) -> u32 {
            let mut m = 0u32;
            for bit in 0..32u32 {
                if self.record_type(bit).filter(|&t| t != 0).is_some() {
                    m |= 1 << bit;
                }
            }
            m
        }
    }

    nvstruct! {
        /// Private ClockClient GET_INFO buffer (RM 0x20809019, the article's
        /// discovery API). Best-effort: rejects all 5 IDA magics live on
        /// R575.74 (-9 UNRESOLVED); discovery is routed through GetControl
        /// (which exposes the mask + per-domain ranges) instead. Total
        /// 0x9B8 = 2488 bytes; layout beyond the version opaque.
        pub struct NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_INFO_PRIVATE_V1 {
            pub version: NvVersion,
            pub rest: [u8; 2484],
        }
    }

    nvversion! { @=NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_INFO_PRIVATE NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_INFO_PRIVATE_V1(1) = 0x9b8 }

    nvapi! {
        /// Private ClockClient GET_INFO (RM 0x20809019). Best-effort on
        /// R575.74 (returns UNRESOLVED); GetControl supersedes it for
        /// discovery. ID 0x57B5A5DF.
        pub unsafe fn NvAPI_GPU_ClockClkDomainsGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_INFO_PRIVATE) -> NvAPI_Status;
    }

    nvapi! {
        /// Private ClockClient GET_CONTROL (RM 0x2080901b, ID 0xF58938F5).
        /// Returns the full controllable-domain block: mask + per-domain
        /// type/range/offset. WORKS live on Ada 4060 Laptop (magic 0x10964).
        pub unsafe fn NvAPI_GPU_ClockClkDomainsGetControl(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *mut NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Private ClockClient SET_CONTROL (RM 0x2080d01c, ID 0xD14B69CF).
        /// DANGEROUS GPU clock write. Always snapshot via GetControl first,
        /// version-gate (magic==0x10964), patch a COPY, SET, read back and
        /// verify, restore the snapshot on mismatch. See medium-layer
        /// `set_clk_domain_offset` for the mandated safety recipe.
        pub unsafe fn NvAPI_GPU_ClockClkDomainsSetControl(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *const NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Private ClockClient MEASURE_FREQ (RM 0x20809006, ID 0xFB8F61EC).
        /// Returns {counter, timestamp}; sample twice and divide for physical
        /// Hz. WORKS live on Ada 4060 Laptop.
        pub unsafe fn NvAPI_GPU_ClockCounterMeasureAvgFreq(hPhysicalGPU: NvPhysicalGpuHandle, pMeasure: *mut NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE) -> NvAPI_Status;
    }

    nvapi! {
        /// Direct clock-frequency read for one ClkDomains measure domain
        /// (ID 0x527FC458). A DIFFERENT, simpler sub-family than
        /// `ClockCounterMeasureAvgFreq` (0xFB8F61EC) above: the driver writes
        /// `freq_khz` directly at +8 — no two-sample Δcounter/Δt computation.
        /// green-curve-main uses this exclusively for XBar/SYS measurement and
        /// for verifying a ClkDomains offset took effect (XBAR=domain 1,
        /// SYS=domain 2). 12-byte V1 struct, magic 0x0001000C.
        pub unsafe fn NvAPI_GPU_ClockClkDomainsMeasureFreq(hPhysicalGPU: NvPhysicalGpuHandle, pMeasure: *mut NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE_FREQ_DIRECT) -> NvAPI_Status;
    }

    /// Byte offsets into the private ClockClient V/F-POINTS GetInfo block
    /// (ID 0x8895B510, RM 0x20809061 — the article's point-discovery API).
    ///
    /// IDA + live-verified on R610.74: the struct is a point DIRECTORY —
    /// a 2048-bit point mask per bank, then 2048 descriptors of 104B (0x68)
    /// per bank. Per-point descriptor: type via sub_1802021F0, rec+4=src[2],
    /// rec+5=0xFF, rec+0x28 = WORD (types 2,5,10,15) or DWORD (types
    /// 3,7,12,17) = src[4]. The mask bytes at +4.. are ALSO the seed the
    /// GetStatus header (+4..+132) must be pre-filled from.
    pub mod clk_vfp_info {
        /// bank-1 point mask dwords (64 dwords = 2048 bits), absolute
        pub const MASK1: usize = 4;
        /// bank-1 descriptors base (absolute), stride 104 × 2048
        pub const DESC1: usize = 772;
        /// per-point descriptor stride
        pub const DESC_STRIDE: usize = 104;
        /// bank-2 point mask dwords (absolute) — exactly DESC1 + 104*2048
        pub const MASK2: usize = 0x34304;
        /// bank-2 descriptors base (absolute)
        pub const DESC2: usize = 0x34604;
        /// points per bank
        pub const POINTS: usize = 2048;
    }

    nvstruct! {
        /// Private ClockClient V/F-POINTS GET_INFO (ID 0x8895B510). Magic
        /// 0x78604 = 493060 bytes. Returns the 2048-bit point masks + 104B
        /// descriptors for both banks; its +4.. output is the seed the
        /// GetStatus header requires. See [`clk_vfp_info`].
        pub struct NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1 {
            pub version: NvVersion,
            /// +4 .. +493060: masks + 2×2048 descriptors
            pub rest: [u8; 493056],
        }
    }

    // NOTE: unlike the sizeof-derived `nvversion!` magics, the V/F-points
    // family's magic dwords are NOT `version<<16 | sizeof` (0x78604 and
    // 0x1E8604 both exceed 16 size bits — the driver's own "size" field is
    // just 0x8604). Stamp the raw literal the IDA handlers compare against.
    pub type NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE =
        NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1;

    impl NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1 {
        /// Literal magic dword the GetInfo handler accepts (live-verified).
        pub const MAGIC: u32 = 0x78604;
        /// Legacy magic (R391.35/Kepler-Fermi): the GetInfo handler on old
        /// drivers only accepts this small-table stamp (83996B) and rejects
        /// the R610 0x78604 with IncompatibleStructVersion. Live-verified on
        /// GT730/391.35: status=0 (escape succeeds) where 0x78604 → -9.
        pub const MAGIC_LEGACY: u32 = 83996; // 0x1481C
    }

    impl Default for NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1 {
        fn default() -> Self {
            Self {
                version: NvVersion::with_version(Self::MAGIC),
                rest: [0; 493056],
            }
        }
    }

    impl NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1 {
        fn off(&self, abs: usize, len: usize) -> Option<usize> {
            let off = abs.checked_sub(4)?;
            let end = off.checked_add(len)?;
            if end <= self.rest.len() {
                Some(off)
            } else {
                None
            }
        }

        fn u32_at(&self, abs: usize) -> Option<u32> {
            let off = self.off(abs, 4)?;
            self.rest
                .get(off..off + 4)
                .and_then(|s| s.try_into().ok())
                .map(u32::from_le_bytes)
        }

        /// Is point `idx` (0..2048) present in bank `bank` (0 or 1)?
        pub fn point_present(&self, bank: usize, idx: usize) -> Option<bool> {
            if bank > 1 || idx >= clk_vfp_info::POINTS {
                return None;
            }
            let mask_base = if bank == 0 {
                clk_vfp_info::MASK1
            } else {
                clk_vfp_info::MASK2
            };
            let dword = self.u32_at(mask_base + 4 * (idx >> 5))?;
            Some(dword & (1 << (idx & 31)) != 0)
        }

        /// Descriptor type byte (u8 @desc+0) for point `idx` in bank `bank`.
        pub fn point_type(&self, bank: usize, idx: usize) -> Option<u8> {
            if bank > 1 || idx >= clk_vfp_info::POINTS {
                return None;
            }
            let base = if bank == 0 {
                clk_vfp_info::DESC1
            } else {
                clk_vfp_info::DESC2
            };
            let off = self.off(base + clk_vfp_info::DESC_STRIDE * idx, 1)?;
            self.rest.get(off).copied()
        }

        /// Copy the +4..+132 mask output into `status`' +4..+132 header —
        /// GetStatus REQUIRES this seed (zero → no records, garbage → -1).
        pub fn seed_status_header(
            &self,
            status: &mut NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1,
        ) {
            let src = self.off(clk_vfp_info::MASK1, 128).unwrap_or(0);
            let dst = status.off_mut(clk_vfp_info::MASK1, 128).unwrap_or(0);
            let n = 128.min(self.rest.len() - src).min(status.rest.len() - dst);
            status.rest[dst..dst + n].copy_from_slice(&self.rest[src..src + n]);
        }
    }

    /// Byte offsets into the private ClockClient V/F-POINTS GetStatus
    /// (ID 0x7FEE9032, RM 0x20809062). Two banks of up to 2048 records,
    /// 488B each; the +4..+132 header MUST be seeded from GetInfo's mask
    /// output first (see
    /// [`NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1::seed_status_header`]).
    ///
    /// Record layout (type-08 = V/F curve points, live-CALIBRATED R610.74
    /// against the public `get-vfp` GPC curve — records are INDEXED BY
    /// VOLTAGE, and the "voltage" fields are actually frequencies):
    /// - type u8 @rec+0
    /// - voltage u32 µV @rec+0x58 (mirrored @+0x68): rec0=450000 µV =
    ///   450 mV = public VFP point #0; the ascending voltage grid
    /// - default frequency u32 MHz @rec+0x24 (public "default MHz" column:
    ///   210 at points #0-3)
    /// - current frequency u32 MHz @rec+0x64 (= default + applied delta:
    ///   300 = 210 + 90 with a +90 MHz offset active; matches public
    ///   current/default exactly)
    pub mod clk_vfp_status {
        /// record header end / records region base for bank 1 (absolute)
        pub const REC1: usize = 772;
        /// bank-2 records base (absolute) — REC1 + 488*2048 + 768
        pub const REC2: usize = 1000964;
        /// per-record stride (user-struct; internal RM stride is 152B = 0x98)
        pub const STRIDE: usize = 488;
        /// records per bank
        pub const POINTS: usize = 2048;
        /// type u8 @rec+0
        pub const TYPE: usize = 0;
        /// default frequency (u32 MHz) for the point's voltage
        pub const FREQ_DEFAULT_MHZ: usize = 0x24;
        /// stock/default voltage (u32 µV; the V/F grid axis)
        pub const VOLTAGE_UV: usize = 0x58;
        /// current/effective frequency (u32 MHz; = default + applied delta)
        pub const FREQ_CURRENT_MHZ: usize = 0x64;
        /// current/effective voltage (u32 µV; = stock voltage + applied
        /// offset). Live 40-series probe with −45 mV: 1240000 → 1195000 —
        /// at stock it EQUALS +0x58, which is why it was once misread as a
        /// "voltage mirror".
        pub const VOLT_CURRENT_UV: usize = 0x68;

        // Blackwell (50-series) slot overrides. The +0x64 dword is a SIGNED
        // per-point voltage offset in µV instead of the current frequency
        // (live user probe 2026-09-02: a −45 mV experiment read back as
        // 4294922296 = 2³² + (−45000)); the frequency term moves to +0x24
        // (the 180 the 3-slot decoder displayed as "default"). NOTE: on
        // Ada, +0x68 is the CURRENT VOLTAGE (µV) — the Blackwell +0x68
        // default-frequency decode below follows the V|VO|C|D hypothesis
        // and is UNVERIFIED; a 50-series --dump-records under an active
        // offset settles it (405000-ish values ⇒ current voltage).
        /// Blackwell: signed per-point voltage offset (i32 µV)
        pub const BW_VOLT_OFFSET_UV: usize = 0x64;
        /// Blackwell: current frequency (u32 MHz)
        pub const BW_FREQ_CURRENT_MHZ: usize = 0x24;
        /// Blackwell: default frequency (u32 MHz — UNVERIFIED slot; Ada
        /// evidence says the modern +0x68 slot is the current voltage)
        pub const BW_FREQ_DEFAULT_MHZ: usize = 0x68;

        // Record model (Turing + Ampere live-verified 2026-09-02): each
        // record = a BASE section — +0x00 type, +0x24 default freq,
        // +0x58 default voltage, +0x64 current freq, +0x68 current volt —
        // optionally followed by an EXTENDED section: up to FOUR slots at
        // 0x10 stride (freq MHz @ +0x74+0x10*k, volt µV @ +4), packing
        // the NON-OWNER domains in ascending ClkDomains order from the
        // roster [XBAR(d1), SYS(d3), MSD(d5), HOST(d9)]:
        //   Turing — only GPC as main records (single block #0..126):
        //   4 slots = XBAR @0x74/78, SYS @0x84/88, MSD @0x94/98, HOST
        //   @0xA4/A8.
        //   Ampere — XBAR promoted to a second main block (#127..253):
        //   #0..126 are base-only, and the XBAR block fills only THREE
        //   slots = SYS @0x74/78, MSD @0x84/88, HOST @0x94/98 (user
        //   domain-id A/B).
        // Extension presence marker: NON-ZERO dwords at +0x2C and/or
        // +0x40 generally mean the extended section follows (base-only
        // records keep them zero). The decoder stays POSITIONAL (slot
        // k = roster-minus-owner[k], resolved by the consumer from the
        // record's segment owner) — never a generation table.
        /// extension-presence marker dword A (non-zero ⇒ extended section)
        pub const DOMAIN_EXT_MARKER_A: usize = 0x2C;
        /// extension-presence marker dword B (non-zero ⇒ extended section)
        pub const DOMAIN_EXT_MARKER_B: usize = 0x40;
        /// first ext slot's freq dword (k=0)
        pub const DOMAIN_CURRENT_BASE: usize = 0x74;
        /// stride per ext slot (freq@+0, volt@+4; +8..+16 unused)
        pub const DOMAIN_CURRENT_STRIDE: usize = 0x10;
        /// slots decoded (k=0..3 → +0x74/+0x84/+0x94/+0xA4)
        pub const DOMAIN_CURRENT_SLOTS: usize = 4;
    }

    nvstruct! {
        /// Private ClockClient V/F-POINTS GET_STATUS (ID 0x7FEE9032). Magic
        /// 2000388 (0x1E8604) bytes. Records at +772 / +1000964, 488B stride.
        /// Seed +4..+132 from GetInfo first. See [`clk_vfp_status`].
        pub struct NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1 {
            pub version: NvVersion,
            /// +4 .. +2000388: seeded header + 2×2048 records
            pub rest: [u8; 2000384],
        }
    }

    pub type NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE =
        NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1;

    impl NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1 {
        /// Literal magic dword the GetStatus handler accepts: the largest of
        /// {85016, 158200, 214652, 300164, 1525252, 2000388} — the full
        /// 2×2048-record layout (live-verified).
        pub const MAGIC: u32 = 2000388;
        /// Legacy magic (R391.35): smallest of the accepted set, 85016B.
        /// Old drivers reject the R610 2000388 stamp with -9; this one
        /// succeeds (live-verified GT730/391.35: status=0).
        pub const MAGIC_LEGACY: u32 = 85016; // 0x14C18
    }

    impl Default for NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1 {
        fn default() -> Self {
            Self {
                version: NvVersion::with_version(Self::MAGIC),
                rest: [0; 2000384],
            }
        }
    }

    impl NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1 {
        fn off(&self, abs: usize, len: usize) -> Option<usize> {
            let off = abs.checked_sub(4)?;
            let end = off.checked_add(len)?;
            if end <= self.rest.len() {
                Some(off)
            } else {
                None
            }
        }

        fn off_mut(&mut self, abs: usize, len: usize) -> Option<usize> {
            let off = abs.checked_sub(4)?;
            let end = off.checked_add(len)?;
            if end <= self.rest.len() {
                Some(off)
            } else {
                None
            }
        }

        fn u32_at(&self, abs: usize) -> Option<u32> {
            let off = self.off(abs, 4)?;
            self.rest
                .get(off..off + 4)
                .and_then(|s| s.try_into().ok())
                .map(u32::from_le_bytes)
        }

        fn rec_base(bank: usize, idx: usize) -> Option<usize> {
            if bank > 1 || idx >= clk_vfp_status::POINTS {
                return None;
            }
            Some(
                if bank == 0 {
                    clk_vfp_status::REC1
                } else {
                    clk_vfp_status::REC2
                } + clk_vfp_status::STRIDE * idx,
            )
        }

        /// Record type byte (u8 @rec+0) for point `idx` in bank `bank`.
        pub fn record_type(&self, bank: usize, idx: usize) -> Option<u8> {
            let base = Self::rec_base(bank, idx)?;
            let off = self.off(base + clk_vfp_status::TYPE, 1)?;
            self.rest.get(off).copied()
        }

        /// Default frequency (u32 MHz @rec+0x24) at the point's voltage.
        pub fn freq_default_mhz(&self, bank: usize, idx: usize) -> Option<u32> {
            let base = Self::rec_base(bank, idx)?;
            self.u32_at(base + clk_vfp_status::FREQ_DEFAULT_MHZ)
        }

        /// Current/effective frequency (u32 MHz @rec+0x64; default + delta).
        pub fn freq_current_mhz(&self, bank: usize, idx: usize) -> Option<u32> {
            let base = Self::rec_base(bank, idx)?;
            self.u32_at(base + clk_vfp_status::FREQ_CURRENT_MHZ)
        }

        /// Point voltage (u32 µV @rec+0x58 — the V/F grid axis).
        pub fn voltage_uv(&self, bank: usize, idx: usize) -> Option<u32> {
            let base = Self::rec_base(bank, idx)?;
            self.u32_at(base + clk_vfp_status::VOLTAGE_UV)
        }

        /// Raw u32 at `offset` inside point (bank, idx)'s record — escape
        /// hatch for generation-specific slots outside the R610.74
        /// calibrated layout (Blackwell's +0x64 voltage-offset / +0x68
        /// default-frequency overrides).
        pub fn raw_dword(&self, bank: usize, idx: usize, offset: usize) -> Option<u32> {
            let base = Self::rec_base(bank, idx)?;
            self.u32_at(base + offset)
        }

        /// The full 488-byte record for point (bank, idx) — diagnostic
        /// escape hatch for per-offset slot maps (--dump-records).
        pub fn raw_record(&self, bank: usize, idx: usize) -> Option<&[u8]> {
            let base = Self::rec_base(bank, idx)?;
            let off = self.off(base, clk_vfp_status::STRIDE)?;
            self.rest.get(off..off + clk_vfp_status::STRIDE)
        }
    }

    nvapi! {
        /// Private ClockClient V/F-POINTS GET_INFO (RM 0x20809061, ID
        /// 0x8895B510). Returns the per-bank point masks + descriptors.
        /// Its +4.. output seeds the GetStatus header. WORKS live (magic
        /// 0x78604) on R610.74.
        pub unsafe fn NvAPI_GPU_ClockClkVfPointsGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE) -> NvAPI_Status;
    }

    nvapi! {
        /// Private ClockClient V/F-POINTS GET_STATUS (RM 0x20809062, ID
        /// 0x7FEE9032). Returns the per-bank 488B point records. The +4..+132
        /// header MUST be seeded from GetInfo's mask output first. WORKS
        /// live (magic 2000388) on R610.74.
        pub unsafe fn NvAPI_GPU_ClockClkVfPointsGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pStatus: *mut NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE) -> NvAPI_Status;
    }

    /// Byte offsets into the private ClockClient V/F-POINTS GetControl /
    /// SetControl block (IDs 0xDA025C3E / 0xFEC00D04, RM cmd 117440585).
    ///
    /// IDA (sub_180215FC0 GET / sub_180218090 SET, R610.74): the canonical
    /// magic is 4670980 (0x474604) over a 4343300-byte (0x424604) buffer —
    /// once again magic ≠ version<<16|sizeof. Both handlers ALSO accept the
    /// smaller magics {82976, 401472, 737404, 1348740}, in which case they
    /// internally allocate the full buffer, stamp 4670980 and fill it from
    /// current driver state (sub_1801FAF30) before copying the user's
    /// masks/records over it — the sanctioned RMW snapshot path.
    ///
    /// Layout: bank-1 point mask @+4 (128B, input seed — copy from
    /// GetInfo), bank-1 records @+772 stride 1060; bank-2 mask @+2171652,
    /// bank-2 records @+2172420 stride 1060. Bank-1 record types
    /// {2,5,10,15}/{3,7,12,17} = pstate-ish; bank-2 record types
    /// {8,13,18} = V/F curve points (anything else → -103).
    ///
    /// Per-record WRITE semantics (what the driver reads back from us):
    /// - rec+0: type dword (remapped via sub_180202580)
    /// - rec+36 (dword[9]): mode — 0 = absolute, 1 = delta
    /// - rec+56: value — mode 0 = kHz frequency OFFSET (same as public
    ///   VFP freqDeltaKHz, max clamp ~990 MHz on Ada); mode 1 = reverse-volt
    ///   lookup (delta → voltage shift → look up default freq at shifted
    ///   voltage → that becomes the freq offset; mapping is non-linear,
    ///   depends on local MHz/mV slope). Both modes produce identical curves
    ///   after RM interpolation (flatten forward + 60 MHz/pt backward ramp).
    /// - rec+96 (byte): passthrough flag (bank-2 only)
    pub mod clk_vfp_control {
        /// canonical magic (accepted input and internal fill stamp)
        pub const MAGIC: u32 = 4670980;
        /// Legacy magic (R391.35): the GetControl/SetControl handlers on old
        /// drivers accept this small stamp (90116B) and reject the R610
        /// 4670980 with -9. Live-verified on GT730/391.35.
        pub const MAGIC_LEGACY: u32 = 90116; // 0x16004
        /// Volta/GV100: BOTH stamps above are rejected (-9) — but the R610
        /// snapshot magics are accepted for GetControl and (per the RMW
        /// path being present) SetControl. Smallest = smallest table.
        /// Live-verified on V100-SXM2/538.78 (2026-09-01).
        pub const MAGIC_SNAPSHOT: u32 = 82976; // 0x14420

        /// Volta legacy CONTROL layout (rest-relative, marker-echo mapped
        /// on V100: head [0,0x20) validated, payload [0x40,..) echoed
        /// verbatim, driver-owned fields = rec+0 flags + rec+0x24 value).
        /// Mask = LE bitfield (bit r = byte r/8 bit r%8) over the same
        /// point space as the legacy GetStatus records (128 GPC curve
        /// points + 4 bins on V100 = 132 present of 136 mask bits).
        pub const LEGACY_MASK: usize = 0x00; // 17 bytes = 136 bits
        pub const LEGACY_REC_BASE: usize = 0x60;
        pub const LEGACY_STRIDE: usize = 0x44;
        pub const LEGACY_POINTS: usize = 136;
        /// driver-owned flags dword: 1 = curve point, 0 = bin (mirrors
        /// the legacy GetStatus flags)
        pub const LEGACY_TYPE: usize = 0x00;
        /// driver-owned mode dword, same +36 offset as the R610 record's
        /// MODE field (0 = absolute kHz offset). First write attempt put
        /// the offset value here and the driver zeroed it — this dword is
        /// state-filled, not the user value slot.
        pub const LEGACY_MODE: usize = 0x24;
        /// user-space value dword (kHz offset), same +56 offset as the
        /// R610 record's VALUE field. ECHOED on GetControl (user space),
        /// so a stored offset is NOT readable back via GetControl — the
        /// effect must be verified through the legacy GetStatus freq
        /// dword (+8) instead. WRITE VERDICT: inert — the legacy
        /// SetControl accepts the struct but never applies the records
        /// (write matrix 2026-09-01, see gpu.rs set_vfp_point_private).
        pub const LEGACY_VALUE: usize = 0x38;
        /// buffer size (0x424604 — NOT derived from the magic)
        pub const SIZE: usize = 4343300;
        /// bank-1 point mask (input seed from GetInfo)
        pub const MASK1: usize = 4;
        /// bank-1 records base, stride 1060
        pub const REC1: usize = 772;
        /// per-record stride (both banks)
        pub const STRIDE: usize = 1060;
        /// bank-2 point mask
        pub const MASK2: usize = 2171652;
        /// bank-2 records base
        pub const REC2: usize = 2172420;
        /// records per bank
        pub const POINTS: usize = 2048;
        /// record type dword
        pub const TYPE: usize = 0;
        /// mode dword: 0 = absolute, 1 = delta
        pub const MODE: usize = 36;
        /// value (u32 absolute @+56; i16 delta at the same offset in mode 1)
        pub const VALUE: usize = 56;
        /// passthrough flag byte (bank-2 records)
        pub const FLAG: usize = 96;
    }

    nvstruct! {
        /// Private ClockClient V/F-POINTS GetControl/SetControl block
        /// (0xDA025C3E / 0xFEC00D04). See [`clk_vfp_control`] for layout and
        /// the per-record write semantics. For a safe RMW: GetControl with
        /// the masks seeded from GetInfo → snapshot → patch → SetControl →
        /// GetControl readback → restore on mismatch.
        pub struct NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE_V1 {
            pub version: NvVersion,
            /// +4 .. +4343300
            pub rest: [u8; 4343296],
        }
    }

    pub type NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE =
        NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE_V1;

    impl NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE_V1 {
        fn off(&self, abs: usize, len: usize) -> Option<usize> {
            let off = abs.checked_sub(4)?;
            let end = off.checked_add(len)?;
            if end <= self.rest.len() {
                Some(off)
            } else {
                None
            }
        }

        fn off_mut(&mut self, abs: usize, len: usize) -> Option<usize> {
            let off = abs.checked_sub(4)?;
            let end = off.checked_add(len)?;
            if end <= self.rest.len() {
                Some(off)
            } else {
                None
            }
        }

        fn u32_at(&self, abs: usize) -> Option<u32> {
            let off = self.off(abs, 4)?;
            self.rest
                .get(off..off + 4)
                .and_then(|s| s.try_into().ok())
                .map(u32::from_le_bytes)
        }

        /// Seed both bank masks from a GetInfo block's +4/+0x34304 mask
        /// outputs (128B each). The handlers only touch masked points.
        pub fn seed_masks_from_info(
            &mut self,
            info: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1,
        ) {
            for (dst_abs, src_abs) in [
                (clk_vfp_control::MASK1, clk_vfp_info::MASK1),
                (clk_vfp_control::MASK2, clk_vfp_info::MASK2),
            ] {
                let dst = self.off_mut(dst_abs, 128).unwrap_or(0);
                let src = info.off(src_abs, 128).unwrap_or(0);
                let n = 128.min(self.rest.len() - dst).min(info.rest.len() - src);
                self.rest[dst..dst + n].copy_from_slice(&info.rest[src..src + n]);
            }
        }

        fn rec_base(bank: usize, idx: usize) -> Option<usize> {
            if bank > 1 || idx >= clk_vfp_control::POINTS {
                return None;
            }
            Some(
                if bank == 0 {
                    clk_vfp_control::REC1
                } else {
                    clk_vfp_control::REC2
                } + clk_vfp_control::STRIDE * idx,
            )
        }

        /// Record type low byte for point `idx` in bank `bank`.
        pub fn record_type(&self, bank: usize, idx: usize) -> Option<u8> {
            let base = Self::rec_base(bank, idx)?;
            let off = self.off(base + clk_vfp_control::TYPE, 1)?;
            self.rest.get(off).copied()
        }

        /// Mode dword (rec+36): 0 = absolute, 1 = delta.
        pub fn mode(&self, bank: usize, idx: usize) -> Option<u32> {
            let base = Self::rec_base(bank, idx)?;
            self.u32_at(base + clk_vfp_control::MODE)
        }

        /// Value dword (rec+56).
        pub fn value(&self, bank: usize, idx: usize) -> Option<u32> {
            let base = Self::rec_base(bank, idx)?;
            self.u32_at(base + clk_vfp_control::VALUE)
        }

        /// Program a point absolutely: mode 0 + u32 value (rec+36/+56).
        pub fn set_absolute(&mut self, bank: usize, idx: usize, value: u32) -> Option<()> {
            let base = Self::rec_base(bank, idx)?;
            let m = self.off_mut(base + clk_vfp_control::MODE, 4)?;
            self.rest[m..m + 4].copy_from_slice(&0u32.to_le_bytes());
            let v = self.off_mut(base + clk_vfp_control::VALUE, 4)?;
            self.rest[v..v + 4].copy_from_slice(&value.to_le_bytes());
            Some(())
        }

        /// Program a point as a delta: mode 1 + i16 delta (rec+36/+56).
        pub fn set_delta(&mut self, bank: usize, idx: usize, delta: i16) -> Option<()> {
            let base = Self::rec_base(bank, idx)?;
            let m = self.off_mut(base + clk_vfp_control::MODE, 4)?;
            self.rest[m..m + 4].copy_from_slice(&1u32.to_le_bytes());
            let v = self.off_mut(base + clk_vfp_control::VALUE, 2)?;
            self.rest[v..v + 2].copy_from_slice(&delta.to_le_bytes());
            Some(())
        }

        /// Set the record type byte (rec+0) — the CONTROL family's user
        /// type, NOT the GetStatus type (the two families use different
        /// type numbering). Bank 0 accepts user types {0,1,3,4,7,8,12,13};
        /// bank 1 accepts {6,10,14}. Use 8 for bank-0 V/F points (mode/
        /// value variant), 6 for bank-1 V/F points (single-u32 variant).
        pub fn set_record_type(&mut self, bank: usize, idx: usize, ty: u8) -> Option<()> {
            let base = Self::rec_base(bank, idx)?;
            let off = self.off_mut(base + clk_vfp_control::TYPE, 1)?;
            self.rest[off] = ty;
            Some(())
        }

        /// Set a bit in the bank mask (enables the point for SET processing).
        pub fn set_mask_bit(&mut self, bank: usize, idx: usize) -> Option<()> {
            if bank > 1 || idx >= clk_vfp_control::POINTS {
                return None;
            }
            let mask_base = if bank == 0 {
                clk_vfp_control::MASK1
            } else {
                clk_vfp_control::MASK2
            };
            let dword_idx = idx >> 5;
            let bit_idx = idx & 31;
            let off = self.off_mut(mask_base + 4 * dword_idx, 4)?;
            let mut dword = u32::from_le_bytes(self.rest[off..off + 4].try_into().ok()?);
            dword |= 1u32 << bit_idx;
            self.rest[off..off + 4].copy_from_slice(&dword.to_le_bytes());
            Some(())
        }

        fn legacy_rec_base(idx: usize) -> Option<usize> {
            if idx >= clk_vfp_control::LEGACY_POINTS {
                return None;
            }
            Some(clk_vfp_control::LEGACY_REC_BASE + clk_vfp_control::LEGACY_STRIDE * idx)
        }

        /// Volta legacy: mode dword (rec+0x24, state-filled on GetControl
        /// — the marker probe showed the driver overwrites user input
        /// here, so this IS the readable-back field).
        pub fn legacy_mode(&self, idx: usize) -> Option<u32> {
            let base = Self::legacy_rec_base(idx)?;
            self.u32_at(base + clk_vfp_control::LEGACY_MODE)
        }

        /// Volta legacy SET field map (538.78 sub_180258570, case 0x14420
        /// — the 0x44-stride normalizer): rec+0x00 = MODE dword (0 =
        /// absolute u32 kHz offset, 1 = delta i16, anything else is
        /// SKIPPED silently), rec+0x24 = VALUE. Note the GET side
        /// OVERLOADS rec+0 as the curve/bin flag (1/0) and rec+0x24 as
        /// its state slot — a snapshot RMW must REWRITE both fields for
        /// every record it sends, never carry the GET echo over.
        pub fn legacy_set_mode_value(&mut self, idx: usize, mode: u32, value: u32) -> Option<()> {
            let base = Self::legacy_rec_base(idx)?;
            let m = self.off_mut(base + clk_vfp_control::LEGACY_TYPE, 4)?;
            self.rest[m..m + 4].copy_from_slice(&mode.to_le_bytes());
            let v = self.off_mut(base + clk_vfp_control::LEGACY_MODE, 4)?;
            self.rest[v..v + 4].copy_from_slice(&value.to_le_bytes());
            Some(())
        }

        /// Volta legacy: neutralize one record for SET (mode 0 + value 0
        /// = absolute zero offset) — used to scrub the GET-echoed flag
        /// bytes out of the mask records that must stay untouched.
        pub fn legacy_set_neutral(&mut self, idx: usize) -> Option<()> {
            self.legacy_set_mode_value(idx, 0, 0)
        }

        /// Volta legacy: set mask bit `idx` (LE bitfield at rest[0..0x11]).
        pub fn legacy_set_mask_bit(&mut self, idx: usize) -> Option<()> {
            if idx >= clk_vfp_control::LEGACY_POINTS {
                return None;
            }
            let off = self.off_mut(clk_vfp_control::LEGACY_MASK + idx / 8, 1)?;
            self.rest[off] |= 1 << (idx % 8);
            Some(())
        }

        /// Volta legacy: seed the control mask from the legacy GetInfo
        /// mask (rest[0..0x14], same LE-bitfield format, truncated to the
        /// control's 136-bit space).
        pub fn legacy_seed_masks_from_info(
            &mut self,
            info: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1,
        ) {
            let src = info.off(clk_vfp_info::MASK1, 17).unwrap_or(0);
            let dst = self.off_mut(clk_vfp_control::LEGACY_MASK, 17).unwrap_or(0);
            let n = 17.min(self.rest.len() - dst).min(info.rest.len() - src);
            self.rest[dst..dst + n].copy_from_slice(&info.rest[src..src + n]);
        }
    }

    impl Default for NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE_V1 {
        fn default() -> Self {
            // Avoid stack overflow: the 4MB rest[] would be allocated on
            // the stack by Default then moved to heap by Box::new. Callers
            // should use `unsafe { std::mem::zeroed() }` + set version
            // instead, but we provide this for non-Box use cases.
            Self {
                version: NvVersion::with_version(clk_vfp_control::MAGIC),
                rest: [0; 4343296],
            }
        }
    }

    nvapi! {
        /// Private ClockClient V/F-POINTS GET_CONTROL (ID 0xDA025C3E). Returns
        /// the 1060B-record control block; seed the bank masks from GetInfo
        /// first. Non-4670980 magics get internally expanded + filled from
        /// current state — the RMW snapshot source for SetControl.
        pub unsafe fn NvAPI_GPU_ClockClkVfPointsGetControl(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *mut NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE) -> NvAPI_Status;
    }

    nvapi! {
        /// Private ClockClient V/F-POINTS SET_CONTROL (ID 0xFEC00D04).
        /// DANGEROUS V/F curve write. Always snapshot via GetControl first,
        /// patch a copy, SET, read back, restore on mismatch.
        pub unsafe fn NvAPI_GPU_ClockClkVfPointsSetControl(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *const NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE) -> NvAPI_Status;
    }

    // --- PerfVfeEqu / PerfVfeVar family (escape 0x070001C6) ----------------
    //
    // IDA + live RE'd 2026-08-26 on R610.74 nvapi64_impl.dll. This is the
    // THIRD V/F edit surface, distinct from the public VfPoints (0x07000049)
    // and the private ClockClient V/F-POINTS (0x2080906x): it exposes the RM
    // voltage-frequency EQUATIONS (Equ) and VARIABLES (Var) that generate the
    // curve, not the resulting points.
    //
    // All 6 IDs: `fn(hGpu, versioned-struct*)` — arg1 is the PHYSICAL GPU
    // handle (`!a1 -> -101`), NOT a domain selector. Escape buffer 0x100440;
    // hGpu @ dword[12]; RM cmd @ dword[13]. SETs are elevation-gated
    // (sub_18038FE40 -> -104 without admin).

    /// Byte offsets for PerfVfeEquGetInfo (ID 0x8D49471C, RM 0x2080A0B5).
    ///
    /// IDA sub_1802AB410: accepted magics {83996, 209092, 221508, 885828};
    /// the handler ALWAYS works in the 885828 (0xD8444) layout internally
    /// (0x98444 = 623684 bytes), so callers should pass 885828 directly.
    /// The 256-dword (8192-bit) mask is pure OUTPUT (driver fills it); info
    /// entries at +1100, stride 76: type u32 @+0, name u16 @+4 (live 4060L:
    /// type 3, names like 0xFF0B/0x2413/0x2514), extras @+34/+36 by type.
    /// Live-verified 2026-08-26: 367 mask bits, 29 typed entries.
    pub mod vfe_equ_info {
        /// live-verified magic (0x1481C) — the layout constants below are
        /// calibrated against THIS tier; larger tiers (209092/221508/885828)
        /// use different internal offsets, do not blindly re-stamp
        pub const MAGIC: u32 = 83996;
        /// total struct size for MAGIC
        pub const SIZE: usize = 83996;
        /// 256-dword output mask (bits 0..8191)
        pub const MASK: usize = 4;
        pub const MASK_LEN: usize = 1024;
        /// first entry base (absolute; live-calibrated 4060L R610.74: record
        /// 0 sits at 0x4DC — the earlier 76-stride reading was a
        /// sampling-drift artifact, consecutive records are 72B apart)
        pub const ENTRIES: usize = 1244;
        /// per-entry stride
        pub const STRIDE: usize = 72;
        /// entry+8: u32 type (nonzero = present; live 1/2/3)
        pub const TYPE: usize = 8;
        /// entry+12: u16 RM name id (live 0xFF0B / 0x2413 / 0x2514 …)
        pub const NAME: usize = 12;
        /// entry+14: u16 aux (live 1/2)
        pub const AUX: usize = 14;
        /// raw payload dwords start
        pub const PAYLOAD: usize = 16;
        pub const MAX_ENTRIES: usize = 8192;
    }

    nvstruct! {
        /// PerfVfeEquGetInfo block (ID 0x8D49471C). Live-verified magic
        /// 83996 (0x1481C); the rest array MUST stay SIZE-4 so the accessor
        /// bounds check matches the actual allocation. GET-only.
        pub struct NV_PERF_VFE_EQU_INFO {
            pub version: NvVersion,
            /// +4..83996: mask@+4, entries@+1244 stride 72
            pub rest: [u8; 83992],
        }
    }

    /// Byte offsets for PerfVfeEquGetControl / SetControl (IDs 0x4C75C9FE /
    /// 0x68B798C4, RM 0x2080A0B6 / 0x2080E0B7).
    ///
    /// IDA sub_1802AA9C0: accepted magics {85016, 209092, 221508, 352580,
    /// 1410116} are CAPACITY classes of the same layout. The 256-dword mask
    /// at +4 is INPUT (copied into the escape; entries are returned for set
    /// bits) and echoed back EXPANDED to the readable set (live: seeded 64
    /// bits -> 480-bit echo). Entries at +1136 stride 172, type u32 @+0.
    /// Live-verified 2026-08-26 with magic 85016.
    pub mod vfe_equ_control {
        /// largest-capacity magic (0x1584C4, 1410116 bytes)
        pub const MAGIC_MAX: u32 = 1410116;
        /// smallest magic (0x14C18, 85016 bytes) — live-verified fallback
        pub const MAGIC_MIN: u32 = 85016;
        /// total struct size for MAGIC_MAX
        pub const SIZE_MAX: usize = 1410116;
        /// in/out entry-selection mask (256 dwords)
        pub const MASK: usize = 4;
        pub const MASK_LEN: usize = 1024;
        /// entries base, stride 172
        pub const ENTRIES: usize = 1136;
        pub const STRIDE: usize = 172;
        /// entry+0: u32 type tag (1/2/3/6/7 per IDA)
        pub const TYPE: usize = 0;
    }

    nvstruct! {
        /// PerfVfeEqu GetControl/SetControl block. Sized for the largest
        /// capacity magic 1410116 (0x1584C4); re-stamp `version` with 85016
        /// to fall back to the smaller live-verified capacity (487 entries).
        pub struct NV_PERF_VFE_EQU_CONTROL {
            pub version: NvVersion,
            /// +4..1410116: mask@+4, entries@+1136 stride 172
            pub rest: [u8; 1410112],
        }
    }

    /// Byte offsets for PerfVfeVarGetInfo (ID 0xB9DA41D6, RM 0x2080A0B1).
    ///
    /// IDA sub_1802AD1C0: accepted magics {70344, 70600, 489736, 3118440};
    /// the handler works in the 3118440 (0x2F9568-magic) layout = 0x2B9568
    /// (2856296) bytes: 32-byte mask (256 bits) @+4, byte @+36, then up to
    /// 255 entries of 11200 bytes each starting at +36. Per-entry type tag
    /// @entry+296 (types 2/3/5/7/8/9/10/11/13/15/17/18), name bytes near
    /// +1916/+1917, deep sub-record arrays at +1920.. (20-byte elements).
    pub mod vfe_var_info {
        /// live-verified magic (0x112C8) — layout below calibrated against
        /// THIS tier on 4060L R610.74; larger tiers use different offsets
        pub const MAGIC: u32 = 70344;
        /// total struct size for MAGIC
        pub const SIZE: usize = 70344;
        /// u32 output mask @+4 (live 0x003FFFFF = 22 bits)
        pub const MASK: usize = 4;
        pub const MASK_LEN: usize = 32;
        /// entries base (absolute; live record 0 @0x48)
        pub const ENTRIES: usize = 72;
        /// per-entry stride (live 0x94)
        pub const STRIDE: usize = 148;
        /// entry+0: u32 type tag (live 13)
        pub const TYPE: usize = 0;
        /// raw payload dwords start
        pub const PAYLOAD: usize = 4;
        pub const MAX_ENTRIES: usize = 255;
    }

    nvstruct! {
        /// PerfVfeVarGetInfo block (ID 0xB9DA41D6). Live-verified magic
        /// 70344 (0x112C8); rest MUST stay SIZE-4 so bounds checks match
        /// the allocation. GET-only.
        pub struct NV_PERF_VFE_VAR_INFO {
            pub version: NvVersion,
            /// +4..70344: mask@+4, entries@+72 stride 148
            pub rest: [u8; 70340],
        }
    }

    /// Byte offsets for PerfVfeVarGetControl / SetControl (IDs 0x5D387298 /
    /// 0x79FA23A2, RM 0x2080A0B3 / 0x2080E0B0).
    ///
    /// Accepted magics {68300 (0x10ACC), 171976 (0x29FC8)} — capacity
    /// classes; the layout below is calibrated against 68300 (the other
    /// tier differs — do not blindly re-stamp). Header: magic @+0, input
    /// mask u32 @+4, u32 record count @+8 (live 0x46=70). Records from
    /// +0x4C stride 0x58=88: {u32 type (live 13), float-ish payload}.
    pub mod vfe_var_control {
        /// live-verified magic (0x10ACC)
        pub const MAGIC: u32 = 68300;
        /// total struct size for MAGIC
        pub const SIZE: usize = 68300;
        /// u32 record count @+8
        pub const COUNT: usize = 8;
        /// input mask u32 @+4 (bits select entries)
        pub const MASK: usize = 4;
        /// entries base (absolute; live record 0 @0x4C)
        pub const ENTRIES: usize = 76;
        /// per-entry stride (live 0x58)
        pub const STRIDE: usize = 88;
        /// entry+0: u32 type tag
        pub const TYPE: usize = 0;
        /// raw payload dwords start
        pub const PAYLOAD: usize = 4;
        pub const MAX_ENTRIES: usize = 255;
    }

    nvstruct! {
        /// PerfVfeVar GetControl/SetControl block. Sized for the
        /// live-verified magic 68300 (0x10ACC); rest MUST stay SIZE-4.
        pub struct NV_PERF_VFE_VAR_CONTROL {
            pub version: NvVersion,
            /// +4..68300: header, entries @+76 stride 88
            pub rest: [u8; 68296],
        }
    }

    impl NV_PERF_VFE_EQU_INFO {
        fn off(&self, abs: usize, len: usize) -> Option<usize> {
            let off = abs.checked_sub(4)?;
            if off + len <= self.rest.len() {
                Some(off)
            } else {
                None
            }
        }

        fn u32_at(&self, abs: usize) -> Option<u32> {
            let o = self.off(abs, 4)?;
            Some(u32::from_le_bytes(self.rest[o..o + 4].try_into().ok()?))
        }

        fn u16_at(&self, abs: usize) -> Option<u16> {
            let o = self.off(abs, 2)?;
            Some(u16::from_le_bytes(self.rest[o..o + 2].try_into().ok()?))
        }

        /// Output mask bit `i` (0..8191).
        pub fn mask_bit(&self, i: usize) -> Option<bool> {
            if i >= vfe_equ_info::MAX_ENTRIES {
                return None;
            }
            let dword = self.u32_at(vfe_equ_info::MASK + 4 * (i >> 5))?;
            Some(dword & (1u32 << (i & 31)) != 0)
        }

        /// Entry type u32 (nonzero = present) for entry `i`.
        pub fn entry_type(&self, i: usize) -> Option<u32> {
            self.u32_at(vfe_equ_info::ENTRIES + vfe_equ_info::STRIDE * i + vfe_equ_info::TYPE)
        }

        /// Entry name u16 for entry `i`.
        pub fn entry_name(&self, i: usize) -> Option<u16> {
            self.u16_at(vfe_equ_info::ENTRIES + vfe_equ_info::STRIDE * i + vfe_equ_info::NAME)
        }

        /// Entry aux u16 for entry `i`.
        pub fn entry_aux(&self, i: usize) -> Option<u16> {
            self.u16_at(vfe_equ_info::ENTRIES + vfe_equ_info::STRIDE * i + vfe_equ_info::AUX)
        }

        /// First `n` dwords of entry `i` (raw payload beyond type/name/aux).
        pub fn entry_dwords(&self, i: usize, n: usize) -> Option<Vec<u32>> {
            let base = vfe_equ_info::ENTRIES + vfe_equ_info::STRIDE * i + vfe_equ_info::PAYLOAD;
            (0..n).map(|k| self.u32_at(base + 4 * k)).collect()
        }
    }

    impl NV_PERF_VFE_EQU_CONTROL {
        fn off_mut(&mut self, abs: usize, len: usize) -> Option<usize> {
            let off = abs.checked_sub(4)?;
            if off + len <= self.rest.len() {
                Some(off)
            } else {
                None
            }
        }

        fn u32_at(&self, abs: usize) -> Option<u32> {
            let off = abs.checked_sub(4)?;
            if off + 4 > self.rest.len() {
                return None;
            }
            Some(u32::from_le_bytes(self.rest[off..off + 4].try_into().ok()?))
        }

        /// Seed the input mask from a GetInfo mask (call before GET_CONTROL).
        pub fn seed_mask_dwords(&mut self, dwords: &[u32]) {
            for (i, &d) in dwords.iter().take(256).enumerate() {
                if let Some(o) = self.off_mut(vfe_equ_control::MASK + 4 * i, 4) {
                    self.rest[o..o + 4].copy_from_slice(&d.to_le_bytes());
                }
            }
        }

        /// Seed the first `bits` mask bits (bits 0..bits-1).
        pub fn seed_mask_bits(&mut self, bits: usize) {
            for i in 0..bits.min(8192) {
                let dword = i >> 5;
                if let Some(o) = self.off_mut(vfe_equ_control::MASK + 4 * dword, 4) {
                    let mut d =
                        u32::from_le_bytes(self.rest[o..o + 4].try_into().unwrap_or([0; 4]));
                    d |= 1u32 << (i & 31);
                    self.rest[o..o + 4].copy_from_slice(&d.to_le_bytes());
                }
            }
        }

        /// Mask echo bit `i`.
        pub fn mask_bit(&self, i: usize) -> Option<bool> {
            if i >= 8192 {
                return None;
            }
            let dword = self.u32_at(vfe_equ_control::MASK + 4 * (i >> 5))?;
            Some(dword & (1u32 << (i & 31)) != 0)
        }

        /// Entry type u32 for entry `i`.
        pub fn entry_type(&self, i: usize) -> Option<u32> {
            self.u32_at(vfe_equ_control::ENTRIES + vfe_equ_control::STRIDE * i)
        }

        /// First `n` dwords of entry `i`.
        pub fn entry_dwords(&self, i: usize, n: usize) -> Option<Vec<u32>> {
            let base = vfe_equ_control::ENTRIES + vfe_equ_control::STRIDE * i;
            (0..n).map(|k| self.u32_at(base + 4 * k)).collect()
        }
    }

    impl NV_PERF_VFE_VAR_INFO {
        fn off(&self, abs: usize, len: usize) -> Option<usize> {
            let off = abs.checked_sub(4)?;
            if off + len <= self.rest.len() {
                Some(off)
            } else {
                None
            }
        }

        fn u32_at(&self, abs: usize) -> Option<u32> {
            let o = self.off(abs, 4)?;
            Some(u32::from_le_bytes(self.rest[o..o + 4].try_into().ok()?))
        }

        /// Output mask bit `i` (0..255).
        pub fn mask_bit(&self, i: usize) -> Option<bool> {
            if i >= vfe_var_info::MAX_ENTRIES {
                return None;
            }
            let dword = self.u32_at(vfe_var_info::MASK + 4 * (i >> 5))?;
            Some(dword & (1u32 << (i & 31)) != 0)
        }

        /// Entry type i32 @entry+0 for entry `i` (0 = absent; live 13).
        pub fn entry_type(&self, i: usize) -> Option<i32> {
            self.u32_at(vfe_var_info::ENTRIES + vfe_var_info::STRIDE * i + vfe_var_info::TYPE)
                .map(|v| v as i32)
        }

        /// First `n` dwords of entry `i` (raw payload).
        pub fn entry_dwords(&self, i: usize, n: usize) -> Option<Vec<u32>> {
            let base = vfe_var_info::ENTRIES + vfe_var_info::STRIDE * i;
            (0..n).map(|k| self.u32_at(base + 4 * k)).collect()
        }
    }

    impl NV_PERF_VFE_VAR_CONTROL {
        fn u32_at(&self, abs: usize) -> Option<u32> {
            let off = abs.checked_sub(4)?;
            if off + 4 > self.rest.len() {
                return None;
            }
            Some(u32::from_le_bytes(self.rest[off..off + 4].try_into().ok()?))
        }

        fn off_mut(&mut self, abs: usize, len: usize) -> Option<usize> {
            let off = abs.checked_sub(4)?;
            if off + len <= self.rest.len() {
                Some(off)
            } else {
                None
            }
        }

        /// Header count-ish u32 @+8 (live 0x46).
        pub fn count(&self) -> Option<u32> {
            self.u32_at(vfe_var_control::COUNT)
        }

        /// Seed the input mask u32 @+4.
        pub fn seed_mask(&mut self, mask: u32) {
            if let Some(o) = self.off_mut(vfe_var_control::MASK, 4) {
                self.rest[o..o + 4].copy_from_slice(&mask.to_le_bytes());
            }
        }

        /// Entry `i` first `n` dwords (raw; base +160 stride 160 tentative).
        pub fn entry_dwords(&self, i: usize, n: usize) -> Option<Vec<u32>> {
            let base = vfe_var_control::ENTRIES + vfe_var_control::STRIDE * i;
            (0..n).map(|k| self.u32_at(base + 4 * k)).collect()
        }
    }

    /// Layout regression tests for the PerfVfeEqu/Var family. The accessors
    /// bounds-check against `rest.len()`, so these pin BOTH the live-tier
    /// offsets and the "rest array must be exactly SIZE-4" allocation rule
    /// (an oversized rest silently reads heap garbage — the bug that caused
    /// the first equ-info decode to return pointers as names).
    #[cfg(test)]
    mod vfe_tests {
        use super::*;

        fn put_u16(rest: &mut [u8], abs: usize, v: u16) {
            rest[abs - 4..abs - 2].copy_from_slice(&v.to_le_bytes());
        }

        fn put_u32(rest: &mut [u8], abs: usize, v: u32) {
            rest[abs - 4..abs].copy_from_slice(&v.to_le_bytes());
        }

        /// rest arrays must be exactly SIZE-4 so `off()` bounds match the
        /// allocation (see module doc above).
        #[test]
        fn vfe_rest_arrays_match_size() {
            assert_eq!(
                size_of::<NV_PERF_VFE_EQU_INFO>(),
                vfe_equ_info::SIZE,
                "EQU_INFO must be allocated at the live magic-83996 tier"
            );
            assert_eq!(size_of::<NV_PERF_VFE_EQU_INFO>() - 4, 83992);
            assert_eq!(size_of::<NV_PERF_VFE_VAR_INFO>(), vfe_var_info::SIZE);
            assert_eq!(size_of::<NV_PERF_VFE_VAR_CONTROL>(), vfe_var_control::SIZE);
            assert_eq!(
                size_of::<NV_PERF_VFE_EQU_CONTROL>(),
                vfe_equ_control::SIZE_MAX
            );
        }

        /// equ-info: synthetic entry decode round-trip at the calibrated
        /// offsets (entries @1244 stride 72, type@+8 name@+12 aux@+14
        /// payload@+16), including a strided entry and out-of-range probes.
        #[test]
        fn vfe_equ_info_entry_decode() {
            let mut s = Box::new(NV_PERF_VFE_EQU_INFO {
                version: NvVersion::with_version(vfe_equ_info::MAGIC),
                rest: [0; 83992],
            });
            for (i, (ty, name, aux)) in [(1u32, 0xFF0Bu16, 1u16), (3, 0x1711, 2)].iter().enumerate()
            {
                let base = vfe_equ_info::ENTRIES + vfe_equ_info::STRIDE * i;
                put_u32(&mut s.rest, base + vfe_equ_info::TYPE, *ty);
                put_u16(&mut s.rest, base + vfe_equ_info::NAME, *name);
                put_u16(&mut s.rest, base + vfe_equ_info::AUX, *aux);
                put_u32(&mut s.rest, base + vfe_equ_info::PAYLOAD, 0xDEAD_BEEF);
            }
            assert_eq!(s.entry_type(0), Some(1));
            assert_eq!(s.entry_name(0), Some(0xFF0B));
            assert_eq!(s.entry_aux(0), Some(1));
            assert_eq!(s.entry_dwords(0, 1), Some(vec![0xDEAD_BEEF]));
            assert_eq!(s.entry_type(1), Some(3));
            assert_eq!(s.entry_name(1), Some(0x1711));
            // stride is 72, not the earlier mis-read 76: entry 1's payload
            // must not overlap entry 2's header
            assert_eq!(s.entry_type(2), Some(0));
            // beyond the allocation the accessor must refuse (None), never
            // read past the buffer
            let over = (vfe_equ_info::SIZE - vfe_equ_info::ENTRIES) / vfe_equ_info::STRIDE + 2;
            assert!(over < vfe_equ_info::MAX_ENTRIES);
            assert_eq!(s.entry_type(over), None);
            // mask dword 0 @+4
            put_u32(&mut s.rest, vfe_equ_info::MASK, 1 << 5 | 1 << 31);
            assert_eq!(s.mask_bit(5), Some(true));
            assert_eq!(s.mask_bit(31), Some(true));
            assert_eq!(s.mask_bit(0), Some(false));
            assert_eq!(s.mask_bit(vfe_equ_info::MAX_ENTRIES), None);
        }

        /// var-info: 22-bit mask + typed entries @72 stride 148.
        #[test]
        fn vfe_var_info_decode() {
            let mut s = Box::new(NV_PERF_VFE_VAR_INFO {
                version: NvVersion::with_version(vfe_var_info::MAGIC),
                rest: [0; 70340],
            });
            put_u32(&mut s.rest, vfe_var_info::MASK, 0x003F_FFFF);
            let base = vfe_var_info::ENTRIES + vfe_var_info::STRIDE * 5;
            put_u32(&mut s.rest, base + vfe_var_info::TYPE, 13);
            put_u32(&mut s.rest, base + vfe_var_info::PAYLOAD, 42);
            assert_eq!(s.mask_bit(21), Some(true));
            assert_eq!(s.mask_bit(22), Some(false));
            assert_eq!(s.mask_bit(vfe_var_info::MAX_ENTRIES), None);
            assert_eq!(s.entry_type(5), Some(13));
            // entry_dwords starts AT the type dword (no +PAYLOAD gap here,
            // unlike equ-info) — dwords[0] is the type
            assert_eq!(s.entry_dwords(5, 2), Some(vec![13, 42]));
            // the tier's byte capacity (70344B) exceeds 255×148 records —
            // MAX_ENTRIES is the driver's record limit, not the buffer's
            let over = (vfe_var_info::SIZE - vfe_var_info::ENTRIES) / vfe_var_info::STRIDE + 2;
            assert!(over > vfe_var_info::MAX_ENTRIES);
            assert_eq!(s.entry_type(over), None);
        }

        /// var-control: count u32 @+8, seed mask u32 @+4, records @76
        /// stride 88.
        #[test]
        fn vfe_var_control_decode() {
            let mut s = Box::new(NV_PERF_VFE_VAR_CONTROL {
                version: NvVersion::with_version(vfe_var_control::MAGIC),
                rest: [0; 68296],
            });
            put_u32(&mut s.rest, vfe_var_control::COUNT, 70);
            s.seed_mask(0xFFFF);
            assert_eq!(s.count(), Some(70));
            let base = vfe_var_control::ENTRIES + vfe_var_control::STRIDE * 3;
            put_u32(&mut s.rest, base + vfe_var_control::TYPE, 13);
            put_u32(&mut s.rest, base + vfe_var_control::PAYLOAD, 7);
            assert_eq!(s.entry_dwords(3, 2), Some(vec![13, 7]));
            let over =
                (vfe_var_control::SIZE - vfe_var_control::ENTRIES) / vfe_var_control::STRIDE + 2;
            assert!(over > vfe_var_control::MAX_ENTRIES);
            assert_eq!(s.entry_dwords(over, 1), None);
        }

        /// equ-control: mask seeding round-trip (IN mask the driver echoes
        /// expanded). Entry decode stays raw/tentative — only bounds pinned.
        /// The 1.4 MB struct is heap-zeroed directly (a literal Box::new
        /// would build it on the stack first and overflow).
        #[test]
        fn vfe_equ_control_mask_seed() {
            let mut s = {
                assert_eq!(size_of::<NV_PERF_VFE_EQU_CONTROL>(), 4 + 1410112);
                let layout = std::alloc::Layout::new::<NV_PERF_VFE_EQU_CONTROL>();
                let ptr =
                    unsafe { std::alloc::alloc_zeroed(layout) as *mut NV_PERF_VFE_EQU_CONTROL };
                assert!(!ptr.is_null());
                let mut s = unsafe { Box::from_raw(ptr) };
                s.version = NvVersion::with_version(vfe_equ_control::MAGIC_MIN);
                s
            };
            s.seed_mask_dwords(&[0x8000_0000, 1]);
            assert_eq!(s.mask_bit(31), Some(true));
            assert_eq!(s.mask_bit(32), Some(true));
            assert_eq!(s.mask_bit(0), Some(false));
            assert_eq!(s.mask_bit(8192), None);
            s.seed_mask_bits(3);
            for i in 0..3 {
                assert_eq!(s.mask_bit(i), Some(true));
            }
        }
    }

    nvapi! {
        /// Private PerfVfeEqu GET_INFO (ID 0x8D49471C, RM 0x2080A0B5).
        /// Returns the equation-directory mask + per-entry type/name.
        /// WORKS live on Ada 4060 Laptop (magic 83996): 367 mask bits,
        /// 239 typed entries @+1244 stride 72.
        pub unsafe fn NvAPI_GPU_PerfVfeEquGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_PERF_VFE_EQU_INFO) -> NvAPI_Status;
    }

    nvapi! {
        /// Private PerfVfeEqu GET_CONTROL (ID 0x4C75C9FE, RM 0x2080A0B6).
        /// Seed the mask from GetInfo first; the driver echoes the readable
        /// set expanded. WORKS live on Ada 4060 Laptop (magic 85016).
        pub unsafe fn NvAPI_GPU_PerfVfeEquGetControl(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *mut NV_PERF_VFE_EQU_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Private PerfVfeEqu SET_CONTROL (ID 0x68B798C4, RM 0x2080E0B7).
        /// DANGEROUS voltage-equation write, elevation-gated (-104 without
        /// admin). Not exposed beyond the medium layer.
        pub unsafe fn NvAPI_GPU_PerfVfeEquSetControl(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *const NV_PERF_VFE_EQU_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Private PerfVfeVar GET_INFO (ID 0xB9DA41D6, RM 0x2080A0B1).
        /// Returns the variable-directory mask + per-entry type.
        pub unsafe fn NvAPI_GPU_PerfVfeVarGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_PERF_VFE_VAR_INFO) -> NvAPI_Status;
    }

    nvapi! {
        /// Private PerfVfeVar GET_CONTROL (ID 0x5D387298, RM 0x2080A0B3).
        /// WORKS live on Ada 4060 Laptop (magic 68300).
        pub unsafe fn NvAPI_GPU_PerfVfeVarGetControl(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *mut NV_PERF_VFE_VAR_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Private PerfVfeVar SET_CONTROL (ID 0x79FA23A2, RM 0x2080E0B0).
        /// DANGEROUS variable write, elevation-gated. Not exposed beyond the
        /// medium layer.
        pub unsafe fn NvAPI_GPU_PerfVfeVarSetControl(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *const NV_PERF_VFE_VAR_CONTROL) -> NvAPI_Status;
    }
}
