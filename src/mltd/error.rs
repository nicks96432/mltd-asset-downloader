//! Error type definitions.

use std::io::Error as IoError;
use std::path::{Path, PathBuf};

use thiserror::Error as ThisError;

#[cfg(feature = "decode-audio")]
use crate::extract::audio::AudioDecodeError;
#[cfg(feature = "process-image")]
use crate::extract::puzzle::Error as PuzzleError;
#[cfg(feature = "decrypt-text")]
use crate::extract::text::AesError;
use crate::manifest::ManifestError;
use crate::net::Error as NetworkError;
#[cfg(any(feature = "rabex", feature = "asset-ripper"))]
use crate::unity::Error as UnityError;

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

/// The category of an [`Error`].
///
/// New kinds may be added, and some of them only exist when the related feature is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Deserialization of an asset failed.
    Deserialize,

    /// A file system operation failed.
    Io,

    /// The manifest cannot be (de)serialized.
    Manifest,

    /// A network request failed.
    Network,

    /// The path read from an untrusted source escapes the directory it should stay in.
    UnsafePath,

    /// The platform name is unknown.
    UnknownPlatform,

    /// An external process failed to start or exited unexpectedly.
    #[cfg(feature = "asset-ripper")]
    Process,

    /// A Unity asset bundle is invalid, or an asset in it cannot be read.
    #[cfg(any(feature = "rabex", feature = "asset-ripper"))]
    Unity,

    /// Audio decoding or encoding failed.
    #[cfg(feature = "decode-audio")]
    AudioDecode,

    /// Text decryption failed.
    #[cfg(feature = "decrypt-text")]
    Aes,

    /// Puzzle solving failed.
    #[cfg(feature = "process-image")]
    Puzzle,

    /// An image cannot be encoded or decoded.
    #[cfg(feature = "process-image")]
    Image,

    /// An index is out of range.
    #[cfg(any(feature = "decode-audio", feature = "process-image"))]
    OutOfRange,
}

impl From<&Repr> for ErrorKind {
    fn from(value: &Repr) -> Self {
        match value {
            Repr::Io(_) => ErrorKind::Io,
            Repr::Manifest(_) => ErrorKind::Manifest,
            Repr::Network(_) => ErrorKind::Network,
            Repr::UnknownPlatform(_) => ErrorKind::UnknownPlatform,
            Repr::UnsafePath(_) => ErrorKind::UnsafePath,

            #[cfg(feature = "asset-ripper")]
            Repr::Process(_) => ErrorKind::Process,

            #[cfg(any(feature = "rabex", feature = "asset-ripper"))]
            Repr::Unity(_) => ErrorKind::Unity,

            #[cfg(any(feature = "extract", feature = "rabex", feature = "asset-ripper"))]
            Repr::JSONDeserialize(_) => ErrorKind::Deserialize,

            #[cfg(feature = "decode-audio")]
            Repr::AudioDecode(_) => ErrorKind::AudioDecode,

            #[cfg(feature = "decrypt-text")]
            Repr::Aes(_) => ErrorKind::Aes,

            #[cfg(feature = "process-image")]
            Repr::Puzzle(_) => ErrorKind::Puzzle,

            #[cfg(feature = "process-image")]
            Repr::Image(_) => ErrorKind::Image,

            #[cfg(any(feature = "decode-audio", feature = "process-image"))]
            Repr::OutOfRange { .. } => ErrorKind::OutOfRange,
        }
    }
}

/// A file system operation failed.
#[derive(Debug, ThisError)]
#[error("failed to {op}{}", PathDisplay(.path.as_deref()))]
pub(crate) struct FsError {
    /// What was being done, e.g. `"create directory"`.
    op: &'static str,

    /// The path the operation was applied to.
    path: Option<PathBuf>,

    #[source]
    source: IoError,
}

/// Displays ` <path>` if the path is known.
struct PathDisplay<'a>(Option<&'a Path>);

impl std::fmt::Display for PathDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(path) => write!(f, " `{}`", path.display()),
            None => Ok(()),
        }
    }
}

/// Error type for this crate.
#[derive(ThisError, Debug)]
pub(crate) enum Repr {
    /// A file system operation failed.
    #[error(transparent)]
    Io(#[from] FsError),

    /// A path escapes the directory it should stay in.
    #[error("unsafe path: {0}")]
    UnsafePath(String),

    /// Unknown platform.
    #[error("unknown platform: {0}")]
    UnknownPlatform(String),

    /// An external process failed to start or exited unexpectedly.
    #[cfg(feature = "asset-ripper")]
    #[error("{0}")]
    Process(String),

    /// An Unity asset bundle is invalid, or an asset in it cannot be read.
    #[cfg(any(feature = "rabex", feature = "asset-ripper"))]
    #[error(transparent)]
    Unity(#[from] UnityError),

    #[cfg(feature = "decode-audio")]
    #[error("failed to decode audio: {0}")]
    AudioDecode(#[from] AudioDecodeError),

    /// Puzzle solving failed.
    #[cfg(feature = "process-image")]
    #[error("failed to solve puzzle: {0}")]
    Puzzle(#[from] PuzzleError),

    /// An image cannot be encoded or decoded.
    #[cfg(feature = "process-image")]
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),

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
    #[cfg(any(feature = "extract", feature = "rabex", feature = "asset-ripper"))]
    #[error(transparent)]
    JSONDeserialize(#[from] serde_json::Error),

    /// Array index out of range error.
    #[cfg(any(feature = "decode-audio", feature = "process-image"))]
    #[error("try to access index {0} but the length is {1}")]
    OutOfRange(usize, usize),
}

impl Repr {
    /// Creates an error for the failed file system operation `op` on `path`.
    pub fn io(op: &'static str, path: Option<&Path>, source: IoError) -> Self {
        Self::Io(FsError { op, path: path.map(Path::to_path_buf), source })
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

    #[test]
    fn test_io_error_message() {
        let source = IoError::from(std::io::ErrorKind::NotFound);

        let err: Error = Repr::io("open file", Some(Path::new("a.txt")), source).into();
        assert_eq!(err.kind(), ErrorKind::Io);
        assert_eq!(err.to_string(), "failed to open file `a.txt`");

        let err: Error = Repr::io("create tempfile", None, IoError::other("x")).into();
        assert_eq!(err.to_string(), "failed to create tempfile");
        assert!(std::error::Error::source(&err).is_some());
    }
}
