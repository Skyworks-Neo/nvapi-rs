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
            /// uiDelta — unsigned percent of boost range, clamped [0, 100]
            /// (AmpereOC + HYDRA both treat as unsigned; never negative).
            pub percent: u32,
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

    // --- melonVolt-path VoltRails family (READ-ONLY) ------------------------
    // RE'd from melonVolt.exe / melonVoltDiag.exe + nvapi64_impl.dll 610.74
    // (reverse/melonvolt/ANALYSIS.md). These are the private siblings the
    // public Client trio forwards to internally; on this driver branch the
    // whole family is exposed in the PUBLIC QueryInterface table, so none of
    // melonVolt's runtime code-scanning is needed.
    //
    // Layout (opaque beyond the documented header; accessors use byte offsets):
    //   rail-info entries: 192-byte stride indexed by rail BIT, type u32 @+76
    //   ctrl/status entries: 84-byte stride indexed DENSELY (set bits only),
    //     seed/type u32 @+72 seeded from rail entry +192*bit+76, then SIX u32
    //     @+76..+100 which SPAN PAST the slot stride — the driver's own getter
    //     copies exactly those six.
    // RM layer: escape 0x07000191, ctrl cmds 0x2080A601 (info) / 0x2080A613
    // (control), ~500 KB driver-internal buffers.
    //
    // VoltVoltRailsSetControl 0x87C55C8A (the µV-offset WRITE path melonVolt
    // drives) is wrapped in the medium layer with the full melonVolt protocol
    // (snapshot -> locate -> sanity -> write -> SET -> readback verify);
    // do NOT call the raw FFI directly.

    /// Byte offsets into the per-rail entries of
    /// [`NV_GPU_VOLT_RAILS_INFO`].
    pub mod rail_entry {
        /// stride per rail BIT index
        pub const STRIDE: usize = 192;
        /// u32 type discriminator (copied into control/status entry seeds)
        pub const TYPE: usize = 76;
    }

    /// Byte offsets into the dense per-rail entries of the control/status
    /// structs.
    pub mod ctrl_entry {
        /// stride per DENSE entry (set bits only, in ascending order)
        pub const STRIDE: usize = 84;
        /// u32 type discriminator (seed input, validated/filled output)
        pub const TYPE: usize = 72;
        /// six u32 payload (µV on voltage/offset entries); starts at +76 and
        /// spans past the 84-byte slot stride into the next slot's unused head
        pub const VALUES: usize = 76;
        pub const VALUES_LEN: usize = 6;
    }

    /// Semantics of the SIX payload u32 in a **status** entry of type 1
    /// (live voltage reading; confirmed on RTX 4060 Laptop / 610.74 and
    /// cross-checked against desktop 20/30-series):
    ///
    /// | index | meaning                                                        |
    /// |-------|----------------------------------------------------------------|
    /// | 0     | current core-rail voltage (live: 0.63 V idle → 0.94 V load)    |
    /// | 1     | target voltage wall (the value the SET side requested)         |
    /// | 2     | vBIOS voltage wall — 0 on mobile; on desktop a hard cap the    |
    /// |       | final effective wall (index 4) cannot exceed                   |
    /// | 3     | VRM-max wall — the max wall the VRM (voltage regulator) can    |
    /// |       | sustain (1.200 V on observed GPUs)                             |
    /// | 4     | effective wall — the final clamped wall actually in force       |
    /// |       | (min of target [1], vBIOS wall [2], VRM-max [3] after clamps)   |
    /// | 5     | P0 core-domain MIN hold voltage (lowest voltage that sustains  |
    /// |       | P0) — the lower bound the old brute-force VFP-lock scan probed |
    ///
    /// The effective wall (index 4) = min(target [1], vBIOS wall [2] if set,
    /// VRM-max [3]); index 1 mirrors index 4 when nothing is clamping.
    /// Indices 1/5 replace `handle_test_voltage_limits`' trial-and-error
    /// VFP-point locking as a direct µV source for the P0 bounds.
    ///
    /// A **control** entry's payload: `values[0]` is the µV offset melonVolt
    /// writes (same role for all types). The TYPE field (+72) distinguishes
    /// the VoltRails control/descriptor-format version — NOT writability and
    /// NOT per-generation architecture (10/20/30/40-series all report type 0,
    /// so type is not a generation marker). Type 0 = legacy format (Pascal→Ada,
    /// 10–40 series); type 3 = Blackwell format (50 series); type 2 = unobserved
    /// intermediate. All three are writable (IDA `sub_18015B6E0` SET encoder
    /// returns success for 0/2/3). 4060 Laptop
    /// type=0 values are all-zero only because stock offset = 0; the wall is
    /// empirically raisable to 1.2V. `values[1..5]` are an opaque blob the
    /// driver blind-copies (SET commit `sub_1801D2450`) with no per-type
    /// dispatch — firmware-interpreted, not driver-interpreted.
    pub mod status_values {
        pub const CURRENT_UV: usize = 0;
        pub const TARGET_WALL_UV: usize = 1;
        pub const VBIOS_WALL_UV: usize = 2;
        pub const VRM_MAX_WALL_UV: usize = 3;
        pub const EFFECTIVE_WALL_UV: usize = 4;
        pub const P0_MIN_HOLD_UV: usize = 5;
    }

    nvstruct! {
        pub struct NV_GPU_VOLT_RAILS_INFO_V2 {
            pub version: NvVersion,
            /// out: bitmask of present rails (RTX 5090: 0x2 = MSVDD @ bit 1;
            /// RTX 4060 Laptop: 0x1 = single core rail)
            pub rail_mask: u32,
            pub rest: [u8; 6212],
        }
    }

    nvversion! { @=NV_GPU_VOLT_RAILS_INFO NV_GPU_VOLT_RAILS_INFO_V2(2) = 6220 }

    impl NV_GPU_VOLT_RAILS_INFO {
        /// Type discriminator of the rail entry for `bit` (u32 @+192*bit+76).
        pub fn rail_type(&self, bit: u32) -> Option<u32> {
            let base = rail_entry::STRIDE.checked_mul(bit as usize)?;
            let off = base + rail_entry::TYPE;
            let end = off + 4;
            let raw = self.rest.get(off - 8..end - 8)?;
            Some(u32::from_le_bytes(raw.try_into().ok()?))
        }

        /// Raw 192-byte rail descriptor for `bit` as 48 little-endian u32.
        /// Only the type @dword 19 is decoded so far — the rest is undecoded
        /// driver data (observed non-zero on 4060 Laptop); dumped for
        /// cross-platform comparison.
        ///
        /// Rail entry 0 starts at struct offset 0, so its first 8 bytes
        /// overlap the version/mask header — dword 0/1 of entry 0 are the
        /// version/mask, not rail data. Entries are read from the struct base
        /// (not `rest`, which begins at offset 8) to avoid underflow.
        pub fn rail_entry_raw(&self, bit: u32) -> Option<Vec<u32>> {
            let base = rail_entry::STRIDE.checked_mul(bit as usize)?;
            // struct size = 8 (version+mask) + rest.len(); entry must fit
            if base + rail_entry::STRIDE > 8 + self.rest.len() {
                return None;
            }
            let mut out = Vec::with_capacity(rail_entry::STRIDE / 4);
            for i in 0..rail_entry::STRIDE / 4 {
                let off = base + 4 * i; // struct offset
                let raw: [u8; 4] = if off < 8 {
                    // head: dword 0 = version, dword 1 = rail_mask
                    let mut b = [0u8; 4];
                    if off == 0 {
                        b = self.version.data.to_le_bytes();
                    } else if off == 4 {
                        b = self.rail_mask.to_le_bytes();
                    }
                    b
                } else {
                    let r = self.rest.get(off - 8..off - 4)?;
                    r.try_into().ok()?
                };
                out.push(u32::from_le_bytes(raw));
            }
            Some(out)
        }
    }

    nvstruct! {
        pub struct NV_GPU_VOLT_RAILS_CONTROL_V2 {
            pub version: NvVersion,
            /// in: bitmask of rails to read (dense entry selection)
            pub rail_mask: u32,
            pub rest: [u8; 2752],
        }
    }

    nvversion! { @=NV_GPU_VOLT_RAILS_CONTROL NV_GPU_VOLT_RAILS_CONTROL_V2(2) = 2760 }

    nvstruct! {
        /// Live-voltage variant: identical layout, but the driver only accepts
        /// the V1 version stamp 0x10AC8 (68296) here.
        pub struct NV_GPU_VOLT_RAILS_STATUS_V1 {
            pub version: NvVersion,
            /// in: bitmask of rails to read
            pub rail_mask: u32,
            pub rest: [u8; 2752],
        }
    }

    nvversion! { @=NV_GPU_VOLT_RAILS_STATUS NV_GPU_VOLT_RAILS_STATUS_V1(1) = 2760 }

    /// Seed/parse helpers shared by the control and status structs.
    macro_rules! volt_rails_entries {
        ($t:ty) => {
            impl $t {
                /// Copy the rail-type seeds from a filled
                /// [`NV_GPU_VOLT_RAILS_INFO`] into the dense entries.
                pub fn seed_from_info(&mut self, info: &NV_GPU_VOLT_RAILS_INFO) {
                    self.rail_mask = info.rail_mask;
                    let mut dense = 0usize;
                    for bit in 0..32u32 {
                        if info.rail_mask & (1 << bit) == 0 {
                            continue;
                        }
                        let typ = info.rail_type(bit).unwrap_or(0).to_le_bytes();
                        let dst = ctrl_entry::STRIDE * dense + ctrl_entry::TYPE;
                        if dst + 4 <= 8 + self.rest.len() {
                            self.rest[dst - 8..dst - 4].copy_from_slice(&typ);
                        }
                        dense += 1;
                    }
                }

                /// Iterate the dense entries as (rail_bit, type, six payload u32).
                pub fn entries(&self) -> impl Iterator<Item = (u32, u32, [i32; 6])> + '_ {
                    let mask = self.rail_mask;
                    let rest = &self.rest;
                    (0..32u32)
                        .filter(move |bit| mask & (1 << bit) != 0)
                        .enumerate()
                        .filter_map(move |(dense, bit)| {
                            let base = ctrl_entry::STRIDE * dense + ctrl_entry::TYPE;
                            if base + 4 + 4 * ctrl_entry::VALUES_LEN > 8 + rest.len() {
                                return None;
                            }
                            let typ = u32::from_le_bytes(rest[base - 8..base - 4].try_into().ok()?);
                            let mut values = [0i32; ctrl_entry::VALUES_LEN];
                            for (i, v) in values.iter_mut().enumerate() {
                                let off = base + 4 + 4 * i - 8;
                                *v = i32::from_le_bytes(rest[off..off + 4].try_into().ok()?);
                            }
                            Some((bit, typ, values))
                        })
                }
            }
        };
    }

    volt_rails_entries!(NV_GPU_VOLT_RAILS_CONTROL_V2);
    volt_rails_entries!(NV_GPU_VOLT_RAILS_STATUS_V1);

    nvapi! {
        /// Private VoltRails "rail builder" (melonVolt's name): fills the rail
        /// mask + per-rail descriptors. Verified live on driver 610.74.
        pub unsafe fn NvAPI_GPU_VoltVoltRailsGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pRailInfo: *mut NV_GPU_VOLT_RAILS_INFO) -> NvAPI_Status;
    }

    nvapi! {
        /// Private VoltRails control-object GET (per-rail offset entries).
        /// The struct must be seeded from a prior GetInfo call.
        pub unsafe fn NvAPI_GPU_VoltVoltRailsGetControl(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *mut NV_GPU_VOLT_RAILS_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Private VoltRails live-status GET (per-rail voltages, µV).
        /// V1-stamped (0x10AC8) struct required; seeded like GetControl.
        pub unsafe fn NvAPI_GPU_VoltVoltRailsGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pStatus: *mut NV_GPU_VOLT_RAILS_STATUS) -> NvAPI_Status;
    }

    nvapi! {
        /// Private VoltRails control-object SET (the µV-offset write path
        /// melonVolt drives on RTX 5090 MSVDD, rail bit 1, entry type 3).
        /// Writes the WHOLE control object — always GET, patch, then SET,
        /// and read back to verify the driver retained the value.
        pub unsafe fn NvAPI_GPU_VoltVoltRailsSetControl(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *const NV_GPU_VOLT_RAILS_CONTROL) -> NvAPI_Status;
    }

    nvstruct! {
        pub struct NV_GPU_CLOCK_CLIENT_CLK_VF_POINT {
            pub freq_kHz: u32,
            pub voltage_uV: u32,
        }
    }

    // ------------------------------------------------------------------
    // NvAPI_SYS_ClientJpacSetControl2 (NDA, ID 0xD27D0629) — GPUMonCmd's
    // multi-feature BB2/WM2 control. RE'd from GPUMonCmd.exe
    // (reverse/GPUMon/GPUMonCmd.exe, handler sub_140017C00 cmdBb2Active /
    // sub_140024A90 setWm2Active / sub_140017720 cmdWm2Mode, all route
    // through sub_140005EC0 which QueryInterface's 0xD27D0629).
    //
    // 1224-byte buffer (0x4C8), version magic 0x104C8 (v1 | size).
    // Single-parameter call: NvAPI(handle_inside_struct). Layout:
    //   dword[0]  (off 0x00) = version magic 0x104C8
    //   dword[1]  (off 0x04) = operation: 1 = active on/off, 2 = SL mode
    //   dword[18] (off 0x48) = feature: 0 = WM2-active, 1 = WM2-mode, 3 = BB2-active
    //   dword[19] (off 0x4C) = enable flag (op 1) or mode enum (op 2)
    //                          WM2 modes: 0=Quieter, 1=Quiet, 2=Balanced
    //   dword[27] (off 0x6C) = constant 2 (WM2-mode only)
    //   dword[28] (off 0x70) = SL sound-level value: Quieter=30, Quiet=40, Balanced=60
    // ------------------------------------------------------------------

    /// Feature selector for the Jpac multi-feature control (dword[18]).
    #[repr(u32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum JpacFeature {
        /// Whisper Mode 2.0 active on/off.
        Wm2Active = 0,
        /// Whisper Mode 2.0 SL (sound-level) mode.
        Wm2Mode = 1,
        /// Battery Boost 2.0 active on/off.
        Bb2Active = 3,
    }

    /// Whisper Mode 2.0 acoustic mode (dword[19] when op=2, feature=Wm2Mode).
    #[repr(u32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Wm2AcousticMode {
        /// Quieter — SL value 30.
        Quieter = 0,
        /// Quiet — SL value 40.
        Quiet = 1,
        /// Balanced — SL value 60.
        Balanced = 2,
    }

    impl Wm2AcousticMode {
        /// The SL sound-level value the driver writes for this mode.
        pub const fn sl_value(self) -> u32 {
            match self {
                Wm2AcousticMode::Quieter => 30,
                Wm2AcousticMode::Quiet => 40,
                Wm2AcousticMode::Balanced => 60,
            }
        }
    }

    nvstruct! {
        /// BB2/WM2 multi-feature control (RE'd from GPUMonCmd; NDA).
        /// 1224 bytes, version magic 0x104C8. Use the builders below — the
        /// raw layout is op/feature multiplexed and most dwords must stay 0.
        pub struct NV_SYS_CLIENT_JPAC_CONTROL_V1 {
            pub version: NvVersion,
            /// Operation: 1 = active on/off, 2 = SL mode (WM2 only).
            pub op: u32,
            pub pad0: Padding<[u32; 16]>,
            /// Feature selector (dword[18], offset 0x48).
            pub feature: JpacFeature,
            /// Enable flag (op=1) or WM2 acoustic mode (op=2, feature=Wm2Mode).
            pub value: u32,
            pub pad1: Padding<[u32; 7]>,
            /// Constant 2 for WM2-mode (dword[27], offset 0x6C); 0 otherwise.
            pub wm2_mode_marker: u32,
            /// SL sound-level value (dword[28], offset 0x70); only for WM2-mode.
            pub sl_value: u32,
            pub pad2: Padding<[u32; 277]>,
        }
    }

    nvversion! { @=NV_SYS_CLIENT_JPAC_CONTROL NV_SYS_CLIENT_JPAC_CONTROL_V1(1) = 0x4C8 }

    impl NV_SYS_CLIENT_JPAC_CONTROL_V1 {
        /// Build a BB2 active on/off control (enable=true → on).
        pub fn bb2_active(enable: bool) -> Self {
            let mut s: Self = unsafe { std::mem::zeroed() };
            s.version = NvVersion::with_version(0x104C8);
            s.op = 1;
            s.feature = JpacFeature::Bb2Active;
            s.value = enable as u32;
            s
        }

        /// Build a WM2 active on/off control (enable=true → on).
        pub fn wm2_active(enable: bool) -> Self {
            let mut s: Self = unsafe { std::mem::zeroed() };
            s.version = NvVersion::with_version(0x104C8);
            s.op = 1;
            s.feature = JpacFeature::Wm2Active;
            s.value = enable as u32;
            s
        }

        /// Build a WM2 SL acoustic-mode control.
        pub fn wm2_mode(mode: Wm2AcousticMode) -> Self {
            let mut s: Self = unsafe { std::mem::zeroed() };
            s.version = NvVersion::with_version(0x104C8);
            s.op = 2;
            s.feature = JpacFeature::Wm2Mode;
            s.value = mode as u32;
            s.wm2_mode_marker = 2;
            s.sl_value = mode.sl_value();
            s
        }
    }

    nvapi! {
        /// Undocumented (NDA, ID 0xD27D0629). BB2/WM2 multi-feature control.
        /// Single-parameter: the 1224-byte control struct (handle inside).
        /// GPUMonCmd uses this for `-bb` (Battery Boost 2.0 on/off) and
        /// `-wm`/`-wmMode` (Whisper Mode 2.0 on/off + acoustic mode).
        pub unsafe fn NvAPI_SYS_ClientJpacSetControl2(pControl: *mut NV_SYS_CLIENT_JPAC_CONTROL) -> NvAPI_Status;
    }

    nvstruct! {
        /// V1 GetStatus entry (28-byte stride). IDA-verified against the
        /// R610.74 impl converter (sub_180200190, V3-internal → V1/V2-user
        /// copy-back): `lea rdx,[user+0x48]; mov [rdx-4],clock_type;
        /// mov [rdx],freq_kHz; mov [rdx+4],voltage_uV` — i.e. entries sit at
        /// +0x44 (68) with 28-byte stride and field map
        /// `{clock_type@+0, freq_kHz@+4, voltage_uV@+8, padding[16]}`.
        /// 68 + 28*255 = 7208 = 0x1C28 exactly.
        ///
        /// The earlier live A/B note that placed a `region` dword at +4 with
        /// freq@+8/volt@+12 was anchored 4 bytes early (raw dump started at
        /// +64, not +68): its dword[1] "region 0/1" pattern was actually
        /// `clock_type` (0 = core V/F curve, 1 = memory — same semantics as
        /// the V3 `clock_type`), dword[2] was freq, dword[3] was voltage.
        /// The converter sources freq/volt from the V3 entry's current pair
        /// (the +156/+160 slot, which the driver fills with a copy of the
        /// current freq/volt) — V1 is current-only, no default/overclocked
        /// pair. Only entries with clock_type < 2 convert; any masked entry
        /// with type >= 2 makes the whole V1/V2 call return -9.
        pub struct NV_GPU_CLOCK_CLIENT_CLK_VF_POINT_STATUS_V1 {
            /// 0 = core V/F curve, 1 = memory (mirrors V3 clock_type).
            pub clock_type: u32,
            pub freq_kHz: u32,
            pub voltage_uV: u32,
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

    // IDA R610.74 (both System32 nvamsi and the impl SKU): the GetStatus
    // handler accepts EXACTLY {0x11C28, 0x21C28, 0x35B0C} and the
    // GetControl/SetControl handlers accept {0x12420, 0x12421, 0x22420,
    // 0x22421} — the legacy 0x10434/1076B magics that third-party tools
    // (aiup/LACT, pre-R610 drivers) use are REJECTED with -9 here, and
    // there is NO GPU-arch dispatch inside these handlers: the 0x1C28
    // status and 0x2420 control layouts are driver-version-fixed and
    // marshaled verbatim to RM (escape 0x07000049, cmds 0x2080902A/C/D).
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

    // ClientPowerModes — NVIDIA App's power-MODE switcher (the UI's
    // Balanced/Max toggle), parallel to the PowerPolicies family above.
    // RE'd from NVIDIA App nvxdapix.dll; all three live-RESOLVED on
    // Windows R610.74 via the standard nvapi64 → nvapi64_impl chain.

    nvenum! {
        pub enum NV_GPU_CLIENT_POWER_MODE_ID / ClientPowerMode {
            NV_GPU_CLIENT_POWER_MODE_ID_BALANCED / Balanced = 0,
            NV_GPU_CLIENT_POWER_MODE_ID_MAX / Max = 1,
        }
    }

    nvenum_display! {
        ClientPowerMode => _
    }

    nvstruct! {
        /// ClientPowerModes GetInfo (magic 0x1150C = v1 | 5388B).
        /// Decoded from NVIDIA App UXDriver PhysicalStructure.cpp consumers
        /// (nvxdapix RE, instruction-verified):
        /// - +0x04 dword: seed value copied into CONTROL+0x04 before
        ///   GetControl in BOTH read and write paths (purpose opaque —
        ///   possibly a session/feature key the driver echoes).
        /// - +0x08 lo-u16 `mode_mask`: bitmask of supported modes
        ///   (0xFFFF observed = all bits set).
        /// - +0x0A hi-u16 `max_mode_idx`: feature-support gate — the App
        ///   exposes the Balanced/Max toggle ONLY when == 1 (0xFFFF on
        ///   4060L → unsupported, no toggle in the UI).
        /// Rest of the 5376-byte payload is never read by the App.
        pub struct NV_GPU_CLIENT_POWER_MODES_INFO_V1 {
            pub version: NvVersion,
            pub seed: u32,
            pub mode_mask: u16,
            pub max_mode_idx: u16,
            pub rest: Padding<[u32; 1344]>,
        }
    }

    nvversion! { @=NV_GPU_CLIENT_POWER_MODES_INFO NV_GPU_CLIENT_POWER_MODES_INFO_V1(1) = 5388 }

    nvstruct! {
        /// ClientPowerModes Get/SetControl (magic 0x1100C = v1 | 4108B):
        /// the active power-mode selector.
        /// SET protocol (App's SetIsGPUPowerMode, instruction-verified):
        /// GET-prime RMW — GetInfo → copy INFO+0x04 into CONTROL+0x04 →
        /// GetControl → write ONLY the u16 `active_mode_idx` at +0x08 →
        /// SetControl (every other byte passes through untouched).
        pub struct NV_GPU_CLIENT_POWER_MODES_CONTROL_V1 {
            pub version: NvVersion,
            pub seed: u32,
            pub active_mode_idx: u16,
            pub padding: Padding<[u8; 2]>,
            pub rest: Padding<[u32; 1024]>,
        }
    }

    nvversion! { @=NV_GPU_CLIENT_POWER_MODES_CONTROL NV_GPU_CLIENT_POWER_MODES_CONTROL_V1(1) = 4108 }

    nvapi! {
        pub unsafe fn NvAPI_GPU_ClientPowerModesGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_CLIENT_POWER_MODES_INFO) -> NvAPI_Status;
    }

    nvapi! {
        pub unsafe fn NvAPI_GPU_ClientPowerModesGetControl(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *mut NV_GPU_CLIENT_POWER_MODES_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        pub unsafe fn NvAPI_GPU_ClientPowerModesSetControl(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *const NV_GPU_CLIENT_POWER_MODES_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Undocumented (NDA-private, ID 0x1504FC3D). PPAB / Dynamic-Boost
        /// controller enable. `active` = 0 disables, non-zero enables. This is a
        /// GLOBAL single-argument by-value setter (NOT a per-GPU `*const` struct
        /// setStatus): the ref tool's thunk calls the resolved fn as `fn(active)` with
        /// NO hPhysicalGPU arg (targets the implicitly-selected GPU). Reversed
        /// from the ref-tool GUI/the ref-tool CLI (`[GPUHandle::setDynamicBoost] active:
        /// %d`, CLI `-db`). Matches the "PPAB Enable" checkbox on the
        /// Dynamic-Boost tab of OEM partner tools.
        pub unsafe fn NvAPI_GPU_ClientDynamicBoostSetStatus(active: BoolU32) -> NvAPI_Status;
    }

    nvapi! {
        /// Undocumented (NDA-private, ID 0xAD298D3F). Private lifecycle/controller
        /// init. the ref tool's init stub calls `fn(arg)` with arg=1 BEFORE any
        /// Dynamic-Boost / QBoost power setter; without it those setters return
        /// NVAPI_API_NOT_INITIALIZED. GLOBAL single u32 by-value arg.
        pub unsafe fn NvAPI_GPU_PrivateLifecycleInit(arg: BoolU32) -> NvAPI_Status;
    }

    // ------------------------------------------------------------------
    // TGP-watts power control (NDA-private triplet, the ref tool `setTgpWatt`).
    //
    // RE'd from the ref-tool GUI sub_1400324A0 ([GPUHandle::setTgpWatt]):
    //   GET  0x8B3E7343 (NvAPI_GPU_ClientTgpWattGetStatus)
    //   SET  0xBFF09E59 (NvAPI_GPU_ClientTgpWattSetStatus)
    // both take a 10016-byte read-modify-write buffer (version magic 0x12720 =
    // v1|10016). dword0 = version, dword1 = mask = (1 << policy_index). The
    // target power in MILLIWATTS is written at dword (553 + 10*policy_index)
    // = byte 0x8A4 + 40*index (the first dword of each 40-byte entry).
    // Caller passes watts; ×1000 → mW; 0xFFFFFFFF = reset to rated/default.
    //
    // The min/default/max mW range + active policy index come from a SEPARATE
    // private GetInfo: NvAPI_GPU_ClientPowerPoliciesGetInfoPrivate (0x67F31384,
    // NOT the public 0x34206D86). It returns a 347136-byte struct; see below.
    // ------------------------------------------------------------------

    /// Number of TGP-watts power-policy entries the params struct reserves.
    pub const NV_GPU_CLIENT_TGP_WATT_ENTRIES_MAX: usize = 32;

    nvstruct! {
        /// TGP-watts control read-modify-write buffer (RE'd from the ref tool; NDA).
        /// dword0 = version (0x12720), dword1 = mask = (1 << policy_index);
        /// per-entry power-mW at dword (553 + 10*index). The bulk of the buffer
        /// is opaque — GET fills it, the caller patches one entry, SET applies.
        pub struct NV_GPU_CLIENT_TGP_WATT_STATUS_V1 {
            pub version: NvVersion,
            pub mask: u32,
            /// Opaque header/descriptor + entry table (raw; GET-filled).
            pub payload: Padding<[u8; 10016 - 8]>,
        }
    }

    impl NV_GPU_CLIENT_TGP_WATT_STATUS_V1 {
        /// Per-entry stride in bytes (the ref-tool CLI writes v14[553 + 10*idx], i.e.
        /// 10 dwords = 40 bytes per entry).
        const POWER_STRIDE_BYTES: usize = 40;
        /// Byte offset WITHIN `payload` of entry 0's power-mW field. the ref-tool CLI's
        /// setTgpWatt writes v14[553 + 10*idx] = buffer byte (553+10*idx)*4.
        /// payload starts at buffer byte 8, so idx-0 base = 553*4 - 8 = 0x89C.
        const POWER_BASE_PAYLOAD_OFF: usize = 0x89C;

        fn power_off(&self, index: usize) -> Option<usize> {
            Self::POWER_BASE_PAYLOAD_OFF.checked_add(Self::POWER_STRIDE_BYTES.checked_mul(index)?)
        }

        /// Read the power-mW field for the given policy entry index.
        pub fn power_mw(&self, index: usize) -> Option<u32> {
            let off = self.power_off(index)?;
            self.payload
                .get(off..off + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        }

        /// Write the power-mW field for the given policy entry index (sets the
        /// mask bit for that entry as well).
        pub fn set_power_mw(&mut self, index: usize, milliwatts: u32) {
            if let Some(off) = self.power_off(index) {
                if let Some(slot) = self.payload.get_mut(off..off + 4) {
                    slot.copy_from_slice(&milliwatts.to_le_bytes());
                    self.mask |= 1u32 << index;
                }
            }
        }
    }

    nvversion! { @=NV_GPU_CLIENT_TGP_WATT_STATUS NV_GPU_CLIENT_TGP_WATT_STATUS_V1(1) = 10016 }

    nvapi! {
        /// Undocumented (NDA, ID 0x8B3E7343). Fills the TGP-watts control buffer
        /// (the GET half of setTgpWatt). Pair with SetStatus.
        pub unsafe fn NvAPI_GPU_ClientTgpWattGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pStatus: *mut NV_GPU_CLIENT_TGP_WATT_STATUS) -> NvAPI_Status;
    }

    nvapi! {
        /// Undocumented (NDA, ID 0xBFF09E59). Applies the TGP-watts control
        /// buffer (the SET half of setTgpWatt). Caller writes target mW into the
        /// active policy entry first.
        pub unsafe fn NvAPI_GPU_ClientTgpWattSetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pStatus: *const NV_GPU_CLIENT_TGP_WATT_STATUS) -> NvAPI_Status;
    }

    // ------------------------------------------------------------------
    // ClientPowerPoliciesGetInfoPrivate (NDA, ID 0x67F31384) — the TGP-watts
    // RANGE source. NOT the public 0x34206D86. Returns a 347136-byte struct
    // (86784 dwords), version magic 0x0F4BF4, per-policy entry stride 10604 B
    // (2651 dwords). Only the fields the ref tool reads are decoded here:
    //   - policy-table selector index: byte offset 0x14 (dword5 low byte; 0xFF
    //     ⇒ default to index 2).
    //   - per-entry min/default/max mW: entry dword +275 / +276 / +277.
    // The rest is opaque research layout (mirrors the PowerMonitor-V4 approach).
    // ------------------------------------------------------------------

    nvstruct! {
        /// TGP-watts policy/range descriptor (RE'd from the ref tool; NDA). Opaque
        /// except for the decoded accessors below.
        pub struct NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE_V1 {
            pub version: NvVersion,
            pub count_or_flags: u32,
            /// dword 2..4 (opaque).
            pub hdr0: u32,
            pub hdr1: u32,
            pub hdr2: u32,
            /// Byte 0x14 (dword5 low byte) = active policy table index; 0xFF ⇒
            /// caller should default to index 2. Pad to a dword boundary.
            pub policy_index_byte: u8,
            pub rsvd0: Padding<[u8; 3]>,
            /// dword 6..11 (opaque).
            pub hdr3: Padding<[u32; 6]>,
            /// dword 12 (the ref tool reads it into a "hide TGP" sibling field).
            pub hide_tgp_flag_dword: u32,
            /// Per-policy entry table (10604 B each); raw, parsed by accessors.
            /// Header before this = 52 bytes (dwords 0..12 + the index byte).
            /// Total struct = 347124 B (matches the ref tool's v7[86784] + memset).
            pub entries: Padding<[u8; 347124 - 52]>,
        }
    }

    impl NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE_V1 {
        /// Per-policy-entry stride in dwords (10604 bytes).
        const ENTRY_STRIDE_DWORDS: usize = 2651;
        const MIN_DWORD: usize = 275;
        const DEFAULT_DWORD: usize = 276;
        const MAX_DWORD: usize = 277;

        /// Active policy-table index; None ⇒ 0xFF (caller should default to 2).
        pub fn policy_index(&self) -> Option<u8> {
            (self.policy_index_byte != 0xFF).then_some(self.policy_index_byte)
        }

        /// Read dword `field` of policy entry `index`. the ref tool indexes these
        /// relative to the START of the whole struct (v7[N]), so the byte offset
        /// is (stride*index + field)*4 from byte 0 — but our typed header is the
        /// first 52 bytes, so subtract 52 to index into the `entries` payload.
        fn entry_dword(&self, index: usize, field: usize) -> Option<u32> {
            let off_struct = (Self::ENTRY_STRIDE_DWORDS * index + field) * 4;
            let off = off_struct.checked_sub(52)?;
            self.entries
                .get(off..off + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        }

        /// Minimum TGP in mW for the given policy entry.
        pub fn min_mw(&self, index: usize) -> Option<u32> {
            self.entry_dword(index, Self::MIN_DWORD)
        }
        /// Rated/default TGP in mW for the given policy entry.
        pub fn default_mw(&self, index: usize) -> Option<u32> {
            self.entry_dword(index, Self::DEFAULT_DWORD)
        }
        /// Maximum TGP in mW for the given policy entry.
        pub fn max_mw(&self, index: usize) -> Option<u32> {
            self.entry_dword(index, Self::MAX_DWORD)
        }

        // ------------------------------------------------------------------
        // D-Notifier (D0-notify / "extern power state") fields.
        //
        // RE'd from the ref-tool GUI / the ref-tool CLI `[GPUHandle::pollDNotifyLimit]`
        // (the ref-tool GUI sub_140028300, the ref-tool CLI sub_140025750) — the GUI build
        // reveals the semantics the CLI build hides: it builds the string
        // "D{n}({power}mW)" (e.g. "D3(45000mW)"), so the dword at
        // `3*Didx + 85682` is the **power limit in mW** for that D level, NOT a
        // display label. Cross-checked against live RTX 4060 Laptop readings:
        //   D1 (-1) = Unlimited   D2 (0) = 55000   D3 (1) = 45000
        //   D4 (2) = 33000         D5 (3) = 10000  (all mW).
        //
        // These fields live in the TAIL of the same 347124-byte struct, AFTER
        // the 32-entry TGP policy table (entry stride 2651 dwords ⇒ entry 32
        // starts at dword 32*2651 = 84832, well before 85679). They are NOT part
        // of any per-policy entry, so they are read by ABSOLUTE dword offset,
        // not via `entry_dword()`.
        //
        // Absolute offsets (struct dword 0 = version):
        //   active D-index ........ dword 85692 (byte 0x53AF0); -1 = Unlimited
        //   per-D power table ..... dword (85682 + 3*Didx), stride 3; the first
        //                          dword of each triple is the mW limit for
        //                          Didx 0..3 (D2..D5). The other two dwords are
        //                          opaque (the ref tool never reads them). D1 (Didx -1)
        //                          is "Unlimited" — the ref tool does NOT consult the
        //                          table for it, so the base is 85682, NOT 85679.
        //                          (An earlier pass reserved a D1 slot at 85679
        //                          and read every value one level too low.)
        // ------------------------------------------------------------------
        const DNOTIFY_ACTIVE_INDEX_DWORD: usize = 85692;
        const DNOTIFY_POWER_TABLE_BASE_DWORD: usize = 85682;
        const DNOTIFY_POWER_TABLE_STRIDE: usize = 3;

        /// Read an arbitrary dword at an ABSOLUTE offset (dword index from the
        /// start of the whole struct, header included). Used for the D-Notifier
        /// tail fields that are not part of any TGP policy entry. Bounds-checked
        /// against the full struct size.
        fn absolute_dword(&self, dword_index: usize) -> Option<u32> {
            // The typed header occupies the first 52 bytes (13 dwords); the rest
            // lives in the `entries` payload. Map an absolute dword into the
            // payload and read it.
            let byte_off = dword_index.checked_mul(4)?;
            let payload_off = byte_off.checked_sub(52)?;
            self.entries
                .get(payload_off..payload_off.checked_add(4)?)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        }

        /// The currently-active D-Notifier (D0-notify) level index as a signed
        /// code: `-1` = D1 / Unlimited, `0..=3` = D2..D5. Returns `None` if the
        /// driver reported the sentinel `4` (invalid / N/A) or the field was out
        /// of bounds. See `DNotifierLevel::from_index` to map this to a level.
        pub fn dnotify_active_index(&self) -> Option<i32> {
            let raw = self.absolute_dword(Self::DNOTIFY_ACTIVE_INDEX_DWORD)? as i32;
            // 4 is the ref tool's "N/A" sentinel (it prints "N/A" and stores -1); -1
            // is the legitimate "D1 - Unlimited" code. Anything else in 0..=3 is
            // a real D2..D5 level.
            if raw == 4 { None } else { Some(raw) }
        }

        /// The power limit in mW for the given D-Notifier level index, as read
        /// from the per-D power table. `didx` follows the same signed code as
        /// [`dnotify_active_index`] (`-1`=D1, `0..=3`=D2..D5). D1 (-1) is
        /// "Unlimited" — its table slot is read but the value is conventionally
        /// unused; callers should treat D1 as unbounded regardless. Returns
        /// `None` if the offset is out of bounds.
        pub fn dnotify_power_mw(&self, didx: i32) -> Option<u32> {
            // D1 (didx -1) is Unlimited — no table entry. the ref tool never reads the
            // table for it; returning None here keeps callers from touching the
            // pre-base dword 85679.
            if didx < 0 {
                return None;
            }
            let dword = Self::DNOTIFY_POWER_TABLE_BASE_DWORD
                .checked_add((didx as usize).checked_mul(Self::DNOTIFY_POWER_TABLE_STRIDE)?)?;
            self.absolute_dword(dword)
        }
    }

    nvversion! { @=NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE_V1(15) = 347124 }

    nvapi! {
        /// Undocumented (NDA, ID 0x67F31384). Private ClientPowerPoliciesGetInfo
        /// variant — the TGP-watts min/default/max range + active policy index.
        /// NOT the public 0x34206D86. Returns a 347124-byte struct with version
        /// magic 0x0F4BF4 (version 15) — the ref tool's queryPowerPolicy uses exactly
        /// this; the version-1 magic I first tried is rejected by the driver.
        pub unsafe fn NvAPI_GPU_ClientPowerPoliciesGetInfoPrivate(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE) -> NvAPI_Status;
    }

    nvapi! {
        /// Undocumented (NDA-private, ID 0x48E0847D). D-Notifier (D0-notify)
        /// "extern power state" SETTER — the write half of the ref tool's
        /// `[GPUHandle::setDNotifyLimit]` (thunk sub_140001780 in the ref-tool CLI).
        /// Raw two-arg call: `(hPhysicalGPU, level: u32)` — NO struct buffer,
        /// unlike the TGP-watts SetStatus path. `level` is the signed D-level
        /// code (0xFFFFFFFF = D1/Unlimited, 0..3 = D2..D5), passed as a raw u32.
        /// The matching GET is `NvAPI_GPU_ClientPowerPoliciesGetInfoPrivate`
        /// (0x67F31384) above, which exposes both the active D level and the
        /// per-D power-cap table.
        pub unsafe fn NvAPI_GPU_ClientExternPowerStateSet(hPhysicalGPU: NvPhysicalGpuHandle, level: u32) -> NvAPI_Status;
    }

    // ------------------------------------------------------------------
    // GC6 / RTD3 force-wake control (NDA). On 610-series mobile drivers the
    // dGPU enters GC6 (link-off) / GCOFF aggressively when idle, which makes
    // overclock operations fail with NVAPI_GPU_NOT_POWERED (-220) or makes
    // NvAPI_Initialize itself return NvidiaDeviceNotFound. These two IDs are
    // the RM-level force-wake path the kernel driver honors — confirmed live
    // (both resolve non-NULL via QueryInterface on the 610 driver) and RE'd
    // from nvapi64_impl.dll. Neither has a per-call GCOFF guard; the only gate
    // is the one-shot nvapi-init flag, so they CAN wake a powered-down dGPU.
    //
    // Sources / addresses (nvapi64_impl.dll):
    //   ForceGC6Exit: handler sub_180187930, RM escape 0x10000FC
    //   GC6Control:   handler sub_180187CA0, RM escape 0x70000ED
    // ------------------------------------------------------------------

    /// `cmd` enum for [`NV_GPU_GC6_CONTROL_V1`] — the action the GC6Control
    /// escape commands the RM driver to take.
    pub const NV_GPU_GC6_CONTROL_CMD_QUERY: u32 = 0; // read current state into `result`
    pub const NV_GPU_GC6_CONTROL_CMD_SLEEP: u32 = 1; // force GC6 entry (idle the dGPU)
    pub const NV_GPU_GC6_CONTROL_CMD_WAKE: u32 = 2; // force GC6 exit (wake the dGPU)

    /// `result` enum for [`NV_GPU_GC6_CONTROL_V1`] — decoded GC6 power state
    /// (populated when `cmd == NV_GPU_GC6_CONTROL_CMD_QUERY`).
    pub const NV_GPU_GC6_STATE_OK: u32 = 0; // command succeeded / no state to report
    pub const NV_GPU_GC6_STATE_GC6_IDLE: u32 = 2; // dGPU is in GC6 (link-off / idle)
    pub const NV_GPU_GC6_STATE_D0_ACTIVE: u32 = 3; // dGPU is in D0 (active / powered on)
    pub const NV_GPU_GC6_STATE_UNKNOWN: u32 = 4;

    nvstruct! {
        /// 12-byte GC6 control struct (version magic `0x1000C`, same v1/12-byte
        /// family as `NV_GPU_RATED_TDP_CONTROL`). Layout: `[0..3]=version`,
        /// `[4]=cmd` (one of `NV_GPU_GC6_CONTROL_CMD_*`), `[8]=result`
        /// (one of `NV_GPU_GC6_STATE_*`, filled by the driver).
        pub struct NV_GPU_GC6_CONTROL_V1 {
            pub version: NvVersion,
            /// Action: QUERY(0) / SLEEP(1) / WAKE(2). Anything else → -5 InvalidArgument.
            pub cmd: u32,
            /// Result-out: driver writes the GC6 state here (OK / GC6_IDLE / D0_ACTIVE).
            pub result: u32,
        }
    }

    nvversion! { @=NV_GPU_GC6_CONTROL NV_GPU_GC6_CONTROL_V1(1) = 12 }

    nvapi! {
        /// Undocumented (NDA, ID 0xD387D414). GC6 control — query current GC6
        /// power state (cmd=0), force GC6 entry/sleep (cmd=1), or force GC6
        /// exit/wake (cmd=2). 12-byte struct, version magic 0x1000C. RM escape
        /// 0x70000ED. The wake path (cmd=2) is one of the two force-wake routes
        /// that reach the kernel driver without a per-call GCOFF guard.
        pub unsafe fn NvAPI_GPU_GC6Control(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *mut NV_GPU_GC6_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Undocumented (NDA, ID 0x55590CB2). Force GC6 exit — a single-purpose
        /// "wake the dGPU now" escape. Takes ONLY the GPU handle (no struct, no
        /// version magic); the escape ID itself (0x10000FC) is the command.
        /// Purpose-built counterpart to the GC6Control cmd=2 path but simpler.
        /// Returns -104 (NoImplementation) on SKUs without GC6 support.
        pub unsafe fn NvAPI_GPU_ForceGC6Exit(hPhysicalGPU: NvPhysicalGpuHandle) -> NvAPI_Status;
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
            /// (ccminer cross-ref: seen 1/4/5 while mining, 16 idle —
            /// bitmask of active policies)
            pub unknown: u32,
            pub zero1: u32,
            /// nanoseconds
            /// (ccminer cross-ref: companion flag field seen 7 and 3)
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
    // (RivaTuner) source `NVAPIInterface.h` + nvapi64_impl.dll handlers
    // (GetInfo @0x180257660, GetStatus @0x180258170; both funnel into the same
    // RM escape 0x06FF0016).
    //
    // WRAPPED & LIVE (validated on RTX 4060 Laptop, units confirmed by exact
    // GPU-Z match: raw mW ÷ 1000 = W). GetInfo returns a capability/topology
    // descriptor (which of up to 32 channels exist, each channel's type/rail/
    // scaling); GetStatus returns the live per-rail wattage. The V2 structs
    // below are the RTSS-derived research layout — they do NOT match the
    // deployed driver's accepted struct sizes (see the V1_2728/V3_3240/V4
    // structs and `nvapi_rs::power` for the live path). Kept as a research
    // record of the RTSS field semantics (channel_type / PowerRail enums etc).
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
        pub fn channels(
            &self,
        ) -> impl Iterator<Item = (usize, &NV_GPU_POWER_MONITOR_POWER_CHANNEL_INFO_V2)> {
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
        /// `total_gpu_power_mw` + per-channel `channels[i]`. LIVE on validated
        /// hardware (units confirmed: mW ÷ 1000 = W). This is the RTSS-derived
        /// V2 layout for research; the deployed driver's live path uses the
        /// v1|392 status buffer — see `nvapi_rs::power::PowerRails` / the
        /// `powermonitor-v4-prewrap` work for the production read path.
        pub unsafe fn NvAPI_GPU_PowerMonitorGetStatus(hPhysicalGPU: NvPhysicalGpuHandle, pStatus: *mut NV_GPU_POWER_MONITOR_GET_STATUS) -> NvAPI_Status;
    }

    // ------------------------------------------------------------------
    // PowerMonitor V4 — the deployed driver's richest GetInfo layout.
    //
    // RE'd 2026-07-27 from the live driver on RTX 4060 Laptop: GetInfo
    // (0xC12EB19E) accepts magic (4<<16)|6312 = 268456, returning a 6312-byte
    // buffer whose first 0x34 bytes are the header below and whose remaining
    // 6260 bytes hold a VARIABLE-LENGTH, SPARSELY-PACKED per-channel
    // descriptor table. Each descriptor's length depends on its channel_type
    // (type 5/7 carry VF-estimation LUT tables; type 1/8 are small), so the
    // records are NOT a fixed-stride array — observed descriptor offsets were
    // 0x34, 0x74, 0xE8, 0x160, 0x28C, ... (irregular strides).
    //
    // Because of that, this struct exposes the descriptor region as a raw
    // byte buffer (`descriptors`); the hi layer parses it by signature scan
    // (channel_type in 1..=8 + a plausible PowerRail), reusing the exact logic
    // proven in core/tests/gpu_readonly.rs::nvapi_power_monitor_raw. Each
    // descriptor's decoded header is: [pwr_device_mask, channel_type,
    // pwr_rail, volt_fixed_uv, pwr_corr_slope(4096=Q12), curr_corr_slope, ...].
    //
    // VERSION ALIGNMENT: v1|2728 (magic 68264), v3|3240 (199848), and v4|6312
    // (268456) share an IDENTICAL header + descriptor-offset layout — they
    // differ ONLY in where the type=5 VF-LUT records truncate (smaller magic
    // = less VF-curve detail, but the same channel identity). v1|404 (65940)
    // is header-only (just channel_mask, no descriptors). The hi-layer reader
    // tries v4 -> v3 -> v1|2728 in order so older drivers that reject v4
    // still get descriptors. See `nvapi_rs::power` for the fallback chain.
    // ------------------------------------------------------------------

    /// Shared header fields for the v1|2728 / v3|3240 / v4|6312 GetInfo
    /// layouts (identical across all three). The descriptor region that
    /// follows is version-sized, so each version struct embeds this header
    /// then a differently-sized raw descriptor buffer.
    macro_rules! powermonitor_getinfo_versioned {
        ($name:ident, $magic_size:expr) => {
            nvstruct! {
                pub struct $name {
                    pub version: NvVersion,
                    pub b_supported: BoolU32,
                    pub sampling_period_ms: u32,
                    pub sample_count: u32,
                    pub channel_mask: u32,
                    pub ch_rel_mask: u32,
                    pub total_gpu_power_channel_mask: u32,
                    pub total_gpu_channel_idx: u8,
                    /// Header padding to byte offset 0x34 (first descriptor).
                    pub header_rsvd: Padding<[u8; 0x34 - 0x1D]>,
                    /// Variable-length, sparsely-packed per-channel descriptors.
                    /// Parsed by signature scan (channel_type 1..=8 + plausible
                    /// PowerRail); record length varies with channel_type.
                    pub descriptors: Padding<[u8; $magic_size - 0x34]>,
                }
            }

            impl $name {
                /// The descriptor region as a raw byte slice (signature-scan
                /// parsed by the hi layer).
                pub fn descriptors_bytes(&self) -> &[u8] {
                    &self.descriptors[..]
                }
            }

            impl Default for $name {
                fn default() -> Self {
                    unsafe { std::mem::zeroed() }
                }
            }
        };
    }

    powermonitor_getinfo_versioned!(NV_GPU_POWER_MONITOR_GET_INFO_V1_2728, 2728);
    powermonitor_getinfo_versioned!(NV_GPU_POWER_MONITOR_GET_INFO_V3_3240, 3240);
    powermonitor_getinfo_versioned!(NV_GPU_POWER_MONITOR_GET_INFO_V4, 6312);

    nvversion! { NV_GPU_POWER_MONITOR_GET_INFO_V1_2728(1) = 2728 }
    nvversion! { NV_GPU_POWER_MONITOR_GET_INFO_V3_3240(3) = 3240 }
    nvversion! { NV_GPU_POWER_MONITOR_GET_INFO_V4(4) = 6312 }
}
