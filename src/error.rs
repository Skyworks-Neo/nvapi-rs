use crate::{Status, sys};
use std::cell::RefCell;
use std::convert::Infallible;
use std::error::Error as StdError;
use std::fmt;
use sys::ArgumentRangeError;

thread_local! {
    /// Most recent NVAPI status failure on this thread, preformatted via
    /// [`NvapiError`]'s Display. Every failed call funnels through
    /// [`status_result`], so wrapper layers that swallow the error into
    /// `Option::None` (rendered as "supported: no") still leave the
    /// ORIGINAL status here for the CLI renderers to surface.
    static LAST_STATUS_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Record a status failure (called from [`status_result`]).
fn record_status_error(err: &NvapiError) {
    LAST_STATUS_ERROR.with(|slot| *slot.borrow_mut() = Some(err.to_string()));
}

/// The most recent NVAPI status failure on this thread, if any.
pub fn last_status_error() -> Option<String> {
    LAST_STATUS_ERROR.with(|slot| slot.borrow().clone())
}

/// Forget the recorded failure — call at the start of each top-level
/// command so any annotation describes the current run only.
pub fn clear_status_error() {
    LAST_STATUS_ERROR.with(|slot| *slot.borrow_mut() = None);
}

pub fn status_result(nvid: sys::Api, status: sys::NvAPI_Status) -> Result<(), NvapiError> {
    match sys::Status::from_raw(status) {
        Ok(sys::Status::Ok) => Ok(()),
        Ok(status) => {
            let err = NvapiError::new(nvid, status);
            record_status_error(&err);
            Err(err)
        }
        // the driver returned a code outside the known table (newer driver or
        // NDA surface) — keep the raw code instead of collapsing it to
        // `Status::Error` where it is lost forever
        Err(_) => {
            let err = NvapiError {
                nvid,
                status: sys::Status::Error,
                raw_status: Some(status),
            };
            record_status_error(&err);
            Err(err)
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Nvapi(NvapiError),
    ArgumentRange(ArgumentRangeError),
}

impl Error {
    pub fn nvapi_status(&self) -> Option<Status> {
        match self {
            Error::Nvapi(e) => Some(e.status),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct NvapiError {
    pub nvid: sys::Api,
    pub status: Status,
    /// The raw driver status code, set only when it has no known `Status`
    /// mapping (`status` is then `Status::Error`). Recognized statuses
    /// carry `None`.
    pub raw_status: Option<sys::NvAPI_Status>,
}

impl NvapiError {
    pub fn new(nvid: sys::Api, status: Status) -> Self {
        Self {
            nvid,
            status,
            raw_status: None,
        }
    }
}

impl StdError for NvapiError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.status as _)
    }
}

impl fmt::Display for NvapiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} failed: {}", self.nvid, self.status)?;
        if let Some(raw) = self.raw_status {
            write!(f, " (raw status {} / {:#x})", raw, raw)?;
        }
        Ok(())
    }
}

impl From<Infallible> for NvapiError {
    fn from(e: Infallible) -> Self {
        match e {}
    }
}

impl From<Infallible> for Error {
    fn from(e: Infallible) -> Self {
        match e {}
    }
}

impl From<NvapiError> for Error {
    fn from(e: NvapiError) -> Self {
        Error::Nvapi(e)
    }
}

impl From<ArgumentRangeError> for Error {
    fn from(e: ArgumentRangeError) -> Self {
        Error::ArgumentRange(e)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(match self {
            Error::Nvapi(e) => e as _,
            Error::ArgumentRange(e) => e as _,
        })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Nvapi(e) => fmt::Display::fmt(e, f),
            Error::ArgumentRange(e) => fmt::Display::fmt(e, f),
        }
    }
}
