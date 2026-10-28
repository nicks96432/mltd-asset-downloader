//! Asset Ripper client.

use std::cell::OnceCell;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use image::{DynamicImage, ImageFormat};
use reqwest::Url;
use reqwest::blocking::{Client, Response};
use serde_json::Value;
use tempfile::{TempDir, tempdir};

use super::float::Float;
use super::{Bundle, Class, Error as UnityError, Extractor, Object, Rect, Sprite};
use crate::error::{Repr, Result};
use crate::net::Error;
use crate::util::HashMap;

/// How long to wait for a spawned Asset Ripper to start listening.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);

/// Asset Ripper client.
#[derive(Debug)]
pub struct AssetRipper {
    /// Base URL of Asset Ripper.
    base_url: String,

    /// HTTP client.
    client: Client,

    /// Asset Ripper child process.
    process: Option<Child>,
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
        let mut process = Command::new(path.as_ref())
            .args(["--port", &port.to_string()])
            .args(["--launch-browser", "false"])
            .args(["--log", "false"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(std::io::stdout())
            .spawn()
            .map_err(|e| Repr::io("start AssetRipper", Some(path.as_ref()), e))?;

        let Some(stdout) = process.stdout.take() else {
            let _ = process.kill();
            let _ = process.wait();
            return Err(Repr::Process("cannot capture the output of AssetRipper".to_owned()).into());
        };

        // The reader thread keeps draining the output after startup so that AssetRipper
        // never blocks on a full pipe. It reports whether the server started listening.
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut listening = false;
            for line in BufReader::new(stdout).lines().map_while(std::result::Result::ok) {
                if !listening && line.contains("listening on: http://") {
                    listening = true;
                    let _ = tx.send(true);
                }
            }
            if !listening {
                let _ = tx.send(false);
            }
        });

        if !matches!(rx.recv_timeout(STARTUP_TIMEOUT), Ok(true)) {
            let _ = process.kill();
            let _ = process.wait();
            return Err(Repr::Process(format!(
                "AssetRipper did not start listening on port {port}"
            ))
            .into());
        }

        Ok(Self {
            base_url: format!("http://localhost:{port}"),
            client: Client::new(),
            process: Some(process),
        })
    }

    /// Connects to an existing Asset Ripper instance with the given host and port.
    ///
    /// # Errors
    ///
    /// This method fails if the network request to the host fails.
    pub fn connect(host: &str, port: u16) -> Result<Self> {
        let client = Client::new();
        let this = Self { base_url: format!("http://{host}:{port}"), client, process: None };

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
        bundles.truncate(bundles.len().saturating_sub(2));

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
            let path_id = nodes
                .children()
                .next()
                .and_then(|cell| cell.first_child())
                .and_then(|text| text.value().as_text())
                .and_then(|text| text.parse::<i64>().ok())
                .ok_or(UnityError::PageLayout("asset path id"))?;

            assets.push(path_id);
        }

        Ok(assets)
    }

    /// Reads the object `path_id` of the collection numbered `collection_no` of the bundle
    /// numbered `bundle_no`, which is the `slot`-th collection of the loaded file.
    fn load_object(
        &self,
        slot: usize,
        bundle_no: usize,
        collection_no: usize,
        path_id: i64,
    ) -> Result<Object> {
        let path = format!(
            r#"{{"C":{{"B":{{"P":[{}]}},"I":{}}},"D":{}}}"#,
            bundle_no, collection_no, path_id
        );
        let html = self.get_text("Assets/View", &path)?;
        let html = scraper::Html::parse_document(&html);

        let name_selector = scraper::Selector::parse("h1").expect("selector should be valid");
        let info_selector =
            scraper::Selector::parse("#nav-information td").expect("selector should be valid");

        let name = html
            .select(&name_selector)
            .next()
            .ok_or(UnityError::PageLayout("asset name"))?
            .inner_html();
        let info_nodes = html.select(&info_selector).collect::<Vec<_>>();
        // asset bundles do not have the original path and bundle name
        let is_bundle = info_nodes.get(3).is_some_and(|n| n.inner_html() == "AssetBundle");
        if info_nodes.len() < if is_bundle { 4 } else { 6 } {
            return Err(UnityError::PageLayout("asset information").into());
        }

        let class = Class::from_name(&info_nodes[3].inner_html());
        let orig_path = match class {
            // asset bundles contains bundle information only
            Class::AssetBundle => None,
            // the Windows version of Asset Ripper uses backslashes
            _ => Some(info_nodes[4].inner_html().replace('\\', "/")),
        };

        Ok(Object::new(slot, path_id, class, name, orig_path))
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
            // the process may have exited already, and panicking in drop could abort
            if let Err(e) = process.kill() {
                tracing::debug!(error = ?e, "failed to kill AssetRipper process");
            }
            let _ = process.wait();
        }
    }
}

impl Extractor for AssetRipper {
    type Bundle<'a> = AssetRipperBundle<'a>;

    fn load(&mut self, path: &Path) -> Result<AssetRipperBundle<'_>> {
        if path.is_dir() {
            return Err(UnityError::IsDirectory.into());
        }

        // Load file and folder actually do the same things currently. See [here][here1] and
        // [here][here2] for more details.
        //
        // [here1]: https://github.com/AssetRipper/AssetRipper/blob/e70e206184e506dd0cc4153288f22a13c9c9f74c/Source/AssetRipper.GUI.Web/Pages/Commands.cs#L56
        // [here2]: https://github.com/AssetRipper/AssetRipper/blob/e70e206184e506dd0cc4153288f22a13c9c9f74c/Source/AssetRipper.GUI.Web/Pages/Commands.cs#L84
        let url = self.full_url("LoadFile");
        let path = path.to_string_lossy().to_string();

        let req = self
            .client
            .post(url.clone())
            .form(&[("Path", &path)])
            .build()
            .map_err(|e| Error::request(url.clone(), Some(e)))?;

        tracing::trace!(?req, body = ?req.body(), "sending load file request");

        if let Err(e) = self.client.execute(req) {
            return Err(Error::request(url, Some(e)).into());
        }

        // the file stays loaded in AssetRipper until the bundle is dropped, even if listing fails
        let mut bundle = AssetRipperBundle {
            ripper: self,
            slots: Vec::new(),
            objects: Vec::new(),
            export: OnceCell::new(),
        };
        bundle.list_objects()?;

        Ok(bundle)
    }
}

/// A file loaded in [`AssetRipper`].
///
/// Asset Ripper cannot load new files after loading any files, so it is reset when the bundle
/// is dropped.
pub struct AssetRipperBundle<'a> {
    ripper: &'a AssetRipper,

    /// `(bundle_no, collection_no)` of the collections in Asset Ripper, indexed by
    /// [`Object::collection`].
    slots: Vec<(usize, usize)>,

    objects: Vec<Object>,

    /// Primary contents exported by Asset Ripper, which is done at most once.
    export: OnceCell<TempDir>,
}

impl AssetRipperBundle<'_> {
    fn list_objects(&mut self) -> Result<()> {
        let bundles = self.ripper.load_bundles()?;

        for bundle_no in 0..bundles.len() {
            for collection_no in 0..self.ripper.load_collection(bundle_no)?.len() {
                let slot = self.slots.len();
                self.slots.push((bundle_no, collection_no));

                for path_id in self.ripper.path_ids(bundle_no, collection_no)? {
                    let object =
                        self.ripper.load_object(slot, bundle_no, collection_no, path_id)?;
                    self.objects.push(object);
                }
            }
        }

        Ok(())
    }

    /// Makes sure that the object is of the `expected` class.
    fn check_class(object: &Object, expected: &Class, as_: &'static str) -> Result<()> {
        if object.class == *expected {
            return Ok(());
        }

        Err(UnityError::UnsupportedClass(object.class.to_string(), as_).into())
    }

    /// Gets the directory the primary contents are exported to, and exports them on first use.
    fn export_dir(&self) -> Result<&Path> {
        if let Some(dir) = self.export.get() {
            return Ok(dir.path());
        }

        let dir = tempdir().map_err(|e| Repr::io("create tempdir", None, e))?;
        self.ripper.export_primary(dir.path())?;

        Ok(self.export.get_or_init(|| dir).path())
    }

    /// Reads the exported file of the object.
    fn read_exported(&self, object: &Object) -> Result<Vec<u8>> {
        let orig_path = object.orig_path.as_deref().ok_or(UnityError::NoOriginalPath)?;
        let path: PathBuf = self.export_dir()?.join(orig_path);

        std::fs::read(&path).map_err(|e| Repr::io("read exported file", Some(&path), e).into())
    }

    /// Gets JSON serialized representation of the object. See [Unity documentation][doc]
    /// for more details.
    ///
    /// Although the original serialization format is YAML, JSON is used here
    /// for better compatibility with Rust ecosystem.
    ///
    /// [doc]: https://docs.unity3d.com/6000.2/Documentation/Manual/FormatDescription.html
    fn serialized(&self, object: &Object) -> Result<Value> {
        let &(bundle_no, collection_no) =
            self.slots.get(object.collection).ok_or(UnityError::ObjectNotFound(object.path_id))?;
        let path_id = object.path_id;

        let path =
            format!(r#"{{"C":{{"B":{{"P":[{bundle_no}]}},"I":{collection_no}}},"D":{path_id}}}"#);
        let url = self.ripper.full_url("Assets/Json");
        let value = self
            .ripper
            .send_request(&url, &path)?
            .json()
            .map_err(|e| crate::net::Error::decode(url, Some(e)))?;

        Ok(value)
    }
}

/// Reads a rectangle field of a serialized sprite.
fn rect_field(json: &Value, pointer: &'static str) -> Result<u32> {
    let value =
        json.pointer(pointer).and_then(Value::as_f64).ok_or(UnityError::MissingField(pointer))?;

    Float(value).to_u32()
}

impl Bundle for AssetRipperBundle<'_> {
    fn objects(&self) -> &[Object] {
        &self.objects
    }

    fn text(&self, object: &Object) -> Result<Vec<u8>> {
        Self::check_class(object, &Class::TextAsset, "text")?;

        self.read_exported(object)
    }

    fn texture(&self, object: &Object) -> Result<DynamicImage> {
        Self::check_class(object, &Class::Texture2D, "texture")?;

        let png = self.read_exported(object)?;

        image::load_from_memory_with_format(&png, ImageFormat::Png)
            .map_err(|e| UnityError::corrupted("decode exported texture", e).into())
    }

    fn sprite(&self, object: &Object) -> Result<Sprite> {
        Self::check_class(object, &Class::Sprite, "sprite")?;

        let json = self.serialized(object)?;
        let texture = json
            .pointer("/m_RD/m_Texture/m_PathID")
            .and_then(Value::as_i64)
            .ok_or(UnityError::MissingField("/m_RD/m_Texture/m_PathID"))?;

        Ok(Sprite {
            texture_id: texture,
            rect: Rect {
                x: rect_field(&json, "/m_Rect/m_X")?,
                y: rect_field(&json, "/m_Rect/m_Y")?,
                width: rect_field(&json, "/m_Rect/m_Width")?,
                height: rect_field(&json, "/m_Rect/m_Height")?,
            },
        })
    }
}

impl Drop for AssetRipperBundle<'_> {
    fn drop(&mut self) {
        let url = self.ripper.full_url("Reset");

        if let Err(e) = self.ripper.client.post(url).send() {
            tracing::warn!("failed to reset asset ripper: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::path::Path;

    use super::AssetRipper;
    use crate::unity::{Bundle, Class, Extractor, Object};
    use crate::util::init_test_logger;

    const ASSET_RIPPER_PATH: &str = if cfg!(windows) {
        concat!(env!("CARGO_MANIFEST_DIR"), "/AssetRipper/AssetRipper.GUI.Free.exe")
    } else {
        concat!(env!("CARGO_MANIFEST_DIR"), "/AssetRipper/AssetRipper.GUI.Free")
    };

    #[derive(Debug)]
    struct TestCase {
        path: &'static str,
        object_count: usize,
        path_id: i64,
        class: Class,
        name: &'static str,
        orig_path: &'static str,
    }

    impl TestCase {
        fn matches(&self, object: &Object) -> bool {
            object.path_id == self.path_id
                && object.class == self.class
                && object.name == self.name
                && object.orig_path.as_deref() == Some(self.orig_path)
        }
    }

    fn test_cases() -> [TestCase; 3] {
        [
            TestCase {
                path: concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_acb.unity3d"),
                object_count: 2,
                path_id: -0x6924_71bf_cbe7_e9c6,
                class: Class::TextAsset,
                name: "song3_00test.acb",
                orig_path: "Assets/imas/resources/adx2/song3/song3_00test.acb.bytes",
            },
            TestCase {
                path: concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_image.unity3d"),
                object_count: 14,
                path_id: 0x5c6b_8a21_bcbc_972e,
                class: Class::Texture2D,
                name: "014mir0011_0",
                orig_path: "Assets/imas/resources/card/014mir0011_0.png",
            },
            TestCase {
                path: concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_text.unity3d"),
                object_count: 2,
                path_id: 0x7aa6_cd6c_f238_30a9,
                class: Class::TextAsset,
                name: "card_blst_014mir0034_zh.gtx",
                orig_path: "Assets/imas/resources/textassets/zh/commu/card/card_blst_014mir0034_zh.gtx.bytes",
            },
        ]
    }

    #[test]
    fn test_load_objects() {
        init_test_logger();

        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56789).unwrap();

        // a bundle is unloaded when dropped, so the next file can be loaded
        for case in test_cases() {
            let bundle = asset_ripper.load(case.path.as_ref()).unwrap();

            assert_eq!(bundle.objects().len(), case.object_count);
            assert!(bundle.objects().iter().any(|o| case.matches(o)));
        }
    }

    #[test]
    fn test_text() {
        init_test_logger();

        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56790).unwrap();
        let case = &test_cases()[0];
        let bundle = asset_ripper.load(case.path.as_ref()).unwrap();
        let object = bundle.objects().iter().find(|o| case.matches(o)).unwrap();

        let expected = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_acb.acb"));
        assert_eq!(bundle.text(object).unwrap(), expected.unwrap());
    }

    #[test]
    fn test_texture_and_sprite() {
        init_test_logger();

        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56791).unwrap();
        let case = &test_cases()[1];
        let bundle = asset_ripper.load(case.path.as_ref()).unwrap();
        let texture = bundle.objects().iter().find(|o| case.matches(o)).unwrap();

        let actual = bundle.texture(texture).unwrap();
        let expected =
            image::open(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_image.png")).unwrap();
        assert_eq!(actual.to_rgba8(), expected.to_rgba8());

        let object = bundle.objects().iter().find(|o| o.name == "014mir0011_0_6").unwrap();
        let sprite = bundle.sprite(object).unwrap();
        assert_eq!(sprite.texture_id, texture.path_id);
        assert_eq!(sprite.rect.x, 1);
        assert_eq!(sprite.rect.width, 640);
    }

    #[test]
    fn test_wrong_class() {
        init_test_logger();

        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56792).unwrap();
        let case = &test_cases()[1];
        let bundle = asset_ripper.load(case.path.as_ref()).unwrap();
        let sprite = bundle.objects().iter().find(|o| o.class == Class::Sprite).unwrap();

        assert!(bundle.text(sprite).is_err());
        assert!(bundle.texture(sprite).is_err());
    }

    #[test]
    fn test_serialized() {
        init_test_logger();

        let mut asset_ripper = AssetRipper::new(ASSET_RIPPER_PATH, 56793).unwrap();

        // skipping acb and text assets because they are too large
        let case = &test_cases()[1];
        let bundle = asset_ripper.load(case.path.as_ref()).unwrap();
        let object = bundle.objects().iter().find(|o| case.matches(o)).unwrap();

        let expected: serde_json::Value = serde_json::from_reader(
            File::open(Path::new(case.path).with_extension("json")).unwrap(),
        )
        .unwrap();

        assert_eq!(bundle.serialized(object).unwrap(), expected);
    }
}
