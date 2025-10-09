//! Error type definitions.

use std::io::Error as IoError;

use thiserror::Error as ThisError;

use crate::manifest::ManifestError;
use crate::net::Error as NetworkError;

#[cfg(feature = "decode-audio")]
use crate::extract::audio::AudioDecodeError;
#[cfg(feature = "process-image")]
use crate::extract::puzzle::Error as PuzzleError;
#[cfg(feature = "decrypt-text")]
use crate::extract::text::AesError;

#[derive(Debug, ThisError)]
#[error(transparent)]
pub struct Error {
    repr: Box<Repr>,
}

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        self.repr.as_ref().into()
    }
}

impl From<Repr> for Error {
    fn from(repr: Repr) -> Self {
        Self { repr: Box::new(repr) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    Deserialize,
    Io,
    Manifest,
    Network,
    UnknownPlatform,

    #[cfg(feature = "decode-audio")]
    AudioDecode,

    #[cfg(feature = "decrypt-text")]
    Aes,

    #[cfg(feature = "process-image")]
    Puzzle,

    #[cfg(any(feature = "decode-audio", feature = "process-image"))]
    OutOfRange,
}

impl From<&Repr> for ErrorKind {
    fn from(value: &Repr) -> Self {
        match value {
            Repr::Io { .. } => ErrorKind::Io,
            Repr::Manifest(_) => ErrorKind::Manifest,
            Repr::Network(_) => ErrorKind::Network,
            Repr::UnknownPlatform(_) => ErrorKind::UnknownPlatform,
            Repr::JSONDeserialize(_) => ErrorKind::Deserialize,

            #[cfg(feature = "decode-audio")]
            Repr::AudioDecode(_) => ErrorKind::AudioDecode,

            #[cfg(feature = "decrypt-text")]
            Repr::Aes(_) => ErrorKind::Aes,

            #[cfg(feature = "process-image")]
            Repr::Puzzle(_) => ErrorKind::Puzzle,

            #[cfg(any(feature = "decode-audio", feature = "process-image"))]
            Repr::OutOfRange { .. } => ErrorKind::OutOfRange,
        }
    }
}

/// Error type for this crate.
#[derive(ThisError, Debug)]
pub(crate) enum Repr {
    /// IO operation failed.
    #[error("{reason}, cause: {source:?}")]
    Io { reason: String, source: Option<IoError> },

    /// Unknown platform.
    #[error("unknown platform: {0}")]
    UnknownPlatform(String),

    #[cfg(feature = "decode-audio")]
    #[error("failed to decode audio: {0}")]
    AudioDecode(#[from] AudioDecodeError),

    /// Puzzle solving failed.
    #[cfg(feature = "process-image")]
    #[error("failed to solve puzzle: {0}")]
    Puzzle(#[from] PuzzleError),

    /// AES related error for text assets.
    #[cfg(feature = "decrypt-text")]
    #[error(transparent)]
    Aes(#[from] AesError),

    /// manifest related error.
    #[error(transparent)]
    Manifest(#[from] ManifestError),

    /// network related error.
    #[error(transparent)]
    Network(#[from] NetworkError),

    /// JSON deserialization error.
    #[cfg(feature = "extract")]
    #[error(transparent)]
    JSONDeserialize(#[from] serde_json::Error),

    /// Array index out of range error.
    #[cfg(any(feature = "decode-audio", feature = "process-image"))]
    #[error("try to access index {0} but the length is {1}")]
    OutOfRange(usize, usize),
}

impl Repr {
    pub fn io(reason: &str, source: Option<IoError>) -> Self {
        Self::Io { reason: reason.to_string(), source }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn assert_send_sync<T>()
    where
        T: Send + Sync,
    {
    }

    #[test]
    const fn test_error() {
        assert_send_sync::<Error>();
    }
}
