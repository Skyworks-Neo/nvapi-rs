use crate::prelude_::*;

pub const NVAPI_MAX_SIZEOF_I2C_DATA_BUFFER: usize = 4096;
pub const NVAPI_MAX_SIZEOF_I2C_REG_ADDRESS: usize = 4;
pub const NVAPI_DISPLAY_DEVICE_MASK_MAX: usize = 24;
pub const NVAPI_I2C_SPEED_DEPRECATED: u32 = 0xffff;

nvenum! {
    pub enum NV_I2C_SPEED / I2cSpeed {
        NVAPI_I2C_SPEED_DEFAULT / Default = 0,
        NVAPI_I2C_SPEED_3KHZ / _3Khz = 1,
        NVAPI_I2C_SPEED_10KHZ / _10Khz = 2,
        NVAPI_I2C_SPEED_33KHZ / _33Khz = 3,
        NVAPI_I2C_SPEED_100KHZ / _100Khz = 4,
        NVAPI_I2C_SPEED_200KHZ / _200Khz = 5,
        NVAPI_I2C_SPEED_400KHZ / _400Khz = 6,
    }
}

nvenum_display! {
    I2cSpeed => {
        _3Khz = "3 kHz",
        _10Khz = "10 kHz",
        _33Khz = "33 kHz",
        _100Khz = "100 kHz",
        _200Khz = "200 kHz",
        _400Khz = "400 kHz",
        _ = _,
    }
}

nvstruct! {
    /// Used in NvAPI_I2CRead() and NvAPI_I2CWrite()
    #[nv_unchecked]
    pub struct NV_I2C_INFO_V1 {
        /// The structure version.
        pub version: NvVersion,
        /// The Display Mask of the concerned display.
        pub displayMask: u32,
        /// This flag indicates either the DDC port (TRUE) or the communication port
        /// (FALSE) of the concerned display.
        pub bIsDDCPort: u8,
        /// The address of the I2C slave.  The address should be shifted left by one.  Fo
        /// example, the I2C address 0x50, often used for reading EDIDs, would be stored
        /// here as 0xA0.  This matches the position within the byte sent by the master,
        /// the last bit is reserved to specify the read or write direction.
        pub i2cDevAddress: u8,
        #[nv_align(16, usize)]
        pub padding0: [u8; ALIGN],
        /// The I2C target register address.  May be NULL, which indicates no register
        /// address should be sent.
        pub pbI2cRegAddress: *const u8,
        /// The size in bytes of target register address.  If pbI2cRegAddress is NULL, this
        /// field must be 0.
        pub regAddrSize: u32,
        #[nv_align(32, usize)]
        pub padding1: [u8; ALIGN],
        /// The buffer of data which is to be read or written (depending on the command).
        pub pbData: *mut u8,
        /// The size of the data buffer, pbData, to be read or written.
        pub cbSize: u32,
        /// The target speed of the transaction (between 28Kbps to 40Kbps; not guaranteed).
        ///
        /// Deprecated in V2+. Must be set to `NVAPI_I2C_SPEED_DEPRECATED`.
        pub i2cSpeed: u32,
    }
}

nvstruct! {
    /// Used in NvAPI_I2CRead() and NvAPI_I2CWrite()
    #[nv_unchecked]
    pub struct NV_I2C_INFO_V2 {
        /*
        /// Must set `v1.i2cSpeed = NVAPI_I2C_SPEED_DEPRECATED`.
        pub v1: NV_I2C_INFO_V1,
        */
        /// The structure version.
        pub version: NvVersion,
        /// The Display Mask of the concerned display.
        pub displayMask: u32,
        /// This flag indicates either the DDC port (TRUE) or the communication port
        /// (FALSE) of the concerned display.
        pub bIsDDCPort: u8,
        /// The address of the I2C slave.  The address should be shifted left by one.  Fo
        /// example, the I2C address 0x50, often used for reading EDIDs, would be stored
        /// here as 0xA0.  This matches the position within the byte sent by the master,
        /// the last bit is reserved to specify the read or write direction.
        pub i2cDevAddress: u8,
        #[nv_align(16, usize)]
        pub padding0: [u8; ALIGN],
        /// The I2C target register address.  May be NULL, which indicates no register
        /// address should be sent.
        pub pbI2cRegAddress: *const u8,
        /// The size in bytes of target register address.  If pbI2cRegAddress is NULL, this
        /// field must be 0.
        pub regAddrSize: u32,
        #[nv_align(32, usize)]
        pub padding1: [u8; ALIGN],
        /// The buffer of data which is to be read or written (depending on the command).
        pub pbData: *mut u8,
        /// The size of the data buffer, pbData, to be read or written.
        pub cbSize: u32,
        /// Deprecated - must be set to `NVAPI_I2C_SPEED_DEPRECATED`.
        pub i2cSpeed: u32,
        /// The target speed of the transaction in (kHz) (Chosen from the enum `NV_I2C_SPEED`).
        pub i2cSpeedKhz: NV_I2C_SPEED,
        #[nv_align(32, usize)]
        pub padding2: [u8; ALIGN],
    }
}

nvstruct! {
    /// Used in NvAPI_I2CRead() and NvAPI_I2CWrite()
    #[nv_unchecked]
    pub struct NV_I2C_INFO_V3 {
        //pub v2: NV_I2C_INFO_V2,
        /// The structure version.
        pub version: NvVersion,
        /// The Display Mask of the concerned display.
        pub displayMask: u32,
        /// This flag indicates either the DDC port (TRUE) or the communication port
        /// (FALSE) of the concerned display.
        pub bIsDDCPort: u8,
        /// The address of the I2C slave.  The address should be shifted left by one.  Fo
        /// example, the I2C address 0x50, often used for reading EDIDs, would be stored
        /// here as 0xA0.  This matches the position within the byte sent by the master,
        /// the last bit is reserved to specify the read or write direction.
        pub i2cDevAddress: u8,
        #[nv_align(16, usize)]
        pub padding0: [u8; ALIGN],
        /// The I2C target register address.  May be NULL, which indicates no register
        /// address should be sent.
        pub pbI2cRegAddress: *const u8,
        /// The size in bytes of target register address.  If pbI2cRegAddress is NULL, this
        /// field must be 0.
        pub regAddrSize: u32,
        #[nv_align(32, usize)]
        pub padding1: [u8; ALIGN],
        /// The buffer of data which is to be read or written (depending on the command).
        pub pbData: *mut u8,
        /// The size of the data buffer, pbData, to be read or written.
        pub cbSize: u32,
        /// Deprecated - must be set to `NVAPI_I2C_SPEED_DEPRECATED`.
        pub i2cSpeed: u32,
        /// The target speed of the transaction in (kHz) (Chosen from the enum `NV_I2C_SPEED`).
        pub i2cSpeedKhz: NV_I2C_SPEED,
        /// The portid on which device is connected (remember to set bIsPortIdSet if this value is set)
        ///
        /// Optional for pre-Kepler
        pub portId: u8,
        #[nv_align(8, u32)]
        pub padding2: [u8; ALIGN],
        /// set this flag on if and only if portid value is set
        pub bIsPortIdSet: u32,
        #[nv_align(32, usize)]
        pub padding3: [u8; ALIGN],
    }
}

nvversion! { NV_I2C_INFO_V1(1) }
nvversion! { NV_I2C_INFO_V2(2) }
nvversion! { @=NV_I2C_INFO NV_I2C_INFO_V3(3) }

nvapi! {
    pub type NvAPI_I2CReadFn = extern "C" fn(hPhysicalGpu: NvPhysicalGpuHandle, pI2cInfo: *mut NV_I2C_INFO) -> NvAPI_Status;

    /// This function reads the data buffer from the I2C port.
    /// The I2C request must be for a DDC port: pI2cInfo->bIsDDCPort = 1.
    ///
    /// A data buffer size larger than 16 bytes may be rejected if a register address is specified.  In such a case,
    /// NVAPI_ARGUMENT_EXCEED_MAX_SIZE would be returned.
    ///
    /// If a register address is specified (i.e. regAddrSize is positive), then the transaction will be performed in
    /// the combined format described in the I2C specification.  The register address will be written, followed by
    /// reading into the data buffer.
    ///
    /// # Returns
    ///
    /// - `NVAPI_OK`: Completed request
    /// - `NVAPI_ERROR`: Miscellaneous error occurred.
    /// - `NVAPI_HANDLE_INVALIDATED`: Handle passed has been invalidated (see user guide).
    /// - `NVAPI_EXPECTED_PHYSICAL_GPU_HANDLE`: Handle passed is not a physical GPU handle.
    /// - `NVAPI_INCOMPATIBLE_STRUCT_VERSION`: Structure version is not supported.
    /// - `NVAPI_INVALID_ARGUMENT`: argument does not meet specified requirements
    /// - `NVAPI_ARGUMENT_EXCEED_MAX_SIZE`: an argument exceeds the maximum
    pub unsafe fn NvAPI_I2CRead;
}

nvapi! {
    pub type NvAPI_I2CWriteFn = extern "C" fn(hPhysicalGpu: NvPhysicalGpuHandle, pI2cInfo: *mut NV_I2C_INFO) -> NvAPI_Status;

    /// This function writes the data buffer to the I2C port.
    ///
    /// The I2C request must be for a DDC port: pI2cInfo->bIsDDCPort = 1.
    ///
    /// A data buffer size larger than 16 bytes may be rejected if a register address is specified.  In such a case,
    /// NVAPI_ARGUMENT_EXCEED_MAX_SIZE would be returned.
    ///
    /// If a register address is specified (i.e. regAddrSize is positive), then the register address will be written
    /// and the data buffer will immediately follow without a restart.
    ///
    /// # Returns
    ///
    /// - `NVAPI_OK`: Completed request
    /// - `NVAPI_ERROR`: Miscellaneous error occurred.
    /// - `NVAPI_HANDLE_INVALIDATED`: Handle passed has been invalidated (see user guide).
    /// - `NVAPI_EXPECTED_PHYSICAL_GPU_HANDLE`: Handle passed is not a physical GPU handle.
    /// - `NVAPI_INCOMPATIBLE_STRUCT_VERSION`: Structure version is not supported.
    /// - `NVAPI_INVALID_ARGUMENT`: Argument does not meet specified requirements
    /// - `NVAPI_ARGUMENT_EXCEED_MAX_SIZE`: exceeds the maximum
    pub unsafe fn NvAPI_I2CWrite;
}

/// Undocumented API
pub mod undocumented {
    use super::NV_I2C_SPEED;
    use crate::prelude_::*;

    nvstruct! {
        /// Used in NvAPI_I2CRead() and NvAPI_I2CWrite()
        #[nv_unchecked]
        pub struct NV_I2C_INFO_EX_V3 {
            /// The structure version.
            pub version: NvVersion,
            /// The Display Mask of the concerned display.
            pub displayMask: u32,
            /// This flag indicates either the DDC port (TRUE) or the communication port
            /// (FALSE) of the concerned display.
            pub bIsDDCPort: u8,
            /// The address of the I2C slave.  The address should be shifted left by one.  Fo
            /// example, the I2C address 0x50, often used for reading EDIDs, would be stored
            /// here as 0xA0.  This matches the position within the byte sent by the master,
            /// the last bit is reserved to specify the read or write direction.
            pub i2cDevAddress: u8,
            #[nv_align(16, usize)]
            pub padding0: [u8; ALIGN],
            /// The I2C target register address.  May be NULL, which indicates no register
            /// address should be sent.
            pub pbI2cRegAddress: *const u8,
            /// The size in bytes of target register address.  If pbI2cRegAddress is NULL, this
            /// field must be 0.
            pub regAddrSize: u32,
            #[nv_align(32, usize)]
            pub padding1: [u8; ALIGN],
            /// The buffer of data which is to be read or written (depending on the command).
            pub pbData: *mut u8,
            /// bytes to read ??? seems required on write too
            pub pbRead: u32,
            /// The size of the data buffer, pbData, to be read or written.
            pub cbSize: u32,
            /// The target speed of the transaction in (kHz) (Chosen from the enum `NV_I2C_SPEED`).
            pub i2cSpeedKhz: NV_I2C_SPEED,
            /// The portid on which device is connected (remember to set bIsPortIdSet if this value is set)
            ///
            /// Optional for pre-Kepler
            pub portId: u8,
            #[nv_align(8, u32)]
            pub padding2: [u8; ALIGN],
            /// set this flag on if and only if portid value is set
            pub bIsPortIdSet: u32,
            #[nv_align(32, usize)]
            pub padding3: [u8; ALIGN],
        }
    }

    nvversion! { @=NV_I2C_INFO_EX NV_I2C_INFO_EX_V3(3) }

    nvapi! {
        pub type NvAPI_I2CReadExFn = extern "C" fn(hPhysicalGpu: NvPhysicalGpuHandle, pI2cInfo: *mut NV_I2C_INFO_EX, pData: *mut u32) -> NvAPI_Status;

        /// Undocumented function. `pData` is often `{ 1, 0 }`?
        pub unsafe fn NvAPI_I2CReadEx;
    }

    nvapi! {
        pub type NvAPI_I2CWriteExFn = extern "C" fn(hPhysicalGpu: NvPhysicalGpuHandle, pI2cInfo: *mut NV_I2C_INFO_EX, pData: *mut u32) -> NvAPI_Status;

        /// Undocumented function. `pData` is often `{ 1, 0 }`?
        pub unsafe fn NvAPI_I2CWriteEx;
    }
}

pub mod driver_v1 {
    //! The I2C Ex handlers' PRIVATE 80-byte layout on R610+ (RE'd from
    //! nvapi64 `I2CReadEx` handler: version gate `== 0x10050` else
    //! INCOMPATIBLE_STRUCT_VERSION; accessMode@0x24 selects the op —
    //! 6 = N-byte read through one inline register byte, 9 = extended
    //! pointer read/write). NOT the public NV_I2C_INFO layout.

    #[derive(Copy, Clone, Debug, Default)]
    #[repr(C)]
    pub struct NV_I2C_INFO_DRIVER_V1 {
        /// Must stamp exactly `0x10050` (size 0x50 | version 1).
        pub version: u32,
        /// Display mask; only bare bits 0/2/4 survive validation (bit 0 is
        /// auto-cleared) — `0xFFFF` fails INVALID_ARGUMENT.
        pub display_mask: u32,
        /// +0x08: low byte = DDC flag, second byte = device address << 1.
        /// Read back as a word when `b_is_port_id_set == 1`.
        pub ddc_and_dev_addr: u16,
        pub _pad0: u16,
        pub _pad1: u32,
        /// +0x10: unused by access modes 0-6 (mode 9 uses +0x38).
        pub _reserved0: u64,
        /// +0x18: access port; `0xFF` = driver auto-probe (RM 0x7000045).
        pub port_id: u8,
        pub _pad2: [u8; 3],
        /// +0x1C: 0 = DDC/mask routing, 1 = use `ddc_and_dev_addr` word.
        pub b_is_port_id_set: u32,
        /// +0x20: speed enum 0-6 (raw > 40000 rejected).
        pub i2c_speed_khz: u32,
        /// +0x24: access mode — 0 none, 1 read 1B, 2 read N, 4 read 1B via
        /// inline reg, 5 read 2B via inline reg, 6 read N via inline reg,
        /// 9 extended pointers.
        pub access_mode: u32,
        /// +0x28: flag byte consumed by the low-level transaction.
        pub flag: u8,
        /// +0x29: inline register bytes (modes 4/5/6, count fixed 1).
        pub inline_reg: [u8; 3],
        /// +0x2C: data length (modes 2/6; mode 9 reuses 0x30 as reg length).
        pub length: u32,
        /// +0x30..0x50: mode-dependent tail (simple data pointer vs mode-9
        /// pointer set — the slots are mode-exclusive).
        pub tail: I2cDriverTail,
    }

    #[derive(Copy, Clone)]
    #[repr(C)]
    pub union I2cDriverTail {
        /// Modes 1/2/4/5/6: data pointer at +0x30.
        pub simple: I2cDriverTailSimple,
        /// Mode 9: reg len @+0x30, reg ptr @+0x38, data len @+0x40,
        /// data ptr @+0x48.
        pub ext: I2cDriverTailExt,
    }

    #[derive(Copy, Clone, Debug, Default)]
    #[repr(C)]
    pub struct I2cDriverTailSimple {
        pub data_ptr: *mut u8,
        pub _pad: [u32; 3],
    }

    #[derive(Copy, Clone, Debug, Default)]
    #[repr(C)]
    pub struct I2cDriverTailExt {
        pub reg_len: u32,
        pub _pad: u32,
        pub reg_ptr: *const u8,
        pub data_len: u32,
        pub _pad2: u32,
        pub data_ptr: *mut u8,
    }

    impl std::fmt::Debug for I2cDriverTail {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("I2cDriverTail(..)")
        }
    }

    impl Default for I2cDriverTail {
        fn default() -> Self {
            Self {
                simple: I2cDriverTailSimple::default(),
            }
        }
    }

    /// The exact stamp the driver validates: `size 0x50 | version 1`.
    pub const NV_I2C_INFO_DRIVER_V1_STAMP: u32 = 0x10050;

    const _: () = assert!(size_of::<NV_I2C_INFO_DRIVER_V1>() == 0x50);
}
