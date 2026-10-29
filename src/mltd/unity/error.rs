//! Errors of reading Unity asset bundles.

// most variants are only constructed by the rabex backend
#![cfg_attr(not(feature = "rabex"), allow(dead_code))]

use thiserror::Error as ThisError;

use crate::error::{Error as CrateError, Repr};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// A Unity asset bundle is invalid, or an object in it cannot be read.
#[derive(Debug, ThisError)]
pub(crate) enum Error {
    /// The file does not start with the `UnityFS` signature.
    #[error("not a UnityFS bundle")]
    NotUnityFs,

    /// The path is a directory.
    #[cfg(feature = "asset-ripper")]
    #[error("path is a directory")]
    IsDirectory,

    /// The object does not exist in the bundle.
    #[error("object {0} does not exist")]
    ObjectNotFound(i64),

    /// The object has no original path.
    #[cfg(feature = "asset-ripper")]
    #[error("object has no original path")]
    NoOriginalPath,

    /// The class of the object cannot be read by the requested method.
    #[error("cannot read object of class {0} as {1}")]
    UnsupportedClass(String, &'static str),

    /// The bundle ends unexpectedly.
    #[error("{0} is truncated")]
    Truncated(&'static str),

    /// The streaming resource of an object cannot be found.
    #[error("streaming resource `{0}` does not exist")]
    ResourceNotFound(String),

    /// The file entry in the bundle is not a resource file.
    #[error("`{0}` is a serialized file, not a resource")]
    NotAResource(String),

    /// The streaming data exceeds the resource file.
    #[error("streaming data is out of range")]
    ResourceOutOfRange,

    /// The object does not have a field that it should have.
    #[cfg(feature = "asset-ripper")]
    #[error("object has no field `{0}`")]
    MissingField(&'static str),

    /// The web page of `AssetRipper` does not have the expected layout.
    #[cfg(feature = "asset-ripper")]
    #[error("unexpected AssetRipper page layout: cannot find {0}")]
    PageLayout(&'static str),

    /// Parsing or decoding the content failed.
    #[error("failed to {what}")]
    Corrupted {
        what: &'static str,

        #[source]
        source: BoxError,
    },
}

impl Error {
    /// Creates an error for the content that cannot be parsed or decoded while doing `what`.
    pub fn corrupted(what: &'static str, source: impl Into<BoxError>) -> Self {
        Self::Corrupted { what, source: source.into() }
    }
}

impl From<Error> for CrateError {
    fn from(value: Error) -> Self {
        Repr::from(value).into()
    }
}
