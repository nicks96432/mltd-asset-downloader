//! Error type definitions.

use std::io;

/// Error type for this crate.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("manifest deserialization failed: {0}")]
    ManifestDeserialize(#[from] rmp_serde::decode::Error),

    #[error("manifest serialization failed: {0}")]
    ManifestSerialize(#[from] rmp_serde::encode::Error),

    #[error("response deserialization failed: {0}")]
    ResponseDeserialize(reqwest::Error),

    #[error("cannot create output file: {0}")]
    FileCreate(io::Error),

    #[error("cannot write output file: {0}")]
    FileWrite(io::Error),

    #[error("fail to send request: {0}")]
    Request(reqwest::Error),

    #[error("unknown os variant: {0}")]
    UnknownVariant(String),
}
