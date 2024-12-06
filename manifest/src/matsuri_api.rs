//! Functions to fetch manifest version from `api.matsurihi.me`.

use mltd_utils::{trace_request, trace_response};
use serde::Deserialize;

use crate::ManifestError;

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
    pub filename: String,

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
/// [`ManifestError::Request`]: if it cannot send request to `api.matsurihi.me`.
///
/// [`ManifestError::ResponseDeserialize`]: if it cannot deserialize response.
///
/// # Examples
///
/// ```no_run
/// use mltd_asset_manifest::latest_asset_version;
///
/// let asset_version = latest_asset_version().unwrap().version;
/// ```
pub fn latest_asset_version() -> Result<AssetVersion, ManifestError> {
    let url = &format!("{}{}", MATSURI_API_ENDPOINT, "/version/latest");
    let req = ureq::get(url).query("prettyPrint", "false");
    trace_request(&req);

    let res = match req.call() {
        Ok(r) => r,
        Err(e) => return Err(ManifestError::Request(Box::new(e))),
    };
    log::trace!("");
    trace_response(&res);

    let version_info = match res.into_json::<VersionInfo>() {
        Ok(info) => info,
        Err(e) => return Err(ManifestError::ResponseDeserialize(e)),
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
/// [`ManifestError::Request`]: if it cannot send request to `api.matsurihi.me`.
///
/// [`ManifestError::ResponseDeserialize`]: if it cannot deserialize response.
pub fn get_all_asset_versions() -> Result<Vec<AssetVersion>, ManifestError> {
    let url = &format!("{}{}", MATSURI_API_ENDPOINT, "/version/assets");
    let req = ureq::get(url).query("prettyPrint", "false");
    trace_request(&req);

    let res = match req.call() {
        Ok(r) => r,
        Err(e) => return Err(ManifestError::Request(Box::new(e))),
    };
    log::trace!("");
    trace_response(&res);

    let versions = match res.into_json::<Vec<AssetVersion>>() {
        Ok(v) => v,
        Err(e) => return Err(ManifestError::ResponseDeserialize(e)),
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
/// [`ManifestError::Request`]: if it cannot send request to `api.matsurihi.me`.
///
/// [`ManifestError::ResponseDeserialize`]: if it cannot deserialize response.
/// 
/// # Examples
///
/// ```no_run
/// use mltd_asset_manifest::get_asset_version;
///
/// let asset_version = get_asset_version(1).unwrap().version;
/// assert_eq!(asset_version.version, 1);
/// ```
pub fn get_asset_version(version: u64) -> Result<AssetVersion, ManifestError> {
    let url = &format!("{}/version/assets/{}", MATSURI_API_ENDPOINT, version);
    let req = ureq::get(url).query("prettyPrint", "false");
    trace_request(&req);

    let res = match req.call() {
        Ok(r) => r,
        Err(e) => return Err(ManifestError::Request(Box::new(e))),
    };
    log::trace!("");
    trace_response(&res);

    let asset_version = match res.into_json::<AssetVersion>() {
        Ok(v) => v,
        Err(e) => return Err(ManifestError::ResponseDeserialize(e)),
    };

    Ok(asset_version)
}

#[cfg(test)]
#[ctor::ctor]
fn init() {
    mltd_utils::init_test_logger!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_asset_version() {
        let version = get_asset_version(1).unwrap();

        assert_eq!(version.version, 1);
        assert_eq!(version.updated_at, "2017-06-29T12:00:00+09:00");
        assert_eq!(version.filename, "6b976a4c875a1984592a66b621872ce44c944e72.data");
    }

    #[test]
    fn test_latest_asset_version() {
        let version = latest_asset_version().unwrap();

        assert_eq!(version.version > 0, true);
        assert_eq!(version.filename.is_empty(), false);
    }

    #[test]
    fn test_all_asset_versions() {
        let versions = get_all_asset_versions().unwrap();

        assert_ne!(versions.len(), 0);
    }
}
