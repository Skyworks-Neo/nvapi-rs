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
/// YOFOO alias: `NvAPI_GPU_ManufacturingInfo` (same ID 0xa4218928; name kept as RE'd above).
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
/// YOFOO alias: `NvAPI_GPU_GetVoltageDomainsInfo` (same ID 0x28766157). Our wrapper
/// reads it as NV_VOLT_STATUS (voltage-step semantics); YOFOO's name is broader.
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

/// YOFOO alias: `NvAPI_GPU_GetDynamicPstatesInfo` (same ID 0x189a1fdf; name kept as RE'd above).
NvAPI_GPU_GetUsages = 0x189a1fdf,

/// YOFOO alias: `NvAPI_GPU_GetRamVendorID` (same ID 0x42aea16a; name kept as RE'd above).
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
NvAPI_Coproc_GetCoprocInfo = 0x1629a173,
/// `NvAPI_DISP_GetAssociatedNvidiaGpuHandle(hDisplay, *phGpu)` — display→GPU handle
/// resolver. Reversed from nvapi64_impl.dll handler @0x180181F10 (trampoline
/// sub_1801013F0, 2 args). Prototype: `__int64 f(NvDisplayHandle, NvPhysicalGpuHandle*)`.
/// Validates (hDisplay & 0xFF000000)==0xDE000000 (display-handle magic); non-matching
/// non-zero => -310 EXPECTED_DISPLAY_HANDLE; hDisplay==0 allowed.
/// RM: sub_180389320(ioctl=0x07000061, &esc52, sz=0x34(52), hDisplay, ...); GPU handle
/// read from HIDWORD(esc[3]) (escape offset 0x1C). One-shot handle lookup.
/// Output: *phGpu = GPU handle. Display/handle plumbing — no sensor data. Wrap only if
/// nvoc builds display-side topology; otherwise skip.
NvAPI_DISP_GetLogicalCudaGPUFromDisplay = 0xf1d2777b,
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
NvAPI_GetPhysicalGPUsFromLogicalGPUInEngineOrder = 0x8efc0978,
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
/// YOFOO alias: `NvAPI_GPU_QueryComputeCaps` (same ID 0xb7bcf50d; name kept as RE'd above).
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
NvAPI_CUDA_EnumComputeCapableByTopology = 0x36e39e6b,
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
NvAPI_GPU_GetRasterBackendCount = 0xfdc129fa,

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

// source: gpumon.exe (NVIDIA OEM partner tool, reverse/the ref tool/the ref-tool GUI)
//
// These IDs were discovered by extracting the ref-tool GUI's complete
// `nvapi_QueryInterface` surface (128 call sites on the cached
// qword_140F1A7B8 pointer; see reverse/gpumon-raw-id-table.md) and naming
// each via its caller's labeled `GPUHandle::*` / `DriverInvoker::*` /
// `Connector::*` method (the ref tool embeds the method name in its own
// `[Class::method] NvAPI fail to ...` log strings). They are NDA /
// undocumented IDs NOT present in NVIDIA's public interface table.
//
// Names use the `Unknown_<HEX>` convention (we do NOT assert a public
// NvAPI_* name unless independently confirmed); the doc comment records
// the the ref tool method + role as the evidence trail. id_hex values are
// re-verified from fresh decompiles (the phase-1 extraction misread a
// few probe-wrapper literals). See reverse/gpumon-id-catalog-for-review.md
// for the prioritized wrap list.

/// `Unknown_845866AD` — GPUHandle::pollPcieErrorCount - PCIe link error COUNT (new NDA, !=GetPCIEInfo)
Unknown_845866AD = 0x845866ad,
/// `NvAPI_GPU_PowerDeviceGetInfo` — GPUHandle::queryPowerDevice - GetPowerSensorInfo (power-rail topology descriptor, 32-ch INA/Internal/OVR-M)
NvAPI_GPU_PowerDeviceGetInfo = 0xdb9ed906,
/// `Unknown_5D1D3A4E` — GPUHandle::pollVoltage - voltage rail info (ClientVoltRailsGetInfo)
Unknown_5D1D3A4E = 0x5d1d3a4e,
/// `NvAPI_GPU_VoltVoltRailsGetInfo` — GPUHandle::pollVoltage - voltage rail data (ClientVoltRailsGetStatus)
NvAPI_GPU_VoltVoltRailsGetInfo = 0x2c73afdc,
/// `Unknown_3B51F399` — GPUHandle::pollPcieBandwidth - NVPCF status data (PCIE Rx/Tx bandwidth)
Unknown_3B51F399 = 0x3b51f399,
/// `NvAPI_Coproc_GetGoldStatistics` — GPUHandle::pollGcOffStatistics - GCOFF statistics (new NDA)
NvAPI_Coproc_GetGoldStatistics = 0x083629b7,
/// `NvAPI_GPU_LpwrPgGetStatistics` — GPUHandle::pollDifrLayer1/2/3 - DIFR power-gating residency statistics
NvAPI_GPU_LpwrPgGetStatistics = 0xf39c1def,
/// `NvAPI_GPU_LpwrRppgGetStatistics` — GPUHandle::pollRppgMs - RPPG residency statistics
NvAPI_GPU_LpwrRppgGetStatistics = 0xa4e81b74,
/// `NvAPI_GPU_LpwrPsiGetStatistics` — GPUHandle::pollPsiGr/pollPsiMs - PSI residency statistics
NvAPI_GPU_LpwrPsiGetStatistics = 0x5726c144,
/// `Unknown_7C95F2D7` — GPUHandle::pollDifrLayer1/2/3 - DIFR power-gating support info
Unknown_7C95F2D7 = 0x7c95f2d7,
/// `Unknown_0078E2A2` — GPUHandle::queryConnectedDisplay - connected display count
Unknown_0078E2A2 = 0x0078e2a2,
/// `NvAPI_Coproc_GetCoprocInfoEx` — aggregator sub-call (used by pollCtac @0x14002c990)
NvAPI_Coproc_GetCoprocInfoEx = 0x019185be,
/// `NvAPI_GPU_FanPolicyGetControl` — GPUHandle::resetFanCurve - ClientFanPoliciesSetStatus
NvAPI_GPU_FanPolicyGetControl = 0x0fe87b7f,
/// `NvAPI_PCF_MasterGetInfo` — DriverInvoker::populateChipsetInfo - chipset id info
NvAPI_PCF_MasterGetInfo = 0x1071e0d3,
/// `NvAPI_GPU_FanArbiterGetInfo` — GPUHandle::pollFanArbiter - ClientFanArbiters info/status
NvAPI_GPU_FanArbiterGetInfo = 0x10741a55,
/// PPAB / Dynamic-Boost ENABLE setter (NDA-private, ID 0x1504FC3D).
/// Raw `u8`/BoolU32 active flag (0=disable, non-zero=enable); NOT a *const
/// struct setStatus. Proven by decompile: the ref-tool GUI thunk 0x140006E60,
/// caller 0x140030D20 logs `[GPUHandle::setDynamicBoost] active: %d`; the the ref tool
/// CLI handler (`-db`, `[CmdDispatch::cmdDynamicBoost]`) passes
/// `active=(int!=0)` and prints `Change dynamic boost controlling state to
/// [enable|disable] successful`. Matches the "PPAB Enable" checkbox on the
/// Dynamic-Boost tab of OEM partner tools.
/// NOTE: an earlier naming pass mislabeled this `Unknown_1504FC3D/setTgpQboost`
/// and mislabeled 0xB6A3DA5B as `setDynamicBoost` — 0xB6A3DA5B is actually
/// `[DriverInvoker::populatePowerLimitTable]` (a SBIOS power-limit-table GET).
/// YOFOO alias: `NvAPI_PCF_DynamicBoostSetStatus` (same ID 0x1504fc3d; name kept as RE'd above).
NvAPI_GPU_ClientDynamicBoostSetStatus = 0x1504fc3d,
/// `Unknown_1B778765` — GPUHandle::setThermalSlowdown - change slowdown
Unknown_1B778765 = 0x1b778765,
/// `Unknown_2A03BCCF` — GPUHandle::queryPciInfo sub-call - PCI info
Unknown_2A03BCCF = 0x2a03bccf,
/// `Unknown_2EB86EE0` — Connector::pollGpuAspm - read register data (ASPM L0s/L1)
Unknown_2EB86EE0 = 0x2eb86ee0,
/// `NvAPI_GPU_ClientThermalPoliciesPrivateGetInfo` — GPUHandle::queryTargetTemperature
/// - thermal policy info. PRIVATE sibling of the documented
/// ClientThermalPoliciesGetInfo (0x0D258BB5); the ref tool resolves THIS one
/// (0x2F69F8E5) with version magic 0x33D38 (~15.7 KB). Returns the packed
/// target-temp policy index (GPS lobte, acoustics byte1 fallback) + VBIOS
/// min/default/max range. Renamed from Unknown_2F69F8E5.
/// YOFOO alias: `NvAPI_GPU_ThermalPolicyGetInfo` (same ID 0x2f69f8e5; name kept as RE'd above).
NvAPI_GPU_ClientThermalPoliciesPrivateGetInfo = 0x2f69f8e5,
/// `Unknown_31B855CD` — GPUHandle::pollPowerPolicy - power policy status read
Unknown_31B855CD = 0x31b855cd,
/// `NvAPI_GPU_GetSKUInfo` — GPUHandle::queryGPUInfo sub-call - GPU info
NvAPI_GPU_GetSKUInfo = 0x32464c6c,
/// `NvAPI_GPU_PerfLimitsSetStatus` — GPUHandle::setGpcClock - limit perf frequency
NvAPI_GPU_PerfLimitsSetStatus = 0x32ca4983,
/// `Unknown_33C7F5EC` — sub_140001060 init: debug-probe register (stored qword_140F1A7C8) [the ref tool init/teardown].
/// In gpumoncmd.exe this SAME ID is additionally the per-call ENTER profiling hook (wrapped around every API call).
Unknown_33C7F5EC = 0x33c7f5ec,
/// `Unknown_34249506` — GPUHandle::setTgpPercent sub-call - client power policy
Unknown_34249506 = 0x34249506,
/// `NvAPI_GPU_FanCoolerGetStatus` — GPUHandle::pollFanSpeed - ClientFanCoolersGetStatus
NvAPI_GPU_FanCoolerGetStatus = 0x3cc2d181,
/// `NvAPI_GPU_LpwrPsiGetSupport` — GPUHandle::pollPsiGr/pollPsiMs - PSI power-saving-idle support info
NvAPI_GPU_LpwrPsiGetSupport = 0x41b2ca9a,
/// `Unknown_42AFA9CA` — GPUHandle::queryFrameBuffer sub-call - FB/VRAM info
Unknown_42AFA9CA = 0x42afa9ca,
/// `Unknown_4324694C` — DriverInvoker::populateNvpcfHandle - get NvPCF platform handle (the ref tool_Requester_)
Unknown_4324694C = 0x4324694c,
/// `NvAPI_SYS_GetACPIIdMappings` — Connector::populateAcpiId - display mapping ID for ACPI
NvAPI_SYS_GetACPIIdMappings = 0x45efab64,
/// `Unknown_470D2D63` — GPUHandle::queryFrameBuffer - physical frame buffer size
Unknown_470D2D63 = 0x470d2d63,
/// D-Notifier (D0-notify / "extern power state") SETTER (NDA-private,
/// ID 0x48E0847D). Raw two-arg call `(hPhysicalGPU, level: u32)` — NO struct
/// buffer. RE'd from the ref tool `[GPUHandle::setDNotifyLimit]` (the ref-tool CLI thunk
/// sub_140001780, `nvapi_QueryInterface(1222673533)`); its caller's switch maps
/// CLI `-dnotifier:<1..5>` to the signed level code `-1,0,1,2,3` (D1=Unlimited,
/// D2..D5). The matching GET is `NvAPI_GPU_ClientPowerPoliciesGetInfoPrivate`
/// (0x67F31384), which exposes the active D level + per-D mW cap table.
/// YOFOO alias: `NvAPI_GPU_SetExternPowerState` (same ID 0x48e0847d; name kept as RE'd above).
NvAPI_GPU_ClientExternPowerStateSet = 0x48e0847d,
/// `NvAPI_DISP_GetSbiosBrightnessInfo` — Connector::getSbiosBrightnessInfo - SBIOS brightness info
NvAPI_DISP_GetSbiosBrightnessInfo = 0x48f421c4,
/// `NvAPI_GPU_ClockClkDomainsGetInfo` — GPUHandle::queryClockDomainIndex - clock domain info (GPC domain)
NvAPI_GPU_ClockClkDomainsGetInfo = 0x57b5a5df,
/// `Unknown_57FA8E2C` — GPUHandle::queryFrameBuffer sub-call - frame-buffer/VRAM query
Unknown_57FA8E2C = 0x57fa8e2c,
/// `Unknown_594762E4` — sub_140001060 init: debug-probe unregister (stored qword_140F1A7D0) [the ref tool init/teardown].
/// In gpumoncmd.exe this SAME ID is additionally the per-call EXIT profiling hook (paired with Unknown_33C7F5EC).
Unknown_594762E4 = 0x594762e4,
/// `NvAPI_DISP_DP_LinkConfiguration` — Connector::doLinkTraining/getCurrentLinkConfig - set/get display link config
NvAPI_DISP_DP_LinkConfiguration = 0x595e3ef6,
/// `NvAPI_GPU_GetNBSIParams` — GPUHandle::queryDrKey - PCIEPowerControl / DR-key
NvAPI_GPU_GetNBSIParams = 0x638cd19c,
/// `NvAPI_GPU_FanCoolerGetInfo` — GPUHandle::pollFanSpeed/setFanSim - ClientFanCoolersGetInfo
NvAPI_GPU_FanCoolerGetInfo = 0x65ce5bfc,
/// `NvAPI_GPU_ThermHwFsSlowdownAmountGet` — GPUHandle::pollSlowdown - slowdown amount read
NvAPI_GPU_ThermHwFsSlowdownAmountGet = 0x661aa3af,
/// ClientPowerPoliciesGetInfo — PRIVATE variant (NDA, ID 0x67F31384). Returns a
/// ~347KB policy-descriptor struct (the TGP policy table). NOT the same as the
/// public `0x34206D86`. the ref tool `GPUHandle::queryPowerPolicy` (sub_1400304B0)
/// uses this to fetch the TGP-watts min/default/max range. Buffer = 86784 dwords
/// (347136 B), version magic 0x0F4BF4; per-policy entry stride 2651 dwords
/// (10604 B); policy-table selector index at byte offset 0x14 (v7[5] low byte,
/// default 2 if 0xFF); per-entry min/default/max mW at entry dword +275/+276/+277.
/// YOFOO alias: `NvAPI_GPU_PowerPolicyGetInfo` (same ID 0x67f31384; name kept as RE'd above).
NvAPI_GPU_ClientPowerPoliciesGetInfoPrivate = 0x67f31384,
/// `Unknown_69043B70` — DriverInvoker::populateBB2TppLimit / GPUHandle::pollCtac - system battery / CTAC thermal-zone data
Unknown_69043B70 = 0x69043b70,
/// `Unknown_6FFA5633` — GPUHandle::queryOverClocking - over-clocking capability info
Unknown_6FFA5633 = 0x6ffa5633,
/// `Unknown_73030846` — GPUHandle::setCpuClock - change CPU max frequency limit
Unknown_73030846 = 0x73030846,
/// `NvAPI_PCF_SystemBatteryGetStatus` — DriverInvoker::populatePowerLimitTable - SBIOS power-limit table
NvAPI_PCF_SystemBatteryGetStatus = 0x7977a946,
/// `Unknown_799D6E11` — GPUHandle::queryFrameBuffer sub-call - FB/VRAM info
Unknown_799D6E11 = 0x799d6e11,
/// `NvAPI_GPS_GetPM1Available` — DriverInvoker::getBoostClock - get PM1 availability (boost-clock status)
NvAPI_GPS_GetPM1Available = 0x7a2d309e,
/// Perf P-states info table (NDA-private, ID 0x7B30AE0D). RE'd from the ref tool
/// `[GPUHandle::queryPStateInfo]` (thunk sub_140003A20). Returns a 275152-byte
/// struct (magic 0x432D0 = version 4 | size); the source of the ref tool's
/// `-pstate` GET "Level[N] P*.Max/P*.Min" table. Layout (byte offsets from the
/// version dword): valid-pstate bitmask @ 0x88 (dword 34); version @ 0x8C
/// (dword 35 low byte); a slot table (one pstate_idx per present pstate) base
/// 0x2114, stride 0x2090; and a freq table indexed BY pstate number with
/// min_kHz @ 0x22C8 / max_kHz @ 0x22F0, stride 0x9C. See
/// NV_GPU_PERF_PSTATES_INFO_PRIVATE accessors for the decoded view.
/// YOFOO alias: `NvAPI_GPU_PerfPstatesGetInfo` (same ID 0x7b30ae0d; name kept as RE'd above).
NvAPI_GPU_PerfPstatesGetInfoPrivate = 0x7b30ae0d,
/// `NvAPI_Diag_GetGC6DebugInfo` — aggregator sub-call (used by pollCtac @0x14002c990)
NvAPI_Diag_GetGC6DebugInfo = 0x7bf85571,
/// `Unknown_7DBF2D2B` — GPUHandle::queryArchitecture sub-call - system/GPU identity
Unknown_7DBF2D2B = 0x7dbf2d2b,
/// TGP-watts power-control GET (NDA, ID 0x8B3E7343). Fills the 10016-byte
/// read-modify-write buffer used by setTgpWatt (the ref tool sub_1400324A0). Paired
/// with NvAPI_GPU_ClientTgpWattSetStatus. Struct version magic 0x12720 (v1|10016).
/// YOFOO alias: `NvAPI_GPU_PowerPolicyGetControl` (same ID 0x8b3e7343; name kept as RE'd above).
NvAPI_GPU_ClientTgpWattGetStatus = 0x8b3e7343,
/// `NvAPI_SYS_ClientRegisterForJpacControlUpdates` — GPUHandle::getCpuClockRange - read CPU frequency range
NvAPI_SYS_ClientRegisterForJpacControlUpdates = 0x8c45954d,
/// `NvAPI_PCF_ControllerGetControl` — GPUHandle::pollWhisperMode - NVPCF status (WM2.0 whisper mode)
NvAPI_PCF_ControllerGetControl = 0x93456591,
/// `NvAPI_GPU_SetExtendedThermalSimulationMode` — GPUHandle::setTempSim/disableTempSim - temperature simulation (VBIOS Secured Overrides)
NvAPI_GPU_SetExtendedThermalSimulationMode = 0x95e71ab6,
/// P-State limit status (NDA-private, ID 0x9962C97C). RE'd from the ref tool's
/// `[GPUHandle::pollPState]` "get p state limit" branch. Returns the list of
/// P-States currently locked by `NvAPI_GPU_PerfClientLimitsSetStatus`
/// (0x39442CFB). 164-byte struct `{version: 0x10088, count:u32, entries[]}`
/// where each entry is 2 bytes `{type:u8, pstate:u8}`; type==0x1A marks a
/// locked pstate (the ref tool renders the locked set as "P0.P3.P5"). Distinct from
/// the current-pstate query (GetCurrentPstate 0x927DA4F6) and from the full
/// PerfClientLimits status (0xE440B867, 780B) — this is the lightweight
/// "which pstates are locked" view.
/// YOFOO alias: `NvAPI_GPU_GetPstateActiveLimits` (same ID 0x9962c97c; name kept as RE'd above).
NvAPI_GPU_ClientPStateLimitStatus = 0x9962c97c,
/// `NvAPI_DISP_GetPanelBrightnessInfo` — Connector::getPanelBrightnessInfo - panel brightness info
NvAPI_DISP_GetPanelBrightnessInfo = 0x99fc9866,
/// `Unknown_A5614A5D` — GPUHandle::queryGPUInfo sub-call - GPU info
Unknown_A5614A5D = 0xa5614a5d,
/// `Unknown_ADE08E5F` — sub_140001060 init: resolves a fn ptr (stored qword_140F1A7A8), likely NvAPI_PrivateInit variant [the ref tool init/teardown]
Unknown_ADE08E5F = 0xade08e5f,
/// `NvAPI_GPU_GetThermalSimulationMode` — GPUHandle::pollTempSim - read temperature-simulation status
NvAPI_GPU_GetThermalSimulationMode = 0xaf97fe75,
/// `Unknown_B0031005` — GPUHandle::queryArchitecture sub-call - identity
Unknown_B0031005 = 0xb0031005,
/// QBoost controller GET info (NDA, ID 0xB4C5D8BA). Returns a 6300-byte
/// controller table (version 0x1189C = v1|6300), 196 bytes/controller, up to
/// 32 controllers. the ref tool `[DriverInvoker::populateQboostIndex]` scans it to
/// find the active controller. ID kept for QI record only — NOT wrapped: the
/// QBoost controller turned out NOT to be the PPAB-paired power slider (wrong
/// path), so the nvoc surface was removed. The real mobile TGP path is the
/// TGP-watt triplet (0x8B3E7343 / 0xAFFC2279) wrapped as set-tgp-watt.
/// YOFOO alias: `NvAPI_PCF_ControllerGetInfo` (same ID 0xb4c5d8ba; name kept as RE'd above).
NvAPI_GPU_ClientQboostGetInfo = 0xb4c5d8ba,
/// SBIOS power-limit-table GET (NDA, ID 0xB6A3DA5B). Earlier naming map
/// mislabeled this `setDynamicBoost`/`set controller setting` — WRONG. Its
/// caller is `[DriverInvoker::populatePowerLimitTable]` (a GET, 0x2388-byte
/// buffer); not a QBoost setter.
NvAPI_PCF_SysPwrLimitGetInfo = 0xb6a3da5b,
/// QBoost controller SET target power (NDA, ID 0xB78734AB). Earlier naming map
/// mislabeled this `pollDynamicBoost` (a READ) — WRONG. ID kept for QI record
/// only — NOT wrapped (the QBoost controller was the wrong PPAB-paired-slider
/// hypothesis; see NvAPI_GPU_ClientQboostGetInfo above).
/// YOFOO alias: `NvAPI_PCF_ControllerSetControl` (same ID 0xb78734ab; name kept as RE'd above).
NvAPI_GPU_ClientQboostSetStatus = 0xb78734ab,
/// TGP-watts power-control SET (NDA, ID 0xAFFC2279). Applies the 10016-byte
/// buffer (target mW at buf+0x8A0+40*idx, byte-verified via WinDbg). the ref tool's
/// `[GPUHandle::setTgpWatt]` writes watts→mW (×1000), range-checks against
/// GetInfoPrivate's min/max; 0xFFFFFFFF = reset. NOTE: this is the ID used by
/// the ref-tool CLI (sub_140001AC0 resolves 0xAFFC2279). The sibling the ref-tool GUI
/// uses 0xBFF09E59 at the same thunk — BOTH are the setTgpWatt SET, one per
/// binary. On RTX 4060 Laptop driver, QI(0xAFFC2279) resolves and
/// QI(0xBFF09E59) returns NULL, so 0xAFFC2279 is the live ID. (Earlier I
/// wrongly concluded 0xAFFC2279 was "not the SET" — corrected by live QI probe.)
/// YOFOO alias: `NvAPI_GPU_PowerPolicySetControl` (same ID 0xaffc2279; name kept as RE'd above).
NvAPI_GPU_ClientTgpWattSetStatus = 0xaffc2279,
/// TGP-watts power-control SET, the ref-tool GUI variant (NDA, ID 0xBFF09E59). Same
/// role as NvAPI_GPU_ClientTgpWattSetStatus above; the ref-tool GUI's thunk resolves
/// this ID. Returns NULL from QI on the RTX 4060 Laptop driver (the
/// the ref-tool CLI/the ref-tool CLI variant 0xAFFC2279 is the live one there). Kept
/// registered for completeness.
NvAPI_GPU_ClientTgpWattSetStatus_GpuMonExe = 0xbff09e59,
/// `NvAPI_GPU_ClientThermalTargetGetStatus` — GPUHandle::setTargetTemperature
/// GET-prime (NDA-private, ID 0xC4554575). The read half of the mobile temp-wall
/// (targettemp) RMW: fills a 992-byte control buffer that the caller patches then
/// SETs via NvAPI_GPU_ClientThermalTargetSetStatus. RE'd from the ref-tool CLI
/// sub_1400040A0; resolves in nvoc's process (probe-confirmed on RTX 4060 Laptop).
/// YOFOO alias: `NvAPI_GPU_ThermalPolicyGetControl` (same ID 0xc4554575; name kept as RE'd above).
NvAPI_GPU_ClientThermalTargetGetStatus = 0xc4554575,
/// `NvAPI_GPS_Ctrl` — DriverInvoker::getThermalController/setThermalController - thermal controller enable/disable
NvAPI_GPS_Ctrl = 0xc74bfb78,
/// `Unknown_C9F86A33` — GPUHandle::setPState/setRatedTdp - PerfClientLimitsSetStatus
Unknown_C9F86A33 = 0xc9f86a33,
/// `Unknown_CF0AB99F` — GPUHandle::queryGPUInfo sub-call - GPU info
Unknown_CF0AB99F = 0xcf0ab99f,
/// `NvAPI_GPU_FanCoolerGetControl` — GPUHandle::pollFanSpeed/setFanSim - ClientFanCoolersGetControl
NvAPI_GPU_FanCoolerGetControl = 0xcf86b990,
/// `NvAPI_SYS_ClientJpacSetControl` — GPUHandle::setBb2Active/setWm2Active - enable BB2/WM2
NvAPI_SYS_ClientJpacSetControl = 0xd2561b69,
/// `Unknown_D8135264` — GPUHandle::queryArchitecture sub-call - identity
Unknown_D8135264 = 0xd8135264,
/// `NvAPI_GPS_SetPM1Available` — DriverInvoker::setBoostClock - set PM1 availability
NvAPI_GPS_SetPM1Available = 0xe262027c,
/// `NvAPI_PCF_MasterGetStatus` — DriverInvoker::populateNvpcfMasterInfo/Index - NvPCF master info
NvAPI_PCF_MasterGetStatus = 0xe415c04e,
/// `Unknown_E4427527` — GPUHandle::isPStateLocked sub-call - client limit
Unknown_E4427527 = 0xe4427527,
/// `Unknown_E642352B` — GPUHandle::isPStateLocked - PerfClientLimitsGetInfo
Unknown_E642352B = 0xe642352b,
/// `Unknown_E64AE812` — GPUHandle::pollRppgMs - RPPG (SRAM low-power) support info
Unknown_E64AE812 = 0xe64ae812,
/// `NvAPI_GPU_FanCoolerSetControl` — GPUHandle::setFanSim - ClientFanCoolersSetControl
NvAPI_GPU_FanCoolerSetControl = 0xeb44e8aa,
/// `Unknown_EFCE7A2F` — GPUHandle::isPStateLocked sub-call - limit status
Unknown_EFCE7A2F = 0xefce7a2f,
/// `NvAPI_DISP_GetEdidParsed` — Connector::populateDisplayName - parsed EDID / display name
NvAPI_DISP_GetEdidParsed = 0xf576f5cf,
/// `Unknown_F9E92A44` — GPUHandle::pollPowerState - power supply state (AC) read
Unknown_F9E92A44 = 0xf9e92a44,

// ---------------------------------------------------------------------------
// source: gpumon.exe + gpumoncmd.exe — second extraction pass (complete surface)
//
// These IDs come from a FULL re-extraction of BOTH the ref tool binaries' nvapi
// QueryInterface surface (authoritative: xref walk of the cached QI pointer
// in each binary — the ref-tool GUI 124 distinct IDs @ qword_140F1A7B8, the ref-tool CLI
// 100 distinct IDs @ qword_1400AA948; union = 127). The earlier gpumon.exe
// block above was hand-built from a PARTIAL extraction and missed these; the
// complete walk plus the previously-unreversed gpumoncmd.exe surfaced 22 IDs
// absent from this registry. See reverse/full_surface_final.json for the
// per-binary presence table and reverse/gpumoncmd-nvapi-extract.md for the
// gpumoncmd-specific pass. Each name below is grounded in the the ref tool
// `[GPUHandle::method]` / `[DriverInvoker::method]` log string of the thunk's
// caller (re-verified via IDA MCP this pass), not guessed from the ID.

/// `NvAPI_GPU_FanArbiterGetStatus` — GPUHandle::pollFanArbiter - ClientFanArbiter GetStatus (fan arbiter status)
NvAPI_GPU_FanArbiterGetStatus = 0x0956ab25,
/// `NvAPI_GPU_SetThermalSlowdownState` — GPUHandle::setThermalSlowdown - ClientThermalSlowdownSetStatus (enable/disable 0xFFFF)
NvAPI_GPU_SetThermalSlowdownState = 0x1b71d425,
/// `NvAPI_GPU_FanPolicySetControl` — GPUHandle::resetFanCurve - ClientFanCoolersPolicy SetStatus (apply new fan policy)
NvAPI_GPU_FanPolicySetControl = 0x2b2a2a45,
/// `NvAPI_GPU_RegisterOp` — GPUHandle::queryArchitecture - system/GPU-type query (multi-caller, architecture init)
NvAPI_GPU_RegisterOp = 0x2eb3c140,
/// `NvAPI_GPU_PowerPolicyGetStatus` — GPUHandle::pollPowerPolicy - ClientPowerPoliciesGetStatus (power policy status)
NvAPI_GPU_PowerPolicyGetStatus = 0x31b7a4cd,
/// `NvAPI_GPU_PerfPerfCfTopologyGetStatus` — GPUHandle::pollPcieBandwidth - NVPCF status data (PCIE Rx/Tx bandwidth)
NvAPI_GPU_PerfPerfCfTopologyGetStatus = 0x3b421ef9,
/// `NvAPI_GPU_VoltVoltRailsGetStatus` — GPUHandle::pollVoltage - ClientVoltRailsGetStatus (voltage rail DATA, magics 68296/68300)
NvAPI_GPU_VoltVoltRailsGetStatus = 0x5d0634ee,
/// `NvAPI_GPU_LpwrPgGetSupport` — GPUHandle::pollDifrLayer1/2/3 - DIFR power-gating support/statistics
NvAPI_GPU_LpwrPgGetSupport = 0x7caac987,
/// `NvAPI_GPU_GetGPCMask` — GPUHandle::queryArchitecture - system/GPU-type query
NvAPI_GPU_GetGPCMask = 0x7dbe90ab,
/// `NvAPI_GPU_PerfPerfCfTopologyGetInfo` — GPUHandle::queryArchitecture - arch info query
NvAPI_GPU_PerfPerfCfTopologyGetInfo = 0xaff54a75,
// NOTE: 0xAFFC2279 is registered above as NvAPI_GPU_ClientTgpWattSetStatus
// (the setTgpWatt SET, the ref-tool CLI variant — live-verified resolvable on RTX 4060
// Laptop driver, unlike the the ref-tool GUI sibling 0xBFF09E59 which QI-returns NULL
// there). The earlier "Unknown_AFFC2279 / role unconfirmed" entry was removed.
/// `NvAPI_GPU_GetGC6Statistics` — GPUHandle::pollGc6Statistics - GC6 (link-off) residency statistics
NvAPI_GPU_GetGC6Statistics = 0xc118ed82,
/// Rated-TDP control (NDA-private, ID 0xC9E9BB33). RE'd from the ref tool's
/// `[GPUHandle::setRatedTdp]`/`[GPUHandle::clearRatedTdp]` (cmdPState index==0
/// path + the setPState preamble). NOT a P-State lock (that's
/// `NvAPI_GPU_PerfClientLimitsSetStatus` 0x39442CFB) despite an earlier naming
/// pass mislabeling it. 12-byte struct `{version: 0x1000C, dword1: 1, mode}`:
///   - mode=3 → ENABLE rated-TDP control (the "P0.TDP" level, the ref tool -pstate:0)
///   - mode=0 → DISABLE/clear (setPState calls this first to reset before
///     applying a new P-State/frequency lock via 0x39442CFB).
/// "Rated TDP" = the GPU's nominal/default power baseline.
/// YOFOO alias: `NvAPI_GPU_PerfRatedTdpSetControl` (same ID 0xc9e9bb33; name kept as RE'd above).
NvAPI_GPU_ClientRatedTdpControl = 0xc9e9bb33,
/// `NvAPI_GPU_ClientThermalTargetSetStatus` — GPUHandle::setTargetTemperature
/// SET (NDA-private, ID 0xE097144F). The write half of the mobile temp-wall
/// (targettemp): applies the 992-byte control buffer (target temp = celsius*256
/// at dword 15*idx+7). RE'd from the ref-tool CLI sub_140004170 / sub_140013090.
/// NOTE: this is NOT the phantom 0xE0765B6F (that ID resolves to NULL in every
/// process — a prior session's mis-hex-conversion of this very decimal value
/// 3767997519 caused hours of misdirection). Resolves in nvoc's process
/// (probe-confirmed: -> 0x7FFE90A12750 on RTX 4060 Laptop, = the rbx windbg saw).
/// YOFOO alias: `NvAPI_GPU_ThermalPolicySetControl` (same ID 0xe097144f; name kept as RE'd above).
NvAPI_GPU_ClientThermalTargetSetStatus = 0xe097144f,
/// `NvAPI_GPU_PerfLimitsGetInfo` — GPUHandle::isPStateLocked - PerfClientLimitsGetInfo (client limit info)
NvAPI_GPU_PerfLimitsGetInfo = 0xe63ae22b,
/// `NvAPI_GPU_LpwrRppgGetSupport` — GPUHandle::pollRppgMs - RPPG (SRAM low-power) support status
NvAPI_GPU_LpwrRppgGetSupport = 0xe65c75b2,
/// `NvAPI_GPU_PerfLimitsGetStatus` — GPUHandle::isPStateLocked - PerfClientLimitsGetStatus (client limit status)
NvAPI_GPU_PerfLimitsGetStatus = 0xefcedd1f,
/// `Unknown_F9D60904` — GPUHandle::pollPowerState - power supply state (AC/DC) read
Unknown_F9D60904 = 0xf9d60904,

// --- gpumoncmd.exe-only IDs (NOT present in the ref-tool GUI) ---
/// `Unknown_01510308` — gpumoncmd.exe init stub - private init/enum resolver (cached @ 0x1400AA940)
Unknown_01510308 = 0x01510308,
// NOTE: 0x33C7F5EC and 0x594762E4 are already registered above (the ref tool init
// debug-probe). In gpumoncmd.exe these SAME two IDs are additionally used as
// per-call ENTER/EXIT profiling hooks wrapping every API call — see the
// existing entries at Unknown_33C7F5EC / Unknown_594762E4 above.

// --- the ref-tool GUI init-stub lifecycle resolvers (resolved at LoadLibrary time,
// cached in qword_140F1A7A8/A7B0/A7C8/A7D0; not per-frame thunks). Kept as
// documentation-only records so the IIDs are reserved/known. ---
/// Private NVAPI lifecycle/controller init (NDA, ID 0xAD298D3F). the ref tool's
/// init stub (sub_140001060) resolves this and calls it as `fn(arg)` with
/// arg=1 (the "controller/client register" path) BEFORE any power-control
/// setter. Without it, the Dynamic-Boost / QBoost setters return
/// NVAPI_API_NOT_INITIALIZED. Single u32 by-value arg (NOT a per-GPU call).
NvAPI_GPU_PrivateLifecycleInit = 0xad298d3f,
/// `Unknown_33C7358C` — the ref-tool GUI init - secondary lifecycle resolver (cached @ qword_140F1A7C8)
Unknown_33C7358C_LifecycleInit = 0x33c7358c,
/// `Unknown_593E8644` — the ref-tool GUI init - secondary lifecycle resolver (cached @ qword_140F1A7D0)
Unknown_593E8644_LifecycleInit = 0x593e8644,

// ============================================================
// YOFOO nvapi key-table additions
// source: reverse/YOFOO.txt (NVIDIA api-key table, see
//         https://www.cnblogs.com/zzz3265/p/16517057.html). Bulk-
// registered for completeness; most are D3D/display/VR paths nvoc does
// not call, but reserved so this enum is a full ID registry. Names are
// the YOFOO canonical names; where a YOFOO name collided it carries a
// _HEX suffix.
// ============================================================

// --- nvCommon.spec (6 IDs) ---
    NvAPI_InitializeEx = 0x0a935ff0,
    NvAPI_GetInterfaceVersionStringEx = 0x15f64d53,
    NvAPI_GPU_GetTvEncoderType = 0x4fcb326e,
    NvAPI_GPU_GetConnectorInfoEx = 0x9f473113,
    /// YOFOO alias: `NvAPI_Initialize` (same ID 0xcb6a5d5a; name kept as RE'd above).
    NvAPI_Initialize_CB6A5D5A = 0xcb6a5d5a,
    NvAPI_UnloadEx = 0xf2f02bad,

// --- nvGpu.spec (138 IDs) ---
    NvAPI_GPU_DisableLicensedFeature = 0x0188fff8,
    NvAPI_GPU_FbSetReadLimit = 0x0518f782,
    NvAPI_GPU_GetBrandType = 0x052d0709,
    NvAPI_GetAssociatedDisplayFromOutputId = 0x070d413b,
    NvAPI_GPU_QueryDPTopology = 0x086418cd,
    NvAPI_GPU_GetEdidEx = 0x089a8e25,
    NvAPI_GetDisplayFromPhysicalGPU = 0x0c228297,
    NvAPI_GPU_GetValuesFromInstalledINF = 0x0f2400ab,
    NvAPI_GPU_GetFBCSessionsInfo = 0x1064b0e1,
    NvAPI_GPU_VdtGetInfo = 0x11a04cb1,
    NvAPI_GPU_GetInternalDisplays = 0x11fbd838,
    NvAPI_GPU_GetP2PCapsData = 0x141681e5,
    NvAPI_GPU_GetSignedGPUID = 0x15cc33b4,
    NvAPI_GPU_NVLINK_GetCounters = 0x1e284692,
    NvAPI_GetTopologyFlagString = 0x21e2cf70,
    NvAPI_GPU_SetOptimizationData = 0x25561a09,
    NvAPI_EnumPhysicalGPUsInternal = 0x264c5763,
    NvAPI_GPU_WorkstationFeatureCommit = 0x28bf0a63,
    NvAPI_GPU_VdtEntriesSetInfo = 0x2bcd70fe,
    NvAPI_GPU_GetCyaAspmInfo = 0x2d4b650c,
    NvAPI_Disp_ConstructIMPMode = 0x2fccf579,
    NvAPI_GetGpuTopologySystemPropertiesStringEx = 0x3079ac59,
    NvAPI_GPU_GetInfoROMData = 0x3526d9aa,
    NvAPI_GPU_Get_PES_INFO = 0x381d2090,
    NvAPI_GPU_GetInterruptInfo = 0x39c003a8,
    NvAPI_NVPM_CreateSharedMemory = 0x3ab6a38a,
    NvAPI_GPU_InjectECCErrors = 0x3dba65b9,
    NvAPI_QueryGpuCap = 0x3f2c5b97,
    NvAPI_ReadDisplayCrcDataEx = 0x408c2faa,
    NvAPI_SYS_EnableLicense = 0x41be3cf9,
    NvAPI_GPU_GetGPUInfoInternal = 0x41fdf198,
    NvAPI_GPU_QueryIsGRIDDisplayless = 0x4535903a,
    NvAPI_GPU_DisableD3ColdSupportOnStopDevice = 0x48924d31,
    NvAPI_DISP_GetUnAttachedDisplayFromPhysicalGPU = 0x48e085a8,
    NvAPI_GPU_HandleAELPG = 0x48ec6e45,
    NvAPI_GPU_NVLINK_ClearCounters = 0x4a0e1abc,
    NvAPI_GPU_NVLINK_GetBridgeEEPROMStatus = 0x4b698cdd,
    NvAPI_GPU_ForceGC6Exit = 0x55590cb2,
    NvAPI_NVPM_CreateGPUMapping = 0x578b6382,
    NvAPI_GPU_FbSetRrd = 0x57c9f781,
    NvAPI_GPU_GetFBCStatistics = 0x5cc38844,
    NvAPI_GPU_Get_I2C_Ports_Info = 0x5e4b36c3,
    NvAPI_DIAG_GetIsModePossibleLog = 0x6090c927,
    NvAPI_GPU_GetSLIStateInfo = 0x61c2c2aa,
    NvAPI_GetHdcpHdmiDiagnostics = 0x625d3b94,
    NvAPI_GPU_GetConnectedOutputsWithLidStateEx = 0x62fb1592,
    NvAPI_GetTopologyStatusString = 0x64f7d53c,
    NvAPI_GPU_QueryActiveAppsEx = 0x655dcd32,
    NvAPI_SYS_SetDisplayPowerSavingState = 0x680649d7,
    NvAPI_GetGpuTopologySystemProperties = 0x6a8937c6,
    NvAPI_GPU_GetOutputIdFromACPIId = 0x6a9c55fc,
    NvAPI_NVPM_ReservePerfmonHW = 0x6ae878a4,
    NvAPI_GetGpuTopologySystemPropertiesString = 0x6b8809b4,
    NvAPI_NVPM_ReserveReleasePerfmonHW = 0x74ad6bf2,
    NvAPI_GPU_GetPerGpuRegistryPath = 0x77c71d7c,
    NvAPI_GetValidGpuTopologiesInternal = 0x79ad0307,
    NvAPI_GPU_GetBridgeVersionInfo = 0x7af170cd,
    NvAPI_GPS_CallACPI = 0x7f0031ac,
    NvAPI_GetPlatformPowerMode = 0x7f8cc5c2,
    NvAPI_GPU_GetVbiosStatusString = 0x8011c22c,
    NvAPI_SYS_ACPI_GetValues = 0x80ebc01f,
    NvAPI_GPU_HandlePSI = 0x83ef3a6d,
    NvAPI_GPU_QueryGpuFlags = 0x8413ddf6,
    NvAPI_GPU_GetNetlistIdentifier = 0x857186dd,
    NvAPI_NVPM_ReleasePerfmonHW = 0x8682711b,
    NvAPI_NVPM_DestroyGPUMapping = 0x8ae4b4ca,
    NvAPI_GPU_GetUEFIInfo = 0x8c294330,
    NvAPI_GPU_GetVbiosExtractionInfo = 0x8c3a58c3,
    NvAPI_GPU_FbSetWriteLimit = 0x8d1a4910,
    NvAPI_GPU_GetVbiosSecurityInfo = 0x8d3ac6b9,
    NvAPI_GPU_GetComputePrecisionInfo = 0x8f01b624,
    NvAPI_GPU_GetVRDesktopCapability = 0x90bb9b56,
    NvAPI_GPU_ControlASPM = 0x92a5a273,
    NvAPI_GPU_GetFrameBufferDetails = 0x92ea3d02,
    NvAPI_GPU_GetSupportedSLIViews = 0x944fc548,
    NvAPI_GetSliGroupFlagString = 0x96370089,
    NvAPI_GPU_GetBusBFD = 0x9d6a8a69,
    NvAPI_GPU_SetPCIELtrInfo = 0xa104ad45,
    NvAPI_GPU_ConvertStringHashToPhysicalGpu = 0xa109a44b,
    NvAPI_GPU_GetConnectedOutputsEx = 0xa3ea6f1d,
    NvAPI_NVPM_SetPMTriggerInsert = 0xa5b31eb8,
    NvAPI_GPU_FbPatchPbrForMining = 0xa8efef61,
    NvAPI_GPU_GetPCIELinkSwitchErrorInfo = 0xa96bddf9,
    NvAPI_GPU_GetDriverModelInfo = 0xaac1002c,
    NvAPI_GPU_GetOptimizationData = 0xac80befd,
    NvAPI_GPU_GetSliMasterRegistryPath = 0xada3ef17,
    NvAPI_GPU_NVLINK_GetErrorInfo = 0xade26f6c,
    NvAPI_GPU_GetFasTelemetryInfo = 0xadf5d0db,
    NvAPI_GPS_SetWebInfo = 0xaf88030f,
    NvAPI_GPS_GetFrmData = 0xb16dc1a6,
    NvAPI_GPU_GetConnectedSLIOutputsWithLidStateEx = 0xb7476d15,
    NvAPI_NVPM_DestroySharedMemory = 0xb7fa5440,
    NvAPI_GPU_GetACPIIdFromOutputId = 0xb82c1f09,
    NvAPI_GPU_SetEDIDInternal = 0xb8416ffa,
    NvAPI_GPU_SetComputePrecision = 0xba52ed4c,
    NvAPI_GPU_GetVbiosOemInfo = 0xbca92ad5,
    NvAPI_GPU_NVLINK_ReserveCounters = 0xbcc45de1,
    NvAPI_IsModePossible = 0xbd669b00,
    NvAPI_GPU_GetZCULLMask = 0xbece67de,
    NvAPI_GPU_QueryDPTopologyEx = 0xc2ab14fd,
    NvAPI_GPU_GetHybridPadInfo = 0xc2d8ed17,
    NvAPI_GPU_GetMaxSourceCount = 0xc307636c,
    NvAPI_GPU_GetLicensableFeaturesInternal = 0xc703a5b3,
    NvAPI_GPU_GetConnectorState = 0xc78fb6ad,
    NvAPI_GPU_GetNBSIObj = 0xc96c4920,
    NvAPI_SetDisplayCrcConfigEx = 0xca193154,
    NvAPI_GPU_GetEdidEx2 = 0xd0c1de0d,
    NvAPI_GPU_GetProcessUtilizationInfo = 0xd22428e1,
    NvAPI_GPU_EnableLicensedFeature = 0xd2294406,
    NvAPI_GPU_GC6Control = 0xd387d414,
    NvAPI_GPU_SetVRDesktop = 0xd3e262e1,
    NvAPI_GPU_GetPCIELtrInfo = 0xd662113a,
    NvAPI_GPU_AccessRefCount = 0xd6979d33,
    NvAPI_GPU_SetActivationState = 0xd7f02c7d,
    NvAPI_GPU_SetDeepIdleStatisticsMode = 0xd81420bc,
    NvAPI_CUDA_PhysxSetState = 0xd875b6c5,
    NvAPI_SYS_GetDisplayPowerSavingState = 0xd8bf768c,
    NvAPI_GetGpuTopologySystemPropertiesEx = 0xd8d37032,
    NvAPI_GPU_NVLINK_GetFatalErrorsCount = 0xda6ab68b,
    NvAPI_GPU_GetVbiosProjectInfo = 0xdb66cada,
    NvAPI_I2CTransaction = 0xdd8cf250,
    NvAPI_DISP_GetTargetPhysicalGPUsFromUnAttachedDisplay = 0xe083a103,
    NvAPI_SetPlatformPowerMode = 0xe2ed2123,
    NvAPI_GPU_GetValidGpuTopologiesMosaic = 0xe35fb751,
    NvAPI_GPU_GetPostTime = 0xe515d18a,
    NvAPI_GPU_GetDisplayChangeInhibitState = 0xe76ada52,
    NvAPI_GPU_CreateStringHashFromPhysicalGpu = 0xe7bed620,
    NvAPI_GPU_QueryNodeInfo = 0xe9b009b9,
    NvAPI_GPU_GetECCErrorInfoEx = 0xea724b87,
    NvAPI_GPU_GetActivationState = 0xeb98c42f,
    NvAPI_GPS_GetPerfSensorsInternal = 0xece69bce,
    NvAPI_GPU_GetTPCMaskOnGPC = 0xed74af30,
    NvAPI_NVPM_GetExportSetting = 0xf0b18453,
    NvAPI_GPU_VdtEntriesGetInfo = 0xf4233ac1,
    NvAPI_GPU_GetGCXWakeUpReasonInfo = 0xf6f0454e,
    NvAPI_GPU_GetDeepIdleStatistics = 0xf7fbca11,
    NvAPI_GPU_GetVPRInfoData = 0xf9bcb37f,
    NvAPI_EnumLogicalGPUsInternal = 0xfb9bc2ab,

// --- nvClocks.spec (50 IDs) ---
    NvAPI_GPU_ClockClkVfRelsSetControl = 0x06e6884c,
    NvAPI_GPU_ClockAdcDevicesGetControl = 0x0c1ef2ca,
    NvAPI_GPU_ClockClkDomainRpc = 0x1391cfd6,
    NvAPI_GPU_SetClocksShmoo = 0x1ab0724b,
    NvAPI_GPU_GetPublicClockInfo = 0x1b46d4cc,
    NvAPI_GPU_ClockClkPropTopsGetInfo = 0x1ea976e2,
    NvAPI_GPU_ClockClkVfRelsGetControl = 0x2224d976,
    NvAPI_GPU_ClockAdcDevicesSetControl = 0x28dca3f0,
    NvAPI_GPU_ClockClkPropTopsGetStatus = 0x2a299aeb,
    NvAPI_GPU_ClockNafllDevicesGetInfo = 0x2bc9f805,
    NvAPI_GPU_ClockClkDomainFreqsEnum = 0x40bddb36,
    NvAPI_GPU_ClockAdcDevicesGetStatus = 0x43d9b26a,
    NvAPI_GPU_ClockClkFreqControllerGetStatus = 0x45c064d5,
    NvAPI_GPU_ClockClkPropRegimesGetControl = 0x4f11eaa4,
    NvAPI_GPU_ClockClkEnumsGetInfo = 0x5439f0b7,
    NvAPI_GPU_ClockClkPropTopsSetControl = 0x56981468,
    NvAPI_GPU_ClockClkFreqControllerGetInfo = 0x58f4f4c1,
    NvAPI_GPU_ClockClkVfRelsGetInfo = 0x5a769461,
    NvAPI_GPU_ClockAdcDevicesGetInfo = 0x68789e2a,
    NvAPI_GPU_ClockClkPropRegimesSetControl = 0x6bd3bb9e,
    NvAPI_GPU_ClockClkPropTopsGetControl = 0x725a4552,
    NvAPI_GPU_ClockPmumonClkDomainsGetSamples = 0x7c955ac0,
    NvAPI_GPU_ClockClkVfPointsGetStatus = 0x7fee9032,
    NvAPI_GPU_ClockNafllDevicesGetControl = 0x811819d7,
    NvAPI_GPU_ClockClkVoltControllerGetStatus = 0x8506c02e,
    NvAPI_GPU_ClockClkVfPointsGetInfo = 0x8895b510,
    NvAPI_GPU_ClockClkProgsSetControl = 0x95f78b36,
    NvAPI_GPU_ClockNafllDevicesSetControl = 0xa5da48ed,
    NvAPI_GPU_ClockNafllDevicesGetStatus = 0xafa4113c,
    NvAPI_GPU_ClockClkProgsGetControl = 0xb135da0c,
    NvAPI_GPU_GetPixelClockRangeInternal = 0xbed4ff0b,
    NvAPI_GPU_GetLockedClockModeStatus = 0xc4733f19,
    NvAPI_GPU_ClockClkPropTopRelsGetControl = 0xcbff71d0,
    NvAPI_GPU_ClockClkPropRegimesGetInfo = 0xcf08e934,
    NvAPI_GPU_ClockClkDomainsSetControl = 0xd14b69cf,
    NvAPI_GPU_ClockClkDomainsGetFreqInfo = 0xd2fc1b34,
    NvAPI_GPU_ClockClkFreqControllersSetControl = 0xd9be5bf9,
    NvAPI_GPU_ClockClkVfPointsGetControl = 0xda025c3e,
    NvAPI_GPU_ClockClkVoltControllersGetControl = 0xdd41633c,
    NvAPI_GPU_ClockClkVfRelsGetStatus = 0xe51c215a,
    NvAPI_GPU_ClockClkPropTopRelsGetInfo = 0xe826e4f0,
    NvAPI_GPU_ClockClkVoltControllerGetInfo = 0xec6fcd0b,
    NvAPI_GPU_ClockClkPropTopRelsSetControl = 0xef3d20ea,
    NvAPI_GPU_ClockClkDomainsGetControl = 0xf58938f5,
    NvAPI_GPU_ClockClkVoltControllersSetControl = 0xf9833206,
    NvAPI_GPU_ClockClkProgsGetInfo = 0xfaceb39b,
    NvAPI_GPU_ClockCounterMeasureAvgFreq = 0xfb8f61ec,
    NvAPI_GPU_ClockClkProgsGetStatus = 0xfbffaf22,
    NvAPI_GPU_ClockClkFreqControllersGetControl = 0xfd7c0ac3,
    NvAPI_GPU_ClockClkVfPointsSetControl = 0xfec00d04,

// --- nvPstate.spec (8 IDs) ---
    NvAPI_GPU_SetForcePstate = 0x025bfb10,
    NvAPI_GPU_GetPstatesEx = 0x3b0d30df,
    NvAPI_GPU_GetPstateLimitsInfo = 0x4af0011d,
    NvAPI_GPU_SetPstates20Private = 0x4c0b519a,
    NvAPI_GPU_SetPstates = 0x825ddf13,
    NvAPI_GPU_GetPstates = 0xa69f8e29,
    NvAPI_GPU_GetPstates20Private = 0xc5ddf56e,
    NvAPI_GPU_SetForcePstateEx = 0xe7b1198d,

// --- nvThermal.spec (11 IDs) ---
    NvAPI_GPU_ThermalHwFsGetInfo = 0x14277c24,
    NvAPI_GPU_ThermalPolicyGetStatus = 0x1b4f669b,
    NvAPI_GPU_ThermMonitorsGetStatus = 0x4b4bd039,
    NvAPI_GPU_GetThermalSlowdownState = 0x6683ee65,
    NvAPI_GPU_ThermDeviceGetInfo = 0x6ff0350c,
    NvAPI_GPU_SetThermalSimulationMode = 0x8cd42541,
    NvAPI_GPU_ThermChannelSetControl = 0x8df19fa2,
    NvAPI_GPU_ThermalPmumonThermChannelsGetSamples = 0xa7f97cf3,
    NvAPI_GPU_ThermChannelGetControl = 0xa933ce98,
    NvAPI_GPU_ThermMonitorsGetInfo = 0xb2c9d666,
    NvAPI_GPU_ThermalHwFsSetInfo = 0xcbc9361b,

// --- nvVoltage.spec (16 IDs) ---
    NvAPI_GPU_VoltVoltPoliciesGetInfo = 0x00d57b3b,
    NvAPI_GPU_VoltVoltDevicesGetControl = 0x02533065,
    NvAPI_GPU_VoltVoltPoliciesSetControl = 0x17117663,
    NvAPI_GPU_GetVoltagesInternal = 0x1785b492,
    NvAPI_GPU_VoltVoltDevicesSetControl = 0x2691615f,
    NvAPI_GPU_VoltVoltPoliciesGetControl = 0x33d32759,
    NvAPI_GPU_VoltPmumonVoltRailsGetSamples = 0x54f67bbf,
    NvAPI_GPU_GetCoreVoltage = 0x58337fa3,
    NvAPI_GPU_GetPMGRVoltageRequestArbiterValues = 0x717648fd,
    NvAPI_GPU_VoltVoltRailsSetControl = 0x87c55c8a,
    NvAPI_GPU_VoltVoltPoliciesGetStatus = 0x8d877b8f,
    NvAPI_GPU_SetPMGRVoltageRequestArbiterValues = 0x9c4bb8d0,
    NvAPI_GPU_VoltVoltRailsGetControl = 0xa3070db0,
    NvAPI_GPU_VoltVoltDevicesGetInfo = 0xa38acf9d,
    NvAPI_GPU_GetCoreVoltageControl = 0xa91f88eb,
    NvAPI_GPU_SetCoreVoltageControl = 0xdc2bd4a6,

// --- nvCooler.spec (8 IDs) ---
    NvAPI_GPU_FanPolicyGetStatus = 0x15b85505,
    NvAPI_GPU_FanPmumonFanCoolersGetSamples = 0x41716ac2,
    NvAPI_GPU_FanPolicyGetInfo = 0x76a38d54,
    NvAPI_GPU_FanTestGetInfo = 0x98a4411a,
    NvAPI_GPU_SetPmuFanControlBlock = 0xb699f73a,
    NvAPI_GPU_GetPmuFanControlBlock = 0xc3adab77,
    NvAPI_GPU_ClientFanPoliciesGetStatus = 0xcf6cef26,
    NvAPI_GPU_QueryFanSpinSenseSupport = 0xfd871348,

// --- nvPower.spec (53 IDs) ---
    NvAPI_GPU_LpwrRppgResetStats = 0x03e66fd0,
    NvAPI_GPU_LpwrPgSetMinThreshold = 0x04a13397,
    NvAPI_GPU_LpwrApSetEnable = 0x05952f34,
    NvAPI_GPU_PowerEquationGetInfo = 0x0d1f035d,
    NvAPI_GPU_LpwrRppgEnable = 0x0d723474,
    NvAPI_GPU_PowerCappingSlowdownGetStatus = 0x0ffb0de1,
    NvAPI_GPU_PwrPmumonPwrChannelsGetSamples = 0x16d63ee2,
    NvAPI_GPU_ClientPwrPoliciesSetControl = 0x17695269,
    NvAPI_GPU_LpwrPgGetSupportExt = 0x1983a8b5,
    NvAPI_GPU_PowerDeviceGetStatus = 0x1b4ff3df,
    NvAPI_GPU_LpwrPgGetEnable = 0x2208533f,
    NvAPI_GPU_LpwrGC6ResetStatistics = 0x2b4dc430,
    NvAPI_GPU_LpwrPgislandGetSupport = 0x2daecab7,
    NvAPI_GPU_ClientPwrPoliciesGetControl = 0x33ab0353,
    NvAPI_GPU_LpwrCgEnableSet = 0x361f4e22,
    NvAPI_GPU_LpwrCacheEnable = 0x391eaa21,
    NvAPI_GPU_LpwrCgEnableMaskGet = 0x3dc16f5f,
    NvAPI_GPU_LpwrPgGetStatisticsExt = 0x41a0e73e,
    NvAPI_GPU_LpwrApGetSupport = 0x43707558,
    NvAPI_GPU_PowerCappingGetInfo = 0x47d61a57,
    NvAPI_GPU_LpwrResetMode = 0x4a397f86,
    NvAPI_GPU_ClientPwrPoliciesGetInfo = 0x54924bf5,
    NvAPI_GPU_LpwrPgGetSubFeatureMask = 0x5a23e845,
    NvAPI_GPU_LpwrPexGetStatistics = 0x5a3878f8,
    NvAPI_GPU_LpwrPgEnable = 0x5c1230b8,
    NvAPI_GPU_LpwrSetMode = 0x60f0b2dc,
    NvAPI_GPU_LpwrApGetStatistics = 0x70de8252,
    NvAPI_GPU_LpwrPexGetSupport = 0x75c1f45d,
    NvAPI_GPU_LpwrPsiEnableSet = 0x7661a544,
    NvAPI_GPU_LpwrGetD3HotCyclesInfo = 0x7d7152c4,
    NvAPI_GPU_ClientPwrPoliciesGetStatus = 0x83bb8d0b,
    NvAPI_GPU_LpwrCacheGetStatus = 0x8e29fc3d,
    NvAPI_GPU_LpwrPexResetStats = 0x907bee3f,
    NvAPI_GPU_LpwrPgSubFeatureMaskSet = 0x98003511,
    NvAPI_GPU_LpwrPgResetStats = 0x9910d3e5,
    NvAPI_GPU_LpwrPexGetEnableStatus = 0x9a1733fb,
    NvAPI_GPU_LpwrPsiResetStats = 0xa408d0f8,
    NvAPI_GPU_LpwrPgSetThreshold = 0xa6774433,
    NvAPI_GPU_PowerLeakageGetStatus = 0xb2f30ba0,
    NvAPI_GPU_LpwrTestStart = 0xb56c50ff,
    NvAPI_GPU_GetPowerConnectorStatus = 0xbf739fff,
    NvAPI_GPU_LpwrGetModeSupportMask = 0xd3c13143,
    NvAPI_GPU_GetLpwrStatistics = 0xd62044b0,
    NvAPI_GPU_PowerLeakageGetInfo = 0xd6b6d3e4,
    NvAPI_GPU_LpwrApGetEnable = 0xd891bba8,
    NvAPI_GPU_LpwrToggleWakeupType = 0xde6902c1,
    NvAPI_GPU_LpwrPsiCrossoverCurrrentSet = 0xe0a5f984,
    NvAPI_GPU_LpwrCgSupportGet = 0xe1339b20,
    NvAPI_GPU_LpwrPgislandGetStatistics = 0xe1825fa8,
    NvAPI_GPU_LpwrPgResetThreshold = 0xe9a1618e,
    NvAPI_GPU_LpwrTestStop = 0xeda9abca,
    NvAPI_GPU_LpwrDidleGetSupport = 0xf0698fc9,
    NvAPI_GPU_LpwrPexFeatureEnableSet = 0xfcfafaba,

// --- power.spec (3 IDs) ---
    NvAPI_SYS_GetClockInfo = 0x087f5f36,
    NvAPI_SYS_SetLowestPowerState = 0x34532b04,
    NvAPI_SYS_EnableDVFS = 0x9b13dfc2,

// --- power.spe (1 IDs) ---
    NvAPI_Power_Unload = 0xc680fabb,

// --- nvTelemetry.spec (3 IDs) ---
    NvAPI_SetTelemetryAllowList = 0x21821692,
    NvAPI_SaveAppStatistics = 0x5b4fa0d2,
    NvAPI_GetAppStatisticsV2 = 0x9fe643e4,

// --- nvDisplay.spec (182 IDs) ---
    NvAPI_DISP_IsFullscreenAppRunning = 0x005850e9,
    NvAPI_DISP_DpDscControl = 0x0286b8ed,
    NvAPI_Disp_GetColorSupportedCombinations = 0x037fb940,
    NvAPI_Disp_DP_GetTestPattern = 0x05ae7faa,
    NvAPI_DISP_ValidateEdid = 0x05d3496e,
    NvAPI_Diag_DP_VideoInfo = 0x06a56e20,
    NvAPI_DISP_DscControl = 0x0754b8a8,
    NvAPI_DISP_GetWideColorRange = 0x077c54db,
    NvAPI_DISP_TestMuxAuxSettleDelay = 0x0801975b,
    NvAPI_IsIntelHybrid = 0x08f90d07,
    NvAPI_DISP_SetEdidData = 0x0a1a174a,
    NvAPI_DISP_SetHyperSamplingSettingsEx = 0x0b396d98,
    NvAPI_DISP_GetScalingCapsOverride = 0x0bb06808,
    NvAPI_GetHybridInfo = 0x106ec09a,
    NvAPI_DISP_DisableUnderscanConfig = 0x1123acb1,
    NvAPI_DISP_GetOsDDisplayInfo = 0x125a1015,
    NvAPI_Diag_DP_TestPattern = 0x130e96d7,
    NvAPI_Coproc_GetCoprocStatusEx = 0x15a3e66e,
    NvAPI_DISP_DpPpsQueryControl = 0x16555400,
    NvAPI_DISP_GetScanoutRasterDimension = 0x1a0b9825,
    NvAPI_DISP_EnableDirectMode = 0x1ac2b1e7,
    NvAPI_Hybrid_StartTransition = 0x1af3b2b7,
    NvAPI_Diag_DPCD = 0x1c0f37ab,
    NvAPI_DISP_ScanoutLogging = 0x1ce58f7b,
    NvAPI_DISP_GetIFlipStateByDisplayName = 0x1e268952,
    NvAPI_Hybrid_SetHybridModeAndDisplayConfig = 0x2029698f,
    NvAPI_DISP_GetGSyncInfo = 0x212f551f,
    NvAPI_DISP_SetWideColorRange = 0x21c299c8,
    NvAPI_DISP_GetDpGenericInfoframe = 0x2678fa1c,
    NvAPI_DISP_QueryIgpuBlenState = 0x26c8af87,
    NvAPI_DISP_GetPanelReplayDebugData = 0x273535bb,
    NvAPI_Hybrid_ControlDriverHDAEx = 0x2a62dfc0,
    NvAPI_DISP_QueryMuxInfo = 0x2ccc37a0,
    NvAPI_LightWeightDGPU = 0x2da1e50f,
    NvAPI_GetHybridStatusString = 0x2df387ad,
    NvAPI_DISP_DirectModeDisplayControl = 0x2eebaccc,
    NvAPI_Coproc_GetCoprocStatusString = 0x2f2b5f5a,
    NvAPI_DISP_Mux_SB_I2C_Control = 0x2f432b2f,
    NvAPI_DISP_GetDirectModeDisplayHandleFromDisplayId = 0x33649f39,
    NvAPI_DISP_GetFeatureConfig = 0x337bc36b,
    NvAPI_DISP_GetSelfRefreshPanelStatus = 0x34f121b1,
    NvAPI_GetHybridSwitchStatus = 0x35582435,
    NvAPI_Coproc_SetGC6WakeBehaviorEx = 0x365d878c,
    NvAPI_DISP_GetDisplayMuxDeviceId = 0x36eed51d,
    NvAPI_HybridIGPUHeadsControl = 0x38af5465,
    NvAPI_DISP_GetCurrentVRRSettings = 0x38eb95d2,
    NvAPI_Coproc_ControlLimitedCycles = 0x3dce1577,
    NvAPI_Coproc_SetCoprocInfoFlagsEx2 = 0x3e983160,
    NvAPI_DISP_ApplyAndSaveCustomResolution = 0x40bc7ee3,
    NvAPI_DISP_GetDisplayIdFromDisplayHandle = 0x4575dd94,
    NvAPI_SetViewExInternal = 0x45aea29b,
    NvAPI_DISP_GetHyperSamplingSettings = 0x461f84c1,
    NvAPI_DISP_MuxTransition = 0x469921df,
    NvAPI_Coproc_GetCoprocFlagsString = 0x46a3dabf,
    NvAPI_DISP_GetTargetModeSet = 0x47a4b0d0,
    NvAPI_DISP_GetPsrInfo = 0x47c9fad3,
    NvAPI_DISP_SetPanelReplayInfo = 0x48f39943,
    NvAPI_Hybrid_GetMirroredDisplays = 0x49f86afb,
    NvAPI_Disp_GetBrightnessNits = 0x4a2b52d0,
    NvAPI_Coproc_QueryHDAudioStateEx = 0x4ac563fe,
    NvAPI_Hybrid_SwapDisplays = 0x4b700d2a,
    NvAPI_Coproc_GetCoprocInfoFlagsEx2 = 0x4bac6d2d,
    NvAPI_SetHDMIAudioStreamMute = 0x4ecbea37,
    NvAPI_DISP_DirectModeGetMissedFrameWaitableObject = 0x4fd61b86,
    NvAPI_Hybrid_StopTransition = 0x504d9215,
    NvAPI_InitHybridMicroController = 0x50f44fef,
    NvAPI_DISP_BpcConfiguration = 0x5216be47,
    NvAPI_Coproc_SetCoprocInfoFlags = 0x52eb0440,
    NvAPI_DISP_SR_RESET_LATENCY_STATS = 0x556911db,
    NvAPI_DISP_LCDOverDriveControl = 0x5675d850,
    NvAPI_Diag_DP_LaneData = 0x5a96d0f4,
    NvAPI_GetHybridModesString = 0x5c909191,
    NvAPI_DISP_LTStats = 0x5e14cb10,
    NvAPI_DISP_GetDisplayMuxStats = 0x5fdd5772,
    NvAPI_DISP_EnumerateDirectModeDisplays = 0x602f60d8,
    NvAPI_DISP_GetDisplayMuxCaps = 0x6050bb05,
    NvAPI_DISP_SetPsrInfo = 0x630babe9,
    NvAPI_Disp_HdrSessionControl = 0x64c24af1,
    NvAPI_DISP_GetPsrDebugData = 0x654aff4a,
    NvAPI_DISP_GetMergedDisplaysTopology = 0x655b7bdc,
    NvAPI_DISP_SetHyperSamplingSettings = 0x655c5ff5,
    NvAPI_DISP_GetTvClassification = 0x6564435d,
    NvAPI_Coproc_GetHysteresisEx = 0x65edd248,
    NvAPI_DISP_GetDpMsaAttributes = 0x66887143,
    NvAPI_Coproc_QueryHDAudioState = 0x670ecfee,
    NvAPI_DISP_EnterBufferedSelfRefresh = 0x67661c03,
    NvAPI_DISP_GetSourceModeSet = 0x68b59b03,
    NvAPI_Coproc_GetHysteresis = 0x699ab941,
    NvAPI_DISP_GetViewPortInfo = 0x69cccd08,
    NvAPI_Hybrid_StopDisplaySwitch = 0x6a04e6ae,
    NvAPI_DISP_ExitSparseSelfRefreshPanel = 0x6b624db7,
    NvAPI_Disp_SetBrightnessNits = 0x6c959fc3,
    NvAPI_DISP_DP_GetStreamIDs = 0x6d3a0a28,
    NvAPI_Hybrid_SetDisplayMUX = 0x6d9651b6,
    NvAPI_Coproc_OverrideCoprocInfoFlagsEx = 0x6e973ac5,
    NvAPI_DISP_SetTargetGammaCorrection = 0x7082a053,
    NvAPI_Coproc_SetHysteresis = 0x708307d6,
    NvAPI_DISP_GetDisplayIdFromDirectModeDisplayHandle = 0x71607531,
    NvAPI_Disp_DP_SetTestPattern = 0x72c8c3d5,
    NvAPI_DISP_SetDisplayMux = 0x73be7f64,
    NvAPI_DISP_GetPanelReplayInfo = 0x749b7111,
    NvAPI_GetVideoLinkParams = 0x76e48826,
    NvAPI_DISP_GetScalingSupport = 0x776fc06f,
    NvAPI_DISP_DisableDirectMode = 0x7951e57c,
    NvAPI_Disp_DP_GetDPLinkStatistics = 0x7abd6e8b,
    NvAPI_DISP_GetBackLightInfo = 0x7b3be896,
    NvAPI_DISP_EnterSparseSelfRefreshPanel = 0x7c33ab5a,
    NvAPI_GetGFAHandle = 0x7eb95503,
    NvAPI_DISP_SetFeatureConfig = 0x7f5cf74d,
    NvAPI_Coproc_GetGoldStatisticsEx = 0x82c48494,
    NvAPI_GPU_SetupClusterTopology = 0x8534b728,
    NvAPI_SendHybridMessage = 0x872b4463,
    NvAPI_Coproc_SetGC6WakeBehavior = 0x88b47f2c,
    NvAPI_DISP_GetHDMIStereoSettings = 0x8cf2d19b,
    NvAPI_GetHybridEDID = 0x8d5ccfcc,
    NvAPI_Hybrid_ControlDriverHDABus = 0x91083ff1,
    NvAPI_DISP_GetCursorState = 0x91b594b3,
    NvAPI_DISP_GetDisplayHandleFromDisplayId = 0x96437923,
    NvAPI_GPU_EnumClusterTopologies = 0x995cd4eb,
    NvAPI_DISP_SR_GET_LATENCY_STATS = 0x9a1a4db8,
    NvAPI_DISP_ExitBufferedSelfRefresh = 0x9b0cddb3,
    NvAPI_Coproc_NotifyCoprocPowerStateEx = 0x9b3e5ee0,
    NvAPI_Hybrid_SetDGPUDriverState = 0x9f30e3d1,
    NvAPI_DISP_GetHCloneIGPUDisplayEdid = 0x9fa3584d,
    NvAPI_Hybrid_GetDGPUDriverState = 0xa3580b83,
    NvAPI_SetHybridModeEx = 0xa3e36f0a,
    NvAPI_DISP_DirectModeGetPresentWaitableObject = 0xa3f1f373,
    NvAPI_DISP_GetDisplayMuxTconInfo = 0xa6143f60,
    NvAPI_DISP_SetOsDDisplayProperties = 0xa9c09858,
    NvAPI_DISP_GetTimingInfo = 0xab3b7419,
    NvAPI_Disp_DP_GetLaneConfig = 0xabf292f1,
    NvAPI_DISP_TwoHeadOneORDynamicSwitchControl = 0xac4d5442,
    NvAPI_Hybrid_GetIntelDeviceMap = 0xaddfa99e,
    NvAPI_DISP_GetMSHybridFBCCaptureState = 0xae9c64f3,
    NvAPI_DISP_EnumerateOsDedicatedDisplays = 0xb2a28bfc,
    NvAPI_Disp_DP_SetLaneConfig = 0xb2eb2c66,
    NvAPI_DISP_SetMergedDisplaysTopology = 0xb3e0dfe5,
    NvAPI_Hybrid_ControlDriverHDA = 0xb5655bce,
    NvAPI_DISP_ColorAccuracyMode = 0xb5e8e855,
    NvAPI_GetSupportedViewsEx = 0xb68f3440,
    NvAPI_DISP_DpFecQueryControl = 0xb69e0b55,
    NvAPI_Hybrid_ControlDriverHDABusEx = 0xb6b2880b,
    NvAPI_RmConfigGet = 0xb8249127,
    NvAPI_DISP_GetHCloneDisplayCaps = 0xbb7f91d9,
    NvAPI_DISP_GetHyperSamplingSettingsEx = 0xbb8052c3,
    NvAPI_DISP_GetPanScanInfo = 0xbca17455,
    NvAPI_GetHybridConnectedOutputs = 0xbe5c71cb,
    NvAPI_GetAssociatedDisplayOutputIdEx = 0xc5e31a58,
    NvAPI_DISP_SetScalingCapsOverride = 0xc89d5384,
    NvAPI_DISP_IMPQuery = 0xcaedd664,
    NvAPI_DISP_SetPanScanInfo = 0xcbc7c82a,
    NvAPI_DISP_DP_QueryStatus = 0xcbd2db05,
    NvAPI_DISP_GetTargetModeSetEx = 0xcbfdd500,
    NvAPI_Disp_SetBrightnessCalibrationData = 0xcc7ed579,
    NvAPI_DP_ReadAuxLogger = 0xd04ff409,
    NvAPI_DISP_SetOculusZoomFactor = 0xd0f299a3,
    NvAPI_DISP_ConstructIMPModeEx2 = 0xd12aba03,
    NvAPI_DISP_PSR_Control = 0xd2762cf4,
    NvAPI_Hybrid_SetDGPUPowerState = 0xd28ea837,
    NvAPI_Video_GetProtectedVideoSessionInfo = 0xd45cdf8b,
    NvAPI_DISP_GetVidPnSourceId = 0xd60fb01a,
    NvAPI_Coproc_RegisterProcess = 0xd8498827,
    NvAPI_DISP_EnterBurstSelfRefresh = 0xd9bbde39,
    NvAPI_QueryHybridIGPUHeadsControl = 0xddb1017f,
    NvAPI_Disp_GetBrightnessCalibrationData = 0xdeacdda5,
    NvAPI_Coproc_OverrideCoprocInfoFlags = 0xe300109a,
    NvAPI_Disp_EnumerateDisplayModes = 0xe507deda,
    NvAPI_DISP_SetCursorState = 0xe6d328cc,
    NvAPI_DISP_ValidateEdidData = 0xe935d9d7,
    NvAPI_DISP_ExitBurstSelfRefresh = 0xed3c8d24,
    NvAPI_DISP_IMPSetGetParams = 0xeea80619,
    NvAPI_Disp_ConstructIMPModeEx = 0xef54e717,
    NvAPI_DISP_GetScalingCaps = 0xf17985ed,
    NvAPI_DISP_SetViewPortInfo = 0xf183ffea,
    NvAPI_Hybrid_GetDGPUPowerState = 0xf4306524,
    NvAPI_DISP_DirectModeGetVSyncWaitableObject = 0xf64d320f,
    NvAPI_Hybrid_StartDisplaySwitch = 0xf698c5e1,
    NvAPI_DISP_EnableUnderscanConfig = 0xf8a3a0a1,
    NvAPI_DISP_SetHDMIStereoSettings = 0xf9c68dd6,
    NvAPI_DISP_DpDscCrcControlControl = 0xfa2e1edd,
    NvAPI_DISP_GetDisplayIdsInCluster = 0xfae7855f,
    NvAPI_Coproc_SetHysteresisEx = 0xfda2e0aa,

// --- nvDRS.spec (32 IDs) ---
    NvAPI_DRS_RestoreDefaultSettings = 0x04357011,
    NvAPI_DRS_SaveSettingsToPrdFile = 0x1267818e,
    NvAPI_DRS_GetSettingNameFromIdEx = 0x1eb13791,
    NvAPI_DRS_GetDrsVersion = 0x250cbfb2,
    NvAPI_DRS_SaveSettingsEx = 0x25fd8ae4,
    NvAPI_DRS_FindApplicationEx = 0x2f203ccb,
    NvAPI_DRS_GetSettingIdFromNameEx = 0x30dc748f,
    NvAPI_DRS_GetGoldDBDrsVersion = 0x3edb7fbe,
    NvAPI_DRS_GetSettingForCurrentProcess = 0x4d994a96,
    NvAPI_DRS_LoadDefaultSettings = 0x4e0f2bc0,
    NvAPI_DRS_CreateApplicationEx = 0x5f3df409,
    NvAPI_DRS_EnumAvailableSettingValuesEx = 0x6feb9b76,
    NvAPI_DRS_GetDefaultGlobalProfile = 0x7d067162,
    NvAPI_DRS_RestoreProfileDefaultSettingEx = 0x7dd5b261,
    NvAPI_DRS_EnumSettingInfo = 0x863b9572,
    NvAPI_DRS_SetSettingEx = 0x8a2cf5f5,
    NvAPI_DRS_DecryptSession = 0x8fc247b7,
    NvAPI_DRS_LoadGoldSettings = 0xa782ea46,
    NvAPI_DRS_SetBaseProfile = 0xade2dadf,
    NvAPI_DRS_GetLastDBChangeTime = 0xb19da6ce,
    NvAPI_DRS_LoadSettingsFromPrdFile = 0xc63c045b,
    NvAPI_DRS_SetProfileData = 0xc83a1baa,
    NvAPI_DRS_EnumSettingsEx = 0xcfd6983e,
    NvAPI_DRS_CreateProfileEx = 0xd117babe,
    NvAPI_DRS_DeleteProfileSettingEx = 0xd20d29df,
    NvAPI_DRS_EnumAvailableSettingIdsEx = 0xe5de48e5,
    NvAPI_DRS_GetSystemDrsVersion = 0xe5e61f73,
    NvAPI_DRS_EnumUISettingValues = 0xe77470d6,
    NvAPI_DRS_GetSettingEx = 0xea99498d,
    NvAPI_DRS_RestoreSettingsInfo = 0xeabded78,
    NvAPI_DRS_GetSettingInfo = 0xf45d6637,
    NvAPI_DRS_GetGoldDBDrsVersionEx = 0xf58d36ad,

// --- nvMosaic.spec (15 IDs) ---
    NvAPI_Mosaic_GetBezelPeeking = 0x11c837ad,
    NvAPI_Mosaic_GetResolutions = 0x1ee2fe62,
    NvAPI_Mosaic_SetDisplayGridsWithSLI = 0x2053f234,
    NvAPI_Mosaic_GetMosaicCapabilitiesEx = 0x2aea7ee5,
    NvAPI_Mosaic_GetSingleGpuMosaicCaps = 0x3bebae6b,
    NvAPI_Mosaic_GetResolutionPruning = 0x451d5f7d,
    NvAPI_Mosaic_SetResolutions = 0x6984421d,
    NvAPI_Mosaic_GetDisplayPhysicalArrangement = 0x83877959,
    NvAPI_Mosaic_SetBezelPeeking = 0x8987054f,
    NvAPI_Mosaic_GetGridOverlapLimits = 0x933e2b3a,
    NvAPI_Mosaic_GetSupportedTopoInfoEx = 0x9afb9dee,
    NvAPI_Mosaic_EnumPossibleConfigs = 0xa4cde3a4,
    NvAPI_Mosaic_SetResolutionPruning = 0xd87cbb9c,
    NvAPI_Mosaic_GetPossibleConfig = 0xdfebcbdf,
    NvAPI_Mosaic_GetSymmetricOrderedDisplayIds = 0xe6480838,

// --- nvGsync.spec (28 IDs) ---
    NvAPI_GSync_UpdateSyncInterval = 0x21821099,
    NvAPI_GSync_Signal_Event = 0x318018a0,
    NvAPI_EnumVisualComputingDevices = 0x32f707a0,
    NvAPI_VCD_GetDeviceInfo = 0x3df6196c,
    NvAPI_GSync_FPGAFlashHelper = 0x3fb513de,
    NvAPI_GSync_UpdateSyncSource = 0x42205bcb,
    NvAPI_GSync_UpdateSyncSkew = 0x462fc0a0,
    NvAPI_GSync_SetSyncState = 0x468abdd7,
    NvAPI_GSync_DisableSync = 0x4c7f9e3e,
    NvAPI_VCD_GetPowerSupplyInfo = 0x4db05311,
    NvAPI_VCD_GetAssociatedGSyncs = 0x5d50bac8,
    NvAPI_GSync_UpdateInterlaceMode = 0x79b85e23,
    NvAPI_GSync_QueryInterlaceMode = 0x7d2794ab,
    NvAPI_VCD_UpdatePerformanceMode = 0x801b5e51,
    NvAPI_GSync_UpdateVideoMode = 0x82cb26b7,
    NvAPI_GSync_QueryTopology = 0x971c1410,
    NvAPI_GSync_EnableSync = 0x9ea851bb,
    NvAPI_EnumGSyncDevices = 0xaafd4ebc,
    NvAPI_GSync_QueryStatusSignals = 0xaca805e6,
    NvAPI_HIC_QueryTopology = 0xba91f490,
    NvAPI_GSync_UpdateSyncPolarity = 0xc929dc63,
    NvAPI_VCD_GetCoolerSettings = 0xcbf2d7a2,
    NvAPI_GSync_Get_DiagnosticSettings = 0xcc40cc37,
    NvAPI_GSync_RegOp = 0xe076f6df,
    NvAPI_GSync_UpdateSyncStartDelay = 0xe140175e,
    NvAPI_VCD_GetThermalInfo = 0xe5566f3f,
    NvAPI_GSync_QuerySyncParameters = 0xe65e001c,
    NvAPI_GSync_QuerySyncStatus = 0xf68eb811,

// --- nvTopps.spec (16 IDs) ---
    NvAPI_GPU_ClientEnableBackgroundOcScanner = 0x06dc7ce8,
    NvAPI_GPU_InternalRegisterForOcConfigChangedUpdates = 0x25b5268d,
    NvAPI_GPU_ClientRegisterForFanCoolerSampleUpdates = 0x28194fe7,
    NvAPI_GPU_ClientRegisterForPerfPolicySampleUpdates = 0x28485bc6,
    NvAPI_SYS_InternalNotifyNvToppsDataReportReady = 0x28e41f16,
    NvAPI_GPU_ClientRegisterForVoltageSampleUpdates = 0x2c317328,
    NvAPI_GPU_ClientRegisterForPowerSampleUpdates = 0x6367f257,
    NvAPI_GPU_InternalRegisterForPowerSampleUpdates = 0x70cd4f5a,
    NvAPI_GPU_ClientEnableGrdOc = 0x9dd000d8,
    NvAPI_SYS_InternalGetNvToppsDataReport = 0xaadce62d,
    NvAPI_GPU_InternalGetOcConfig = 0xb66c4594,
    NvAPI_GPU_ClientRegisterForClockSampleUpdates = 0xbd9a05f0,
    NvAPI_GPU_GetLastIncompleteOcScannerResults = 0xbe371d0a,
    NvAPI_SYS_InternalNvtoppsGlobalSetControl = 0xdcd066bd,
    NvAPI_GPU_ClientRegisterForThermalSampleUpdates = 0xea0a86ae,
    NvAPI_SYS_ClientJpacGetInfo = 0xea20226c,

// --- nv3DVP.spec (34 IDs) ---
    NvAPI_3DVP_GetGlassesName = 0x0201f358,
    NvAPI_3DVP_IdentifyGlasses = 0x096fac80,
    NvAPI_3DVP_SetGlassesSyncCycle = 0x278b9dc0,
    NvAPI_3DVP_GetTransceiverChannel = 0x343c9695,
    NvAPI_3DVP_DestroyContext = 0x345c5135,
    NvAPI_3DVP_GetGlassesState = 0x35471427,
    NvAPI_3DVP_GetTransceiverState = 0x3743c166,
    NvAPI_3DVP_DiscoverGlasses = 0x3d37cec1,
    NvAPI_3DVP_SetTransceiverChannel = 0x4108cad8,
    NvAPI_3DVP_PairGlasses = 0x549d4354,
    NvAPI_3DVP_GetTransceiverMode = 0x5ab06757,
    NvAPI_3DVP_SetTransceiverChannels = 0x648dceb5,
    NvAPI_3DVP_SetTransceiverMode = 0x66d88f05,
    NvAPI_3DVP_IsAirplaneModeEnabled = 0x6985d54a,
    NvAPI_3DVP_GetGlassesAccess = 0x750cc7ca,
    NvAPI_3DVP_SetGlassesName = 0x75674f27,
    NvAPI_3DVP_GetTransceiverAccess = 0x785bbda4,
    NvAPI_3DVP_WaitEvent = 0x7c066002,
    NvAPI_3DVP_GetTransceiver = 0x8d9a865a,
    NvAPI_3DVP_GetTransceiverChannelInfo = 0x907d15bb,
    NvAPI_3DVP_EnumTransceiver = 0x9a10a809,
    NvAPI_3DVP_GetTransceiverSignalQuality = 0x9f6ee934,
    NvAPI_3DVP_CreateContext = 0xa36b6a85,
    NvAPI_3DVP_OpenTransceiver = 0xa652b8b3,
    NvAPI_3DVP_GetTransceiverChannels = 0xa7a0f539,
    NvAPI_3DVP_GetTransceiverInfo = 0xa8e5cf0c,
    NvAPI_3DVP_GetGlassesSyncCycle = 0xae5d3934,
    NvAPI_3DVP_UnpairGlasses = 0xb24f1ea1,
    NvAPI_3DVP_GetGlassesInfo = 0xc264e4c9,
    NvAPI_3DVP_CloseTransceiver = 0xc8b007df,
    NvAPI_3DVP_ResetGlassesToFactorySettings = 0xcb276f5c,
    NvAPI_3DVP_EnumGlasses = 0xd1c0ea13,
    NvAPI_3DVP_OpenTransceiverPriviledged = 0xee401d43,
    NvAPI_3DVP_ResetTransceiverToFactorySettings = 0xf2ae32c9,

// --- nvStereo.spec (14 IDs) ---
    NvAPI_Stereo_GetWindowedMode = 0x1eb29590,
    NvAPI_Stereo_ModeEnumControl = 0x2b04794b,
    NvAPI_Stereo_Dongle_Status = 0x32bcb3cc,
    NvAPI_Stereo_GetStereoDiag = 0x50da0e87,
    NvAPI_Stereo_GetStereoCapsInternal = 0x5e5f6c12,
    NvAPI_Stereo_SetWindowedMode = 0x86fda772,
    NvAPI_Stereo_SetProfileName = 0x87045ea3,
    NvAPI_Stereo_GetInfo = 0xaeabc278,
    NvAPI_Stereo_DongleControl = 0xb843694b,
    NvAPI_Stereo_IsDisplayAegisDTType = 0xb9fd41c4,
    NvAPI_Stereo_IsAccessoryDisplayEnabled = 0xbce88c04,
    NvAPI_Stereo_SetVideoControl = 0xbda6f001,
    NvAPI_Stereo_SetVideoMetadata = 0xc3007f26,
    NvAPI_Stereo_GetAppInfo = 0xe949b228,

// --- nvAudio.spec (14 IDs) ---
    NvAPI_GPU_GetMaxAudioStreamCount = 0x05ee23b6,
    NvAPI_SecureAudio_NegotiateDHExchange = 0x2afd7dc7,
    NvAPI_SecureAudio_CheckAPICompatibility = 0x467819d6,
    NvAPI_Audio_SetDeviceParametersOverride = 0x47bac7d9,
    NvAPI_GPU_GetAudioDeviceEntryPriorityList = 0x4ad30f76,
    NvAPI_SecureAudio_GetCustomFormatGUID = 0x74fe99ff,
    NvAPI_GPU_SetAudioStreams = 0x7ca86a93,
    NvAPI_Audio_EnumDeviceHandle = 0x88330469,
    NvAPI_Audio_GetDeviceParameters = 0xafc95163,
    NvAPI_SecureAudio_ComputeSessionKey = 0xd3b0b765,
    NvAPI_GPU_SetAudioDeviceEntryPriorityList = 0xdd89c278,
    NvAPI_SecureAudio_GetSecureAudioAPIRevision = 0xdf1f8f1b,
    NvAPI_GPU_GetAudioStreams = 0xe4e75871,
    NvAPI_SecureAudio_PollOOSDState = 0xece87388,

// --- nvVideo.spec (40 IDs) ---
    NvAPI_GetVideoStateEx = 0x0b6ef8b9,
    NvAPI_Video_SetEncodeInfo = 0x0c462f15,
    NvAPI_GetVideoStreamInfo = 0x0cae9a69,
    NvAPI_GetVideoDeviceInfo = 0x1003328a,
    NvAPI_RegisterVideoDataProvider = 0x13f7111e,
    NvAPI_Video_GetEncodeInfo = 0x155f9182,
    NvAPI_Video_PmmC_SetMode = 0x18a35445,
    NvAPI_GetVideoStreamCount = 0x1bb5779a,
    NvAPI_Video_GetVideoSurfaceCount = 0x1d5b485b,
    NvAPI_SetVideoPerformanceDataCollectionEnabled = 0x1ead124c,
    NvAPI_GetVideoPerformanceData = 0x2040ac5d,
    NvAPI_Video_PostProcessing_GetDefault = 0x23b19713,
    NvAPI_Video_PostProcessing_Set = 0x2f948148,
    NvAPI_Video_ReleaseEncodeInfo = 0x307249d3,
    NvAPI_GetVideoData = 0x32789091,
    NvAPI_Video_PostProcessing_Get = 0x357cfb24,
    NvAPI_Video_ColorControl_GetDefault = 0x38ac3081,
    NvAPI_Video_Pmmc_SetDomainConfig = 0x4306deac,
    NvAPI_EnumVideoDataProviders = 0x4821ecd8,
    NvAPI_Video_PmmC_GetReport = 0x493584d6,
    NvAPI_VideoCtrl = 0x58934e9a,
    NvAPI_GetVideoPerformanceDataCollectionEnabled = 0x5cac215e,
    NvAPI_EnumerateVideoControlPoints = 0x5f6d8e71,
    NvAPI_GetActiveVideoDevice = 0x6579f1c8,
    NvAPI_Video_GetOverlayInfo = 0x6f83fb5b,
    NvAPI_Video_Bringup = 0x8f947c8b,
    NvAPI_SetVideoStateEx = 0x9321ca5b,
    NvAPI_GetVidLocMap = 0x93373c0f,
    NvAPI_GPU_SetVidPnInfo = 0xb94b341b,
    NvAPI_Video_GetDXVAInfo = 0xc5ad6e48,
    NvAPI_Video_GetVideoStateInfo = 0xc9116276,
    NvAPI_GetVideoDeviceCount = 0xcf00d48e,
    NvAPI_Video_ColorControl_Set = 0xe0c76dfd,
    NvAPI_Video_GetVideoSurfaceInfo = 0xe44bd59d,
    NvAPI_Video_EvoOverlayLUT_Set = 0xe563af29,
    NvAPI_VideoControl = 0xe74750e2,
    NvAPI_UnregisterVideoDataProvider = 0xebc7a299,
    NvAPI_SetActiveVideoDevice = 0xf8181529,
    NvAPI_Video_ColorControl_Get = 0xfa2f1791,
    NvAPI_Video_EvoOverlayLUT_Get = 0xff8bd545,

// --- nvIllum.spec (6 IDs) ---
    NvAPI_GPU_IllumDevicesGetControl = 0x89396bf8,
    NvAPI_GPU_IllumZonesGetInfo = 0x8b773228,
    NvAPI_GPU_IllumDevicesGetInfo = 0x92d70a37,
    NvAPI_GPU_IllumDevicesSetControl = 0xadfb3ac2,
    NvAPI_GPU_IllumZonesGetControl = 0xd4bb05a1,
    NvAPI_GPU_IllumZonesSetControl = 0xf079549b,

// --- nvGeneral.spec (13 IDs) ---
    NvAPI_GetDriverModuleLocation = 0x03d2f7d0,
    NvAPI_SetPersistenceData = 0x1b765642,
    NvAPI_GetPersistenceData = 0x271ebe10,
    NvAPI_GPU_GetAppStatisticsVm = 0x536ca5dc,
    NvAPI_GetAppStatistics = 0x5e4f3b9f,
    NvAPI_Diag_GetNvConfigData = 0x843b4f60,
    NvAPI_GPU_GetAppStatistics = 0xad1e4a48,
    NvAPI_EnumAppStatistics = 0xb0c0a5fd,
    NvAPI_SYS_GetDriverAndBranchVersionEx = 0xbf75a81e,
    NvAPI_Mjolnir_SetupStreamingSession = 0xd1682334,
    NvAPI_GetAppStatisticsVm = 0xdf3e555e,
    NvAPI_Mjolnir_GetStreamingInfo = 0xed94e84c,
    NvAPI_Diag_DP_ASSR = 0xf932e4f1,

// --- nvSystem.spec (60 IDs) ---
    NvAPI_SYS_GetJTCaps = 0x08a3c81d,
    NvAPI_SYS_GetNBCIPlatCaps = 0x0bcd56e5,
    NvAPI_SYS_GetVGXInfo = 0x0dec64e5,
    NvAPI_SYS_ValidateLicense = 0x107dea30,
    NvAPI_SYS_ClearMiscLicenseInfo = 0x10a3ef29,
    NvAPI_SYS_ACPI_NotifyEvent = 0x11bdc4d7,
    NvAPI_SYS_SetFeatureState = 0x1813f7c4,
    NvAPI_Reflex_TestModesSet = 0x1b2f4490,
    NvAPI_SYS_GetMDTLData = 0x1e8cd32b,
    NvAPI_Set_NIS2_Sharpness = 0x28ee366c,
    NvAPI_SYS_GetCpuInfo = 0x29fbc5ea,
    NvAPI_SYS_NotifySBiosDisplaySwitch = 0x2d6f7431,
    NvAPI_SYS_GetPipeServerInformation = 0x2d9bac00,
    NvAPI_SYS_NVIF_PlatformConfig = 0x2fa7e0eb,
    NvAPI_SYS_GetHwbcInfo = 0x33c955ee,
    NvAPI_SYS_NVIF_SetValues = 0x38a18c40,
    NvAPI_SYS_SetScreenSaverState = 0x3aca17a6,
    NvAPI_SYS_GetPowerStatus = 0x45ba381b,
    NvAPI_SYS_GetSmartDimmerLevel = 0x4c8c1bd2,
    NvAPI_SYS_Frl20AlignVblank = 0x59c4cfc7,
    NvAPI_SYS_UIControl = 0x5c3565d0,
    NvAPI_SYS_GetCursorInfo = 0x609aa6c5,
    NvAPI_SYS_SpbControl = 0x60a56701,
    NvAPI_SYS_FixInvalidDriverState = 0x66583afe,
    NvAPI_SYS_InternalGetNocatJournal = 0x6952dc98,
    NvAPI_GPU_GetSmartDimmerConfig = 0x69a6f90d,
    NvAPI_SYS_CheckIfDriverHackedForSLI = 0x79e4d097,
    NvAPI_SYS_GenerateLicense = 0x7c5471c0,
    NvAPI_SYS_SpbGetPstateTable = 0x7ea256de,
    NvAPI_SYS_GetFeatureState = 0x805cc526,
    NvAPI_SYS_RemoveLicense = 0x80e2301a,
    NvAPI_SYS_IsPhysXApplication = 0x8545513e,
    NvAPI_SYS_GetDisplayIdFromLUID = 0x95e11d62,
    NvAPI_SYS_EnableDisplayHotkeyHandling = 0x977c45e6,
    NvAPI_SYS_SaveOCAFingerprint = 0x99d379de,
    NvAPI_SYS_SetValidMDTLIndex = 0xa0b3e958,
    NvAPI_SYS_GetGpuCount = 0xab4a478e,
    NvAPI_SYS_SetStereoMetaData = 0xb43c2e0d,
    NvAPI_Reflex_FlashIndicatorSet = 0xbb417425,
    NvAPI_SYS_CheckLicense = 0xbf527664,
    NvAPI_SYS_IdentifyLicense = 0xbf63a7ec,
    NvAPI_SYS_SpbBatchControl = 0xc073cb58,
    NvAPI_SYS_GetDDSInfo = 0xc2b6fb12,
    NvAPI_SYS_SetSmartDimmerLevel = 0xc55abf26,
    NvAPI_SYS_VRRIndicatorState = 0xc6d56f44,
    NvAPI_SYS_GetChipSetSliBondInfo = 0xce8200e8,
    NvAPI_SYS_GetLUIDFromDisplayID = 0xd4a859f2,
    NvAPI_SYS_GetSMPInfo = 0xd5dec731,
    NvAPI_SYS_SpbGetSensorConfig = 0xd6522c3f,
    NvAPI_SYS_NVIF_QuerySupport = 0xd9c0e326,
    NvAPI_SYS_SetMiscLicenseInfo = 0xda425c46,
    NvAPI_SYS_NVIF_GetValues = 0xe5a518dc,
    NvAPI_SYS_GetApprovalCookies = 0xe607d50f,
    NvAPI_SYS_GetMiscLicenseInfo = 0xe62ab414,
    NvAPI_SYS_UIControl_Internal = 0xe75c1922,
    NvAPI_SYS_IsPhysXValidConfig = 0xefb6cde2,
    NvAPI_SYS_SetSMPInfo = 0xf11c960b,
    NvAPI_GPU_SetSmartDimmerConfig = 0xf4c71dec,
    NvAPI_SYS_InternalNvtoppsSetNocatTag = 0xf5cb2b74,
    NvAPI_SYS_DisableDisplayHotkeyHandling = 0xf74926f5,

// --- nvTools.spec (51 IDs) ---
    NvAPI_DPFAKE_CreateModel = 0x0bbcce50,
    NvAPI_DPFAKE_ResetDPAssertBuffer = 0x119b5747,
    NvAPI_DPFAKE_GetFakeDeviceProperties = 0x1ac71f13,
    NvAPI_DPFAKE_QueryDPLogs = 0x1eedc052,
    NvAPI_DPFAKE_CheckSimulationStatus = 0x256d13b8,
    NvAPI_SwInstr_StopCaptureSession = 0x25abda26,
    NvAPI_SwInstr_StartCaptureSession = 0x29e0a109,
    NvAPI_DPFAKE_DestroyModel = 0x3225b2d2,
    NvAPI_Xcode_GetNvapiDMAInfo = 0x33619ac3,
    NvAPI_DIAG_ModeRestrictInfo = 0x361bf51c,
    NvAPI_DirectModeQuery = 0x3762294e,
    NvAPI_Diag_NvRmFree = 0x391728ae,
    NvAPI_Diag_GetDiagnosticData = 0x3c2e04e9,
    NvAPI_DPFAKE_StopSimulation = 0x4370cc33,
    NvAPI_Diag_ResetKMDCoverageData = 0x4a930222,
    NvAPI_SwInstr_GetInterfaceRevision = 0x55a45ad2,
    NvAPI_DPFAKE_GetDeviceConnectionProperties = 0x616aa082,
    NvAPI_ToggleICafeImageDump = 0x62d71566,
    NvAPI_DPFAKE_DisconnectDevice = 0x7acf7e00,
    NvAPI_SwInstr_CloseCaptureSession = 0x7def8998,
    NvAPI_Diag_GetInternalThermalSensorInfo = 0x7e2c4b7b,
    NvAPI_Xcode_GetEncodeInfo = 0x829269f1,
    NvAPI_Diag_GetThermalSensorInfo = 0x86bd66f9,
    NvAPI_Xcode_SetDecodeInfo = 0x87999729,
    NvAPI_DIAG_GetEDCInfo = 0x8afd39c0,
    NvAPI_SwInstr_GetSensorPoints = 0x8b8c90e4,
    NvAPI_DPFAKE_ConnectDevice = 0x94e3d504,
    NvAPI_DIAG_GetRcErrorData = 0x9a60640c,
    NvAPI_Xcode_SetEncodeInfo = 0x9b8bd766,
    NvAPI_ToggleICafeStatsDump = 0x9ce07de0,
    NvAPI_Diag_ResetPexCounters = 0x9e724364,
    NvAPI_Xcode_GetDecodeInfo = 0x9e8029be,
    NvAPI_Diag_NvRmControl = 0x9fa7e206,
    NvAPI_SwInstr_OpenCaptureSession = 0xacce03f4,
    NvAPI_Diag_NvRmAllocRoot = 0xae621cc9,
    NvAPI_GPU_GetTDRInfo = 0xb9582b10,
    NvAPI_SwInstr_GetCapabilities = 0xc0c4c680,
    NvAPI_DPFAKE_NewFakeDevice = 0xc1ce87cd,
    NvAPI_DPFAKE_NotifySimulationSetupCompletion = 0xc95b2d7b,
    NvAPI_Diag_GetPexCounters = 0xcbe4d53c,
    NvAPI_DIAG_DumpAllDisplayIds = 0xce7f37a0,
    NvAPI_Diag_InduceChannelFault = 0xd0ce373b,
    NvAPI_ToggleICafeFrameTag = 0xd4bf6e90,
    NvAPI_Diag_SetHybridDiag = 0xdab71fb6,
    NvAPI_DPFAKE_StartSimulation = 0xdb2d6805,
    NvAPI_Diag_NvRmAlloc = 0xe1977c19,
    NvAPI_GPU_MapGpuTimestampToUsermode = 0xe23fa9a2,
    NvAPI_DPFAKE_ChangeBandwidthOnLink = 0xefcb59bb,
    NvAPI_Diag_Escape = 0xf159df98,
    NvAPI_SwInstr_GetSnapshot = 0xfa66ed29,
    NvAPI_SwInstr_GetCaptureData = 0xfb90bdc1,

// --- nvEvent.spec (6 IDs) ---
    NvAPI_Event_RegisterDriverNotification = 0x0e7ab7e5,
    NvAPI_Event_GetDriverRegisteredClients = 0x7989a457,
    NvAPI_SYS_SetDisplayDeviceInfo = 0x80a5cd5d,
    NvAPI_Event_RegisterForEvents = 0x84244524,
    NvAPI_Event_UnregisterDriverNotification = 0x9be979dc,
    NvAPI_Event_Unregister = 0xcb7b09d2,

// --- nvPerf.spec (80 IDs) ---
    NvAPI_GPU_PerfPstatesGetStatus = 0x03caeb65,
    NvAPI_GPU_PerfSetDefaultMode = 0x0b277a29,
    NvAPI_GPU_PerfPerfLimitsGetStatus = 0x0b62b9e2,
    NvAPI_GPU_PerfPstatesSetControl = 0x0f03dc87,
    NvAPI_GPU_PerfPerfCfPwrModelsGetStatus = 0x10e769c9,
    NvAPI_GPU_PerfChangeSeqGetControl = 0x139c77f6,
    NvAPI_GPU_PerfPerfCfControllerGetInfo = 0x153de751,
    NvAPI_GPU_PerfClientPerfCfPolicyGetStatus = 0x1c21439a,
    NvAPI_GPU_PerfPerfCfPolicyGetInfo = 0x1cac3865,
    NvAPI_GPU_PerfPerfCfControllerSetControl = 0x1f11c1d2,
    NvAPI_GPU_PerfDebugModeGetInfo = 0x1f7bc2b4,
    NvAPI_GPU_PerfPerfCfSensorGetControl = 0x1f9216fb,
    NvAPI_GPU_GetPerfLimit = 0x24245bef,
    NvAPI_GPU_GetVideoEnginePerfSample = 0x27f059b5,
    NvAPI_GPU_PerfPstatesGetControl = 0x2bc18dbd,
    NvAPI_GPU_PerfChangeSeqSetControl = 0x375e26cc,
    NvAPI_GPU_PerfPerfCfSensorGetStatus = 0x39050045,
    NvAPI_GPU_GetPerfPwmInfo = 0x39efde5e,
    NvAPI_GPU_PerfPerfCfSensorSetControl = 0x3b5047c1,
    NvAPI_GPU_PerfPerfCfControllerGetControl = 0x3bd390e8,
    NvAPI_GPU_PerfVideoGetStatus = 0x3d1b9e83,
    NvAPI_GPU_PerfVfTablesGetInfo = 0x3f475f9b,
    NvAPI_GPU_PerfVpstatesGetControl = 0x4150ff5c,
    NvAPI_GPU_SetPerfLimit = 0x4491e797,
    NvAPI_GPU_PerfPerfCfPwrModelGetInfo = 0x484f6825,
    NvAPI_GPU_PerfVfeEquGetControl = 0x4c75c9fe,
    NvAPI_GPU_PerfPerfCfPolicySetControl = 0x4d1c7d6e,
    NvAPI_GPU_PerfPerfLimitsGetInfo = 0x4d2c0a9c,
    NvAPI_GPU_PerfPerfCfPmSensorGetInfo = 0x4fb05d81,
    NvAPI_GPU_PerfPerfCfTopologyGetControl = 0x510e9195,
    NvAPI_GPU_PerfClientPerfCfPolicyGetInfo = 0x531594eb,
    NvAPI_GPU_PerfDebugModeGetStatus = 0x547f2bb0,
    NvAPI_GPU_PerfChangeSeqGetInfo = 0x55590bdb,
    NvAPI_GPU_PerfClientPerfCfPwrModelProfileScale = 0x5a513a89,
    NvAPI_GPU_PerfVfeVarGetControl = 0x5d387298,
    NvAPI_GPU_PerfGetOptpStatus = 0x614c2d7f,
    NvAPI_GPU_PerfVpstatesSetControl = 0x6592ae66,
    NvAPI_GPU_PerfChangeSeqGetStatus = 0x674d172f,
    NvAPI_GPU_PerfVfeEquSetControl = 0x68b798c4,
    NvAPI_GPU_PerfPerfCfPolicyGetControl = 0x69de2c54,
    NvAPI_GPU_PerfVpstatesGetInfo = 0x6d932ec7,
    NvAPI_GPU_PerfClientPerfCfPwrModelProfileScaleSetup = 0x71decba8,
    NvAPI_GPU_PerfPerfCfControllerGetStatus = 0x7520cb28,
    NvAPI_GPU_PerfPerfCfTopologySetControl = 0x75ccc0af,
    NvAPI_GPU_SetPerfLevel = 0x75dd3e6a,
    NvAPI_GPU_GetPerfClockControl = 0x77d8f573,
    NvAPI_GPU_PerfVfeVarSetControl = 0x79fa23a2,
    NvAPI_GPU_PerfModeBoost = 0x7e237d59,
    NvAPI_GPU_PerfPerfLimitsSetControl = 0x8159b63f,
    NvAPI_GPU_PerfRatedTdpGetInfo = 0x87bd35ef,
    NvAPI_GPU_PerfCheckDefaultMode = 0x8aa0e961,
    NvAPI_GPU_PerfVfeEquGetInfo = 0x8d49471c,
    NvAPI_GPU_PerfClientPerfCfControllerGetStatus = 0x97aa9bef,
    NvAPI_GPU_GetPerfSensorCounterInfo = 0x9be35455,
    NvAPI_GPU_PerfDebugModeGetControl_Wrapper = 0x9cd57b5f,
    NvAPI_GPU_PerfDebugModeSetControl_Wrapper = 0xa0bd930d,
    NvAPI_GPU_PerfPerfLimitsGetControl = 0xa59be705,
    NvAPI_GPU_PerfPerfCfSensorGetInfo = 0xa84de4cb,
    NvAPI_GPU_PerfPerfCfPwrModelScale = 0xaa4e0eab,
    NvAPI_GPU_PerfDebugModeGetStatus_LEGACY = 0xb16235c5,
    NvAPI_GPU_PerfDebugModeGetInfo_Wrapper = 0xb3935cc2,
    NvAPI_GPU_PerfVfeVarGetInfo = 0xb9da41d6,
    NvAPI_GPU_GetPerfDecreaseInfo_Internal = 0xc2eba427,
    NvAPI_GPU_GetPerfFstate = 0xc68c3e8d,
    NvAPI_GPU_PerfPerfCfPolicyGetStatus = 0xd3996f8f,
    NvAPI_GPU_PerfDebugModeSetControl = 0xdc13f285,
    NvAPI_GPU_PerfClientPerfCfControllerGetInfo = 0xdc530e6b,
    NvAPI_GPU_PerfDebugModeSetControl_LEGACY = 0xdfa1b288,
    NvAPI_GPU_PerfPerfCfPmSensorGetStatus = 0xec9c8717,
    NvAPI_GPU_PerfRatedTdpGetControl = 0xed2bea09,
    NvAPI_GPU_PerfPmumonPerfCfTopologiesGetSamples = 0xee7e1290,
    NvAPI_GPU_PerfVfChangeInject = 0xf0939208,
    NvAPI_GPU_PerfPmumonPerfPoliciesGetSamples = 0xf5d85e57,
    NvAPI_GPU_PerfPmumonPerfCfPmSensorsGetSamples = 0xf76db8fb,
    NvAPI_GPU_PerfDebugModeGetControl = 0xf8d1a3bf,
    NvAPI_GPU_PerfDebugModeGetControl_LEGACY = 0xf91f7f9b,
    NvAPI_GPU_SetPerfPwmPeriod = 0xfa524578,
    NvAPI_GPU_PerfClientPerfCfPwrModelProfileGetInfo = 0xfb0b1709,
    NvAPI_GPU_PerfRatedTdpGetStatus = 0xfcbdf642,
    NvAPI_GPU_SetPerfClockControl = 0xfe0e5187,

// --- nvNne.spec (4 IDs) ---
    NvAPI_GPU_NneNneLayersGetInfo = 0x3c33fcea,
    NvAPI_GPU_NneNneDescInference = 0x4c347c5f,
    NvAPI_GPU_NneNneDescsGetInfo = 0xcc86d85c,
    NvAPI_GPU_NneNneVarsGetInfo = 0xdda13f96,

// --- nvVio.spec (1 IDs) ---
    NvAPI_CameraTest = 0xfce33ca6,

// --- nvD3D.spec (310 IDs) ---
    NvAPI_D3D12_SetDriverDebugState = 0x00b179ec,
    NvAPI_D3D_GetSmoothAnimationTime = 0x015a2170,
    NvAPI_D3D11_SignalSynchronizationObjectCpu = 0x025dd0ca,
    NvAPI_D3D_DisableSilk = 0x027dade0,
    NvAPI_D3D11_WaitForSynchronizationObjectCpu = 0x0293cc82,
    NvAPI_D3D9_EndShareResource = 0x02b4c430,
    NvAPI_D3D_DestroyPeriodicFrameNotification = 0x02d67589,
    NvAPI_D3D12_RSSetPixelShadingRateSampleOrder = 0x02f7af3c,
    NvAPI_D3D12_QueryAsyncComputeHint = 0x032650af,
    NvAPI_OGL_NsightGetPrivateConstDataSlotAndOffset = 0x03cdc2ec,
    NvAPI_D3D10_GetResourceHandle = 0x03cea1df,
    NvAPI_D3D12_CheckResourceVirtualAddress = 0x04a3f3dc,
    NvAPI_D3D9_NVFBC_NextGen_Entry = 0x0523aa9f,
    NvAPI_D3D11_SignalSynchronizationObjectGpu = 0x053f62d6,
    NvAPI_D3D11_WaitForSynchronizationObjectGpu = 0x05f17e9e,
    NvAPI_D3D9_NVFBC_SysmemToNV12BLVideoSurface = 0x06df696d,
    NvAPI_D3D11_CreateComputeOnlyDevice = 0x07b86467,
    NvAPI_OGL_NsightPushTag = 0x0821e003,
    NvAPI_D3D11_QueryAsyncComputeHint = 0x09f22adf,
    NvAPI_D3D11_SetShaderDebuggerHeapSize = 0x0aeb7668,
    NvAPI_D3D_UpdatePeriodicFrameNotification = 0x0afddcf6,
    NvAPI_D3D11_Aftermath_GetPageFaultInformation = 0x0bba25d7,
    NvAPI_OGL_NsightGetPrivateHandle = 0x0c0cff15,
    NvAPI_D3D12_InvalidateTask = 0x0daf078b,
    NvAPI_OGL_NsightAttachEx = 0x0e82faf0,
    NvAPI_D3D_DirectModeGetDeviceAndSurface = 0x0f215864,
    NvAPI_D3D1x_IFR_ReleaseSession = 0x0fc65236,
    NvAPI_D3D9_FBC_CheckSupport = 0x0fffb6a0,
    NvAPI_D3D12_DestroyHLSLTask = 0x105ecc21,
    NvAPI_D3D9_NVFBC_CaptureBufferToCUDAWithFormat = 0x11a9bd34,
    NvAPI_D3D1X_NVIFR_GetCaps = 0x11d789db,
    NvAPI_D3D10_GetResourceAllocationInfoSize = 0x1432278c,
    NvAPI_D3D_QueryNViewSupport = 0x1482157a,
    NvAPI_D3D9_NVFBC_CreateCaptureBuffer = 0x158bc5c3,
    NvAPI_D3D_SetDriverDebugState = 0x15e50fc9,
    NvAPI_D3D1x_NsightCommunication = 0x16f62c80,
    NvAPI_D3D9_IFR_SetUpTargetBufferToSys_Pvt = 0x1770df6b,
    NvAPI_D3D_IFR_DisconnectCrossProcessSharedSurface = 0x1843a7a0,
    NvAPI_D3D_ForcePerSampleInterlock = 0x19bfdccb,
    NvAPI_D3D10_NsightEnableReporting = 0x1a94f5e3,
    NvAPI_D3D9_NVFBC_GetStatusEx = 0x1c4dfcf4,
    NvAPI_D3D12_DecompressDepthStencilView = 0x1cb72bd7,
    NvAPI_D3D11_NsightEnableReporting = 0x1cd8dc33,
    NvAPI_D3D11_EnableAsyncComputePerfCounters = 0x1d289e10,
    NvAPI_D3D11_InitializeSMPAssist = 0x1d29014a,
    NvAPI_D3D11_Aftermath_GetDeviceStatus = 0x1de221dd,
    NvAPI_D3D1x_QueryLowLatencySupport = 0x1e62257e,
    NvAPI_D3D10_SetPixelShaderInstructions = 0x1ed3ea3f,
    NvAPI_D3D1X_AtomicCopyBuffer = 0x20489f98,
    NvAPI_D3D_CreateStreamProcessor = 0x21581625,
    NvAPI_D3D9_NVFBC_ToSys_SetUpEx = 0x224c1546,
    NvAPI_D3D10_GetResourceAllocationInfo = 0x2361fc95,
    NvAPI_D3D10_GetAllContextRmHandles = 0x250db928,
    NvAPI_D3D_ReleaseDirectModeDisplay = 0x2523cab6,
    NvAPI_D3D12_GetSkedReflectedVA = 0x286add77,
    NvAPI_D3D9_NVFBC_SysmemToCUDAWithFormat = 0x29b60aa8,
    NvAPI_D3D10_SetShaderDebuggerCallback = 0x2acfea19,
    NvAPI_D3D12_DispatchTasks = 0x2c1941b5,
    NvAPI_D3D11_GetPrivateConstDataSlotAndOffset = 0x2d27ef9c,
    NvAPI_D3D_SetNViewMode = 0x2d7be86e,
    NvAPI_D3D9_NVFBC_SetUpToDX9Vid = 0x2e1889f0,
    NvAPI_D3D9_NVFBC_SetUpToNV12BLVideoSurface = 0x2e874f64,
    NvAPI_OGL_NsightAttach = 0x2fbcf41b,
    NvAPI_OGL_NsightSetDrawCallId = 0x32f66fc9,
    NvAPI_D3D11_GetResourceAllocationInfoSize = 0x35bc2bf4,
    NvAPI_D3D12_DevtoolsPushPatchableNops = 0x3736f7a9,
    NvAPI_D3D_CreatePeriodicFrameNotification = 0x396acca7,
    NvAPI_D3D11_TagDepthTextureForImplicitMSAAPromotion = 0x3a36cbe9,
    NvAPI_D3D9_IFR_TransferRenderTarget_Pvt = 0x3a7a1347,
    NvAPI_D3D12_SetAllowBackgroundShaderCompiles = 0x3abffa40,
    NvAPI_D3D1x_IFR_CheckDeviceSupport = 0x3b4719b3,
    NvAPI_OGL_NsightGetDeviceKmtHandle = 0x3b49bd5c,
    NvAPI_D3D12_BindRootArgsToTask = 0x3cb7f4b6,
    NvAPI_D3D12_GetRayTracingStateObjectCuModules = 0x3eda7e38,
    NvAPI_OGL_NsightBuildDebugShaderInstance = 0x415747f7,
    NvAPI_D3D12_HasPendingBackgroundShaderCompiles = 0x41b468ab,
    NvAPI_D3D11_SetPrivateConstData = 0x43f6b55c,
    NvAPI_D3D9_IFR_SetUpTargetBufferToNV12BLVideoSurface_Pvt = 0x44018057,
    NvAPI_D3D11_NsightSetCustomReportData = 0x440aaccb,
    NvAPI_D3D_EnableSilk = 0x47309cbf,
    NvAPI_OGL_NsightSetDeviceStateSaveBuffer = 0x4790186a,
    NvAPI_D3D_GetPeriodicFrameNotificationStats = 0x48179c2e,
    NvAPI_D3D11_SetDriverDebugState = 0x4aba338f,
    NvAPI_D3D12_SetSkedVspanOverflowBuffer = 0x4d8fad1f,
    NvAPI_D3D12_DispatchHLSLTasks = 0x4da09fe9,
    NvAPI_D3D10_GetShaderLocalMemoryAllocationInfo = 0x4f0f3028,
    NvAPI_D3D9_NVFBC_CaptureBufferToDX9Vid = 0x501ff971,
    NvAPI_OGL_NsightGetContextRmHandles = 0x50794994,
    NvAPI_D3D9_BDVMA = 0x52417944,
    NvAPI_D3D1x_IFR_SetUpTargetBufferToSys_Pvt = 0x52f0c937,
    NvAPI_D3D_IFR_CopyFromCrossProcessSharedSurface = 0x544291f6,
    NvAPI_D3D12_CreatePlacedResource = 0x54ec7168,
    NvAPI_D3D11_QuerySMPAssistSupport = 0x5577fd13,
    NvAPI_D3D1x_NsightEnableDebugMode = 0x56cd1d16,
    NvAPI_D3D_DirectModeSetDisplayMode = 0x56df9d51,
    NvAPI_D3D11_GetMsHybridActivitiesInfo = 0x57ae9183,
    NvAPI_D3D11_NsightSetCurrentContextAndD3DCallCount = 0x58045f57,
    NvAPI_D3D12_GetTaskScratchSize = 0x5a7f41a1,
    NvAPI_D3D11_NsightCommunication = 0x5a87e539,
    NvAPI_D3D9_NVIFR_ReleaseSession = 0x5ab2dd42,
    NvAPI_D3D10_SetDeviceStateSaveBuffer = 0x5abae412,
    NvAPI_OGL_NsightEnableDebugMode = 0x5afceef7,
    NvAPI_D3D11_GetPixelShaderHandle = 0x5b40c835,
    NvAPI_D3D_DestroyStreamProcessor = 0x5c29cbad,
    NvAPI_D3D10_BuildDebugShaderInstance2 = 0x5c67a9d6,
    NvAPI_D3D11_GetGeometryShaderHandle = 0x5d273987,
    NvAPI_D3D11_NsightFlushReporting = 0x5d886a04,
    NvAPI_D3D_DirectModePresent12 = 0x5ed18dc4,
    NvAPI_D3D1x_IFR_TransferRenderTargetToNV12BLVideoSurface = 0x6126010d,
    NvAPI_D3D9_NVFBC_CaptureBufferToNV12BLVideoSurface = 0x612cdac5,
    NvAPI_D3D9_NVFBC_FORCE_GRAB_FULL_FRAME = 0x62050534,
    NvAPI_D3D1x_GetAliasSurfaceHandle = 0x629519af,
    NvAPI_D3D9_NVFBC_CaptureBufferToCUDAWithFormat_2 = 0x62f2fbc5,
    NvAPI_D3D12_Aftermath_GetDeviceStatus = 0x633d88e1,
    NvAPI_D3D12_Aftermath_GetPageFaultInformation = 0x6446beb8,
    NvAPI_D3D_CudaInteropFunction = 0x64d6c83d,
    NvAPI_D3D10_GetAllocDebugInfo = 0x6670b9e7,
    NvAPI_D3D9_NVFBC_MOUSE_CAPTURE = 0x667bee91,
    NvAPI_D3D_IFR_UpdateSessionCount = 0x687e18db,
    NvAPI_D3D11_GetBufferStreamOutBytesWritten = 0x688c0647,
    NvAPI_D3D_NVFBC_ChangeState = 0x68cfa912,
    NvAPI_D3D11_GetShaderLocalMemoryAllocationInfo = 0x68dfc3ce,
    NvAPI_D3D12_Devtools_DestroyContext = 0x69364d3f,
    NvAPI_D3D9_NVFBC_GetCaps = 0x6937c8d1,
    NvAPI_D3D11_OpenResourceOnComputeDevice = 0x69c2d61e,
    NvAPI_D3D11_GetComputeShaderHandle = 0x6a6d8127,
    NvAPI_D3D11_OpenSharedSynchronizationObject = 0x6bf6f893,
    NvAPI_D3D11_GetAllocDebugInfo = 0x6c6b31be,
    NvAPI_D3D9_NVFBC_MOUSE_CAPTURE_SETUP = 0x6c8af10c,
    NvAPI_D3D11_GetNeedsAppFPBlendClamping = 0x6d67533c,
    NvAPI_D3D1x_IFR_SetUpTargetBufferToNV12BLVideoSurface_Pvt = 0x6d9324cd,
    NvAPI_D3D_PerformPostProcessOps = 0x6dd888e4,
    NvAPI_D3D_DirectModeImplicitSLIControl = 0x70703f49,
    NvAPI_D3D12_SetDriverDebugStateCQ = 0x70a08e64,
    NvAPI_D3D1x_IFR_SetUpVideoTargetBuffer_Pvt = 0x71018daa,
    NvAPI_D3D1x_MoveShaderCacheBetweenVidAndSys = 0x735d6473,
    NvAPI_D3D11_GetAllContextRmHandles = 0x73a2249a,
    NvAPI_D3D11_BindCblResources = 0x74154e52,
    NvAPI_D3D9_EnableStereoOverlay = 0x74ee6b3f,
    NvAPI_D3D12_InitCubinTask = 0x75b8e639,
    NvAPI_D3D11_CreateSynchronizationObject = 0x76101e1f,
    NvAPI_D3D10_UnlockCb = 0x76a3261a,
    NvAPI_D3D10_NsightSetCurrentContextAndD3DCallCount = 0x77646004,
    NvAPI_D3D10_UnregisterDevice = 0x77c9a2d0,
    NvAPI_D3D_SetCurrentSwapChain = 0x7922a53c,
    NvAPI_D3D_DirectModeGetDisplayModes = 0x79c3fe22,
    NvAPI_D3D9_IFR_CopyToSharedSurface_Pvt = 0x7a1ac6e1,
    NvAPI_D3D10_GetBufferStreamOutBytesWritten = 0x7a62d510,
    NvAPI_OGL_NsightSetShaderDebuggerCallback = 0x7defcfba,
    NvAPI_D3D12_TagDepthTextureForImplicitMSAAPromotion = 0x7e023eb9,
    NvAPI_D3D12_DevtoolsPushPatchableMethods = 0x7f86bd52,
    NvAPI_D3D9_IFR_DestroySharedSurface = 0x806a72d1,
    NvAPI_D3D_DirectModeRenderWait = 0x81caf891,
    NvAPI_OGL_NsightEnableWarpSemaphoreReports = 0x81e3f9b1,
    NvAPI_D3D_IFR_DestroyCrossProcessSharedSurface = 0x8307d18d,
    NvAPI_D3D9_NVFBC_SetHWCursorCapture = 0x8309a65d,
    NvAPI_D3D12_GetFlipPatternInCurrentMode = 0x85edb738,
    NvAPI_D3D10_SetPrivateConstData = 0x864f7810,
    NvAPI_D3D_DirectModeCreateSurface = 0x86bdb518,
    NvAPI_D3D9_NVFBC_CaptureBufferToSys_2 = 0x87a6b934,
    NvAPI_D3D11_SetSMDisableMask = 0x8898997d,
    NvAPI_D3D11_BuildDebugShaderInstance2 = 0x8a6dc5af,
    NvAPI_D3D12_DispatchGraphics = 0x8ab10c89,
    NvAPI_D3D_VRPerfLevelControl = 0x8b5ed062,
    NvAPI_D3D12_DevtoolsGetCommandList = 0x8bbad2b8,
    NvAPI_OGL_NsightSetShaderDebuggerHeapSize = 0x8bc13fb2,
    NvAPI_D3D_DirectModeGetHDCPStatus = 0x8c4375de,
    NvAPI_D3D12_Aftermath_SetMarker = 0x8c68f0f1,
    NvAPI_D3D_DirectModePresent = 0x8c84a92a,
    NvAPI_D3D1x_CreateLowLatencyDeviceHint = 0x8ea884c0,
    NvAPI_D3D_IFR_ConnectToCrossProcessSharedSurface = 0x8f58e920,
    NvAPI_D3D12_GetExperimentalFeatureEtbl = 0x8f6beaec,
    NvAPI_D3D10_GetGeometryShaderHandle = 0x90ab6d32,
    NvAPI_D3D10_NsightSetCustomReportData = 0x9200c0b2,
    NvAPI_OGL_NsightDetach = 0x92eba502,
    NvAPI_D3D_CopyMSAADataForImplicitlyPromotedDepthTexture = 0x93829529,
    NvAPI_D3D_SetDeviceHint = 0x93a00243,
    NvAPI_D3D9_NVFBC_SetUpToSys = 0x93ac4616,
    NvAPI_D3D12_GetDevice = 0x9531657c,
    NvAPI_D3D11_Aftermath_GetContextData = 0x96161acb,
    NvAPI_D3D11_GetVertexShaderHandle = 0x988775c0,
    NvAPI_D3D9_NVFBC_SetUpToNV12BLVideoSurfaceEx = 0x989d9ddd,
    NvAPI_D3D10_GetResourceAllocationInfo2 = 0x98db138c,
    NvAPI_D3D9_NVFBC_CaptureBufferToCUDA = 0x995c4a80,
    NvAPI_D3D11_GetResourceAllocationInfo2 = 0x99aa6cd7,
    NvAPI_D3D11_GetDevice = 0x9b05f0f9,
    NvAPI_OGL_NsightSetCustomReportData = 0x9b5aab97,
    NvAPI_D3D9_IFR_CopyFromSharedSurface = 0x9d0d31ad,
    NvAPI_D3D10_GetVertexShaderHandle = 0x9ecb5c10,
    NvAPI_D3D1x_HintCreateLowLatencyDevice = 0x9eead6e5,
    NvAPI_D3D9_NVFBC_CaptureBufferToSys = 0x9fc72559,
    NvAPI_D3D11_GetCblToUmdIf = 0xa052fe50,
    NvAPI_D3D12_RSGetPixelShadingRateSampleOrder = 0xa291daa7,
    NvAPI_D3D_SetServerThreadPriority = 0xa2ca8bcf,
    NvAPI_D3D12_IsNvMeshShaderSupported = 0xa47716f8,
    NvAPI_OGL_NsightFlushReporting = 0xa48cccf4,
    NvAPI_D3D_GetDeviceKmtHandle = 0xa4ad0870,
    NvAPI_D3D10_GetResourceSubresourceInfo2 = 0xa66bc2f0,
    NvAPI_D3D11_CreateMultiSampledUAV = 0xa6e2c73a,
    NvAPI_D3D_TagFrameWithAnimationTime = 0xa7bee7ac,
    NvAPI_D3D12_SetDriverDebugStringCQ = 0xa829b223,
    NvAPI_D3D10_GetPixelShaderHandle = 0xa8c27fc7,
    NvAPI_D3D9_NVFBC_DestroyCaptureBuffer = 0xa993c162,
    NvAPI_D3D11_GetCommandList = 0xab6c6010,
    NvAPI_D3D_GetVRRState = 0xac00800a,
    NvAPI_D3D_GetDirectModePresentStats = 0xace116bc,
    NvAPI_D3D12_Devtools_CreateContext = 0xad0a8e39,
    NvAPI_D3D9_GetOSCSurfaceInfo = 0xadc65d3d,
    NvAPI_D3D12_RSSetExclusiveScissorRects = 0xaddef202,
    NvAPI_D3D10_NsightFlushReporting = 0xae0addf6,
    NvAPI_OGL_NsightGetDriverResourceInfo = 0xae63b9e0,
    NvAPI_D3D10_GetPixelShaderInstructions = 0xae6ad564,
    NvAPI_D3D10_GetShaderUCodeAllocationInfo = 0xae6c7fb5,
    NvAPI_D3D_VRSetPowerMode = 0xae9d975b,
    NvAPI_D3D12_SetCWDRefcount = 0xaeb4c6b9,
    NvAPI_D3D1x_SetAliasSurfaceCreation = 0xb1ee38d9,
    NvAPI_D3D9_IFR_TransferRenderTargetToNV12BLVideoSurface_Pvt = 0xb26ed0a0,
    NvAPI_D3D1X_ResourceInvalidate = 0xb2a1694b,
    NvAPI_D3D12_Aftermath_GetContextData = 0xb2e3e2a2,
    NvAPI_D3D1x_IFR_SetUpTargetBufferToNV12BLVideoSurface = 0xb2e98ab8,
    NvAPI_D3D1x_GetLowLatencySupport = 0xb2edaa72,
    NvAPI_OGL_NsightEnableReporting = 0xb30f2135,
    NvAPI_D3D12_ForceLoadBalanceMode = 0xb4345710,
    NvAPI_D3D10_GetPrivateConstDataSlotAndOffset = 0xb7f46cf3,
    NvAPI_D3D_GetIDXGIAdapter = 0xb7fbdbfa,
    NvAPI_D3D1x_DeclareVRProcessType = 0xb8a4ca1c,
    NvAPI_D3D10_CreateDevice_McCompat = 0xb99b688a,
    NvAPI_D3D12_RTXSetDispatchResultBuffer = 0xba88626c,
    NvAPI_D3D11_SetDeviceStateSaveBuffer = 0xbaa94758,
    NvAPI_D3D10_LockCb = 0xbbc0380c,
    NvAPI_D3D12_RTXSetInstrumentedPipelineCreation = 0xbc3fe2a4,
    NvAPI_D3D9_IFR_CreateSharedSurface = 0xbca31366,
    NvAPI_D3D11_WksReadScanout = 0xbcb1c536,
    NvAPI_D3D1x_NsightPushMethodIntoPushBuffer = 0xbec02eb1,
    NvAPI_D3D12_RSSetShadingRateResourceView = 0xc0546d17,
    NvAPI_D3D11_GetAsyncComputePerfCounters = 0xc060ca16,
    NvAPI_OGL_NsightPop = 0xc0c3c41f,
    NvAPI_D3D9_NVFBC_Prefilter_CaptureBufferToNV12BLVideoSurface = 0xc0d3c8dd,
    NvAPI_D3D_IFR_CopyToCrossProcessSharedSurface = 0xc193e62d,
    NvAPI_D3D12_CreatePipelineState = 0xc2500e4f,
    NvAPI_D3D12_GetCblToUmdIf = 0xc383c071,
    NvAPI_D3D10_GetResourceSubresourceInfo = 0xc484a232,
    NvAPI_D3D_AcquireDirectModeDisplay = 0xc4e4efda,
    NvAPI_D3D11_DevtoolsInvokeFunctor = 0xc53e7ab5,
    NvAPI_D3D12_SetAsyncComputeControlData = 0xc5f0365e,
    NvAPI_D3D11_GetResourceSubresourceInfo = 0xc5f5dd69,
    NvAPI_D3D11_SetAsyncComputeControlData = 0xc663b7b3,
    NvAPI_D3D11_Aftermath_SetMarker = 0xc663ba92,
    NvAPI_D3D11_EnableWarpSemaphoreReports = 0xc667dee8,
    NvAPI_D3D_DirectModeDestroySurface = 0xc708c006,
    NvAPI_D3D10_EnableWarpSemaphoreReports = 0xc716a1b3,
    NvAPI_D3D9_NVFBC_SetUpToCUDA = 0xc7339ee4,
    NvAPI_D3D_SetDriverDebugString = 0xc7fb9e5d,
    NvAPI_D3D11_RegisterDeviceFromOGL = 0xc83c4d5d,
    NvAPI_D3D9_IFR_CreateSharedSurface_Pvt = 0xc850ee0b,
    NvAPI_OGL_NsightCommunication = 0xc8527e9f,
    NvAPI_D3D12_SetDriverDebugString = 0xc8590b46,
    NvAPI_D3D11_DestroySharedSynchronizationObject = 0xc8de469b,
    NvAPI_D3D11_SetBufferStreamOutBytesWritten = 0xc8ea73dc,
    NvAPI_OGL_NsightGetDriverResourceInfoSize = 0xc91df792,
    NvAPI_D3D_QueryPeriodicFrameNotificationSupport = 0xc975c0b1,
    NvAPI_D3D11_RegisterContext = 0xc99f4a67,
    NvAPI_D3D11_Aftermath_Initialize = 0xcba3f913,
    NvAPI_D3D1x_IFR_TransferRenderTargetToNV12BLVideoSurface_Pvt = 0xcc6275a9,
    NvAPI_D3D1x_IFR_TransferRenderTarget_Pvt = 0xcc6f1bfd,
    NvAPI_D3D9_ReleaseOSCSurface = 0xcd3e15cd,
    NvAPI_D3D12_RSSetViewportsPixelShadingRates = 0xcdf7571a,
    NvAPI_D3D12_DestroyCubinTask = 0xd10b4c28,
    NvAPI_OGL_NsightGetDeviceHandles = 0xd1556c79,
    NvAPI_D3D11_CreatePixelShaderEx = 0xd1747d38,
    NvAPI_D3D11_GetResourceSubresourceInfo2 = 0xd3d58447,
    NvAPI_D3D_UpdateSLIMask = 0xd451e834,
    NvAPI_D3D9_IFR_CopyFromSharedSurface_Pvt = 0xd5119dd6,
    NvAPI_D3D9_IFR_DestroySharedSurface_Pvt = 0xd557bd64,
    NvAPI_D3D11_GetComputeShaderInfo = 0xd74d35b7,
    NvAPI_D3D11_RSSetViewportsEx = 0xd77ef2b4,
    NvAPI_D3D11_WaitSharedSynchronizationObject = 0xd79d304e,
    NvAPI_D3D9_BeginShareResource = 0xd8070bb3,
    NvAPI_D3D11_SetDriverDebugString = 0xd81fcee7,
    NvAPI_D3D10_SetBufferStreamOutBytesWritten = 0xda04a08b,
    NvAPI_D3D11_DestroySynchronizationObject = 0xda5b591e,
    NvAPI_D3D12_GetCommandList = 0xdbb4a15e,
    NvAPI_D3D12_Aftermath_Initialize = 0xdbe53cb2,
    NvAPI_D3D9_IFR_CopyToSharedSurface = 0xdc0bd667,
    NvAPI_D3D10_RegisterDevice = 0xdcda434d,
    NvAPI_D3D10_SetShaderDebuggerHeapSize = 0xdce11a11,
    NvAPI_OGL_NsightGetProgramiv = 0xdd5c5f1a,
    NvAPI_D3D12_Devtools_UploadShader = 0xde86a016,
    NvAPI_D3D11_GetDomainShaderHandle = 0xdeb20047,
    NvAPI_D3D_FramePresentNotify = 0xe025a372,
    NvAPI_D3D11_CreateSharedSynchronizationObject = 0xe0cac5c7,
    NvAPI_D3D11_SignalSharedSynchronizationObject = 0xe19b2cb0,
    NvAPI_OGL_NsightMoveShaderCacheBetweenVidAndSys = 0xe23b5cb6,
    NvAPI_OGL_NsightSetPrivateConstData = 0xe54e2339,
    NvAPI_D3D_SetVRRState = 0xe5767a36,
    NvAPI_OGL_NsightSetGlobalCustomReportData = 0xe68105bb,
    NvAPI_D3D11_GetShaderUCodeAllocationInfo = 0xe75bbe39,
    NvAPI_D3D_GetFlipPattern = 0xe8a59f97,
    NvAPI_D3D11_GetHullShaderHandle = 0xe8b1fd8b,
    NvAPI_OGL_NsightDisableReporting = 0xea54e430,
    NvAPI_D3D_IFR_CreateCrossProcessSharedSurface = 0xeead8305,
    NvAPI_D3D1x_SetSystemMemorySurfaceCreation = 0xef88f7b9,
    NvAPI_D3D_IsDeviceSandbaggedByDefault = 0xf080c99c,
    NvAPI_D3D_CudaInteropGetObjectHandle = 0xf0f4a5e0,
    NvAPI_D3D12_RegisterContext = 0xf1ea1980,
    NvAPI_D3D11_GetResourceAllocationInfo = 0xf56b90ec,
    NvAPI_D3D12_CreateShadingRateResourceView = 0xfb583977,
    NvAPI_D3D12_CreateHLSLTask = 0xfc82f9d9,
    NvAPI_D3D11_SetShaderDebuggerCallback = 0xfcc58660,
    NvAPI_D3D12_CreateCubinTask = 0xfd9b0935,

// --- nvPcf.spec (7 IDs) ---
    NvAPI_PCF_MasterSetControl = 0x0bc681fb,
    NvAPI_PCF_MasterGetControl = 0x2f04d0c1,
    NvAPI_PCF_ConfigGetInfo = 0x80e9a056,
    NvAPI_PCF_ControllerGetStatus = 0x954df4fb,
    NvAPI_PCF_CpuIntelTurboRatioSetControl = 0x9873472b,
    NvAPI_PCF_CpuIntelTurboRatioGetControl = 0xbcb11611,
    NvAPI_PCF_DynamicBoostGetStatus = 0xc80068a1,
}
