//! Functions to fetch manifest version from `api.matsurihi.me`.

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::error::{Repr, Result};
use crate::net::Error;

/// matshrihi.me MLTD v2 API `/version/assets/:app` response body structure.
#[derive(Debug, Clone, Deserialize)]
pub struct AppVersion {
    /// App version.
    pub version: String,

    /// Forced update date and time.
    #[serde(rename = "updatedAt")]
    pub updated_at: String,

    /// Revision number
    ///
    /// The value will be [`None`] if the version is not actually released.
    pub revision: Option<u64>,
}

/// matshrihi.me MLTD v2 API `/version/assets/:version` response body structure.
#[derive(Debug, Clone, Deserialize)]
pub struct AssetVersion {
    /// Manifest filename on MLTD asset server.
    #[serde(rename = "indexName")]
    pub manifest_filename: String,

    /// Delivery date and time.
    #[serde(rename = "updatedAt")]
    pub updated_at: String,

    /// Asset version.
    pub version: u64,
}

/// matshrihi.me MLTD v2 API `/version/latest` response body structure.
#[derive(Debug, Deserialize)]
pub struct VersionInfo {
    /// App version information
    #[serde(rename = "app")]
    pub app_version: AppVersion,

    /// Asset version information
    #[serde(rename = "asset")]
    pub asset_version: AssetVersion,
}

macro_rules! matsuri_api_endpoint {
    () => {
        "https://api.matsurihi.me/api/mltd/v2"
    };
}

pub const MATSURI_API_ENDPOINT: & str = matsuri_api_endpoint!();

async fn send_request<T: DeserializeOwned>(url: &str) -> Result<T> {
    let client = reqwest::Client::new();

    let url = match reqwest::Url::parse(url) {
        Ok(u) => Ok(u),
        Err(_) => Err(Repr::bug(&format!("invalid url: {url:?}"))),
    }?;

    let req = client.get(url.clone()).query(&[("prettyPrint", "false")]);

    let res = match req.send().await {
        Ok(r) => Ok(r),
        Err(e) => Err(Error::request(url.clone(), Some(e))),
    }?;

    let value: T = match res.json().await {
        Ok(v) => Ok(v),
        Err(e) => Err(Error::decode(url, Some(e))),
    }?;

    Ok(value)
}

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
/// use mltd::net::latest_asset_version;
///
/// tokio_test::block_on(async {
///     let asset_version = latest_asset_version().await.unwrap().version;
/// });
/// ```
pub async fn latest_asset_version() -> Result<AssetVersion> {
    let url = concat!(matsuri_api_endpoint!(), "/version/latest");
    let version_info: VersionInfo = send_request(url).await?;

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
pub async fn get_all_asset_versions() -> Result<Vec<AssetVersion>> {
    let url = concat!(matsuri_api_endpoint!(), "/version/assets");
    let versions: Vec<AssetVersion> = send_request(url).await?;

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
/// use mltd::net::get_asset_version;
///
/// tokio_test::block_on(async {
///     let asset_version = get_asset_version(1).await.unwrap();
///     assert_eq!(asset_version.version, 1);
/// });
/// ```
pub async fn get_asset_version(version: u64) -> Result<AssetVersion> {
    let url = format!("{MATSURI_API_ENDPOINT}/version/assets/{version}");
    let version: AssetVersion = send_request(&url).await?;

    Ok(version)
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
