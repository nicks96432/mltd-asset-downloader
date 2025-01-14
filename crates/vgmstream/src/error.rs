use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("vgmstream initialization failed")]
    InitializationFailed,

    #[error("invalid sample type: {0}")]
    InvalidSampleType(u32),

    #[error("vgmstream generic error")]
    Generic,
}
