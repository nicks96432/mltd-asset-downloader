//! Error type definitions.

use std::io;

use tokio::task::JoinError;

/// Error type for this crate.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Manifest deserialization failed.
    #[error("manifest deserialization failed: {0}")]
    ManifestDeserialize(#[from] rmp_serde::decode::Error),

    /// Manifest serialization failed.
    #[error("manifest serialization failed: {0}")]
    ManifestSerialize(#[from] rmp_serde::encode::Error),

    /// VGMStream error.
    #[error("vgmstream error: {0}")]
    VGMStream(#[from] vgmstream::Error),

    /// FFmpeg Error.
    #[error("ffmpeg error: {0}")]
    FFmpeg(#[from] ffmpeg_next::Error),

    /// Reqwest response serialization failed.
    #[error("response deserialization failed: {0}")]
    ResponseDeserialize(reqwest::Error),

    /// File creation failed.
    #[error("cannot create file: {0}")]
    FileCreate(io::Error),

    /// File reading failed.
    #[error("cannot read file: {0}")]
    FileRead(io::Error),

    /// File writing failed.
    #[error("cannot write file: {0}")]
    FileWrite(io::Error),

    /// Reqwest request failed.
    #[error("failed to send request: {0}")]
    Request(reqwest::Error),

    /// Unknown platform.
    #[error("unknown platform: {0}")]
    UnknownPlatform(String),

    /// Thread join failed.
    #[error("failed to join thread: {0}")]
    ThreadJoin(#[from] JoinError),

    /// Generic error.
    #[error("{0}")]
    Generic(String),
}
