//! Asset Ripper client.

use std::borrow::Cow;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use ahash::{HashMapExt, HashSetExt};
use reqwest::Url;
use reqwest::blocking::{Client, Response};
use tempfile::tempdir;

use super::{AssetInfo, Extractor};
use crate::error::{Repr, Result};
use crate::net::Error;
use crate::util::{HashMap, HashSet};

/// Map of assets in the format of
/// {`bundle_name`: (`bundle_no`, {`collection_name`: (`collection_no`, {`path_id`: `AssetInfo`})})}
type AssetsMap = HashMap<String, (usize, HashMap<String, (usize, HashMap<i64, AssetInfo>)>)>;

/// Asset Ripper client.
#[derive(Debug)]
pub struct AssetRipper {
    /// Base URL of Asset Ripper.
    base_url: String,

    /// HTTP client.
    client: Client,

    /// Asset Ripper child process.
    process: Option<Child>,

    /// Map of loaded asset infos.
    assets: AssetsMap,

    /// Set of load paths.
    loaded_paths: HashSet<String>,
}

impl AssetRipper {
    fn full_url(&self, rel_url: &str) -> Url {
        let url = format!("{}/{}", self.base_url, rel_url);
        Url::parse(&url).expect("url suould be valid")
    }

    /// Sends a GET request with the given path parameter to Asset Ripper.
    fn send_request(&self, url: &Url, path: &str) -> Result<Response, Error> {
        let req = self.client.get(url.clone());
        let req = if path.is_empty() { req } else { req.query(&[("Path", path)]) };
        let req = req.build().map_err(|e| Error::request(url.clone(), Some(e)))?;

        tracing::trace!(?req, "sending request");

        let response = match self.client.execute(req) {
            Ok(r) => Ok(r),
            Err(e) => Err(Error::request(url.clone(), Some(e))),
        }?;

        Ok(response)
    }

    fn get_text(&self, rel_url: &str, path: &str) -> Result<String, Error> {
        let url = self.full_url(rel_url);

        let text = match self.send_request(&url, path)?.text() {
            Ok(t) => Ok(t),
            Err(e) => Err(Error::decode(url, Some(e))),
        }?;

        Ok(text)
    }

    /// Starts a new Asset Ripper instance with the given executable path and port.
    ///
    /// # Errors
    ///
    /// This method fails if the process fails to start.
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
            .stderr(std::io::stdout())
            .spawn()
        {
            Ok(mut process) => {
                let reader = BufReader::new(
                    process.stdout.take().ok_or_else(|| Repr::io("failed to get stdout", None))?,
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

        Ok(Self {
            base_url: format!("http://localhost:{port}"),
            client: Client::new(),
            process: Some(process),
            assets: HashMap::new(),
            loaded_paths: HashSet::new(),
        })
    }

    /// Connects to an existing Asset Ripper instance with the given host and port.
    ///
    /// # Errors
    ///
    /// This method fails if the network request to the host fails.
    pub fn connect(host: &str, port: u16) -> Result<Self> {
        let client = Client::new();
        let this = Self {
            base_url: format!("http://{host}:{port}"),
            client,
            process: None,
            assets: HashMap::new(),
            loaded_paths: HashSet::new(),
        };

        let url = this.full_url("");
        let req = this.client.head(url.clone());

        let _ = req.send().map_err(|e| Error::request(url, Some(e)))?;

        Ok(this)
    }

    fn load_bundles(&self) -> Result<Vec<String>> {
        let html = {
            let s = self.get_text("Bundles/View", r#"{"P":[]}"#)?;
            scraper::Html::parse_document(&s)
        };

        let selector =
            scraper::Selector::parse("#app > ul:nth-child(3) a").expect("selector should be valid");
        let mut bundles: Vec<_> = html.select(&selector).map(|node| node.inner_html()).collect();

        // Remove the last two items (Generated Engine Collections, Generated Hierarchy Assets)
        bundles.truncate(bundles.len() - 2);

        Ok(bundles)
    }

    fn load_collection(&self, bundle_no: usize) -> Result<Vec<String>> {
        let html = self.get_text("Bundles/View", &format!(r#"{{"P":[{bundle_no}]}}"#))?;
        let html = scraper::Html::parse_document(&html);

        let selector =
            scraper::Selector::parse("#app > ul:nth-child(5) a").expect("selector should be valid");

        Ok(html.select(&selector).map(|node| node.inner_html()).collect())
    }

    fn path_ids(&self, bundle_no: usize, collection_no: usize) -> Result<Vec<i64>> {
        let path = format!(r#"{{"B":{{"P":[{bundle_no}]}},"I":{collection_no}}}"#);
        let html = self.get_text("Collections/View", &path)?;
        let html = scraper::Html::parse_document(&html);

        let selector = scraper::Selector::parse("tbody > tr").expect("selector should be valid");

        let mut assets = Vec::new();
        for nodes in html.select(&selector) {
            let children = nodes.children().collect::<Vec<_>>();
            let path_id: i64 = children[0]
                .first_child()
                .expect("first child should exist")
                .value()
                .as_text()
                .expect("inner text should exist")
                .parse()
                .expect("inner text should be integer");

            assets.push(path_id);
        }

        Ok(assets)
    }

    fn load_asset(
        &self,
        bundle_no: usize,
        collection_no: usize,
        path_id: i64,
    ) -> Result<AssetInfo> {
        let path = format!(
            r#"{{"C":{{"B":{{"P":[{}]}},"I":{}}},"D":{}}}"#,
            bundle_no, collection_no, path_id
        );
        let html = self.get_text("Assets/View", &path)?;
        let html = scraper::Html::parse_document(&html);

        let name_selector = scraper::Selector::parse("h1").expect("selector should be valid");
        let info_selector =
            scraper::Selector::parse("#nav-information td").expect("selector should be valid");

        let name =
            html.select(&name_selector).next().expect("asset name should exist").inner_html();
        let info_nodes = html.select(&info_selector).collect::<Vec<_>>();

        let class = info_nodes[3].inner_html();
        let collection_name = info_nodes[0]
            .first_child()
            .expect("ancher tag should exist")
            .first_child()
            .expect("inner text should exist")
            .value()
            .as_text()
            .expect("inner element should be text")
            .to_string();

        if class == "AssetBundle" {
            return Ok(AssetInfo {
                path_id,
                class: Cow::Owned(class),
                name: Cow::Owned(name.clone()),
                orig_path: None,
                orig_bundle: Cow::Owned(name),
                collection: Cow::Owned(collection_name),
            });
        }

        Ok(AssetInfo {
            path_id,
            class: Cow::Owned(class),
            name: Cow::Owned(name),
            orig_path: Some(Cow::Owned(info_nodes[4].inner_html())),
            orig_bundle: Cow::Owned(info_nodes[5].inner_html()),
            collection: Cow::Owned(collection_name),
        })
    }

    /// Exports the primary contents in the Asset Ripper.
    ///
    /// # Errors
    ///
    /// This method fails if the export request fails.
    pub fn export_primary<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let url = self.full_url("Export/PrimaryContent");

        if let Err(e) = self.client.post(url.clone()).form(&[("path", path.as_ref())]).send() {
            return Err(Error::request(url, Some(e)).into());
        }

        Ok(())
    }

    /// Updates the settings on the Asset Ripper.
    ///
    /// # Errors
    ///
    /// This method fails if the update request fails.
    pub fn set(&self, form: &HashMap<String, String>) -> Result<()> {
        let url = self.full_url("Settings/update");

        if let Err(e) = self.client.post(url.clone()).form(form).send() {
            return Err(Error::request(url, Some(e)).into());
        }

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

impl Extractor for AssetRipper {
    fn load_file(&mut self, path: &Path) -> Result<()> {
        if path.is_dir() {
            return Err(Repr::io("path is a directory, use load_folder instead", None).into());
        }

        let path = path.to_string_lossy().to_string();
        if self.loaded_paths.contains(&path) {
            tracing::debug!(?path, "asset already loaded, skipping");
            return Ok(());
        }

        // Load file and folder actually do the same things currently. See [here][here1] and
        // [here][here2] for more details.
        //
        // [here1]: https://github.com/AssetRipper/AssetRipper/blob/e70e206184e506dd0cc4153288f22a13c9c9f74c/Source/AssetRipper.GUI.Web/Pages/Commands.cs#L56
        // [here2]: https://github.com/AssetRipper/AssetRipper/blob/e70e206184e506dd0cc4153288f22a13c9c9f74c/Source/AssetRipper.GUI.Web/Pages/Commands.cs#L84
        let url = self.full_url("LoadFile");

        // Asset Ripper cannot load new files after loading any files, so we need to
        // load the old files again.
        let form = self
            .loaded_paths
            .iter()
            .chain(std::iter::once(&path))
            .map(|p| ("Path", p))
            .collect::<Vec<_>>();

        let req = self
            .client
            .post(url.clone())
            .form(&form)
            .build()
            .map_err(|e| Error::request(url.clone(), Some(e)))?;

        tracing::trace!(?req, body = ?req.body(), "sending load file request");

        if let Err(e) = self.client.execute(req) {
            return Err(Error::request(url, Some(e)).into());
        }
        drop(form);

        let bundles = self.load_bundles()?;

        for (bundle_id, _) in bundles.into_iter().enumerate() {
            let mut collections = HashMap::new();

            let mut orig_bundle_name = String::new();

            for (collection_id, collection) in
                self.load_collection(bundle_id)?.into_iter().enumerate()
            {
                let mut assets = HashMap::new();
                for asset in self
                    .path_ids(bundle_id, collection_id)?
                    .into_iter()
                    .map(|path_id| self.load_asset(bundle_id, collection_id, path_id))
                {
                    let asset = asset?;
                    let bundle_name = asset.orig_bundle.to_string();
                    assets.insert(asset.path_id, asset);

                    if orig_bundle_name.is_empty() {
                        orig_bundle_name = bundle_name;
                    } else if orig_bundle_name != bundle_name {
                        tracing::warn!(
                            first_bundle_name = orig_bundle_name,
                            cur_bundle_name = bundle_name,
                            "bundle name mismatch, using the first one"
                        );
                    }
                }
                collections.insert(collection, (collection_id, assets));
            }

            self.assets.insert(orig_bundle_name.clone(), (bundle_id, collections));
        }

        self.loaded_paths.insert(path);

        Ok(())
    }

    #[inline]
    fn bundles(&self) -> Vec<&str> {
        self.assets.keys().map(String::as_str).collect()
    }

    #[inline]
    fn collections<'a>(&'a self, bundle: &str) -> Option<Vec<&'a str>> {
        Some(self.assets.get(bundle)?.1.keys().map(String::as_str).collect())
    }

    #[inline]
    fn asset_infos<'a>(&'a self, bundle: &str, collection: &str) -> Option<Vec<&'a AssetInfo>> {
        Some(self.assets.get(bundle)?.1.get(collection)?.1.values().collect())
    }

    #[inline]
    fn asset_info<'a>(
        &'a self,
        bundle: &str,
        collection: &str,
        path_id: i64,
    ) -> Option<&'a AssetInfo> {
        self.assets.get(bundle)?.1.get(collection)?.1.get(&path_id)
    }

    fn asset_serialized(&self, info: &AssetInfo) -> Result<serde_json::Value> {
        let (bundle_no, collection) = self
            .assets
            .get(info.orig_bundle.as_ref())
            .unwrap_or_else(|| panic!("bundle {} should exist", info.orig_bundle));
        let (collection_no, _) = collection
            .get(info.collection.as_ref())
            .unwrap_or_else(|| panic!("collection {} should exist", info.collection));
        let path_id = info.path_id;

        let path =
            format!(r#"{{"C":{{"B":{{"P":[{bundle_no}]}},"I":{collection_no}}},"D":{path_id}}}"#);
        let url = self.full_url("Assets/Json");
        let value = self
            .send_request(&url, &path)?
            .json()
            .map_err(|e| crate::net::Error::decode(url, Some(e)))?;

        Ok(value)
    }

    #[inline]
    fn all_asset_infos(&self) -> Vec<&AssetInfo> {
        self.assets.values().flat_map(|v| v.1.values().flat_map(|v| v.1.values())).collect()
    }

    fn extract(&self, info: &AssetInfo) -> Result<File> {
        let tempdir = tempdir().map_err(|e| Repr::io("cannot create tempdir", Some(e)))?;
        self.export_primary(tempdir.path())?;

        let file = File::open(
            tempdir.path().join(
                info.orig_path
                    .as_deref()
                    .ok_or_else(|| Repr::io("info does not have original path", None))?,
            ),
        )
        .map_err(|e| Repr::io("cannot open file", Some(e)))?;

        Ok(file)
    }

    fn clear(&mut self) {
        let url = self.full_url("Reset");

        if let Err(e) = self.client.post(url.clone()).send() {
            tracing::warn!("failed to reset asset ripper: {}", e);
        }

        self.assets.clear();
        self.loaded_paths.clear();
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::fs::File;
    use std::io::{BufReader, Read};
    use std::path::Path;

    use super::AssetRipper;
    use crate::unity::{AssetInfo, Extractor};
    use crate::util::init_test_logger;

    const ASSET_RIPPER_PATH: &str = if cfg!(windows) {
        concat!(env!("CARGO_MANIFEST_DIR"), "/AssetRipper/AssetRipper.GUI.Free.exe")
    } else {
        concat!(env!("CARGO_MANIFEST_DIR"), "/AssetRipper/AssetRipper.GUI.Free")
    };

    #[derive(Debug)]
    struct TestCase {
        pub asset_path: &'static str,
        pub collection_count: usize,
        pub asset_count: usize,
        pub asset_info: AssetInfo,
    }

    const TEST_CASES: &[TestCase] = &[
        TestCase {
            asset_path: concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_acb.unity3d"),
            collection_count: 1,
            asset_count: 2,
            asset_info: AssetInfo {
                path_id: -0x6924_71bf_cbe7_e9c6,
                class: Cow::Borrowed("TextAsset"),
                name: Cow::Borrowed("song3_00test.acb"),
                orig_path: Some(Cow::Borrowed(
                    "Assets/imas/resources/adx2/song3/song3_00test.acb.bytes",
                )),
                orig_bundle: Cow::Borrowed("song3_00test.acb.unity3d"),
                collection: Cow::Borrowed("cab-9f50c764e78a89007649817560896fbe"),
            },
        },
        TestCase {
            asset_path: concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_image.unity3d"),
            collection_count: 1,
            asset_count: 14,
            asset_info: AssetInfo {
                path_id: 0x5c6b_8a21_bcbc_972e,
                class: Cow::Borrowed("Texture2D"),
                name: Cow::Borrowed("014mir0011_0"),
                orig_path: Some(Cow::Borrowed("Assets/imas/resources/card/014mir0011_0.png")),
                orig_bundle: Cow::Borrowed("014mir0011_0.unity3d"),
                collection: Cow::Borrowed("cab-a374172171027544962fcca998dc31af"),
            },
        },
        TestCase {
            asset_path: concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_text.unity3d"),
            collection_count: 1,
            asset_count: 2,
            asset_info: AssetInfo {
                path_id: 0x7aa6_cd6c_f238_30a9,
                class: Cow::Borrowed("TextAsset"),
                name: Cow::Borrowed("card_blst_014mir0034_zh.gtx"),
                orig_path: Some(Cow::Borrowed(
                    "Assets/imas/resources/textassets/zh/commu/card/card_blst_014mir0034_zh.gtx.bytes",
                )),
                orig_bundle: Cow::Borrowed("card_blst_014mir0034_zh.gtx.unity3d"),
                collection: Cow::Borrowed("cab-720fa4e99ecaab2adf961cebffd0edb6"),
            },
        },
    ];

    #[test]
    fn test_load_file_bundles_clear() {
        init_test_logger();

        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56789).unwrap();

        assert_eq!(asset_ripper.bundles().len(), 0);

        for (i, case) in TEST_CASES.iter().enumerate() {
            asset_ripper.load_file(case.asset_path.as_ref()).unwrap();
            assert_eq!(asset_ripper.bundles().len(), i + 1);
        }

        asset_ripper.clear();
        assert_eq!(asset_ripper.bundles().len(), 0);
    }

    #[test]
    fn test_collections() {
        init_test_logger();

        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56790).unwrap();

        for case in TEST_CASES {
            asset_ripper.load_file(case.asset_path.as_ref()).unwrap();

            let collections = asset_ripper.collections(&case.asset_info.orig_bundle);
            assert_eq!(collections.unwrap().len(), case.collection_count);

            asset_ripper.clear();
        }
    }

    #[test]
    fn test_asset_infos() {
        init_test_logger();

        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56791).unwrap();

        for case in TEST_CASES {
            asset_ripper.load_file(case.asset_path.as_ref()).unwrap();

            let infos = asset_ripper
                .asset_infos(&case.asset_info.orig_bundle, &case.asset_info.collection)
                .unwrap();

            assert_eq!(infos.len(), case.asset_count);

            asset_ripper.clear();
        }
    }

    #[test]
    fn test_asset_info() {
        init_test_logger();

        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56792).unwrap();

        for case in TEST_CASES {
            asset_ripper.load_file(case.asset_path.as_ref()).unwrap();

            let info = asset_ripper.asset_info(
                &case.asset_info.orig_bundle,
                &case.asset_info.collection,
                case.asset_info.path_id,
            );
            assert!(info.is_some());
            assert_eq!(info.unwrap(), &case.asset_info);

            asset_ripper.clear();
        }
    }

    #[test]
    fn test_extract() {
        init_test_logger();

        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56793).unwrap();

        let case = &TEST_CASES[0];
        asset_ripper.load_file(case.asset_path.as_ref()).unwrap();

        let asset_info = asset_ripper
            .asset_info(
                &case.asset_info.orig_bundle,
                &case.asset_info.collection,
                case.asset_info.path_id,
            )
            .unwrap();

        let buf1 = {
            let file = asset_ripper.extract(asset_info).unwrap();
            let mut buf = vec![];
            BufReader::new(file).read_to_end(&mut buf).unwrap();
            buf
        };

        let buf2 = {
            let file =
                File::open(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_acb.acb")).unwrap();
            let mut buf = vec![];
            BufReader::new(file).read_to_end(&mut buf).unwrap();
            buf
        };

        assert_eq!(buf1, buf2);
    }

    #[test]
    fn test_load_multiple_extract() {
        init_test_logger();

        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56794).unwrap();
        for case in TEST_CASES {
            asset_ripper.load_file(case.asset_path.as_ref()).unwrap();
        }

        for case in TEST_CASES {
            let asset_info = asset_ripper
                .asset_info(
                    &case.asset_info.orig_bundle,
                    &case.asset_info.collection,
                    case.asset_info.path_id,
                )
                .unwrap();

            let buf = {
                let file = asset_ripper.extract(asset_info).unwrap();
                let mut buf = vec![];
                BufReader::new(file).read_to_end(&mut buf).unwrap();
                buf
            };

            assert_ne!(buf, [] as [u8; 0]);
        }
    }

    #[test]
    fn test_asset_serialized() {
        init_test_logger();

        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56795).unwrap();

        // skipping acb and text assets because they are too large
        let case = &TEST_CASES[1];
        asset_ripper.load_file(case.asset_path.as_ref()).unwrap();

        let asset_info = asset_ripper
            .asset_info(
                &case.asset_info.orig_bundle,
                &case.asset_info.collection,
                case.asset_info.path_id,
            )
            .unwrap();

        let value = asset_ripper.asset_serialized(asset_info).unwrap();
        let expected: serde_json::Value = serde_json::from_reader(
            File::open(Path::new(case.asset_path).with_extension("json")).unwrap(),
        )
        .unwrap();

        assert_eq!(value, expected);
    }
}
