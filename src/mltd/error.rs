//! Error type definitions.

use std::io;

/// Error type for this crate.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Manifest deserialization failed.
    #[error("manifest deserialization failed: {0}")]
    ManifestDeserialize(#[from] rmp_serde::decode::Error),

    /// Manifest serialization failed.
    #[error("manifest serialization failed: {0}")]
    ManifestSerialize(#[from] rmp_serde::encode::Error),

    /// Reqwest response serialization failed.
    #[error("response deserialization failed: {0}")]
    ResponseDeserialize(reqwest::Error),

    /// Output file creation failed.
    #[error("cannot create output file: {0}")]
    FileCreate(io::Error),

    /// Output file writing failed.
    #[error("cannot write output file: {0}")]
    FileWrite(io::Error),

    /// Reqwest request failed.
    #[error("failed to send request: {0}")]
    Request(reqwest::Error),

    /// Unknown platform.
    #[error("unknown platform: {0}")]
    UnknownPlatform(String),
}
