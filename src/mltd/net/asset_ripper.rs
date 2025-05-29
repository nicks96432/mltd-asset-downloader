//! AssetRipper client.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use futures::TryStreamExt;
use indicatif::ProgressBar;
use reqwest::Response;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use crate::Error;
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

impl AssetRipper {
    /// Starts a new AssetRipper instance with the given executable path and port.
    pub fn new<P>(path: P, port: u16) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let process = match Command::new(path.as_ref().as_os_str())
            .args(["--port", &port.to_string()])
            .args(["--launch-browser", "false"])
            .args(["--log", "false"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .env_clear()
            .spawn()
        {
            Ok(mut process) => {
                let reader = BufReader::new(process.stdout.take().unwrap());
                for line in reader.lines() {
                    let line = line.expect("failed to read line");
                    if line.contains("listening on: http://") {
                        break;
                    }
                }

                process
            }
            Err(err) => return Err(err.into()),
        };

        Ok(Self { base_url: format!("http://localhost:{port}"), process: Some(process) })
    }

    /// Connects th an existing AssetRipper instance with the given host and port.
    pub fn connect(host: &str, port: u16) -> Result<Self, Error> {
        Ok(Self { base_url: format!("http://{host}:{port}"), process: None })
    }

    /// Loads an asset or a folder into the AssetRipper.
    pub async fn load<P>(&mut self, path: P) -> Result<(), Error>
    where
        P: AsRef<Path>,
    {
        let url = if path.as_ref().is_dir() { "LoadFolder" } else { "LoadFile" };
        let url = format!("{}/{}", &self.base_url, url);

        let mut form = HashMap::new();
        form.insert("path", path.as_ref().to_string_lossy().to_string());

        let client = reqwest::Client::new();
        let req = client.post(url).form(&form);

        self.check_process()?;

        if let Err(e) = req.send().await {
            return Err(Error::Request(e));
        }

        Ok(())
    }

    /// Sends a GET request with the given path parameter to AssetRipper.
    pub async fn send_request(&mut self, url: &str, path: &str) -> Result<Response, Error> {
        let url = format!("{}/{}", &self.base_url, url);
        let client = reqwest::Client::new();

        let req = client.get(url).query(&[("Path", path)]);

        self.check_process()?;

        let res = match req.send().await {
            Ok(r) => r,
            Err(e) => return Err(Error::Request(e)),
        };

        Ok(res)
    }

    /// Returns a list of loaded bundles.
    pub async fn bundles(&mut self) -> Result<Vec<String>, Error> {
        let path = r#"{"P":[]}"#;

        let html = match self.send_request("Bundles/View", path).await?.text().await {
            Ok(html) => html,
            Err(e) => return Err(Error::ResponseDeserialize(e)),
        };

        let html = scraper::Html::parse_document(&html);
        let selector =
            scraper::Selector::parse("#app > ul:nth-child(3) a").expect("cannot create selector");
        let mut bundles: Vec<_> = html.select(&selector).map(|node| node.inner_html()).collect();

        // Remove the last two items (Generated Engine Collections, Generated Hierarchy Assets)
        bundles.truncate(bundles.len() - 2);

        Ok(bundles)
    }

    /// Returns a list of collections in the specified bundle.
    pub async fn collections(&mut self, bundle_no: usize) -> Result<Vec<String>, Error> {
        let path = format!(r#"{{"P":[{bundle_no}]}}"#);

        let html = match self.send_request("Bundles/View", &path).await?.text().await {
            Ok(html) => html,
            Err(e) => return Err(Error::ResponseDeserialize(e)),
        };

        let html = scraper::Html::parse_document(&html);
        let selector =
            scraper::Selector::parse("#app > ul:nth-child(5) a").expect("cannot create selector");

        Ok(html.select(&selector).map(|node| node.inner_html()).collect())
    }

    /// Returns the number of assets in the specified collection.
    pub async fn assets(
        &mut self,
        bundle_no: usize,
        collection_no: usize,
    ) -> Result<Vec<AssetEntry>, Error> {
        let path = format!(r#"{{"B":{{"P":[{bundle_no}]}},"I":{collection_no}}}"#);

        let html = match self.send_request("Collections/View", &path).await?.text().await {
            Ok(html) => html,
            Err(e) => return Err(Error::ResponseDeserialize(e)),
        };

        let html = scraper::Html::parse_document(&html);
        let selector = scraper::Selector::parse("tbody > tr").expect("cannot create selector");

        let mut assets = Vec::new();
        for nodes in html.select(&selector) {
            let children = nodes.children().collect::<Vec<_>>();
            let path_id: i64 = children[0]
                .first_child()
                .expect("<td>")
                .value()
                .as_text()
                .expect("inner text")
                .parse()?;
            let class = children[1]
                .first_child()
                .expect("<td>")
                .value()
                .as_text()
                .expect("inner text")
                .to_string();
            let name = children[2]
                .first_child()
                .expect("<td>")
                .first_child()
                .expect("<a>")
                .value()
                .as_text()
                .expect("inner text")
                .to_string();

            assets.push(AssetEntry(path_id, class, name));
        }

        Ok(assets)
    }

    /// Returns the number of assets in the specified collection.
    pub async fn asset_count(
        &mut self,
        bundle_no: usize,
        collection_no: usize,
    ) -> Result<usize, Error> {
        let path = format!(r#"{{"B":{{"P":[{bundle_no}]}},"I":{collection_no}}}"#);

        let text = match self.send_request("Bundles/View", &path).await?.text().await {
            Ok(text) => text,
            Err(e) => return Err(Error::ResponseDeserialize(e)),
        };

        Ok(text.parse()?)
    }

    /// Returns information about the specified asset.
    pub async fn asset_info(
        &mut self,
        bundle_no: usize,
        collection_no: usize,
        path_id: i64,
    ) -> Result<AssetInfo, Error> {
        let path =
            format!(r#"{{"C":{{"B":{{"P":[{bundle_no}]}},"I":{collection_no}}},"D":{path_id}}}"#);

        let html = match self.send_request("Assets/View", &path).await?.text().await {
            Ok(html) => html,
            Err(e) => return Err(Error::ResponseDeserialize(e)),
        };

        let html = scraper::Html::parse_document(&html);
        let name_selector = scraper::Selector::parse("h1").expect("cannot create selector");
        let info_selector =
            scraper::Selector::parse("#nav-information td").expect("cannot create selector");

        let name = html.select(&name_selector).next().unwrap().inner_html();
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
    pub async fn asset_json(
        &mut self,
        bundle_no: usize,
        collection_no: usize,
        path_id: i64,
    ) -> Result<serde_json::Value, Error> {
        let path =
            format!(r#"{{"C":{{"B":{{"P":[{bundle_no}]}},"I":{collection_no}}},"D":{path_id}}}"#);

        match self.send_request("Assets/Json", &path).await?.json().await {
            Ok(json) => Ok(json),
            Err(e) => Err(Error::ResponseDeserialize(e)),
        }
    }

    /// Returns the text data stream of the specified asset.
    pub async fn asset_text(
        &mut self,
        bundle_no: usize,
        collection_no: usize,
        path_id: i64,
    ) -> Result<impl futures::Stream<Item = reqwest::Result<bytes::Bytes>>, Error> {
        let path =
            format!(r#"{{"C":{{"B":{{"P":[{bundle_no}]}},"I":{collection_no}}},"D":{path_id}}}"#);

        Ok(self.send_request("Assets/Text", &path).await?.bytes_stream())
    }

    /// Returns the image (in png) data stream of the specified asset.
    pub async fn asset_image(
        &mut self,
        bundle_no: usize,
        collection_no: usize,
        path_id: i64,
    ) -> Result<impl futures::Stream<Item = reqwest::Result<bytes::Bytes>> + use<>, Error> {
        let path =
            format!(r#"{{"C":{{"B":{{"P":[{bundle_no}]}},"I":{collection_no}}},"D":{path_id}}}"#);

        let url = format!("{}/Assets/Image", &self.base_url);
        let client = reqwest::Client::new();

        let req = client.get(url).query(&[("Path", path)]).query(&[("Extension", "png")]);

        self.check_process()?;

        let res = match req.send().await {
            Ok(r) => r,
            Err(e) => return Err(Error::Request(e)),
        };

        Ok(res.bytes_stream())
    }

    /// Exports the primary content on the AssetRipper.
    pub async fn export_primary<P>(&mut self, path: P) -> Result<(), Error>
    where
        P: AsRef<Path>,
    {
        let url = format!("{}/Export/PrimaryContent", &self.base_url);
        let mut form = HashMap::new();
        form.insert("path", path.as_ref().to_string_lossy().to_string());

        let client = reqwest::Client::new();
        let req = client.post(&url).form(&form);

        if let Err(e) = req.send().await {
            return Err(Error::Request(e));
        };

        Ok(())
    }

    fn check_process(&mut self) -> Result<(), Error> {
        if let Some(ref mut process) = self.process {
            match process.try_wait() {
                Ok(None) => Ok(()),
                Ok(Some(status)) => {
                    Err(Error::Generic(format!("AssetRipper process died with status {status}")))
                }
                Err(err) => Err(err.into()),
            }?;
        }

        Ok(())
    }

    /// Downloads the latest release zip of AssetRipper from GitHub to the given path.
    pub async fn download_latest_zip<P>(
        path: P,
        progress_bar: Option<&mut ProgressBar>,
    ) -> Result<(), Error>
    where
        P: AsRef<Path>,
    {
        let client = reqwest::Client::new();
        let base_url = "https://github.com/AssetRipper/AssetRipper/releases/latest/download/";

        let zip_path = path.as_ref().join("AssetRipper.zip");
        let file = match tokio::fs::File::create_new(&zip_path).await {
            Ok(mut file) => {
                let os = match std::env::consts::OS {
                    "windows" => "win",
                    "linux" => "linux",
                    "macos" => "mac",
                    _ => return Err(Error::Generic("unsupported OS".to_string())),
                };
                let arch = match std::env::consts::ARCH {
                    "x86_64" => "x64",
                    "aarch64" => "arm64",
                    _ => return Err(Error::Generic("unsupported architecture".to_string())),
                };
                let req = client.get(format!("{base_url}/AssetRipper_{os}_{arch}.zip"));

                let res = match req.send().await {
                    Ok(res) => res,
                    Err(e) => return Err(Error::Request(e)),
                };

                let stream_reader = res
                    .bytes_stream()
                    .map_err(|e| std::io::Error::other(e))
                    .into_async_read()
                    .compat();

                let mut stream_reader = ProgressReadAdapter::new(stream_reader, progress_bar);
                tokio::io::copy(&mut stream_reader, &mut file).await?;

                file
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::AlreadyExists => {
                    log::debug!("AssetRipper.zip already exists, skipping download");
                    tokio::fs::File::open(&zip_path).await?
                }
                _ => return Err(e.into()),
            },
        };

        let mut file = zip::ZipArchive::new(file.into_std().await)?;

        let mut output_path = zip_path.clone();
        output_path.pop();
        output_path.push("AssetRipper");
        log::debug!("unzip to {}", output_path.display());
        file.extract(output_path)?;
        drop(file);

        tokio::fs::remove_file(zip_path).await?;

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
    use super::AssetRipper;
    const ASSET_RIPPER_PATH: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/AssetRipper/AssetRipper.GUI.Free");
    const TEST_CASES: &[(&str, usize, usize)] = &[
        (concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_acb.unity3d"), 1, 2),
        (concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_sprite.unity3d"), 1, 14),
        (concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_text.unity3d"), 1, 2),
    ];

    #[tokio::test]
    #[ignore]
    async fn test_bundles() {
        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56789).unwrap();
        asset_ripper.load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests")).await.unwrap();

        let bundles = asset_ripper.bundles().await.unwrap();

        assert_eq!(bundles.len(), 3);
    }

    #[tokio::test]
    #[ignore]
    async fn test_collections() {
        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56790).unwrap();

        for (path, collection_count, _) in TEST_CASES {
            asset_ripper.load(path).await.unwrap();
            let collections = asset_ripper.collections(0).await.unwrap();
            assert_eq!(collections.len(), *collection_count);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_asset_count() {
        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56791).unwrap();

        for (path, _, asset_count) in TEST_CASES {
            asset_ripper.load(path).await.unwrap();
            let count = asset_ripper.asset_count(0, 0).await.unwrap();
            assert_eq!(count, *asset_count);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_assets() {
        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56792).unwrap();

        for (path, _, asset_count) in TEST_CASES {
            asset_ripper.load(path).await.unwrap();
            let assets = asset_ripper.assets(0, 0).await.unwrap();
            assert_eq!(assets.len(), *asset_count);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_asset_info() {
        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56793).unwrap();

        let (acb_path, ..) = TEST_CASES[0];
        asset_ripper.load(acb_path).await.unwrap();
        let asset_entry = &asset_ripper.assets(0, 0).await.unwrap()[0];
        let asset_info = asset_ripper.asset_info(0, 0, asset_entry.0).await.unwrap();

        assert_eq!(
            asset_info.original_path,
            Some(String::from("Assets/imas/resources/adx2/song3/song3_00test.acb.bytes"))
        );
    }
}
