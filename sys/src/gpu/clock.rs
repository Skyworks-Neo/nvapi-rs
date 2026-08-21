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
            pub hdr: Padding<[u32; 33]>,
            /// Byte 0x88 (dword 34) = bitmask of present pstates (bit i ⇔ P{i}).
            pub pstate_mask: u32,
            /// Byte 0x8C (dword 35) low byte = table version (logged by the ref tool).
            pub table_version: u8,
            pub rsvd0: Padding<[u8; 3]>,
            /// Bytes 0x90..(then the slot + freq tables). Header above = 144 B.
            /// Total struct = 275152 B (the ref tool's memset clears 0x432CC bytes from
            /// v19[1], i.e. struct = 4 + 0x432CC = 0x432D0 = 275152; the version
            /// magic with_struct(4) yields exactly 0x432D0).
            pub payload: Padding<[u8; 275152 - 144]>,
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
            pub entries: Padding<[u8; 164 - 8]>,
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
            self.record_u32(bit, clk_ctrl_entry::OFFSET_KHZ).map(|v| v as i32)
        }

        /// Range minimum (i32 @record+48) for domain `bit`.
        pub fn range_min(&self, bit: u32) -> Option<i32> {
            self.record_u32(bit, clk_ctrl_entry::RANGE_MIN).map(|v| v as i32)
        }

        /// Range maximum (i32 @record+52) for domain `bit`.
        pub fn range_max(&self, bit: u32) -> Option<i32> {
            self.record_u32(bit, clk_ctrl_entry::RANGE_MAX).map(|v| v as i32)
        }

        /// Applied value (i32 @record+56) for domain `bit`.
        pub fn applied(&self, bit: u32) -> Option<i32> {
            self.record_u32(bit, clk_ctrl_entry::APPLIED).map(|v| v as i32)
        }

        /// Write the signed kHz offset (i32 @record+44) for domain `bit`.
        pub fn set_offset_khz(&mut self, bit: u32, offset_khz: i32) -> Option<()> {
            self.set_record_u32(bit, clk_ctrl_entry::OFFSET_KHZ, offset_khz as u32)
        }

        /// Iterate (bit, type, offset_kHz, range_min, range_max, applied) for
        /// every domain the driver actually filled a record for (record type
        /// != 0). This derives the TRUE controllable set from filled records
        /// rather than trusting the echoed +8 mask (which is just the seed).
        pub fn entries(
            &self,
        ) -> impl Iterator<Item = (u32, u8, i32, i32, i32, i32)> + '_ {
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
        }
    }

    nvversion! { @=NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE2 NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE_V2(2) = 0x20 }

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
            let off = clk_measure_v3::ENTRIES + clk_measure_v3::STRIDE * i + field_off
                - 4;
            let end = off.checked_add(len)?;
            if end <= self.entries.len() { Some(off) } else { None }
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
        pub fn set_entry(&mut self, i: usize, domain: u32, counter: u64, timestamp_ns: u64) -> Option<()> {
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
            if end <= self.rest.len() { Some(off) } else { None }
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
            self.rec_off(bit, clk_ctrl_entry_v2::VALUES + 4 * i, 4).and_then(|off| {
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
    pub type NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE = NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1;

    impl NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1 {
        /// Literal magic dword the GetInfo handler accepts (live-verified).
        pub const MAGIC: u32 = 0x78604;
    }

    impl Default for NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1 {
        fn default() -> Self {
            Self { version: NvVersion::with_version(Self::MAGIC), rest: [0; 493056] }
        }
    }

    impl NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE_V1 {
        fn off(&self, abs: usize, len: usize) -> Option<usize> {
            let off = abs.checked_sub(4)?;
            let end = off.checked_add(len)?;
            if end <= self.rest.len() { Some(off) } else { None }
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
            let mask_base = if bank == 0 { clk_vfp_info::MASK1 } else { clk_vfp_info::MASK2 };
            let dword = self.u32_at(mask_base + 4 * (idx >> 5))?;
            Some(dword & (1 << (idx & 31)) != 0)
        }

        /// Descriptor type byte (u8 @desc+0) for point `idx` in bank `bank`.
        pub fn point_type(&self, bank: usize, idx: usize) -> Option<u8> {
            if bank > 1 || idx >= clk_vfp_info::POINTS {
                return None;
            }
            let base = if bank == 0 { clk_vfp_info::DESC1 } else { clk_vfp_info::DESC2 };
            let off = self.off(base + clk_vfp_info::DESC_STRIDE * idx, 1)?;
            self.rest.get(off).copied()
        }

        /// Copy the +4..+132 mask output into `status`' +4..+132 header —
        /// GetStatus REQUIRES this seed (zero → no records, garbage → -1).
        pub fn seed_status_header(&self, status: &mut NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1) {
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
        /// point voltage (u32 µV; the V/F grid axis), mirrored @+0x68
        pub const VOLTAGE_UV: usize = 0x58;
        /// current/effective frequency (u32 MHz; = default + applied delta)
        pub const FREQ_CURRENT_MHZ: usize = 0x64;
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

    pub type NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE = NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1;

    impl NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1 {
        /// Literal magic dword the GetStatus handler accepts: the largest of
        /// {85016, 158200, 214652, 300164, 1525252, 2000388} — the full
        /// 2×2048-record layout (live-verified).
        pub const MAGIC: u32 = 2000388;
    }

    impl Default for NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1 {
        fn default() -> Self {
            Self { version: NvVersion::with_version(Self::MAGIC), rest: [0; 2000384] }
        }
    }

    impl NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE_V1 {
        fn off(&self, abs: usize, len: usize) -> Option<usize> {
            let off = abs.checked_sub(4)?;
            let end = off.checked_add(len)?;
            if end <= self.rest.len() { Some(off) } else { None }
        }

        fn off_mut(&mut self, abs: usize, len: usize) -> Option<usize> {
            let off = abs.checked_sub(4)?;
            let end = off.checked_add(len)?;
            if end <= self.rest.len() { Some(off) } else { None }
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
            Some(if bank == 0 { clk_vfp_status::REC1 } else { clk_vfp_status::REC2 }
                + clk_vfp_status::STRIDE * idx)
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
    /// - rec+56: u32 value (mode 0) or i16 delta (mode 1); for bank-2
    ///   type-8 records this is the V/F point's programmed value
    /// - rec+96 (byte): passthrough flag (bank-2 only)
    pub mod clk_vfp_control {
        /// canonical magic (accepted input and internal fill stamp)
        pub const MAGIC: u32 = 4670980;
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

    pub type NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE = NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE_V1;

    impl NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE_V1 {
        fn off(&self, abs: usize, len: usize) -> Option<usize> {
            let off = abs.checked_sub(4)?;
            let end = off.checked_add(len)?;
            if end <= self.rest.len() { Some(off) } else { None }
        }

        fn off_mut(&mut self, abs: usize, len: usize) -> Option<usize> {
            let off = abs.checked_sub(4)?;
            let end = off.checked_add(len)?;
            if end <= self.rest.len() { Some(off) } else { None }
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
                let n = 128
                    .min(self.rest.len() - dst)
                    .min(info.rest.len() - src);
                self.rest[dst..dst + n].copy_from_slice(&info.rest[src..src + n]);
            }
        }

        fn rec_base(bank: usize, idx: usize) -> Option<usize> {
            if bank > 1 || idx >= clk_vfp_control::POINTS {
                return None;
            }
            Some(if bank == 0 { clk_vfp_control::REC1 } else { clk_vfp_control::REC2 }
                + clk_vfp_control::STRIDE * idx)
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
    }

    impl Default for NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE_V1 {
        fn default() -> Self {
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
}
