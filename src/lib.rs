//#![deny(missing_docs)]
#![doc(html_root_url = "https://docs.rs/nvapi/0.3.0")]

pub use nvapi_sys as sys;

/// High-level aggregate API (the former `nvapi-hi` crate): the `Gpu` wrapper,
/// `GpuInfo`/`GpuStatus`/`GpuSettings` aggregates, and the
/// `allowable_result*` error-tolerating helpers.
pub mod hi;

#[macro_use]
mod macros;
mod clock;
mod ecc;
mod error;
mod gpu;
mod gsync;
#[cfg(feature = "i2c")]
mod i2c_impl;
mod info;
mod power;
mod pstate;
mod thermal;
mod types;

pub use clock::*;
pub use ecc::*;
pub use error::*;
pub use gpu::*;
pub use gsync::*;
#[cfg(feature = "i2c")]
pub use i2c_impl::*;
pub use info::*;
pub use power::*;
pub use pstate::*;
pub use thermal::*;
pub use types::*;

pub use sys::Status;
/// The result of a fallible NVAPI call.
pub type Result<T> = std::result::Result<T, Error>;
/// The result of a fallible NVAPI call.
pub type NvapiResult<T> = std::result::Result<T, NvapiError>;
