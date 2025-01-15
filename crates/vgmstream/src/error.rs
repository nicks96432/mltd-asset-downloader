use std::ffi::c_int;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("vgmstream initialization failed")]
    InitializationFailed,

    #[error("invalid sample type: {0}")]
    InvalidSampleType(c_int),

    #[error("vgmstream generic error")]
    Generic,
}
