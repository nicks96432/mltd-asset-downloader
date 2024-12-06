//! Error type definitions.

use std::io;

use thiserror::Error;

/// Error type for manifest operations.
#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("manifest deserialization failed: {0}")]
    ManifestDeserialize(#[from] rmp_serde::decode::Error),

    #[error("manifest serialization failed: {0}")]
    ManifestSerialize(#[from] rmp_serde::encode::Error),

    #[error("response deserialization failed: {0}")]
    ResponseDeserialize(io::Error),

    #[error("cannot create output file: {0}")]
    FileCreate(io::Error),

    #[error("cannot write output file: {0}")]
    FileWrite(io::Error),

    #[error("fail to send request: {0}")]
    Request(#[from] Box<ureq::Error>),

    #[error("unknown os variant: {0}")]
    UnknownVariant(String),
}
