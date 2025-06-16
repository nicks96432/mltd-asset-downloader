//! Error type definitions.

use std::backtrace::Backtrace;
use std::io::Error as IoError;
use std::panic::Location;

use thiserror::Error as ThisError;

use crate::extract::audio::AudioDecodeError;
use crate::extract::puzzle::PuzzleError;
use crate::extract::text::AesError;
use crate::manifest::ManifestError;
use crate::net::Error as NetworkError;

#[derive(Debug, ThisError)]
#[error("{repr}")]
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
    Io,
    UnknownPlatform,
    Aes,
    AudioDecode,
    Puzzle,
    Manifest,
    Network,
    OutOfRange,
    Bug,
}

impl From<&Repr> for ErrorKind {
    fn from(value: &Repr) -> Self {
        match value {
            Repr::Io { .. } => ErrorKind::Io,
            Repr::UnknownPlatform(_) => ErrorKind::UnknownPlatform,
            Repr::Aes(_) => ErrorKind::Aes,
            Repr::AudioDecode(_) => ErrorKind::AudioDecode,
            Repr::Puzzle(_) => ErrorKind::Puzzle,
            Repr::Manifest(_) => ErrorKind::Manifest,
            Repr::Network(_) => ErrorKind::Network,
            Repr::OutOfRange { .. } => ErrorKind::OutOfRange,
            Repr::Bug { .. } => ErrorKind::Bug,
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

    /// AES related error for text assets.
    #[error("{0}")]
    Aes(#[from] AesError),

    #[error("failed to decode audio: {0}")]
    AudioDecode(#[from] AudioDecodeError),

    /// Puzzle solving failed.
    #[error("failed to solve puzzle: {0}")]
    Puzzle(#[from] PuzzleError),

    /// manifest related error.
    #[error("{0}")]
    Manifest(#[from] ManifestError),

    /// network related error.
    #[error("{0}")]
    Network(#[from] NetworkError),

    /// Array index out of range error.
    #[error("try to access index {0} but the length is {1}")]
    OutOfRange(usize, usize),

    /// Bug occurred.
    #[error("bug: {msg}, at {location}\nsee backtraces above for more details")]
    Bug { msg: String, location: &'static Location<'static> },
}

impl Repr {
    #[track_caller]
    pub fn bug(msg: &str) -> Self {
        let backtrace = Backtrace::force_capture();
        println!("{backtrace}");

        Self::Bug { msg: msg.to_string(), location: Location::caller() }
    }

    pub fn io(reason: &str, source: Option<IoError>) -> Self {
        Self::Io { reason: reason.to_string(), source }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync<T>()
    where
        T: Send + Sync,
    {
    }

    #[test]
    fn test_error() {
        assert_send_sync::<Error>();
    }
}
