//! Manifest file handling.

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Cursor, Write};
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;

use clap::ValueEnum;
use human_bytes::human_bytes;
use linked_hash_map::LinkedHashMap;
use mltd_utils::{fetch_asset, trace_response};
use serde::{Deserialize, Serialize};
use ureq::AgentBuilder;

use super::error::ManifestError;
use super::matsuri_api::{get_asset_version, latest_asset_version, AssetVersion};

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
pub struct RawManifest([LinkedHashMap<String, ManifestEntry>; 1]);

impl RawManifest {
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
    /// This function will return [`ManifestError::ManifestDeserialize`] if
    /// it cannot deserialize the message pack bytes.
    #[inline]
    pub fn from_slice(value: &[u8]) -> Result<Self, ManifestError> {
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
    /// The updated and removed entries in the new manifest.
    pub fn diff<'a>(&'a self, other: &'a RawManifest) -> ManifestDiff<'a> {
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
}

impl Deref for RawManifest {
    type Target = LinkedHashMap<String, ManifestEntry>;

    fn deref(&self) -> &Self::Target {
        &self.0[0]
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

/// A manifest file.
#[derive(Debug, Clone, Serialize)]
#[serde(into = "RawManifest")]
pub struct Manifest {
    /// The underlying raw manifest data.
    pub data: RawManifest,

    /// The asset version of the manifest, including the version and filename.
    #[serde(skip)]
    pub asset_version: AssetVersion,

    /// The platform variant of the manifest.
    #[serde(skip)]
    pub platform: Platform,
}

impl Manifest {
    /// Returns the number of entries in the manifest.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.0[0].len()
    }

    /// Returns `true` if the manifest is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Downloads the specified manifest from MLTD asset server.
    ///
    /// # Arguments
    ///
    /// * `variant` - OS variant
    /// * `version` - manifest version, if not specified, the latest version will be downloaded
    ///
    /// # Errors
    ///
    /// This function will return [`ManifestError::Request`] if
    /// it cannot send request to MLTD asset server.
    ///
    /// This function will return [`ManifestError::ManifestDeserialize`] if
    /// it cannot deserialize response.
    ///
    /// # Examples
    ///
    /// Download the manifest version 1 for Android:
    ///
    /// ```no_run
    /// use mltd_asset_manifest::{Manifest, Platform};
    ///
    /// let manifest = Manifest::from_version(&Platform::Android, Some(1)).unwrap();
    /// assert_eq!(manifest.platform, Platform::Android);
    /// assert_eq!(manifest.asset_version.version, 1);
    /// ```
    pub fn from_version(platform: &Platform, version: Option<u64>) -> Result<Self, ManifestError> {
        log::debug!("getting latest version from matsurihi.me");
        let asset_version = match version {
            None => latest_asset_version(),
            Some(v) => get_asset_version(v),
        }?;

        let agent_builder = AgentBuilder::new().https_only(true).user_agent(platform.user_agent());
        let agent = agent_builder.build();

        let asset_url_base = format!("/{}/production/2018/{}", asset_version.version, platform);

        log::debug!("reading manifest from MLTD asset server");
        let manifest_url = format!("{}/{}", asset_url_base, asset_version.filename);
        let manifest_res = match fetch_asset(&agent, &manifest_url) {
            Ok(r) => r,
            Err(e) => return Err(ManifestError::Request(e)),
        };
        trace_response(&manifest_res);

        let mut buf = Vec::new();
        if let Err(e) = manifest_res.into_reader().read_to_end(&mut buf) {
            log::warn!("cannot read response body: {}", e);
            log::warn!("manifest may not be complete!");
        }

        log::debug!("reading response body to buf");
        let mut reader = Cursor::new(&buf);

        let manifest = Manifest {
            data: rmp_serde::from_read::<_, RawManifest>(&mut reader)?,
            asset_version,
            platform: *platform,
        };

        log::info!(
            "downloaded manifest version {} (updated at {}), manifest file {}, total asset size {}",
            manifest.asset_version.version,
            manifest.asset_version.updated_at,
            manifest.asset_version.filename,
            human_bytes(manifest.asset_size() as f64)
        );

        Ok(manifest)
    }

    /// Returns the total size of all assets in the manifest.
    #[inline]
    pub fn asset_size(&self) -> usize {
        self.data.0[0].values().fold(0, |acc, v| acc + v.2)
    }

    /// Save the manifest to the specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to save the manifest file.
    ///
    /// # Errors
    ///
    /// This function will return [`ManifestError::FileCreate`] if
    /// it cannot create the file.
    ///
    /// This function will return [`ManifestError::FileWrite`] if
    /// it cannot write to the file.
    pub fn save(&self, path: &PathBuf) -> Result<(), ManifestError> {
        let mut file = match File::create(path) {
            Ok(f) => f,
            Err(e) => return Err(ManifestError::FileCreate(e)),
        };

        match file.write_all(&rmp_serde::to_vec(&self.data)?) {
            Ok(()) => Ok(()),
            Err(e) => Err(ManifestError::FileWrite(e)),
        }
    }
}

impl Deref for Manifest {
    type Target = RawManifest;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl From<Manifest> for RawManifest {
    fn from(value: Manifest) -> Self {
        value.data
    }
}

/// Platform variant of the manifest.
///
/// Although the game and the manifest name looks the same on both platforms,
/// their manifests are different.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Platform {
    Android,
    IOS,
}

impl Platform {
    /// Returns the string representation of the [`Platform`].
    pub fn as_str(&self) -> &str {
        match self {
            Self::Android => "Android",
            Self::IOS => "iOS",
        }
    }

    /// Returns the `User-Agent` string of the [`Platform`] in HTTP request.
    pub fn user_agent(&self) -> &str {
        match self {
            Self::Android => "UnityPlayer/2020.3.32f1 (UnityWebRequest/1.0, libcurl/7.80.0-DEV)",
            Self::IOS => "ProductName/5.2.000 CFNetwork/1333.0.4 Darwin/21.5.0",
        }
    }
}

impl Display for Platform {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Platform {
    type Err = ManifestError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "android" => Ok(Self::Android),
            "ios" => Ok(Self::IOS),
            s => Err(ManifestError::UnknownVariant(s.to_string())),
        }
    }
}

#[cfg(test)]
#[ctor::ctor]
fn init() {
    mltd_utils::init_test_logger!();
}

#[cfg(test)]
mod tests {
    use super::{Manifest, Platform, RawManifest};

    #[test]
    fn test_raw_manifest_from_slice() {
        let buf = include_bytes!("../tests/test1.msgpack");
        let manifest = RawManifest::from_slice(buf).unwrap();
        assert_eq!(rmp_serde::to_vec(&manifest).unwrap(), buf);
    }

    #[test]
    fn test_raw_manifest_diff() {
        let manifest1 = RawManifest::from_slice(include_bytes!("../tests/test1.msgpack")).unwrap();
        let manifest2 = RawManifest::from_slice(include_bytes!("../tests/test2.msgpack")).unwrap();

        let diff = manifest1.diff(&manifest2);
        assert_eq!(diff.added.len(), 8);
        assert_eq!(diff.updated.len(), 6);
        assert_eq!(diff.removed.len(), 0);
    }

    #[test]
    fn test_manifest_from_version() {
        let manifest = Manifest::from_version(&Platform::Android, None).unwrap();

        assert_eq!(manifest.platform, Platform::Android);
        assert!(manifest.asset_version.version > 0);
        assert!(!manifest.asset_version.filename.is_empty());
        assert!(manifest.asset_size() > 0);

        assert_eq!(
            rmp_serde::to_vec(&manifest).unwrap(),
            rmp_serde::to_vec(&RawManifest::from(manifest)).unwrap()
        );
    }
}
