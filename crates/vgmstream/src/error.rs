use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("vgmstream initialization failed")]
    InitializationFailed,

    #[error("invalid channel mapping: {0}")]
    InvalidChannelMapping(u32),

    #[error("vgmstream generic error")]
    Generic,
}
