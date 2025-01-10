//! Manifest file handling.

use std::collections::HashMap;
use std::ops::Deref;

use linked_hash_map::LinkedHashMap;
use serde::{Deserialize, Serialize};

use crate::asset::Asset;
use crate::error::Error;

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
#[repr(transparent)]
pub struct Manifest {
    #[serde(flatten)]
    pub data: [LinkedHashMap<String, ManifestEntry>; 1],
}

impl Manifest {
    /// Deserializes the specified bytes into a raw manifest.
    ///
    /// The bytes must be in message pack format.
    ///
    /// # Arguments
    ///
    /// * `value` - The message pack bytes to deserialize.
    ///
    /// # Errors
    ///
    /// This function will return [`Error::ManifestDeserialize`] if
    /// it cannot deserialize the message pack bytes.
    #[inline]
    pub fn from_slice(value: &[u8]) -> Result<Self, Error> {
        Ok(rmp_serde::from_slice(value)?)
    }

    /// Computes the difference between two manifests.
    ///
    /// # Arguments
    ///
    /// * `other` - The other manifest.
    ///
    /// # Returns
    ///
    /// The added, updated and removed entries in the new manifest.
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
    pub fn len(&self) -> usize {
        self.data[0].len()
    }

    /// Returns `true` if the manifest is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the total size of all assets in the manifest.
    #[inline]
    pub fn asset_size(&self) -> usize {
        self.data[0].values().fold(0, |acc, v| acc + v.2)
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

#[derive(Debug, Serialize)]
pub struct ManifestDiff<'a> {
    pub added: HashMap<&'a String, &'a ManifestEntry>,
    pub updated: HashMap<&'a String, &'a ManifestEntry>,
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
    use super::Manifest;

    #[test]
    fn test_raw_manifest_from_slice() {
        let buf = include_bytes!("../../tests/test1.msgpack");
        let manifest = Manifest::from_slice(buf).unwrap();
        assert_eq!(rmp_serde::to_vec(&manifest).unwrap(), buf);
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
