//! GPU illumination (LED) control — public since driver R300.05, surfaced
//! by MinerLamp's `setLED`. The three calls all take a single parm struct
//! (the physical GPU handle travels INSIDE the struct, not as an argument)
//! and share the same 24-byte shape
//! `{version u32, pad u32, hPhysicalGpu handle u64, attribute u32, value u32}`
//! (C layout: the handle forces 8-byte alignment, hence the pad).
//!
//! `Value` for the brightness attributes is 0..=100 (percent); per the SDK
//! note only ONE GPU per illuminated element manages the attribute even in
//! SLI — enumerate with QueryIlluminationSupport to find the owner.
//!
//! Not live-tested yet (no illuminated-GPU hardware on the bench); layout
//! cross-checked against the R380-era SDK headers shipped with MinerLamp.

use crate::prelude_::*;

nvenum! {
    /// Used in the illumination parm structs
    pub enum NV_GPU_ILLUMINATION_ATTRIB / IlluminationAttrib {
        /// Brightness of the GPU logo LED
        NV_GPU_IA_LOGO_BRIGHTNESS / LogoBrightness = 0,
        /// Brightness of the SLI bridge LED
        NV_GPU_IA_SLI_BRIGHTNESS / SliBrightness = 1,
    }
}

nvenum_display! {
    IlluminationAttrib => {
        LogoBrightness = "Logo",
        SliBrightness = "SLI Bridge",
        _ = _,
    }
}

nvstruct! {
    /// NvAPI_GPU_QueryIlluminationSupport parm (OUT: bSupported)
    pub struct NV_GPU_QUERY_ILLUMINATION_SUPPORT_PARM {
        pub version: NvVersion,
        /// C alignment pad (handle at +8)
        pub padding: Padding<[u32; 1]>,
        /// IN: the GPU being asked about the attribute
        pub hPhysicalGpu: NvPhysicalGpuHandle,
        /// IN: attribute to query
        pub attribute: NV_GPU_ILLUMINATION_ATTRIB,
        /// OUT: 1 if this GPU manages the attribute on the element
        pub bSupported: u32,
    }
}

nvversion! { NV_GPU_QUERY_ILLUMINATION_SUPPORT_PARM(1) }

nvstruct! {
    /// NvAPI_GPU_GetIllumination parm (OUT: value)
    pub struct NV_GPU_GET_ILLUMINATION_PARM {
        pub version: NvVersion,
        pub padding: Padding<[u32; 1]>,
        pub hPhysicalGpu: NvPhysicalGpuHandle,
        pub attribute: NV_GPU_ILLUMINATION_ATTRIB,
        /// OUT: current value of the attribute (0..=100 for brightness)
        pub value: u32,
    }
}

nvversion! { NV_GPU_GET_ILLUMINATION_PARM(1) }

nvstruct! {
    /// NvAPI_GPU_SetIllumination parm (IN: value)
    pub struct NV_GPU_SET_ILLUMINATION_PARM {
        pub version: NvVersion,
        pub padding: Padding<[u32; 1]>,
        pub hPhysicalGpu: NvPhysicalGpuHandle,
        pub attribute: NV_GPU_ILLUMINATION_ATTRIB,
        /// IN: new value for the attribute (0..=100 for brightness)
        pub value: u32,
    }
}

nvversion! { NV_GPU_SET_ILLUMINATION_PARM(1) }

nvapi! {
    /// Reports whether the specified illumination attribute is supported
    /// and managed by this GPU. Windows Vista+.
    pub unsafe fn NvAPI_GPU_QueryIlluminationSupport(pIlluminationSupportInfo: *mut NV_GPU_QUERY_ILLUMINATION_SUPPORT_PARM) -> NvAPI_Status;
}

nvapi! {
    /// Retrieves the current value of the specified illumination attribute.
    pub unsafe fn NvAPI_GPU_GetIllumination(pIlluminationInfo: *mut NV_GPU_GET_ILLUMINATION_PARM) -> NvAPI_Status;
}

nvapi! {
    /// Sets the value of the specified illumination attribute.
    pub unsafe fn NvAPI_GPU_SetIllumination(pIlluminationInfo: *mut NV_GPU_SET_ILLUMINATION_PARM) -> NvAPI_Status;
}
