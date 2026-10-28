//! Extractor backed by the pure Rust [`rabex`](rabex) crate.

use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::{Cursor, Seek};
use std::path::Path;
use std::sync::Arc;

use ahash::HashMapExt;
use image::ImageFormat;
use rabex::files::SerializedFile;
use rabex::files::bundlefile::{BundleFileReader, ExtractionConfig};
use rabex::files::unityfile::FileEntry;
use rabex::objects::ClassId;
use rabex::tpk::TpkTypeTreeBlob;
use rabex::typetree::typetree_cache::sync::TypeTreeCache;
use serde::Deserialize;

use super::texture::{Bytes, Int, decode_texture};
use super::{AssetInfo, Extractor};
use crate::error::{Error, Repr, Result};
use crate::util::HashMap;

type TypeTrees = TypeTreeCache<TpkTypeTreeBlob>;

/// Makes a path read from a bundle safe to join with the output directory.
fn sanitize_path(path: &str) -> String {
    path.split(['/', '\\'])
        .filter(|part| !matches!(*part, "" | "." | ".."))
        .map(|part| part.replace(|c: char| c.is_control() || "<>:\"|?*".contains(c), "_"))
        .collect::<Vec<_>>()
        .join("/")
}

fn original_path(container_path: &str) -> String {
    let path = sanitize_path(container_path);
    match path.strip_prefix("assets/") {
        Some(rest) => format!("Assets/{rest}"),
        None => path,
    }
}

fn io_error(reason: &str, source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Error {
    Repr::io(reason, Some(std::io::Error::other(source))).into()
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct Named {
    #[serde(default)]
    m_Name: String,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct PPtrData {
    m_FileID: i32,
    m_PathID: i64,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct ContainerEntry {
    asset: PPtrData,
    preloadIndex: i32,
    preloadSize: i32,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct AssetBundleData {
    m_PreloadTable: Vec<PPtrData>,
    m_Container: Vec<(String, ContainerEntry)>,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct TextAssetData {
    m_Script: Bytes,
}

#[derive(Default, Deserialize)]
struct StreamingInfo {
    #[serde(default)]
    offset: Int,
    #[serde(default)]
    size: Int,
    #[serde(default)]
    path: String,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct Texture2DData {
    m_Width: Int,
    m_Height: Int,
    m_TextureFormat: Int,
    #[serde(rename = "image data", default)]
    image_data: Bytes,
    #[serde(default)]
    m_StreamData: StreamingInfo,
}

/// A serialized file inside a bundle.
struct Collection {
    /// Decompressed bytes of the serialized file.
    data: Vec<u8>,
    file: SerializedFile,
    infos: HashMap<i64, AssetInfo>,
}

/// A loaded bundle file.
struct Bundle {
    reader: BundleFileReader<Cursor<Vec<u8>>>,
    collections: HashMap<String, Collection>,
}

impl Bundle {
    /// Reads `size` bytes at `offset` of a non-serialized file, decompressing only the blocks
    /// covering them.
    fn read_resource(&self, name: &str, offset: i64, size: i64) -> Result<Vec<u8>> {
        let entry = self
            .reader
            .file(name)
            .ok_or_else(|| Repr::io("streaming resource does not exist", None))?;

        if entry.flags & FileEntry::FLAG_SERIALIZEDFILE != 0 {
            return Err(Repr::io("serialized file is not a resource", None).into());
        }

        // the range is checked here, the reader panics or truncates on a range it cannot serve
        let range = (offset >= 0 && size >= 0)
            .then(|| offset.checked_add(size).filter(|end| *end <= entry.size))
            .flatten()
            .and_then(|_| entry.offset.checked_add(offset))
            .map(|offset| FileEntry { offset, size, flags: entry.flags, path: entry.path.clone() })
            .ok_or_else(|| Repr::io("streaming data is out of range", None))?;

        let data = self
            .reader
            .read_at_entry(&range)
            .map_err(|e| Repr::io("failed to decompress bundle file", Some(e)))?;
        if usize::try_from(size).ok() != Some(data.len()) {
            return Err(Repr::io("streaming data is truncated", None).into());
        }

        Ok(data)
    }
}

pub struct RabexExtractor {
    type_trees: Arc<TypeTrees>,
    bundles: HashMap<String, Bundle>,
}

impl Debug for RabexExtractor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RabexExtractor")
            .field("bundles", &self.bundles.keys())
            .finish_non_exhaustive()
    }
}

impl Default for RabexExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl RabexExtractor {
    /// Creates a new extractor with the embedded type tree database.
    #[must_use]
    pub fn new() -> Self {
        Self::with_type_trees(Arc::new(TypeTreeCache::new(TpkTypeTreeBlob::embedded())))
    }

    /// Creates `count` extractors sharing one type tree database.
    #[must_use]
    pub fn pool(count: usize) -> Vec<Self> {
        let type_trees = Arc::new(TypeTreeCache::new(TpkTypeTreeBlob::embedded()));
        (0..count).map(|_| Self::with_type_trees(Arc::clone(&type_trees))).collect()
    }

    fn with_type_trees(type_trees: Arc<TypeTrees>) -> Self {
        Self { type_trees, bundles: HashMap::new() }
    }

    fn locate(&self, info: &AssetInfo) -> Result<(&Bundle, &Collection)> {
        let bundle = self
            .bundles
            .get(info.orig_bundle.as_ref())
            .ok_or_else(|| Repr::io("bundle is not loaded", None))?;
        let collection = bundle
            .collections
            .get(info.collection.as_ref())
            .ok_or_else(|| Repr::io("collection is not loaded", None))?;

        Ok((bundle, collection))
    }

    /// Maps path ids to the container (original) path of the asset bundle.
    fn container_paths(&self, file: &SerializedFile, data: &[u8]) -> HashMap<i64, String> {
        let mut paths = HashMap::new();

        let Some(info) = file.objects().find(|o| o.m_ClassID == ClassId::AssetBundle) else {
            return paths;
        };
        let bundle: AssetBundleData =
            match file.read(info, &*self.type_trees, &mut Cursor::new(data)) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!(error = ?e, "failed to read asset bundle container");
                    return paths;
                }
            };

        // the asset the entry points to takes priority over assets that are only preloaded by it
        for (path, entry) in &bundle.m_Container {
            if entry.asset.m_FileID == 0 {
                paths.entry(entry.asset.m_PathID).or_insert_with(|| original_path(path));
            }
        }
        for (path, entry) in &bundle.m_Container {
            let start = usize::try_from(entry.preloadIndex).unwrap_or(0);
            let len = usize::try_from(entry.preloadSize).unwrap_or(0);
            let range = bundle.m_PreloadTable.iter().skip(start).take(len);
            for ptr in range.filter(|p| p.m_FileID == 0) {
                paths.entry(ptr.m_PathID).or_insert_with(|| original_path(path));
            }
        }

        paths
    }

    /// Gets the name of the asset bundle object of a serialized file, if it has one.
    fn asset_bundle_name(&self, file: &SerializedFile, data: &[u8]) -> Option<String> {
        let object = file.objects().find(|o| o.m_ClassID == ClassId::AssetBundle)?;
        let named: Named = file.read(object, &*self.type_trees, &mut Cursor::new(data)).ok()?;

        match named.m_Name.as_str() {
            "" => None,
            _ => Some(named.m_Name),
        }
    }

    /// Builds the collection of a parsed serialized file in the bundle named `bundle_name`.
    fn load_collection(
        &self,
        bundle_name: &str,
        collection_name: &str,
        file: SerializedFile,
        data: Vec<u8>,
    ) -> Collection {
        let container = self.container_paths(&file, &data);

        let mut infos = HashMap::new();
        for object in file.objects() {
            let class = object.m_ClassID.name().unwrap_or_default();
            let name = match file.read::<Named>(object, &*self.type_trees, &mut Cursor::new(&data))
            {
                Ok(named) => named.m_Name,
                Err(e) => {
                    tracing::debug!(error = ?e, path_id = object.m_PathID, class, "cannot read name");
                    String::new()
                }
            };

            let orig_path = if object.m_ClassID == ClassId::AssetBundle {
                None
            } else {
                Some(Cow::Owned(container.get(&object.m_PathID).cloned().unwrap_or_else(|| {
                    let safe_name = sanitize_path(&name);
                    match class {
                        // the CLI strips the `.bytes` suffix of text assets
                        "TextAsset" => format!("Assets/{class}/{safe_name}.bytes"),
                        _ => format!("Assets/{class}/{safe_name}"),
                    }
                })))
            };

            infos.insert(
                object.m_PathID,
                AssetInfo {
                    path_id: object.m_PathID,
                    class: Cow::Owned(class.to_owned()),
                    orig_path,
                    orig_bundle: Cow::Owned(bundle_name.to_owned()),
                    collection: Cow::Owned(collection_name.to_owned()),
                    name: Cow::Owned(name),
                },
            );
        }

        Collection { data, file, infos }
    }
}

impl Extractor for RabexExtractor {
    fn load_file(&mut self, path: &Path) -> Result<()> {
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .ok_or_else(|| Repr::io("path has no file name", None))?;

        let bytes = std::fs::read(path).map_err(|e| Repr::io("cannot read bundle", Some(e)))?;
        // rabex panics on the other signatures (UnityWeb, UnityRaw, ...) instead of returning an error
        if !bytes.starts_with(b"UnityFS\0") {
            return Err(Repr::io("not a UnityFS bundle", None).into());
        }
        let config = ExtractionConfig::default().assume_recent_unity();
        let reader = BundleFileReader::from_reader(Cursor::new(bytes), &config)
            .map_err(|e| io_error("failed to parse bundle", e))?;

        let mut serialized_files = Vec::new();
        for entry in reader.serialized_files() {
            let data = reader
                .read_at_entry(entry)
                .map_err(|e| Repr::io("failed to decompress bundle file", Some(e)))?;

            let file = SerializedFile::from_reader(&mut Cursor::new(&data))
                .map_err(|e| io_error("failed to parse serialized file", e))?;
            // same naming as AssetRipper: lower-cased collection names
            serialized_files.push((entry.path.to_lowercase(), file, data));
        }

        // the bundle is named by the asset bundle object, like AssetRipper does
        let bundle_name = serialized_files
            .iter()
            .find_map(|(_, file, data)| self.asset_bundle_name(file, data))
            .unwrap_or(file_name);
        if self.bundles.contains_key(&bundle_name) {
            return Err(Repr::io("a bundle with the same name is already loaded", None).into());
        }
        let mut bundle = Bundle { reader, collections: HashMap::new() };
        for (collection_name, file, data) in serialized_files {
            let collection = self.load_collection(&bundle_name, &collection_name, file, data);
            bundle.collections.insert(collection_name, collection);
        }
        self.bundles.insert(bundle_name, bundle);

        Ok(())
    }

    fn bundles(&self) -> Vec<&str> {
        let mut names: Vec<_> = self.bundles.keys().map(String::as_str).collect();
        names.sort_unstable();
        names
    }

    fn collections<'a>(&'a self, bundle: &str) -> Option<Vec<&'a str>> {
        let mut names: Vec<_> =
            self.bundles.get(bundle)?.collections.keys().map(String::as_str).collect();
        names.sort_unstable();
        Some(names)
    }

    fn asset_infos<'a>(&'a self, bundle: &str, collection: &str) -> Option<Vec<&'a AssetInfo>> {
        let mut infos: Vec<_> =
            self.bundles.get(bundle)?.collections.get(collection)?.infos.values().collect();
        infos.sort_unstable_by_key(|info| info.path_id);
        Some(infos)
    }

    fn asset_info<'a>(
        &'a self,
        bundle: &str,
        collection: &str,
        path_id: i64,
    ) -> Option<&'a AssetInfo> {
        self.bundles.get(bundle)?.collections.get(collection)?.infos.get(&path_id)
    }

    fn asset_serialized(&self, info: &AssetInfo) -> Result<serde_json::Value> {
        let (_, collection) = self.locate(info)?;
        let object = collection
            .file
            .get_object_info(info.path_id)
            .ok_or_else(|| Repr::io("asset does not exist", None))?;

        collection
            .file
            .read(object, &*self.type_trees, &mut Cursor::new(&collection.data))
            .map_err(|e| io_error("failed to deserialize asset", e))
    }

    fn extract(&self, info: &AssetInfo) -> Result<File> {
        let (bundle, collection) = self.locate(info)?;
        let object = collection
            .file
            .get_object_info(info.path_id)
            .ok_or_else(|| Repr::io("asset does not exist", None))?;
        let mut reader = Cursor::new(&collection.data);

        let mut out =
            tempfile::tempfile().map_err(|e| Repr::io("cannot create tempfile", Some(e)))?;

        match &*info.class {
            "TextAsset" => {
                let text: TextAssetData = collection
                    .file
                    .read(object, &*self.type_trees, &mut reader)
                    .map_err(|e| io_error("failed to read TextAsset", e))?;

                std::io::Write::write_all(&mut out, &text.m_Script.0)
                    .map_err(|e| Repr::io("cannot write tempfile", Some(e)))?;
            }

            "Texture2D" => {
                let texture: Texture2DData = collection
                    .file
                    .read(object, &*self.type_trees, &mut reader)
                    .map_err(|e| io_error("failed to read Texture2D", e))?;

                let stream = &texture.m_StreamData;
                let streamed;
                let data = if stream.size.0 > 0 {
                    let name = stream.path.rsplit('/').next().unwrap_or_default();
                    streamed = bundle.read_resource(name, stream.offset.0, stream.size.0)?;
                    &streamed
                } else {
                    &texture.image_data.0
                };

                let dimension = |v: Int| {
                    u32::try_from(v.0).map_err(|e| io_error("invalid texture dimension", e))
                };
                let mut image = decode_texture(
                    i32::try_from(texture.m_TextureFormat.0)
                        .map_err(|e| io_error("invalid texture format", e))?,
                    dimension(texture.m_Width)?,
                    dimension(texture.m_Height)?,
                    data,
                )
                .map_err(|e| io_error("failed to decode Texture2D", e))?;

                // Unity textures are bottom-up, flip them like AssetRipper does
                image::imageops::flip_vertical_in_place(&mut image);
                image
                    .write_to(&mut out, ImageFormat::Png)
                    .map_err(|e| io_error("failed to encode png", e))?;
            }

            class => {
                return Err(
                    Repr::io(&format!("cannot extract asset of class {class}"), None).into()
                );
            }
        }

        out.rewind().map_err(|e| Repr::io("cannot rewind tempfile", Some(e)))?;

        Ok(out)
    }

    fn clear(&mut self) {
        self.bundles.clear();
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::io::Read;

    use rabex::files::unityfile::FileEntry;

    use super::RabexExtractor;
    use crate::unity::{AssetInfo, Extractor};
    use crate::util::init_test_logger;

    struct TestCase {
        asset_path: &'static str,
        collection_count: usize,
        asset_count: usize,
        asset_info: AssetInfo,
    }

    fn test_cases() -> [TestCase; 3] {
        [
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
        ]
    }

    fn read_all(mut file: impl Read) -> Vec<u8> {
        let mut buf = vec![];
        file.read_to_end(&mut buf).unwrap();
        buf
    }

    #[test]
    fn test_load_file_bundles_clear() {
        init_test_logger();

        let mut extractor = RabexExtractor::new();
        assert_eq!(extractor.bundles().len(), 0);

        for (i, case) in test_cases().iter().enumerate() {
            extractor.load_file(case.asset_path.as_ref()).unwrap();
            assert_eq!(extractor.bundles().len(), i + 1);
        }

        extractor.clear();
        assert_eq!(extractor.bundles().len(), 0);
    }

    #[test]
    fn test_load_file_not_unityfs() {
        init_test_logger();

        let mut bytes = std::fs::read(test_cases()[1].asset_path).unwrap();
        bytes[..8].copy_from_slice(b"UnityWeb");
        let path = std::env::temp_dir().join("mltd_test_not_unityfs.unity3d");
        std::fs::write(&path, bytes).unwrap();

        let mut extractor = RabexExtractor::new();
        let result = extractor.load_file(&path);
        std::fs::remove_file(&path).unwrap();

        assert!(result.is_err());
        assert_eq!(extractor.bundles().len(), 0);
    }

    #[test]
    fn test_sanitize_path() {
        assert_eq!(super::original_path("assets/a/b.png"), "Assets/a/b.png");
        assert_eq!(super::original_path("assets/../../x"), "Assets/x");
        assert_eq!(super::sanitize_path("C:\\x/./y?z"), "C_/x/y_z");
    }

    #[test]
    fn test_load_file_missing() {
        init_test_logger();

        let mut extractor = RabexExtractor::new();
        assert!(extractor.load_file("tests/does_not_exist.unity3d".as_ref()).is_err());
        assert_eq!(extractor.bundles().len(), 0);
    }

    #[test]
    fn test_collections_and_infos() {
        init_test_logger();

        let mut extractor = RabexExtractor::new();
        for case in test_cases() {
            extractor.load_file(case.asset_path.as_ref()).unwrap();
            let info = &case.asset_info;

            let collections = extractor.collections(&info.orig_bundle).unwrap();
            assert_eq!(collections.len(), case.collection_count);

            let infos = extractor.asset_infos(&info.orig_bundle, &info.collection).unwrap();
            assert_eq!(infos.len(), case.asset_count);

            let found = extractor.asset_info(&info.orig_bundle, &info.collection, info.path_id);
            assert_eq!(found, Some(info));

            extractor.clear();
        }
    }

    #[test]
    fn test_extract_text_asset() {
        init_test_logger();

        let mut extractor = RabexExtractor::new();
        let expected = [
            (0, concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_acb.acb")),
            (2, concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_text.gtx")),
        ];

        let cases = test_cases();
        for (i, path) in expected {
            let info = &cases[i].asset_info;
            extractor.load_file(cases[i].asset_path.as_ref()).unwrap();

            let asset = extractor.asset_info(&info.orig_bundle, &info.collection, info.path_id);
            let buf = read_all(extractor.extract(asset.unwrap()).unwrap());

            assert_eq!(buf, std::fs::read(path).unwrap());
        }
    }

    #[test]
    fn test_extract_texture_2d() {
        init_test_logger();

        let mut extractor = RabexExtractor::new();
        let case = &test_cases()[1];
        let info = &case.asset_info;
        extractor.load_file(case.asset_path.as_ref()).unwrap();

        let asset = extractor.asset_info(&info.orig_bundle, &info.collection, info.path_id);
        let png = read_all(extractor.extract(asset.unwrap()).unwrap());

        let actual = image::load_from_memory_with_format(&png, image::ImageFormat::Png).unwrap();
        let expected =
            image::open(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_image.png")).unwrap();

        assert_eq!(actual.to_rgba8(), expected.to_rgba8());
    }

    #[test]
    fn test_read_resource() {
        init_test_logger();

        let mut extractor = RabexExtractor::new();
        let case = &test_cases()[1];
        extractor.load_file(case.asset_path.as_ref()).unwrap();

        let bundle = &extractor.bundles[case.asset_info.orig_bundle.as_ref()];
        let resource = bundle
            .reader
            .files()
            .iter()
            .find(|entry| entry.flags & FileEntry::FLAG_SERIALIZEDFILE == 0)
            .unwrap();
        let name = resource.path.as_str();
        let all = bundle.reader.read_at(name).unwrap().unwrap();
        assert_eq!(all.len(), usize::try_from(resource.size).unwrap());

        // a range is the same as the matching part of the whole file
        assert_eq!(bundle.read_resource(name, 0, resource.size).unwrap(), all);
        assert_eq!(bundle.read_resource(name, 3, 5).unwrap(), all[3..8]);

        // invalid ranges and names fail instead of panicking
        assert!(bundle.read_resource(name, 0, resource.size + 1).is_err());
        assert!(bundle.read_resource(name, resource.size, 1).is_err());
        assert!(bundle.read_resource(name, -1, 1).is_err());
        assert!(bundle.read_resource(name, 1, i64::MAX).is_err());
        assert!(bundle.read_resource("missing.resS", 0, 1).is_err());
        // serialized files are not resources
        let serialized = &bundle.reader.serialized_files().next().unwrap().path;
        assert!(bundle.read_resource(serialized, 0, 1).is_err());
    }

    #[test]
    fn test_extract_unsupported_class() {
        init_test_logger();

        let mut extractor = RabexExtractor::new();
        let case = &test_cases()[1];
        let info = &case.asset_info;
        extractor.load_file(case.asset_path.as_ref()).unwrap();

        let sprite = extractor
            .asset_infos(&info.orig_bundle, &info.collection)
            .unwrap()
            .into_iter()
            .find(|i| i.class == "Sprite")
            .unwrap();

        assert!(extractor.extract(sprite).is_err());
    }

    #[test]
    fn test_asset_serialized() {
        init_test_logger();

        let mut extractor = RabexExtractor::new();
        let case = &test_cases()[1];
        let info = &case.asset_info;
        extractor.load_file(case.asset_path.as_ref()).unwrap();

        let texture = extractor.asset_info(&info.orig_bundle, &info.collection, info.path_id);
        let value = extractor.asset_serialized(texture.unwrap()).unwrap();
        assert_eq!(value["m_Name"], "014mir0011_0");
        assert_eq!(value["m_Width"], 1024);
        assert_eq!(value["m_Height"], 1024);
        assert_eq!(value["m_TextureFormat"], 54);

        let sprites = extractor.asset_infos(&info.orig_bundle, &info.collection).unwrap();
        let sprite = sprites.into_iter().find(|i| i.name == "014mir0011_0_6").unwrap();
        let value = extractor.asset_serialized(sprite).unwrap();
        assert_eq!(value["m_Rect"]["x"], 1.0);
        assert_eq!(value["m_Rect"]["width"], 640.0);
        assert_eq!(value["m_RD"]["texture"]["m_PathID"], info.path_id);
    }
}
