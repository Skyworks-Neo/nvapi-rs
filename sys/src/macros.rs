macro_rules! nv_declare_handle {
    (
        $(#[$meta:meta])*
        $name:ident
    ) => {
        $(#[$meta])*
        #[derive(Copy, Clone, Debug)]
        #[repr(transparent)]
        pub struct $name(*const ::std::os::raw::c_void);

        impl $name {
            pub fn as_ptr(&self) -> *const ::std::os::raw::c_void {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                $name(::std::ptr::null())
            }
        }

        unsafe impl zerocopy::AsBytes for $name {
            fn only_derive_is_allowed_to_implement_this_trait() where Self: Sized { }
        }

        unsafe impl zerocopy::FromBytes for $name {
            fn only_derive_is_allowed_to_implement_this_trait() where Self: Sized { }
        }
    };
}

macro_rules! nvstruct {
    ($($tt:tt)*) => {
        #[crate::nvapi::NvStruct]
        $($tt)*
    };
}

macro_rules! nvenum_legacy {
    (
        $(#[$meta:meta])*
        pub enum $enum:ident / $enum_name:ident {
            $(
                $(#[$metai:meta])*
                $symbol:ident / $name:ident = $value:expr,
            )*
        }
    ) => {
        $(#[$meta])*
        pub type $enum = ::std::os::raw::c_int;
        $(
            $(#[$metai])*
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            #[allow(overflowing_literals)]
            pub const $symbol: $enum = $value as _;
        )*

        $(#[$meta])*
        #[allow(overflowing_literals)]
        #[allow(clippy::unsafe_derive_deserialize)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        #[repr(i32)]
        pub enum $enum_name {
            $(
                $(#[$metai])*
                $name = $symbol as _,
            )*
        }

        impl $enum_name {
            /// Convert a raw NVAPI enum value into a typed variant.
            ///
            /// # Errors
            ///
            /// Returns [`crate::ArgumentRangeError`] when `raw` does not match a known value.
            #[allow(overflowing_literals)]
            pub fn from_raw(raw: $enum) -> ::std::result::Result<Self, crate::ArgumentRangeError> {
                match raw {
                    $(
                        $symbol
                    )|* => Ok(unsafe { ::std::mem::transmute::<$enum, $enum_name>(raw) }),
                    _ => Err(crate::ArgumentRangeError::new(raw as _)),
                }
            }

            pub fn raw(&self) -> $enum {
                *self as _
            }

            pub fn values() -> impl Iterator<Item=Self> {
                [
                    $(
                        $enum_name::$name
                    ),*
                ].into_iter()
            }
        }

        impl From<$enum_name> for $enum {
            fn from(value: $enum_name) -> $enum {
                value as _
            }
        }

        impl TryFrom<$enum> for $enum_name {
            type Error = crate::ArgumentRangeError;

            fn try_from(raw: $enum) -> ::std::result::Result<Self, crate::ArgumentRangeError> {
                Self::from_raw(raw)
            }
        }
    };
}

macro_rules! nvenum {
    ($($tt:tt)*) => { nvapi_macros::nvenum! { $($tt)* } };
}

macro_rules! nvbits {
    ($($tt:tt)*) => { nvapi_macros::nvbits! { $($tt)* } };
}

macro_rules! nvenum_display {
    ($($tt:tt)*) => { nvapi_macros::nvenum_display! { $($tt)* } };
}

macro_rules! nvapi {
    (
        $(#[$meta:meta])*
        pub unsafe fn $fn:ident($($arg:ident: $arg_ty:ty),*) -> $ret:ty;
    ) => {
        $(#[$meta])*
        #[doc = "# Safety\n\nThis function forwards to NVAPI. Callers must ensure all pointers are valid, the target NVAPI entry point is available, and the NVAPI library is initialized as required by the driver."]
        pub unsafe fn $fn($($arg: $arg_ty),*) -> $ret {
            static CACHE: ::std::sync::atomic::AtomicPtr<::std::os::raw::c_void> = ::std::sync::atomic::AtomicPtr::new(::core::ptr::null_mut());

            let res = match crate::nvapi::query_interface(crate::nvid::Api::$fn.id(), &CACHE) {
                Ok(ptr) => ::std::mem::transmute::<*mut ::std::os::raw::c_void, extern "C" fn($($arg: $arg_ty),*) -> $ret>(ptr)($($arg),*),
                Err(e) => e.raw(),
            };
            #[cfg(feature = "log")] {
                log::trace!(target: "nvapi_sys::api", "{:?} = {}", crate::nvid::Api::$fn, crate::status::Status::from_raw(res).map(|s| format!("{s:?}")).unwrap_or_else(|_| format!("raw({res})")));
            }
            res
        }
    };
    (
        pub type $name:ident = extern "C" fn($($arg:ident: $arg_ty:ty),*) -> $ret:ty;

        $(#[$meta:meta])*
        pub unsafe fn $fn:ident;
    ) => {
        pub type $name = extern "C" fn($($arg: $arg_ty),*) -> $ret;

        nvapi! {
            $(#[$meta])*
            pub unsafe fn $fn($($arg: $arg_ty),*) -> $ret;
        }
    };
}

/// Proc-macro shim: accepts the exact v0.2.x `nvversion!` syntax
/// (`@ = Alias Target(ver) = size`) and expands to the same code the old
/// `macro_rules!` arms produced. See `nvapi_macros::nvversion` for the
/// deliberate deviation from the donor's family syntax (Default pins the
/// marked `@` version, not the oldest declared one).
macro_rules! nvversion {
    ($($tt:tt)*) => { nvapi_macros::nvversion! { $($tt)* } };
}
