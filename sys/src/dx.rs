use crate::prelude_::*;
/// Opaque COM `IUnknown` stand-in: the NVAPI D3D entry points only pass these
/// through as opaque pointer targets, so an empty #[repr(C)] type is
/// ABI-equivalent without pulling in windows-sys's COM interface definitions.
#[repr(C)]
pub struct IUnknown {
    _opaque: [u8; 0],
}

nv_declare_handle! { NVDX_ObjectHandle }
pub const NVDX_OBJECT_NONE: NVDX_ObjectHandle = NVDX_ObjectHandle(::std::ptr::null());

nvapi! {
    pub type D3D_GetObjectHandleForResourceFn = extern "C" fn(pDevice: *const IUnknown, pResource: *const IUnknown, pHandle: *mut NVDX_ObjectHandle) -> NvAPI_Status;

    /// This API gets a handle to a resource.
    pub unsafe fn NvAPI_D3D_GetObjectHandleForResource;
}
