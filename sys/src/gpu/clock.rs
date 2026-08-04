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

    // ------------------------------------------------------------------
    // PerfPstatesGetInfoPrivate (NDA, ID 0x7B30AE0D) — the P-State level
    // table behind GPUMon's `-pstate` GET ("Level[N] P*.Max/P*.Min").
    //
    // RE'd from GPUMon `[GPUHandle::queryPStateInfo]` (thunk sub_140003A20).
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
        /// Perf P-states info (RE'd from GPUMon; NDA). Opaque except for the
        /// bitmask/version header and the decoded accessors below.
        pub struct NV_GPU_PERF_PSTATES_INFO_PRIVATE_V4 {
            pub version: NvVersion,
            /// dwords 1..34 (opaque header). Bytes 4..0x88.
            pub hdr: Padding<[u32; 33]>,
            /// Byte 0x88 (dword 34) = bitmask of present pstates (bit i ⇔ P{i}).
            pub pstate_mask: u32,
            /// Byte 0x8C (dword 35) low byte = table version (logged by GPUMon).
            pub table_version: u8,
            pub rsvd0: Padding<[u8; 3]>,
            /// Bytes 0x90..(then the slot + freq tables). Header above = 144 B.
            /// Total struct = 275152 B (GPUMon's memset clears 0x432CC bytes from
            /// v19[1], i.e. struct = 4 + 0x432CC = 0x432D0 = 275152; the version
            /// magic with_struct(4) yields exactly 0x432D0).
            pub payload: Padding<[u8; 275152 - 144]>,
        }
    }

    impl NV_GPU_PERF_PSTATES_INFO_PRIVATE_V4 {
        // Freq table layout (RE'd from GPUMon queryPStateInfo loop):
        //   max_kHz byte offset = 0x22F0 + slot*0x2090 + domain*0x9C
        //   min_kHz byte offset = 0x22C8 + slot*0x2090 + domain*0x9C
        // where:
        //   - `slot` = the k-th set bit in `pstate_mask` (one slot per present
        //     pstate, in ascending bit order). NOT the pstate NUMBER — each slot
        //     is 0x2090 (8336) bytes apart.
        //   - `domain` = clock-domain index (0=GPC/core typically; GPUMon
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

        /// Table version byte (GPUMon logs this as "P state version: 0x%X").
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
        /// (0=GPC/core by default; GPUMon resolves it via 0x57B5A5DF).
        /// Mirrors GPUMon's queryPStateInfo loop.
        pub fn pstate_entries_domain(&self, domain: usize) -> Vec<PStateEntryRaw> {
            let mut out = Vec::new();
            for bit in 0u32..32 {
                if (self.pstate_mask >> bit) & 1 == 0 {
                    continue;
                }
                // Slot index = number of set bits already emitted (GPUMon's v10
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
        /// in kHz). Source of GPUMon's `-pstate` GET listing. Returns a
        /// 275152-byte struct with version magic 0x432D0 (version 4).
        pub unsafe fn NvAPI_GPU_PerfPstatesGetInfoPrivate(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_PERF_PSTATES_INFO_PRIVATE) -> NvAPI_Status;
    }

    // ------------------------------------------------------------------
    // ClientPStateLimitStatus (NDA, ID 0x9962C97C) — the "which P-States are
    // currently locked" view. RE'd from GPUMon's `[GPUHandle::pollPState]`
    // "get p state limit" branch (thunk sub_140003D60). GPUMon allocates a
    // 164-byte buffer but the driver's version magic 0x10088 reports size 136
    // (v1) — the tail is padding. Entries start at byte 8, each 2 bytes
    // {type:u8, pstate:u8}; type == 0x1A marks a pstate locked by
    // PerfClientLimitsSetStatus (0x39442CFB). GPUMon renders the locked set as
    // "P0.P3.P5".
    // ------------------------------------------------------------------

    nvstruct! {
        /// P-State limit-status (RE'd from GPUMon; NDA). Opaque except for the
        /// count + entry table decoded by the accessor below.
        pub struct NV_GPU_CLIENT_PSTATE_LIMIT_STATUS_V1 {
            pub version: NvVersion,
            /// Number of valid entries in `entries`.
            pub count: u32,
            /// Entry table: count × {type:u8, pstate:u8}, type==0x1A = locked.
            /// 164-byte buffer total (driver magic reports 136; tail is pad).
            pub entries: Padding<[u8; 164 - 8]>,
        }
    }

    impl NV_GPU_CLIENT_PSTATE_LIMIT_STATUS_V1 {
        /// The set of P-State numbers currently locked, in entry order. Each
        /// entry is `{type:u8, pstate:u8}`; GPUMon's pollPState only renders
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
}
