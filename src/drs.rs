//! Driver-settings (DRS) registry access — minimal surface for the
//! driver-wide Frame Rate Limiter, exactly the path MSI Afterburner
//! 4.6.7.19412 uses (RE'd from RTHAL: setting id `0x10834FEE`, SET writes
//! `fps | 0xF0000000` into the setting value dword, the u16 half is the fps;
//! any GetSetting failure means "no limit set" in AB and is mapped to
//! `Ok(None)` here — the raw status stays reachable via the last-error
//! ledger convention of the surrounding calls).
//!
//! The DRS registry is process/driver-global — no GPU handle involved.

use crate::sys::api;
use crate::sys::drs::NV_DRS_SETTING;
use crate::sys::handles::{NvDRSProfileHandle, NvDRSSessionHandle};
use crate::sys::status_result as raw_status_result;

/// DRS setting id AB 19412 drives for the global frame rate limiter.
pub const FRAME_RATE_LIMIT_SETTING_ID: u32 = 0x1083_4FEE;
/// High flag word AB ORs into the setting value (`fps | 0xF0000000`).
pub const FRAME_RATE_LIMIT_FLAG: u32 = 0xF000_0000;

fn with_session<T>(
    f: impl FnOnce(NvDRSSessionHandle, NvDRSProfileHandle) -> crate::Result<T>,
) -> crate::Result<T> {
    let mut session = NvDRSSessionHandle::default();
    unsafe { nvcall!(NvAPI_DRS_CreateSession(&mut session))? };
    let result = (|| {
        unsafe { nvcall!(NvAPI_DRS_LoadSettings(session))? };
        let mut profile = NvDRSProfileHandle::default();
        unsafe { nvcall!(NvAPI_DRS_GetBaseProfile(session, &mut profile))? };
        f(session, profile)
    })();
    // DestroySession on every path; a failing destroy must not mask the inner
    // result, but is itself worth surfacing when the inner run succeeded.
    let destroyed =
        raw_status_result(unsafe { api::NvAPI_DRS_DestroySession(session) }).map_err(|status| {
            crate::Error::Nvapi(crate::NvapiError::new(
                crate::sys::Api::NvAPI_DRS_DestroySession,
                status,
            ))
        });
    match (result, destroyed) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), _) => Err(e),
        (Ok(_), Err(e)) => Err(e),
    }
}

/// Read the global frame rate limiter fps cap; `None` when unset/failed
/// (AB-faithful: the ref implementation returns 0 on any GetSetting error).
pub fn framerate_limit() -> crate::Result<Option<u32>> {
    with_session(|session, profile| {
        let mut setting = NV_DRS_SETTING::default();
        let status = unsafe {
            api::NvAPI_DRS_GetSetting(
                session,
                profile,
                FRAME_RATE_LIMIT_SETTING_ID,
                core::ptr::from_mut(&mut setting).cast(),
            )
        };
        if raw_status_result(status).is_err() {
            return Ok(None);
        }
        let fps = setting.value & !FRAME_RATE_LIMIT_FLAG;
        Ok(if fps == 0 { None } else { Some(fps) })
    })
}

/// Set the global frame rate limiter to `Some(fps)` fps, or clear it with
/// `None` (AB: value 0 deletes the DRS setting instead of writing 0).
/// Persists to the driver registry (SaveSettings) — takes effect on new
/// 3D contexts.
pub fn set_framerate_limit(fps: Option<u32>) -> crate::Result<()> {
    with_session(|session, profile| match fps {
        Some(fps) => {
            let mut setting = NV_DRS_SETTING::default();
            setting.setting_id = FRAME_RATE_LIMIT_SETTING_ID;
            setting.value = (fps & !FRAME_RATE_LIMIT_FLAG) | FRAME_RATE_LIMIT_FLAG;
            unsafe {
                nvcall!(NvAPI_DRS_SetSetting(
                    session,
                    profile,
                    core::ptr::from_ref(&setting).cast()
                ))?;
            };
            unsafe { nvcall!(NvAPI_DRS_SaveSettings(session)) }.map_err(crate::Error::from)
        }
        None => {
            unsafe {
                nvcall!(NvAPI_DRS_DeleteProfileSetting(
                    session,
                    profile,
                    FRAME_RATE_LIMIT_SETTING_ID
                ))?;
            };
            unsafe { nvcall!(NvAPI_DRS_SaveSettings(session)) }.map_err(crate::Error::from)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::drs::NV_DRS_SETTING;

    const _: () = assert!(size_of::<NV_DRS_SETTING>() == 0x3020);

    #[test]
    fn drs_setting_default_stamps_ab_version() {
        // AB 19412 Get/SetSetting both stamp 0x13020 (size 0x3020 | v1).
        let setting = NV_DRS_SETTING::default();
        assert_eq!(setting.version.data, 0x13020);
    }

    #[test]
    fn fps_encoding_roundtrip() {
        // SET writes fps | FLAG; GET masks the flag back off (u16 half).
        let raw = 165u32 | FRAME_RATE_LIMIT_FLAG;
        assert_eq!(raw & !FRAME_RATE_LIMIT_FLAG, 165);
        assert_eq!(set_encoding(165), 165 | FRAME_RATE_LIMIT_FLAG);
    }

    fn set_encoding(fps: u32) -> u32 {
        (fps & !FRAME_RATE_LIMIT_FLAG) | FRAME_RATE_LIMIT_FLAG
    }
}

/// Whether this driver carries the legacy FRL DRS setting at all (AB parity
/// with `IsFramerateLimitControlSupported`: the limiter UI only lights up
/// when GetSetting answers). False on branches where SetSetting would fail
/// with `NVAPI_SETTING_NOT_FOUND` (-160).
pub fn framerate_limit_supported() -> crate::Result<bool> {
    with_session(|session, profile| {
        let mut setting = NV_DRS_SETTING::default();
        let status = unsafe {
            api::NvAPI_DRS_GetSetting(
                session,
                profile,
                FRAME_RATE_LIMIT_SETTING_ID,
                core::ptr::from_mut(&mut setting).cast(),
            )
        };
        Ok(raw_status_result(status).is_ok())
    })
}
