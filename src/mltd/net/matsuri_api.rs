//! Functions to fetch manifest version from `api.matsurihi.me`.

use reqwest::{Client, ClientBuilder, Url};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::error::Result;
use crate::net::Error;

/// matshrihi.me MLTD v2 API `/version/assets/:app` response body structure.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
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

#[derive(Debug, Clone)]
pub struct MatsuriApiClient {
    client: Client,
}

impl MatsuriApiClient {
    /// Creates a new [`MatsuriApiClient`].
    ///
    /// # Panics
    ///
    /// This function will panic if the client cannot be built.
    #[must_use]
    pub fn new() -> Self {
        let client = ClientBuilder::new().zstd(true).https_only(true).build().unwrap();
        Self { client }
    }

    async fn send_request<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let url = Url::parse(url).unwrap();

        let res = match self.client.get(url.clone()).query(&[("prettyPrint", "false")]).send().await
        {
            Ok(r) => Ok(r),
            Err(e) => Err(Error::request(url.clone(), Some(e))),
        }?;

        let value: T = match res.json().await {
            Ok(v) => Ok(v),
            Err(e) => Err(Error::decode(url, Some(e))),
        }?;

        Ok(value)
    }

    /// Gets all asset versions from `api.matsurihi.me`.
    ///
    /// # Errors
    ///
    /// This method fails if it cannot send request to `api.matsurihi.me`.
    pub async fn get_all_asset_versions(&self) -> Result<Vec<AssetVersion>> {
        let url = concat!(matsuri_api_endpoint!(), "/version/assets");
        let versions: Vec<AssetVersion> = self.send_request(url).await?;

        Ok(versions)
    }

    /// Gets the specified asset version from `api.matsurihi.me`.
    ///
    /// # Errors
    ///
    /// This method fails if it cannot send request to `api.matsurihi.me`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use mltd::net::MatsuriApiClient;
    ///
    /// tokio_test::block_on(async {
    ///     let asset_version = MatsuriApiClient::new().get_asset_version(1).await.unwrap();
    ///     assert_eq!(asset_version.version, 1);
    /// });
    /// ```
    #[inline]
    pub async fn get_asset_version(&self, version: u64) -> Result<AssetVersion> {
        self.send_request(&format!(concat!(matsuri_api_endpoint!(), "/version/assets/{}"), version))
            .await
    }

    /// Gets the latest manifest filename and version from matsurihi.me.
    ///
    /// # Errors
    ///
    /// This method fails if it cannot send request to `api.matsurihi.me`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use mltd::net::MatsuriApiClient;
    ///
    /// tokio_test::block_on(async {
    ///     let asset_version = MatsuriApiClient::new().latest_asset_version().await.unwrap().version;
    /// });
    /// ```
    pub async fn latest_asset_version(&self) -> Result<AssetVersion> {
        let version_info: VersionInfo =
            self.send_request(concat!(matsuri_api_endpoint!(), "/version/latest")).await?;

        Ok(version_info.asset_version)
    }
}

impl Default for MatsuriApiClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use predicates::prelude::*;

    use crate::util::init_test_logger;

    use super::*;

    #[tokio::test]
    async fn test_get_asset_version() {
        init_test_logger();

        let version = MatsuriApiClient::new().get_asset_version(1).await.unwrap();
        assert_eq!(
            version,
            AssetVersion {
                version: 1,
                updated_at: "2017-06-29T12:00:00+09:00".to_string(),
                manifest_filename: "6b976a4c875a1984592a66b621872ce44c944e72.data".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn test_latest_asset_version() {
        init_test_logger();

        let version = MatsuriApiClient::new().latest_asset_version().await.unwrap();

        assert!(version.version > 0);
        assert!(!version.updated_at.is_empty());
        assert!(
            predicates::str::is_match(r"^[0-9a-f]{40}.data$")
                .unwrap()
                .eval(&version.manifest_filename)
        );
    }

    #[tokio::test]
    async fn test_all_asset_versions() {
        init_test_logger();

        let versions = MatsuriApiClient::new().get_all_asset_versions().await.unwrap();

        assert_ne!(versions.len(), 0);
    }
}
