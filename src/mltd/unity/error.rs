//! Errors of reading Unity asset bundles.

// most variants are only constructed by the rabex backend
#![cfg_attr(not(feature = "rabex"), allow(dead_code))]

use thiserror::Error as ThisError;

use crate::error::{Error, Repr};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// A Unity asset bundle is invalid, or an asset in it cannot be read.
#[derive(Debug, ThisError)]
pub(crate) enum UnityError {
    /// The file does not start with the `UnityFS` signature.
    #[error("not a UnityFS bundle")]
    NotUnityFs,

    /// The path has no file name.
    #[error("path has no file name")]
    NoFileName,

    /// The path is a directory.
    #[cfg(feature = "asset-ripper")]
    #[error("path is a directory")]
    IsDirectory,

    /// A bundle with the same name is loaded already.
    #[error("a bundle named `{0}` is already loaded")]
    DuplicateBundle(String),

    /// The bundle of an asset is not loaded.
    #[error("bundle `{0}` is not loaded")]
    BundleNotLoaded(String),

    /// The collection of an asset is not loaded.
    #[error("collection `{0}` is not loaded")]
    CollectionNotLoaded(String),

    /// The asset does not exist.
    #[error("asset {0} does not exist")]
    AssetNotFound(i64),

    /// The asset has no original path.
    #[cfg(feature = "asset-ripper")]
    #[error("asset has no original path")]
    NoOriginalPath,

    /// The class of the asset cannot be extracted.
    #[error("cannot extract asset of class {0}")]
    UnsupportedClass(String),

    /// The bundle ends unexpectedly.
    #[error("{0} is truncated")]
    Truncated(&'static str),

    /// The streaming resource of an asset cannot be found.
    #[error("streaming resource `{0}` does not exist")]
    ResourceNotFound(String),

    /// The file entry in the bundle is not a resource file.
    #[error("`{0}` is a serialized file, not a resource")]
    NotAResource(String),

    /// The streaming data exceeds the resource file.
    #[error("streaming data is out of range")]
    ResourceOutOfRange,

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

impl UnityError {
    /// Creates an error for the content that cannot be parsed or decoded while doing `what`.
    pub fn corrupted(what: &'static str, source: impl Into<BoxError>) -> Self {
        Self::Corrupted { what, source: source.into() }
    }
}

impl From<UnityError> for Error {
    fn from(value: UnityError) -> Self {
        Repr::from(value).into()
    }
}
