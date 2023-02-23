use super::error::ManifestError;
use super::os_variant::OsVariant;
use linked_hash_map::LinkedHashMap;
use mltd_utils::{fetch_asset, trace_request, trace_response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_tuple::Deserialize_tuple as DeserializeTuple;
use serde_tuple::Serialize_tuple as SerializeTuple;
use std::io::{copy, Cursor};
use ureq::AgentBuilder;

/// Type of an entry in the manifest file.
///
/// The order of the members matters because it's deserialized from an array.
#[derive(Debug, Clone, SerializeTuple, DeserializeTuple)]
pub struct ManifestEntry {
    /// SHA1 hash of the file.
    pub hash: String,

    /// File name on the server.
    pub filename: String,

    /// File size.
    pub size: u64,
}

type RawManifest = [LinkedHashMap<String, ManifestEntry>; 1];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "RawManifest", into = "RawManifest")]
pub struct Manifest {
    pub data: RawManifest,

    #[serde(skip)]
    pub name: String,

    #[serde(skip)]
    pub version: u64,
}

impl Manifest {
    pub fn new() -> Self {
        Self {
            data: [LinkedHashMap::new(); 1],
            name: String::new(),
            version: 0u64,
        }
    }

    pub fn len(&self) -> usize {
        self.data[0].len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Gets the latest manifest filename and version from matsurihi.me.
    ///
    /// # Errors
    ///
    /// This function will return [`ManifestError`] with
    /// [`ManifestErrorKind::RequestFailed`] if it cannot send request to
    /// matsurihi.me.
    ///
    /// This function will return [`ManifestError`] with
    /// [`ManifestErrorKind::DeserializeFailed`] if it cannot deserialize
    /// response.
    ///
    /// This function will return [`ManifestError`] with
    /// [`ManifestErrorKind::StructFieldNotFound`] if it cannot find manifest
    /// filename or version from the deserialized response.
    fn latest_version() -> Result<(String, u64), ManifestError> {
        let url = "https://api.matsurihi.me/api/mltd/v2/version/latest";
        let req = ureq::get(url).query("prettyPrint", "false");
        trace_request(&req);

        let res = match req.call() {
            Ok(r) => r,
            Err(_) => return Err(ManifestError::RequestFailed),
        };
        log::trace!("");
        trace_response(&res);

        let json = match res.into_json::<Value>() {
            Ok(v) => v,
            Err(_) => return Err(ManifestError::DeserializeFailed),
        };

        let filename = match json["asset"]["indexName"].as_str() {
            Some(s) => s.to_owned(),
            None => return Err(ManifestError::StructFieldNotFound),
        };
        let version = match json["asset"]["version"].to_string().parse::<u64>() {
            Ok(v) => v,
            Err(_) => return Err(ManifestError::StructFieldNotFound),
        };

        Ok((filename, version))
    }

    pub fn download(variant: &OsVariant) -> Result<Self, ManifestError> {
        log::debug!("getting version from matsurihi.me");
        let (manifest_name, manifest_version) = Self::latest_version()?;

        log::info!(
            "the latest version is {}, manifest file {}",
            manifest_version,
            manifest_name
        );

        log::debug!("building request agent");
        let agent_builder = AgentBuilder::new()
            .https_only(true)
            .user_agent(variant.user_agent());
        let agent = agent_builder.build();

        let asset_url_base = format!("/{}/production/2018/{}", manifest_version, variant);

        log::debug!("reading manifest from MLTD asset server");
        let manifest_url = format!("{}/{}", asset_url_base, manifest_name);
        let manifest_res = match fetch_asset(&agent, &manifest_url) {
            Ok(r) => r,
            Err(_) => return Err(ManifestError::RequestFailed),
        };
        trace_response(&manifest_res);

        log::debug!("reading response body to buf");

        let mut buf = Vec::new();
        if copy(&mut manifest_res.into_reader(), &mut buf).is_err() {
            return Err(ManifestError::RequestFailed);
        }
        let mut reader = Cursor::new(&buf);

        let manifest = Manifest {
            data: match rmp_serde::from_read::<_, RawManifest>(&mut reader) {
                Ok(m) => m,
                Err(_) => return Err(ManifestError::DeserializeFailed),
            },
            name: manifest_name,
            version: manifest_version,
        };

        Ok(manifest)
    }

    pub fn from_slice(value: &[u8]) -> Result<Self, ManifestError> {
        if let Ok(manifest) = rmp_serde::from_slice(value) {
            return Ok(manifest);
        }

        #[cfg(feature = "json")]
        if let Ok(manifest) = serde_json::from_slice(value) {
            return Ok(manifest);
        }

        #[cfg(feature = "yaml")]
        if let Ok(manifest) = serde_yaml::from_slice(value) {
            return Ok(manifest);
        }

        Err(ManifestError::DeserializeFailed)
    }

    pub fn msgpack(&self) -> Result<Vec<u8>, ManifestError> {
        match rmp_serde::to_vec(self) {
            Ok(v) => Ok(v),
            Err(_) => Err(ManifestError::SerializeFailed),
        }
    }

    #[cfg(feature = "json")]
    pub fn json(&self) -> Result<Vec<u8>, ManifestError> {
        match serde_json::to_vec(self) {
            Ok(v) => Ok(v),
            Err(_) => Err(ManifestError::SerializeFailed),
        }
    }

    #[cfg(feature = "yaml")]
    pub fn yaml(&self) -> Result<Vec<u8>, ManifestError> {
        let mut buf = Vec::new();
        match serde_yaml::to_writer(&mut buf, self) {
            Ok(_) => Ok(buf),
            Err(_) => Err(ManifestError::SerializeFailed),
        }
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self::new()
    }
}

impl From<RawManifest> for Manifest {
    fn from(value: RawManifest) -> Self {
        Self {
            data: value,
            name: String::new(),
            version: 0u64,
        }
    }
}

impl From<Manifest> for RawManifest {
    fn from(value: Manifest) -> Self {
        value.data
    }
}

#[cfg(test)]
#[ctor::ctor]
fn init() {
    mltd_utils::init_test_logger!();
}

#[cfg(test)]
mod tests {
    use super::Manifest;
    use std::fs::File;
    use std::io::Read;
    use std::path::Path;

    #[test]
    fn test_manifest_msgpack() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("test.msgpack");

        let mut file = File::open(path).unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();

        let manifest = Manifest::from_slice(buf.as_slice()).unwrap();
        assert_eq!(manifest.msgpack().unwrap(), buf);
    }
}
