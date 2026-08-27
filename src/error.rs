use crate::{Status, sys};
use std::convert::Infallible;
use std::error::Error as StdError;
use std::fmt;
use sys::ArgumentRangeError;

pub fn status_result(nvid: sys::Api, status: sys::NvAPI_Status) -> Result<(), NvapiError> {
    match sys::Status::from_raw(status) {
        Ok(sys::Status::Ok) => Ok(()),
        Ok(status) => Err(NvapiError::new(nvid, status)),
        // the driver returned a code outside the known table (newer driver or
        // NDA surface) — keep the raw code instead of collapsing it to
        // `Status::Error` where it is lost forever
        Err(_) => Err(NvapiError {
            nvid,
            status: sys::Status::Error,
            raw_status: Some(status),
        }),
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
