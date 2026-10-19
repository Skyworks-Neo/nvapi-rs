/// Raw GPU register access (`NvAPI_GPU_RegisterOp`, query id `0x2EB3C140` —
/// see `nvid.rs`).
///
/// A batched read/write window over the card's global register space, driven
/// entirely from user mode through the driver's QueryInterface — no kernel
/// lane involved. The LACT daemon uses the READ half for two measurement
/// surfaces this crate mirrors in [`crate::Gpu`]:
///
/// - per-chip GDDR6x/GDDR7 temperature sensors (`0x9024C0` family), and
/// - the Blackwell aggregated hotspot register (`0xAD0AA0`).
///
/// Writes exist in the flags but are intentionally unexposed here.
pub mod undocumented {
    use crate::prelude_::*;

    // Flag decode proven from the 610.47 nvapi64_impl handler (sub_1801AF1D0):
    // one direction bit (READ=1 | WRITE=2) + one size bit (32BIT=4 | 64BIT=8)
    // is mandatory — every other combination bails with INVALID_ARGUMENT (-5)
    // before the kernel call; then exactly one type bit must be set
    // (GLOBAL=16 → internal type 0, TYPE_20=32 → internal type 1).
    pub const NV_GPU_REGISTER_OP_FLAG_READ: u16 = 1;
    pub const NV_GPU_REGISTER_OP_FLAG_32BIT: u16 = 4;
    pub const NV_GPU_REGISTER_OP_FLAG_64BIT: u16 = 8;
    pub const NV_GPU_REGISTER_OP_FLAG_TYPE_GLOBAL: u16 = 16;
    pub const NV_GPU_REGISTER_OP_FLAG_TYPE_20: u16 = 32;

    // One register operation (24 B, natural `repr(C)` layout: 8 B header,
    // then two u64 slots). `status` is filled per-op by the driver; `value`
    // carries the read result (32-bit reads occupy the low dword).
    nvstruct! {
        pub struct NV_GPU_REGISTER_OP {
            pub flags: u16,
            pub status: u16,
            pub offset: u32,
            pub write_mask: u64,
            pub value: u64,
        }
    }

    // Batch header: up to 256 ops per call. Version = size | 1<<16
    // (0x11808), same stamp LACT writes.
    nvstruct! {
        pub struct NV_GPU_REGISTER_OP_DATA_V1 {
            pub version: NvVersion,
            pub op_count: u32,
            pub op: Array<[NV_GPU_REGISTER_OP; 256]>,
        }
    }

    nvversion! { @=NV_GPU_REGISTER_OP_DATA NV_GPU_REGISTER_OP_DATA_V1(1) = 6152 }

    nvapi! {
        /// Batched raw register access. The id doubles as the driver's
        /// internal architecture-query entry (nvid.rs note); the GLOBAL flag
        /// selects the raw register-decode path LACT drives.
        pub unsafe fn NvAPI_GPU_RegisterOp(hPhysicalGPU: NvPhysicalGpuHandle, pRegisterOp: *mut NV_GPU_REGISTER_OP_DATA_V1) -> NvAPI_Status;
    }
}
