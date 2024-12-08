//! Functions to fetch manifest version from `api.matsurihi.me`.

use serde::Deserialize;

use crate::Error;

/// matshrihi.me MLTD v2 API `/version/assets/:app` response body structure.
#[derive(Debug, Clone, Deserialize)]
pub struct AppVersion {
    pub version: String,

    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    pub revision: u64,
}

/// matshrihi.me MLTD v2 API `/version/assets/:version` response body structure.
#[derive(Debug, Clone, Deserialize)]
pub struct AssetVersion {
    #[serde(rename = "indexName")]
    pub manifest_filename: String,

    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    pub version: u64,
}

/// matshrihi.me MLTD v2 API `/version/latest` response body structure.
#[derive(Debug, Deserialize)]
pub struct VersionInfo {
    #[serde(rename = "app")]
    pub app_version: AppVersion,

    #[serde(rename = "asset")]
    pub asset_version: AssetVersion,
}

const MATSURI_API_ENDPOINT: &str = "https://api.matsurihi.me/api/mltd/v2";

/// Gets the latest manifest filename and version from matsurihi.me.
///
/// # Returns
///
/// Returns a tuple of manifest filename and version.
///
/// # Errors
///
/// [`Error::Request`]: if it cannot send request to `api.matsurihi.me`.
///
/// [`Error::ResponseDeserialize`]: if it cannot deserialize response.
///
/// # Examples
///
/// ```no_run
/// use mltd_asset_manifest::latest_asset_version;
///
/// let asset_version = latest_asset_version().unwrap().version;
/// ```
pub async fn latest_asset_version() -> Result<AssetVersion, Error> {
    let client = reqwest::Client::new();
    let req = client
        .get(format!("{}{}", MATSURI_API_ENDPOINT, "/version/latest"))
        .query(&[("prettyPrint", "false")]);

    let res = match req.send().await {
        Ok(r) => r,
        Err(e) => return Err(Error::Request(e)),
    };

    let version_info: VersionInfo = match res.json().await {
        Ok(info) => info,
        Err(e) => return Err(Error::ResponseDeserialize(e)),
    };

    Ok(version_info.asset_version)
}

/// Gets all asset versions from `api.matsurihi.me`.
///
/// # Returns
///
/// Returns a vector of asset versions.
///
/// # Errors
///
/// [`Error::Request`]: if it cannot send request to `api.matsurihi.me`.
///
/// [`Error::ResponseDeserialize`]: if it cannot deserialize response.
pub async fn get_all_asset_versions() -> Result<Vec<AssetVersion>, Error> {
    let client = reqwest::Client::new();
    let req = client
        .get(format!("{}{}", MATSURI_API_ENDPOINT, "/version/assets"))
        .query(&[("prettyPrint", "false")]);

    let res = match req.send().await {
        Ok(r) => r,
        Err(e) => return Err(Error::Request(e)),
    };

    let versions: Vec<AssetVersion> = match res.json().await {
        Ok(v) => v,
        Err(e) => return Err(Error::ResponseDeserialize(e)),
    };

    Ok(versions)
}

/// Gets the specified asset version from `api.matsurihi.me`.
///
/// # Returns
///
/// Returns the specified asset version.
///
/// # Errors
///
/// [`Error::Request`]: if it cannot send request to `api.matsurihi.me`.
///
/// [`Error::ResponseDeserialize`]: if it cannot deserialize response.
///
/// # Examples
///
/// ```no_run
/// use mltd_asset_manifest::get_asset_version;
///
/// let asset_version = get_asset_version(1).unwrap();
/// assert_eq!(asset_version.version, 1);
/// ```
pub async fn get_asset_version(version: u64) -> Result<AssetVersion, Error> {
    let client = reqwest::Client::new();
    let req = client
        .get(format!("{}/version/assets/{}", MATSURI_API_ENDPOINT, version))
        .query(&[("prettyPrint", "false")]);

    let res = match req.send().await {
        Ok(r) => r,
        Err(e) => return Err(Error::Request(e)),
    };

    let asset_version: AssetVersion = match res.json().await {
        Ok(v) => v,
        Err(e) => return Err(Error::ResponseDeserialize(e)),
    };

    Ok(asset_version)
}

#[cfg(test)]
#[ctor::ctor]
fn init() {
    crate::util::init_test_logger!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_asset_version() {
        let version = get_asset_version(1).await.unwrap();

        assert_eq!(version.version, 1);
        assert_eq!(version.updated_at, "2017-06-29T12:00:00+09:00");
        assert_eq!(version.manifest_filename, "6b976a4c875a1984592a66b621872ce44c944e72.data");
    }

    #[tokio::test]
    async fn test_latest_asset_version() {
        let version = latest_asset_version().await.unwrap();

        assert!(version.version > 0);
        assert!(!version.manifest_filename.is_empty());
    }

    #[tokio::test]
    async fn test_all_asset_versions() {
        let versions = get_all_asset_versions().await.unwrap();

        assert_ne!(versions.len(), 0);
    }
}
