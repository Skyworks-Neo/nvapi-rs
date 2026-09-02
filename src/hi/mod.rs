mod gpu;
pub use gpu::*;

pub use crate::{
    ClkVfControlPointPrivate, ClkVfControlPrivate, ClkVfDomainClass, ClkVfDomainHint,
    ClkVfPointPrivate, ClkVfPointsPrivate, ClkVfRawRecord, ClkVfSegmentKind, Error, NvapiError,
    Result, Status, chipset_info, driver_version, error_message, initialize, interface_version,
    sys, unload,
};

use std::result::Result as StdResult;

/// `NvidiaDeviceNotFound` joins NotSupported/NoImplementation in the
/// "feature absent on this GPU" set: on TCC devices (Tesla compute cards,
/// live P100 / 582.41) per-GPU informational calls whose concept does not
/// exist without a WDDM display stack — GetDriverModel,
/// GetConnectedDisplayIds, … — return -6 even though the handle is valid
/// and every other query succeeds. Hard-failing the whole aggregated query
/// (info()) on an absent sub-feature made TCC GPUs unusable for every
/// command that touches info().
pub fn allowable_result_fallback<T, E: Into<Error>>(v: StdResult<T, E>, fallback: T) -> Result<T> {
    match v.map_err(Into::into) {
        Ok(v) => Ok(v),
        Err(Error::Nvapi(NvapiError {
            status: Status::NotSupported,
            ..
        }))
        | Err(Error::Nvapi(NvapiError {
            status: Status::NoImplementation,
            ..
        }))
        | Err(Error::Nvapi(NvapiError {
            status: Status::NvidiaDeviceNotFound,
            ..
        }))
        | Err(Error::Nvapi(NvapiError {
            status: Status::ArgumentExceedMaxSize,
            ..
        }))
        | Err(Error::ArgumentRange(..)) => Ok(fallback),
        Err(e) => Err(e),
    }
}

pub fn allowable_result<T, E: Into<Error>>(v: StdResult<T, E>) -> Result<Result<T>> {
    match v.map_err(Into::into) {
        Ok(v) => Ok(Ok(v)),
        Err(
            e @ Error::Nvapi(NvapiError {
                status: Status::NotSupported,
                ..
            }),
        )
        | Err(
            e @ Error::Nvapi(NvapiError {
                status: Status::NoImplementation,
                ..
            }),
        )
        | Err(
            e @ Error::Nvapi(NvapiError {
                status: Status::NvidiaDeviceNotFound,
                ..
            }),
        )
        | Err(e @ Error::ArgumentRange(..)) => Ok(Err(e)),
        Err(e) => Err(e),
    }
}
