//! Manifest file handling.

use std::collections::HashMap;
use std::ops::Deref;

use linked_hash_map::LinkedHashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

use crate::asset::Asset;
use crate::error::{Error, Repr, Result};

#[derive(Debug, ThisError)]
#[error("manifest error: {kind}")]
pub(crate) struct ManifestError {
    pub kind: ManifestErrorKind,
    pub manifest: Vec<u8>,
}

#[derive(Debug, ThisError)]
pub(crate) enum ManifestErrorKind {
    /// Manifest deserialization failed.
    #[error("cannot deserialize manifest: {0}")]
    Deserialize(#[from] rmp_serde::decode::Error),

    /// Manifest serialization failed.
    #[error("cannot serialize manifest: {0}")]
    Serialize(#[from] rmp_serde::encode::Error),
}

impl ManifestError {
    pub fn new(kind: ManifestErrorKind, manifest: Vec<u8>) -> Self {
        Self { kind, manifest }
    }
}

impl From<ManifestError> for Error {
    fn from(value: ManifestError) -> Self {
        Repr::from(value).into()
    }
}

/// An entry in the manifest file.
///
/// It contains the SHA1 hash of the file, the file name on the server and the
/// file size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEntry(pub String, pub String, pub usize);

/// Deserialized raw manifest.
///
/// It contains a dictionary of the manifest entries, where the key is the actual
/// file name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Manifest {
    /// The underlying raw manifest data.
    data: [LinkedHashMap<String, ManifestEntry>; 1],
}

impl Manifest {
    /// Computes the difference from `other` manifest.
    #[must_use]
    pub fn diff<'a>(&'a self, other: &'a Manifest) -> ManifestDiff<'a> {
        let mut diff = ManifestDiff::new();

        for (key, value) in other.iter() {
            if !self.contains_key(key) {
                diff.added.insert(key, value);
            } else if self[key].0 != value.0 || self[key].2 != value.2 {
                // although the hash and file size are the same, the hashed
                // file name may be different across different versions
                // for some unknown reason (maybe they hashed full path?)
                diff.updated.insert(key, value);
            }
        }

        for (key, value) in self.iter() {
            if !other.contains_key(key) {
                diff.removed.insert(key, value);
            }
        }

        diff
    }

    /// Returns the number of entries in the manifest.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.data[0].len()
    }

    /// Returns `true` if the manifest is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the total size of all assets in the manifest.
    #[inline]
    #[must_use]
    pub fn asset_size(&self) -> usize {
        self.data[0].values().fold(0, |acc, v| acc + v.2)
    }

    pub fn from_slice(value: &[u8]) -> Result<Self> {
        rmp_serde::from_slice(value)
            .map_err(|e| ManifestError::new(e.into(), value.to_vec()).into())
    }
}

impl Deref for Manifest {
    type Target = LinkedHashMap<String, ManifestEntry>;

    fn deref(&self) -> &Self::Target {
        &self.data[0]
    }
}

impl TryFrom<Asset<'_>> for Manifest {
    type Error = Error;

    fn try_from(asset: Asset) -> Result<Self, Self::Error> {
        Self::from_slice(&asset.data)
    }
}

impl TryFrom<&[u8]> for Manifest {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::from_slice(value)
    }
}

impl TryFrom<Manifest> for Vec<u8> {
    type Error = Error;

    fn try_from(value: Manifest) -> Result<Self, Self::Error> {
        rmp_serde::to_vec(&value).map_err(|e| ManifestError::new(e.into(), Vec::new()).into())
    }
}

/// A diff of two manifests.
#[derive(Debug, Serialize)]
pub struct ManifestDiff<'a> {
    /// The added entries in the new manifest.
    pub added: HashMap<&'a String, &'a ManifestEntry>,

    /// The updated entries in the new manifest.
    pub updated: HashMap<&'a String, &'a ManifestEntry>,

    /// The removed entries in the new manifest.
    pub removed: HashMap<&'a String, &'a ManifestEntry>,
}

impl ManifestDiff<'_> {
    fn new() -> Self {
        Self { added: HashMap::new(), updated: HashMap::new(), removed: HashMap::new() }
    }
}

#[cfg(test)]
#[ctor::ctor]
fn init() {
    crate::util::init_test_logger!();
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;

    use super::Manifest;

    #[test]
    fn test_raw_manifest_from_slice() {
        let expected = include_bytes!("../../tests/test1.msgpack");
        let manifest: Manifest = Manifest::try_from(&expected[..]).unwrap();

        assert_eq!(*expected, *rmp_serde::to_vec(&[manifest.deref()]).unwrap());
    }

    #[test]
    fn test_raw_manifest_diff() {
        let manifest1 = Manifest::from_slice(include_bytes!("../../tests/test1.msgpack")).unwrap();
        let manifest2 = Manifest::from_slice(include_bytes!("../../tests/test2.msgpack")).unwrap();

        let diff = manifest1.diff(&manifest2);
        assert_eq!(diff.added.len(), 8);
        assert_eq!(diff.updated.len(), 6);
        assert_eq!(diff.removed.len(), 0);
    }
}
