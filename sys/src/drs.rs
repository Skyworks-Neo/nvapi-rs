//! Driver Settings (DRS) registry — the profile/setting store behind
//! per-application driver options. IDs are registered in [`crate::nvid`]
//! (`NvAPI_DRS_*`). MSI Afterburner 4.6.7.19412 implements its Framerate
//! Limit feature directly on this family (RE'd from RTHAL 19412:
//! CreateSession → LoadSettings → GetBaseProfile → Get/SetSetting →
//! SaveSettings → DestroySession, setting id 0x10834FEE, value dword
//! `fps | 0xF0000000`).

use crate::handles::{NvDRSProfileHandle, NvDRSSessionHandle};
use crate::prelude_::*;

pub const NVAPI_DRS_SETTING_VER1: u32 = 0x3020;

nvstruct! {
    /// `NvDRSSetting` v1 as driven by AB 19412's FramerateLimit path: a
    /// 12320-byte buffer struct whose only fields AB touches are settingId
    /// (dword @0x1004), a zero word @0x1008, and the setting value
    /// (dword @0x201C — SET writes `fps | 0xF0000000`, GET returns the low
    /// u16 as the fps cap). Everything else is left zeroed.
    pub struct NV_DRS_SETTING_V1 {
        pub version: NvVersion,
        pub unknown0: Padding<[u8; 0x1000]>,
        pub setting_id: u32,
        pub unknown1: u32,
        pub unknown2: Padding<[u8; 0x1010]>,
        pub value: u32,
        pub unknown3: Padding<[u8; 0x1000]>,
    }
}

nvversion! { @=NV_DRS_SETTING NV_DRS_SETTING_V1(1) = 0x3020 }

nvapi! {
    pub unsafe fn NvAPI_DRS_CreateSession(phSession: *mut NvDRSSessionHandle) -> NvAPI_Status;
}

nvapi! {
    pub unsafe fn NvAPI_DRS_DestroySession(hSession: NvDRSSessionHandle) -> NvAPI_Status;
}

nvapi! {
    pub unsafe fn NvAPI_DRS_LoadSettings(hSession: NvDRSSessionHandle) -> NvAPI_Status;
}

nvapi! {
    pub unsafe fn NvAPI_DRS_SaveSettings(hSession: NvDRSSessionHandle) -> NvAPI_Status;
}

nvapi! {
    pub unsafe fn NvAPI_DRS_GetBaseProfile(hSession: NvDRSSessionHandle, phProfile: *mut NvDRSProfileHandle) -> NvAPI_Status;
}

nvapi! {
    pub unsafe fn NvAPI_DRS_GetSetting(hSession: NvDRSSessionHandle, hProfile: NvDRSProfileHandle, settingId: u32, pSetting: *mut NV_DRS_SETTING) -> NvAPI_Status;
}

nvapi! {
    pub unsafe fn NvAPI_DRS_SetSetting(hSession: NvDRSSessionHandle, hProfile: NvDRSProfileHandle, pSetting: *const NV_DRS_SETTING) -> NvAPI_Status;
}

nvapi! {
    pub unsafe fn NvAPI_DRS_DeleteProfileSetting(hSession: NvDRSSessionHandle, hProfile: NvDRSProfileHandle, settingId: u32) -> NvAPI_Status;
}
