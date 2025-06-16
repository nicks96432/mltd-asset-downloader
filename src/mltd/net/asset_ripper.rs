//! AssetRipper client.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use futures::TryStreamExt;
use indicatif::ProgressBar;
use reqwest::Response;
use scraper::error::SelectorErrorKind;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use crate::error::{Repr, Result};
use crate::net::Error;
use crate::util::ProgressReadAdapter;

/// Asset entry on `/Collections/View`.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct AssetEntry(pub i64, pub String, pub String);

/// Information about an asset on `/Assets/View`.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct AssetInfo {
    /// Asset entry on `/Collections/View`.
    pub entry: AssetEntry,

    /// Original path of the asset before bundled.
    pub original_path: Option<String>,

    /// Name of the bundle file.
    pub asset_bundle_name: String,
}

/// AssetRipper client.
#[derive(Debug)]
pub struct AssetRipper {
    /// Base URL of AssetRipper.
    base_url: String,

    /// AssetRipper child process.
    process: Option<Child>,
}

fn selector_should_be_valid(_: SelectorErrorKind) -> Repr {
    Repr::bug("selector should be valid")
}

impl AssetRipper {
    fn full_url(&self, url: &str) -> Result<reqwest::Url> {
        let url = format!("{}/{}", &self.base_url, url);
        reqwest::Url::parse(&url).map_err(|_| Repr::bug("url should be valid").into())
    }

    /// Sends a GET request with the given path parameter to AssetRipper.
    async fn send_request(&mut self, url: &reqwest::Url, path: &str) -> Result<Response> {
        let client = reqwest::Client::new();

        let req = client.get(url.clone());
        let req = if path.is_empty() { req } else { req.query(&[("Path", path)]) };

        let res = match req.send().await {
            Ok(r) => Ok(r),
            Err(e) => Err(Error::request(url.clone(), Some(e))),
        }?;

        Ok(res)
    }

    async fn get_text(&mut self, url: &str, path: &str) -> Result<String> {
        let url = self.full_url(url)?;

        let res = self.send_request(&url, path).await?;

        let text = match res.text().await {
            Ok(t) => Ok(t),
            Err(e) => Err(Error::decode(url, Some(e))),
        }?;

        Ok(text)
    }

    /// Starts a new AssetRipper instance with the given executable path and port.
    pub fn new<P>(path: P, port: u16) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let process = match Command::new(path.as_ref())
            .args(["--port", &port.to_string()])
            .args(["--launch-browser", "false"])
            .args(["--log", "false"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(mut process) => {
                let reader = BufReader::new(
                    process.stdout.take().ok_or_else(|| Repr::bug("failed to get stdout"))?,
                );
                for line in reader.lines() {
                    let line = line.map_err(|e| {
                        Repr::io("failed to read line from AssetRipper output", Some(e))
                    })?;
                    if line.contains("listening on: http://") {
                        break;
                    }
                }

                Ok(process)
            }
            Err(err) => Err(Repr::io("failed to start AssetRipper", Some(err))),
        }?;

        Ok(Self { base_url: format!("http://localhost:{port}"), process: Some(process) })
    }

    /// Connects th an existing AssetRipper instance with the given host and port.
    #[must_use]
    pub fn connect(host: &str, port: u16) -> Self {
        Self { base_url: format!("http://{host}:{port}"), process: None }
    }

    /// Loads an asset or a folder into the AssetRipper.
    pub async fn load<P>(&mut self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        let url = if path.is_dir() { "LoadFolder" } else { "LoadFile" };
        let url = self.full_url(url)?;

        let mut form = HashMap::new();
        form.insert("path", path.to_string_lossy().to_string());

        let client = reqwest::Client::new();
        let req = client.post(url.clone()).form(&form);

        if let Err(e) = req.send().await {
            return Err(Error::request(url, Some(e)).into());
        }

        Ok(())
    }

    /// Returns a list of loaded bundles.
    pub async fn bundles(&mut self) -> Result<Vec<String>> {
        let path = r#"{"P":[]}"#;
        let html = self.get_text("Bundles/View", path).await?;
        let html = scraper::Html::parse_document(&html);

        let selector = scraper::Selector::parse("#app > ul:nth-child(3) a")
            .map_err(selector_should_be_valid)?;
        let mut bundles: Vec<_> = html.select(&selector).map(|node| node.inner_html()).collect();

        // Remove the last two items (Generated Engine Collections, Generated Hierarchy Assets)
        bundles.truncate(bundles.len() - 2);

        Ok(bundles)
    }

    /// Returns a list of collections in the specified bundle.
    pub async fn collections(&mut self, bundle_no: usize) -> Result<Vec<String>> {
        let path = format!(r#"{{"P":[{bundle_no}]}}"#);
        let html = self.get_text("Bundles/View", &path).await?;
        let html = scraper::Html::parse_document(&html);

        let selector = scraper::Selector::parse("#app > ul:nth-child(5) a")
            .map_err(selector_should_be_valid)?;

        Ok(html.select(&selector).map(|node| node.inner_html()).collect())
    }

    /// Returns the number of assets in the specified collection.
    pub async fn assets(
        &mut self,
        bundle_no: usize,
        collection_no: usize,
    ) -> Result<Vec<AssetEntry>> {
        let path = format!(r#"{{"B":{{"P":[{bundle_no}]}},"I":{collection_no}}}"#);
        let html = self.get_text("Collections/View", &path).await?;
        let html = scraper::Html::parse_document(&html);

        let selector = scraper::Selector::parse("tbody > tr").map_err(selector_should_be_valid)?;

        let first_child_should_exist = || Repr::bug("first child should exist");
        let inner_text_should_exist = || Repr::bug("inner text should exist");

        let mut assets = Vec::new();
        for nodes in html.select(&selector) {
            let children = nodes.children().collect::<Vec<_>>();
            let path_id: i64 = children[0]
                .first_child()
                .ok_or_else(first_child_should_exist)?
                .value()
                .as_text()
                .ok_or_else(inner_text_should_exist)?
                .parse()
                .map_err(|_| Repr::bug("inner text should be integer"))?;
            let class = children[1]
                .first_child()
                .ok_or_else(first_child_should_exist)?
                .value()
                .as_text()
                .ok_or_else(inner_text_should_exist)?
                .to_string();
            let name = children[2]
                .first_child()
                .ok_or_else(first_child_should_exist)?
                .first_child()
                .ok_or_else(first_child_should_exist)?
                .value()
                .as_text()
                .ok_or_else(inner_text_should_exist)?
                .to_string();

            assets.push(AssetEntry(path_id, class, name));
        }

        Ok(assets)
    }

    /// Returns the number of assets in the specified collection.
    pub async fn asset_count(&mut self, bundle_no: usize, collection_no: usize) -> Result<usize> {
        let path = format!(r#"{{"B":{{"P":[{bundle_no}]}},"I":{collection_no}}}"#);
        let text = self.get_text("Collections/Count", &path).await?;

        let count = text.parse().map_err(|_| Repr::bug("text should be integer"))?;

        Ok(count)
    }

    /// Returns information about the specified asset.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error`] with [`crate::ErrorKind::Network`] if the request fails.
    pub async fn asset_info(
        &mut self,
        bundle_no: usize,
        collection_no: usize,
        path_id: i64,
    ) -> Result<AssetInfo> {
        let path =
            format!(r#"{{"C":{{"B":{{"P":[{bundle_no}]}},"I":{collection_no}}},"D":{path_id}}}"#);
        let html = self.get_text("Assets/View", &path).await?;
        let html = scraper::Html::parse_document(&html);

        let name_selector = scraper::Selector::parse("h1").map_err(selector_should_be_valid)?;
        let info_selector =
            scraper::Selector::parse("#nav-information td").map_err(selector_should_be_valid)?;

        let name = html
            .select(&name_selector)
            .next()
            .ok_or_else(|| Repr::bug("asset name should exist"))?
            .inner_html();
        let info = html.select(&info_selector).map(|node| node.inner_html()).collect::<Vec<_>>();

        let class = info[3].clone();
        if class == "AssetBundle" {
            return Ok(AssetInfo {
                entry: AssetEntry(path_id, class, name.clone()),
                original_path: None,
                asset_bundle_name: name,
            });
        }

        Ok(AssetInfo {
            entry: AssetEntry(path_id, class, name),
            original_path: Some(info[4].clone()),
            asset_bundle_name: info[5].clone(),
        })
    }

    /// Returns the JSON representation of the specified asset.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error`] with [`crate::ErrorKind::Network`] if the request fails.
    pub async fn asset_json(
        &mut self,
        bundle_no: usize,
        collection_no: usize,
        path_id: i64,
    ) -> Result<serde_json::Value> {
        let path =
            format!(r#"{{"C":{{"B":{{"P":[{bundle_no}]}},"I":{collection_no}}},"D":{path_id}}}"#);

        let url = self.full_url("Assets/Json")?;
        let value = match self.send_request(&url, &path).await?.json().await {
            Ok(json) => Ok(json),
            Err(e) => Err(Error::decode(url, Some(e))),
        }?;

        Ok(value)
    }

    /// Returns the text data stream of the specified asset.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error`] with [`crate::ErrorKind::Network`] if the request fails.
    pub async fn asset_text(
        &mut self,
        bundle_no: usize,
        collection_no: usize,
        path_id: i64,
    ) -> Result<impl futures::Stream<Item = reqwest::Result<bytes::Bytes>>> {
        let path =
            format!(r#"{{"C":{{"B":{{"P":[{bundle_no}]}},"I":{collection_no}}},"D":{path_id}}}"#);
        let url = self.full_url("Assets/Text")?;

        Ok(self.send_request(&url, &path).await?.bytes_stream())
    }

    /// Returns the image (in png) data stream of the specified asset.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error`] with [`crate::ErrorKind::Network`] if the request fails.
    pub async fn asset_image(
        &mut self,
        bundle_no: usize,
        collection_no: usize,
        path_id: i64,
    ) -> Result<impl futures::Stream<Item = reqwest::Result<bytes::Bytes>>> {
        let path =
            format!(r#"{{"C":{{"B":{{"P":[{bundle_no}]}},"I":{collection_no}}},"D":{path_id}}}"#);

        let url = self.full_url("Assets/Image")?;
        let client = reqwest::Client::new();

        let req = client.get(url.clone()).query(&[("Path", path)]).query(&[("Extension", "png")]);

        let res = match req.send().await {
            Ok(r) => Ok(r),
            Err(e) => Err(Error::request(url, Some(e))),
        }?;

        Ok(res.bytes_stream())
    }

    /// Exports the primary content on the AssetRipper.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error`] with [`crate::ErrorKind::Network`] if the export request fails.
    pub async fn export_primary<P>(&mut self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let url = self.full_url("Export/PrimaryContent")?;
        let mut form = HashMap::new();
        form.insert("path", path.as_ref().to_string_lossy().to_string());

        let client = reqwest::Client::new();
        let req = client.post(url.clone()).form(&form);

        if let Err(e) = req.send().await {
            return Err(Error::request(url, Some(e)).into());
        }

        Ok(())
    }

    /// Downloads the latest release zip of AssetRipper from GitHub to the given path.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error`] with [`crate::ErrorKind::Network`] if the download fails.
    ///
    /// Returns [`crate::Error`] with [`crate::ErrorKind::Io`] if the file cannot be accessed.
    #[cfg(all(
        any(target_os = "windows", target_os = "linux", target_os = "macos"),
        any(target_arch = "x86_64", target_arch = "aarch64"),
    ))]
    pub async fn download_latest_zip<P>(
        path: P,
        progress_bar: Option<&mut ProgressBar>,
    ) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let client = reqwest::Client::new();

        let zip_path = path.as_ref().join("AssetRipper.zip");

        let mut downloaded = false;

        let mut file = match tokio::fs::File::create_new(&zip_path).await {
            Ok(file) => Ok(file),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                log::debug!("AssetRipper.zip already exists, skipping download");
                downloaded = true;
                let f = tokio::fs::File::open(&zip_path)
                    .await
                    .map_err(|e| Repr::io("failed to open file", Some(e)))?;

                Ok(f)
            }
            Err(e) => Err(Repr::io("failed to create file", Some(e))),
        }?;

        if !downloaded {
            let os = match std::env::consts::OS {
                "windows" => "win",
                "linux" => "linux",
                "macos" => "mac",
                _ => unreachable!("unsupported OS"),
            };
            let arch = match std::env::consts::ARCH {
                "x86_64" => "x64",
                "aarch64" => "arm64",
                _ => unreachable!("unsupported architecture"),
            };

            let base_url = "https://github.com/AssetRipper/AssetRipper/releases/latest/download/";
            let url = reqwest::Url::parse(&format!("{base_url}/AssetRipper_{os}_{arch}.zip"))
                .map_err(|_| Repr::bug("url should be valid"))?;

            let res = match client.get(url.clone()).send().await {
                Ok(res) => Ok(res),
                Err(e) => Err(Error::request(url, Some(e))),
            }?;

            let stream_reader =
                res.bytes_stream().map_err(std::io::Error::other).into_async_read().compat();

            let mut stream_reader = ProgressReadAdapter::new(stream_reader, progress_bar);
            tokio::io::copy(&mut stream_reader, &mut file)
                .await
                .map_err(|e| Repr::io("failed to download zip file", Some(e)))?;
        }

        let mut file = zip::ZipArchive::new(file.into_std().await).map_err(|e| match e {
            zip::result::ZipError::Io(e) => Repr::io("failed to open zip file", Some(e)),
            _ => Repr::bug("zip file should be valid"),
        })?;

        let mut output_path = zip_path.clone();
        output_path.pop();
        output_path.push("AssetRipper");

        log::debug!("unzip to {}", output_path.display());
        file.extract(output_path).map_err(|e| match e {
            zip::result::ZipError::Io(e) => Repr::io("failed to extract zip file", Some(e)),
            _ => Repr::bug("zip file should be valid"),
        })?;
        drop(file);

        tokio::fs::remove_file(zip_path)
            .await
            .map_err(|e| Repr::io("failed to remove zip file", Some(e)))?;

        Ok(())
    }
}

impl Drop for AssetRipper {
    fn drop(&mut self) {
        if let Some(ref mut process) = self.process {
            process.kill().expect("failed to kill AssetRipper process");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{MAIN_SEPARATOR_STR, Path};

    use super::AssetRipper;

    const ASSET_RIPPER_PATH: &str = if cfg!(windows) {
        concat!(env!("CARGO_MANIFEST_DIR"), "/AssetRipper/AssetRipper.GUI.Free.exe")
    } else {
        concat!(env!("CARGO_MANIFEST_DIR"), "/AssetRipper/AssetRipper.GUI.Free")
    };

    const TEST_CASES: &[(&str, usize, usize)] = &[
        (concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_acb.unity3d"), 1, 2),
        (concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_sprite.unity3d"), 1, 14),
        (concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_text.unity3d"), 1, 2),
    ];

    #[tokio::test]
    #[ignore = "need to download AssetRipper"]
    async fn test_bundles() {
        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56789).unwrap();
        asset_ripper.load(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")).await.unwrap();

        let bundles = asset_ripper.bundles().await.unwrap();

        assert_eq!(bundles.len(), 3);
    }

    #[tokio::test]
    #[ignore = "need to download AssetRipper"]
    async fn test_collections() {
        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56790).unwrap();

        for (path, collection_count, _) in TEST_CASES {
            asset_ripper.load(path).await.unwrap();
            let collections = asset_ripper.collections(0).await.unwrap();
            assert_eq!(collections.len(), *collection_count);
        }
    }

    #[tokio::test]
    #[ignore = "need to download AssetRipper"]
    async fn test_asset_count() {
        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56791).unwrap();

        for (path, _, asset_count) in TEST_CASES {
            asset_ripper.load(*path).await.unwrap();
            let count = asset_ripper.asset_count(0, 0).await.unwrap();
            assert_eq!(count, *asset_count);
        }
    }

    #[tokio::test]
    #[ignore = "need to download AssetRipper"]
    async fn test_assets() {
        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56792).unwrap();

        for (path, _, asset_count) in TEST_CASES {
            asset_ripper.load(path).await.unwrap();
            let assets = asset_ripper.assets(0, 0).await.unwrap();
            assert_eq!(assets.len(), *asset_count);
        }
    }

    #[tokio::test]
    #[ignore = "need to download AssetRipper"]
    async fn test_asset_info() {
        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56793).unwrap();

        let (acb_path, ..) = TEST_CASES[0];
        asset_ripper.load(acb_path).await.unwrap();
        let asset_entry = &asset_ripper.assets(0, 0).await.unwrap()[0];
        let asset_info = asset_ripper.asset_info(0, 0, asset_entry.0).await.unwrap();

        assert_eq!(
            asset_info.original_path,
            Some(
                ["Assets", "imas", "resources", "adx2", "song3", "song3_00test.acb.bytes"]
                    .join(MAIN_SEPARATOR_STR)
            )
        );
    }
}
