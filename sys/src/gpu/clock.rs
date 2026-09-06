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
    /// GetAllClocks V1 slot count: the driver stamps the 260-byte V1 layout
    /// (`4 + 64*4`) as `0x10104` — IDA-verified identical on 391.35 / 538.78 /
    /// 560.94 / 582.41 / 610.88. (288 was the V2 `extendedDomain` count; V2
    /// uses its own `NVAPI_MAX_GPU_CLOCKS`.)
    pub const NVAPI_MAX_CLOCKS_PER_GPU: usize = 64;

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

    #[cfg(test)]
    mod get_all_clocks_layout_tests {
        /// GetAllClocks (0x1bd69f49) accepted stamps, IDA-verified identical
        /// on 391.35 / 538.78 / 560.94 / 582.41 / 610.88: the V1 260-byte
        /// layout (`0x10104`) and the V2 1156-byte effective-clocks layout
        /// (`0x20484`). Keep both pinned so the `effective_clocks` V2→V1
        /// fallback cannot silently drift off the driver's accepted set.
        #[test]
        fn get_all_clocks_layout_sizes() {
            use crate::api::{NV_CLOCKS_INFO, NV_GPU_CLOCK_EFFECTIVE_INFO};
            assert_eq!(std::mem::size_of::<NV_CLOCKS_INFO>(), 260);
            assert_eq!(
                1u32 << 16 | std::mem::size_of::<NV_CLOCKS_INFO>() as u32,
                0x10104
            );
            assert_eq!(std::mem::size_of::<NV_GPU_CLOCK_EFFECTIVE_INFO>(), 1156);
            assert_eq!(
                2u32 << 16 | std::mem::size_of::<NV_GPU_CLOCK_EFFECTIVE_INFO>() as u32,
                0x20484
            );
        }
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
    // 0x07000048); re-verified across 391.35/538.78/560.94/582.41/610.88
    // (version-coverage-audit): the version check accepts EXACTLY three
    // caller magic dwords — 0x379C8 (native V3, 227784 B), 0x319C8 (V3,
    // 203208 B) and 0x119C8 (V1, 72136 B); anything else (incl. the V4
    // 0x432D0/0x832D0 the R610-era ref tool and our V4 path send) → -9
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
    //   +4  u32 min_kHz   ← R538-era; R465-era: PSTATE NUMBER (see below)
    //   +8  u32 max_kHz (bit0 = driver flag, masked off)
    //   +12 u8  pstate number (R538/V100-era); R465-era: slot index
    // (V1 aggregates mask bits BY pstate number — bit p ⇔ P{p} present;
    //  V3's mask is the raw slot mask and the pstate number rides in each
    //  record. Iterating set bits and reading record.pstate decodes BOTH.)
    //
    // GENERATION SPLIT for the header vs +12 semantics — three live decodes:
    //   R538 (538.78 IDA): +4/+8 = real kHz bounds, +12 = pstate number.
    //   V100 (2026-09-02 live): header zero, +12 = pstate number.
    //   R465 (462.96 live, GTX 1650 SUPER): +4 = pstate number (records
    //       arrive P8/P5/P3/P2/P0), +12 = slot index 0..n, +8 = 0. Trusting
    //       +12 labels the rows P0..P4 (wrong), trusting +4 as kHz yields
    //       "Min 0.008 MHz" (wrong). The per-buffer decider is
    //       perf_pstates_legacy_id_from_header; the kHz plausibility gate
    //       routes header values to the clock columns only when they are
    //       actually frequencies. The per-domain sub-table (below) is
    //       stable across all three generations.
    // ------------------------------------------------------------------

    /// V3 legacy version magic — driver-accepted value 0x319C8 (= the
    /// 203208-byte buffer's `size | 3<<16` truncated encoding; the historic
    /// 0x31A38 here was a hex typo for 203208 and every V3-legacy call -9'd
    /// into the V1 fallback).
    pub const PERF_PSTATES_INFO_PRIVATE_V3_LEGACY_MAGIC: u32 = 0x319C8;
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
    /// Re-verified on R465 (462.96, GTX 1650 SUPER, 2026-09-05): identical
    /// entry offsets/stride (P0 GPC {nominal 645000, min 300000, max 645000},
    /// P1 {645000, 300000, 2100000}); see
    /// tests/pstates_private_r465_probe.rs.
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

    /// Legacy record header (+4 min / +8 max) plausibility floor. R538-era
    /// drivers fill them with real kHz bounds; R465 (462.96 live) instead
    /// leaves small non-kHz control values there (P0..P4 → 8/5/3/2/0 — read
    /// as "Min 0.008 MHz" by anything trusting the header), and V100-era
    /// leaves them zero. Real clock floors seen in live decodes start at
    /// 135000 kHz (NVML P0-min parity), so anything below 10 MHz is not a
    /// frequency — the caller should fall back to the per-domain sub-table
    /// (stable across all three generations, see
    /// [`perf_pstates_legacy_domain_clock`]).
    pub const PERF_PSTATES_LEGACY_HEADER_MIN_PLAUSIBLE_KHZ: u32 = 10_000;

    pub fn perf_pstates_legacy_header_plausible(khz: u32) -> bool {
        khz >= PERF_PSTATES_LEGACY_HEADER_MIN_PLAUSIBLE_KHZ
    }

    /// Does header +4 carry the record's P-STATE NUMBER (instead of a kHz
    /// min)? Decided per buffer across the three decoded generations:
    ///  - R538-era: header +4/+8 are real kHz bounds (≥ 10 MHz) → the ID
    ///    rides at +12 (IDA-verified marshal) → `false`.
    ///  - R465-era (462.96 live, GTX 1650 SUPER): header +4 IS the pstate
    ///    ID — records arrive as P8/P5/P3/P2/P0 (the GPU's real pstate set)
    ///    — and +12 is the slot index 0..n, so trusting +12 mislabels every
    ///    row as P0..P4 → `true`.
    ///  - V100-era: header all-zero → ID at +12 → `false`.
    /// Rule: NO header value plausible as kHz, AND every +4 value is a valid
    /// pstate id (≤ 15), AND the set is distinct (a real id set never
    /// repeats). A single-record buffer cannot prove distinctness and stays
    /// on the +12 convention.
    pub fn perf_pstates_legacy_id_from_header(records: &[(u32, u32, u32, u8)]) -> bool {
        if records.iter().any(|&(_, mn, mx, _)| {
            perf_pstates_legacy_header_plausible(mn) || perf_pstates_legacy_header_plausible(mx)
        }) {
            return false;
        }
        let ids: std::collections::HashSet<u32> = records.iter().map(|&(_, mn, _, _)| mn).collect();
        records.iter().all(|&(_, mn, _, _)| mn <= 15) && ids.len() > 1
    }

    /// Header-gate regression for the R465 get-pstate-lock misdecode
    /// (GTX 1650 SUPER, 462.96, dumped 2026-09-05 by
    /// tests/pstates_private_r465_probe.rs → .zcode/pstate-dumps/): the V3
    /// legacy header carries small non-kHz values (P0..P4 → 8/5/3/2/0) while
    /// the real bounds live in the per-domain sub-table at the V100-derived
    /// offsets. A synthetic V3 buffer with those exact bytes must decode the
    /// sub-table and reject the header values as clock candidates.
    #[cfg(test)]
    mod legacy_header_gate_tests {
        use super::*;

        /// V3 buffer: mask 0x5b, records at 72 + 2252*bit; record0 header
        /// {type 2, "min" 8, "max" 0, pstate 0}; record6 header "min" 0;
        /// sub-table entries at +72 + 68*domain with V100 field offsets.
        fn synthetic_r465_v3() -> Vec<u8> {
            let mut buf = vec![0u8; PERF_PSTATES_INFO_PRIVATE_V3_LEGACY_LEN];
            buf[..4].copy_from_slice(&PERF_PSTATES_INFO_PRIVATE_V3_LEGACY_MAGIC.to_ne_bytes());
            buf[4..8].copy_from_slice(&0x5bu32.to_ne_bytes());
            buf[8] = 0x35; // table_version
            let put = |buf: &mut [u8], off: usize, v: u32| {
                buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
            };
            // record 0 (P0): type 2, header "min" 8 (non-kHz junk), max 0
            put(&mut buf, 72, 2);
            put(&mut buf, 76, 8);
            buf[72 + 12] = 0;
            // record 6 (P4): header "min" 0 → sub-table rescue (old behavior)
            put(&mut buf, 72 + 2252 * 6, 2);
            buf[72 + 2252 * 6 + 12] = 4;
            // P0 GPC sub-entry: nominal 645000, min 300000, max 645000, tail 9
            let e0 = 72 + 72;
            put(&mut buf, e0 + 8, 645_000);
            put(&mut buf, e0 + 12, 300_000);
            put(&mut buf, e0 + 16, 645_000);
            put(&mut buf, e0 + 40, 9);
            // P4 GPC sub-entry: min 330000, max 2130000
            let e6 = 72 + 2252 * 6 + 72;
            put(&mut buf, e6 + 8, 2_130_000);
            put(&mut buf, e6 + 12, 330_000);
            put(&mut buf, e6 + 16, 2_130_000);
            put(&mut buf, e6 + 40, 9);
            buf
        }

        #[test]
        fn r465_header_junk_never_reaches_the_clock_columns() {
            let buf = synthetic_r465_v3();
            // header values as the driver wrote them
            let (ty, min, max, pstate) = perf_pstates_legacy_record(&buf, 0);
            assert_eq!((ty, min, max, pstate), (2, 8, 0, 0));
            // the sub-table carries the real bounds, same offsets as V100
            assert_eq!(
                perf_pstates_legacy_domain_clock(&buf, 0, 0),
                Some((645_000, 300_000, 645_000))
            );
            assert_eq!(
                perf_pstates_legacy_domain_clock(&buf, 6, 0),
                Some((2_130_000, 330_000, 2_130_000))
            );
            // the gate: junk header values are implausible, real floors are not
            assert!(!perf_pstates_legacy_header_plausible(8));
            assert!(!perf_pstates_legacy_header_plausible(0));
            assert!(perf_pstates_legacy_header_plausible(135_000));
            // resolution rule the caller implements: implausible header →
            // sub-table; plausible header → header (R538-era behavior kept)
            let (_, hmin, hmax, _) = perf_pstates_legacy_record(&buf, 0);
            let clocks = perf_pstates_legacy_domain_clock(&buf, 0, 0);
            let min = if perf_pstates_legacy_header_plausible(hmin) {
                Some(hmin)
            } else {
                clocks.map(|(_, live_min, _)| live_min)
            };
            let max = if perf_pstates_legacy_header_plausible(hmax) {
                Some(hmax)
            } else {
                clocks.map(|(_, _, mx)| mx)
            };
            assert_eq!(min, Some(300_000));
            assert_eq!(max, Some(645_000));
        }

        /// R465 records arrive as P8/P5/P3/P2/P0: header +4 carries the
        /// pstate number, +12 the slot index. The decider must pick the
        /// header for that shape and reject the V100 (all-zero header) and
        /// R538 (kHz header) shapes.
        #[test]
        fn r465_pstate_ids_live_in_header_plus4() {
            // (type, header+4, header+8, +12) — dump-verbatim 1650 SUPER set
            let r465: Vec<(u32, u32, u32, u8)> = vec![
                (2, 8, 0, 0),
                (2, 5, 0, 1),
                (2, 3, 0, 2),
                (2, 2, 0, 3),
                (2, 0, 0, 4),
            ];
            assert!(perf_pstates_legacy_id_from_header(&r465));
            // V100: header all-zero → +12 stays the ID source
            let v100: Vec<(u32, u32, u32, u8)> = vec![(0, 0, 0, 0), (0, 0, 0, 1), (0, 0, 0, 2)];
            assert!(!perf_pstates_legacy_id_from_header(&v100));
            // R538: header values are real kHz → +12 stays the ID source
            let r538: Vec<(u32, u32, u32, u8)> =
                vec![(0, 300_000, 2_100_000, 0), (0, 300_000, 2_100_000, 1)];
            assert!(!perf_pstates_legacy_id_from_header(&r538));
        }
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
        /// discovery API). Stamp-audit fix (2026-09-05): the declared size was
        /// 0x9B8 (2488 B) → stamp 0x109B8, which NO driver branch accepts —
        /// every branch's version gate wants 0x109D8 (v1|2520) or a
        /// generation-larger stamp ({560+: 0x21624/0x34128/0x486AC/0x506AC}),
        /// plain equality, no version mask (IDA: 391 `*a2 != 68056`; 538/560/
        /// 582 `!= 68056 && != 136740 && != 213288`-family; 610 adds 296620/
        /// 329388). The previous "rejects all 5 IDA magics live on R575.74"
        /// note predates this size correction — v1|2520 with a full 2520-byte
        /// buffer is the smallest universally accepted form; live behaviour
        /// still unverified per-branch. Layout beyond the version dword
        /// opaque; discovery is routed through GetControl (which exposes the
        /// mask + per-domain ranges) either way. Total 0x9D8 = 2520 bytes.
        pub struct NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_INFO_PRIVATE_V1 {
            pub version: NvVersion,
            pub rest: [u8; 2516],
        }
    }

    nvversion! { @=NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_INFO_PRIVATE NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_INFO_PRIVATE_V1(1) = 0x9d8 }

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
        /// Widest R535-era stamp (369796 = 0x5A484): its response carries a
        /// 64B (512-bit) bank-1 mask window — the seed the canonical STATUS
        /// read needs to reach points ≥256 (e.g. the 4th/5th mem pstate
        /// bins at 256..258, unreachable through the gen-1 INFO's 32B
        /// 256-bit window). Ladder: modern → WIDE → LEGACY.
        pub const MAGIC_R535_WIDE: u32 = 369_796; // 0x5A484
        /// Full GetInfo stamp whitelist per branch, from IDA (the stamps are
        /// STRUCT SIZES in bytes, and the whitelist IS the ABI — anything
        /// outside it is rejected with IncompatibleStructVersion):
        /// - R391.35: `{83996}` only (sub_180124460, single compare → -9)
        /// - R535.78: `{83996, 157692, 249844, 369796}` (sub_1802702E0) —
        ///   the 475.14 (R47x) whitelist is IDENTICAL (sub_1802444E0)
        /// - R582.41/R610: the R535 set + `{493060}` (this MAGIC)
        ///
        /// The mid sizes are only needed to DECODE responses stamped with
        /// them; readers that just need the point-mask seed can send
        /// MAGIC_R535_WIDE (512-bit windows) or MAGIC_LEGACY everywhere the
        /// modern stamp is rejected.
        pub const MAGIC_R535_MID: [u32; 2] = [157_692, 249_844];
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
        // the curve domains WITHOUT their own main record block in
        // ascending ClkDomains order:
        //   Turing — only GPC as main records (single block #0..126):
        //   4 slots = XBAR @0x74/78, SYS @0x84/88, MSD @0x94/98, HOST
        //   @0xA4/A8.
        //   Ampere — XBAR promoted to a second main block (#127..253):
        //   #0..126 are base-only, and the XBAR block fills THREE
        //   slots = SYS @0x74/78, MSD @0x84/88, HOST @0x94/98 (user
        //   domain-id A/B).
        //   Ada — MSD promoted too: the XBAR block fills TWO slots =
        //   SYS @0x74/78, HOST @0x84/88 (live A/B: the 35-distinct
        //   225..1335 slot is HOST, not MSD).
        // Extension presence marker: NON-ZERO dwords at +0x2C and/or
        // +0x40 generally mean the extended section follows (base-only
        // records keep them zero). The decoder stays POSITIONAL (slot
        // k = non-main-block-domain[k], resolved by the consumer from
        // the segments present in the table) — never a generation table.
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

    /// Byte offsets into the R535-era CANONICAL GetStatus layout (stamp
    /// 300164 = 0x49484). IDA sub_180270EA0 (nvapi64_53878.dll): when the
    /// caller stamps 300164, marshal-in (sub_180258B20) zero-copies the
    /// USER buffer as the handler's internal image, so the response carries
    /// the FULL 292B per-point payload — unlike the gen-1 compaction
    /// (sub_18025A5A0 case 0x14C18) which only round-trips the +4/+8 value
    /// pair for record types 0/1 and silently DROPS the V/F fields of
    /// curve-typed records (types 3/4/7/8/12/13) — the mechanism behind
    /// 538.78's all-zero `get-private-vftable` curve output.
    ///
    /// Layout (absolute offsets): 512-bit present mask per bank @+4 /
    /// +150084 (the caller must SEED these — with stamp==300164 the driver
    /// never rewrites them), records @+580 / +150660, 292B stride, 512
    /// points per bank; 580+512*292 = 150660 and 150660+512*292 = 300164 =
    /// the stamp itself.
    ///
    /// **The 292B record is the gen7 (488B) record TRUNCATED** — the base
    /// and extended-section slots sit at the SAME offsets with the same
    /// semantics (LIVE-VERIFIED RTX A4000 / 538.78 2026-09-04: +0x24
    /// 210..1950 MHz and +0x58 450000..1237500 µV ascending V/F grids
    /// matching the public GPC VFP curve point-for-point; +0x64/+0x68
    /// mirror them at stock like gen7's current pair; ext ranges
    /// +0x74..+0xA8 populate like gen7's). Types 0/1/2 records were not
    /// exercised live on R535 — the +0x24/+0x58 read is presumed to hold
    /// for them too (it does on every gen7 generation).
    pub mod clk_vfp_status_canonical {
        /// canonical struct size = the stamp the handler accepts
        pub const SIZE: usize = 300_164;
        /// bank-1 present mask (64B = 512 bits, dword-LSB) — CALLER-SEEDED
        pub const MASK1: usize = 4;
        /// bank-1 records base (type dword @+0)
        pub const REC1: usize = 580;
        /// bank-2 present mask (64B) — CALLER-SEEDED
        pub const MASK2: usize = 150_084;
        /// bank-2 records base
        pub const REC2: usize = 150_660;
        /// per-record stride (gen7 488B record truncated at +0x124)
        pub const STRIDE: usize = 292;
        /// points per bank (mask window width)
        pub const POINTS: usize = 512;
        /// type dword @rec+0 (7/8 = curve points, same encoding as gen7)
        pub const TYPE: usize = 0;
        /// default frequency (u32 MHz @rec+0x24 — gen7 slot)
        pub const FREQ_DEFAULT_MHZ: usize = 0x24;
        /// small-typed record voltage (u32 µV @rec+0x28 — types 0/1/2,
        /// where +0x24 holds a u16 frequency)
        pub const GEN2_VOLT_UV: usize = 0x28;
        /// stock/default voltage (u32 µV @rec+0x58 — gen7 slot)
        pub const VOLTAGE_UV: usize = 0x58;
        /// current/effective frequency (u32 MHz @rec+0x64 — gen7 slot;
        /// == default at stock)
        pub const FREQ_CURRENT_MHZ: usize = 0x64;
        /// current/effective voltage (u32 µV @rec+0x68 — gen7 slot; ==
        /// default at stock)
        pub const VOLT_CURRENT_UV: usize = 0x68;
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

        /// 292B-record geometry: the gen7-aligned record slots
        /// (+0x24/+0x58/+0x64/+0x68 + ext section) tile identically in the
        /// gen3 and gen23 compactions — only the record bases, mask
        /// windows and point counts move (R582.41 sub_1801E8310 and
        /// R535.78 sub_18025A5A0 case 0x3467C agree on gen3).
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct ClkVfpGeo {
            pub rec1: usize,
            pub rec2: usize,
            pub mask1: usize,
            pub mask2: usize,
            pub points: usize,
            pub stride: usize,
        }

        /// stamp 300164 canonical — LIVE-VERIFIED (A4000/538.78)
        pub const GEO_CANONICAL: ClkVfpGeo = ClkVfpGeo {
            rec1: 580,
            rec2: 150_660,
            mask1: 4,
            mask2: 150_084,
            points: 512,
            stride: 292,
        };
        /// stamp 214652 (gen3) — IDA-derived (both branches), no live
        /// binary seen accepting it exclusively
        pub const GEO_GEN3: ClkVfpGeo = ClkVfpGeo {
            rec1: 100,
            rec2: 74_656,
            mask1: 4,
            mask2: 74_560,
            points: 255,
            stride: 292,
        };
        /// stamp 1525252 (gen23, R582-only rung) — the seed only covers the
        /// first 512 points (no wide-mask INFO below gen7 on those
        /// branches), so points 512.. decode as absent
        pub const GEO_GEN23: ClkVfpGeo = ClkVfpGeo {
            rec1: 772,
            rec2: 599_556,
            mask1: 4,
            mask2: 598_788,
            points: 2_048,
            stride: 292,
        };
    }

    /// Stamp 158200 (gen2) layout: 255 × 620B records @+100, bank-0 only
    /// (100 + 255×620 = 158200 = the stamp). Lossy like gen-1 — type 0/1
    /// records keep freq u16 @+0x24 + volt u32 @+0x28, types 3/4 a partial
    /// payload at +0x58.., and curve-typed records (7/8) are DROPPED by
    /// the driver's compaction (R582.41 sub_1801E8310 case 0x269F8).
    pub mod clk_vfp_status_gen2 {
        pub const SIZE: usize = 158_200;
        /// present mask (32B = 256 bits used of the 64B window) — CALLER-SEEDED
        pub const MASK1: usize = 4;
        /// records base (type dword @+0)
        pub const REC1: usize = 100;
        /// per-record stride
        pub const STRIDE: usize = 620;
        /// points per bank (single bank)
        pub const POINTS: usize = 255;
        /// type dword @rec+0
        pub const TYPE: usize = 0;
        /// freq u16 MHz @rec+0x24 — types 0/1
        pub const FREQ_MHZ: usize = 0x24;
        /// volt u32 µV @rec+0x28 — types 0/1
        pub const VOLT_UV: usize = 0x28;

        /// geometry for the shared geo accessors (stride 620, bank-0 only
        /// — mask2 = 0 skips the second window in seed_geo_header)
        pub const GEO: super::clk_vfp_status_canonical::ClkVfpGeo =
            super::clk_vfp_status_canonical::ClkVfpGeo {
                rec1: 100,
                rec2: 0,
                mask1: 4,
                mask2: 0,
                points: 255,
                stride: 620,
            };
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
        /// 2×2048-record layout (live-verified). As everywhere in this
        /// family the stamp is the STRUCT SIZE in bytes, and the accepted
        /// whitelist per branch (IDA) is the ABI:
        /// - R391.35: `{85016}` only (sub_180124A40, single compare → -9)
        /// - R535.78: `{85016, 158200, 214652, 300164}` (sub_180270EA0)
        /// - R582.41: the R535 set + `{2000388}` (this MAGIC)
        ///
        /// Era map (driver-generation taxonomy: R391 / R471 / R53x / R560+):
        /// era-1 R391 anchors on gen-1 alone, era-3 R53x accepts the four
        /// sizes below, era-4 R560+ adds this 488B gen7 layout. **Era-2
        /// R471 is UNSCANNED** (no binary on hand) — the reader degrades to
        /// MAGIC_LEGACY there, which era-1 and era-3 both anchor on; before
        /// trusting any non-legacy decode on a 471 install, run
        /// `cargo test -p nvapi --test r535_stamp_probe -- --nocapture
        /// --ignored` (the probe walks every stamp in this set) and pin the
        /// whitelist from its output.
        pub const MAGIC: u32 = 2000388;
        /// R535-era CANONICAL stamp (300164 = 0x49484): marshal-in
        /// (sub_180258B20) zero-copies the user buffer as the handler's
        /// internal image, so the response keeps the FULL 292B per-point
        /// payload — the gen-1 compaction (MAGIC_LEGACY) silently drops the
        /// V/F fields of curve-typed records (types ≥ 2), which is why
        /// 538.78 `get-private-vftable` rendered an all-zero curve. Prefer
        /// this over MAGIC_LEGACY whenever the modern stamp is rejected.
        /// Layout: `clk_vfp_status_canonical`.
        pub const MAGIC_R535_CANONICAL: u32 = 300_164; // 0x49484
        /// gen23 mid stamp (1525252 = 0x174604, R582-only): 2048-pt, 292B
        /// records at the gen7-aligned slots (bases +772/+599556, masks
        /// +4/+598788). Decodable via GEO_GEN23.
        pub const MAGIC_R582_MID: u32 = 1_525_252; // 0x174604
        /// Mid sizes accepted by R535/R582 GetStatus alongside
        /// MAGIC_LEGACY/MAGIC_R535_CANONICAL (each expands through the same
        /// lossy per-gen compaction; only needed to DECODE responses
        /// stamped with them, which this reader never sends).
        pub const MAGIC_R535_MID: [u32; 2] = [158_200, 214_652]; // 0x269F8 / 0x3467C
        /// Legacy magic (R391.35): smallest of the accepted set, 85016B.
        /// Old drivers reject the R610 2000388 stamp with -9; this one
        /// succeeds (live-verified GT730/391.35: status=0). WARNING: the
        /// gen-1 wire format only carries +4/+8 value pairs for record
        /// types 0/1 — on drivers whose kernel emits curve-typed (type 8)
        /// records the values are lost in the driver's own compaction; use
        /// MAGIC_R535_CANONICAL there.
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

        // ---- R535-era canonical layout (stamp 300164, see
        // `clk_vfp_status_canonical`) — the buffers are SMALLER than this
        // struct's modern size, so every accessor is bounds-checked through
        // `off()` and simply returns None past the canonical end. ----

        fn canonical_rec_base(bank: usize, idx: usize) -> Option<usize> {
            if bank > 1 || idx >= clk_vfp_status_canonical::POINTS {
                return None;
            }
            Some(
                if bank == 0 {
                    clk_vfp_status_canonical::REC1
                } else {
                    clk_vfp_status_canonical::REC2
                } + clk_vfp_status_canonical::STRIDE * idx,
            )
        }

        /// Canonical present-bit test over the caller-seeded 512-bit mask
        /// windows (@+4 / +150084). With stamp 300164 the driver never
        /// rewrites the masks, so this reads back exactly what was seeded.
        pub fn canonical_point_present(&self, bank: usize, idx: usize) -> Option<bool> {
            if bank > 1 || idx >= clk_vfp_status_canonical::POINTS {
                return None;
            }
            let mask_base = if bank == 0 {
                clk_vfp_status_canonical::MASK1
            } else {
                clk_vfp_status_canonical::MASK2
            };
            let dword = self.u32_at(mask_base + 4 * (idx >> 5))?;
            Some(dword & (1 << (idx & 31)) != 0)
        }

        /// Canonical record type dword (u32 @rec+0).
        pub fn canonical_type(&self, bank: usize, idx: usize) -> Option<u32> {
            let base = Self::canonical_rec_base(bank, idx)?;
            self.u32_at(base + clk_vfp_status_canonical::TYPE)
        }

        /// Canonical default frequency (u32 MHz @rec+0x24 — the gen7 slot;
        /// LIVE-VERIFIED A4000/538.78 against the public GPC VFP curve).
        pub fn canonical_freq_default_mhz(&self, bank: usize, idx: usize) -> Option<u32> {
            let base = Self::canonical_rec_base(bank, idx)?;
            self.u32_at(base + clk_vfp_status_canonical::FREQ_DEFAULT_MHZ)
        }

        /// Canonical current/effective frequency (u32 MHz @rec+0x64 — the
        /// gen7 slot; equals the default at stock).
        pub fn canonical_freq_current_mhz(&self, bank: usize, idx: usize) -> Option<u32> {
            let base = Self::canonical_rec_base(bank, idx)?;
            self.u32_at(base + clk_vfp_status_canonical::FREQ_CURRENT_MHZ)
        }

        /// Canonical voltage (u32 µV @rec+0x58 — the gen7 slot;
        /// LIVE-VERIFIED A4000/538.78: the 6.25 mV public grid).
        pub fn canonical_volt_uv(&self, bank: usize, idx: usize) -> Option<u32> {
            let base = Self::canonical_rec_base(bank, idx)?;
            self.u32_at(base + clk_vfp_status_canonical::VOLTAGE_UV)
        }

        /// Canonical current/effective voltage (u32 µV @rec+0x68 — the
        /// gen7 slot; equals the default at stock).
        pub fn canonical_volt_current_uv(&self, bank: usize, idx: usize) -> Option<u32> {
            let base = Self::canonical_rec_base(bank, idx)?;
            self.u32_at(base + clk_vfp_status_canonical::VOLT_CURRENT_UV)
        }

        /// Raw u32 at `offset` inside the canonical record — used for the
        /// gen7-aligned ext-section markers (+0x2C/+0x40) and per-domain
        /// current slots (+0x74+0x10*k).
        pub fn canonical_raw_dword(&self, bank: usize, idx: usize, offset: usize) -> Option<u32> {
            let base = Self::canonical_rec_base(bank, idx)?;
            self.u32_at(base + offset)
        }

        /// The full 292-byte canonical record for point (bank, idx) —
        /// diagnostic (--dump-records) and live-calibration source.
        pub fn canonical_raw_record(&self, bank: usize, idx: usize) -> Option<&[u8]> {
            let base = Self::canonical_rec_base(bank, idx)?;
            let off = self.off(base, clk_vfp_status_canonical::STRIDE)?;
            self.rest.get(off..off + clk_vfp_status_canonical::STRIDE)
        }

        /// Seed the canonical 512-bit mask windows (+4/+150084, 64B each)
        /// from a GetInfo block's point masks. The modern INFO mask regions
        /// are 256B/bank; the canonical window is their first 64B (512
        /// points) — every generation so far lives inside that. With the
        /// LEGACY GetInfo stamp the response only guarantees a 160-bit
        /// window at +4; the tail bytes are typically zero there, which
        /// simply reports those points as absent.
        pub fn seed_canonical_header(
            &mut self,
            info: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1,
        ) {
            for (dst_abs, src_abs) in [
                (clk_vfp_status_canonical::MASK1, clk_vfp_info::MASK1),
                (clk_vfp_status_canonical::MASK2, clk_vfp_info::MASK2),
            ] {
                let dst = self.off_mut(dst_abs, 64).unwrap_or(0);
                let src = info.off(src_abs, 64).unwrap_or(0);
                let n = 64.min(self.rest.len() - dst).min(info.rest.len() - src);
                self.rest[dst..dst + n].copy_from_slice(&info.rest[src..src + n]);
            }
        }

        // ---- geometry-parameterized 292B-record accessors (canonical /
        // gen3 / gen23 share the gen7-aligned field slots) ----

        fn geo_rec_base(
            &self,
            geo: &clk_vfp_status_canonical::ClkVfpGeo,
            bank: usize,
            idx: usize,
        ) -> Option<usize> {
            if bank > 1 || idx >= geo.points {
                return None;
            }
            Some((if bank == 0 { geo.rec1 } else { geo.rec2 }) + geo.stride * idx)
        }

        /// Present-bit test over the geometry's caller-seeded mask windows.
        pub fn geo_point_present(
            &self,
            geo: &clk_vfp_status_canonical::ClkVfpGeo,
            bank: usize,
            idx: usize,
        ) -> Option<bool> {
            if bank > 1 || idx >= geo.points {
                return None;
            }
            let mask_base = if bank == 0 { geo.mask1 } else { geo.mask2 };
            let dword = self.u32_at(mask_base + 4 * (idx >> 5))?;
            Some(dword & (1 << (idx & 31)) != 0)
        }

        /// Record type dword (u32 @rec+0).
        pub fn geo_type(
            &self,
            geo: &clk_vfp_status_canonical::ClkVfpGeo,
            bank: usize,
            idx: usize,
        ) -> Option<u32> {
            let base = self.geo_rec_base(geo, bank, idx)?;
            self.u32_at(base + clk_vfp_status_canonical::TYPE)
        }

        /// Default frequency (u32 MHz @rec+0x24 — curve-typed records).
        pub fn geo_freq_default_mhz(
            &self,
            geo: &clk_vfp_status_canonical::ClkVfpGeo,
            bank: usize,
            idx: usize,
        ) -> Option<u32> {
            let base = self.geo_rec_base(geo, bank, idx)?;
            self.u32_at(base + clk_vfp_status_canonical::FREQ_DEFAULT_MHZ)
        }

        /// Frequency for SMALL-typed records (u16 MHz @rec+0x24 — the
        /// adjacent +0x28 dword is the voltage, so a u32 read here would
        /// be contaminated).
        pub fn geo_freq_small_mhz(
            &self,
            geo: &clk_vfp_status_canonical::ClkVfpGeo,
            bank: usize,
            idx: usize,
        ) -> Option<u32> {
            let base = self.geo_rec_base(geo, bank, idx)?;
            let off = self.off(base + clk_vfp_status_canonical::FREQ_DEFAULT_MHZ, 2)?;
            self.rest
                .get(off..off + 2)
                .and_then(|s| s.try_into().ok())
                .map(u16::from_le_bytes)
                .map(u32::from)
        }

        /// Voltage for SMALL-typed records (u32 µV @rec+0x28).
        pub fn geo_volt_small_uv(
            &self,
            geo: &clk_vfp_status_canonical::ClkVfpGeo,
            bank: usize,
            idx: usize,
        ) -> Option<u32> {
            let base = self.geo_rec_base(geo, bank, idx)?;
            self.u32_at(base + clk_vfp_status_canonical::GEN2_VOLT_UV)
        }

        /// Current/effective frequency (u32 MHz @rec+0x64 — curve types).
        pub fn geo_freq_current_mhz(
            &self,
            geo: &clk_vfp_status_canonical::ClkVfpGeo,
            bank: usize,
            idx: usize,
        ) -> Option<u32> {
            let base = self.geo_rec_base(geo, bank, idx)?;
            self.u32_at(base + clk_vfp_status_canonical::FREQ_CURRENT_MHZ)
        }

        /// Current/effective voltage (u32 µV @rec+0x68 — curve types).
        pub fn geo_volt_current_uv(
            &self,
            geo: &clk_vfp_status_canonical::ClkVfpGeo,
            bank: usize,
            idx: usize,
        ) -> Option<u32> {
            let base = self.geo_rec_base(geo, bank, idx)?;
            self.u32_at(base + clk_vfp_status_canonical::VOLT_CURRENT_UV)
        }

        /// Raw u32 at `offset` inside the geometry's record (curve voltage
        /// +0x58, ext markers +0x2C/+0x40, per-domain slots +0x74+0x10*k).
        pub fn geo_raw_dword(
            &self,
            geo: &clk_vfp_status_canonical::ClkVfpGeo,
            bank: usize,
            idx: usize,
            offset: usize,
        ) -> Option<u32> {
            let base = self.geo_rec_base(geo, bank, idx)?;
            self.u32_at(base + offset)
        }

        /// The full 292-byte record for point (bank, idx).
        pub fn geo_raw_record(
            &self,
            geo: &clk_vfp_status_canonical::ClkVfpGeo,
            bank: usize,
            idx: usize,
        ) -> Option<&[u8]> {
            let base = self.geo_rec_base(geo, bank, idx)?;
            let off = self.off(base, geo.stride)?;
            self.rest.get(off..off + geo.stride)
        }

        /// Seed a geometry's mask windows (64B each) from a GetInfo block.
        /// Windows at offset 0 are skipped (single-bank geometries).
        pub fn seed_geo_header(
            &mut self,
            info: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1,
            geo: &clk_vfp_status_canonical::ClkVfpGeo,
        ) {
            for (dst_abs, src_abs) in [
                (geo.mask1, clk_vfp_info::MASK1),
                (geo.mask2, clk_vfp_info::MASK2),
            ] {
                if dst_abs == 0 {
                    continue;
                }
                let dst = self.off_mut(dst_abs, 64).unwrap_or(0);
                let src = info.off(src_abs, 64).unwrap_or(0);
                let n = 64.min(self.rest.len() - dst).min(info.rest.len() - src);
                self.rest[dst..dst + n].copy_from_slice(&info.rest[src..src + n]);
            }
        }
    }

    /// Layout canaries (compile-time): the canonical region must tile
    /// exactly — bank-1 records end where the bank-2 mask begins, bank-2
    /// records end at the stamp size, and the whole region fits inside the
    /// modern-sized STATUS buffer it is read through.
    const _: () = assert!(
        clk_vfp_status_canonical::REC1
            + clk_vfp_status_canonical::POINTS * clk_vfp_status_canonical::STRIDE
            == clk_vfp_status_canonical::MASK2
    );
    const _: () = assert!(
        clk_vfp_status_canonical::REC2
            + clk_vfp_status_canonical::POINTS * clk_vfp_status_canonical::STRIDE
            == clk_vfp_status_canonical::SIZE
    );
    const _: () = assert!(clk_vfp_status_canonical::SIZE <= 2_000_388 + 4);

    #[cfg(test)]
    mod clk_vfp_canonical_tests {
        use super::*;

        fn put_u32(
            status: &mut NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1,
            abs: usize,
            v: u32,
        ) {
            let o = abs - 4;
            status.rest[o..o + 4].copy_from_slice(&v.to_le_bytes());
        }

        /// The nvstruct buffer this reader allocates is the gen7/modern
        /// size — pin it so a struct change cannot silently shrink the
        /// canonical window below the accessor bounds.
        #[test]
        fn canonical_fits_modern_buffer() {
            assert_eq!(
                size_of::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1>(),
                2_000_388
            );
            assert!(
                clk_vfp_status_canonical::SIZE
                    <= size_of::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1>()
            );
        }

        /// Synthetic type-8 (curve) + type-1 (small) record decode at the
        /// R535 canonical slots (gen7-aligned: +0x24/+0x58/+0x64/+0x68),
        /// plus present-bit and absence behavior.
        #[test]
        fn canonical_record_decode() {
            let mut s = unsafe {
                let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1>::new_zeroed();
                let mut b = b.assume_init();
                b.version = NvVersion::with_version(
                    NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1::MAGIC_R535_CANONICAL,
                );
                b
            };
            // present bits 0 (curve) and 1 (small) in bank 0; bit 0 in bank 1
            put_u32(&mut s, clk_vfp_status_canonical::MASK1, 0b11);
            put_u32(&mut s, clk_vfp_status_canonical::MASK2, 1);
            // point 0: curve record
            let r0 = clk_vfp_status_canonical::REC1;
            put_u32(&mut s, r0, 8);
            put_u32(&mut s, r0 + clk_vfp_status_canonical::FREQ_DEFAULT_MHZ, 405);
            put_u32(&mut s, r0 + clk_vfp_status_canonical::VOLTAGE_UV, 450_000);
            put_u32(&mut s, r0 + clk_vfp_status_canonical::FREQ_CURRENT_MHZ, 435);
            put_u32(
                &mut s,
                r0 + clk_vfp_status_canonical::VOLT_CURRENT_UV,
                450_000,
            );
            // point 1: small record — same gen7-aligned slots
            let r1 = clk_vfp_status_canonical::REC1 + clk_vfp_status_canonical::STRIDE;
            put_u32(&mut s, r1, 1);
            put_u32(&mut s, r1 + clk_vfp_status_canonical::FREQ_DEFAULT_MHZ, 648);
            put_u32(&mut s, r1 + clk_vfp_status_canonical::VOLTAGE_UV, 810_000);
            // bank-2 point 0
            let r2 = clk_vfp_status_canonical::REC2;
            put_u32(&mut s, r2, 7);
            put_u32(&mut s, r2 + clk_vfp_status_canonical::FREQ_DEFAULT_MHZ, 210);

            assert!(s.canonical_point_present(0, 0).unwrap());
            assert!(s.canonical_point_present(0, 1).unwrap());
            assert!(!s.canonical_point_present(0, 2).unwrap());
            assert_eq!(s.canonical_type(0, 0), Some(8));
            assert_eq!(s.canonical_type(0, 1), Some(1));
            // default and current pairs read independently
            assert_eq!(s.canonical_volt_uv(0, 0), Some(450_000));
            assert_eq!(s.canonical_volt_uv(0, 1), Some(810_000));
            assert_eq!(s.canonical_volt_current_uv(0, 0), Some(450_000));
            assert_eq!(s.canonical_freq_default_mhz(0, 0), Some(405));
            assert_eq!(s.canonical_freq_current_mhz(0, 0), Some(435));
            assert_eq!(s.canonical_freq_default_mhz(0, 1), Some(648));
            // bank 2 window and records are independent
            assert!(s.canonical_point_present(1, 0).unwrap());
            assert_eq!(s.canonical_freq_default_mhz(1, 0), Some(210));
            assert_eq!(s.canonical_raw_record(0, 0).unwrap().len(), 292);
            // past the 512-point canonical window → None (bounds-checked)
            assert_eq!(s.canonical_point_present(0, 512), None);
        }

        /// gen3 geometry (214652): 292B records @+100/+74656 decode with
        /// the same gen7-aligned slots as the canonical geometry.
        #[test]
        fn gen3_geo_record_decode() {
            let mut s = unsafe {
                let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1>::new_zeroed();
                let mut b = b.assume_init();
                b.version = NvVersion::with_version(214_652);
                b
            };
            let geo = clk_vfp_status_canonical::GEO_GEN3;
            put_u32(&mut s, geo.mask1, 0b11);
            // record 0: curve type at the gen3 base
            let r0 = geo.rec1;
            put_u32(&mut s, r0, 8);
            put_u32(&mut s, r0 + clk_vfp_status_canonical::FREQ_DEFAULT_MHZ, 405);
            put_u32(&mut s, r0 + clk_vfp_status_canonical::VOLTAGE_UV, 450_000);
            assert!(s.geo_point_present(&geo, 0, 0).unwrap());
            assert_eq!(s.geo_type(&geo, 0, 0), Some(8));
            assert_eq!(s.geo_freq_default_mhz(&geo, 0, 0), Some(405));
            assert_eq!(
                s.geo_raw_dword(&geo, 0, 0, clk_vfp_status_canonical::VOLTAGE_UV),
                Some(450_000)
            );
            assert_eq!(s.geo_raw_record(&geo, 0, 0).unwrap().len(), 292);
            // record region must tile into the bank-2 mask
            assert_eq!(geo.rec1 + geo.points * geo.stride, geo.mask2);
        }

        /// gen2 (158200): 620B records @+100; small types keep the
        /// +0x24 u16 freq / +0x28 u32 volt pair and the region tiles to
        /// the stamp size.
        #[test]
        fn gen2_lossy_record_decode() {
            let mut s = unsafe {
                let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1>::new_zeroed();
                let mut b = b.assume_init();
                b.version = NvVersion::with_version(158_200);
                b
            };
            let geo = clk_vfp_status_gen2::GEO;
            put_u32(&mut s, geo.mask1, 0b11);
            let r0 = geo.rec1;
            put_u32(&mut s, r0, 1);
            let f = r0 + clk_vfp_status_gen2::FREQ_MHZ;
            s.rest[f - 4..f - 2].copy_from_slice(&405u16.to_le_bytes());
            put_u32(&mut s, r0 + clk_vfp_status_gen2::VOLT_UV, 450_000);
            assert!(s.geo_point_present(&geo, 0, 0).unwrap());
            assert_eq!(s.geo_type(&geo, 0, 0), Some(1));
            assert_eq!(s.geo_freq_small_mhz(&geo, 0, 0), Some(405));
            assert_eq!(s.geo_volt_small_uv(&geo, 0, 0), Some(450_000));
            // 100 + 255*620 == 158200 (the stamp IS the region span)
            assert_eq!(
                geo.rec1 + geo.points * geo.stride,
                clk_vfp_status_gen2::SIZE
            );
        }

        /// seed_canonical_header copies the first 64B of each GetInfo bank
        /// mask into the canonical windows; the driver's dword-LSB mask
        /// convention means the byte copy preserves bit semantics.
        #[test]
        fn canonical_seed_from_info() {
            // NB: never construct the big structs by value in a test — the
            // literal materializes ~2 MB on the (1 MB) test-thread stack.
            // Zeroed heap allocation + a version stamp, as in production.
            let mut info = unsafe {
                let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1>::new_zeroed();
                let mut b = b.assume_init();
                b.version = NvVersion::with_version(
                    NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1::MAGIC,
                );
                b
            };
            let put = |info: &mut NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1,
                       abs: usize,
                       v: u32| {
                let o = abs - 4;
                info.rest[o..o + 4].copy_from_slice(&v.to_le_bytes());
            };
            put(&mut info, clk_vfp_info::MASK1, 0x8000_0001);
            put(&mut info, clk_vfp_info::MASK1 + 4, 0x0000_00FF);
            put(&mut info, clk_vfp_info::MASK2, 0x0000_0001);

            let mut status = unsafe {
                let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1>::new_zeroed();
                let mut b = b.assume_init();
                b.version = NvVersion::with_version(
                    NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1::MAGIC_R535_CANONICAL,
                );
                b
            };
            status.seed_canonical_header(&info);
            assert!(status.canonical_point_present(0, 0).unwrap());
            assert!(!status.canonical_point_present(0, 1).unwrap());
            assert!(status.canonical_point_present(0, 31).unwrap());
            // second mask dword (0x0000_00FF) covers bits 32..39 only
            assert!(status.canonical_point_present(0, 39).unwrap());
            assert!(!status.canonical_point_present(0, 63).unwrap());
            assert!(!status.canonical_point_present(0, 64).unwrap());
            assert!(status.canonical_point_present(1, 0).unwrap());
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
        /// CONFLICT (IDA 2026-09-04): the 391.35 binary's GetControl
        /// whitelist is `{82976}` ONLY (sub_180123E50: `*a2 != 82976 → -9`)
        /// — 90116 should never pass there. The live note above may have
        /// observed a different 391.xx build or the snapshot fallback;
        /// re-verify on hardware before relying on this stamp. The
        /// reader's legacy branch already prefers MAGIC_SNAPSHOT (accepted
        /// by R391 AND R535 per IDA), so nothing depends on this stamp.
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
        /// Present-extent observed on V100 (132 points); the gen1 control
        /// REQUEST mask window is 16 dwords (64B = 512 bits) on every
        /// branch, so the bound is widened to the window — enumeration
        /// loops still bound by the actual present bits.
        pub const LEGACY_POINTS: usize = 512;
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

        /// Legacy control request mask, seeded from the GetInfo mask
        /// window (same LE-bitfield format). Width 64B: the gen1 control
        /// REQUEST carries a 16-dword bitmap on every branch (R582.41
        /// sub_1801E61C0 marshal-in; R535.78 scratch packing), and the
        /// seed always lands on a zeroed buffer — bits beyond a kernel's
        /// present extent are zero, and zero bits are no-ops in the
        /// driver's per-bit processing, so widening past the V100-observed
        /// 17B present extent (132 points) changes nothing there.
        pub fn legacy_seed_masks_from_info(
            &mut self,
            info: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1,
        ) {
            let src = info.off(clk_vfp_info::MASK1, 64).unwrap_or(0);
            let dst = self.off_mut(clk_vfp_control::LEGACY_MASK, 64).unwrap_or(0);
            let n = 64.min(self.rest.len() - dst).min(info.rest.len() - src);
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

    // ------------------------------------------------------------------
    // ClockClkVoltController(s) family — per-clock-domain VOLTAGE controller
    // object tree (the freq sibling 0x58F4F4C1/0x45C064D5/0xFD7C0AC3/
    // 0xD9BE5BF9 reads the same tree shape for FREQUENCY controllers).
    // RE'd 2026-09-06 from reverse/version-audit/nvapi64_impl_61088.dll
    // (R610.88; handler VAs identical to the archive's R610.74 evidence
    // table) with an R462.96 cross-check:
    //
    //   ID          handler(610)    magic   size(B) RM(610)      462.96
    //   GetInfo     0xEC6FCD0B  0x18021C330 0x10CD4 3284 0x20809033  0x20801033 esc 0x070004A
    //   GetStatus   0x8506C02E  0x18021C9A0 0x10BC8 3016 0x20809034
    //   GetControl  0xDD41633C  0x18021D010 0x10C48 3144 0x20809035  (esc 610 = 0x0700049)
    //   SetControl  0xF9833206  0x18021D640 0x10C48 3144 0x2080D036
    //
    // Single version magic per call, PLAIN equality (`cmp dword [user],imm`
    // → -9): all three are (1<<16)|size — the first private clock structs
    // whose size exceeds 0x0B00, so the magic's low half IS the size and the
    // gate is exact. Escape 0x0700049 (same family as VfPoints); 462.96
    // shifts to 0x070004A + RM 0x208010xx inside the handler — the USER
    // layouts are byte-identical across both generations, so one set of
    // structs serves both. SetControl passes the elevation gate
    // (sub_18038FE40) FIRST: -104 (0xFFFFFF98) without admin, before any
    // validation. GET_STATUS / GET_CONTROL are MASK-SEEDED at +4 (ClkDomains
    // GetControl seeds at +8; this family at +4); records are BIT-SPARSE by
    // bit index. GetInfo takes no input mask — the driver fills it at +4.
    //
    // User-buffer layouts (record-relative; GET unpack / SET pack mirrored,
    // byte-for-byte cross-checked between the two handlers):
    //   INFO    rec@0x54 + bit*0x64: active u32@0 · u8@4 · u8@5 · u8@6 ·
    //           u16@8 · u32@0xC · u32@0x10 · u32@0x14 · i32@0x38 (wire i16
    //           sign-extended) · i32@0x3C (wire i16 sign-extended) · u32@0x40
    //   STATUS  rec@0x148 + bit*0x54: active u32@0 · u32@4 · u32@0x28 ·
    //           u32@0x2C · u32@0x30 (header echoes 4×u32 @+8..+0x14 and
    //           4×u32 @+0x88..+0x94 from the escape reply)
    //   CONTROL rec@0x48 + bit*0x60: active u32@0 · u8@4 · u16@6 · u32@8 ·
    //           u32@0xC · u32@0x10 · u32@0x34 · u32@0x38 · u32@0x3C
    //   (Closure: BASE + 32*stride + 4 == magic size field for all three.)
    //
    // Live 462.96 (GTX 1650 SUPER): all four QI-resolve; INFO and CONTROL
    // ACCEPT the 610 magics and return OK — but the VOLTAGE controller table
    // is EMPTY on that part (INFO mask 0 with zeroed records; CONTROL with a
    // broadcast seed → -1; STATUS → -1). The freq sibling DOES return
    // records there (mask 0x7, rec0 {u16@6:15, u32@8:327680 ≈ idle-GPC kHz,
    // i32@0x3C:−18750 dynamic}), so the object tree works — this GPU just
    // exposes no voltage controllers. Laptop 610 parts are the real
    // verification surface for non-empty data and field semantics.
    // ------------------------------------------------------------------

    /// Byte offsets into [`NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_CONTROL`] —
    /// the mask-seeded GET/SET block. All constants are START-biased
    /// ABSOLUTE struct offsets (the field occupies `[c, c+width)`;
    /// `rest[c-4..c]` addresses it because `rest` begins at struct +4) —
    /// the same convention as the ClkDomains `clk_ctrl_entry` above.
    pub mod clk_volt_ctrl_entry {
        /// input/echoed controller mask (u32 @+4)
        pub const MASK: usize = 4;
        /// first bit-sparse record base (absolute)
        pub const BASE: usize = 0x48;
        /// per-bit record stride
        pub const STRIDE: usize = 0x60;
        /// rec+0x00: active flag (u32; SET only commits records with ==1
        /// and stamps the wire type byte to 1 for them)
        pub const ACTIVE: usize = 0x00;
        /// rec+0x04: u8 field
        pub const B04: usize = 0x04;
        /// rec+0x06: u16 field
        pub const U16_06: usize = 0x06;
        /// rec+0x08: u32 field
        pub const U32_08: usize = 0x08;
        /// rec+0x0C: u32 field
        pub const U32_0C: usize = 0x0C;
        /// rec+0x10: u32 field
        pub const U32_10: usize = 0x10;
        /// rec+0x34: extended u32 (wire-packed ONLY when active==1)
        pub const U32_34: usize = 0x34;
        /// rec+0x38: extended u32 (packed only when active==1)
        pub const U32_38: usize = 0x38;
        /// rec+0x3C: extended u32 (packed only when active==1; on the freq
        /// sibling this slot is DYNAMIC — hypothesis: live offset value)
        pub const U32_3C: usize = 0x3C;
    }

    /// (START-biased record-relative offset, byte width) of every CONTROL
    /// record field, in the canonical nine-field order used by
    /// [`NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_CONTROL::record_full`] /
    /// [`NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_CONTROL::set_record_full`]:
    /// `[active u32, b04 u8, u16_06, u32_08, u32_0C, u32_10, u32_34, u32_38, u32_3C]`.
    const CLK_VOLT_CTRL_FIELDS: [(usize, usize); 9] = [
        (clk_volt_ctrl_entry::ACTIVE, 4),
        (clk_volt_ctrl_entry::B04, 1),
        (clk_volt_ctrl_entry::U16_06, 2),
        (clk_volt_ctrl_entry::U32_08, 4),
        (clk_volt_ctrl_entry::U32_0C, 4),
        (clk_volt_ctrl_entry::U32_10, 4),
        (clk_volt_ctrl_entry::U32_34, 4),
        (clk_volt_ctrl_entry::U32_38, 4),
        (clk_volt_ctrl_entry::U32_3C, 4),
    ];

    nvstruct! {
        /// GET/SET block for the private voltage-controller family
        /// (RM 0x20809035 / 0x2080D036, IDs 0xDD41633C / 0xF9833206).
        /// Seed [`clk_volt_ctrl_entry::MASK`] at +4 before GET_CONTROL;
        /// derive the REAL controller set from records with a nonzero
        /// `active` u32 (the driver echoes the seed, not the populated
        /// set). Total 0x10C48 = (1<<16)|3144.
        pub struct NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_CONTROL_V1 {
            pub version: NvVersion,
            /// +4 .. +3144: mask@+4, opaque header, 32×0x60 records @0x48
            pub rest: [u8; 3140],
        }
    }

    nvversion! { @=NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_CONTROL NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_CONTROL_V1(1) = 0xc48 }

    /// Byte offsets into [`NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_INFO`] — the
    /// descriptor read (driver fills the mask; no input seeding). Constants
    /// are START-biased absolute struct offsets.
    pub mod clk_volt_info_entry {
        pub const MASK: usize = 4;
        pub const BASE: usize = 0x54;
        pub const STRIDE: usize = 0x64;
        pub const ACTIVE: usize = 0x00;
        pub const B04: usize = 0x04;
        pub const B05: usize = 0x05;
        pub const B06: usize = 0x06;
        pub const U16_08: usize = 0x08;
        pub const U32_0C: usize = 0x0C;
        pub const U32_10: usize = 0x10;
        pub const U32_14: usize = 0x14;
        /// wire i16 @rec+0x38, sign-extended by the handler
        pub const I32_38: usize = 0x38;
        /// wire i16 @rec+0x3C, sign-extended by the handler
        pub const I32_3C: usize = 0x3C;
        pub const U32_40: usize = 0x40;
    }

    nvstruct! {
        /// GET_INFO descriptor block (RM 0x20809033, ID 0xEC6FCD0B).
        /// Magic 0x10CD4 = (1<<16)|3284; the driver fills mask@+4 and the
        /// per-bit records (record gate: u32@rec == 1).
        pub struct NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_INFO_V1 {
            pub version: NvVersion,
            /// +4 .. +3284: mask@+4, header, 32×0x64 records @0x54
            pub rest: [u8; 3280],
        }
    }

    nvversion! { @=NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_INFO NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_INFO_V1(1) = 0xcd4 }

    /// Byte offsets into [`NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_STATUS`] — the
    /// live-status read (MASK-SEEDED at +4, like CONTROL). Constants are
    /// START-biased absolute struct offsets.
    pub mod clk_volt_status_entry {
        pub const MASK: usize = 4;
        pub const BASE: usize = 0x148;
        pub const STRIDE: usize = 0x54;
        pub const ACTIVE: usize = 0x00;
        pub const U32_04: usize = 0x04;
        pub const U32_28: usize = 0x28;
        pub const U32_2C: usize = 0x2C;
        pub const U32_30: usize = 0x30;
    }

    nvstruct! {
        /// GET_STATUS block (RM 0x20809034, ID 0x8506C02E).
        /// Magic 0x10BC8 = (1<<16)|3016; seed mask@+4. On a part with no
        /// voltage controllers (live 462.96) this call returns
        /// NVAPI_ERROR(-1) regardless of seed.
        pub struct NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_STATUS_V1 {
            pub version: NvVersion,
            /// +4 .. +3016: mask@+4, header echo, 32×0x54 records @0x148
            pub rest: [u8; 3012],
        }
    }

    nvversion! { @=NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_STATUS NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_STATUS_V1(1) = 0xbc8 }

    impl NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_CONTROL {
        /// Seed the input mask at +4 (call before GET_CONTROL).
        pub fn set_mask(&mut self, mask: u32) {
            let o = clk_volt_ctrl_entry::MASK - 4;
            self.rest[o..o + 4].copy_from_slice(&mask.to_le_bytes());
        }

        /// Read the mask at +4 (echoed on GET).
        pub fn mask(&self) -> u32 {
            let o = clk_volt_ctrl_entry::MASK - 4;
            u32::from_le_bytes(self.rest[o..o + 4].try_into().unwrap_or([0; 4]))
        }

        fn rec_u32(&self, bit: u32, field: usize) -> Option<u32> {
            let abs = clk_volt_ctrl_entry::BASE
                .checked_add(bit as usize * clk_volt_ctrl_entry::STRIDE)?
                .checked_add(field)?;
            let s = self.rest.get(abs - 4..abs)?;
            Some(u32::from_le_bytes(s.try_into().ok()?))
        }

        /// Record active flag (u32 @rec+0) for `bit` — the driver-side
        /// "this controller exists" gate (SET commits only ==1 records).
        pub fn record_active(&self, bit: u32) -> Option<u32> {
            self.rec_u32(bit, clk_volt_ctrl_entry::ACTIVE)
        }

        /// u8 field @rec+4.
        pub fn record_b04(&self, bit: u32) -> Option<u8> {
            let abs = clk_volt_ctrl_entry::BASE
                .checked_add(bit as usize * clk_volt_ctrl_entry::STRIDE)?
                .checked_add(clk_volt_ctrl_entry::B04)?;
            self.rest.get(abs - 4).copied()
        }

        /// u16 field @rec+6.
        pub fn record_u16_06(&self, bit: u32) -> Option<u16> {
            let abs = clk_volt_ctrl_entry::BASE
                .checked_add(bit as usize * clk_volt_ctrl_entry::STRIDE)?
                .checked_add(clk_volt_ctrl_entry::U16_06)?;
            Some(u16::from_le_bytes(
                self.rest.get(abs - 4..abs - 2)?.try_into().ok()?,
            ))
        }

        /// u32 field @rec+`which` for `which` ∈ {8, 0xC, 0x10}.
        pub fn record_u32(&self, bit: u32, which: usize) -> Option<u32> {
            match which {
                clk_volt_ctrl_entry::U32_08
                | clk_volt_ctrl_entry::U32_0C
                | clk_volt_ctrl_entry::U32_10 => self.rec_u32(bit, which),
                _ => None,
            }
        }

        /// Extended u32 @rec+`which` for `which` ∈ {0x34, 0x38, 0x3C}
        /// (wire-packed only when active==1).
        pub fn record_ext_u32(&self, bit: u32, which: usize) -> Option<u32> {
            match which {
                clk_volt_ctrl_entry::U32_34
                | clk_volt_ctrl_entry::U32_38
                | clk_volt_ctrl_entry::U32_3C => self.rec_u32(bit, which),
                _ => None,
            }
        }

        /// Read the FULL nine-field payload of record `bit`:
        /// `[active, b04, u16_06, u32_08, u32_0C, u32_10, u32_34, u32_38, u32_3C]`.
        pub fn record_full(&self, bit: u32) -> Option<[u32; 9]> {
            let base = clk_volt_ctrl_entry::BASE
                .checked_add(bit as usize * clk_volt_ctrl_entry::STRIDE)?;
            let mut out = [0u32; 9];
            for (k, (field, width)) in CLK_VOLT_CTRL_FIELDS.iter().enumerate() {
                let abs = base.checked_add(*field)?;
                out[k] = match *width {
                    1 => *self.rest.get(abs - 4)? as u32,
                    2 => {
                        u16::from_le_bytes(self.rest.get(abs - 4..abs - 2)?.try_into().ok()?) as u32
                    }
                    _ => u32::from_le_bytes(self.rest.get(abs - 4..abs)?.try_into().ok()?),
                };
            }
            Some(out)
        }

        /// Overwrite the FULL nine-field payload of record `bit` (same
        /// order as [`Self::record_full`]); each field is written at its
        /// native width (u8/u16/u32), never spilling into its neighbour.
        pub fn set_record_full(&mut self, bit: u32, vals: [u32; 9]) -> Option<()> {
            let base = clk_volt_ctrl_entry::BASE
                .checked_add(bit as usize * clk_volt_ctrl_entry::STRIDE)?;
            for (k, (field, width)) in CLK_VOLT_CTRL_FIELDS.iter().enumerate() {
                let abs = base.checked_add(*field)?;
                match *width {
                    1 => *self.rest.get_mut(abs - 4)? = vals[k] as u8,
                    2 => self
                        .rest
                        .get_mut(abs - 4..abs - 2)?
                        .copy_from_slice(&(vals[k] as u16).to_le_bytes()),
                    _ => self
                        .rest
                        .get_mut(abs - 4..abs)?
                        .copy_from_slice(&vals[k].to_le_bytes()),
                }
            }
            Some(())
        }
    }

    impl NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_INFO {
        /// Driver-filled controller mask @+4.
        pub fn mask(&self) -> u32 {
            let o = clk_volt_info_entry::MASK - 4;
            u32::from_le_bytes(self.rest[o..o + 4].try_into().unwrap_or([0; 4]))
        }

        /// Raw u32 at record-relative offset `field` of bit `bit`.
        pub fn rec_u32(&self, bit: u32, field: usize) -> Option<u32> {
            if field >= clk_volt_info_entry::STRIDE {
                return None;
            }
            let abs = clk_volt_info_entry::BASE
                .checked_add(bit as usize * clk_volt_info_entry::STRIDE)?
                .checked_add(field)?;
            let s = self.rest.get(abs - 4..abs)?;
            Some(u32::from_le_bytes(s.try_into().ok()?))
        }

        /// i32 @rec+0x38 — the wire carries a 16-bit value the handler
        /// sign-extends; reconstruct the same value from those 2 bytes.
        pub fn rec_i32_38(&self, bit: u32) -> Option<i32> {
            let abs = clk_volt_info_entry::BASE
                .checked_add(bit as usize * clk_volt_info_entry::STRIDE)?
                .checked_add(clk_volt_info_entry::I32_38)?;
            Some(i16::from_le_bytes(self.rest.get(abs - 4..abs - 2)?.try_into().ok()?) as i32)
        }

        /// i32 @rec+0x3C (wire i16 sign-extended).
        pub fn rec_i32_3c(&self, bit: u32) -> Option<i32> {
            let abs = clk_volt_info_entry::BASE
                .checked_add(bit as usize * clk_volt_info_entry::STRIDE)?
                .checked_add(clk_volt_info_entry::I32_3C)?;
            Some(i16::from_le_bytes(self.rest.get(abs - 4..abs - 2)?.try_into().ok()?) as i32)
        }
    }

    impl NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_STATUS {
        /// Seed the input mask at +4 (call before GET_STATUS).
        pub fn set_mask(&mut self, mask: u32) {
            let o = clk_volt_status_entry::MASK - 4;
            self.rest[o..o + 4].copy_from_slice(&mask.to_le_bytes());
        }

        /// Raw u32 at record-relative offset `field` of bit `bit`.
        pub fn rec_u32(&self, bit: u32, field: usize) -> Option<u32> {
            if field >= clk_volt_status_entry::STRIDE {
                return None;
            }
            let abs = clk_volt_status_entry::BASE
                .checked_add(bit as usize * clk_volt_status_entry::STRIDE)?
                .checked_add(field)?;
            let s = self.rest.get(abs - 4..abs)?;
            Some(u32::from_le_bytes(s.try_into().ok()?))
        }
    }

    nvapi! {
        /// Voltage-controller GET_INFO (ID 0xEC6FCD0B, RM 0x20809033, magic
        /// 0x10CD4). Driver fills the controller mask + per-bit descriptors
        /// (two wire-i16 fields arrive sign-extended).
        pub unsafe fn NvAPI_GPU_ClockClkVoltControllerGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_INFO) -> NvAPI_Status;
    }

    nvapi! {
        /// Voltage-controller GET_STATUS (ID 0x8506C02E, RM 0x20809034,
        /// magic 0x10BC8). MASK-SEEDED at +4.
        pub unsafe fn NvAPI_GPU_ClockClkVoltControllerGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pStatus: *mut NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_STATUS) -> NvAPI_Status;
    }

    nvapi! {
        /// Voltage-controllers GET_CONTROL (ID 0xDD41633C, RM 0x20809035,
        /// magic 0x10C48). MASK-SEEDED at +4; the per-record `active` u32
        /// gates which controllers exist.
        pub unsafe fn NvAPI_GPU_ClockClkVoltControllersGetControl(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *mut NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Voltage-controllers SET_CONTROL (ID 0xF9833206, RM 0x2080D036).
        /// DANGEROUS VRM-controller write: elevation-gated (-104 without
        /// admin) BEFORE any validation, and only records with active==1
        /// are committed. Always snapshot via GetControl, patch a COPY,
        /// SET, read back, verify, restore on mismatch (see medium-layer
        /// `set_clk_volt_controller_record`). Field semantics are
        /// unconfirmed pending a laptop 610 A/B — do not call with
        /// synthetic values.
        pub unsafe fn NvAPI_GPU_ClockClkVoltControllersSetControl(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *const NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_CONTROL) -> NvAPI_Status;
    }

    /// Layout regression tests for the ClockClkVoltController(s) family:
    /// magic closure (BASE + 32*stride + 4 == size field) and accessor
    /// round-trips at the disasm-derived offsets.
    #[cfg(test)]
    mod clk_volt_tests {
        use super::*;

        fn put_u32(rest: &mut [u8], abs: usize, v: u32) {
            rest[abs - 4..abs].copy_from_slice(&v.to_le_bytes());
        }

        #[test]
        fn clk_volt_magic_closure() {
            // record region ends exactly at the magic size field:
            // BASE + 32*stride == size (BASE already sits inside the size).
            // INFO: 0x54 + 32*0x64 == 0xCD4 (3284)
            assert_eq!(
                clk_volt_info_entry::BASE + 32 * clk_volt_info_entry::STRIDE,
                0xCD4
            );
            assert_eq!(size_of::<NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_INFO>(), 3284);
            // STATUS: 0x148 + 32*0x54 == 0xBC8 (3016)
            assert_eq!(
                clk_volt_status_entry::BASE + 32 * clk_volt_status_entry::STRIDE,
                0xBC8
            );
            assert_eq!(size_of::<NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_STATUS>(), 3016);
            // CONTROL: 0x48 + 32*0x60 == 0xC48 (3144)
            assert_eq!(
                clk_volt_ctrl_entry::BASE + 32 * clk_volt_ctrl_entry::STRIDE,
                0xC48
            );
            assert_eq!(size_of::<NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_CONTROL>(), 3144);
            // rest arrays are exactly SIZE-4 (the OOB-read-garbage rule)
            assert_eq!(
                size_of::<NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_CONTROL>() - 4,
                3140
            );
        }

        #[test]
        fn clk_volt_control_record_roundtrip() {
            let mut c = NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_CONTROL::default();
            c.set_mask(0xFFFF_FFFF);
            assert_eq!(c.mask(), 0xFFFF_FFFF);
            // bit 5 active with the nine-field payload
            let vals: [u32; 9] = [
                1,
                0xAB,
                15,
                327_680,
                983_040,
                1_638_400,
                12,
                34,
                0xFFFD_BEA2,
            ];
            c.set_record_full(5, vals).unwrap();
            assert_eq!(c.record_active(5), Some(1));
            assert_eq!(c.record_b04(5), Some(0xAB));
            assert_eq!(c.record_u16_06(5), Some(15));
            assert_eq!(c.record_u32(5, 8), Some(327_680));
            assert_eq!(c.record_u32(5, 0xC), Some(983_040));
            assert_eq!(c.record_u32(5, 0x10), Some(1_638_400));
            assert_eq!(c.record_ext_u32(5, 0x34), Some(12));
            assert_eq!(c.record_ext_u32(5, 0x38), Some(34));
            assert_eq!(c.record_ext_u32(5, 0x3C), Some(0xFFFD_BEA2));
            assert_eq!(c.record_full(5), Some(vals));
            // untouched bit stays zero; bit 31 (last record) is in bounds
            assert_eq!(c.record_active(6), Some(0));
            assert!(c.record_active(32).is_none());
            let vals31: [u32; 9] = [1, 0, 0, 1, 2, 3, 4, 5, 6];
            c.set_record_full(31, vals31).unwrap();
            assert_eq!(c.record_full(31), Some(vals31));
        }

        #[test]
        fn clk_volt_info_status_accessors() {
            let mut info = NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_INFO::default();
            put_u32(&mut info.rest, clk_volt_info_entry::MASK, 0x7);
            assert_eq!(info.mask(), 0x7);
            // i32 @0x38: the driver writes a 16-bit -18750 (0xB6C2) there
            // and sign-extends on read; the accessor must reconstruct
            // -18750 from exactly those 2 bytes.
            let abs = clk_volt_info_entry::BASE
                + 1 * clk_volt_info_entry::STRIDE
                + clk_volt_info_entry::I32_38;
            info.rest[abs - 4..abs - 2].copy_from_slice(&0xB6C2u16.to_le_bytes());
            assert_eq!(info.rec_i32_38(1), Some(-18750));
            assert!(info.rec_u32(1, 0x64).is_none());

            let mut st = NV_GPU_CLOCK_CLK_VOLT_CONTROLLERS_STATUS::default();
            st.set_mask(1 << 3);
            let abs = clk_volt_status_entry::BASE
                + 3 * clk_volt_status_entry::STRIDE
                + clk_volt_status_entry::U32_2C;
            put_u32(&mut st.rest, abs, 625_000);
            assert_eq!(st.rec_u32(3, clk_volt_status_entry::U32_2C), Some(625_000));
            assert_eq!(st.rec_u32(4, clk_volt_status_entry::U32_2C), Some(0));
        }
    }

    // ------------------------------------------------------------------
    // nvClocks.spec P1 batch (2026-09-06 audit — reverse/version-audit/
    // nvclocks-audit/, 4-agent static RE + live 462.96 GTX 1650 SUPER
    // read-only probes). Six read surfaces + the ADC GetInfo seed source.
    // All constants END-biased ([abs-width..abs] addressing); masks u32.
    // Cross-generation (462.96 ↔ 610.88): every user layout/magic below is
    // IDENTICAL; only RM cmd (0x2080_1XXX ↔ 0x2080_9XXX/DXXX) and escape
    // (0x070004A ↔ 0x0700049 etc.) shift inside the handlers.
    // ------------------------------------------------------------------

    /// ADC device directory offsets ([`NV_GPU_CLOCK_ADC_DEVICES_INFO`]) —
    /// the mask seed source for GetStatus. Records @0x50+bit*0x4C.
    pub mod adc_devices_info_entry {
        pub const MASK: usize = 4;
        /// V1 record base (10 slots). V2 (0x209F0) has a larger header —
        /// records start at [`V2_BASE`] 0x70 instead.
        pub const BASE: usize = 0x50;
        /// V2 record base (32 slots; IDA R610.88 closure 0x70+32×0x4C=0x9F0)
        pub const V2_BASE: usize = 0x70;
        pub const STRIDE: usize = 0x4C;
        /// rec+0: u32 device type (probe saw small ints)
        pub const REC_TYPE: usize = 0x04;
        /// rec+4: u32 channel id (probe: {8,16,32,0xC05})
        pub const REC_CHANNEL: usize = 0x08;
        /// rec+8..+24: 16 name bytes
        pub const REC_NAME: usize = 0x0C;
    }

    nvstruct! {
        /// ADC device directory V1 (ID 0x68789E2A, RM 0x208090A0-family;
        /// magic 0x10348 = (1<<16)|840). Driver fills mask@+4 and per-bit
        /// device descriptors; the mask seeds
        /// [`NV_GPU_CLOCK_ADC_DEVICES_STATUS`]. Parts with MORE than 10
        /// ADC devices reject this stamp with -174
        /// INSUFFICIENT_BUFFER (live 4060 Laptop/R610) — the caller must
        /// fall back to [`NV_GPU_CLOCK_ADC_DEVICES_INFO2`] (32 slots).
        pub struct NV_GPU_CLOCK_ADC_DEVICES_INFO_V1 {
            pub version: NvVersion,
            /// +4 .. +840: mask@+4, header, 10×0x4C records @0x50
            pub rest: [u8; 836],
        }
    }

    nvversion! { @=NV_GPU_CLOCK_ADC_DEVICES_INFO NV_GPU_CLOCK_ADC_DEVICES_INFO_V1(1) = 0x348 }

    nvstruct! {
        /// ADC device directory V2 — magic **0x209F0** = (2<<16)|2544
        /// (IDA R610.88: `cmp eax, 0x209f0` at 0x18020312A, alongside the
        /// V1 gate 0x10348; unpack copies mask@wire+0x3C→+4, then
        /// per-bit 4-byte records; V1 path passes count 0xA, V2 count
        /// 0x20). Same layout as V1 with 32 slots. REQUIRED on parts
        /// exposing more than 10 devices (4060 Laptop/R610 rejects V1
        /// with -174). The earlier 0x109C8 guess was a stamp misread.
        pub struct NV_GPU_CLOCK_ADC_DEVICES_INFO_V2 {
            pub version: NvVersion,
            /// +4 .. +2544: mask@+4, header, 32×0x4C records @0x50
            pub rest: [u8; 2540],
        }
    }

    nvversion! { @=NV_GPU_CLOCK_ADC_DEVICES_INFO2 NV_GPU_CLOCK_ADC_DEVICES_INFO_V2(2) = 0x9f0 }

    impl NV_GPU_CLOCK_ADC_DEVICES_INFO2 {
        pub fn mask(&self) -> u32 {
            u32::from_le_bytes(
                self.rest[adc_devices_info_entry::MASK - 4..adc_devices_info_entry::MASK]
                    .try_into()
                    .unwrap_or([0; 4]),
            )
        }

        pub fn rec_u32(&self, bit: u32, field: usize) -> Option<u32> {
            if field >= adc_devices_info_entry::STRIDE {
                return None;
            }
            // V2 header is 0x20 larger — records start at 0x70
            let abs = adc_devices_info_entry::V2_BASE
                .checked_add(bit as usize * adc_devices_info_entry::STRIDE)?
                .checked_add(field)?;
            Some(u32::from_le_bytes(
                self.rest.get(abs - 4..abs)?.try_into().ok()?,
            ))
        }

        pub fn rec_name(&self, bit: u32) -> Option<&[u8]> {
            let abs = adc_devices_info_entry::V2_BASE
                .checked_add(bit as usize * adc_devices_info_entry::STRIDE)?
                .checked_add(adc_devices_info_entry::REC_NAME)?;
            self.rest.get(abs - 4..abs + 12)
        }
    }

    /// ADC live-status offsets ([`NV_GPU_CLOCK_ADC_DEVICES_STATUS`]) — ★P1:
    /// the ONLY live telemetry stream found in the whole nvClocks audit.
    /// Records @0x48+bit*0x4C; live 462.96 (4 devices): state −1,
    /// +4 = rail voltage µV (idle 625000..637500, load 1043750 — the four
    /// channels read DIFFERENT rails), +8/+0xA = VF-table voltage-point
    /// index pair (idle ↔ 28-31, load ↔ 94-96; user cross-cert
    /// 2026-09-06 — the earlier temperature-°C hypothesis was WRONG),
    /// +9 = 0, type byte = 2, +0x2C = 0x7FFFFFFF sentinel.
    pub mod adc_devices_status_entry {
        pub const MASK: usize = 4;
        pub const BASE: usize = 0x48;
        pub const STRIDE: usize = 0x4C;
        /// rec+0x00: u32 state (−1 observed = invalid/reserved)
        pub const REC_STATE: usize = 0x00;
        /// rec+0x04: u32 rail voltage µV (cross-certified: four distinct
        /// per-rail readings that track load)
        pub const REC_VALUE_UV: usize = 0x04;
        /// rec+0x08: u8 VF-table voltage-point index (锁定电压点 id;
        /// idle ≈630 mV ↔ 28-31, load 1043 mV ↔ 94-96)
        pub const REC_VFPID_A: usize = 0x08;
        /// rec+0x09: u8 zero observed
        pub const REC_ZERO: usize = 0x09;
        /// rec+0x0A: u8 second VF-point index (±1 of REC_VFPID_A — likely
        /// current-vs-target or min-vs-max tracking)
        pub const REC_VFPID_B: usize = 0x0A;
        /// rec+0x0B: u8 value format (1=no value / 2=u32 / 3,4=u8)
        pub const REC_TYPE: usize = 0x0B;
        /// rec+0x2C: secondary value (u32 when type==2, else u8 low byte);
        /// 0x7FFFFFFF sentinel observed
        pub const REC_VALUE2: usize = 0x2C;
    }

    nvstruct! {
        /// ADC live status (ID 0x43D9B26A, RM 0x208090A1, magic 0x10340 =
        /// (1<<16)|832). MASK-SEEDED at +4 (seed from
        /// [`NV_GPU_CLOCK_ADC_DEVICES_INFO`]; use its V2 stamp when the
        /// part exposes >10 devices — 4060 Laptop/R610 needs it).
        pub struct NV_GPU_CLOCK_ADC_DEVICES_STATUS_V1 {
            pub version: NvVersion,
            /// +4 .. +832: mask@+4, header, 10×0x4C records @0x48
            pub rest: [u8; 828],
        }
    }

    nvversion! { @=NV_GPU_CLOCK_ADC_DEVICES_STATUS NV_GPU_CLOCK_ADC_DEVICES_STATUS_V1(1) = 0x340 }

    nvstruct! {
        /// ADC live status V2 — magic 0x109C8 = (1<<16)|2504, 32 slots,
        /// same record layout. Required on parts exposing more than 10
        /// ADC devices (4060 Laptop/R610; live R610.88 handler gate
        /// `cmp eax, 0x109c8` at 0x180203C5D).
        pub struct NV_GPU_CLOCK_ADC_DEVICES_STATUS_V2 {
            pub version: NvVersion,
            /// +4 .. +2504: mask@+4, header, 32×0x4C records @0x48
            pub rest: [u8; 2500],
        }
    }

    nvversion! { @=NV_GPU_CLOCK_ADC_DEVICES_STATUS2 NV_GPU_CLOCK_ADC_DEVICES_STATUS_V2(1) = 0x9c8 }

    impl NV_GPU_CLOCK_ADC_DEVICES_STATUS2 {
        pub fn set_mask(&mut self, mask: u32) {
            let o = adc_devices_status_entry::MASK - 4;
            self.rest[o..o + 4].copy_from_slice(&mask.to_le_bytes());
        }

        pub fn rec_u32(&self, bit: u32, field: usize) -> Option<u32> {
            if field >= adc_devices_status_entry::STRIDE {
                return None;
            }
            let abs = adc_devices_status_entry::BASE
                .checked_add(bit as usize * adc_devices_status_entry::STRIDE)?
                .checked_add(field)?;
            Some(u32::from_le_bytes(
                self.rest.get(abs - 4..abs)?.try_into().ok()?,
            ))
        }
    }

    /// ClkPropRegimes directory offsets ([`NV_GPU_CLOCK_CLK_PROP_REGIMES_INFO`])
    /// — P1; settles the archived "scaling-sibling triple" mystery ID.
    pub mod clk_prop_regimes_info_entry {
        /// +4: u32 availability status (wire 3-state: 0/1/0xF)
        pub const STATUS: usize = 4;
        /// +8: u32 regime mask (output)
        pub const MASK: usize = 8;
        pub const BASE: usize = 0x10C;
        pub const STRIDE: usize = 0x4C;
        /// rec+0: u32 status (wire gate byte == 1 → 0, else −1 + whole-call
        /// −103)
        pub const REC_STATUS: usize = 0x00;
        /// rec+4: u32 regime type — wire byte remapped through the 19-entry
        /// jump table @0x180212808 → {1..7,9,0xF..0x1A}; illegal → 0x1F
        pub const REC_TYPE: usize = 0x04;
        /// rec+8: u32 FIXED driver constant (live 4060/R610: 19 entries
        /// static across clock/OC changes — NOT frequency anchors; same
        /// values as the archived scaling-sibling table)
        pub const REC_VALUE: usize = 0x08;
    }

    nvstruct! {
        /// ClkPropRegimes directory (ID 0xCF08E934, RM 0x20809079, magic
        /// 0x10A8C = (1<<16)|2700). No input seed. Each record = one
        /// domain×regime entry (archived "4060: 19 per-domain values"
        /// matches the 19-entry type remap).
        pub struct NV_GPU_CLOCK_CLK_PROP_REGIMES_INFO_V1 {
            pub version: NvVersion,
            /// +4 .. +2700: status@+4, mask@+8, 32×0x4C records @0x10C
            pub rest: [u8; 2696],
        }
    }

    nvversion! { @=NV_GPU_CLOCK_CLK_PROP_REGIMES_INFO NV_GPU_CLOCK_CLK_PROP_REGIMES_INFO_V1(1) = 0xa8c }

    /// ClkPropRegimes control offsets ([`NV_GPU_CLOCK_CLK_PROP_REGIMES_CONTROL`]).
    pub mod clk_prop_regimes_ctrl_entry {
        /// +8: u32 mask, BOTH input seed and echo
        pub const MASK: usize = 8;
        pub const BASE: usize = 0x48;
        pub const STRIDE: usize = 0x108;
        /// rec+0x48: u32 status (0 / −1)
        pub const REC_STATUS: usize = 0x48;
        /// rec+0x4C: u32 value
        pub const REC_VALUE: usize = 0x4C;
    }

    nvstruct! {
        /// ClkPropRegimes control snapshot (ID 0x4F11EAA4, RM 0x2080907B,
        /// magic 0x12148 = (1<<16)|8520). Mask-seeded at +8; pairs with
        /// [`NV_GPU_CLOCK_CLK_PROP_REGIMES_INFO`].
        pub struct NV_GPU_CLOCK_CLK_PROP_REGIMES_CONTROL_V1 {
            pub version: NvVersion,
            /// +4 .. +8520: mask@+8, 32×0x108 records @0x48
            pub rest: [u8; 8516],
        }
    }

    nvversion! { @=NV_GPU_CLOCK_CLK_PROP_REGIMES_CONTROL NV_GPU_CLOCK_CLK_PROP_REGIMES_CONTROL_V1(1) = 0x2148 }

    nvstruct! {
        /// Per-clock-domain legal frequency enumeration (ID 0x40BDDDB36,
        /// RM 0x2080901A, magic 0x10808 = 8 + 512×4). Selector u8@+4 picks
        /// the domain (live: 0=core 141 pts 30..2130 step 15, 2=memory
        /// [405,810,5001,5751,6001], 4=[147,324,405,540,648,810]); the
        /// driver fills count u16@+6 and the freq table. **Unit is MHz**
        /// (the only non-kHz table in the family — verified by magnitude:
        /// 30 MHz idle floor, 6001 ≈ GDDR6 half-rate).
        pub struct NV_GPU_CLOCK_CLK_DOMAIN_FREQS_ENUM_V1 {
            pub version: NvVersion,
            /// +4: selector u8 (input) · +6: count u16 (output) · +8: 512×u32 freqs
            pub rest: [u8; 2052],
        }
    }

    nvversion! { @=NV_GPU_CLOCK_CLK_DOMAIN_FREQS_ENUM NV_GPU_CLOCK_CLK_DOMAIN_FREQS_ENUM_V1(1) = 0x808 }

    impl NV_GPU_CLOCK_CLK_DOMAIN_FREQS_ENUM {
        /// Domain selector input (+4).
        pub fn set_selector(&mut self, selector: u8) {
            self.rest[0] = selector;
        }

        /// Frequency count output (+6, u16).
        pub fn count(&self) -> u16 {
            u16::from_le_bytes(self.rest[2..4].try_into().unwrap_or([0; 2]))
        }

        /// The filled frequency table (MHz), truncated to `count`.
        pub fn freqs_mhz(&self) -> Vec<u32> {
            let count = (self.count() as usize).min(512);
            (0..count)
                .map(|k| {
                    let o = 4 + k * 4;
                    u32::from_le_bytes(self.rest[o..o + 4].try_into().unwrap_or([0; 4]))
                })
                .collect()
        }
    }

    nvstruct! {
        /// Public clock info (ID 0x1B46D4CC, NO RM — escape 0x070010B with
        /// the secondary "escDomainData" wire type; magic 0x10188 = 392B).
        /// Driver fills count@+4 and lands {value, flag, max} u32 triples
        /// into the 32×12B slot area at type-dependent offsets (observed:
        /// type1→+8, type4→+0x38, type2→+0x5C, type8→+0x68); the caller
        /// PRE-FILLS untouched slots with {32, ?, 100}. Raw slot access
        /// until the slot map is confirmed on more parts.
        pub struct NV_GPU_PUBLIC_CLOCK_INFO_V1 {
            pub version: NvVersion,
            /// +4: count u32 · +8..+392: 32×12B slots
            pub rest: [u8; 388],
        }
    }

    nvversion! { @=NV_GPU_PUBLIC_CLOCK_INFO NV_GPU_PUBLIC_CLOCK_INFO_V1(1) = 0x188 }

    /// Named landing offsets (absolute) inside
    /// [`NV_GPU_PUBLIC_CLOCK_INFO`] — where the driver wrote each wire
    /// type's {value, flag, max} triple on the live 462.96 probe.
    pub mod public_clock_info_slots {
        pub const COUNT: usize = 4;
        pub const SLOT_AREA: usize = 8;
        pub const TYPE1: usize = 0x08;
        pub const TYPE4: usize = 0x38;
        pub const TYPE2: usize = 0x5C;
        pub const TYPE8: usize = 0x68;
    }

    impl NV_GPU_PUBLIC_CLOCK_INFO {
        /// Pre-fill every 12B slot with the driver's expected default
        /// {32, 0, 100} triple (the handler only overwrites filled
        /// domains; probe observed defaults 0x20/0x64).
        pub fn preset_defaults(&mut self) {
            for slot in 0..32 {
                let o = 4 + slot * 12;
                self.rest[o..o + 4].copy_from_slice(&32u32.to_le_bytes());
                self.rest[o + 4..o + 8].copy_from_slice(&0u32.to_le_bytes());
                self.rest[o + 8..o + 12].copy_from_slice(&100u32.to_le_bytes());
            }
        }

        /// Domain count output (+4).
        pub fn count(&self) -> u32 {
            u32::from_le_bytes(self.rest[0..4].try_into().unwrap_or([0; 4]))
        }

        /// Raw u32 at absolute offset `abs` (≥8).
        pub fn u32_at(&self, abs: usize) -> Option<u32> {
            if abs < public_clock_info_slots::SLOT_AREA || abs + 4 > self.rest.len() + 4 {
                return None;
            }
            Some(u32::from_le_bytes(
                self.rest.get(abs - 4..abs)?.try_into().ok()?,
            ))
        }
    }

    nvstruct! {
        /// Locked-clock mode status (ID 0xC4733F19, NO RM — escape
        /// 0x07001D6, 8-byte struct). modeMask bit0..3 = four mode flags.
        /// VERDICT DOWNGRADE (user live test 2026-09-06): returns 0 with
        /// BOTH PerfClientLimits voltage-lock AND frequency-lock active —
        /// this is NOT the readback of any nvoc lock plane; the control
        /// surface it mirrors is unidentified. Keep raw; don't surface as
        /// "locked" state.
        pub struct NV_GPU_LOCKED_CLOCK_MODE_STATUS_V1 {
            pub version: NvVersion,
            pub mode_mask: u32,
        }
    }

    nvversion! { @=NV_GPU_LOCKED_CLOCK_MODE_STATUS NV_GPU_LOCKED_CLOCK_MODE_STATUS_V1(1) = 0x8 }

    impl NV_GPU_CLOCK_ADC_DEVICES_INFO {
        /// Driver-filled ADC device mask @+4.
        pub fn mask(&self) -> u32 {
            u32::from_le_bytes(
                self.rest[adc_devices_info_entry::MASK - 4..adc_devices_info_entry::MASK]
                    .try_into()
                    .unwrap_or([0; 4]),
            )
        }

        /// Raw u32 at record-relative offset `field` of bit `bit`.
        pub fn rec_u32(&self, bit: u32, field: usize) -> Option<u32> {
            if field >= adc_devices_info_entry::STRIDE {
                return None;
            }
            let abs = adc_devices_info_entry::BASE
                .checked_add(bit as usize * adc_devices_info_entry::STRIDE)?
                .checked_add(field)?;
            Some(u32::from_le_bytes(
                self.rest.get(abs - 4..abs)?.try_into().ok()?,
            ))
        }

        /// The 16 name bytes of device `bit`.
        pub fn rec_name(&self, bit: u32) -> Option<&[u8]> {
            let abs = adc_devices_info_entry::BASE
                .checked_add(bit as usize * adc_devices_info_entry::STRIDE)?
                .checked_add(adc_devices_info_entry::REC_NAME)?;
            self.rest.get(abs - 4..abs + 12)
        }
    }

    impl NV_GPU_CLOCK_ADC_DEVICES_STATUS {
        /// Seed the input mask at +4 (call before GetStatus).
        pub fn set_mask(&mut self, mask: u32) {
            let o = adc_devices_status_entry::MASK - 4;
            self.rest[o..o + 4].copy_from_slice(&mask.to_le_bytes());
        }

        /// Echoed mask @+4.
        pub fn mask(&self) -> u32 {
            let o = adc_devices_status_entry::MASK - 4;
            u32::from_le_bytes(self.rest[o..o + 4].try_into().unwrap_or([0; 4]))
        }

        /// Raw u32 at record-relative offset `field` of bit `bit`.
        pub fn rec_u32(&self, bit: u32, field: usize) -> Option<u32> {
            if field >= adc_devices_status_entry::STRIDE {
                return None;
            }
            let abs = adc_devices_status_entry::BASE
                .checked_add(bit as usize * adc_devices_status_entry::STRIDE)?
                .checked_add(field)?;
            Some(u32::from_le_bytes(
                self.rest.get(abs - 4..abs)?.try_into().ok()?,
            ))
        }
    }

    impl NV_GPU_CLOCK_CLK_PROP_REGIMES_INFO {
        /// Availability status @+4 (0/1/0xF).
        pub fn status(&self) -> u32 {
            let o = clk_prop_regimes_info_entry::STATUS - 4;
            u32::from_le_bytes(self.rest[o..o + 4].try_into().unwrap_or([0; 4]))
        }

        /// Regime mask @+8.
        pub fn mask(&self) -> u32 {
            let o = clk_prop_regimes_info_entry::MASK - 4;
            u32::from_le_bytes(self.rest[o..o + 4].try_into().unwrap_or([0; 4]))
        }

        /// Raw u32 at record-relative offset `field` of bit `bit`.
        pub fn rec_u32(&self, bit: u32, field: usize) -> Option<u32> {
            if field >= clk_prop_regimes_info_entry::STRIDE {
                return None;
            }
            let abs = clk_prop_regimes_info_entry::BASE
                .checked_add(bit as usize * clk_prop_regimes_info_entry::STRIDE)?
                .checked_add(field)?;
            Some(u32::from_le_bytes(
                self.rest.get(abs - 4..abs)?.try_into().ok()?,
            ))
        }
    }

    impl NV_GPU_CLOCK_CLK_PROP_REGIMES_CONTROL {
        /// Seed the input mask at +8 (call before GetControl).
        pub fn set_mask(&mut self, mask: u32) {
            let o = clk_prop_regimes_ctrl_entry::MASK - 4;
            self.rest[o..o + 4].copy_from_slice(&mask.to_le_bytes());
        }

        /// Echoed mask @+8.
        pub fn mask(&self) -> u32 {
            let o = clk_prop_regimes_ctrl_entry::MASK - 4;
            u32::from_le_bytes(self.rest[o..o + 4].try_into().unwrap_or([0; 4]))
        }

        /// Raw u32 at record-relative offset `field` of bit `bit`.
        pub fn rec_u32(&self, bit: u32, field: usize) -> Option<u32> {
            if field >= clk_prop_regimes_ctrl_entry::STRIDE {
                return None;
            }
            let abs = clk_prop_regimes_ctrl_entry::BASE
                .checked_add(bit as usize * clk_prop_regimes_ctrl_entry::STRIDE)?
                .checked_add(field)?;
            Some(u32::from_le_bytes(
                self.rest.get(abs - 4..abs)?.try_into().ok()?,
            ))
        }
    }

    // ------------------------------------------------------------------
    // nvClocks.spec P2 batch (2026-09-06 audit, nvclocks-audit/
    // nafll-adc-freqctrl.md). READ-ONLY surfaces with broken layouts:
    // NAFLL devices (info/status), ClkFreqController(s) (info/status/
    // control), HWFS GetInfo, ThermalSlowdownState. Same start-biased
    // offset convention; same cross-generation stamp stability.
    // ------------------------------------------------------------------

    /// NAFLL device directory offsets ([`NV_GPU_CLOCK_NAFLL_DEVICES_INFO`],
    /// v2 0x20C58 — the layout this generation serves; v1 0x10414 legacy).
    pub mod nafll_info_entry {
        pub const MASK: usize = 4;
        pub const BASE: usize = 0x58;
        pub const STRIDE: usize = 0x60;
        /// rec+0x00: u32 device type (live 7 devices: all 2)
        pub const REC_DEV_TYPE: usize = 0x00;
        /// rec+0x04: u8 raw domain id (live {3,4,5,2,0,A,B})
        pub const REC_DOMAIN_ID: usize = 0x04;
        /// rec+0x0C: u32 step count (cap 32)
        pub const REC_STEPS: usize = 0x0C;
        /// rec+0x10: u32 mode (0/1→0, 2→2, else 0xFE)
        pub const REC_MODE: usize = 0x10;
        /// rec+0x18: u16 (live 10)
        pub const REC_U16_18: usize = 0x18;
        /// rec+0x1A: u16 base frequency MHz (live 405 ≈ GPC min VF)
        pub const REC_BASE_MHZ: usize = 0x1A;
        /// rec+0x1C/1D: u8 pair (live 8, …)
        pub const REC_B1C: usize = 0x1C;
    }

    nvstruct! {
        /// NAFLL device directory (ID 0x2BC9F805, RM 0x208090B0, magic
        /// 0x20C58 = (2<<16)|3160 = 0x58 + 32×0x60). No input seed. Live
        /// TU116: mask 0x7F, header {u8@8=0x80, 6250@0xC, 450000@0x10}.
        pub struct NV_GPU_CLOCK_NAFLL_DEVICES_INFO_V2 {
            pub version: NvVersion,
            /// +4 .. +3160: mask@+4, header, 32×0x60 records @0x58
            pub rest: [u8; 3156],
        }
    }

    nvversion! { @=NV_GPU_CLOCK_NAFLL_DEVICES_INFO NV_GPU_CLOCK_NAFLL_DEVICES_INFO_V2(2) = 0xc58 }

    impl NV_GPU_CLOCK_NAFLL_DEVICES_INFO {
        pub fn mask(&self) -> u32 {
            let o = nafll_info_entry::MASK - 4;
            u32::from_le_bytes(self.rest[o..o + 4].try_into().unwrap_or([0; 4]))
        }

        pub fn rec_u32(&self, bit: u32, field: usize) -> Option<u32> {
            if field >= nafll_info_entry::STRIDE {
                return None;
            }
            let abs = nafll_info_entry::BASE
                .checked_add(bit as usize * nafll_info_entry::STRIDE)?
                .checked_add(field)?;
            Some(u32::from_le_bytes(
                self.rest.get(abs - 4..abs)?.try_into().ok()?,
            ))
        }
    }

    /// NAFLL live-status offsets ([`NV_GPU_CLOCK_NAFLL_DEVICES_STATUS`],
    /// v1 0x10F48 — the only multi-generation-stable layout; records
    /// EMPIRICAL stride 0x180, 80×4B entries per device).
    pub mod nafll_status_entry {
        pub const MASK: usize = 4;
        pub const BASE: usize = 0x48;
        pub const STRIDE: usize = 0x180;
        /// entry count in the record's table (u32-ish; live 7 tables of
        /// up to 80 entries)
        pub const REC_TABLE: usize = 0x00;
        /// first table entry (u32; entries are {u16,u16} or {u16,idx}
        /// pairs by wire type)
        pub const REC_TABLE0: usize = 0x04;
        /// tail flag u8 (live rec0 = 01)
        pub const REC_B140: usize = 0x140;
    }

    nvstruct! {
        /// NAFLL devices status (ID 0xAFA4113C, RM 0x208090B1, v1 magic
        /// 0x10F48). MASK-SEEDED at +4. V2/V3/V6 stamps exist on 610 but
        /// v1 is the cross-generation-stable surface (live 462.96:
        /// 7 records, static 80-entry u16 tables 29..125 — voltage×10mV
        /// or freq÷15MHz ladder, unit unclosed).
        pub struct NV_GPU_CLOCK_NAFLL_DEVICES_STATUS_V1 {
            pub version: NvVersion,
            /// +4 .. +3912: mask@+4, records @0x48 (stamp size 0xF48;
            /// 7 live records ×0x180 fit, remaining tail opaque)
            pub rest: [u8; 3908],
        }
    }

    nvversion! { @=NV_GPU_CLOCK_NAFLL_DEVICES_STATUS NV_GPU_CLOCK_NAFLL_DEVICES_STATUS_V1(1) = 0xf48 }

    impl NV_GPU_CLOCK_NAFLL_DEVICES_STATUS {
        pub fn set_mask(&mut self, mask: u32) {
            let o = nafll_status_entry::MASK - 4;
            self.rest[o..o + 4].copy_from_slice(&mask.to_le_bytes());
        }
    }

    /// ClkFreqController directory offsets
    /// ([`NV_GPU_CLOCK_CLK_FREQ_CONTROLLER_INFO`], 0x10C4C — the freq
    /// sibling of the ClkVolt INFO; live values anchor the units).
    pub mod clk_freq_ctrl_info_entry {
        pub const MASK: usize = 4;
        pub const BASE: usize = 0x4C;
        pub const STRIDE: usize = 0x60;
        /// rec+0: u32 supported flag (domain type ∈ {1,3})
        pub const REC_SUPPORTED: usize = 0x00;
        /// rec+4: u8 type enum (raw 0..8 identity, 9→0xF, 10→0x10)
        pub const REC_TYPE: usize = 0x04;
        /// rec+8: u32 step count
        pub const REC_STEPS: usize = 0x08;
        /// rec+0x2C: u32 max VF frequency kHz (live 1638400 = 1638.4 MHz)
        pub const REC_MAX_KHZ: usize = 0x2C;
        /// rec+0x38: i32 −18750 (freq offset floor, ±18.75 MHz pair)
        pub const REC_MIN_OFFSET: usize = 0x38;
        /// rec+0x3C: i32 +18750 (offset ceiling)
        pub const REC_MAX_OFFSET: usize = 0x3C;
    }

    nvstruct! {
        /// ClkFreqController directory (ID 0x58F4F4C1, RM 0x20809025,
        /// magic 0x10C4C = (1<<16)|3148 = 0x4C + 32×0x60). No seed.
        /// Live TU116: mask 0x7, records carry {1638400 max_kHz,
        /// −18750/+18750 offset pair}.
        pub struct NV_GPU_CLOCK_CLK_FREQ_CONTROLLER_INFO_V1 {
            pub version: NvVersion,
            /// +4 .. +3148: mask@+4, header, 32×0x60 records @0x4C
            pub rest: [u8; 3144],
        }
    }

    nvversion! { @=NV_GPU_CLOCK_CLK_FREQ_CONTROLLER_INFO NV_GPU_CLOCK_CLK_FREQ_CONTROLLER_INFO_V1(1) = 0xc4c }

    impl NV_GPU_CLOCK_CLK_FREQ_CONTROLLER_INFO {
        pub fn mask(&self) -> u32 {
            let o = clk_freq_ctrl_info_entry::MASK - 4;
            u32::from_le_bytes(self.rest[o..o + 4].try_into().unwrap_or([0; 4]))
        }

        pub fn rec_u32(&self, bit: u32, field: usize) -> Option<u32> {
            if field >= clk_freq_ctrl_info_entry::STRIDE {
                return None;
            }
            let abs = clk_freq_ctrl_info_entry::BASE
                .checked_add(bit as usize * clk_freq_ctrl_info_entry::STRIDE)?
                .checked_add(field)?;
            Some(u32::from_le_bytes(
                self.rest.get(abs - 4..abs)?.try_into().ok()?,
            ))
        }
    }

    nvstruct! {
        /// ClkFreqController live status V1 (ID 0x45C064D5, RM 0x20809026,
        /// magic 0x109CC = (1<<16)|2508 = 0x4C + 32×0x4C). MASK-SEEDED at
        /// +4. Live TU116: all records {6, 0, 0} static on the v1 path
        /// (470-generation data unfilled — re-evaluate with the v3 stamp
        /// 0x30970 on 610+ drivers).
        pub struct NV_GPU_CLOCK_CLK_FREQ_CONTROLLER_STATUS_V1 {
            pub version: NvVersion,
            /// +4 .. +2508: mask@+4, 32×0x4C records @0x4C
            pub rest: [u8; 2504],
        }
    }

    nvversion! { @=NV_GPU_CLOCK_CLK_FREQ_CONTROLLER_STATUS NV_GPU_CLOCK_CLK_FREQ_CONTROLLER_STATUS_V1(1) = 0x9cc }

    nvstruct! {
        /// HWFS (hardware forced slowdown) control (ID 0x14277C24, GET
        /// RM 0x20808540/0x20800540, escape 0x07000190/0x07000191 — the
        /// "hwfsControlEscData" string in the binary anchors this ID).
        /// Flat 52-byte struct, no mask/records: +4 u8 in, +5 u8 out,
        /// +0x24 u32 selector (jump-table 0..0x1D), +0x28/+0x2C/+0x30
        /// u32 in/out. Live sel=2 → {out=10, 0x7FFFFFFF, 64, 256(Q8 1.0?)};
        /// sel 0/1/3/4 → -104 on TU116.
        pub struct NV_GPU_THERMAL_HWFS_CONTROL_V1 {
            pub version: NvVersion,
            /// +4: u8 input selector pass-through
            pub b04_in: u8,
            /// +5: u8 output (driver-filled)
            pub b05_out: u8,
            /// +6..+0x24: opaque
            pub pad: [u8; 30],
            /// +0x24: u32 selector (domain 0..0x1D via jump table)
            pub selector: u32,
            /// +0x28: u32 out (0x7FFFFFFF = "not set" hypothesis)
            pub out_a: u32,
            /// +0x2C: u32 in/out (live 64)
            pub out_b: u32,
            /// +0x30: u32 in/out (live 256 — Q8 1.0 hypothesis)
            pub out_c: u32,
        }
    }

    nvversion! { @=NV_GPU_THERMAL_HWFS_CONTROL NV_GPU_THERMAL_HWFS_CONTROL_V1(1) = 0x34 }

    nvstruct! {
        /// Thermal slowdown state (ID 0x6683EE65, NO RM — escape
        /// 0x0700011, cross-generation stable; the cheapest
        /// "is the GPU thermally slowed" boolean). 0 = normal,
        /// 0xFFFF = slowdown, other driver replies map to -1.
        pub struct NV_GPU_THERMAL_SLOWDOWN_STATE_V1 {
            pub version: NvVersion,
            pub state: u32,
        }
    }

    nvversion! { @=NV_GPU_THERMAL_SLOWDOWN_STATE NV_GPU_THERMAL_SLOWDOWN_STATE_V1(1) = 0x8 }

    // P2 FFI (IDs 0x2BC9F805 / 0xAFA4113C / 0x58F4F4C1 / 0x45C064D5 /
    // 0x14277C24 / 0x6683EE65 — registered in nvid.rs, first-time FFI).

    nvapi! {
        /// NAFLL device directory (ID 0x2BC9F805, magic 0x20C58 v2).
        pub unsafe fn NvAPI_GPU_ClockNafllDevicesGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_CLOCK_NAFLL_DEVICES_INFO) -> NvAPI_Status;
    }

    nvapi! {
        /// NAFLL devices status (ID 0xAFA4113C, v1 magic 0x10F48).
        /// MASK-SEEDED at +4.
        pub unsafe fn NvAPI_GPU_ClockNafllDevicesGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pStatus: *mut NV_GPU_CLOCK_NAFLL_DEVICES_STATUS) -> NvAPI_Status;
    }

    nvapi! {
        /// ClkFreqController directory (ID 0x58F4F4C1, magic 0x10C4C).
        pub unsafe fn NvAPI_GPU_ClockClkFreqControllerGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_CLOCK_CLK_FREQ_CONTROLLER_INFO) -> NvAPI_Status;
    }

    nvapi! {
        /// ClkFreqController live status V1 (ID 0x45C064D5, magic 0x109CC).
        /// MASK-SEEDED at +4.
        pub unsafe fn NvAPI_GPU_ClockClkFreqControllerGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pStatus: *mut NV_GPU_CLOCK_CLK_FREQ_CONTROLLER_STATUS) -> NvAPI_Status;
    }

    nvapi! {
        /// HWFS control GET (ID 0x14277C24, magic 0x10034, flat 52B).
        pub unsafe fn NvAPI_GPU_ThermalHwFsGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_THERMAL_HWFS_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Thermal slowdown state (ID 0x6683EE65, 8B, no magic gate —
        /// version dword still required as first field).
        pub unsafe fn NvAPI_GPU_GetThermalSlowdownState(hPhysicalGPU: NvPhysicalGpuHandle, pState: *mut NV_GPU_THERMAL_SLOWDOWN_STATE) -> NvAPI_Status;
    }

    nvapi! {
        /// ADC device directory V1 (ID 0x68789E2A, magic 0x10348, 10
        /// slots). Parts with more devices reject it with -174 — fall
        /// back to [`NvAPI_GPU_ClockAdcDevicesGetInfoV2`].
        pub unsafe fn NvAPI_GPU_ClockAdcDevicesGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_CLOCK_ADC_DEVICES_INFO) -> NvAPI_Status;
    }

    /// ADC device directory V2 entry point — SAME QI id as
    /// [`NvAPI_GPU_ClockAdcDevicesGetInfo`] (0x68789E2A); the stamp in the
    /// caller's buffer selects the slot count. Declared by hand (not via
    /// `nvapi!`) because the macro derives the `Api` variant from the fn
    /// name and the enum cannot carry a duplicate id.
    pub unsafe fn NvAPI_GPU_ClockAdcDevicesGetInfoV2(
        hPhysicalGPU: NvPhysicalGpuHandle,
        pInfo: *mut NV_GPU_CLOCK_ADC_DEVICES_INFO2,
    ) -> NvAPI_Status {
        // identical signature up to the pointee; route through the V1
        // symbol so the QI cache/log path stays shared
        unsafe {
            NvAPI_GPU_ClockAdcDevicesGetInfo(
                hPhysicalGPU,
                pInfo as *mut NV_GPU_CLOCK_ADC_DEVICES_INFO,
            )
        }
    }

    nvapi! {
        /// ADC live status V1 (ID 0x43D9B26A, magic 0x10340) — ★P1 live
        /// rail-voltage telemetry (the only dynamic sensor stream in the
        /// nvClocks audit). MASK-SEEDED at +4.
        pub unsafe fn NvAPI_GPU_ClockAdcDevicesGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pStatus: *mut NV_GPU_CLOCK_ADC_DEVICES_STATUS) -> NvAPI_Status;
    }

    /// ADC live status V2 entry point — SAME QI id as
    /// [`NvAPI_GPU_ClockAdcDevicesGetStatus`]; the stamp in the caller's
    /// buffer (0x10340 vs 0x109C8) selects the slot count. Hand-written
    /// wrapper over the V1 symbol (the `nvapi!` macro derives the `Api`
    /// variant from the fn name; the enum cannot carry a duplicate id).
    pub unsafe fn NvAPI_GPU_ClockAdcDevicesGetStatusV2(
        hPhysicalGPU: NvPhysicalGpuHandle,
        pStatus: *mut NV_GPU_CLOCK_ADC_DEVICES_STATUS_V2,
    ) -> NvAPI_Status {
        unsafe {
            NvAPI_GPU_ClockAdcDevicesGetStatus(
                hPhysicalGPU,
                pStatus.cast::<NV_GPU_CLOCK_ADC_DEVICES_STATUS>(),
            )
        }
    }

    nvapi! {
        /// ClkPropRegimes directory (ID 0xCF08E934, magic 0x10A8C) — P1;
        /// settles the archived scaling-sibling mystery ID.
        pub unsafe fn NvAPI_GPU_ClockClkPropRegimesGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_CLOCK_CLK_PROP_REGIMES_INFO) -> NvAPI_Status;
    }

    nvapi! {
        /// ClkPropRegimes control snapshot (ID 0x4F11EAA4, magic 0x12148).
        /// MASK-SEEDED at +8.
        pub unsafe fn NvAPI_GPU_ClockClkPropRegimesGetControl(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *mut NV_GPU_CLOCK_CLK_PROP_REGIMES_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Per-domain legal frequency enumeration (ID 0x40BDDDB36, magic
        /// 0x10808) — MHz units, selector-driven.
        pub unsafe fn NvAPI_GPU_ClockClkDomainFreqsEnum(hPhysicalGPU: NvPhysicalGpuHandle, pFreqsEnum: *mut NV_GPU_CLOCK_CLK_DOMAIN_FREQS_ENUM) -> NvAPI_Status;
    }

    nvapi! {
        /// Public clock info (ID 0x1B46D4CC, magic 0x10188, "escDomainData"
        /// wire type, no RM). Pre-fill slots via
        /// [`NV_GPU_PUBLIC_CLOCK_INFO::preset_defaults`] before calling.
        pub unsafe fn NvAPI_GPU_GetPublicClockInfo(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_PUBLIC_CLOCK_INFO) -> NvAPI_Status;
    }

    nvapi! {
        /// Locked-clock mode status (ID 0xC4733F19, magic 0x10008, 8B).
        pub unsafe fn NvAPI_GPU_GetLockedClockModeStatus(hPhysicalGPU: NvPhysicalGpuHandle, pStatus: *mut NV_GPU_LOCKED_CLOCK_MODE_STATUS) -> NvAPI_Status;
    }

    /// Layout regression tests for the nvClocks P1 batch: magic closure
    /// and accessor round-trips at the audit-derived offsets.
    #[cfg(test)]
    mod p1_batch_tests {
        use super::*;

        fn put_u32(rest: &mut [u8], abs: usize, v: u32) {
            rest[abs - 4..abs].copy_from_slice(&v.to_le_bytes());
        }

        #[test]
        fn p1_magic_closure() {
            // ADC INFO: 0x50 + 10*0x4C == 0x348 (840)
            assert_eq!(
                adc_devices_info_entry::BASE + 10 * adc_devices_info_entry::STRIDE,
                0x348
            );
            assert_eq!(size_of::<NV_GPU_CLOCK_ADC_DEVICES_INFO>(), 840);
            // ADC STATUS: 0x48 + 10*0x4C == 0x340 (832)
            assert_eq!(
                adc_devices_status_entry::BASE + 10 * adc_devices_status_entry::STRIDE,
                0x340
            );
            assert_eq!(size_of::<NV_GPU_CLOCK_ADC_DEVICES_STATUS>(), 832);
            // REGIMES INFO: 0x10C + 32*0x4C == 0xA8C (2700)
            assert_eq!(
                clk_prop_regimes_info_entry::BASE + 32 * clk_prop_regimes_info_entry::STRIDE,
                0xA8C
            );
            assert_eq!(size_of::<NV_GPU_CLOCK_CLK_PROP_REGIMES_INFO>(), 2700);
            // REGIMES CONTROL: 0x48 + 32*0x108 == 0x2148 (8520)
            assert_eq!(
                clk_prop_regimes_ctrl_entry::BASE + 32 * clk_prop_regimes_ctrl_entry::STRIDE,
                0x2148
            );
            assert_eq!(size_of::<NV_GPU_CLOCK_CLK_PROP_REGIMES_CONTROL>(), 8520);
            // FREQS ENUM: 8 + 512*4 == 0x808 (2056)
            assert_eq!(size_of::<NV_GPU_CLOCK_CLK_DOMAIN_FREQS_ENUM>(), 2056);
            // PUBLIC CLOCK INFO: 8 + 32*12 == 0x188 (392)
            assert_eq!(size_of::<NV_GPU_PUBLIC_CLOCK_INFO>(), 392);
            // LOCKED CLOCK MODE: 4 + 4 == 0x8
            assert_eq!(size_of::<NV_GPU_LOCKED_CLOCK_MODE_STATUS>(), 8);
        }

        #[test]
        fn p1_accessors() {
            // ADC status record decode: live-shaped record
            let mut st = NV_GPU_CLOCK_ADC_DEVICES_STATUS::default();
            st.set_mask(0xF);
            assert_eq!(st.mask(), 0xF);
            let base = adc_devices_status_entry::BASE + 2 * adc_devices_status_entry::STRIDE;
            put_u32(
                &mut st.rest,
                base + adc_devices_status_entry::REC_STATE,
                0xFFFF_FFFF,
            );
            put_u32(
                &mut st.rest,
                base + adc_devices_status_entry::REC_VALUE_UV,
                631_250,
            );
            st.rest[base + adc_devices_status_entry::REC_VFPID_A - 4] = 29;
            st.rest[base + adc_devices_status_entry::REC_TYPE - 4] = 2;
            put_u32(
                &mut st.rest,
                base + adc_devices_status_entry::REC_VALUE2,
                0x7FFF_FFFF,
            );
            assert_eq!(
                st.rec_u32(2, adc_devices_status_entry::REC_STATE),
                Some(0xFFFF_FFFF)
            );
            assert_eq!(
                st.rec_u32(2, adc_devices_status_entry::REC_VALUE_UV),
                Some(631_250)
            );
            assert_eq!(
                st.rest[base + adc_devices_status_entry::REC_VFPID_A - 4],
                29
            );
            assert_eq!(st.rec_u32(2, adc_devices_status_entry::REC_TYPE), Some(2));
            assert_eq!(
                st.rec_u32(2, adc_devices_status_entry::REC_VALUE2),
                Some(0x7FFF_FFFF)
            );

            // Regimes info record decode
            let mut info = NV_GPU_CLOCK_CLK_PROP_REGIMES_INFO::default();
            put_u32(&mut info.rest, clk_prop_regimes_info_entry::MASK, 0x3);
            let base = clk_prop_regimes_info_entry::BASE + 1 * clk_prop_regimes_info_entry::STRIDE;
            put_u32(
                &mut info.rest,
                base + clk_prop_regimes_info_entry::REC_STATUS,
                0,
            );
            put_u32(
                &mut info.rest,
                base + clk_prop_regimes_info_entry::REC_TYPE,
                0xF,
            );
            put_u32(
                &mut info.rest,
                base + clk_prop_regimes_info_entry::REC_VALUE,
                1470,
            );
            assert_eq!(
                info.rec_u32(1, clk_prop_regimes_info_entry::REC_TYPE),
                Some(0xF)
            );
            assert_eq!(
                info.rec_u32(1, clk_prop_regimes_info_entry::REC_VALUE),
                Some(1470)
            );

            // Regimes control: mask seed + sparse record fields
            let mut ctl = NV_GPU_CLOCK_CLK_PROP_REGIMES_CONTROL::default();
            ctl.set_mask(1 << 5);
            let base = clk_prop_regimes_ctrl_entry::BASE + 5 * clk_prop_regimes_ctrl_entry::STRIDE;
            put_u32(
                &mut ctl.rest,
                base + clk_prop_regimes_ctrl_entry::REC_VALUE,
                1638400,
            );
            assert_eq!(
                ctl.rec_u32(5, clk_prop_regimes_ctrl_entry::REC_VALUE),
                Some(1_638_400)
            );
            assert_eq!(
                ctl.rec_u32(6, clk_prop_regimes_ctrl_entry::REC_VALUE),
                Some(0)
            );

            // Freqs enum decode
            let mut fe = NV_GPU_CLOCK_CLK_DOMAIN_FREQS_ENUM::default();
            fe.set_selector(2);
            fe.rest[2..4].copy_from_slice(&3u16.to_le_bytes());
            for (k, v) in [405u32, 810, 6001].into_iter().enumerate() {
                let o = 4 + k * 4;
                fe.rest[o..o + 4].copy_from_slice(&v.to_le_bytes());
            }
            assert_eq!(fe.count(), 3);
            assert_eq!(fe.freqs_mhz(), vec![405, 810, 6001]);

            // Public clock info: presets + typed landing zone
            let mut pc = NV_GPU_PUBLIC_CLOCK_INFO::default();
            pc.preset_defaults();
            assert_eq!(pc.u32_at(public_clock_info_slots::TYPE1), Some(32));
            assert_eq!(pc.u32_at(public_clock_info_slots::TYPE1 + 8), Some(100));
            put_u32(&mut pc.rest, public_clock_info_slots::TYPE4 + 4, 4);
            assert_eq!(pc.u32_at(public_clock_info_slots::TYPE4 + 4), Some(4));

            // Locked clock mode: plain field struct
            let mut lm = NV_GPU_LOCKED_CLOCK_MODE_STATUS::default();
            lm.mode_mask = 0b1010;
            assert_eq!(lm.mode_mask, 0b1010);
        }
    }
}
