use crate::status::{NvAPI_Status, Status};
use crate::types;
use std::fmt;
use std::mem::size_of;
use std::os::raw::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicPtr, Ordering};

pub use nvapi_macros::{NvInherit, NvStruct, VersionedStructField};

pub type QueryInterfaceFn = extern "C" fn(id: u32) -> *const c_void;

#[cfg(all(windows, target_pointer_width = "32"))]
pub const LIBRARY_NAME: &[u8; 10] = b"nvapi.dll\0";
// WOA(Windows on ARM64): 616+ WOA driver installs a dedicated native-ARM64
// shim `nvapia64.dll` alongside the ARM64X `nvapi64.dll` in System32
// (nv_surface_woa.inf [nv_system32_copyfiles]). Prefer it on aarch64 targets
// so we skip ARM64X dual-view resolution entirely; x64 targets keep loading
// `nvapi64.dll`, whose x64 base view is exactly what an x64 process gets.
#[cfg(all(windows, target_arch = "aarch64", target_pointer_width = "64"))]
pub const LIBRARY_NAME: &[u8; 13] = b"nvapia64.dll\0";
#[cfg(all(windows, target_pointer_width = "64", not(target_arch = "aarch64")))]
pub const LIBRARY_NAME: &[u8; 12] = b"nvapi64.dll\0";
#[cfg(target_os = "linux")]
pub const LIBRARY_NAME: &[u8; 19] = b"libnvidia-api.so.1\0";

pub const FN_NAME: &[u8; 21] = b"nvapi_QueryInterface\0";

static QUERY_INTERFACE_CACHE: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

/// OS-level detail for the most recent dynamic-library load failure in this
/// process. The NVAPI ABI can only carry `NVAPI_LIBRARY_NOT_FOUND`, which
/// folds the real reason away (missing DLL vs wrong machine vs missing
/// symbol); this side channel preserves it so consumers surface the true
/// error instead of a bare "library not found".
#[derive(Debug, Clone)]
pub struct LoadError {
    /// Failing call with its argument, e.g. `LoadLibraryA("nvapi64.dll")`.
    pub call: String,
    /// Raw OS status code, when the platform has one (Windows `GetLastError`).
    pub os_code: Option<u32>,
    /// Human-readable OS message (`io::Error` display / `dlerror` string).
    pub message: String,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // On Windows `message` already carries "os error N" via io::Error's
        // display; dlerror on Linux has no numeric code to add.
        write!(f, "{} failed: {}", self.call, self.message)
    }
}

static LAST_LOAD_ERROR: Mutex<Option<LoadError>> = Mutex::new(None);

pub(crate) fn record_load_error(call: String, os_code: Option<u32>, message: String) {
    *LAST_LOAD_ERROR.lock().unwrap() = Some(LoadError {
        call,
        os_code,
        message,
    });
}

/// The most recent library-load failure in this process, if any. Global (not
/// thread-local): loads are one-shot and callers only read this on the
/// failure path, so a racing reader seeing the previous attempt's error is
/// acceptable for diagnostics.
pub fn last_load_error() -> Option<LoadError> {
    LAST_LOAD_ERROR.lock().unwrap().clone()
}

/// `LIBRARY_NAME` without its NUL terminator, for diagnostics strings.
#[cfg(not(target_os = "macos"))]
fn library_name_str() -> &'static str {
    std::str::from_utf8(&LIBRARY_NAME[..LIBRARY_NAME.len() - 1]).unwrap_or("<nvapi library>")
}

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
    record_load_error(
        "nvapi loader".to_string(),
        None,
        "no NVAPI loader is implemented on this platform".to_string(),
    );
    Err(Status::LibraryNotFound)
}

// Since v525 NVIDIA drivers have libnvidia-api.so.1 which implements NVAPI but the implementation is still poor
// (many functions are not there, like it's impossible to identify physical handler by pci slot etc)
#[cfg(target_os = "linux")]
pub fn nvapi_QueryInterface(id: u32) -> crate::Result<*mut c_void> {
    use libc::{RTLD_LAZY, RTLD_LOCAL, dlerror, dlopen, dlsym};
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
                    // dlerror is thread-local and cleared by the next call;
                    // read it immediately after the failed dlopen
                    let detail = match dlerror() {
                        p if p.is_null() => "unknown dlopen failure".to_string(),
                        msg => std::ffi::CStr::from_ptr(msg).to_string_lossy().into_owned(),
                    };
                    record_load_error(format!("dlopen({:?})", library_name_str()), None, detail);
                    Err(Status::LibraryNotFound)
                } else {
                    let ptr = dlsym(lib, FN_NAME.as_ptr() as *const c_char);
                    if ptr.is_null() {
                        let detail = match dlerror() {
                            p if p.is_null() => "symbol not found".to_string(),
                            msg => std::ffi::CStr::from_ptr(msg).to_string_lossy().into_owned(),
                        };
                        record_load_error(format!("dlsym({:?})", library_name_str()), None, detail);
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
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};

    unsafe {
        let ptr = match QUERY_INTERFACE_CACHE.load(Ordering::Relaxed) {
            p if p.is_null() => {
                let lib = LoadLibraryA(LIBRARY_NAME.as_ptr());
                if lib.is_null() {
                    // GetLastError must be read before any other API call —
                    // this is the only place the real reason survives (193
                    // = bad machine, 126 = not found, 5 = access denied, …)
                    let code = GetLastError();
                    let message = std::io::Error::from_raw_os_error(code as i32).to_string();
                    record_load_error(
                        format!("LoadLibraryA({:?})", library_name_str()),
                        Some(code),
                        message,
                    );
                    Err(Status::LibraryNotFound)
                } else {
                    // FARPROC is Option<fn>; a missing symbol is None, and a
                    // present one transmutes to an opaque pointer for caching
                    let proc_ = GetProcAddress(lib, FN_NAME.as_ptr());
                    let ptr = match proc_ {
                        Some(f) => f as *mut c_void,
                        None => std::ptr::null_mut(),
                    };
                    if ptr.is_null() {
                        let code = GetLastError();
                        let message = std::io::Error::from_raw_os_error(code as i32).to_string();
                        record_load_error(
                            format!(
                                "GetProcAddress({:?}, {:?})",
                                library_name_str(),
                                "nvapi_QueryInterface"
                            ),
                            Some(code),
                            message,
                        );
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
    zerocopy::IntoBytes,
    zerocopy::Immutable,
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
        let mut zero = <Self as zerocopy::FromZeros>::new_zeroed();
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

/// Version-dword encoding pins (audit #17): macro constants and NvVersion
/// methods must agree on the same bit layout, live-RE magics must not drift,
/// and the >64 KiB truncation semantics stay documented (see NvVersion::new).
#[cfg(test)]
mod nvversion_tests {
    use super::NvVersion;
    use crate::types::{GET_NVAPI_SIZE, GET_NVAPI_VERSION};

    #[test]
    fn cascade_roundtrip() {
        for &(size, version) in &[(0usize, 0u16), (0xFFFF, 0xFFFF), (456, 3), (7416, 3)] {
            let v = NvVersion::new(size, version);
            assert_eq!(v.version(), version);
            assert_eq!(v.size(), size & 0xffff);
            assert_eq!(GET_NVAPI_VERSION(v.data), version);
            assert_eq!(GET_NVAPI_SIZE(v.data), size & 0xffff);
        }
    }

    #[test]
    fn pinned_live_magics() {
        // GetPstates20 cascade head (live-verified R610.74, see pstates20_size_tests)
        use crate::api::NV_GPU_PERF_PSTATES20_INFO_V2;
        assert_eq!(
            NvVersion::with_struct::<NV_GPU_PERF_PSTATES20_INFO_V2>(3).data,
            0x31CF8
        );
    }

    #[test]
    fn size_truncation_documented() {
        // >64 KiB structs truncate the size field by design: the driver builds
        // the same lossy dword from its own struct, so both sides compare
        // equal. Pin it so a change here is a conscious one.
        assert_eq!(NvVersion::new(0x10000, 1).size(), 0);
        assert_eq!(NvVersion::new(0x10000, 1).data, 0x10000);
    }
}
