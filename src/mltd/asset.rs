use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::io;
use std::path::Path;
use std::str::FromStr;

use clap::ValueEnum;
use futures::TryStreamExt;
use indicatif::ProgressBar;
use reqwest::header::HeaderMap;
use tokio::fs::File;
use tokio::io::BufWriter;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use crate::net::AssetVersion;
use crate::util::ProgressReadAdapter;
use crate::Error;

pub const ASSET_URL_BASE: &str = "https://td-assets.bn765.com";
pub const UNITY_VERSION: &str = "2020.3.32f1";

#[derive(Debug, Clone)]
pub struct AssetInfo {
    pub filename: String,
    pub platform: Platform,
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
    async fn send_request(asset_info: &AssetInfo) -> Result<reqwest::Response, Error> {
        let client = reqwest::Client::new();

        let mut headers = HeaderMap::new();
        headers.insert("X-Unity-Version", UNITY_VERSION.parse().unwrap());
        headers.insert("User-Agent", asset_info.platform.user_agent().parse().unwrap());

        let req = client.get(asset_info.to_url()).headers(headers);

        req.send().await.map_err(Error::Request)
    }

    pub async fn download(
        asset_info: AssetInfo,
        progress_bar: Option<&mut ProgressBar>,
    ) -> Result<Self, Error> {
        let res = Self::send_request(&asset_info).await?;

        if let Some(ref pb) = progress_bar {
            pb.set_length(res.content_length().unwrap_or(0));
        }

        log::debug!("reading response body to buf");

        let stream_reader = res
            .bytes_stream()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
            .into_async_read()
            .compat();

        let mut stream_reader = ProgressReadAdapter::new(stream_reader, progress_bar);

        let mut buf = BufWriter::new(Vec::new());
        tokio::io::copy(&mut stream_reader, &mut buf).await.map_err(Error::FileWrite)?;

        Ok(Self { data: Cow::Owned(buf.into_inner()), info: asset_info })
    }

    pub async fn download_to_file(
        asset_info: &AssetInfo,
        output: Option<&Path>,
        progress_bar: Option<&mut ProgressBar>,
    ) -> Result<(), Error> {
        let output = output.unwrap_or(asset_info.filename.as_ref());
        let mut out = BufWriter::new(File::create(output).await.map_err(Error::FileCreate)?);

        let res = Self::send_request(asset_info).await?;

        if let Some(ref pb) = progress_bar {
            pb.set_length(res.content_length().unwrap_or(0));
        }

        log::debug!("downloading response body to file");

        let stream_reader = res
            .bytes_stream()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
            .into_async_read()
            .compat();

        let mut stream_reader = ProgressReadAdapter::new(stream_reader, progress_bar);

        tokio::io::copy(&mut stream_reader, &mut out).await.map_err(Error::FileWrite)?;

        Ok(())
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
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "android" => Ok(Self::Android),
            "ios" => Ok(Self::IOS),
            s => Err(Error::UnknownVariant(s.to_string())),
        }
    }
}