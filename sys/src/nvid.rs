#![allow(non_camel_case_types)]

use std::mem;

macro_rules! nvapis {
    ($(
        $(#[$($meta:meta)*])*
        $name:ident = $id:expr,
    )*) => {
        #[repr(u32)]
        #[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum Api {
        $(
            $(#[$($meta)*])*
            $name = $id,
        )*
        }

        impl Api {
            pub fn from_id(id: u32) -> Result<Self, crate::ArgumentRangeError> {
                match id {
                $(
                    $id
                )|* => Ok(unsafe { mem::transmute::<u32, Api>(id) }),
                    _ => Err(Default::default()),
                }
            }

            pub fn id(&self) -> u32 {
                *self as _
            }
        }
    };
}

nvapis! {

// source: https://stackoverflow.com/a/16497265 (full dump as of May 2013)

NvAPI_Initialize = 0x0150e828,
NvAPI_Unload = 0xd22bdd7e,
NvAPI_GetErrorMessage = 0x6c2d048c,
NvAPI_GetInterfaceVersionString = 0x01053fa5,
// Note: declared in nvapi.h but not present in nvapi_interface.h table.
// Not in the interface table, so no known ID for NvAPI_QueryInterface.
// NvAPI_GetInterfaceVersionStringEx = <unknown>,
//
// NOTE — `nvapi_pepQueryInterface` is NOT an IID and is deliberately NOT wrapped
// here. It is a SEPARATE exported SYMBOL of nvapi.dll (resolved by name via
// GetProcAddress, exactly like nvapi_QueryInterface itself — not by a 32-bit ID
// passed to nvapi_QueryInterface), so it cannot be an `Api` enum variant.
//
// "PEP" = Privileged Execution Path. It is a SECOND QueryInterface entry point
// that routes NVAPI calls through an elevated RM escape for controls that need
// admin privileges. Discovered in MSI Afterburner's RTHAL.dll init stub
// (sub_10001070): when its `a2==1` flag is set it does
//     dword_100DFBE4 = GetProcAddress(hModule, "nvapi_pepQueryInterface");
// and, if present, routes ALL subsequent NVAPI resolution through that pointer
// instead of the normal nvapi_QueryInterface path (dword_100DFBE0).
//
// Why it is NOT wrapped / NOT used by nvapi-rs or nvoc:
//  1. It is NOT needed for GPU detection or any monitoring read. nvapi-rs/nvoc
//     enumerate GPUs fine via the normal nvapi_QueryInterface + an explicit
//     NvAPI_Initialize (see core/src/target.rs). MSI uses the normal path for
//     enumeration too — PEP is only taken for a few privileged RM controls.
//  2. The PEP path routes to \\.\NvAdminDevice and returns
//     NVAPI_INVALID_USER_PRIVILEGE without elevation, so wrapping it would add
//     an admin-privilege failure mode to every consumer for no monitoring gain.
//  3. It is undocumented, absent from NVIDIA's interface table, and
//     MSI-specific in its usage — wrapping it would bind us to a private,
//     unstable entry point.
//  4. The only things MSI reaches via PEP (raw MMIO / privileged limit sets)
//     are exactly the surfaces nvoc has already concluded are unreachable
//     without a kernel driver (see docs/gpuz-per-rail-investigation.md).
// Kept here as a documentation-only record. Do NOT add as an Api variant and
// do NOT route nvapi-rs resolution through it.
NvAPI_GetDisplayDriverVersion = 0xf951a4d1,
NvAPI_SYS_GetDriverAndBranchVersion = 0x2926aaad,
NvAPI_EnumNvidiaDisplayHandle = 0x9abdd40d,
NvAPI_EnumNvidiaUnAttachedDisplayHandle = 0x20de9260,
NvAPI_EnumPhysicalGPUs = 0xe5ac921f,
NvAPI_EnumTCCPhysicalGPUs = 0xd9930b07,
NvAPI_EnumLogicalGPUs = 0x48b3ea59,
NvAPI_GetPhysicalGPUsFromDisplay = 0x34ef9506,
NvAPI_GetPhysicalGPUFromUnAttachedDisplay = 0x5018ed61,
NvAPI_CreateDisplayFromUnAttachedDisplay = 0x63f9799e,
NvAPI_GetLogicalGPUFromDisplay = 0xee1370cf,
NvAPI_GetLogicalGPUFromPhysicalGPU = 0xadd604d1,
NvAPI_GetPhysicalGPUsFromLogicalGPU = 0xaea3fa32,
NvAPI_GetAssociatedNvidiaDisplayHandle = 0x35c29134,
NvAPI_DISP_GetAssociatedUnAttachedNvidiaDisplayHandle = 0xa70503b2,
NvAPI_GetAssociatedNvidiaDisplayName = 0x22a78b05,
NvAPI_GetUnAttachedAssociatedDisplayName = 0x4888d790,
NvAPI_EnableHWCursor = 0x2863148d,
NvAPI_DisableHWCursor = 0xab163097,
NvAPI_GetVBlankCounter = 0x67b5db55,
NvAPI_SetRefreshRateOverride = 0x3092ac32,
NvAPI_GetAssociatedDisplayOutputId = 0xd995937e,
NvAPI_GetDisplayPortInfo = 0xc64ff367,
NvAPI_SetDisplayPort = 0xfa13e65a,
NvAPI_GetHDMISupportInfo = 0x6ae16ec3,
NvAPI_DISP_EnumHDMIStereoModes = 0xd2ccf5d6,
NvAPI_GetInfoFrame = 0x09734f1d,
NvAPI_SetInfoFrame = 0x69c6f365,
NvAPI_SetInfoFrameState = 0x67efd887,
NvAPI_GetInfoFrameState = 0x41511594,
NvAPI_Disp_InfoFrameControl = 0x6067af3f,
NvAPI_Disp_ColorControl = 0x92f9d80d,
NvAPI_Disp_GetHdrCapabilities = 0x84f2a8df,
NvAPI_Disp_HdrColorControl = 0x351da224,
NvAPI_Disp_SetSourceColorSpace = 0x473b6caf,
NvAPI_Disp_GetSourceColorSpace = 0xceedc85b,
NvAPI_Disp_SetSourceHdrMetadata = 0x905eb63b,
NvAPI_Disp_GetSourceHdrMetadata = 0x0d3f52da,
NvAPI_Disp_SetOutputMode = 0x98e7661a,
NvAPI_Disp_GetOutputMode = 0x81fed88d,
NvAPI_Disp_SetHdrToneMapping = 0xdd6da362,
NvAPI_Disp_GetHdrToneMapping = 0xfbd36e71,
NvAPI_DISP_GetVirtualModeData = 0x3230d69a,
NvAPI_DISP_OverrideDisplayModeList = 0x0291bff2,
NvAPI_GetDisplayDriverMemoryInfo = 0x774aa982,
NvAPI_GetDriverMemoryInfo = 0x2dc95125,
NvAPI_GetDVCInfo = 0x4085de45,
NvAPI_SetDVCLevel = 0x172409b4,
NvAPI_GetDVCInfoEx = 0x0e45002d,
NvAPI_SetDVCLevelEx = 0x4a82c2b1,
NvAPI_GetHUEInfo = 0x95b64341,
NvAPI_SetHUEAngle = 0xf5a0f22c,
NvAPI_GetImageSharpeningInfo = 0x9fb063df,
NvAPI_SetImageSharpeningLevel = 0x3fc9a59c,
NvAPI_D3D_GetCurrentSLIState = 0x4b708b54,
NvAPI_D3D9_RegisterResource = 0xa064bdfc,
NvAPI_D3D9_UnregisterResource = 0xbb2b17aa,
NvAPI_D3D9_AliasSurfaceAsTexture = 0xe5ceae41,
NvAPI_D3D9_StretchRectEx = 0x22de03aa,
NvAPI_D3D9_ClearRT = 0x332d3942,
NvAPI_D3D_CreateQuery = 0x5d19bca4,
NvAPI_D3D_DestroyQuery = 0xc8ff7258,
NvAPI_D3D_Query_Begin = 0xe5a9aae0,
NvAPI_D3D_Query_End = 0x2ac084fa,
NvAPI_D3D_Query_GetData = 0xf8b53c69,
NvAPI_D3D_Query_GetDataSize = 0xf2a54796,
NvAPI_D3D_Query_GetType = 0x4aceeaf7,
NvAPI_D3D_RegisterApp = 0xd44d3c4e,
NvAPI_D3D9_CreatePathContextNV = 0xa342f682,
NvAPI_D3D9_DestroyPathContextNV = 0x667c2929,
NvAPI_D3D9_CreatePathNV = 0x71329df3,
NvAPI_D3D9_DeletePathNV = 0x73e0019a,
NvAPI_D3D9_PathVerticesNV = 0xc23df926,
NvAPI_D3D9_PathParameterfNV = 0xf7ff00c1,
NvAPI_D3D9_PathParameteriNV = 0xfc31236c,
NvAPI_D3D9_PathMatrixNV = 0xd2f6c499,
NvAPI_D3D9_PathDepthNV = 0xfcb16330,
NvAPI_D3D9_PathClearDepthNV = 0x157e45c4,
NvAPI_D3D9_PathEnableDepthTestNV = 0xe99ba7f3,
NvAPI_D3D9_PathEnableColorWriteNV = 0x3e2804a2,
NvAPI_D3D9_DrawPathNV = 0x13199b3d,
NvAPI_D3D9_GetSurfaceHandle = 0x0f2dd3f2,
NvAPI_D3D9_GetOverlaySurfaceHandles = 0x6800f5fc,
NvAPI_D3D9_GetTextureHandle = 0xc7985ed5,
NvAPI_D3D9_GpuSyncGetHandleSize = 0x80c9fd3b,
NvAPI_D3D9_GpuSyncInit = 0x6d6fdad4,
NvAPI_D3D9_GpuSyncEnd = 0x754033f0,
NvAPI_D3D9_GpuSyncMapTexBuffer = 0xcde4a28a,
NvAPI_D3D9_GpuSyncMapSurfaceBuffer = 0x2ab714ab,
NvAPI_D3D9_GpuSyncMapVertexBuffer = 0xdbc803ec,
NvAPI_D3D9_GpuSyncMapIndexBuffer = 0x12ee68f2,
NvAPI_D3D9_SetPitchSurfaceCreation = 0x18cdf365,
NvAPI_D3D9_GpuSyncAcquire = 0xd00b8317,
NvAPI_D3D9_GpuSyncRelease = 0x3d7a86bb,
NvAPI_D3D9_GetCurrentRenderTargetHandle = 0x022cad61,
NvAPI_D3D9_GetCurrentZBufferHandle = 0xb380f218,
NvAPI_D3D9_GetIndexBufferHandle = 0xfc5a155b,
NvAPI_D3D9_GetVertexBufferHandle = 0x72b19155,
NvAPI_D3D9_CreateTexture = 0xd5e13573,
NvAPI_D3D9_AliasPrimaryAsTexture = 0x13c7112e,
NvAPI_D3D9_PresentSurfaceToDesktop = 0x0f7029c5,
NvAPI_D3D9_CreateVideoBegin = 0x84c9d553,
NvAPI_D3D9_CreateVideoEnd = 0xb476bf61,
NvAPI_D3D9_CreateVideo = 0x89ffd9a3,
NvAPI_D3D9_FreeVideo = 0x3111bed1,
NvAPI_D3D9_PresentVideo = 0x5cf7f862,
NvAPI_D3D9_VideoSetStereoInfo = 0xb852f4db,
NvAPI_D3D9_SetGamutData = 0x2bbda32e,
NvAPI_D3D9_SetSurfaceCreationLayout = 0x5609b86a,
NvAPI_D3D9_GetVideoCapabilities = 0x3d596b93,
NvAPI_D3D9_QueryVideoInfo = 0x1e6634b3,
NvAPI_D3D9_AliasPrimaryFromDevice = 0x7c20c5be,
NvAPI_D3D9_SetResourceHint = 0x905f5c27,
NvAPI_D3D9_Lock = 0x6317345c,
NvAPI_D3D9_Unlock = 0xc182027e,
NvAPI_D3D9_GetVideoState = 0xa4527bf8,
NvAPI_D3D9_SetVideoState = 0xbd4bc56f,
NvAPI_D3D9_EnumVideoFeatures = 0x1db7c52c,
NvAPI_D3D9_GetSLIInfo = 0x694bff4d,
NvAPI_D3D9_SetSLIMode = 0xbfdc062c,
NvAPI_D3D9_QueryAAOverrideMode = 0xddf5643c,
NvAPI_D3D9_VideoSurfaceEncryptionControl = 0x9d2509ef,
NvAPI_D3D9_DMA = 0x962b8af6,
NvAPI_D3D9_EnableStereo = 0x492a6954,
NvAPI_D3D9_StretchRect = 0xaeaecd41,
NvAPI_D3D9_CreateRenderTarget = 0x0b3827c8,
NvAPI_D3D9_NVFBC_GetStatus = 0xbd3eb475,
NvAPI_D3D9_IFR_SetUpTargetBufferToSys = 0x55255d05,
NvAPI_D3D9_GPUBasedCPUSleep = 0xd504dda7,
NvAPI_D3D9_IFR_TransferRenderTarget = 0x0ab7c2dc,
NvAPI_D3D9_IFR_SetUpTargetBufferToNV12BLVideoSurface = 0xcfc92c15,
NvAPI_D3D9_IFR_TransferRenderTargetToNV12BLVideoSurface = 0x5fe72f64,
NvAPI_D3D10_AliasPrimaryAsTexture = 0x8aac133d,
NvAPI_D3D10_SetPrimaryFlipChainCallbacks = 0x73eb9329,
NvAPI_D3D10_ProcessCallbacks = 0xae9c2019,
NvAPI_D3D10_GetRenderedCursorAsBitmap = 0xcac3ce5d,
NvAPI_D3D10_BeginShareResource = 0x35233210,
NvAPI_D3D10_BeginShareResourceEx = 0xef303a9d,
NvAPI_D3D10_EndShareResource = 0x0e9c5853,
NvAPI_D3D10_SetDepthBoundsTest = 0x4eadf5d2,
NvAPI_D3D10_CreateDevice = 0x2de11d61,
NvAPI_D3D10_CreateDeviceAndSwapChain = 0x5b803daf,
NvAPI_D3D11_CreateDevice = 0x6a16d3a0,
NvAPI_D3D11_CreateDeviceAndSwapChain = 0xbb939ee5,
NvAPI_D3D11_BeginShareResource = 0x0121bdc6,
NvAPI_D3D11_EndShareResource = 0x8ffb8e26,
NvAPI_D3D11_SetDepthBoundsTest = 0x7aaf7a04,
NvAPI_D3D11_IsNvShaderExtnOpCodeSupported = 0x5f68da40,
NvAPI_D3D11_SetNvShaderExtnSlot = 0x8e90bb9f,
NvAPI_D3D12_SetNvShaderExtnSlotSpace = 0xac2dfeb5,
NvAPI_D3D12_SetNvShaderExtnSlotSpaceLocalThread = 0x43d867c0,
NvAPI_D3D11_SetNvShaderExtnSlotLocalThread = 0x0e6482a0,
NvAPI_D3D11_BeginUAVOverlapEx = 0xba08208a,
NvAPI_D3D11_BeginUAVOverlap = 0x65b93ca8,
NvAPI_D3D11_EndUAVOverlap = 0x2216a357,
NvAPI_D3D11_GetResourceHandle = 0x09d52986,
NvAPI_GPU_GetShaderPipeCount = 0x63e2f56f,
NvAPI_GPU_GetShaderSubPipeCount = 0x0be17923,
NvAPI_GPU_GetPartitionCount = 0x86f05d7a,
NvAPI_GPU_GetMemPartitionMask = 0x329d77cd,
NvAPI_GPU_GetTPCMask = 0x4a35df54,
NvAPI_GPU_GetSMMask = 0xeb7af173,
NvAPI_GPU_GetTotalTPCCount = 0x4e2f76a8,
NvAPI_GPU_GetTotalSMCount = 0xae5fbcfe,
NvAPI_GPU_GetTotalSPCount = 0xb6d62591,
NvAPI_GPU_GetGpuCoreCount = 0xc7026a87,
NvAPI_GPU_GetAllOutputs = 0x7d554f8e,
NvAPI_GPU_GetConnectedOutputs = 0x1730bfc9,
NvAPI_GPU_GetConnectedSLIOutputs = 0x0680de09,
NvAPI_GPU_GetConnectedDisplayIds = 0x0078dba2,
NvAPI_GPU_GetAllDisplayIds = 0x785210a2,
NvAPI_GPU_GetConnectedOutputsWithLidState = 0xcf8caf39,
NvAPI_GPU_GetConnectedSLIOutputsWithLidState = 0x96043cc7,
NvAPI_GPU_GetSystemType = 0xbaaabfcc,
NvAPI_GPU_GetActiveOutputs = 0xe3e89b6f,
NvAPI_GPU_GetEDID = 0x37d32e69,
NvAPI_GPU_SetEDID = 0xe83d6456,
NvAPI_GPU_GetOutputType = 0x40a505e4,
NvAPI_GPU_GetDeviceDisplayMode = 0xd2277e3a,
NvAPI_GPU_GetFlatPanelInfo = 0x36cff969,
NvAPI_GPU_ValidateOutputCombination = 0x34c9c2d4,
NvAPI_GPU_GetConnectorInfo = 0x4eca2c10,
NvAPI_GPU_GetFullName = 0xceee8e9f,
NvAPI_GPU_GetPCIIdentifiers = 0x2ddfb66e,
NvAPI_GPU_GetGPUType = 0xc33baeb1,
NvAPI_GPU_GetBusType = 0x1bb18724,
NvAPI_GPU_GetBusId = 0x1be0b8e5,
NvAPI_GPU_GetBusSlotId = 0x2a0a350f,
NvAPI_GPU_GetIRQ = 0xe4715417,
NvAPI_GPU_GetVbiosRevision = 0xacc3da0a,
NvAPI_GPU_GetVbiosOEMRevision = 0x2d43fb31,
NvAPI_GPU_GetVbiosVersionString = 0xa561fd7d,
NvAPI_GPU_GetAGPAperture = 0x6e042794,
NvAPI_GPU_GetCurrentAGPRate = 0xc74925a0,
NvAPI_GPU_GetCurrentPCIEDownstreamWidth = 0xd048c3b1,
NvAPI_GPU_GetPhysicalFrameBufferSize = 0x46fbeb03,
NvAPI_GPU_GetVirtualFrameBufferSize = 0x5a04b644,
NvAPI_GPU_GetQuadroStatus = 0xe332fa47,
NvAPI_GPU_GetBoardInfo = 0x22d54523,
NvAPI_GPU_GetRamBusWidth = 0x7975c581,
NvAPI_GPU_GetRamType = 0x57f7caac,
NvAPI_GPU_GetFBWidthAndLocation = 0x11104158,
NvAPI_GPU_GetAllClockFrequencies = 0xdcb616c3,
NvAPI_GPU_GetPerfClocks = 0x1ea54a3b,
NvAPI_GPU_SetPerfClocks = 0x07bcf4ac,
NvAPI_GPU_GetCoolerSettings = 0xda141340,
NvAPI_GPU_SetCoolerLevels = 0x891fa0ae,
NvAPI_GPU_RestoreCoolerSettings = 0x8f6ed0fb,
NvAPI_GPU_GetCoolerPolicyTable = 0x0518a32c,
NvAPI_GPU_SetCoolerPolicyTable = 0x987947cd,
NvAPI_GPU_RestoreCoolerPolicyTable = 0xd8c4fe63,
NvAPI_GPU_GetPstatesInfo = 0xba94c56e,
NvAPI_GPU_GetPstatesInfoEx = 0x843c0256,
NvAPI_GPU_SetPstatesInfo = 0xcdf27911,
NvAPI_GPU_GetPstates20 = 0x6ff81213,
NvAPI_GPU_SetPstates20 = 0x0f4dae6b,
NvAPI_GPU_GetCurrentPstate = 0x927da4f6,
NvAPI_GPU_GetPstateClientLimits = 0x88c82104,
NvAPI_GPU_SetPstateClientLimits = 0xfdfc7d49,
NvAPI_GPU_EnableOverclockedPstates = 0xb23b70ee,
NvAPI_GPU_EnableDynamicPstates = 0xfa579a0f,
NvAPI_GPU_GetDynamicPstatesInfoEx = 0x60ded2ed,
NvAPI_GPU_GetVoltages = 0x7d656244,
NvAPI_GPU_GetThermalSettings = 0xe3640a56,
NvAPI_GPU_SetDitherControl = 0xdf0dfcdd,
NvAPI_GPU_GetDitherControl = 0x932ac8fb,
NvAPI_GPU_GetColorSpaceConversion = 0x8159e87a,
NvAPI_GPU_SetColorSpaceConversion = 0xfcabd23a,
NvAPI_GetTVOutputInfo = 0x30c805d5,
NvAPI_GetTVEncoderControls = 0x5757474a,
NvAPI_SetTVEncoderControls = 0xca36a3ab,
NvAPI_GetTVOutputBorderColor = 0x6dfd1c8c,
NvAPI_SetTVOutputBorderColor = 0xaed02700,
NvAPI_GetDisplayPosition = 0x6bb1ee5d,
NvAPI_SetDisplayPosition = 0x57d9060f,
NvAPI_GetValidGpuTopologies = 0x5dfab48a,
NvAPI_GetInvalidGpuTopologies = 0x15658be6,
NvAPI_SetGpuTopologies = 0x25201f3d,
NvAPI_GPU_GetPerGpuTopologyStatus = 0xa81f8992,
NvAPI_SYS_GetChipSetTopologyStatus = 0x8a50f126,
NvAPI_GPU_Get_DisplayPort_DongleInfo = 0x76a70e8d,
NvAPI_I2CRead = 0x2fde12c5,
NvAPI_I2CWrite = 0xe812eb07,
NvAPI_I2CWriteEx = 0x283ac65a,
NvAPI_I2CReadEx = 0x4d7b0709,
NvAPI_GPU_GetPowerMizerInfo = 0x76bfa16b,
NvAPI_GPU_SetPowerMizerInfo = 0x50016c78,
NvAPI_GPU_GetVoltageDomainsStatus = 0xc16c7e2c,
NvAPI_GPU_ClientPowerTopologyGetInfo = 0xa4dfd3f2,
NvAPI_GPU_ClientPowerTopologyGetStatus = 0xedcf624e,
NvAPI_GPU_ClientPowerPoliciesGetInfo = 0x34206d86,
NvAPI_GPU_ClientPowerPoliciesGetStatus = 0x70916171,
NvAPI_GPU_ClientPowerPoliciesSetStatus = 0xad95f5ed,
NvAPI_GPU_WorkstationFeatureSetup = 0x6c1f3fe4,
NvAPI_GPU_WorkstationFeatureQuery = 0x004537df,
NvAPI_GPU_QueryWorkstationFeatureSupport = 0x80b1abb9,
NvAPI_SYS_GetChipSetInfo = 0x53dabbca,
NvAPI_SYS_GetLidAndDockInfo = 0xcda14d8a,
NvAPI_OGL_ExpertModeSet = 0x3805ef7a,
NvAPI_OGL_ExpertModeGet = 0x22ed9516,
NvAPI_OGL_ExpertModeDefaultsSet = 0xb47a657e,
NvAPI_OGL_ExpertModeDefaultsGet = 0xae921f12,
NvAPI_SetDisplaySettings = 0xe04f3d86,
NvAPI_GetDisplaySettings = 0xdc27d5d4,
NvAPI_GetTiming = 0xafc4833e,
NvAPI_DISP_GetTiming = 0x175167e9,
NvAPI_DISP_GetMonitorCapabilities = 0x3b05c7e1,
NvAPI_DISP_GetMonitorColorCapabilities = 0x6ae4cfb5,
NvAPI_DISP_EnumCustomDisplay = 0xa2072d59,
NvAPI_DISP_TryCustomDisplay = 0x1f7db630,
NvAPI_DISP_DeleteCustomDisplay = 0x552e5b9b,
NvAPI_DISP_SaveCustomDisplay = 0x49882876,
NvAPI_DISP_RevertCustomDisplayTrial = 0xcbbd40f0,
NvAPI_EnumCustomDisplay = 0x42892957,
NvAPI_TryCustomDisplay = 0xbf6c1762,
NvAPI_RevertCustomDisplayTrial = 0x854ba405,
NvAPI_DeleteCustomDisplay = 0xe7cb998d,
NvAPI_SaveCustomDisplay = 0xa9062c78,
NvAPI_QueryUnderscanCap = 0x61d7b624,
NvAPI_EnumUnderscanConfig = 0x4144111a,
NvAPI_DeleteUnderscanConfig = 0xf98854c8,
NvAPI_SetUnderscanConfig = 0x3efada1d,
NvAPI_GetDisplayFeatureConfig = 0x8e985ccd,
NvAPI_SetDisplayFeatureConfig = 0xf36a668d,
NvAPI_GetDisplayFeatureConfigDefaults = 0x0f5f4d01,
NvAPI_SetView = 0x0957d7b6,
NvAPI_GetView = 0xd6b99d89,
NvAPI_SetViewEx = 0x06b89e68,
NvAPI_GetViewEx = 0xdbbc0af4,
NvAPI_GetSupportedViews = 0x66fb7fc0,
NvAPI_GetHDCPLinkParameters = 0xb3bb0772,
NvAPI_Disp_DpAuxChannelControl = 0x8eb56969,
NvAPI_SetHybridMode = 0xfb22d656,
NvAPI_GetHybridMode = 0xe23b68c1,
NvAPI_Coproc_GetCoprocStatus = 0x1efc3957,
NvAPI_Coproc_SetCoprocInfoFlagsEx = 0xf4c863ac,
NvAPI_Coproc_GetCoprocInfoFlagsEx = 0x69a9874d,
NvAPI_Coproc_NotifyCoprocPowerState = 0xcadcb956,
NvAPI_Coproc_GetApplicationCoprocInfo = 0x79232685,
NvAPI_GetVideoState = 0x1c5659cd,
NvAPI_SetVideoState = 0x054fe75a,
NvAPI_SetFrameRateNotify = 0x18919887,
NvAPI_SetPVExtName = 0x4feeb498,
NvAPI_GetPVExtName = 0x2f5b08e0,
NvAPI_SetPVExtProfile = 0x8354a8f4,
NvAPI_GetPVExtProfile = 0x1b1b9a16,
NvAPI_VideoSetStereoInfo = 0x97063269,
NvAPI_VideoGetStereoInfo = 0x8e1f8cfe,
NvAPI_Mosaic_GetSupportedTopoInfo = 0xfdb63c81,
NvAPI_Mosaic_GetTopoGroup = 0xcb89381d,
NvAPI_Mosaic_GetOverlapLimits = 0x989685f0,
NvAPI_Mosaic_SetCurrentTopo = 0x9b542831,
NvAPI_Mosaic_GetCurrentTopo = 0xec32944e,
NvAPI_Mosaic_EnableCurrentTopo = 0x5f1aa66c,
NvAPI_Mosaic_SetGridTopology = 0x3f113c77,
NvAPI_Mosaic_GetMosaicCapabilities = 0xda97071e,
NvAPI_Mosaic_GetDisplayCapabilities = 0xd58026b9,
NvAPI_Mosaic_EnumGridTopologies = 0xa3c55220,
NvAPI_Mosaic_GetDisplayViewportsByResolution = 0xdc6dc8d3,
NvAPI_Mosaic_GetMosaicViewports = 0x07eba036,
NvAPI_Mosaic_SetDisplayGrids = 0x4d959a89,
NvAPI_Mosaic_ValidateDisplayGridsWithSLI = 0x1ecfd263,
NvAPI_Mosaic_ValidateDisplayGrids = 0xcf43903d,
NvAPI_Mosaic_EnumDisplayModes = 0x78db97d7,
NvAPI_Mosaic_ChooseGpuTopologies = 0xb033b140,
NvAPI_Mosaic_EnumDisplayGrids = 0xdf2887af,
NvAPI_GetSupportedMosaicTopologies = 0x410b5c25,
NvAPI_GetCurrentMosaicTopology = 0xf60852bd,
NvAPI_SetCurrentMosaicTopology = 0xd54b8989,
NvAPI_EnableCurrentMosaicTopology = 0x74073cc9,
NvAPI_GSync_EnumSyncDevices = 0xd9639601,
NvAPI_GSync_QueryCapabilities = 0x44a3f1d1,
NvAPI_GSync_GetTopology = 0x4562bc38,
NvAPI_GSync_SetSyncStateSettings = 0x60acdfdd,
NvAPI_GSync_GetControlParameters = 0x16de1c6a,
NvAPI_GSync_SetControlParameters = 0x8bbff88b,
NvAPI_GSync_AdjustSyncDelay = 0x2d11ff51,
NvAPI_GSync_GetSyncStatus = 0xf1f5b434,
NvAPI_GSync_GetStatusParameters = 0x70d404ec,
NvAPI_QueryNonMigratableApps = 0xbb9ef1c3,
NvAPI_GPU_QueryActiveApps = 0x65b1c5f5,
NvAPI_Hybrid_QueryUnblockedNonMigratableApps = 0x5f35bcb5,
NvAPI_Hybrid_QueryBlockedMigratableApps = 0xf4c2f8cc,
NvAPI_Hybrid_SetAppMigrationState = 0xfa0b9a59,
NvAPI_Hybrid_IsAppMigrationStateChangeable = 0x584cb0b6,
NvAPI_GPU_GPIOQueryLegalPins = 0xfab69565,
NvAPI_GPU_GPIOReadFromPin = 0xf5e10439,
NvAPI_GPU_GPIOWriteToPin = 0xf3b11e68,
NvAPI_GPU_GetHDCPSupportStatus = 0xf089eef5,
NvAPI_SetTopologyFocusDisplayAndView = 0x0a8064f9,
NvAPI_Stereo_CreateConfigurationProfileRegistryKey = 0xbe7692ec,
NvAPI_Stereo_DeleteConfigurationProfileRegistryKey = 0xf117b834,
NvAPI_Stereo_SetConfigurationProfileValue = 0x24409f48,
NvAPI_Stereo_DeleteConfigurationProfileValue = 0x49bceecf,
NvAPI_Stereo_Enable = 0x239c4545,
NvAPI_Stereo_Disable = 0x2ec50c2b,
NvAPI_Stereo_IsEnabled = 0x348ff8e1,
NvAPI_Stereo_GetStereoCaps = 0xdfc063b7,
NvAPI_Stereo_GetStereoSupport = 0x296c434d,
NvAPI_Stereo_CreateHandleFromIUnknown = 0xac7e37f4,
NvAPI_Stereo_DestroyHandle = 0x3a153134,
NvAPI_Stereo_Activate = 0xf6a1ad68,
NvAPI_Stereo_Deactivate = 0x2d68de96,
NvAPI_Stereo_IsActivated = 0x1fb0bc30,
NvAPI_Stereo_GetSeparation = 0x451f2134,
NvAPI_Stereo_SetSeparation = 0x5c069fa3,
NvAPI_Stereo_DecreaseSeparation = 0xda044458,
NvAPI_Stereo_IncreaseSeparation = 0xc9a8ecec,
NvAPI_Stereo_GetConvergence = 0x4ab00934,
NvAPI_Stereo_SetConvergence = 0x3dd6b54b,
NvAPI_Stereo_DecreaseConvergence = 0x4c87e317,
NvAPI_Stereo_IncreaseConvergence = 0xa17daabe,
NvAPI_Stereo_GetFrustumAdjustMode = 0xe6839b43,
NvAPI_Stereo_SetFrustumAdjustMode = 0x7be27fa2,
NvAPI_Stereo_CaptureJpegImage = 0x932cb140,
NvAPI_Stereo_InitActivation = 0xc7177702,
NvAPI_Stereo_Trigger_Activation = 0x0d6c6cd2,
NvAPI_Stereo_CapturePngImage = 0x8b7e99b5,
NvAPI_Stereo_ReverseStereoBlitControl = 0x3cd58f89,
NvAPI_Stereo_SetNotificationMessage = 0x6b9b409e,
NvAPI_Stereo_SetActiveEye = 0x96eea9f8,
NvAPI_Stereo_SetDriverMode = 0x5e8f0bec,
NvAPI_Stereo_GetEyeSeparation = 0xce653127,
NvAPI_Stereo_IsWindowedModeSupported = 0x40c8ed5e,
NvAPI_Stereo_AppHandShake = 0x8c610bda,
NvAPI_Stereo_HandShake_Trigger_Activation = 0xb30cd1a7,
NvAPI_Stereo_HandShake_Message_Control = 0x315e0ef0,
NvAPI_Stereo_SetSurfaceCreationMode = 0xf5dcfcba,
NvAPI_Stereo_GetSurfaceCreationMode = 0x36f1c736,
NvAPI_Stereo_Debug_WasLastDrawStereoized = 0xed4416c5,
NvAPI_Stereo_ForceToScreenDepth = 0x2d495758,
NvAPI_Stereo_SetVertexShaderConstantF = 0x416c07b3,
NvAPI_Stereo_SetVertexShaderConstantB = 0x5268716f,
NvAPI_Stereo_SetVertexShaderConstantI = 0x7923ba0e,
NvAPI_Stereo_GetVertexShaderConstantF = 0x622fdc87,
NvAPI_Stereo_GetVertexShaderConstantB = 0x712baa5b,
NvAPI_Stereo_GetVertexShaderConstantI = 0x5a60613a,
NvAPI_Stereo_SetPixelShaderConstantF = 0xa9657f32,
NvAPI_Stereo_SetPixelShaderConstantB = 0xba6109ee,
NvAPI_Stereo_SetPixelShaderConstantI = 0x912ac28f,
NvAPI_Stereo_GetPixelShaderConstantF = 0xd4974572,
NvAPI_Stereo_GetPixelShaderConstantB = 0xc79333ae,
NvAPI_Stereo_GetPixelShaderConstantI = 0xecd8f8cf,
NvAPI_Stereo_SetDefaultProfile = 0x44f0ecd1,
NvAPI_Stereo_GetDefaultProfile = 0x624e21c2,
NvAPI_Stereo_Is3DCursorSupported = 0xd7c9ec09,
NvAPI_Stereo_GetCursorSeparation = 0x72162b35,
NvAPI_Stereo_SetCursorSeparation = 0xfbc08fc1,
NvAPI_VIO_GetCapabilities = 0x1dc91303,
NvAPI_VIO_Open = 0x44ee4841,
NvAPI_VIO_Close = 0xd01bd237,
NvAPI_VIO_Status = 0x0e6ce4f1,
NvAPI_VIO_SyncFormatDetect = 0x118d48a3,
NvAPI_VIO_GetConfig = 0xd34a789b,
NvAPI_VIO_SetConfig = 0x0e4eec07,
NvAPI_VIO_SetCSC = 0xa1ec8d74,
NvAPI_VIO_GetCSC = 0x7b0d72a3,
NvAPI_VIO_SetGamma = 0x964bf452,
NvAPI_VIO_GetGamma = 0x51d53d06,
NvAPI_VIO_SetSyncDelay = 0x2697a8d1,
NvAPI_VIO_GetSyncDelay = 0x462214a9,
NvAPI_VIO_GetPCIInfo = 0xb981d935,
NvAPI_VIO_IsRunning = 0x96bd040e,
NvAPI_VIO_Start = 0xcde8e1a3,
NvAPI_VIO_Stop = 0x6ba2a5d6,
NvAPI_VIO_IsFrameLockModeCompatible = 0x7bf0a94d,
NvAPI_VIO_EnumDevices = 0xfd7c5557,
NvAPI_VIO_QueryTopology = 0x869534e2,
NvAPI_VIO_EnumSignalFormats = 0xead72fe4,
NvAPI_VIO_EnumDataFormats = 0x221fa8e8,
NvAPI_GPU_GetTachReading = 0x5f608315,
NvAPI_3D_GetProperty = 0x8061a4b1,
NvAPI_3D_SetProperty = 0xc9175e8d,
NvAPI_3D_GetPropertyRange = 0xb85de27c,
NvAPI_GPS_GetPowerSteeringStatus = 0x540ee82e,
NvAPI_GPS_SetPowerSteeringStatus = 0x9723d3a2,
NvAPI_GPS_SetVPStateCap = 0x68888eb4,
NvAPI_GPS_GetVPStateCap = 0x71913023,
NvAPI_GPS_GetThermalLimit = 0x583113ed,
NvAPI_GPS_SetThermalLimit = 0xc07e210f,
NvAPI_GPS_GetPerfSensors = 0x271c1109,
NvAPI_SYS_GetDisplayIdFromGpuAndOutputId = 0x08f2bab4,
NvAPI_SYS_GetGpuAndOutputIdFromDisplayId = 0x112ba1a5,
NvAPI_GPU_ClientRegisterForUtilizationSampleUpdates = 0xadeeaf67,
NvAPI_SYS_GetDisplayDriverInfo = 0x721faceb,
NvAPI_SYS_GetPhysicalGpuFromDisplayId = 0x9ea74659,
NvAPI_DISP_GetDisplayIdByDisplayName = 0xae457190,
NvAPI_DISP_GetGDIPrimaryDisplayId = 0x1e9d8a31,
NvAPI_DISP_GetDisplayConfig = 0x11abccf8,
NvAPI_DISP_SetDisplayConfig = 0x5d8cf8de,
NvAPI_DISP_GetAdaptiveSyncData = 0xb73d1ee9,
NvAPI_DISP_SetAdaptiveSyncData = 0x3eebba1d,
NvAPI_DISP_GetVirtualRefreshRateData = 0x8c00429a,
NvAPI_DISP_SetVirtualRefreshRateData = 0x5abbe6a3,
NvAPI_DISP_SetPreferredStereoDisplay = 0xc9d0e25f,
NvAPI_DISP_GetPreferredStereoDisplay = 0x1f6b4666,
NvAPI_DISP_GetNvManagedDedicatedDisplays = 0xdbdf0cb2,
NvAPI_DISP_AcquireDedicatedDisplay = 0x47c917ba,
NvAPI_DISP_ReleaseDedicatedDisplay = 0x1247825f,
NvAPI_Disp_GetDisplayIdInfo = 0xbae8aa5e,
NvAPI_Disp_GetDisplayIdsFromTarget = 0xe7e5f89e,
NvAPI_Disp_GetVRRInfo = 0xdf8fda57,
NvAPI_GPU_GetPixelClockRange = 0x66af10b7,
NvAPI_GPU_SetPixelClockRange = 0x5ac7f8e5,
NvAPI_GPU_GetECCStatusInfo = 0xca1ddaf3,
NvAPI_GPU_GetECCErrorInfo = 0xc71f85a6,
NvAPI_GPU_ResetECCErrorInfo = 0xc02eec20,
NvAPI_GPU_GetECCConfigurationInfo = 0x77a796f3,
NvAPI_GPU_SetECCConfiguration = 0x1cf639d9,
NvAPI_D3D1x_CreateSwapChain = 0x1bc21b66,
NvAPI_D3D9_CreateSwapChain = 0x1a131e09,
NvAPI_D3D_SetFPSIndicatorState = 0xa776e8db,
NvAPI_D3D9_Present = 0x05650beb,
NvAPI_D3D9_QueryFrameCount = 0x9083e53a,
NvAPI_D3D9_ResetFrameCount = 0xfa6a0675,
NvAPI_D3D9_QueryMaxSwapGroup = 0x5995410d,
NvAPI_D3D9_QuerySwapGroup = 0xeba4d232,
NvAPI_D3D9_JoinSwapGroup = 0x7d44bb54,
NvAPI_D3D9_BindSwapBarrier = 0x9c39c246,
NvAPI_D3D_SetVerticalSyncMode = 0x5526cfd1,
NvAPI_D3D1x_Present = 0x03b845a1,
NvAPI_D3D1x_QueryFrameCount = 0x9152e055,
NvAPI_D3D1x_ResetFrameCount = 0xfbbb031a,
NvAPI_D3D1x_QueryMaxSwapGroup = 0x9bb9d68f,
NvAPI_D3D1x_QuerySwapGroup = 0x407f67aa,
NvAPI_D3D1x_JoinSwapGroup = 0x14610cd7,
NvAPI_D3D1x_BindSwapBarrier = 0x9de8c729,
NvAPI_SYS_VenturaGetState = 0xcb7c208d,
NvAPI_SYS_VenturaSetState = 0x0ce2e9d9,
NvAPI_SYS_VenturaGetCoolingBudget = 0xc9d86e33,
NvAPI_SYS_VenturaSetCoolingBudget = 0x85ff5a15,
NvAPI_SYS_VenturaGetPowerReading = 0x63685979,
NvAPI_DISP_GetDisplayBlankingState = 0x63e5d8db,
NvAPI_DISP_SetDisplayBlankingState = 0x1e17e29b,
NvAPI_DRS_CreateSession = 0x0694d52e,
NvAPI_DRS_DestroySession = 0xdad9cff8,
NvAPI_DRS_LoadSettings = 0x375dbd6b,
NvAPI_DRS_SaveSettings = 0xfcbc7e14,
NvAPI_DRS_LoadSettingsFromFile = 0xd3ede889,
NvAPI_DRS_SaveSettingsToFile = 0x2be25df8,
NvAPI_DRS_CreateProfile = 0xcc176068,
NvAPI_DRS_DeleteProfile = 0x17093206,
NvAPI_DRS_SetCurrentGlobalProfile = 0x1c89c5df,
NvAPI_DRS_GetCurrentGlobalProfile = 0x617bff9f,
NvAPI_DRS_GetProfileInfo = 0x61cd6fd6,
NvAPI_DRS_SetProfileInfo = 0x16abd3a9,
NvAPI_DRS_FindProfileByName = 0x7e4a9a0b,
NvAPI_DRS_EnumProfiles = 0xbc371ee0,
NvAPI_DRS_GetNumProfiles = 0x1dae4fbc,
NvAPI_DRS_CreateApplication = 0x4347a9de,
NvAPI_DRS_DeleteApplicationEx = 0xc5ea85a1,
NvAPI_DRS_DeleteApplication = 0x2c694bc6,
NvAPI_DRS_GetApplicationInfo = 0xed1f8c69,
NvAPI_DRS_EnumApplications = 0x7fa2173a,
NvAPI_DRS_FindApplicationByName = 0xeee566b2,
NvAPI_DRS_SetSetting = 0x577dd202,
NvAPI_DRS_GetSetting = 0x73bf8338,
NvAPI_DRS_EnumSettings = 0xae3039da,
NvAPI_DRS_EnumAvailableSettingIds = 0xf020614a,
NvAPI_DRS_EnumAvailableSettingValues = 0x2ec39f90,
NvAPI_DRS_GetSettingIdFromName = 0xcb7309cd,
NvAPI_DRS_GetSettingNameFromId = 0xd61cbe6e,
NvAPI_DRS_DeleteProfileSetting = 0xe4a26362,
NvAPI_DRS_RestoreAllDefaults = 0x5927b094,
NvAPI_DRS_RestoreProfileDefault = 0xfa5f6134,
NvAPI_DRS_RestoreProfileDefaultSetting = 0x53f0381e,
NvAPI_DRS_GetBaseProfile = 0xda8466a0,
NvAPI_Event_RegisterCallback = 0xe6dbea69,
NvAPI_Event_UnregisterCallback = 0xde1f9b45,
NvAPI_GPU_GetCurrentThermalLevel = 0xd2488b79,
NvAPI_GPU_GetCurrentFanSpeedLevel = 0xbd71f0c9,
NvAPI_GPU_SetScanoutIntensity = 0xa57457a4,
NvAPI_GPU_GetScanoutIntensityState = 0xe81ce836,
NvAPI_GPU_SetScanoutWarping = 0xb34bab4f,
NvAPI_GPU_GetScanoutWarpingState = 0x6f5435af,
NvAPI_GPU_SetScanoutCompositionParameter = 0xf898247d,
NvAPI_GPU_GetScanoutCompositionParameter = 0x58fe51e6,
NvAPI_GPU_GetScanoutConfiguration = 0x6a9f5b63,
NvAPI_GPU_GetScanoutConfigurationEx = 0xe2e1e6f0,
NvAPI_DISP_SetHCloneTopology = 0x61041c24,
NvAPI_DISP_GetHCloneTopology = 0x47bad137,
NvAPI_DISP_ValidateHCloneTopology = 0x5f4c2664,
NvAPI_GPU_GetAdapterIdFromPhysicalGpu = 0x0ff07fde,
NvAPI_GPU_GetVirtualizationInfo = 0x44e022a9,
NvAPI_GPU_GetLogicalGpuInfo = 0x842b066e,
NvAPI_GPU_GetLicensableFeatures = 0x3fc596aa,
NvAPI_GPU_GetVRReadyData = 0x81d629c5,
NvAPI_GPU_GetPerfDecreaseInfo = 0x7f7f4600,
NvAPI_GPU_QueryIlluminationSupport = 0xa629da31,
NvAPI_GPU_GetIllumination = 0x9a1b9365,
NvAPI_GPU_SetIllumination = 0x0254a187,
NvAPI_D3D1x_IFR_SetUpTargetBufferToSys = 0x473f7828,
NvAPI_D3D1x_IFR_TransferRenderTarget = 0x9fbae4eb,

// source: https://github.com/Kaldaien/BMT/blob/master/BMT/dxgi.cpp

NvAPI_GetPhysicalGPUFromDisplay = 0x1890e8da,
NvAPI_GetPhysicalGPUFromGPUID = 0x5380ad1a,
NvAPI_GetGPUIDfromPhysicalGPU = 0x6533ea3e,

NvAPI_GetInfoFrameStatePvt = 0x7fc17574,
NvAPI_GPU_GetMemoryInfo = 0x07f9b368,
NvAPI_GPU_GetMemoryInfoEx = 0xc0599498,

NvAPI_LoadMicrocode = 0x3119f36e,
NvAPI_GetLoadedMicrocodePrograms = 0x919b3136,
NvAPI_GetDisplayDriverBuildTitle = 0x7562e947,
NvAPI_GetDisplayDriverCompileType = 0x988aea78,
NvAPI_GetDisplayDriverSecurityLevel = 0x9d772bba,
NvAPI_AccessDisplayDriverRegistry = 0xf5579360,
NvAPI_GetDisplayDriverRegistryPath = 0x0e24ceee,
NvAPI_GetUnAttachedDisplayDriverRegistryPath = 0x633252d8,
NvAPI_GPU_GetRawFuseData = 0xe0b1dce9,
NvAPI_GPU_GetFoundry = 0x5d857a00,
NvAPI_GPU_GetVPECount = 0xd8cbf37b,

NvAPI_GPU_GetTargetID = 0x35b5fd2f,

NvAPI_GPU_GetShortName = 0xd988f0f3,

NvAPI_GPU_GetVbiosMxmVersion = 0xe1d5daba,
NvAPI_GPU_GetVbiosImage = 0xfc13ee11,
NvAPI_GPU_GetMXMBlock = 0xb7ab19b9,

NvAPI_GPU_SetCurrentPCIEWidth = 0x3f28e1b9,
NvAPI_GPU_SetCurrentPCIESpeed = 0x3bd32008,
NvAPI_GPU_GetPCIEInfo = 0xe3795199,
NvAPI_GPU_ClearPCIELinkErrorInfo = 0x8456ff3d,
NvAPI_GPU_ClearPCIELinkAERInfo = 0x521566bb,
NvAPI_GPU_GetFrameBufferCalibrationLockFailures = 0x524b9773,
NvAPI_GPU_SetDisplayUnderflowMode = 0x387b2e41,
NvAPI_GPU_GetDisplayUnderflowStatus = 0xed9e8057,

NvAPI_GPU_GetBarInfo = 0xe4b701e3,

NvAPI_GPU_GetPSFloorSweepStatus = 0xdee047ab,
NvAPI_GPU_GetVSFloorSweepStatus = 0xd4f3944c,
NvAPI_GPU_GetSerialNumber = 0x14b83a5f,
NvAPI_GPU_GetManufacturingInfo = 0xa4218928,

NvAPI_GPU_GetRamConfigStrap = 0x51ccdb2a,

NvAPI_GPU_GetRamBankCount = 0x17073a3c,
NvAPI_GPU_GetArchInfo = 0xd8265d24,
NvAPI_GPU_GetExtendedMinorRevision = 0x25f17421,
NvAPI_GPU_GetSampleType = 0x32e1d697,
NvAPI_GPU_GetHardwareQualType = 0xf91e777b,
NvAPI_GPU_GetAllClocks = 0x1bd69f49,
NvAPI_GPU_SetClocks = 0x6f151055,
NvAPI_GPU_SetPerfHybridMode = 0x7bc207f8,
NvAPI_GPU_GetPerfHybridMode = 0x5d7ccaeb,
NvAPI_GPU_GetHybridControllerInfo = 0xd26b8a58,

NvAPI_RestartDisplayDriver = 0xb4b26b65,
NvAPI_GPU_GetAllGpusOnSameBoard = 0x4db019e6,

NvAPI_SetTopologyDisplayGPU = 0xf409d5e5,
NvAPI_GetTopologyDisplayGPU = 0x813d89a8,
NvAPI_SYS_GetSliApprovalCookie = 0xb539a26e,

NvAPI_CreateUnAttachedDisplayFromDisplay = 0xa0c72ee4,
NvAPI_GetDriverModel = 0x25eeb2c4,
NvAPI_GPU_CudaEnumComputeCapableGpus = 0x5786cc6e,
NvAPI_GPU_PhysxSetState = 0x4071b85e,
NvAPI_GPU_PhysxQueryRecommendedState = 0x7a4174f4,
NvAPI_GPU_GetDeepIdleState = 0x1aad16b4,
NvAPI_GPU_SetDeepIdleState = 0x568a2292,

NvAPI_GetScalingCaps = 0x8e875cf9,
NvAPI_GPU_GetThermalTable = 0xc729203c,
NvAPI_GPU_ThermChannelGetStatus = 0x65fe3aad, // undocumented: RTSS ThermChannel STATUS read (168B channel[32] layout). Pair with 0x0BC8163D GetInfo; pass GetInfo's channel_mask, read channel[priChIdx[type]]. Was previously wrapped as the values[40] "GetThermalSensors" layout — unified to the RTSS channel[32] layout (channel[k]==old values[k+8]).
NvAPI_SYS_SetPostOutput = 0xd3a092b1,

// source: PX18 ManagedNvApi.dll (see also: ccminer/nvapi.cpp)

NvAPI_GPU_PerfPoliciesGetInfo = 0x409d9841,
NvAPI_GPU_PerfPoliciesGetStatus = 0x3d358a0c,
NvAPI_GPU_ClientThermalPoliciesGetInfo = 0x0d258bb5,
NvAPI_GPU_ClientThermalPoliciesGetStatus = 0xe9c425a1,
NvAPI_GPU_ClientThermalPoliciesSetStatus = 0x34c0b13d,
NvAPI_GPU_ClientVoltRailsGetStatus = 0x465f9bcf, // aka NVAPI_ID_VOLTAGE_GET / NvAPI_{DLL,GPU}_GetCurrentVoltage
NvAPI_GPU_GetVoltageStep = 0x28766157, // unsure of the name
NvAPI_GPU_ClockClientClkDomainsGetInfo = 0x64b43a6a, // aka NVAPI_ID_CLK_RANGE_GET / NvAPI_{DLL,GPU}_GetClockBoostRanges
NvAPI_GPU_ClockClientClkVfPointsGetInfo = 0x507b4b59, // aka NVAPI_ID_CLK_BOOST_MASK / NvAPI_{DLL,GPU}_GetClockBoostMask
NvAPI_GPU_ClockClientClkVfPointsGetControl = 0x23f1b133, // aka NVAPI_ID_CLK_BOOST_TABLE_GET / NvAPI_{DLL,GPU}_GetClockBoostTable
NvAPI_GPU_ClockClientClkVfPointsSetControl = 0x0733e009, // aka NVAPI_ID_CLK_BOOST_TABLE_SET / NvAPI_{DLL,GPU}_SetClockBoostTable
NvAPI_GPU_ClockClientClkVfPointsGetStatus = 0x21537ad4, // aka NVAPI_ID_VFP_CURVE_GET / NvAPI_{DLL,GPU}_GetVFPCurve
NvAPI_GPU_PerfClientLimitsGetStatus = 0xe440b867, // aka NVAPI_ID_CURVE_GET / NvAPI_GPU_GetClockBoostLock
NvAPI_GPU_PerfClientLimitsSetStatus = 0x39442cfb, // aka NVAPI_ID_CURVE_SET / NvAPI_GPU_SetClockBoostLock
NvAPI_GPU_ClientVoltRailsGetControl = 0x9df23ca1, // aka NVAPI_ID_VOLTBOOST_GET / NvAPI_{DLL,GPU}_GetCoreVoltageBoostPercent
NvAPI_GPU_ClientVoltRailsSetControl = 0xb9306d9b, // aka NVAPI_ID_VOLTBOOST_SET / NvAPI_{DLL,GPU}_SetCoreVoltageBoostPercent

NvAPI_GPU_ClientFanArbitersGetControl = 0x600f612e,
NvAPI_GPU_ClientFanArbitersGetInfo = 0xdddfda38,
NvAPI_GPU_ClientFanArbitersGetStatus = 0xcde021b9,
NvAPI_GPU_ClientFanArbitersSetControl = 0x44cd3014,
NvAPI_GPU_ClientFanCoolersGetControl = 0x814b209f,
NvAPI_GPU_ClientFanCoolersGetInfo = 0xfb85b01e,
NvAPI_GPU_ClientFanCoolersGetStatus = 0x35aed5e8,
NvAPI_GPU_ClientFanCoolersSetControl = 0xa58971a5,
NvAPI_GPU_ClientFanPoliciesGetControl = 0xe543c540,
NvAPI_GPU_ClientFanPoliciesGetInfo = 0x52b76d12,
NvAPI_GPU_ClientFanPoliciesSetControl = 0xc181947a,
NvAPI_GPU_ClientGetLastOcScannerResults = 0x593e8e72,
NvAPI_GPU_ClientGetOcConfig = 0x210f1841,
NvAPI_GPU_ClientIllumDevicesGetInfo = 0xd4100e58,
NvAPI_GPU_ClientIllumDevicesGetControl = 0x73c01d58,
NvAPI_GPU_ClientIllumDevicesSetControl = 0x57024c62,
NvAPI_GPU_ClientIllumZonesGetControl = 0x3dbf5764,
NvAPI_GPU_ClientIllumZonesGetInfo = 0x4b81241b,
NvAPI_GPU_ClientIllumZonesSetControl = 0x197d065e,
NvAPI_GPU_ClientRegisterForOcConfigChangedUpdates = 0xf627074f,
NvAPI_GPU_ClientRegisterForOcScannerStatusUpdates = 0x1cb41116,
NvAPI_GPU_ClientRevertOc = 0xcc727b22,
NvAPI_GPU_ClientStartOcScanner = 0xbc4aee25,
NvAPI_GPU_ClientStopOcScanner = 0xc28b73de,

// source: https://github.com/processhacker2/plugins-extra/blob/master/NvGpuPlugin/nvidia.c

NvAPI_GPU_GetUsages = 0x189a1fdf,

NvAPI_GPU_GetRamMaker = 0x42aea16a,

// source: nvapi.lib

NvAPI_D3D_GetObjectHandleForResource = 0xfceac864,
NvAPI_D3D_SetResourceHint = 0x6c0ed98c,
NvAPI_D3D_BeginResourceRendering = 0x91123d6a,
NvAPI_D3D_EndResourceRendering = 0x37e7191c,
NvAPI_D3D12_QueryPresentBarrierSupport = 0xa15faef7,
NvAPI_D3D12_CreatePresentBarrierClient = 0x4d815de9,
NvAPI_D3D12_RegisterPresentBarrierResources = 0xd53c9ef0,
NvAPI_DestroyPresentBarrierClient = 0x3c5c351b,
NvAPI_JoinPresentBarrier = 0x17f6bf82,
NvAPI_LeavePresentBarrier = 0xc3ec5a7f,
NvAPI_QueryPresentBarrierFrameStatistics = 0x61b844a1,
NvAPI_D3D12_CreateDDisplayPresentBarrierClient = 0xb5a21987,
NvAPI_D3D11_CreateRasterizerState = 0xdb8d28af,
NvAPI_D3D_ConfigureAnsel = 0x341c6c7f,
NvAPI_D3D11_CreateTiledTexture2DArray = 0x7886981a,
NvAPI_D3D11_CheckFeatureSupport = 0x106a487e,
NvAPI_D3D11_CreateImplicitMSAATexture2D = 0xb8f79632,
NvAPI_D3D12_CreateCommittedImplicitMSAATexture2D = 0x24c6a07b,
NvAPI_D3D11_ResolveSubresourceRegion = 0xe6bfedd6,
NvAPI_D3D12_ResolveSubresourceRegion = 0xc24a15bf,
NvAPI_D3D11_TiledTexture2DArrayGetDesc = 0xf1a2b9d5,
NvAPI_D3D11_UpdateTileMappings = 0x9a06ea07,
NvAPI_D3D11_CopyTileMappings = 0xc09ee6bc,
NvAPI_D3D11_TiledResourceBarrier = 0xd6839099,
NvAPI_D3D11_AliasMSAATexture2DAsNonMSAA = 0xf1c54fc9,
NvAPI_D3D11_CreateGeometryShaderEx_2 = 0x99ed5c1c,
NvAPI_D3D11_CreateVertexShaderEx = 0x0beaa0b2,
NvAPI_D3D11_CreateHullShaderEx = 0xb53cab00,
NvAPI_D3D11_CreateDomainShaderEx = 0xa0d7180d,
NvAPI_D3D11_CreatePixelShaderEx_2 = 0x4162822b,
NvAPI_D3D11_CreateFastGeometryShaderExplicit = 0x71ab7c9c,
NvAPI_D3D11_CreateFastGeometryShader = 0x525d43be,
NvAPI_D3D11_DecompressView = 0x3a94e822,
NvAPI_D3D12_CreateGraphicsPipelineState = 0x2fc28856,
NvAPI_D3D12_CreateComputePipelineState = 0x2762deac,
NvAPI_D3D12_SetDepthBoundsTestValues = 0xb9333fe9,
NvAPI_D3D12_CreateReservedResource = 0x2c85f101,
NvAPI_D3D12_CreateHeap = 0x5cb397cf,
NvAPI_D3D12_CreateHeap2 = 0x924be9d6,
NvAPI_D3D12_QueryCpuVisibleVidmem = 0x26322bc3,
NvAPI_D3D12_ReservedResourceGetDesc = 0x9aa2aabb,
NvAPI_D3D12_UpdateTileMappings = 0xc6017a7d,
NvAPI_D3D12_CopyTileMappings = 0x47f78194,
NvAPI_D3D12_ResourceAliasingBarrier = 0xb942bab7,
NvAPI_D3D12_CaptureUAVInfo = 0x6e5ea9db,
NvAPI_D3D11_GetResourceGPUVirtualAddressEx = 0xaf6d14da,
NvAPI_D3D11_EnumerateMetaCommands = 0xc7453ba8,
NvAPI_D3D11_CreateMetaCommand = 0xf505fba0,
NvAPI_D3D11_InitializeMetaCommand = 0xaec629e9,
NvAPI_D3D11_ExecuteMetaCommand = 0x82236c47,
NvAPI_D3D12_EnumerateMetaCommands = 0xcd9141d8,
NvAPI_D3D12_CreateMetaCommand = 0xeb29634b,
NvAPI_D3D12_InitializeMetaCommand = 0xa4125399,
NvAPI_D3D12_ExecuteMetaCommand = 0xde24fc3d,
NvAPI_D3D12_CreateCommittedResource = 0x027e98ae,
NvAPI_D3D12_GetCopyableFootprints = 0xf6305eb5,
NvAPI_D3D12_CopyTextureRegion = 0x82b91b25,
NvAPI_D3D12_IsNvShaderExtnOpCodeSupported = 0x3dfacec8,
NvAPI_D3D12_GetOptimalThreadCountForMesh = 0xb43995cb,
NvAPI_D3D_IsGSyncCapable = 0x9c1eed78,
NvAPI_D3D_IsGSyncActive = 0xe942b0ff,
NvAPI_D3D1x_DisableShaderDiskCache = 0xd0cbca7d,
NvAPI_D3D11_MultiGPU_GetCaps = 0xd2d25687,
NvAPI_D3D11_MultiGPU_Init = 0x017be49e,
NvAPI_D3D11_CreateMultiGPUDevice = 0xbdb20007,
NvAPI_D3D_QuerySinglePassStereoSupport = 0x6f5f0a6d,
NvAPI_D3D_SetSinglePassStereoMode = 0xa39e6e6e,
NvAPI_D3D12_QuerySinglePassStereoSupport = 0x3b03791b,
NvAPI_D3D12_SetSinglePassStereoMode = 0x83556d87,
NvAPI_D3D_QueryMultiViewSupport = 0xb6e0a41c,
NvAPI_D3D_SetMultiViewMode = 0x8285c8da,
NvAPI_D3D_QueryModifiedWSupport = 0xcbf9f4f5,
NvAPI_D3D_SetModifiedWMode = 0x06ea4bf4,
NvAPI_D3D12_QueryModifiedWSupport = 0x51235248,
NvAPI_D3D12_SetModifiedWMode = 0xe1fdaba7,
NvAPI_D3D_CreateLateLatchObject = 0x2db27d09,
NvAPI_D3D_QueryLateLatchSupport = 0x8ceca0ec,
NvAPI_D3D_RegisterDevice = 0x8c02c4d0,
NvAPI_D3D11_MultiDrawInstancedIndirect = 0xd4e26bbf,
NvAPI_D3D11_MultiDrawIndexedInstancedIndirect = 0x59e890f9,
NvAPI_D3D_ImplicitSLIControl = 0x2aede111,
NvAPI_D3D12_GetNeedsAppFPBlendClamping = 0x6ef4d2d1,
NvAPI_D3D12_UseDriverHeapPriorities = 0xf0d978a8,
NvAPI_D3D12_Mosaic_GetCompanionAllocations = 0xa46022c7,
NvAPI_D3D12_Mosaic_GetViewportAndGpuPartitions = 0xb092b818,
NvAPI_D3D1x_GetGraphicsCapabilities = 0x52b1499a,
NvAPI_D3D12_GetGraphicsCapabilities = 0x01e87354,
NvAPI_D3D11_RSSetExclusiveScissorRects = 0xae4d73ef,
NvAPI_D3D11_RSSetViewportsPixelShadingRates = 0x34f7938f,
NvAPI_D3D11_CreateShadingRateResourceView = 0x99ca2dff,
NvAPI_D3D11_RSSetShadingRateResourceView = 0x1b0c2f83,
NvAPI_D3D11_RSGetPixelShadingRateSampleOrder = 0x092442a1,
NvAPI_D3D11_RSSetPixelShadingRateSampleOrder = 0xa942373a,
NvAPI_D3D_InitializeVRSHelper = 0x4780d70b,
NvAPI_D3D_InitializeNvGazeHandler = 0x5b3b7479,
NvAPI_D3D_InitializeSMPAssist = 0x42763d0c,
NvAPI_D3D_QuerySMPAssistSupport = 0xc57921de,
NvAPI_D3D_GetSleepStatus = 0xaef96ca1,
NvAPI_D3D_SetSleepMode = 0xac1ca9e0,
NvAPI_D3D_Sleep = 0x852cd1d2,
NvAPI_D3D_SetReflexSync = 0xb9f6faff,
NvAPI_D3D_GetLatency = 0x1a587f9c,
NvAPI_D3D_SetLatencyMarker = 0xd9984c05,
NvAPI_D3D12_SetAsyncFrameMarker = 0x13c98f73,
NvAPI_D3D12_NotifyOutOfBandCommandQueue = 0x03d6e8cb,
NvAPI_D3D12_CreateCubinComputeShader = 0x2a2c79e8,
NvAPI_D3D12_CreateCubinComputeShaderEx = 0x3151211b,
NvAPI_D3D12_CreateCubinComputeShaderWithName = 0x1dc7261f,
NvAPI_D3D12_LaunchCubinShader = 0x5c52bb86,
NvAPI_D3D12_DestroyCubinComputeShader = 0x7fb785ba,
NvAPI_D3D12_GetCudaTextureObject = 0x80403fc9,
NvAPI_D3D12_GetCudaSurfaceObject = 0x48f5b2ee,
NvAPI_D3D12_IsFatbinPTXSupported = 0x70c07832,
NvAPI_D3D12_CreateCuModule = 0xad1a677d,
NvAPI_D3D12_EnumFunctionsInModule = 0x7ab88d88,
NvAPI_D3D12_CreateCuFunction = 0xe2436e22,
NvAPI_D3D12_LaunchCuKernelChain = 0x24973538,
NvAPI_D3D12_LaunchCuKernelChainEx = 0x846a9bf0,
NvAPI_D3D12_DestroyCuModule = 0x41c65285,
NvAPI_D3D12_DestroyCuFunction = 0xdf295ea6,
NvAPI_D3D11_CreateCubinComputeShader = 0x0ed98181,
NvAPI_D3D11_CreateCubinComputeShaderEx = 0x32c2a0f6,
NvAPI_D3D11_CreateCubinComputeShaderWithName = 0xb672be19,
NvAPI_D3D11_LaunchCubinShader = 0x427e236d,
NvAPI_D3D11_DestroyCubinComputeShader = 0x01682c86,
NvAPI_D3D11_IsFatbinPTXSupported = 0x6086bd93,
NvAPI_D3D11_CreateUnorderedAccessView = 0x74a497a1,
NvAPI_D3D11_CreateShaderResourceView = 0x65cb431e,
NvAPI_D3D11_CreateSamplerState = 0x89eca416,
NvAPI_D3D11_GetCudaTextureObject = 0x9006fa68,
NvAPI_D3D11_GetResourceGPUVirtualAddress = 0x1819b423,
NvAPI_D3D12_GetRaytracingCaps = 0x85a6c2a0,
NvAPI_D3D12_GetRaytracingDisplacementMicromapArrayPrebuildInfo = 0xfa99b6de,
NvAPI_D3D12_GetRaytracingOpacityMicromapArrayPrebuildInfo = 0x4726d180,
NvAPI_D3D12_SetCreatePipelineStateOptions = 0x5c607a27,
NvAPI_D3D12_CheckDriverMatchingIdentifierEx = 0xafb237d4,
NvAPI_D3D12_GetRaytracingAccelerationStructurePrebuildInfoEx = 0x8d025b77,
NvAPI_D3D12_BuildRaytracingOpacityMicromapArray = 0x814f8d11,
NvAPI_D3D12_RelocateRaytracingOpacityMicromapArray = 0x0425c538,
NvAPI_D3D12_BuildRaytracingDisplacementMicromapArray = 0x066f569d,
NvAPI_D3D12_RelocateRaytracingDisplacementMicromapArray = 0x1c142308,
NvAPI_D3D12_EmitRaytracingDisplacementMicromapArrayPostbuildInfo = 0x68b9a790,
NvAPI_D3D12_EmitRaytracingOpacityMicromapArrayPostbuildInfo = 0x1d9a39b6,
NvAPI_D3D12_BuildRaytracingAccelerationStructureEx = 0xe24ead45,
NvAPI_D3D12_QueryWorkstationFeatureProperties = 0xa92ea23a,
NvAPI_D3D12_CreateCommittedRDMABuffer = 0xe78dcb44,
NvAPI_DirectD3D12GraphicsCommandList_Create = 0x74a4e712,
NvAPI_DirectD3D12GraphicsCommandList_Release = 0x99da3dde,
NvAPI_DirectD3D12GraphicsCommandList_Reset = 0x999c26d8,

// source: nvapi_interface.h (2026)

NvAPI_DISP_GetEdidData = 0x436ced76,
NvAPI_DISP_GetNvManagedDedicatedDisplayMetadata = 0xd645d80c,
NvAPI_DISP_SetNvManagedDedicatedDisplayMetadata = 0x3d8b129a,
NvAPI_Disp_GetColorimetry = 0x00b421ad,
NvAPI_GPU_GetEncoderSessionsInfo = 0xd8a72ce5,
NvAPI_GPU_GetEncoderStatistics = 0xf0a9aeeb,
NvAPI_GPU_GetGPUInfo = 0xafd1b02c,
NvAPI_GPU_GetGspFeatures = 0x581c4391,
NvAPI_GPU_GetUUID = 0xdc95673d,
NvAPI_GPU_NVLINK_GetCaps = 0xbef1119d,
NvAPI_GPU_NVLINK_GetStatus = 0xc72a38e3,
NvAPI_NGX_GetDriverFeatureSupport = 0x6194b19d,
NvAPI_NGX_GetNGXOverrideState = 0x3fd96fba,
NvAPI_NGX_SetNGXOverrideState = 0xb60fcb4e,
NvAPI_RegisterRiseCallback = 0x9cfe8f94,
NvAPI_RequestRise = 0x5047de98,
NvAPI_SYS_GetLogicalGPUs = 0xccfffc10,
NvAPI_SYS_GetPhysicalGPUs = 0xd3b24d2d,
NvAPI_UninstallRise = 0xab8d09f6,
NvAPI_Vulkan_DestroyLowLatencyDevice = 0x11a5932b,
NvAPI_Vulkan_GetLatency = 0x3233d44a,
NvAPI_Vulkan_GetSleepStatus = 0xadf966af,
NvAPI_Vulkan_InitLowLatencyDevice = 0x5c1696b6,
NvAPI_Vulkan_NotifyOutOfBandVkQueue = 0x5d6d3840,
NvAPI_Vulkan_SetLatencyMarker = 0xa17d13d6,
NvAPI_Vulkan_SetSleepMode = 0x2acfd162,
NvAPI_Vulkan_Sleep = 0x36732b1e,
NvAPI_D3D11_CreateCubinComputeShaderExV2 = 0xf2c71d48,
NvAPI_D3D11_GetCudaIndependentViewObject = 0x34d2afa8,
NvAPI_D3D11_GetCudaMergedTextureSamplerObject = 0x5d637d8f,
NvAPI_D3D11_SetAsyncFrameMarker = 0x59c2c510,
NvAPI_D3D12_BuildRaytracingPartitionedTlasIndirect = 0x7cfc6fc3,
NvAPI_D3D12_ConvertCooperativeVectorMatrix = 0x0f252cb3,
NvAPI_D3D12_ConvertCooperativeVectorMatrixMultiple = 0x96ba5235,
NvAPI_D3D12_CreateCubinComputeShaderExV2 = 0x299f5fdc,
NvAPI_D3D12_EnableRaytracingValidation = 0x1de5991b,
NvAPI_D3D12_FlushRaytracingValidationMessages = 0xb8fb1fcb,
NvAPI_D3D12_GetCudaIndependentDescriptorObject = 0x0ddac234,
NvAPI_D3D12_GetCudaMergedTextureSamplerObject = 0x329fe6e0,
NvAPI_D3D12_GetPhysicalDeviceCooperativeVectorProperties = 0x8f182aec,
NvAPI_D3D12_GetRaytracingMultiIndirectClusterOperationRequirementsInfo = 0x5c9163f4,
NvAPI_D3D12_GetRaytracingPartitionedTlasIndirectPrebuildInfo = 0xcdfdc5f2,
NvAPI_D3D12_RaytracingExecuteMultiIndirectClusterOperation = 0x67c798af,
NvAPI_D3D12_RegisterRaytracingValidationMessageCallback = 0x8554eb38,
NvAPI_D3D12_SetCreateCommandQueueLowLatencyHint = 0x548c224f,
NvAPI_D3D12_SetFlipConfig = 0xf3148c42,
NvAPI_D3D12_UnregisterRaytracingValidationMessageCallback = 0x26975da6,

// source: gpu-z

/// `NvAPI_GPU_GetCOPROCInfo(NV_COPROC_INFO *p)` — hybrid-graphics dGPU co-processor
/// power-management state query. "COPROC" = the discrete GPU acting as a CO-PROCESSOR
/// to the integrated GPU on NVIDIA Optimus / MS-Hybrid laptop platforms — NOT NVLink
/// topology, NOT GB202-style companion chiplets, NOT MIG/vGPU partitioning (the "NVL_"
/// escape prefix is just the RM control-class namespace). Proven by the NV_COPROC_*
/// status strings (MGPU_NOT_SUPPORTED, DGPU_NOT_SUPPORTED, DISABLED_BY_HYBRID,
/// DGPU_POSTING_DEVICE) and NV_COPROC_FLAGS_* (OPTIMUS_STYLE_POWER_MANAGEMENT,
/// GCOFF_ENABLED [=GPU Choreography OFF / dGPU power-gating], D3HOT_SUPPORTED,
/// LONG_IDLE_D3_SUPPORTED, MS_HYBRID_NV_APPROVED, FORCE_GPU_SWITCH_AVAILABLE).
/// Reversed from nvapi64_impl.dll handler @0x1803DA280 (core impl sub_1803D8AE0,
/// trampoline sub_1800FC880, 1 arg). RTTI public struct `.?AUNV_COPROC_INFO_V7@@`
/// (qword_180505950) and RM escape `.?AU_NVL_ESC_COMMON_COPROC_QUERY_INFO@@`
/// (qword_180505978). Prototype: `__int64 f(NV_COPROC_INFO *p)` (1 arg).
/// Input: p->version (offset 0, u32); accepts a v1..v7 family: 0x00010008 (v1,sz8),
/// 0x0002000C (v2,sz12), 0x00030018 (v3,sz24), 0x00040020 (v4,sz32), 0x00050024
/// (v5,sz36), 0x00060058 (v6,sz88), 0x00070060 (v7,sz96); mismatch => -9.
/// RM escape 0x0100009F via sub_180389320, 158-byte buffer, hPhysicalGPU @buf+0x30.
/// Output (NV_COPROC_INFO_V7, 96 bytes): caps1@4 (14 bit-remapped flags from esc+0x34),
/// caps2@8 (~30 remapped bits from esc+0x38; virt remaps src 0x1000000->out 0x20000000,
/// src 0x2000000->out 0x1000000), state dwords@12..32, co-proc mode enum@36/37,
/// more enum/state fields out to @92 (V7+). SR-IOV anomaly @0x1803D92B6: if
/// (caps2 & 0x420)==0x420 returns -1 (two mutually-exclusive hybrid/virt caps asserted).
/// LOW WRAP VALUE for nvoc: reports platform hybrid-power policy (dGPU posting, GCOFF
/// power-gating, D3hot, iGPU/dGPU switchability, MS-Hybrid approval) — no clocks/power/
/// thermals/fans and no tuning surface. On a desktop single-GPU box every cap is clear
/// and the query is a no-op. Read-only descriptor, not a sensor or OC lever — skip.
Unknown_1629A173 = 0x1629a173,
/// `NvAPI_DISP_GetAssociatedNvidiaGpuHandle(hDisplay, *phGpu)` — display→GPU handle
/// resolver. Reversed from nvapi64_impl.dll handler @0x180181F10 (trampoline
/// sub_1801013F0, 2 args). Prototype: `__int64 f(NvDisplayHandle, NvPhysicalGpuHandle*)`.
/// Validates (hDisplay & 0xFF000000)==0xDE000000 (display-handle magic); non-matching
/// non-zero => -310 EXPECTED_DISPLAY_HANDLE; hDisplay==0 allowed.
/// RM: sub_180389320(ioctl=0x07000061, &esc52, sz=0x34(52), hDisplay, ...); GPU handle
/// read from HIDWORD(esc[3]) (escape offset 0x1C). One-shot handle lookup.
/// Output: *phGpu = GPU handle. Display/handle plumbing — no sensor data. Wrap only if
/// nvoc builds display-side topology; otherwise skip.
Unknown_F1D2777B = 0xf1d2777b,
/// `NvAPI_GPU_GetPhysicalGpuHandlesFromLogical(hLogicalGpu, *outHandles, *pCount)` —
/// LOGICAL→PHYSICAL GPU handle fan-out (NOT display topology). Reversed from
/// nvapi64_impl.dll handler @0x1801BBF60 (trampoline sub_1801019D0, 3 args). Prototype:
/// `__int64 f(NvLogicalGpuHandle, NvPhysicalGpuHandle*, uint32_t*)`.
/// Validates (hLogicalGpu & 0xFF000000)==0xAA000000 (logical-GPU handle magic) @0x1801bbc82.
/// Helper sub_1801BBB30 issues RM escape 0x07000006 (sub_180389320, 0x138(312)-byte buf);
/// RTTI `.?AU_NV_ESC_NVAPI_GET_PHYSICAL_FROM_LOGICAL_GPU@@` (qword_180503A20) — dispositive.
/// Output: copies buf[13]→*pCount and buf[14+i]→outHandles[i] as 8-byte (QWORD) PHYSICAL
/// GPU handles (loop @0x1801becb). Outer wrapper then calls sub_180370990 (escape 0x0700010C,
/// RTTI _NV_ESC_NVAPI_GET_DETAILS_FROM_DISPLAYID) per display to pick the "primary" physical
/// GPU and reorders outHandles to put it first. Pure GPU-handle plumbing — no display IDs,
/// no sensor data. CORRECTION: previous RE wrongly called this "GetConnectedDisplayIds";
/// that is a DIFFERENT id (0x0078dba2, escape 0x07000112, enumerates NV_GPU_DISPLAYIds).
/// Skip for monitoring; wrap only if nvoc builds logical/physical GPU topology.
Unknown_8EFC0978 = 0x8efc0978,
/// `NvAPI_GPU_GetComputeCapabilities(hGpu, *pCaps)` — PhysX/compute/framebuffer
/// capability word (NOT virtualization, despite the name). Authoritatively reversed
/// from nvapi64_impl.dll handler @0x1801ABAD0 (trampoline sub_1800C2530, 2 args).
/// RTTI `.?AU_NV_ESC_NVAPI_GET_COMPUTE_CAPS@@` (qword_1805043A0).
/// Prototype: `__int64 f(NvPhysicalGpuHandle, NV_GPU_COMPUTE_CAPS_INFO*)`.
/// Input: pCaps->version MUST be 0x00010008 (v1,sz8). RM escape 0x7000029 via
/// sub_18038A360 (status dword @buf+0x38 bit0 = compute-capable -> 0x2); supporting
/// escapes 0x700023D/0x7000025 (physical VRAM KB -> 0x200, sub_18019EC20),
/// 0x7000024 (PCI id quadruple), sub_18017BE10 (Physx.cpl >=8.9.4.0 -> 0x100),
/// sub_18039B9C0 (registry PhysxGpuId match -> 0x400), sub_1803B94B0 (board-DB match -> 0x4).
/// Output capability word (see NV_GPU_COMPUTE_CAPS bitflags for per-bit semantics):
/// 0x1 BASE_COMPUTE, 0x2 COMPUTE_CAPABLE, 0x4 BOARD_DB_MATCH, 0x100 PHYSX_INSTALLED,
/// 0x200 VRAM_GE_256MB, 0x400 PHYSX_GPU_SELECTED. Measured on dev laptop: 0x703 =
/// 0x1|0x2|0x100|0x200|0x400 (0x4 absent = no board-DB row matched this SKU).
/// -104 (DATA_NOT_FOUND) is mapped to 0 (success-but-empty). One-shot capability
/// assembly — not a live sensor. Good for a one-shot GpuCapabilities struct at startup.
/// WRAPPED as `NvAPI_GPU_GetComputeCapabilities` (variant name must match the `nvapi!`-
/// declared FFI function so the macro resolves the ID via `Api::NvAPI_GPU_GetComputeCapabilities.id()`).
NvAPI_GPU_GetComputeCapabilities = 0xb7bcf50d,
/// `NvAPI_GPU_GetAllComputeCapabilities(*pInfo, hGpu, flags, hDisplay)` — bulk per-output
/// capability enumerator (wraps 0xB7BCF50D once per connected output). Reversed from
/// nvapi64_impl.dll handler @0x180180D50 (trampoline sub_1800BE7C0, 4 args). Prototype:
/// `__int64 f(NV_GPU_ALL_OUTPUTS_INFO*, NvPhysicalGpuHandle, uint32_t, NvDisplayHandle)`.
/// Input: pInfo->version accepts 0x00020010 (v2,sz16, carries entry-array ptr) or
/// 0x00010008 (v1,sz8, count-only). (Old guess "0x0002000c" size was 12; real size is
/// 16 = 0x10.) The "handles?" intuition was right — it is a per-display-handle array.
/// RM: sub_180389E30(ioctl=0x070000A9, esc60, sz=0x3C(60), &out, ...); allocates a
/// 0x7810 (30736)B buffer = 256 entries. Calls sub_1801ABAD0 (the 0xB7BCF50D handler)
/// once per connected output. One-shot enumeration.
/// Output: {u64 displayId; u32 capsFlags}[] per output + count; active entry marked 0x8.
/// Only useful for per-display capability breakdown; for a single-GPU daemon,
/// 0xB7BCF50D alone suffices. Not a live sensor.
Unknown_36E39E6B = 0x36e39e6b,
/// `Unknown(hGpu, *mut { version = 0x00010048 (v1, 72 bytes), flags@4, count@5, data[count]@6 })`.
/// Reversed from nvapi64_impl.dll handler @0x180238CC0 (0x60B30 RM family). Calls the
/// GPU-control RM dispatcher with subcommand 0x20882CF9 (100ms x2 retry when struct
/// offset 70 is non-zero). MEASURED NOT a live power/voltage read: the 32-byte `data`
/// payload is identical across repeated reads AND does not change under GPU load, even
/// with admin privileges. The call also returns NVAPI_INVALID_USER_PRIVILEGE without
/// elevation — it routes through the privileged `\\.\NvAdminDevice` RM path, unlike the
/// plaintext public reads (power topology 0x20880B33, volt rails). Concluded: this is a
/// deterministic, privileged, non-realtime blob (capability/key/descriptor) — NOT the
/// GPU-Z per-rail Board/Chip/MVDDC/PWR_SRC/16-pin live readings. Do not wrap as a
/// status field. See docs/gpuz-nvapi-runtime-windbg.md for the dynamic-confirmation path.
Unknown_7457CAB5 = 0x7457cab5,
/// `GPU_GetRasterOperators(hGpu, *mut u32)`
Unknown_GetROPCount = 0xfdc129fa,

// --- RE-record entries: 3 unknown QueryInterface IDs resolved via static RE of
// nvapi64_impl.dll (IDA). Dispatch table `off_1804DE000` is 12-byte entries
// [4B id][4B pad][4B handler ptr]. These three are thermal/descriptor queries,
// not power-rail sources — kept here as documentation-only records; all three
// are table/descriptor/stub queries, not status reads — do not wrap.
// (NB: live per-rail power DOES come from NVAPI — see PowerMonitor
// 0xC12EB19E/0xF40238EF above, now wrapped. The earlier "per-rail watts are
// WinRing0-only" conclusion in docs/gpuz-per-rail-investigation.md was for the
// specific IDs probed there, not PowerMonitor.)

/// `ThermChannelGetStatus(hGpu, *mut { version, .. })`.
/// Reversed from nvapi64_impl.dll handler @0x1801E0BC0. Identity proven by embedded
/// error string "NvAPI_GPU_ThermChannelGetStatus received version..." @0x180485cf0
/// and RTTI `_NV_ESC_NVAPI_GPU_THERMAL_RMCTRLS`. Prototype:
/// `__int64 __fastcall(int hGpu, __int64 structPtr)`.
/// Input struct first DWORD = version magic; accepted: 65596 (v1, sz60),
/// 131240 (v2, sz168), 210120 (v3, sz13512); mismatch => -9 INCOMPATIBLE_STRUCT.
/// Allocates a 0xC9E0 (51680)B RM buffer, dispatches via sub_180389320(117440911, ...)
/// => RM ioctl 0x0700018F (thermal control, NOT the 0x07000046 power ioctl);
/// escape subcommand written to buf[13] = 0x2080853B.
/// `NvAPI_GPU_ThermChannelGetInfo(hGpu, *NV_GPU_THERMAL_THERM_CHANNEL_INFO_PARAMS)`
/// — thermal-channel capability/topology descriptor (the INFO half of the
/// ThermChannel pair; the live-reading STATUS half is `0x65FE3AAD`
/// `NvAPI_GPU_GetThermalSensors`). NDA-developer-SDK private API. Identity confirmed
/// by RTSS (RivaTuner) source: `NVAPIIID_GPU_ThermChannelGetInfo = 0x0BC8163D`.
/// Reversed from nvapi64_impl.dll handler @0x1801E0BC0 (RTTI
/// `_NV_ESC_NVAPI_GPU_THERMAL_RMCTRLS`). Prototype:
/// `__int64 __fastcall(int hGpu, __int64 structPtr)`.
/// Input struct first DWORD = version magic `(v<<16)|sizeof`; RTSS uses
/// `NV_GPU_THERMAL_THERM_CHANNEL_INFO_PARAMS_V2` with version 2. On success fills
/// `channelMask` (which of 32 channels exist) + per-channel info records + a
/// `priChIdx[5]` LUT indexing the primary channel per type
/// (GPU_AVG=0, GPU_MAX=1, BOARD=2, MEMORY=3, PWR_SUPPLY=4). The caller passes
/// `priChIdx[type]` to the STATUS read to get that type's temperature.
/// Iterates i=0..0xFE; per set channel writes a record at &buf[35*i]
/// (controller type via sub_1801DA7E0, etc.).
/// HIGH WRAP VALUE: this is the NVAPI-native path to HOTSPOT (GPU_MAX) and MEMORY
/// temperatures — exactly what the `hotspot-temp-sensor` branch needs, with no
/// MMIO/kernel-driver requirement. Pair with 0x65FE3AAD (GetThermalSensors/STATUS).
NvAPI_GPU_ThermChannelGetInfo = 0x0bc8163d,
/// `NvAPI_GPU_PowerMonitorGetInfo(hGpu, *NV_GPU_POWER_MONITOR_GET_INFO)` — power-
/// monitor capability/topology descriptor (the INFO half; live wattage is the STATUS
/// half `0xF40238EF`). NDA-developer-SDK private API. Identity confirmed by RTSS
/// source: `NVAPIIID_GPU_PowerMonitorGetInfo = 0xC12EB19E`. Handler @0x180257660 in
/// nvapi64_impl.dll (RTTI `_NV_ESC_NVAPI_GPU_POWER_RMCTRLS`, same family as
/// NvAPI_GPU_GetPowerTopology). Prototype: `__int64 __fastcall(uint hGpu, _DWORD*)`.
///
/// WRAPPED & LIVE. Returns a DESCRIPTOR table: `bSupported`, `channelMask`,
/// `totalGpuPowerChannelMask`, `totalGpuChannelIdx`, and per-channel info
/// (channel_type, pwr_rail, volt_fixed_uv, pwr_corr_slope, …). The descriptor
/// region is variable-stride (record length depends on channel_type; type 5/7
/// carry VF-estimation LUTs) — parsed by signature scan in `nvapi_rs::power`.
/// Use to discover `channelMask` + per-channel identity.
///
/// STRUCT-SIZE GATE (verified @0x180257660): the handler reads the caller's
/// first DWORD (version magic `(ver<<16)|sizeof`) and accepts ONLY these:
///   65940  = (1<<16)|396   (header-only: just channel_mask, no descriptors)
///   68264  = (1<<16)|2728  (descriptors; type=5 VF-LUTs truncated)
///   199848 = (3<<16)|3208  (more complete VF-LUTs)
///   268456 = (4<<16)|6312  (richest; full VF-LUTs)
///   377896 = (5<<16)|50216 (different header layout — mask@+0x2C, not +0x10)
/// Anything else → -9 INCOMPATIBLE_STRUCT_VERSION. v1|2728 / v3|3208 / v4|6312
/// share an IDENTICAL header + descriptor-offset layout (differ only in
/// type=5 VF-LUT truncation), so the reader works on whichever the driver
/// accepts; nvapi-rs tries v4→v3→v1|2728.
///
/// HISTORICAL: an earlier probe concluded GetInfo was "unsupported on all GPUs"
/// — that was a probe BUG (it fed the GetStatus accepted-magics to GetInfo;
/// the two IIDs share NO accepted magics). With the correct per-IID magics
/// both return Ok. See `powermonitor-per-channel-working` memory.
NvAPI_GPU_PowerMonitorGetInfo = 0xc12eb19e,
/// `NvAPI_GPU_PowerMonitorGetStatus(hGpu, *NV_GPU_POWER_MONITOR_GET_STATUS)` — live
/// per-rail GPU power in mW (the STATUS half of the PowerMonitor pair).
/// NDA-developer-SDK private API. `NVAPIIID_GPU_PowerMonitorGetStatus = 0xF40238EF`.
///
/// WRAPPED & LIVE. Handler @0x180258170 in nvapi64_impl.dll, funneling into the
/// same RM escape 0x06FF0016 as GetInfo. The caller sets the INPUT `channel_mask`
/// at struct +0x04 (copy from GetInfo); the driver fills only those channels.
/// Units CONFIRMED by exact GPU-Z OCR match (raw mW ÷ 1000 = W) under core +
/// memory load: +0x08=Board, +0x14=Chip, +0x2C=MVDDC, +0x98=PWR_SRC (channel-
/// order-dependent offsets, validated on RTX 4060 Laptop). nvapi-rs surfaces
/// these 4 as `PowerRails`; the full per-channel table is `PowerMonitor`.
///
/// STRUCT-SIZE GATE (@0x180258170) accepts these (NOT the same as GetInfo's):
///   65928  = (1<<16)|392   66972 = (1<<16)|1436
///   69408  = (1<<16)|3872  74968 = (1<<16)|9432
///   336752 = (5<<16)|9072
/// HISTORICAL: an earlier RE thought the handler was a -104 stub — that was the
/// wrong IID's handler (0x18024D4E0); the real GetStatus handler @0x180258170
/// is functional. Units earlier seemed ambiguous (idle ratio to NVML
/// collapsed) but that was because ch0 is an input/16-pin summation channel,
/// not the board total — resolved by GPU-Z cross-validation.
NvAPI_GPU_PowerMonitorGetStatus = 0xf40238ef,
/// Internal NVAPI unload/cleanup function — the sibling of `NvAPI_Unload`
/// (`0xD22BDD7E`). MSI Afterburner's RTHAL.dll `CNVAPIInterface::Uninit`
/// (handler `?Uninit@CNVAPIInterface@@QAEXXZ_0` @0x10029D00) resolves BOTH
/// `0xD7C61344` (primary) and `0xD22BDD7E` (fallback) via nvapi_QueryInterface at
/// teardown and calls whichever is present to decrement NVAPI's refcount /
/// tear down the session before `FreeLibrary(nvapi.dll)`. Present in the
/// reference NVAPI dispatch table `nvapi64_impl_qi_table.txt` at idx 6
/// (handler VA 0x1800E62E0 in nvapi64_impl.dll), but unnamed there.
/// This was the SINGLE real IID gap found when auditing MSI Afterburner's full
/// NVAPI surface vs nvapi-rs (~70 IIDs used, all others already registered).
/// DO NOT WRAP: it is a cleanup-only teardown helper with no monitoring or
/// control value. nvapi-rs already exposes `NvAPI_Unload` (0xD22BDD7E) for the
/// unload path, which is the documented public API. Kept here as a
/// documentation-only record so the IID is reserved/known.
Unknown_D7C61344_InternalUnload = 0xd7c61344,

// source: gpumon.exe (NVIDIA OEM partner tool, reverse/GPUMon/GPUMon.exe)
//
// These IDs were discovered by extracting GPUMon.exe's complete
// `nvapi_QueryInterface` surface (128 call sites on the cached
// qword_140F1A7B8 pointer; see reverse/gpumon-raw-id-table.md) and naming
// each via its caller's labeled `GPUHandle::*` / `DriverInvoker::*` /
// `Connector::*` method (GPUMon embeds the method name in its own
// `[Class::method] NvAPI fail to ...` log strings). They are NDA /
// undocumented IDs NOT present in NVIDIA's public interface table.
//
// Names use the `Unknown_<HEX>` convention (we do NOT assert a public
// NvAPI_* name unless independently confirmed); the doc comment records
// the GPUMon method + role as the evidence trail. id_hex values are
// re-verified from fresh decompiles (the phase-1 extraction misread a
// few probe-wrapper literals). See reverse/gpumon-id-catalog-for-review.md
// for the prioritized wrap list.

/// `Unknown_845866AD` — GPUHandle::pollPcieErrorCount - PCIe link error COUNT (new NDA, !=GetPCIEInfo)
Unknown_845866AD = 0x845866ad,
/// `Unknown_DB9ED906` — GPUHandle::queryPowerDevice - GetPowerSensorInfo (power-rail topology descriptor, 32-ch INA/Internal/OVR-M)
Unknown_DB9ED906 = 0xdb9ed906,
/// `Unknown_5D1D3A4E` — GPUHandle::pollVoltage - voltage rail info (ClientVoltRailsGetInfo)
Unknown_5D1D3A4E = 0x5d1d3a4e,
/// `Unknown_2C73AFDC` — GPUHandle::pollVoltage - voltage rail data (ClientVoltRailsGetStatus)
Unknown_2C73AFDC = 0x2c73afdc,
/// `Unknown_3B51F399` — GPUHandle::pollPcieBandwidth - NVPCF status data (PCIE Rx/Tx bandwidth)
Unknown_3B51F399 = 0x3b51f399,
/// `Unknown_083629B7` — GPUHandle::pollGcOffStatistics - GCOFF statistics (new NDA)
Unknown_083629B7 = 0x083629b7,
/// `Unknown_F39C1DEF` — GPUHandle::pollDifrLayer1/2/3 - DIFR power-gating residency statistics
Unknown_F39C1DEF = 0xf39c1def,
/// `Unknown_A4E81B74` — GPUHandle::pollRppgMs - RPPG residency statistics
Unknown_A4E81B74 = 0xa4e81b74,
/// `Unknown_5726C144` — GPUHandle::pollPsiGr/pollPsiMs - PSI residency statistics
Unknown_5726C144 = 0x5726c144,
/// `Unknown_7C95F2D7` — GPUHandle::pollDifrLayer1/2/3 - DIFR power-gating support info
Unknown_7C95F2D7 = 0x7c95f2d7,
/// `Unknown_0078E2A2` — GPUHandle::queryConnectedDisplay - connected display count
Unknown_0078E2A2 = 0x0078e2a2,
/// `Unknown_019185BE` — aggregator sub-call (used by pollCtac @0x14002c990)
Unknown_019185BE = 0x019185be,
/// `Unknown_0FE87B7F` — GPUHandle::resetFanCurve - ClientFanPoliciesSetStatus
Unknown_0FE87B7F = 0x0fe87b7f,
/// `Unknown_1071E0D3` — DriverInvoker::populateChipsetInfo - chipset id info
Unknown_1071E0D3 = 0x1071e0d3,
/// `Unknown_10741A55` — GPUHandle::pollFanArbiter - ClientFanArbiters info/status
Unknown_10741A55 = 0x10741a55,
/// PPAB / Dynamic-Boost ENABLE setter (NDA-private, ID 0x1504FC3D).
/// Raw `u8`/BoolU32 active flag (0=disable, non-zero=enable); NOT a *const
/// struct setStatus. Proven by decompile: GPUMon.exe thunk 0x140006E60,
/// caller 0x140030D20 logs `[GPUHandle::setDynamicBoost] active: %d`; the GPUMon
/// CLI handler (`-db`, `[CmdDispatch::cmdDynamicBoost]`) passes
/// `active=(int!=0)` and prints `Change dynamic boost controlling state to
/// [enable|disable] successful`. Matches the "PPAB Enable" checkbox on the
/// Dynamic-Boost tab of OEM partner tools.
/// NOTE: an earlier naming pass mislabeled this `Unknown_1504FC3D/setTgpQboost`
/// and mislabeled 0xB6A3DA5B as `setDynamicBoost` — 0xB6A3DA5B is actually
/// `[DriverInvoker::populatePowerLimitTable]` (a SBIOS power-limit-table GET).
NvAPI_GPU_ClientDynamicBoostSetStatus = 0x1504fc3d,
/// `Unknown_1B778765` — GPUHandle::setThermalSlowdown - change slowdown
Unknown_1B778765 = 0x1b778765,
/// `Unknown_2A03BCCF` — GPUHandle::queryPciInfo sub-call - PCI info
Unknown_2A03BCCF = 0x2a03bccf,
/// `Unknown_2EB86EE0` — Connector::pollGpuAspm - read register data (ASPM L0s/L1)
Unknown_2EB86EE0 = 0x2eb86ee0,
/// `Unknown_2F69F8E5` — GPUHandle::queryTargetTemperature - thermal policy info
Unknown_2F69F8E5 = 0x2f69f8e5,
/// `Unknown_31B855CD` — GPUHandle::pollPowerPolicy - power policy status read
Unknown_31B855CD = 0x31b855cd,
/// `Unknown_32464C6C` — GPUHandle::queryGPUInfo sub-call - GPU info
Unknown_32464C6C = 0x32464c6c,
/// `Unknown_32CA4983` — GPUHandle::setGpcClock - limit perf frequency
Unknown_32CA4983 = 0x32ca4983,
/// `Unknown_33C7F5EC` — sub_140001060 init: debug-probe register (stored qword_140F1A7C8) [GPUMon init/teardown].
/// In gpumoncmd.exe this SAME ID is additionally the per-call ENTER profiling hook (wrapped around every API call).
Unknown_33C7F5EC = 0x33c7f5ec,
/// `Unknown_34249506` — GPUHandle::setTgpPercent sub-call - client power policy
Unknown_34249506 = 0x34249506,
/// `Unknown_3CC2D181` — GPUHandle::pollFanSpeed - ClientFanCoolersGetStatus
Unknown_3CC2D181 = 0x3cc2d181,
/// `Unknown_41B2CA9A` — GPUHandle::pollPsiGr/pollPsiMs - PSI power-saving-idle support info
Unknown_41B2CA9A = 0x41b2ca9a,
/// `Unknown_42AFA9CA` — GPUHandle::queryFrameBuffer sub-call - FB/VRAM info
Unknown_42AFA9CA = 0x42afa9ca,
/// `Unknown_4324694C` — DriverInvoker::populateNvpcfHandle - get NvPCF platform handle (GPUMon_Requester_)
Unknown_4324694C = 0x4324694c,
/// `Unknown_45EFAB64` — Connector::populateAcpiId - display mapping ID for ACPI
Unknown_45EFAB64 = 0x45efab64,
/// `Unknown_470D2D63` — GPUHandle::queryFrameBuffer - physical frame buffer size
Unknown_470D2D63 = 0x470d2d63,
/// `Unknown_48E0847D` — GPUHandle::setDNotifyLimit - set extern power state (D1-D5 D0-notify)
Unknown_48E0847D = 0x48e0847d,
/// `Unknown_48F421C4` — Connector::getSbiosBrightnessInfo - SBIOS brightness info
Unknown_48F421C4 = 0x48f421c4,
/// `Unknown_57B5A5DF` — GPUHandle::queryClockDomainIndex - clock domain info (GPC domain)
Unknown_57B5A5DF = 0x57b5a5df,
/// `Unknown_57FA8E2C` — GPUHandle::queryFrameBuffer sub-call - frame-buffer/VRAM query
Unknown_57FA8E2C = 0x57fa8e2c,
/// `Unknown_594762E4` — sub_140001060 init: debug-probe unregister (stored qword_140F1A7D0) [GPUMon init/teardown].
/// In gpumoncmd.exe this SAME ID is additionally the per-call EXIT profiling hook (paired with Unknown_33C7F5EC).
Unknown_594762E4 = 0x594762e4,
/// `Unknown_595E3EF6` — Connector::doLinkTraining/getCurrentLinkConfig - set/get display link config
Unknown_595E3EF6 = 0x595e3ef6,
/// `Unknown_638CD19C` — GPUHandle::queryDrKey - PCIEPowerControl / DR-key
Unknown_638CD19C = 0x638cd19c,
/// `Unknown_65CE5BFC` — GPUHandle::pollFanSpeed/setFanSim - ClientFanCoolersGetInfo
Unknown_65CE5BFC = 0x65ce5bfc,
/// `Unknown_661AA3AF` — GPUHandle::pollSlowdown - slowdown amount read
Unknown_661AA3AF = 0x661aa3af,
/// ClientPowerPoliciesGetInfo — PRIVATE variant (NDA, ID 0x67F31384). Returns a
/// ~347KB policy-descriptor struct (the TGP policy table). NOT the same as the
/// public `0x34206D86`. GPUMon `GPUHandle::queryPowerPolicy` (sub_1400304B0)
/// uses this to fetch the TGP-watts min/default/max range. Buffer = 86784 dwords
/// (347136 B), version magic 0x0F4BF4; per-policy entry stride 2651 dwords
/// (10604 B); policy-table selector index at byte offset 0x14 (v7[5] low byte,
/// default 2 if 0xFF); per-entry min/default/max mW at entry dword +275/+276/+277.
NvAPI_GPU_ClientPowerPoliciesGetInfoPrivate = 0x67f31384,
/// `Unknown_69043B70` — DriverInvoker::populateBB2TppLimit / GPUHandle::pollCtac - system battery / CTAC thermal-zone data
Unknown_69043B70 = 0x69043b70,
/// `Unknown_6FFA5633` — GPUHandle::queryOverClocking - over-clocking capability info
Unknown_6FFA5633 = 0x6ffa5633,
/// `Unknown_73030846` — GPUHandle::setCpuClock - change CPU max frequency limit
Unknown_73030846 = 0x73030846,
/// `Unknown_7977A946` — DriverInvoker::populatePowerLimitTable - SBIOS power-limit table
Unknown_7977A946 = 0x7977a946,
/// `Unknown_799D6E11` — GPUHandle::queryFrameBuffer sub-call - FB/VRAM info
Unknown_799D6E11 = 0x799d6e11,
/// `Unknown_7A2D309E` — DriverInvoker::getBoostClock - get PM1 availability (boost-clock status)
Unknown_7A2D309E = 0x7a2d309e,
/// `Unknown_7B30AE0D` — GPUHandle::queryPStateInfo - Perf P-states info
Unknown_7B30AE0D = 0x7b30ae0d,
/// `Unknown_7BF85571` — aggregator sub-call (used by pollCtac @0x14002c990)
Unknown_7BF85571 = 0x7bf85571,
/// `Unknown_7DBF2D2B` — GPUHandle::queryArchitecture sub-call - system/GPU identity
Unknown_7DBF2D2B = 0x7dbf2d2b,
/// TGP-watts power-control GET (NDA, ID 0x8B3E7343). Fills the 10016-byte
/// read-modify-write buffer used by setTgpWatt (GPUMon sub_1400324A0). Paired
/// with NvAPI_GPU_ClientTgpWattSetStatus. Struct version magic 0x12720 (v1|10016).
NvAPI_GPU_ClientTgpWattGetStatus = 0x8b3e7343,
/// `Unknown_8C45954D` — GPUHandle::getCpuClockRange - read CPU frequency range
Unknown_8C45954D = 0x8c45954d,
/// `Unknown_93456591` — GPUHandle::pollWhisperMode - NVPCF status (WM2.0 whisper mode)
Unknown_93456591 = 0x93456591,
/// `Unknown_95E71AB6` — GPUHandle::setTempSim/disableTempSim - temperature simulation (VBIOS Secured Overrides)
Unknown_95E71AB6 = 0x95e71ab6,
/// `Unknown_9962C97C` — GPUHandle::pollPState - P-state limit status
Unknown_9962C97C = 0x9962c97c,
/// `Unknown_99FC9866` — Connector::getPanelBrightnessInfo - panel brightness info
Unknown_99FC9866 = 0x99fc9866,
/// `Unknown_A5614A5D` — GPUHandle::queryGPUInfo sub-call - GPU info
Unknown_A5614A5D = 0xa5614a5d,
/// `Unknown_ADE08E5F` — sub_140001060 init: resolves a fn ptr (stored qword_140F1A7A8), likely NvAPI_PrivateInit variant [GPUMon init/teardown]
Unknown_ADE08E5F = 0xade08e5f,
/// `Unknown_AF97FE75` — GPUHandle::pollTempSim - read temperature-simulation status
Unknown_AF97FE75 = 0xaf97fe75,
/// `Unknown_B0031005` — GPUHandle::queryArchitecture sub-call - identity
Unknown_B0031005 = 0xb0031005,
/// `Unknown_B4C5D8BA` — DriverInvoker::populateQboostIndex - QBoost controller info
Unknown_B4C5D8BA = 0xb4c5d8ba,
/// `Unknown_B6A3DA5B` — GPUHandle::setDynamicBoost - set controller setting
Unknown_B6A3DA5B = 0xb6a3da5b,
/// `Unknown_B78734AB` — GPUHandle::pollDynamicBoost - QBoost controller status
Unknown_B78734AB = 0xb78734ab,
/// TGP-watts power-control SET (NDA, ID 0xBFF09E59). Applies the 10016-byte
/// buffer (with the target mW written at dword 553+10*index). GPUMon
/// `[GPUHandle::setTgpWatt]` writes watts→mW (×1000) and range-checks against
/// GetInfoPrivate's min/max. 0xFFFFFFFF = reset to rated/default.
NvAPI_GPU_ClientTgpWattSetStatus = 0xbff09e59,
/// `Unknown_C4554575` — GPUHandle::setTargetTemperature - set thermal control
Unknown_C4554575 = 0xc4554575,
/// `Unknown_C74BFB78` — DriverInvoker::getThermalController/setThermalController - thermal controller enable/disable
Unknown_C74BFB78 = 0xc74bfb78,
/// `Unknown_C9F86A33` — GPUHandle::setPState/setRatedTdp - PerfClientLimitsSetStatus
Unknown_C9F86A33 = 0xc9f86a33,
/// `Unknown_CF0AB99F` — GPUHandle::queryGPUInfo sub-call - GPU info
Unknown_CF0AB99F = 0xcf0ab99f,
/// `Unknown_CF86B990` — GPUHandle::pollFanSpeed/setFanSim - ClientFanCoolersGetControl
Unknown_CF86B990 = 0xcf86b990,
/// `Unknown_D2561B69` — GPUHandle::setBb2Active/setWm2Active - enable BB2/WM2
Unknown_D2561B69 = 0xd2561b69,
/// `Unknown_D8135264` — GPUHandle::queryArchitecture sub-call - identity
Unknown_D8135264 = 0xd8135264,
/// `Unknown_E262027C` — DriverInvoker::setBoostClock - set PM1 availability
Unknown_E262027C = 0xe262027c,
/// `Unknown_E415C04E` — DriverInvoker::populateNvpcfMasterInfo/Index - NvPCF master info
Unknown_E415C04E = 0xe415c04e,
/// `Unknown_E4427527` — GPUHandle::isPStateLocked sub-call - client limit
Unknown_E4427527 = 0xe4427527,
/// `Unknown_E642352B` — GPUHandle::isPStateLocked - PerfClientLimitsGetInfo
Unknown_E642352B = 0xe642352b,
/// `Unknown_E64AE812` — GPUHandle::pollRppgMs - RPPG (SRAM low-power) support info
Unknown_E64AE812 = 0xe64ae812,
/// `Unknown_EB44E8AA` — GPUHandle::setFanSim - ClientFanCoolersSetControl
Unknown_EB44E8AA = 0xeb44e8aa,
/// `Unknown_EFCE7A2F` — GPUHandle::isPStateLocked sub-call - limit status
Unknown_EFCE7A2F = 0xefce7a2f,
/// `Unknown_F576F5CF` — Connector::populateDisplayName - parsed EDID / display name
Unknown_F576F5CF = 0xf576f5cf,
/// `Unknown_F9E92A44` — GPUHandle::pollPowerState - power supply state (AC) read
Unknown_F9E92A44 = 0xf9e92a44,

// ---------------------------------------------------------------------------
// source: gpumon.exe + gpumoncmd.exe — second extraction pass (complete surface)
//
// These IDs come from a FULL re-extraction of BOTH GPUMon binaries' nvapi
// QueryInterface surface (authoritative: xref walk of the cached QI pointer
// in each binary — GPUMon.exe 124 distinct IDs @ qword_140F1A7B8, GPUMonCmd.exe
// 100 distinct IDs @ qword_1400AA948; union = 127). The earlier gpumon.exe
// block above was hand-built from a PARTIAL extraction and missed these; the
// complete walk plus the previously-unreversed gpumoncmd.exe surfaced 22 IDs
// absent from this registry. See reverse/full_surface_final.json for the
// per-binary presence table and reverse/gpumoncmd-nvapi-extract.md for the
// gpumoncmd-specific pass. Each name below is grounded in the GPUMon
// `[GPUHandle::method]` / `[DriverInvoker::method]` log string of the thunk's
// caller (re-verified via IDA MCP this pass), not guessed from the ID.

/// `Unknown_0956AB25` — GPUHandle::pollFanArbiter - ClientFanArbiter GetStatus (fan arbiter status)
Unknown_0956AB25 = 0x0956ab25,
/// `Unknown_1B71D425` — GPUHandle::setThermalSlowdown - ClientThermalSlowdownSetStatus (enable/disable 0xFFFF)
Unknown_1B71D425 = 0x1b71d425,
/// `Unknown_2B2A2A45` — GPUHandle::resetFanCurve - ClientFanCoolersPolicy SetStatus (apply new fan policy)
Unknown_2B2A2A45 = 0x2b2a2a45,
/// `Unknown_2EB3C140` — GPUHandle::queryArchitecture - system/GPU-type query (multi-caller, architecture init)
Unknown_2EB3C140 = 0x2eb3c140,
/// `Unknown_31B7A4CD` — GPUHandle::pollPowerPolicy - ClientPowerPoliciesGetStatus (power policy status)
Unknown_31B7A4CD = 0x31b7a4cd,
/// `Unknown_3B421EF9` — GPUHandle::pollPcieBandwidth - NVPCF status data (PCIE Rx/Tx bandwidth)
Unknown_3B421EF9 = 0x3b421ef9,
/// `Unknown_5D0634EE` — GPUHandle::pollVoltage - ClientVoltRailsGetStatus (voltage rail DATA, magics 68296/68300)
Unknown_5D0634EE = 0x5d0634ee,
/// `Unknown_7CAAC987` — GPUHandle::pollDifrLayer1/2/3 - DIFR power-gating support/statistics
Unknown_7CAAC987 = 0x7caac987,
/// `Unknown_7DBE90AB` — GPUHandle::queryArchitecture - system/GPU-type query
Unknown_7DBE90AB = 0x7dbe90ab,
/// `Unknown_AFF54A75` — GPUHandle::queryArchitecture - arch info query
Unknown_AFF54A75 = 0xaff54a75,
/// `Unknown_AFFC2279` — earlier naming map attributed this to `setTgpWatt`, but
/// that is WRONG: the setTgpWatt SET is 0xBFF09E59 (NvAPI_GPU_ClientTgpWattSetStatus,
/// verified by direct decompile of sub_1400324A0). This ID is not loaded as an
/// immediate anywhere in GPUMon.exe's .text; its true role is unconfirmed.
Unknown_AFFC2279 = 0xaffc2279,
/// `Unknown_C118ED82` — GPUHandle::pollGc6Statistics - GC6 (link-off) residency statistics
Unknown_C118ED82 = 0xc118ed82,
/// `Unknown_C9E9BB33` — GPUHandle::setPState - PerfClientLimitsSetStatus (P-State/frequency lock)
Unknown_C9E9BB33 = 0xc9e9bb33,
/// `Unknown_E097144F` — GPUHandle::setTargetTemperature - ClientThermalPoliciesSetStatus (target temp)
Unknown_E097144F = 0xe097144f,
/// `Unknown_E63AE22B` — GPUHandle::isPStateLocked - PerfClientLimitsGetInfo (client limit info)
Unknown_E63AE22B = 0xe63ae22b,
/// `Unknown_E65C75B2` — GPUHandle::pollRppgMs - RPPG (SRAM low-power) support status
Unknown_E65C75B2 = 0xe65c75b2,
/// `Unknown_EFCEDD1F` — GPUHandle::isPStateLocked - PerfClientLimitsGetStatus (client limit status)
Unknown_EFCEDD1F = 0xefcedd1f,
/// `Unknown_F9D60904` — GPUHandle::pollPowerState - power supply state (AC/DC) read
Unknown_F9D60904 = 0xf9d60904,

// --- gpumoncmd.exe-only IDs (NOT present in GPUMon.exe) ---
/// `Unknown_01510308` — gpumoncmd.exe init stub - private init/enum resolver (cached @ 0x1400AA940)
Unknown_01510308 = 0x01510308,
// NOTE: 0x33C7F5EC and 0x594762E4 are already registered above (GPUMon init
// debug-probe). In gpumoncmd.exe these SAME two IDs are additionally used as
// per-call ENTER/EXIT profiling hooks wrapping every API call — see the
// existing entries at Unknown_33C7F5EC / Unknown_594762E4 above.

// --- GPUMon.exe init-stub lifecycle resolvers (resolved at LoadLibrary time,
// cached in qword_140F1A7A8/A7B0/A7C8/A7D0; not per-frame thunks). Kept as
// documentation-only records so the IIDs are reserved/known. ---
/// `Unknown_AD298D3F` — GPUMon.exe init - primary lifecycle resolver (QI at init, cached @ qword_140F1A7A8)
Unknown_AD298D3F_LifecycleInit = 0xad298d3f,
/// `Unknown_33C7358C` — GPUMon.exe init - secondary lifecycle resolver (cached @ qword_140F1A7C8)
Unknown_33C7358C_LifecycleInit = 0x33c7358c,
/// `Unknown_593E8644` — GPUMon.exe init - secondary lifecycle resolver (cached @ qword_140F1A7D0)
Unknown_593E8644_LifecycleInit = 0x593e8644,

}
