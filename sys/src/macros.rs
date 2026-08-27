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
    };
}

macro_rules! nvinherit {
    (
        struct $v2:ident($id:ident: $v1:ty)
    ) => {
        impl ::std::ops::Deref for $v2 {
            type Target = $v1;

            fn deref(&self) -> &Self::Target {
                &self.$id
            }
        }

        impl ::std::ops::DerefMut for $v2 {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.$id
            }
        }
    };
    (
        $v2:ident($id:ident: $v1:ty)
    ) => {
        nvinherit! { struct $v2($id: $v1) }

        impl crate::nvapi::VersionedStruct for $v2 {
            fn nvapi_version_mut(&mut self) -> &mut crate::nvapi::NvVersion {
                self.$id.nvapi_version_mut()
            }

            fn nvapi_version(&self) -> crate::nvapi::NvVersion {
                self.$id.nvapi_version()
            }
        }
    };
}

macro_rules! nvstruct {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $($tt:tt)*
        }
    ) => {
        $(#[$meta])*
        #[repr(C)]
        #[derive(Copy, Clone, Debug)]
        pub struct $name {
            $($tt)*
        }

        unsafe impl zerocopy::AsBytes for $name {
            fn only_derive_is_allowed_to_implement_this_trait() where Self: Sized { }
        }

        unsafe impl zerocopy::FromBytes for $name {
            fn only_derive_is_allowed_to_implement_this_trait() where Self: Sized { }
        }

        nvstruct! { @int fields $name ($($tt)*) }
    };
    (@int fields $name:ident (
            $(#[$meta:meta])*
            pub $id:ident: NvVersion,
            $($tt:tt)*)
        ) => {
        impl crate::nvapi::VersionedStruct for $name {
            fn nvapi_version_mut(&mut self) -> &mut NvVersion {
                &mut self.$id
            }

            fn nvapi_version(&self) -> NvVersion {
                self.$id
            }
        }
    };
    (@int fields $name:ident ($($tt:tt)*)) => { };
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
                    _ => Err(Default::default()),
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
            static CACHE: ::std::sync::atomic::AtomicUsize = ::std::sync::atomic::AtomicUsize::new(0);

            let res = match crate::nvapi::query_interface(crate::nvid::Api::$fn.id(), &CACHE) {
                Ok(ptr) => ::std::mem::transmute::<usize, extern "C" fn($($arg: $arg_ty),*) -> $ret>(ptr)($($arg),*),
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

macro_rules! nvversion {
    (@ $(=$name:ident)? $target:ident($ver:expr) $(= $sz:expr)?) => {
        nvversion! { $(=$name)? $target($ver) $(=$sz)? }

        impl crate::nvapi::StructVersion for $target {
            const NVAPI_VERSION: crate::nvapi::NvVersion = <$target as crate::nvapi::StructVersion<{$ver}>>::NVAPI_VERSION;

            fn versioned() -> Self {
                <$target as crate::nvapi::StructVersion<{$ver}>>::versioned()
            }
        }

        impl Default for $target {
            fn default() -> Self {
                crate::nvapi::StructVersion::<0>::versioned()
            }
        }
    };
    ($(=$name:ident)? $target:ident($ver:expr) $(= $sz:expr)?) => {
        $(
            pub type $name = $target;
        )?

        impl crate::nvapi::StructVersion<$ver> for $target {
            const NVAPI_VERSION: crate::nvapi::NvVersion = NvVersion::with_struct::<$target>($ver);
        }

        $(
            const _: () = assert!($sz == std::mem::size_of::<$target>());
        )?
    };
    ($struct:ident(@.$id:ident)) => {
        impl crate::nvapi::VersionedStruct for $v2 {
            fn nvapi_version_mut(&mut self) -> &mut crate::nvapi::NvVersion {
                &mut self.$id
            }

            fn nvapi_version(&self) -> crate::nvapi::NvVersion {
                self.$id
            }
        }
    };
}
