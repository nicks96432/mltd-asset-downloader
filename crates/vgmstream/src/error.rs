use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0} failed")]
    VgmStream(String),

    #[error("unknown channel mapping: {0}")]
    UnknownChannelMapping(u32),

    #[error("unknown sample format: {0}")]
    UnknownSampleFormat(u32),

    #[error("{0} is null")]
    NullPointer(String),
}
