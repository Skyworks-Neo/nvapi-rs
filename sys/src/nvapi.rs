use crate::status::{NvAPI_Status, Status};
use crate::types;
use std::mem::size_of;
use std::os::raw::c_void;
use std::sync::atomic::{AtomicPtr, Ordering};

pub use nvapi_macros::{NvInherit, NvStruct, VersionedStructField};

pub type QueryInterfaceFn = extern "C" fn(id: u32) -> *const c_void;

#[cfg(all(windows, target_pointer_width = "32"))]
pub const LIBRARY_NAME: &[u8; 10] = b"nvapi.dll\0";
#[cfg(all(windows, target_pointer_width = "64"))]
pub const LIBRARY_NAME: &[u8; 12] = b"nvapi64.dll\0";
#[cfg(target_os = "linux")]
pub const LIBRARY_NAME: &[u8; 19] = b"libnvidia-api.so.1\0";

pub const FN_NAME: &[u8; 21] = b"nvapi_QueryInterface\0";

static QUERY_INTERFACE_CACHE: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

/// # Safety
///
/// `ptr` must point to a valid NVAPI `QueryInterface` implementation and remain callable for
/// the lifetime of the process.
pub unsafe fn set_query_interface(ptr: QueryInterfaceFn) {
    QUERY_INTERFACE_CACHE.store(ptr as *mut c_void, Ordering::Relaxed);
}

#[cfg(target_os = "macos")]
pub fn nvapi_QueryInterface(id: u32) -> crate::Result<*mut c_void> {
    // TODO: Apparently nvapi is available for macOS?
    Err(Status::LibraryNotFound)
}

// Since v525 NVIDIA drivers have libnvidia-api.so.1 which implements NVAPI but the implementation is still poor
// (many functions are not there, like it's impossible to identify physical handler by pci slot etc)
#[cfg(target_os = "linux")]
pub fn nvapi_QueryInterface(id: u32) -> crate::Result<*mut c_void> {
    use libc::{RTLD_LAZY, RTLD_LOCAL, dlopen, dlsym};
    use std::mem;
    use std::os::raw::c_char;

    unsafe {
        let ptr = match QUERY_INTERFACE_CACHE.load(Ordering::Relaxed) {
            p if p.is_null() => {
                let lib = dlopen(
                    LIBRARY_NAME.as_ptr() as *const c_char,
                    RTLD_LAZY | RTLD_LOCAL,
                );
                if lib.is_null() {
                    Err(Status::LibraryNotFound)
                } else {
                    let ptr = dlsym(lib, FN_NAME.as_ptr() as *const c_char);
                    if ptr.is_null() {
                        Err(Status::LibraryNotFound)
                    } else {
                        QUERY_INTERFACE_CACHE.store(ptr, Ordering::Relaxed);
                        Ok(ptr)
                    }
                }
            }
            ptr => Ok(ptr),
        }?;

        match mem::transmute::<*mut c_void, QueryInterfaceFn>(ptr)(id) as *mut c_void {
            p if p.is_null() => Err(Status::NoImplementation),
            p => Ok(p),
        }
    }
}

#[cfg(windows)]
pub fn nvapi_QueryInterface(id: u32) -> crate::Result<*mut c_void> {
    use std::mem;
    use std::os::raw::c_char;
    use winapi::um::libloaderapi::{GetProcAddress, LoadLibraryA};

    unsafe {
        let ptr = match QUERY_INTERFACE_CACHE.load(Ordering::Relaxed) {
            p if p.is_null() => {
                let lib = LoadLibraryA(LIBRARY_NAME.as_ptr() as *const c_char);
                if lib.is_null() {
                    Err(Status::LibraryNotFound)
                } else {
                    let ptr = GetProcAddress(lib, FN_NAME.as_ptr() as *const c_char);
                    if ptr.is_null() {
                        Err(Status::LibraryNotFound)
                    } else {
                        let ptr = ptr as *mut c_void;
                        QUERY_INTERFACE_CACHE.store(ptr, Ordering::Relaxed);
                        Ok(ptr)
                    }
                }
            }
            ptr => Ok(ptr),
        }?;

        match mem::transmute::<*mut c_void, QueryInterfaceFn>(ptr)(id) as *mut c_void {
            p if p.is_null() => Err(Status::NoImplementation),
            p => Ok(p),
        }
    }
}

pub(crate) fn query_interface(id: u32, cache: &AtomicPtr<c_void>) -> crate::Result<*mut c_void> {
    match cache.load(Ordering::Relaxed) {
        p if p.is_null() => {
            let value = nvapi_QueryInterface(id)?;
            cache.store(value, Ordering::Relaxed);
            Ok(value)
        }
        value => Ok(value),
    }
}

nvapi! {
    pub type InitializeFn = extern "C" fn() -> NvAPI_Status;

    /// This function initializes the NvAPI library (if not already initialized) but always increments the ref-counter.
    /// This must be called before calling other NvAPI_ functions.
    pub unsafe fn NvAPI_Initialize;
}

nvapi! {
    pub type UnloadFn = extern "C" fn() -> NvAPI_Status;

    /// Decrements the ref-counter and when it reaches ZERO, unloads NVAPI library.
    /// This must be called in pairs with NvAPI_Initialize.
    ///
    /// Unloading NvAPI library is not supported when the library is in a resource locked state.
    /// Some functions in the NvAPI library initiates an operation or allocates certain resources
    /// and there are corresponding functions available, to complete the operation or free the
    /// allocated resources. All such function pairs are designed to prevent unloading NvAPI library.
    ///
    /// For example, if NvAPI_Unload is called after NvAPI_XXX which locks a resource, it fails with
    /// NVAPI_ERROR. Developers need to call the corresponding NvAPI_YYY to unlock the resources,
    /// before calling NvAPI_Unload again.
    ///
    /// Note: By design, it is not mandatory to call NvAPI_Initialize before calling any NvAPI.
    /// When any NvAPI is called without first calling NvAPI_Initialize, the internal refcounter
    /// will be implicitly incremented. In such cases, calling NvAPI_Initialize from a different thread will
    /// result in incrementing the refcount again and the user has to call NvAPI_Unload twice to
    /// unload the library. However, note that the implicit increment of the refcounter happens only once.
    /// If the client wants unload functionality, it is recommended to always call NvAPI_Initialize and NvAPI_Unload in pairs.
    pub unsafe fn NvAPI_Unload;
}

nvapi! {
    pub type GetErrorMessageFn = extern "C" fn(nr: NvAPI_Status, szDesc: *mut types::NvAPI_ShortString) -> NvAPI_Status;

    /// This function converts an NvAPI error code into a null terminated string.
    pub unsafe fn NvAPI_GetErrorMessage;
}

nvapi! {
    pub type GetInterfaceVersionStringFn = extern "C" fn(szDesc: *mut types::NvAPI_ShortString) -> NvAPI_Status;

    /// This function returns a string describing the version of the NvAPI library.
    /// The contents of the string are human readable.  Do not assume a fixed format.
    pub unsafe fn NvAPI_GetInterfaceVersionString;
}

/// NvAPI Version Definition
#[derive(
    Default,
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    zerocopy::FromBytes,
    zerocopy::AsBytes,
)]
#[repr(transparent)]
pub struct NvVersion {
    pub data: u32,
}

impl NvVersion {
    pub const fn with_version(data: u32) -> Self {
        Self { data }
    }

    pub const fn new(size: usize, version: u16) -> Self {
        // NVAPI packs size into the low 16 bits of the version dword. For
        // structs >64 KiB (the RE'd private mega-structs: pstates private V4,
        // clk-vf-points V3, power-policies V1) the size bits are TRUNCATED —
        // this is fine in practice because the driver builds its expected
        // dword from its own struct the same way, so the lossy encoding
        // compares equal on both sides. Layout drift is pinned separately by
        // the `= size` literals in nvversion! and the layout tests.
        Self {
            data: size as u32 | (version as u32) << 16,
        }
    }

    #[doc(alias = "MAKE_NVAPI_VERSION")]
    pub const fn with_struct<T>(version: u16) -> Self {
        Self::new(size_of::<T>(), version)
    }

    #[doc(alias = "GET_NVAPI_VERSION")]
    pub const fn version(&self) -> u16 {
        (self.data >> 16) as u16
    }

    #[doc(alias = "GET_NVAPI_SIZE")]
    pub const fn size(&self) -> usize {
        self.data as usize & 0xffff
    }
}

impl From<u32> for NvVersion {
    fn from(version: u32) -> Self {
        Self::with_version(version)
    }
}

impl From<NvVersion> for u32 {
    fn from(ver: NvVersion) -> u32 {
        ver.data
    }
}

/// Field-level version access: implemented for structs that carry an
/// [`NvVersion`] field (directly or via delegation through an inherited
/// sub-struct). Generated by the `VersionedStructField` derive.
pub trait VersionedStructField {
    fn nvapi_version_ref(&self) -> &NvVersion;
    fn nvapi_version_mut(&mut self) -> &mut NvVersion;

    fn nvapi_version_init<const VER: u16>(&mut self)
    where
        Self: StructVersion<VER>,
    {
        *self.nvapi_version_mut() = Self::NVAPI_VERSION;
    }

    fn new_versioned<const VER: u16>() -> Self
    where
        Self: Sized + zerocopy::FromBytes + StructVersion<VER>,
    {
        let mut zero = <Self as zerocopy::FromBytes>::new_zeroed();
        zero.nvapi_version_init::<VER>();
        zero
    }
}

impl VersionedStructField for NvVersion {
    fn nvapi_version_ref(&self) -> &NvVersion {
        self
    }

    fn nvapi_version_mut(&mut self) -> &mut NvVersion {
        self
    }
}

/// Read-only version access, blanket-implemented for every
/// [`VersionedStructField`].
pub trait VersionedStruct {
    fn nvapi_version(&self) -> NvVersion;
}

impl<T: VersionedStructField> VersionedStruct for T {
    fn nvapi_version(&self) -> NvVersion {
        *self.nvapi_version_ref()
    }
}

pub trait StructVersion<const VER: u16 = 0>: VersionedStruct {
    const NVAPI_VERSION: NvVersion;

    fn versioned() -> Self
    where
        Self: Sized + zerocopy::FromBytes + VersionedStructField,
    {
        VersionedStructField::new_versioned::<VER>()
    }
}
