//! Error type definitions.

use std::io;

use tokio::task::JoinError;

/// Error type for this crate.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
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

    /// IO operation failed.
    #[error("IO operation failed: {0}")]
    IO(#[from] io::Error),

    /// Glob error.
    #[error("glob error: {0}")]
    Glob(#[from] glob::PatternError),

    /// Reqwest request failed.
    #[error("failed to send request: {0}")]
    Request(reqwest::Error),

    /// Unknown platform.
    #[error("unknown platform: {0}")]
    UnknownPlatform(String),

    /// Thread join failed.
    #[error("failed to join thread: {0}")]
    ThreadJoin(#[from] JoinError),

    /// Failed to parse integer from string.
    #[error("failed to parse int: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    /// AES related error.
    #[error("AES error: {0}")]
    Aes(String),

    /// zip related error.
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// image crate related error.
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),

    /// Puzzle solving failed.
    #[error("failed to solve puzzle: {0}")]
    Puzzle(String),

    /// Generic error.
    #[error("{0}")]
    Generic(String),
}
