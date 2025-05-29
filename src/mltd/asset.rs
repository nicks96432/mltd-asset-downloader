//! MLTD asset handling.

use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::io::{self, Cursor};
use std::path::Path;
use std::str::FromStr;

use clap::ValueEnum;
use futures::TryStreamExt;
use indicatif::ProgressBar;
use reqwest::header::HeaderMap;
use tokio::fs::File;
use tokio::io::BufWriter;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use crate::Error;
use crate::net::AssetVersion;
use crate::util::ProgressReadAdapter;

/// Base URL of MLTD asset server.
pub const ASSET_URL_BASE: &str = "https://td-assets.bn765.com";

/// Unity version of MLTD game client.
pub const UNITY_VERSION: &str = "2020.3.32f1";

/// Information of an MLTD asset.
#[derive(Debug, Clone)]
pub struct AssetInfo {
    /// Asset filename
    pub filename: String,

    /// Platform variant
    pub platform: Platform,

    /// Asset version
    pub version: AssetVersion,
}

impl AssetInfo {
    fn to_url(&self) -> reqwest::Url {
        reqwest::Url::parse(&format!(
            "{}/{}/production/2018/{}/{}",
            ASSET_URL_BASE, self.version.version, self.platform, self.filename
        ))
        .unwrap()
    }
}

/// An asset from MLTD asset server.
pub struct Asset<'data> {
    /// Asset data
    pub data: Cow<'data, [u8]>,

    /// Asset info
    pub info: AssetInfo,
}

impl Asset<'_> {
    /// Send download request to MLTD asset server and returns the response.
    ///
    /// This will send GET request to MLTD asset server according to `asset_info`,
    /// with the headers set in the game client.
    ///
    /// # Errors
    ///
    /// - [`Error::Request`]: if it cannot send request to MLTD asset server.
    async fn send_request(asset_info: &AssetInfo) -> Result<reqwest::Response, Error> {
        let client = reqwest::Client::new();

        let mut headers = HeaderMap::new();
        headers.insert("X-Unity-Version", UNITY_VERSION.parse().unwrap());
        headers.insert("User-Agent", asset_info.platform.user_agent().parse().unwrap());

        let req = client.get(asset_info.to_url()).headers(headers);

        req.send().await.map_err(Error::Request)
    }

    /// Download the specified asset from MLTD asset server.
    ///
    /// This will send GET request to MLTD asset server according to `asset_info`.
    /// An optional `progress_bar` can be specified to track the download progress.
    ///
    /// # Errors
    ///
    /// - [`Error::IO`]: if it cannot write the downloaded data to file.
    /// - [`Error::Request`]: if it cannot send request to MLTD asset server.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use mltd::asset::{Asset, AssetInfo, Platform};
    /// use mltd::net;
    ///
    /// tokio_test::block_on(async {
    ///     let asset_version = net::latest_asset_version().await.unwrap();
    ///     let asset_info = AssetInfo {
    ///         filename: asset_version.manifest_filename.clone(),
    ///         platform: Platform::Android,
    ///         version: asset_version,
    ///     };
    ///
    ///     let asset = Asset::download(asset_info, None).await.unwrap();
    ///     println!("asset size: {}", asset.data.len());
    /// });
    /// ```
    pub async fn download(
        asset_info: AssetInfo,
        progress_bar: Option<&mut ProgressBar>,
    ) -> Result<Self, Error> {
        let res = Self::send_request(&asset_info).await?;

        if let Some(ref pb) = progress_bar {
            pb.set_length(res.content_length().unwrap_or(0));
        }

        log::debug!("download {} to buf", asset_info.filename);

        let stream_reader =
            res.bytes_stream().map_err(|e| io::Error::other(e)).into_async_read().compat();

        let mut stream_reader = ProgressReadAdapter::new(stream_reader, progress_bar);

        let mut buf = Cursor::new(Vec::new());
        tokio::io::copy(&mut stream_reader, &mut buf).await?;

        Ok(Self { data: Cow::from(buf.into_inner()), info: asset_info })
    }

    /// Download the specified asset from MLTD asset server and write it to disk.
    ///
    /// If `output` is not specified, it will be default to `asset_info.filename`.
    /// An optional `progress_bar` can be specified to track the download progress.
    ///
    /// # Errors
    ///
    /// - [`Error::IO`]: if it cannot write the downloaded data to file.
    /// - [`Error::Request`]: if it cannot send request to MLTD asset server.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::path::Path;
    ///
    /// use mltd::asset::{Asset, AssetInfo, Platform};
    /// use mltd::net;
    ///
    /// tokio_test::block_on(async {
    ///     let asset_version = net::latest_asset_version().await.unwrap();
    ///     let asset_info = AssetInfo {
    ///         filename: asset_version.manifest_filename.clone(),
    ///         platform: Platform::Android,
    ///         version: asset_version,
    ///     };
    ///
    ///     Asset::download_to_file(&asset_info, Some(Path::new("asset.unity3d")), None).await.unwrap();
    /// });
    /// ```
    pub async fn download_to_file(
        asset_info: &AssetInfo,
        output: Option<&Path>,
        progress_bar: Option<&mut ProgressBar>,
    ) -> Result<(), Error> {
        let output = output.unwrap_or(asset_info.filename.as_ref());
        let mut out = BufWriter::new(File::create(output).await?);

        let res = Self::send_request(asset_info).await?;

        if let Some(ref pb) = progress_bar {
            pb.set_length(res.content_length().unwrap_or(0));
        }

        log::debug!("save asset to {}", output.display());

        let stream_reader =
            res.bytes_stream().map_err(|e| io::Error::other(e)).into_async_read().compat();

        let mut stream_reader = ProgressReadAdapter::new(stream_reader, progress_bar);

        tokio::io::copy(&mut stream_reader, &mut out).await?;

        Ok(())
    }
}

/// Platform variant of the manifest.
///
/// Although the game and the manifest name looks the same on both platforms,
/// their manifests are different.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Platform {
    /// Android platform
    Android,

    /// iOS platform
    IOS,
}

impl Platform {
    /// Returns the string representation of the [`Platform`].
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Android => "Android",
            Self::IOS => "iOS",
        }
    }

    /// Returns the `User-Agent` string of the [`Platform`] in HTTP request.
    #[must_use]
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
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "android" => Ok(Self::Android),
            "ios" => Ok(Self::IOS),
            s => Err(Error::UnknownPlatform(s.to_string())),
        }
    }
}
