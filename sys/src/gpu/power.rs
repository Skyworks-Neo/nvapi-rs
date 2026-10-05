/// Undocumented API
pub mod undocumented {
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
    // RM layer (full marshal RE, docs/reverse-engineering/nvapi/
    // voltrails-family-full-layout-r610.md): escape 0x07000191 through
    // sub_180389320 with a 500,008-byte (0x7A118) request buffer
    // (buf[12]=gpu, buf[13]=ctrl cmd, buf[15]=rail-mask filter), one RM
    // ctrl cmd per op — GetInfo **0x2080B201** (76B records), GetStatus
    // **0x2080B202** (100B records), GetControl **0x2080B213** / SetControl
    // **0x2080F214** (32B records, type byte @+0, 6-dword payload @+4).
    // The cmds previously recorded here (0x2080A601/0x2080A613) belong to
    // the OTHER family — the percent-ClientVoltRails surface (handler
    // sub_180235F10, 104B struct, 16B entries) — not this one.
    //
    // GetInfo handler sub_1801D1420 sends mask filter 0 (all rails);
    // GetStatus/GetControl forward the caller's mask; GetStatus also
    // copies the head when mask==0 (v2 dst[2..7] ← rm[16..21]).
    //
    // VoltVoltRailsSetControl 0x87C55C8A (the µV-offset WRITE path melonVolt
    // drives) is wrapped in the medium layer with the full melonVolt protocol
    // (snapshot -> locate -> sanity -> write -> SET -> readback verify);
    // do NOT call the raw FFI directly.

    /// Byte offsets into the per-rail entries of
    /// [`NV_GPU_VOLT_RAILS_INFO`].
    ///
    /// Full marshal map (R610.74 `sub_1801D1420`, 76B RM records → 192B
    /// slots; live 4060L values in parens; semantics in brackets are
    /// structurally pinned but not A/B-confirmed):
    ///
    /// | off | width | src | live | meaning |
    /// |-----|-------|-----|------|---------|
    /// | +76 | u32 | byte 0x44 → type enc | 0 | format tag (see type enc) |
    /// | +80 | u32 | byte 0x77 → class enc | 1 | rail class 1..8 (also V1-status "type") |
    /// | +84 | u8 | 0x53 | 1 | |
    /// | +88 | u32 | 0x48 | 750000 | µV (0.75 V) [nominal/vfloor candidate] |
    /// | +92 | u16 | 0x4C | 2 | |
    /// | +94 | u16 | 0x4E | 0xFFFF | [invalid marker] |
    /// | +96 | u16 | 0x50 | 8 | |
    /// | +100 | u32 | 0x64 | 1 | |
    /// | +104 | u16 | 0x56 | 7 | |
    /// | +106 | u16 | 0x58 | 10 | |
    /// | +110 | u16 | 0x5E (byte) | 2 | |
    /// | +112 | u8 | 0x54 | 1 | |
    /// | +113 | u8 | const | 0xFF | always 0xFF |
    /// | +116 | u32 | 0x60 | 29 | |
    /// | +120 | u8 | 0x5A | 16 | |
    /// | +124 | u32 | 0x6C | 820000 | µV (0.82 V) [nominal/vfloor candidate] |
    /// | +128 | u32 | 0x68 | 959 | |
    /// | +132..+139 | per-type tail | 0x7A..0x84 | 2/17/16 | type-0: u8@132←0x7A, u16@134←0x7C, u16@136←0x7E; type-2: u32@132←0x80, u8@136←min(word 0x84,0xFF); type-3: u32@132←0x7C, u8@136←0x80 |
    ///
    /// Struct head outside the loop: `mask(+4) |= 1<<bit` (RM dword 0x3C);
    /// struct byte +8 ← RM byte +0x40.
    pub mod rail_entry {
        /// stride per rail BIT index
        pub const STRIDE: usize = 192;
        /// u32 type discriminator (copied into control/status entry seeds)
        pub const TYPE: usize = 76;
        /// u32 rail class 1..8 (`sub_18015B540`: RM byte 1..8 identity,
        /// else 0 + status -5); the field GetStatus V1 mirrors as "type"
        pub const CLASS: usize = 80;
        /// u32 µV reading (0.75 V on live 4060L rail 0)
        pub const UV_A: usize = 88;
        /// u32 µV reading (0.82 V on live 4060L rail 0)
        pub const UV_B: usize = 124;
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
    ///
    /// Type provenance (IDA `sub_18015B690`): the driver maps the RM raw
    /// byte to the exported dword — `2 → 0, 4 → 2, 5 → 3, 0xFF → -1,
    /// other → -2` (never errors). The exported 0/2/3 space is a per-rail
    /// format tag.
    ///
    /// **GetStatus V1 caveat** (handler `sub_1801D1CD0` → back-converter
    /// `sub_1801C83E0`): the V1 status entry's type@+72 is NOT this RM type
    /// tag — the V1 path internally re-runs GetInfo and copies the
    /// descriptor's rail CLASS (GetInfo +80, RM class byte 1..8 identity,
    /// else 0 + status -5) into +72. That is why the live 4060L status
    /// reports type=1 while the GetInfo descriptor reports type=0 on the
    /// same rail: different fields (class 1 vs RM type tag 0).
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
        /// V1 stamp ((1<<16)|0xACC, 2764B total): the variant Volta parts
        /// (V100/GV100, live 538.78, 2026-09-01 — probe
        /// tests/volta_voltrails_v1_layout.rs) and R391-era drivers accept
        /// where the V2 stamp ((2<<16)|6220) is rejected with
        /// IncompatibleStructVersion. SAME dense rail-entry layout as V2
        /// (192B/rail, type @+76, undecoded descriptor dwords) — only the
        /// header differs: V1 carries NO rail mask (single fixed dense
        /// rail; entry 0 live on V100 with type=1).
        pub const MAGIC_V1: u32 = 0x10ACC;

        /// Type discriminator of the rail entry for `bit` (u32 @+192*bit+76).
        pub fn rail_type(&self, bit: u32) -> Option<u32> {
            let base = rail_entry::STRIDE.checked_mul(bit as usize)?;
            let off = base + rail_entry::TYPE;
            let end = off + 4;
            let raw = self.rest.get(off - 8..end - 8)?;
            Some(u32::from_le_bytes(raw.try_into().ok()?))
        }

        /// Raw 192-byte rail descriptor for `bit` as 48 little-endian u32.
        /// Decoded dwords: 19 = type, 20 = class, 22/31 = the two µV
        /// readings (see the [`rail_entry`] map); the rest is marshaled
        /// but semantically unconfirmed driver data (observed non-zero on
        /// 4060 Laptop); dumped for cross-platform comparison.
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

    impl NV_GPU_VOLT_RAILS_CONTROL {
        /// V1 stamp (0x10AC8 — coincidentally the same value as the STATUS
        /// V1 stamp; both structs total 2760B): the GetControl/SetControl
        /// variant Volta/R391-era drivers accept (live V100 GetControl
        /// V1: status=0, entry type=1, stock offsets all zero).
        pub const MAGIC_V1: u32 = 0x10AC8;
    }

    nvstruct! {
        /// Live-voltage variant: identical layout, but the driver only accepts
        /// the V1 version stamp 0x10AC8 (68296) here.
        ///
        /// The handler (`sub_1801D1CD0`) ALSO accepts a V2 stamp **0x21620**
        /// ((2<<16)|0x1620, 5664 B): entries become 43-dword (172 B) slots
        /// indexed by RAIL BIT (not dense), entry base = struct +160 +
        /// 172·bit, each with 9 payload dwords (+4..+39 = values[0..8] —
        /// [0..5] same semantics as V1, [6..8] extra, live 4060L: 0/625000/0),
        /// an enum byte @+40 (RM byte +132; 0/1/2 valid, else -1 + status
        /// -5), and for type-0 entries a tail: dwords +48/+52/+56 ← RM
        /// dwords +168/+164/+172 (live 810000/820000/29) + byte +44 ← RM
        /// byte +176 (live 1). V1 is a lossy projection of that V2 form.
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

    // ------------------------------------------------------------------
    // VoltVoltDevicesGetInfo (0xA38ACF9D, R465 handler @0x180202600) — the
    // melonVolt VOLTAGE-DOMAIN ("device") enumerator, sibling of the rail
    // builder above but a different RM surface (escape id 117440913 =
    // 0x07000091 family, 2264B internal buffer). Stamp: 0x10F48 (v1|3912)
    // on ALL branches 391→610 (610 adds a v15|41240 stamp 0x7A118); plain
    // equality gate (IDA 538: `*a2 != 69448`).
    // Caller layout: version dword + present-mask dword (@+4) + 32 entries
    // of 30 dwords (120 B) + a 64-byte trailing block (unwritten by the
    // 538 handler; part of the stamped size) = 3912 B total. The driver
    // walks its internal 56 B/device records (14 dwords) against the mask
    // bits:
    //   entry dword[18] ← device TYPE byte (@internal+64): only type 1
    //     (voltage rail) and type 3 (monitor) are valid — anything else →
    //     -5 with entry[18] = -1;
    //   type 3 additionally fills dword[23]/[24]/[26] + bytes 100/108;
    //   type 1 fills three 12-byte (u8 triplets) groups @92..127 — the
    //     min/max/current voltage triplet lanes;
    //   every valid entry: dwords[20..22] ← internal[18..20], bytes
    //     76..78 ← internal[66..68]; present bit `1 << k` ORed into +4.
    // ------------------------------------------------------------------
    nvapi! {
        /// melonVolt voltage-domain enumerator (NDA, ID 0xA38ACF9D): fills
        /// the device present mask + per-device descriptor table (see the
        /// layout comment above). Stamp 0x10F48 universal 391→610.
        pub unsafe fn NvAPI_GPU_VoltVoltDevicesGetInfo(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_VOLT_DEVICES_INFO) -> NvAPI_Status;
    }

    nvstruct! {
        /// Caller buffer for [`NvAPI_GPU_VoltVoltDevicesGetInfo`]: version
        /// 0x10F48 (v1|3912) + present mask + 32 × 120 B device entries +
        /// 64 B trailer. Fields beyond the mask are opaque raw (see the
        /// handler comment for the per-entry dword map); decode lives in
        /// the consumer.
        pub struct NV_GPU_VOLT_DEVICES_INFO_V1 {
            pub version: NvVersion,
            /// present-device bitmask (driver ORs `1 << k` per valid entry)
            pub present_mask: u32,
            /// per-device descriptor table, 30 dwords (120 B) each
            pub devices: Array<[[u32; 30]; 32]>,
            /// stamped tail block (unwritten by the R538 handler)
            pub tail: Array<[u8; 64]>,
        }
    }

    nvversion! { @=NV_GPU_VOLT_DEVICES_INFO NV_GPU_VOLT_DEVICES_INFO_V1(1) = 3912 }

    nvapi! {
        /// Private VoltRails control-object GET (per-rail offset entries).
        /// The struct must be seeded from a prior GetInfo call.
        pub unsafe fn NvAPI_GPU_VoltVoltRailsGetControl(hPhysicalGPU: NvPhysicalGpuHandle, pControl: *mut NV_GPU_VOLT_RAILS_CONTROL) -> NvAPI_Status;
    }

    nvapi! {
        /// Private VoltRails live-status GET (per-rail voltages, µV).
        /// V1 stamp 0x10AC8 (dense 84B entries; the production path) or V2
        /// stamp 0x21620 (5664B, bit-indexed 172B entries — see
        /// [`NV_GPU_VOLT_RAILS_STATUS_V1`]); seeded like GetControl.
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
    // NvAPI_SYS_ClientJpacSetControl (NDA, ID 0xD2561B69) — ref tool 2's
    // multi-feature BB2/WM2 control. RE'd from ref tool
    // (handler sub_140017C00 cmdBb2Active /
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

    unsafe impl zerocopy::IntoBytes for JpacFeature {
        fn only_derive_is_allowed_to_implement_this_trait()
        where
            Self: Sized,
        {
        }
    }
    unsafe impl zerocopy::Immutable for JpacFeature {
        fn only_derive_is_allowed_to_implement_this_trait()
        where
            Self: Sized,
        {
        }
    }
    unsafe impl zerocopy::FromBytes for JpacFeature {
        fn only_derive_is_allowed_to_implement_this_trait()
        where
            Self: Sized,
        {
        }
    }
    unsafe impl zerocopy::TryFromBytes for JpacFeature {
        fn only_derive_is_allowed_to_implement_this_trait()
        where
            Self: Sized,
        {
        }
        fn is_bit_valid<A>(candidate: zerocopy::Maybe<'_, Self, A>) -> bool
        where
            A: zerocopy::invariant::Alignment,
        {
            // plain old data: every bit pattern is valid
            let _ = candidate;
            true
        }
    }
    unsafe impl zerocopy::FromZeros for JpacFeature {
        fn only_derive_is_allowed_to_implement_this_trait()
        where
            Self: Sized,
        {
        }
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

    unsafe impl zerocopy::IntoBytes for Wm2AcousticMode {
        fn only_derive_is_allowed_to_implement_this_trait()
        where
            Self: Sized,
        {
        }
    }
    unsafe impl zerocopy::Immutable for Wm2AcousticMode {
        fn only_derive_is_allowed_to_implement_this_trait()
        where
            Self: Sized,
        {
        }
    }
    unsafe impl zerocopy::FromBytes for Wm2AcousticMode {
        fn only_derive_is_allowed_to_implement_this_trait()
        where
            Self: Sized,
        {
        }
    }
    unsafe impl zerocopy::TryFromBytes for Wm2AcousticMode {
        fn only_derive_is_allowed_to_implement_this_trait()
        where
            Self: Sized,
        {
        }
        fn is_bit_valid<A>(candidate: zerocopy::Maybe<'_, Self, A>) -> bool
        where
            A: zerocopy::invariant::Alignment,
        {
            // plain old data: every bit pattern is valid
            let _ = candidate;
            true
        }
    }
    unsafe impl zerocopy::FromZeros for Wm2AcousticMode {
        fn only_derive_is_allowed_to_implement_this_trait()
        where
            Self: Sized,
        {
        }
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
        /// BB2/WM2 multi-feature control (RE'd from ref tool 2;).
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
        /// Undocumented (ID 0xD27D0629). BB2/WM2 multi-feature control.
        /// Single-parameter: the 1224-byte control struct (handle inside).
        /// ref tool 2 uses this for `-bb` (Battery Boost 2.0 on/off) and
        /// `-wm`/`-wmMode` (Whisper Mode 2.0 on/off + acoustic mode).
        pub unsafe fn NvAPI_SYS_ClientJpacSetControl(pControl: *mut NV_SYS_CLIENT_JPAC_CONTROL) -> NvAPI_Status;
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
            counted(&*self.entries, self.count as usize)
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
            pub rest: Array<[u32; 1344]>,
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
            pub rest: Array<[u32; 1024]>,
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

    // ------------------------------------------------------------------
    // PowerMizer (NVCP "电源模式" dropdown) is a SEPARATE family from
    // SetPerfLevel. On R610.74:
    //   GetPowerMizerInfo  0x76bfa16b — 4-arg GET, mode ∈ {6,7}, escape 0x700003A op=3
    //   SetPowerMizerInfo  0x50016c78 — 4-arg SET, escape 0x700003A op=2
    //   SetPerfLevel       0x75dd3e6a — NOT a power-mode dropdown (2026-08-26
    //     correction, user-measured + live-verified): it is an ADMIN-FREE
    //     pstate lock. `fn(hGpu, level)` where level INDEXES the GPU's real
    //     available P-States (4060 Laptop: 0=P8, 1=P5, 2=P4, 3=P3, 4=P0 —
    //     NOT a fixed enum; other GPUs expose different P-State sets).
    //     Impl @0x1802B2260 sends RM escape 0x7000040 (0x38B workbuf,
    //     hGpu@+0x30, level@+0x34). RM accepts ONLY valid indices — a full
    //     live sweep (-1, -16, -255, 5..255, 65536, 0x1000000, i32::MIN …)
    //     all return NVAPI_ERROR, so there is NO release argument (neither
    //     -1 nor the SetForcePstate sentinel 16).
    //     The lock survives reset-force-pstate, elevated reset-pstate-native
    //     (PerfClientLimits clear), EnableDynamicPstates(0) and
    //     SetPowerMizerInfo — a 4th independent lock store; only a driver
    //     reload/reboot clears it. Re-locking another index DOES re-target
    //     (last call wins). The nominal GET companion 0x77D8F573 (escape
    //     0x7000042) returns a constant (1,1,0) regardless of locked level
    //     and is NOT a level readback.
    // ------------------------------------------------------------------

    nvapi! {
        /// Admin-free pstate lock (escape 0x7000040). `level` is an INDEX
        /// into the GPU's actual available P-State list (see
        /// `PerfPstatesGetInfoPrivate` / get-pstate-native) — NOT a fixed
        /// enum. On the 4060 Laptop (P3/P4/P5/P8 + P0) the user-measured
        /// mapping is 0=P8, 1=P5, 2=P4, 3=P3, 4=P0, but other GPUs expose a
        /// different P-State set and the index means "the Nth of THIS GPU's
        /// P-States". RM accepts only valid indices — a full live sweep
        /// (-1, -16, 5..255, 65536, 0x1000000, i32::MIN …) all return
        /// NVAPI_ERROR, so there is NO release argument. The lock survives
        /// reset-force-pstate, elevated reset-pstate-native
        /// (PerfClientLimits clear), EnableDynamicPstates(0) and
        /// SetPowerMizerInfo — only a driver reload/reboot clears it.
        /// Re-locking another index DOES re-target (last call wins). The
        /// nominal GET companion 0x77D8F573 (escape 0x7000042) returns a
        /// constant (1,1,0) regardless of locked level and is NOT a readback.
        pub unsafe fn NvAPI_GPU_SetPerfLevel(hPhysicalGPU: NvPhysicalGpuHandle, level: u32) -> NvAPI_Status;
    }

    nvapi! {
        /// NVCP power-mode GET (RE'd R610.74 @0x1802392A0: 4-ARG, not 2).
        /// `fn(hGpu, powerSource∈{1,2}, queryType=3, *outMode)` — output mode
        /// ∈ {6,7} only (internal 0→6, 1→7). queryType must be 3; RM/DI escape
        /// 0x700003A with a 0x48-byte private struct, op=3.
        pub unsafe fn NvAPI_GPU_GetPowerMizerInfo(hPhysicalGPU: NvPhysicalGpuHandle, powerSource: u32, queryType: u32, pMode: *mut u32) -> NvAPI_Status;
    }

    nvapi! {
        /// NVCP power-mode SET (RE'd R610.74 @0x180261BC0). 4-arg by-value
        /// mirror of the GET: `fn(hGpu, powerSource∈{1,2}, queryType=3,
        /// mode∈{6,7})` — validated `queryType==3 && (mode-6)<=1`, mapped
        /// 6→0 / 7→1 internally. Same 0x700003A escape, op=2.
        pub unsafe fn NvAPI_GPU_SetPowerMizerInfo(hPhysicalGPU: NvPhysicalGpuHandle, powerSource: u32, queryType: u32, mode: u32) -> NvAPI_Status;
    }

    nvapi! {
        /// PPAB / Dynamic-Boost enable GET (RE'd R610.74 @0x180069CC0).
        /// `fn(active: *mut bool)` — single byte out, NO GPU handle (mirrors
        /// the by-value SET 0x1504FC3D). `*active = (statusByte != 2 &&
        /// statusByte2 != 2)` from the PCF private table (cmd 0x10C68).
        pub unsafe fn NvAPI_PCF_DynamicBoostGetStatus(pActive: *mut BoolU32) -> NvAPI_Status;
    }

    // ------------------------------------------------------------------
    // Core-voltage scalar triplet (RE'd R610.74; escape 0x07000043/44/45,
    // 56-byte buffer, selector @esc+0x28, value @esc+0x34). Distinct RM
    // surface from VoltVoltRails (0x07000191) — plain scalars, no version
    // magic, no GPU handle (selector-scoped).
    // ------------------------------------------------------------------

    nvapi! {
        /// Direct core-voltage read (0x58337FA3 @0x1801C9CE0):
        /// `fn(hGpu, *value: u32)` — the GPU handle lands in escape +0x28.
        /// (Live-verified: a non-handle first arg returns
        /// NVAPI_EXPECTED_PHYSICAL_GPU_HANDLE.)
        pub unsafe fn NvAPI_GPU_GetCoreVoltage(hPhysicalGPU: NvPhysicalGpuHandle, pValue: *mut u32) -> NvAPI_Status;
    }

    nvapi! {
        /// Core-voltage control-object read (0xA91F88EB @0x1801C9E30):
        /// `fn(hGpu, *value: u32)` — same shape, escape 0x07000045.
        pub unsafe fn NvAPI_GPU_GetCoreVoltageControl(hPhysicalGPU: NvPhysicalGpuHandle, pValue: *mut u32) -> NvAPI_Status;
    }

    nvapi! {
        /// Core-voltage control SET (0xDC2BD4A6 @0x1801CB300):
        /// `fn(hGpu, value: u32)` — both packed into the 56-byte escape
        /// (0x07000044). Elevation-gated (-104 without admin).
        pub unsafe fn NvAPI_GPU_SetCoreVoltageControl(hPhysicalGPU: NvPhysicalGpuHandle, value: u32) -> NvAPI_Status;
    }

    // ------------------------------------------------------------------
    // PMGR voltage-request arbiter (RE'd R610.74; escape 0x0700019F,
    // 112-byte buffer: gpuSelector@+0x30, get/set flag@+0x34). Versioned
    // struct: v1 magic 0x10024 (8 payload dwords), v2 0x20030 (+3 dwords).
    // Escape dword map: [1..4]@+0x38..0x44, [5..8]@+0x50..0x5C;
    // v2-only [9..10]@+0x48..0x4C, [11]@+0x60. Distinct from VoltVoltRails.
    // ------------------------------------------------------------------

    nvstruct! {
        /// v2 (48B): version + 11 payload dwords. Use this for both GET and
        /// SET (the driver accepts v1 0x10024 and v2 0x20030; v2 is the
        /// superset).
        pub struct NV_PMGR_VOLTAGE_ARBITER_VALUES_V2 {
            pub version: NvVersion,
            /// dwords[1..=11]: opaque arbiter values; the GET copies the
            /// driver's 8 (v1) or 11 (v2) dwords back, the SET forwards them.
            pub values: [u32; 11],
        }
    }

    nvversion! { @=NV_PMGR_VOLTAGE_ARBITER_VALUES NV_PMGR_VOLTAGE_ARBITER_VALUES_V2(2) = 48 }

    nvapi! {
        /// PMGR voltage-request arbiter GET (0x717648FD @0x1801C9F80):
        /// `fn(gpuSelector: u32, pVals)` — escape 0x0700019F with get-flag 0.
        pub unsafe fn NvAPI_GPU_GetPMGRVoltageRequestArbiterValues(hPhysicalGPU: NvPhysicalGpuHandle, pValues: *mut NV_PMGR_VOLTAGE_ARBITER_VALUES) -> NvAPI_Status;
    }

    nvapi! {
        /// PMGR voltage-request arbiter SET (0x9C4BB8D0 @0x1801CB480):
        /// same signature, escape get/set flag 1. Elevation-gated (-104).
        pub unsafe fn NvAPI_GPU_SetPMGRVoltageRequestArbiterValues(hPhysicalGPU: NvPhysicalGpuHandle, pValues: *const NV_PMGR_VOLTAGE_ARBITER_VALUES) -> NvAPI_Status;
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
    //
    // SEMANTIC CORRECTION (xOCD RE, 2026-10-05): the same GET/SET pair
    // carries MORE than TGP milliwatts — the (policyId, subtype)-keyed
    // channel table includes policyId 19 = per-rail OCP CURRENT limits in
    // raw mA: NVVDD=(19,13)/MSVDD=(19,12) (legacy (13,19)/(14,19)). The
    // 2636B v1|0x10A4C stamp (R465 variant below) is the layout xOCD drives
    // with a compact 40B channel stride. See NV_GPU_CLIENT_POWER_CHANNELS_INFO_V4.
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
            pub payload: Array<[u8; 10016 - 8]>,
        }
    }

    impl NV_GPU_CLIENT_TGP_WATT_STATUS_V1 {
        /// Byte offset WITHIN `payload` of entry 0's base — i.e. of the
        /// buffer byte 0x8A0. The ref-tool CLI writes `v14[553 + 10*idx]`,
        /// and payload starts at buffer byte 8, so entry 0's value field
        /// lands at buffer byte 553*4 = 0x8A4 = base 0x8A0 + value offset 4.
        pub const ENTRY_BASE: usize = 0x8A0;
        /// Per-entry stride in bytes (10 dwords = 40 bytes per entry) — the
        /// SAME stride the xOCD compact 0x10A4C view uses
        /// ([`NV_GPU_CLIENT_TGP_WATT_STATUS_10A4C_V1::COMPACT_STRIDE`]).
        pub const ENTRY_STRIDE: usize = 40;
        /// Byte offset of the power-mW value WITHIN an entry — the SAME
        /// value offset the compact view uses
        /// ([`NV_GPU_CLIENT_TGP_WATT_STATUS_10A4C_V1::COMPACT_VALUE_OFF`]).
        pub const ENTRY_VALUE_OFF: usize = 4;

        /// Byte offset WITHIN `payload` of entry 0's power-mW field, i.e.
        /// `ENTRY_BASE - 8` (payload base) `+ ENTRY_VALUE_OFF` = 0x89C.
        ///
        /// The 0x12720 full table and the xOCD compact 0x10A4C stamp are two
        /// views of ONE control table: they share the per-entry layout
        /// exactly (stride 40, value at entry+4 — see
        /// [`Self::ENTRY_STRIDE`]/[`Self::ENTRY_VALUE_OFF`] and the compact
        /// constants) and differ only in the table base (0x8A0 here vs
        /// [`NV_GPU_CLIENT_TGP_WATT_STATUS_10A4C_V1::COMPACT_ENTRY_BASE`] =
        /// 28 there). Row index = info-mask bit = control entry = write-mask
        /// bit; the write contract is `1 << index`.
        const POWER_STRIDE_BYTES: usize = Self::ENTRY_STRIDE;
        const POWER_BASE_PAYLOAD_OFF: usize = Self::ENTRY_BASE - 8 + Self::ENTRY_VALUE_OFF;

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

    // Pre-R538 TGP-watts GET variants (ID 0x8B3E7343). RE'd from
    // nvapi64_46296.dll (R465, handler 0x180266850): the version switch
    // accepts {0x10298, 0x106DC, 0x10A4C, 0x11F10} — the modern 0x12720 stamp
    // is 538+ only (diff-matrix-confirmed on 391.35/538.78/560.94/582.41/
    // 610.88impl: {0x10298, 0x106DC, 0x10A4C} are the universal stamps).
    // Geometry from the R465 fill code, all variants share the 136-byte entry
    // stride; they differ in header size and entry capacity:
    //   0x11F10 (v1|7952): 32 entries, entry table at buffer byte 3536
    //     (fill writes `dword[884 + 34*idx]`; per-entry: +0 status code,
    //     +4 u16 state, +72 power mW);
    //   0x10A4C (v1|2636): 6 entries, entry table at buffer byte 1756
    //     (fill writes `dword[439 + 34*idx]`, same per-entry layout);
    //   0x106DC (v1|1756): header-only in the 1756-byte struct — the fill
    //     path for it indexes past its end, so it is header-equivalent to
    //     0x10A4C's table base; not a useful GET target;
    //   0x10298 (v1|664): same-generation header view (no entry capacity).
    nvstruct! {
        /// 0x11F10 GET buffer: v1|7952, 32 × 136B entries @ +3536.
        pub struct NV_GPU_CLIENT_TGP_WATT_STATUS_11F10_V1 {
            pub version: NvVersion,
            pub mask: u32,
            /// Opaque header + entry table (raw; GET-filled).
            pub payload: Array<[u8; 7952 - 8]>,
        }
    }

    impl NV_GPU_CLIENT_TGP_WATT_STATUS_11F10_V1 {
        const ENTRY_STRIDE: usize = 136;
        const ENTRY_BASE: usize = 3536;
        const ENTRY_MW_OFF: usize = 72;
        const ENTRIES: usize = 32;

        /// Power-mW of entry `index` (None = sentinel 0xFFFFFFFF / OOB).
        pub fn power_mw(&self, index: usize) -> Option<u32> {
            if index >= Self::ENTRIES {
                return None;
            }
            let off = Self::ENTRY_BASE + Self::ENTRY_STRIDE * index + Self::ENTRY_MW_OFF;
            self.payload
                .get(off - 8..off - 8 + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .filter(|mw| *mw != 0xFFFF_FFFF)
        }
    }

    nvstruct! {
        /// 0x10A4C GET buffer: v1|2636, 6 × 136B entries @ +1756.
        pub struct NV_GPU_CLIENT_TGP_WATT_STATUS_10A4C_V1 {
            pub version: NvVersion,
            pub mask: u32,
            /// Opaque header + entry table (raw; GET-filled).
            pub payload: Array<[u8; 2636 - 8]>,
        }
    }

    impl NV_GPU_CLIENT_TGP_WATT_STATUS_10A4C_V1 {
        /// The exact struct-version stamp xOCD drives (`ReferenceOcp.Layout
        /// Small.Header` = 68172 = 0x0001_0A4C = v1|2636). NOTE: 68172 is
        /// NOT 0x0010_0A4C — the version nibble is 1, not 0x10. Every call
        /// site stamped 0x0010_0A4C through the pre-fix era and got -9
        /// (INCOMPATIBLE_STRUCT_VERSION) on every generation, 50-series
        /// included; that was a literal typo, not a generation gate.
        pub const STAMP: u32 = 0x0001_0A4C;

        pub const ENTRY_STRIDE: usize = 136;
        pub const ENTRY_BASE: usize = 1756;
        pub const ENTRY_MW_OFF: usize = 72;
        const ENTRIES: usize = 6;

        /// xOCD compact geometry (`ParsePowerChannelPolicies`
        /// NvApiSource.cs:2007/2036 `index*40+32`; `ReferenceOcp.Layout
        /// Small` = EntryBase 28, Stride 40, ValueDelta 4): the raw value
        /// sits at BUFFER byte 32+40*i (= entry+4; payload 24+40*i), with
        /// the entry's rail-type dword at entry+0 (28+40*i). Raw unit =
        /// **mA for the OCP channels** (policyId 19 family; UI shows
        /// A ×1000), TGP-mW on the Dlevel channels per the ref-tool
        /// reading — units are per-channel, see
        /// [`NV_GPU_CLIENT_POWER_CHANNELS_INFO_V4`].
        pub const COMPACT_STRIDE: usize = 40;
        /// Value-field offset within an entry (entry+4 ⇒ buffer 32+40*i).
        pub const COMPACT_VALUE_OFF: usize = 4;
        pub const COMPACT_ENTRY_BASE: usize = 28;
        /// The compact table is bounded by its mask contract (0x7FFF,
        /// xOCD `WriteMask`/`VerifyValues` iterate bits 0..=14) — 15
        /// entries, not the 6 the R465 136B-stride region can hold.
        pub const COMPACT_ENTRIES: usize = 15;

        /// Power-mW of entry `index` (None = sentinel 0xFFFFFFFF / OOB).
        pub fn power_mw(&self, index: usize) -> Option<u32> {
            if index >= Self::ENTRIES {
                return None;
            }
            let off = Self::ENTRY_BASE + Self::ENTRY_STRIDE * index + Self::ENTRY_MW_OFF;
            self.payload
                .get(off - 8..off - 8 + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .filter(|mw| *mw != 0xFFFF_FFFF)
        }

        /// Compact-geometry channel value (xOCD layout; see
        /// [`Self::COMPACT_STRIDE`]). None = OOB / sentinel.
        pub fn channel_value_compact(&self, index: usize) -> Option<u32> {
            if index >= Self::COMPACT_ENTRIES {
                return None;
            }
            let off =
                Self::COMPACT_ENTRY_BASE + Self::COMPACT_STRIDE * index + Self::COMPACT_VALUE_OFF
                    - 8;
            self.payload
                .get(off..off + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .filter(|v| *v != 0xFFFF_FFFF)
        }

        /// Write the compact-geometry channel value (sets mask bit `index`).
        pub fn set_channel_value_compact(&mut self, index: usize, raw: u32) {
            if index >= Self::COMPACT_ENTRIES {
                return;
            }
            let off =
                Self::COMPACT_ENTRY_BASE + Self::COMPACT_STRIDE * index + Self::COMPACT_VALUE_OFF
                    - 8;
            if let Some(slot) = self.payload.get_mut(off..off + 4) {
                slot.copy_from_slice(&raw.to_le_bytes());
                self.mask |= 1u32 << index;
            }
        }

        /// Detect which geometry the live GET filled: compare both candidate
        /// value lists against the info-side rows — a geometry "fits" when
        /// every **indexed** entry value lands inside its [min, max]
        /// window. The row index is the info-mask bit position, i.e. the
        /// control entry index (xOCD drives control entry i for info entry
        /// i; the populated set can have gaps). Returns true for the
        /// compact (xOCD) geometry, false for the R465 136B-stride one,
        /// None when ambiguous.
        pub fn detect_compact_geometry(&self, rows: &[(usize, u32, u32, u32)]) -> Option<bool> {
            if rows.is_empty() {
                return None;
            }
            let fits = |get: &dyn Fn(usize) -> Option<u32>| {
                rows.iter().all(|&(idx, lo, _def, hi)| {
                    get(idx).map(|v| v >= lo && v <= hi).unwrap_or(false)
                })
            };
            let compact_fits = fits(&|i| self.channel_value_compact(i));
            let r465_fits = fits(&|i| self.power_mw(i));
            match (compact_fits, r465_fits) {
                (true, false) => Some(true),
                (false, true) => Some(false),
                _ => None,
            }
        }
    }

    // ------------------------------------------------------------------
    // PowerChannels policy descriptors (NDA, ID 0x67F31384) — the OCP half
    // of the SAME interface family the ref tool drives as TGP-watts. xOCD RE
    // (NvApiSource.cs:1715-2058): info v4, stamp (4<<16)+2672 = 264816,
    // per-channel entries at buffer byte 56+88*i:
    //   policyId @+4, subtype @+8, min @+16, default @+20, max @+24
    // (raw **mA** on the policyId-19 OCP channels; UI shows A = raw/1000).
    // Channel identity is the (policyId, subtype) pair:
    //   NVVDD OCP = (19,13), legacy fallback (13,19);
    //   MSVDD OCP = (19,12), legacy fallback (14,19).
    // "firmware default" = the info `default` dword. The control half is
    // 0x8B3E7343/0xAFFC2279 with the 0x10A4C stamp (compact geometry) — see
    // NV_GPU_CLIENT_TGP_WATT_STATUS_10A4C_V1::channel_value_compact. Our
    // earlier 347124B ref-tool layout (below) is a DIFFERENT stamp of the
    // same ID; dispatch by returned/stamped size, never assume.
    // ------------------------------------------------------------------

    nvstruct! {
        /// PowerChannels policy/range descriptor — xOCD layout, info v4,
        /// stamp (4<<16)+2672. Opaque except the decoded accessors below.
        pub struct NV_GPU_CLIENT_POWER_CHANNELS_INFO_V4 {
            pub version: NvVersion,
            /// channel-populated mask (xOCD requests 0x7FFF)
            pub mask: u32,
            /// Opaque body; per-channel entries decoded by accessors.
            pub payload: Array<[u8; 2672 - 8]>,
        }
    }

    /// Byte offsets / identity constants for
    /// [`NV_GPU_CLIENT_POWER_CHANNELS_INFO_V4`]. Channel identities are
    /// generation-skewed — live E3 round 1 (2026-10-05, RTX 2070 + RTX 3060
    /// + Tesla P100):
    ///
    /// - board power = policyId 0, **raw mW**, on all three generations:
    ///   2070 (0,9) def 175000/max 219000 (= the 175 W spec), 3060 (0,0)
    ///   def 170000/max 212000 (= 170 W), P100 (0,0) def=max=250000
    ///   (= 250 W, Tesla-locked; min 125000) — match on policyId alone.
    /// - OCP current = raw mA, identity per generation: 50-series (19,13)/
    ///   (19,12) (xOCD pairing), Ampere (13,19) (3060: def 135000 / max
    ///   138068), Turing (6,19) (2070: def 215860 / max 240000). P100 has
    ///   NO current-OCP channel — its (6,1)/(3,7) entries are sentinels.
    /// - `default == max == 5001000` (and the (x,11)/(3,7) 1001000)
    ///   entries are "unbounded" sentinels — not real limits.
    pub mod power_channels_info_v4 {
        /// first per-channel entry (buffer-absolute)
        pub const ENTRY_BASE: usize = 56;
        /// per-channel entry stride
        pub const ENTRY_STRIDE: usize = 88;
        pub const POLICY_ID: usize = 4;
        pub const SUBTYPE: usize = 8;
        pub const MIN: usize = 16;
        pub const DEFAULT: usize = 20;
        pub const MAX: usize = 24;
        /// board-power channel policyId (raw mW; subtype generation-skewed)
        pub const BOARD_POWER_POLICY_ID: u32 = 0;
        /// NVVDD OCP (policyId, subtype) — 50-series pairing (xOCD)
        pub const OCP_NVVDD: (u32, u32) = (19, 13);
        /// NVVDD OCP Ampere pairing (RTX 3060 live E3)
        pub const OCP_NVVDD_AMPERE: (u32, u32) = (13, 19);
        /// NVVDD OCP Turing pairing (RTX 2070 live E3)
        pub const OCP_NVVDD_TURING: (u32, u32) = (6, 19);
        /// MSVDD OCP (policyId, subtype) — 50-series pairing (xOCD)
        pub const OCP_MSVDD: (u32, u32) = (19, 12);
        /// MSVDD OCP legacy fallback (xOCD); present-but-sentinel on Ampere
        pub const OCP_MSVDD_LEGACY: (u32, u32) = (14, 19);
        /// write-value hard clamp from xOCD (raw mA)
        pub const OCP_RAW_MIN: u32 = 1000;
        /// write-value hard clamp from xOCD (raw mA) AND the family's
        /// "unbounded" sentinel value
        pub const OCP_RAW_MAX: u32 = 5_001_000;
    }

    impl NV_GPU_CLIENT_POWER_CHANNELS_INFO_V4 {
        fn entry_field(&self, i: usize, off: usize) -> Option<u32> {
            let abs = power_channels_info_v4::ENTRY_BASE
                .checked_add(i.checked_mul(power_channels_info_v4::ENTRY_STRIDE)?)?
                .checked_add(off)?;
            let off = abs.checked_sub(8)?;
            self.payload
                .get(off..off + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        }

        /// Channel `i` (policyId, subtype).
        pub fn channel_id(&self, i: usize) -> Option<(u32, u32)> {
            Some((
                self.entry_field(i, power_channels_info_v4::POLICY_ID)?,
                self.entry_field(i, power_channels_info_v4::SUBTYPE)?,
            ))
        }

        /// Channel `i` (min, default, max) raw triplet (mA on OCP channels).
        pub fn channel_range(&self, i: usize) -> Option<(u32, u32, u32)> {
            Some((
                self.entry_field(i, power_channels_info_v4::MIN)?,
                self.entry_field(i, power_channels_info_v4::DEFAULT)?,
                self.entry_field(i, power_channels_info_v4::MAX)?,
            ))
        }

        /// Index of the channel matching `(policy_id, subtype)` among the
        /// mask-populated entries — exact pair first, then the legacy
        /// fallback pair when provided.
        pub fn find_channel(
            &self,
            mask_bits: u32,
            policy_id: u32,
            subtype: u32,
            legacy: Option<(u32, u32)>,
        ) -> Option<usize> {
            for exact in [true, false] {
                if !exact && legacy.is_none() {
                    break;
                }
                let (p, s) = if exact {
                    (policy_id, subtype)
                } else {
                    legacy.unwrap()
                };
                for bit in 0..15u32 {
                    if mask_bits & (1 << bit) == 0 {
                        continue;
                    }
                    if self.channel_id(bit as usize) == Some((p, s)) {
                        return Some(bit as usize);
                    }
                }
            }
            None
        }
    }

    nvversion! { @=NV_GPU_CLIENT_POWER_CHANNELS_INFO NV_GPU_CLIENT_POWER_CHANNELS_INFO_V4(4) = 2672 }

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
            pub hdr3: Array<[u32; 6]>,
            /// dword 12 (the ref tool reads it into a "hide TGP" sibling field).
            pub hide_tgp_flag_dword: u32,
            /// Per-policy entry table (10604 B each); raw, parsed by accessors.
            /// Header before this = 52 bytes (dwords 0..12 + the index byte).
            /// Total struct = 347124 B (matches the ref tool's v7[86784] + memset).
            pub entries: Array<[u8; 347124 - 52]>,
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

    nvstruct! {
        /// Private ClientPowerPoliciesGetInfo SMALL-V1 (ID 0x67F31384, stamp
        /// `0x612E4` = v6|4836). RE'd from nvapi64_46296.dll (R465): the
        /// handler (0x1802670A0) accepts {0x10628, 0x10828, 0x10A3C, 0x40A70,
        /// 0x50DE4, 0x612E4} — the ver-15 347KB stamp (0xF4BF4) is R560+ only
        /// (first accepted in nvapi64_56094.dll; 538.78 rejects it too, with
        /// INCOMPATIBLE_STRUCT_VERSION (-9)). 0x612E4 is the universal
        /// pre-R560 stamp (391.35/462.96/538.78 all take it). Layout beyond
        /// the version dword is opaque research territory: the handler fills
        /// a per-policy {flag byte, dword} quad at +656..+704 (8-byte stride
        /// × 4 from the internal escape buffer at +5240) plus scattered bytes
        /// at +30..36/+362 — min/default/max mW offsets not yet pinned. Field
        /// semantics pending live calibration.
        pub struct NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE_SMALL_V1 {
            pub version: NvVersion,
            /// opaque driver-filled payload (4832 bytes)
            pub payload: Array<[u8; 4836 - 4]>,
        }
    }

    impl NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE_SMALL_V1 {
        /// The ver-6 stamp 0x612E4 (v6|4836) every pre-R560 branch accepts.
        pub const STAMP: u32 = 0x612E4;
    }

    // ==== RM power-policy object model (kernel) ↔ the wrapped TGP surfaces ====
    //
    // The NvpwrControl 616.92 kernel RE
    // (docs/reverse-engineering/nvapi/nvpwrcontrol-blackwell-tuner-audit.md §2)
    // resolved nvlddmkm's live power-policy objects. The model UNDER this
    // wrapped surface reads:
    //
    //   global+0x208 → GPU table (stride 0x10) → Major+0x25B0 → PowerRoot:
    //     init@0x3D10, elig@0x3D11, amountActive@0x3D12, base@0x3D14,
    //     amount@0x3D18, policyKey@0x3D1C, LOWER@0x3D20, UPPER@0x3D24 (mW)
    //   Board: setFn@0x2D0, selector2 = MAX (source 0xFE) @0x104,
    //     selector3 = CURRENT (source 0xF7 = generator output) @0x1F4;
    //     selector slot = {mode@0, count@1, effective@4, secondary@8,
    //     src[]@0xC stride 8}
    //
    // The F7 generator (RVA 0x4E3EF0 on 616.92) computes
    //     F7 = min(C+A, U)
    // from base C, PPAB amount A, LOWER B and UPPER U, active when
    // elig=1 ∧ amountActive=1 ∧ U>B ∧ A≤U−B. State semantics:
    //   stock    : elig=0, amount=0, base==UPPER, F7==UPPER (current rides max)
    //   DB-active: elig=1, base=cTGP, amount=DB budget → F7=min(cTGP+A, U)
    //
    // Consequences for THIS crate:
    //   - A SET above MAX is rejected by the RM policy layer (why the desktop
    //     power-limit path cannot exceed laptop ceilings) — only the generator
    //     INPUTS (base/amount via PPAB/TGP-watt, elig via 0x1504FC3D) have
    //     wrapped exits; the ceilings (UPPER / Board MAX / Type07 / NVPCF
    //     maxima) have NO command-surface exit (audit §6: user-mode cannot
    //     raise them).
    //   - User-mode view mapping: `tgp_watt_range().min_mw` ↔ LOWER,
    //     `max_mw` ↔ Board MAX/UPPER ceiling, `tgp_watt_status().current_mw`
    //     ↔ the F7/CURRENT side. The kernel stock base (==UPPER) and the live
    //     elig/amount flags are NOT directly visible — GetInfo's "default" is
    //     the slider default, NOT the kernel base (70 W vs 140 W on the
    //     reference machine).
    //   - The kernel route (if ever) is a documented three-phase transfer —
    //     A: stage base+amount under the OEM ceiling, B: raise Board MAX +
    //     UPPER, C: re-run the native generator + full readback — with
    //     same-session baseline capture and identity-guarded rollback
    //     (audit §2.6). Test-signed-driver costs: audit §1.

    nvapi! {
        /// Undocumented (NDA, ID 0x67F31384). Private ClientPowerPoliciesGetInfo
        /// variant — the TGP-watts min/default/max range + active policy index.
        /// NOT the public 0x34206D86. Returns a 347124-byte struct with version
        /// magic 0x0F4BF4 (version 15) — the ref tool's queryPowerPolicy uses exactly
        /// this; the version-1 magic I first tried is rejected by the driver.
        pub unsafe fn NvAPI_GPU_ClientPowerPoliciesGetInfoPrivate(hPhysicalGPU: NvPhysicalGpuHandle, pInfo: *mut NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE) -> NvAPI_Status;
    }

    nvstruct! {
        /// ClientPowerPoliciesSetInfo PRIVATE V1 (ID 0xAD9A2E6D, PNY VelocityX
        /// `NVpower_wrapper.dll` Nvpower_GPU_SetPowerCap, RE'd 2026-08-25).
        /// Minimal TGP-watt SET layout — the struct the wrapper builds
        /// instruction-verified: version magic 0x10088, +4 dword = 1 (policy
        /// selector), +8 dword = power target in mW. VelocityX precedes the
        /// SET with a public GetInfo (0x34206D86) using the 188B 0x100F8 V1
        /// layout. Live-probe R610.74 mobile 4060: QI NULL (desktop-driver
        /// family — untested there).
        pub struct NV_GPU_CLIENT_POWER_POLICIES_SET_INFO_V1 {
            pub version: NvVersion,
            pub selector: u32,
            pub power_target_mw: u32,
            pub padding: Padding<[u32; 136/4 - 3]>,
        }
    }

    nvversion! { @=NV_GPU_CLIENT_POWER_POLICIES_SET_INFO NV_GPU_CLIENT_POWER_POLICIES_SET_INFO_V1(1) = 136 }

    nvapi! {
        /// Undocumented (ID 0xAD9A2E6D). ClientPowerPoliciesSetInfo private
        /// variant — minimal V1 TGP-watt setter (PNY VelocityX SetPowerCap).
        /// Sibling of the public SetStatus (0xAD95F5ED); struct magic 0x10088.
        pub unsafe fn NvAPI_GPU_ClientPowerPoliciesSetInfoPrivate(hPhysicalGPU: NvPhysicalGpuHandle, pSetInfo: *const NV_GPU_CLIENT_POWER_POLICIES_SET_INFO) -> NvAPI_Status;
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
            counted(&*self.channels, self.count as usize)
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
            counted(&*self.entries, self.count as usize)
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
            pub buf1: Array<[u32; 30]>,
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
            counted(&*self.entries, self.count as usize)
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
            pub data: Array<[u8; 16]>,
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
            pub data: Array<[u8; 32]>,
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
            /// explicit padding to 4-byte alignment for `channels` (29 -> 32)
            pub rsvd: Padding<[u8; 11]>,
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
                .then_some(())
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
                    pub header_rsvd: Array<[u8; 0x34 - 0x1D]>,
                    /// Variable-length, sparsely-packed per-channel descriptors.
                    /// Parsed by signature scan (channel_type 1..=8 + plausible
                    /// PowerRail); record length varies with channel_type.
                    pub descriptors: Array<[u8; $magic_size - 0x34]>,
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

    // ------------------------------------------------------------------
    // xOCD 2.0 ExtendedLimits geometries (audit: docs/reverse-engineering/
    // nvapi/xocd-2.0.0-capability-delta.md §4; decompiled sources
    // xOCD.ExtendedLimits `BlackwellPowerCommand` / `BlackwellNativePort` /
    // `PolicyGraph` / `BlackwellLimitSession`).
    //
    // 0x33AB0353 GET / 0x17695269 SET are the PwrPolicies "Control" pair — a
    // per-channel command/lease surface distinct from the TGP-watt control
    // (0x8B3E7343/0xBFF09E59): xOCD's kernel-driver flow drives Blackwell
    // power-cap patches through it with readback verification. The large
    // buffers below do NOT follow the (ver<<16)|size shortcut for their
    // stamps — send the recorded dwords verbatim.
    // ------------------------------------------------------------------

    nvstruct! {
        /// Power-command lease packet (NDA 0x33AB0353 GET / 0x17695269 SET;
        /// xOCD 2.0 `BlackwellPowerCommand.Packet`). 1320 bytes:
        /// `[0]=stamp 0x0001_0528` (v1|1320), `[4]=channel mask 1<<channel`,
        /// then 32 slots of 40 bytes at `40*(channel+1)` — `[+0]=value`,
        /// `[+4]=command` (0xFE writable request/lease, 0xF8 observed-only);
        /// the trailing 32 bytes are reserved. The GET echoes stamp + mask +
        /// command and returns the slot value; xOCD validates a write by
        /// re-reading and only ever writes 0xFE.
        pub struct NV_GPU_POWER_COMMAND_PACKET_V1 {
            pub version: NvVersion,
            pub mask: u32,
            pub payload: Array<[u8; 1320 - 8]>,
        }
    }

    impl NV_GPU_POWER_COMMAND_PACKET_V1 {
        /// v1|1320.
        pub const STAMP: u32 = 0x0001_0528;
        /// Writable per-channel request/lease command (the only one xOCD writes).
        pub const COMMAND_FE: u32 = 0xFE;
        /// Observed-only readback command.
        pub const COMMAND_F8: u32 = 0xF8;
        /// Highest addressable channel (the mask is 32 bits; xOCD's packet
        /// builder validates 0..=31).
        pub const MAX_CHANNEL: usize = 31;
        /// Per-channel slot stride in bytes.
        pub const SLOT_STRIDE: usize = 40;

        /// Buffer offset of `channel`'s 40-byte slot (10 bytes before each
        /// 40-byte stride boundary — xOCD's `40*(channel+1)`).
        pub fn slot_off(channel: usize) -> Option<usize> {
            Self::SLOT_STRIDE.checked_mul(channel.checked_add(1)?)
        }

        /// Slot value dword (buffer-absolute `40*(channel+1)`).
        pub fn value(&self, channel: usize) -> Option<u32> {
            let off = Self::slot_off(channel)?.checked_sub(8)?;
            self.payload
                .get(off..off + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        }

        /// Slot command dword (buffer-absolute `40*(channel+1)+4`).
        pub fn command(&self, channel: usize) -> Option<u32> {
            let off = Self::slot_off(channel)?.checked_sub(8)?.checked_add(4)?;
            self.payload
                .get(off..off + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        }
    }

    nvapi! {
        /// Undocumented (NDA, ID 0x33AB0353). PwrPolicies Control GET — the
        /// read half of the power-command lease pair (xOCD
        /// `BlackwellNativePort.ReadPowerCommand`). Packet:
        /// [`NV_GPU_POWER_COMMAND_PACKET_V1`].
        pub unsafe fn NvAPI_GPU_ClientPwrPoliciesGetControl(hPhysicalGPU: NvPhysicalGpuHandle, pPacket: *mut NV_GPU_POWER_COMMAND_PACKET_V1) -> NvAPI_Status;
    }

    nvapi! {
        /// Undocumented (NDA, ID 0x17695269). PwrPolicies Control SET — the
        /// writable half of the power-command lease pair (xOCD
        /// `BlackwellNativePort.SetPowerCommand`); xOCD writes command 0xFE
        /// and verifies by re-reading.
        pub unsafe fn NvAPI_GPU_ClientPwrPoliciesSetControl(hPhysicalGPU: NvPhysicalGpuHandle, pPacket: *const NV_GPU_POWER_COMMAND_PACKET_V1) -> NvAPI_Status;
    }

    /// One decoded policy entry from the power graph (xOCD `PolicyEntry`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PolicyGraphEntry {
        pub index: usize,
        /// xOCD `Type`: 0 board / 4 (Ada clock role) / 23 shared / 24 root /
        /// 25 core.
        pub role_type: u32,
        /// xOCD `Channel` (entry+4).
        pub channel: u8,
        /// xOCD `Unit` (entry+8).
        pub unit: u32,
        /// xOCD `DefaultRaw` (entry+16).
        pub default_raw: u32,
        /// xOCD `MaximumRaw` (entry+20).
        pub maximum_raw: u32,
    }

    nvstruct! {
        /// Private ClientPowerPoliciesGetInfo BLACKWELL graph (ID 0x67F31384;
        /// xOCD 2.0 `BlackwellNativePort.PowerGraph`): 2,727,984 bytes with
        /// stamp 0x2BA030 — NOT (ver<<16)|size here (the low word 0xA030 is
        /// not the buffer size 0x29A030); send the stamp verbatim. xOCD
        /// synthesizes this shape from the 347,124-byte layout when the
        /// driver refuses this geometry with -9. Layout beyond the offsets
        /// [`Self`]'s accessors read is opaque.
        pub struct NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE_BLACKWELL_V1 {
            pub version: NvVersion,
            pub payload: Array<[u8; 2_727_984 - 4]>,
        }
    }

    impl NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE_BLACKWELL_V1 {
        /// v43|0xA030 — send verbatim (the low word is NOT the buffer size).
        pub const STAMP: u32 = 0x2BA030;
        /// `PolicyGraph.Validate`'s exact length contract.
        pub const SIZE: usize = 2_727_984;
        /// Root policy-index byte pair (xOCD prefers [133] over [132]; 0xFF
        /// = absent).
        pub const ROOT_INDEX_OFF: usize = 132;
        /// Policy-entry table (xOCD `PolicyGraph.Entries`).
        pub const ENTRY_BASE: usize = 1200;
        pub const ENTRY_STRIDE: usize = 10604;
        /// `PolicyGraph.Entries` iterates 0..255.
        pub const MAX_POLICIES: usize = 255;
        /// Relation table (xOCD `PolicyGraph.Rel`): stride 72, validity mask
        /// at byte 100.
        pub const REL_BASE: usize = 2_705_220;
        pub const REL_STRIDE: usize = 72;

        fn dword(&self, abs: usize) -> Option<u32> {
            let off = abs.checked_sub(4)?;
            self.payload
                .get(off..off + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        }

        fn word(&self, abs: usize) -> Option<u16> {
            let off = abs.checked_sub(4)?;
            self.payload
                .get(off..off + 2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
        }

        fn byte(&self, abs: usize) -> Option<u8> {
            self.payload.get(abs.checked_sub(4)?).copied()
        }

        /// The root policy index (xOCD's exact read: [132], else [133], 0xFF
        /// = absent).
        pub fn root_index(&self) -> Option<usize> {
            let a = self.byte(Self::ROOT_INDEX_OFF)?;
            if a == 0xFF {
                return None;
            }
            let b = self.byte(Self::ROOT_INDEX_OFF + 1)?;
            Some(if b == 0xFF { a } else { b } as usize)
        }

        /// Whether the entry-validity mask at byte 4 marks policy `index`
        /// present.
        pub fn policy_valid(&self, index: usize) -> bool {
            index < Self::MAX_POLICIES
                && self
                    .dword(4 + (index / 32) * 4)
                    .map(|m| m & (1u32 << (index % 32)) != 0)
                    .unwrap_or(false)
        }

        /// Decode policy entry `index` (xOCD `PolicyEntry`).
        pub fn policy_entry(&self, index: usize) -> Option<PolicyGraphEntry> {
            if !self.policy_valid(index) {
                return None;
            }
            let base = Self::ENTRY_BASE.checked_add(index.checked_mul(Self::ENTRY_STRIDE)?)?;
            Some(PolicyGraphEntry {
                index,
                role_type: self.dword(base)?,
                channel: self.byte(base + 4)?,
                unit: self.dword(base + 8)?,
                default_raw: self.dword(base + 16)?,
                maximum_raw: self.dword(base + 20)?,
            })
        }

        /// The relation-index pair at entry+364/+366 (Blackwell reads these
        /// from the root and shared entries to reach the roles).
        pub fn policy_children(&self, index: usize) -> Option<(u16, u16)> {
            if !self.policy_valid(index) {
                return None;
            }
            let base = Self::ENTRY_BASE.checked_add(index.checked_mul(Self::ENTRY_STRIDE)?)?;
            Some((self.word(base + 364)?, self.word(base + 366)?))
        }

        /// Ada clock-role discriminator: the byte at entry+364 (0 = core,
        /// 1 = memory — xOCD's exact selector).
        pub fn policy_clock_discriminator(&self, index: usize) -> Option<u8> {
            if !self.policy_valid(index) {
                return None;
            }
            let base = Self::ENTRY_BASE.checked_add(index.checked_mul(Self::ENTRY_STRIDE)?)?;
            self.byte(base + 364)
        }

        /// Follow relation `rel_index` (xOCD `PolicyGraph.Rel`): must be set
        /// in the byte-100 validity mask, carry 0 at +0 and the 4096 contract
        /// u16 at +24; returns the target policy index (byte +4).
        pub fn relation_target(&self, rel_index: usize) -> Option<usize> {
            if rel_index >= Self::MAX_POLICIES {
                return None;
            }
            let mask = self.dword(100 + (rel_index / 32) * 4)?;
            if mask & (1u32 << (rel_index % 32)) == 0 {
                return None;
            }
            let base = Self::REL_BASE.checked_add(rel_index.checked_mul(Self::REL_STRIDE)?)?;
            if self.dword(base)? != 0 || self.word(base + 24)? != 4096 {
                return None;
            }
            Some(self.byte(base + 4)? as usize)
        }
    }

    /// Role-type tag of the board policy entry in the input-policy CONTROL
    /// buffer (xOCD `PowerControlValue`).
    pub const NV_GPU_POWER_CONTROL_ROLE_TYPE_BOARD: u32 = 0;
    /// Role-type tag of the shared policy entry.
    pub const NV_GPU_POWER_CONTROL_ROLE_TYPE_SHARED: u32 = 23;
    /// Role-type tag of the root policy entry.
    pub const NV_GPU_POWER_CONTROL_ROLE_TYPE_ROOT: u32 = 24;
    /// Input-policy CONTROL entry stride (xOCD `BlackwellLimitSession`).
    pub const NV_GPU_POWER_CONTROL_ENTRY_STRIDE: usize = 9288;

    /// Decode one input-policy CONTROL role entry — type at
    /// `entry_base + policy*9288`, value at +68 (xOCD
    /// `BlackwellNativePort.ReadPowerControlObservation`). `policy` must be
    /// inside the 32-bit mask contract.
    pub fn power_control_input_entry(
        buf: &[u8],
        entry_base: usize,
        policy: usize,
    ) -> Option<(u32, u32)> {
        if policy >= 32 {
            return None;
        }
        let at = entry_base.checked_add(policy.checked_mul(NV_GPU_POWER_CONTROL_ENTRY_STRIDE)?)?;
        let role_type = buf
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))?;
        let value = buf
            .get(at + 68..at + 72)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))?;
        Some((role_type, value))
    }

    nvstruct! {
        /// Input-policy CONTROL, xOCD "Modern" geometry of the multiplexed
        /// GET 0x8B3E7343 (the same ID as the TGP-watt control): 2,393,824
        /// bytes, stamp 0x2786E0, role mask seeded at byte 136 and echoed
        /// back. Entries at 2664 + policy*9288 (type@+0, value@+68); opaque
        /// elsewhere. xOCD `BlackwellLimitSession.ModernInput`.
        pub struct NV_GPU_CLIENT_POWER_CONTROL_INPUT_MODERN_V1 {
            pub version: NvVersion,
            pub header_rsvd: Padding<[u8; 132]>,
            pub mask: u32,
            pub payload: Array<[u8; 2_393_824 - 140]>,
        }
    }

    impl NV_GPU_CLIENT_POWER_CONTROL_INPUT_MODERN_V1 {
        pub const STAMP: u32 = 0x2786E0;
        pub const ENTRY_BASE: usize = 2664;
    }

    nvstruct! {
        /// Input-policy CONTROL, xOCD "Legacy" geometry of GET 0x8B3E7343:
        /// 307,376 bytes, stamp 0x5B0B0, mask at byte 136, entries at
        /// 2608 + policy*9288. xOCD `BlackwellLimitSession.LegacyInput`.
        pub struct NV_GPU_CLIENT_POWER_CONTROL_INPUT_LEGACY_V1 {
            pub version: NvVersion,
            pub header_rsvd: Padding<[u8; 132]>,
            pub mask: u32,
            pub payload: Array<[u8; 307_376 - 140]>,
        }
    }

    impl NV_GPU_CLIENT_POWER_CONTROL_INPUT_LEGACY_V1 {
        pub const STAMP: u32 = 0x5B0B0;
        pub const ENTRY_BASE: usize = 2608;
    }

    #[cfg(test)]
    mod xocd2_extended_limits_tests {
        use super::*;

        const GRAPH_SZ: usize = NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE_BLACKWELL_V1::SIZE;

        fn put_dword(buf: &mut [u8], at: usize, v: u32) {
            buf[at..at + 4].copy_from_slice(&v.to_le_bytes());
        }

        fn put_word(buf: &mut [u8], at: usize, v: u16) {
            buf[at..at + 2].copy_from_slice(&v.to_le_bytes());
        }

        fn graph_entry(i: usize) -> usize {
            NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE_BLACKWELL_V1::ENTRY_BASE
                + i * NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE_BLACKWELL_V1::ENTRY_STRIDE
        }

        fn graph_rel(i: usize) -> usize {
            NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE_BLACKWELL_V1::REL_BASE
                + i * NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE_BLACKWELL_V1::REL_STRIDE
        }

        #[test]
        fn power_command_packet_layout() {
            use NV_GPU_POWER_COMMAND_PACKET_V1 as Pkt;
            assert_eq!(core::mem::size_of::<Pkt>(), 1320);
            assert_eq!(Pkt::STAMP, 0x0001_0528);
            assert_eq!(Pkt::STAMP, (1 << 16) | 1320);
            assert_eq!(Pkt::slot_off(0), Some(40));
            assert_eq!(Pkt::slot_off(31), Some(40 * 32));
            let mut pkt = Pkt::zeroed();
            pkt.version.data = Pkt::STAMP;
            pkt.mask = 1 << 3;
            // channel 3 slot: buffer offset 160 → payload offset 152
            pkt.payload[152..156].copy_from_slice(&42u32.to_le_bytes());
            pkt.payload[156..160].copy_from_slice(&Pkt::COMMAND_FE.to_le_bytes());
            assert_eq!(pkt.value(3), Some(42));
            assert_eq!(pkt.command(3), Some(Pkt::COMMAND_FE));
            assert_eq!(pkt.value(2), Some(0));
        }

        /// The 0x12720 full table (v1|10016) and the xOCD compact 0x10A4C
        /// table (v1|2636) are two views of ONE control table: same per-entry
        /// stride, same value-field offset, different table base. Row index =
        /// info-mask bit = control entry = write-mask bit in both.
        #[test]
        fn tgp_full_and_compact_views_align() {
            use NV_GPU_CLIENT_TGP_WATT_STATUS_10A4C_V1 as Compact;
            use NV_GPU_CLIENT_TGP_WATT_STATUS_V1 as Full;

            // Shared per-entry geometry.
            assert_eq!(Full::ENTRY_STRIDE, Compact::COMPACT_STRIDE);
            assert_eq!(Full::ENTRY_VALUE_OFF, Compact::COMPACT_VALUE_OFF);
            assert_eq!(Full::ENTRY_STRIDE, 40);
            assert_eq!(Full::ENTRY_VALUE_OFF, 4);

            // The bases differ (full header 0x8A0 vs compact 28) — that is
            // the ONLY geometric difference between the two stamps.
            assert_ne!(Full::ENTRY_BASE, Compact::COMPACT_ENTRY_BASE);
            assert_eq!(Full::ENTRY_BASE, 0x8A0);
            assert_eq!(Compact::COMPACT_ENTRY_BASE, 28);

            // A buffer whose two views agree per index: write the full-table
            // value at its buffer dword (0x8A4 + 40i) and read it back through
            // the compact reader on a buffer laid out at the compact base.
            // Both resolve to their own base + value offset, i.e. the value
            // dword is always `entry_base + 4`.
            let mut buf = vec![0u8; 10016];
            buf[..4].copy_from_slice(&((1u32 << 16) | 10016).to_ne_bytes());
            let full = buf.as_mut_ptr() as *mut Full;
            for i in 0..15usize {
                unsafe { (*full).set_power_mw(i, 100_000 + i as u32) };
            }
            for i in 0..15usize {
                assert_eq!(unsafe { (*full).power_mw(i) }, Some(100_000 + i as u32));
                // the value dword is always base + value offset, stride 40
                let buf_dword =
                    (Full::ENTRY_BASE + Full::ENTRY_STRIDE * i + Full::ENTRY_VALUE_OFF) / 4;
                let word =
                    u32::from_ne_bytes(buf[buf_dword * 4..buf_dword * 4 + 4].try_into().unwrap());
                assert_eq!(word, 100_000 + i as u32);
            }
        }

        /// Byte-replay of xOCD `PolicyGraph.Roles` on a synthetic Blackwell
        /// graph: root 10 (type 24) → board 2 (type 0) + shared 3 (type 23)
        /// → core 4 (type 25, unit 1).
        #[test]
        fn power_graph_offsets_blackwell() {
            let mut g = vec![0u8; GRAPH_SZ];
            put_dword(
                &mut g,
                0,
                NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE_BLACKWELL_V1::STAMP,
            );
            let mut mask = 0u32;
            for i in [2usize, 3, 4, 10] {
                mask |= 1 << i;
            }
            put_dword(&mut g, 4, mask);
            g[132] = 10;
            g[133] = 10;
            put_dword(&mut g, graph_entry(10), 24);
            put_word(&mut g, graph_entry(10) + 364, 20);
            put_word(&mut g, graph_entry(10) + 366, 21);
            put_dword(&mut g, graph_entry(3), 23);
            put_word(&mut g, graph_entry(3) + 364, 22);
            put_dword(&mut g, graph_entry(4), 25);
            put_dword(&mut g, graph_entry(4) + 8, 1);
            let mut rel_mask = 0u32;
            for r in [20usize, 21, 22] {
                rel_mask |= 1 << r;
            }
            put_dword(&mut g, 100, rel_mask);
            for (r, target) in [(20usize, 2usize), (21, 3), (22, 4)] {
                put_dword(&mut g, graph_rel(r), 0);
                put_word(&mut g, graph_rel(r) + 24, 4096);
                g[graph_rel(r) + 4] = target as u8;
            }
            let graph: &NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE_BLACKWELL_V1 =
                unsafe { &*(g.as_ptr() as *const _) };
            assert_eq!(graph.root_index(), Some(10));
            let root = graph.policy_entry(10).expect("root entry");
            assert_eq!((root.role_type, root.unit), (24, 0));
            assert_eq!(graph.policy_children(10), Some((20, 21)));
            assert_eq!(graph.relation_target(20), Some(2));
            assert_eq!(graph.relation_target(21), Some(3));
            assert_eq!(graph.relation_target(22), Some(4));
            let shared = graph.policy_entry(3).expect("shared entry");
            assert_eq!((shared.role_type, shared.unit), (23, 0));
            let core = graph.policy_entry(4).expect("core entry");
            assert_eq!((core.role_type, core.unit), (25, 1));
            // the mask contract: entry 1's bytes are zero and it is NOT valid
            assert_eq!(graph.policy_entry(1), None);
            // a relation whose contract u16 is wrong is rejected
            put_word(&mut g, graph_rel(20) + 24, 4095);
            let g2: &NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE_BLACKWELL_V1 =
                unsafe { &*(g.as_ptr() as *const _) };
            assert_eq!(g2.relation_target(20), None);
        }

        /// Ada family: root type 0; two type-4/unit-1 clock roles
        /// distinguished by the entry+364 discriminator 0/1 with an equal
        /// channel byte.
        #[test]
        fn power_graph_offsets_ada_clock_roles() {
            let mut g = vec![0u8; GRAPH_SZ];
            put_dword(
                &mut g,
                0,
                NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE_BLACKWELL_V1::STAMP,
            );
            let mut mask = 0u32;
            for i in [5usize, 6, 7] {
                mask |= 1 << i;
            }
            put_dword(&mut g, 4, mask);
            g[132] = 5;
            g[133] = 5;
            for (i, disc) in [(6usize, 0u8), (7, 1)] {
                put_dword(&mut g, graph_entry(i), 4);
                put_dword(&mut g, graph_entry(i) + 8, 1);
                g[graph_entry(i) + 4] = 9;
                g[graph_entry(i) + 364] = disc;
            }
            let graph: &NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE_BLACKWELL_V1 =
                unsafe { &*(g.as_ptr() as *const _) };
            assert_eq!(graph.root_index(), Some(5));
            assert_eq!(graph.policy_clock_discriminator(6), Some(0));
            assert_eq!(graph.policy_clock_discriminator(7), Some(1));
            assert_eq!(graph.policy_entry(6).map(|e| e.channel), Some(9));
        }

        #[test]
        fn input_control_geometry_layout() {
            assert_eq!(NV_GPU_CLIENT_POWER_CONTROL_INPUT_MODERN_V1::STAMP, 0x2786E0);
            assert_eq!(NV_GPU_CLIENT_POWER_CONTROL_INPUT_LEGACY_V1::STAMP, 0x5B0B0);
            assert_eq!(
                core::mem::size_of::<NV_GPU_CLIENT_POWER_CONTROL_INPUT_MODERN_V1>(),
                2_393_824
            );
            assert_eq!(
                core::mem::size_of::<NV_GPU_CLIENT_POWER_CONTROL_INPUT_LEGACY_V1>(),
                307_376
            );
            let at = NV_GPU_CLIENT_POWER_CONTROL_INPUT_MODERN_V1::ENTRY_BASE + 3 * 9288;
            let mut buf = vec![0u8; at + 72];
            put_dword(&mut buf, at, NV_GPU_POWER_CONTROL_ROLE_TYPE_SHARED);
            put_dword(&mut buf, at + 68, 1234);
            assert_eq!(
                power_control_input_entry(
                    &buf,
                    NV_GPU_CLIENT_POWER_CONTROL_INPUT_MODERN_V1::ENTRY_BASE,
                    3
                ),
                Some((NV_GPU_POWER_CONTROL_ROLE_TYPE_SHARED, 1234))
            );
            // the 32-bit mask contract bounds the index
            assert_eq!(
                power_control_input_entry(
                    &buf,
                    NV_GPU_CLIENT_POWER_CONTROL_INPUT_MODERN_V1::ENTRY_BASE,
                    32
                ),
                None
            );
            // the Legacy base is 112 bytes lower — the same fixture reads
            // zeros there (geometry separation is real)
            assert_eq!(
                power_control_input_entry(
                    &buf,
                    NV_GPU_CLIENT_POWER_CONTROL_INPUT_LEGACY_V1::ENTRY_BASE,
                    3
                ),
                Some((0, 0))
            );
        }
    }

    #[cfg(test)]
    mod xocd_power_channels_tests {
        use super::*;

        /// xOCD PowerChannels info v4 layout: entries at 56+88*i with
        /// policyId/subtype/min/default/max — byte-replay of the (19,13)
        /// NVVDD OCP channel.
        #[test]
        fn power_channels_info_v4_parse() {
            let mut buf = NV_GPU_CLIENT_POWER_CHANNELS_INFO_V4::zeroed();
            buf.mask = 0x7FFF;
            // entry 0: policyId@60, subtype@64, min@72, default@76, max@80
            // (buffer-absolute; payload starts at buffer byte 8)
            fn w(buf: &mut NV_GPU_CLIENT_POWER_CHANNELS_INFO_V4, abs: usize, v: u32) {
                buf.payload[abs - 8..abs - 4].copy_from_slice(&v.to_le_bytes());
            }
            w(&mut buf, 60, 19);
            w(&mut buf, 64, 13);
            w(&mut buf, 72, 1000);
            w(&mut buf, 76, 63_000);
            w(&mut buf, 80, 125_000);
            assert_eq!(buf.channel_id(0), Some((19, 13)));
            assert_eq!(buf.channel_range(0), Some((1000, 63_000, 125_000)));
            assert_eq!(buf.find_channel(0x7FFF, 19, 13, Some((13, 19))), Some(0));
            // legacy fallback hit when the exact pair is absent
            assert_eq!(buf.find_channel(0x7FFF, 19, 14, Some((13, 19))), None);
            w(&mut buf, 60, 13);
            w(&mut buf, 64, 19);
            assert_eq!(buf.find_channel(0x7FFF, 19, 14, Some((13, 19))), Some(0));
            assert!(buf.payload.len() >= 2672 - 8);
        }

        /// The 0x10A4C control carries TWO candidate geometries (xOCD
        /// compact 40B@28 — type @entry+0, value @entry+4 ⇒ buffer
        /// 32+40*i — vs R465 136B@1756) — both must address distinct,
        /// in-range dwords, and the detector must separate them.
        #[test]
        fn tgp_10a4c_dual_geometry() {
            let mut c = NV_GPU_CLIENT_TGP_WATT_STATUS_10A4C_V1::zeroed();
            // compact entry 2: type @28+80=108 (payload 100), value
            // @32+80=112 (payload 104)
            c.payload[100..104].copy_from_slice(&19u32.to_le_bytes());
            c.payload[104..108].copy_from_slice(&42424u32.to_le_bytes());
            assert_eq!(c.channel_value_compact(2), Some(42424));
            // the type dword is NOT the value: nothing read at 100
            assert_ne!(c.channel_value_compact(2), Some(19));
            // r465 entry 1 → buffer 1756+136+72 = 1964 → payload 1956
            c.payload[1956..1960].copy_from_slice(&777u32.to_le_bytes());
            assert_eq!(c.power_mw(1), Some(777));
            // 15-entry bound = the 0x7FFF mask contract (xOCD writes it)
            assert_eq!(c.channel_value_compact(14), Some(0));

            // detector: compact inside the window, r465 outside ⇒ compact
            let mut d = NV_GPU_CLIENT_TGP_WATT_STATUS_10A4C_V1::zeroed();
            d.payload[24..28].copy_from_slice(&5_000u32.to_le_bytes()); // compact ch0 (buffer 32)
            d.payload[1820..1824].copy_from_slice(&999_999u32.to_le_bytes()); // r465 e0
            assert_eq!(
                d.detect_compact_geometry(&[(0, 1000, 5000, 9000)]),
                Some(true)
            );
            // and the mirror image
            let mut e = NV_GPU_CLIENT_TGP_WATT_STATUS_10A4C_V1::zeroed();
            e.payload[24..28].copy_from_slice(&999_999u32.to_le_bytes());
            e.payload[1820..1824].copy_from_slice(&5_000u32.to_le_bytes());
            assert_eq!(
                e.detect_compact_geometry(&[(0, 1000, 5000, 9000)]),
                Some(false)
            );
            // a gapped info mask keys rows by INDEX: bit 2's window must be
            // compared against compact entry 2 (value buffer 32+80=112 ⇒
            // payload 104), not against positional entry 0.
            let mut f = NV_GPU_CLIENT_TGP_WATT_STATUS_10A4C_V1::zeroed();
            f.payload[24..28].copy_from_slice(&999_999u32.to_le_bytes()); // entry 0
            f.payload[104..108].copy_from_slice(&5_000u32.to_le_bytes()); // entry 2
            assert_eq!(
                f.detect_compact_geometry(&[(2, 1000, 5000, 9000)]),
                Some(true)
            );
        }
    }
}
