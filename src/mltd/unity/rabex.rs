//! Extractor backed by the pure Rust [`rabex`] crate.

use std::fmt::{Debug, Formatter};
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use ahash::HashMapExt;
use image::DynamicImage;
use rabex::files::SerializedFile;
use rabex::files::bundlefile::{BundleFileReader, ExtractionConfig};
use rabex::files::unityfile::FileEntry;
use rabex::objects::ClassId;
use rabex::tpk::TpkTypeTreeBlob;
use rabex::typetree::typetree_cache::sync::TypeTreeCache;
use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::float::Float;
use super::texture::{Bytes, Int, decode_texture};
use super::{Bundle, Class, Error as UnityError, Extractor, Object, Rect, Sprite};
use crate::error::{Repr, Result};
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
#[allow(non_snake_case, clippy::struct_field_names)]
struct AssetBundleData {
    #[serde(default)]
    m_Name: String,
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

#[derive(Deserialize)]
struct RectData {
    x: Float,
    y: Float,
    width: Float,
    height: Float,
}

#[derive(Deserialize)]
struct SpriteRenderData {
    texture: PPtrData,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct SpriteData {
    m_Rect: RectData,
    m_RD: SpriteRenderData,
}

/// Reads the asset bundle object of a serialized file, if it has one.
fn read_asset_bundle(
    type_trees: &TypeTrees,
    file: &SerializedFile,
    data: &[u8],
) -> Option<AssetBundleData> {
    let info = file.objects().find(|o| o.m_ClassID == ClassId::AssetBundle)?;

    match file.read(info, type_trees, &mut Cursor::new(data)) {
        Ok(bundle) => Some(bundle),
        Err(e) => {
            tracing::warn!(error = ?e, "failed to read asset bundle container");
            None
        }
    }
}

/// Maps path ids to the container (original) path of the asset bundle.
fn container_paths(bundle: &AssetBundleData) -> HashMap<i64, String> {
    let mut paths = HashMap::new();

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

/// Lists the objects of the serialized file `collection`.
fn list_objects(
    type_trees: &TypeTrees,
    collection: usize,
    file: &SerializedFile,
    data: &[u8],
    asset_bundle: Option<&AssetBundleData>,
) -> Vec<Object> {
    let container = asset_bundle.map(container_paths).unwrap_or_default();

    let mut objects = Vec::new();
    for object in file.objects() {
        let class = object.m_ClassID.name().unwrap_or_default();
        let name = if object.m_ClassID == ClassId::AssetBundle {
            asset_bundle.map(|b| b.m_Name.clone()).unwrap_or_default()
        } else {
            match file.read::<Named>(object, type_trees, &mut Cursor::new(data)) {
                Ok(named) => named.m_Name,
                Err(e) => {
                    tracing::debug!(error = ?e, path_id = object.m_PathID, class, "cannot read name");
                    String::new()
                }
            }
        };

        let orig_path = if object.m_ClassID == ClassId::AssetBundle {
            None
        } else {
            Some(container.get(&object.m_PathID).cloned().unwrap_or_else(|| {
                let safe_name = sanitize_path(&name);
                match class {
                    // the CLI strips the `.bytes` suffix of text assets
                    "TextAsset" => format!("Assets/{class}/{safe_name}.bytes"),
                    _ => format!("Assets/{class}/{safe_name}"),
                }
            }))
        };

        objects.push(Object::new(
            collection,
            object.m_PathID,
            Class::from_name(class),
            name,
            orig_path,
        ));
    }

    objects
}

/// A serialized file inside a bundle.
struct Collection {
    /// Decompressed bytes of the serialized file.
    data: Vec<u8>,
    file: SerializedFile,
}

/// Extractor that reads bundles with the [`rabex`] crate.
pub struct RabexExtractor {
    type_trees: Arc<TypeTrees>,
}

impl Debug for RabexExtractor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RabexExtractor").finish_non_exhaustive()
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
        Self { type_trees: Arc::new(TypeTreeCache::new(TpkTypeTreeBlob::embedded())) }
    }

    /// Creates `count` extractors sharing one type tree database.
    #[must_use]
    pub fn pool(count: usize) -> Vec<Self> {
        let type_trees = Arc::new(TypeTreeCache::new(TpkTypeTreeBlob::embedded()));
        (0..count).map(|_| Self { type_trees: Arc::clone(&type_trees) }).collect()
    }
}

impl Extractor for RabexExtractor {
    type Bundle<'a> = RabexBundle<'a>;

    fn load(&mut self, path: &Path) -> Result<RabexBundle<'_>> {
        RabexBundle::open(&self.type_trees, path)
    }
}

/// A bundle loaded by [`RabexExtractor`].
pub struct RabexBundle<'a> {
    type_trees: &'a TypeTrees,
    reader: BundleFileReader<Cursor<Vec<u8>>>,
    collections: Vec<Collection>,
    objects: Vec<Object>,
}

impl<'a> RabexBundle<'a> {
    fn open(type_trees: &'a TypeTrees, path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path).map_err(|e| Repr::io("read bundle", Some(path), e))?;
        // rabex panics on the other signatures (UnityWeb, UnityRaw, ...) instead of returning an error
        if !bytes.starts_with(b"UnityFS\0") {
            return Err(UnityError::NotUnityFs.into());
        }
        let len = bytes.len() as u64;
        let config = ExtractionConfig::default().assume_recent_unity();
        let reader = BundleFileReader::from_reader(Cursor::new(bytes), &config)
            .map_err(|e| UnityError::corrupted("parse bundle", e))?;

        // resources are decompressed on demand, so a truncated bundle has to be rejected here
        let compressed: u64 = reader.blocks().iter().map(|b| u64::from(b.compressed_size)).sum();
        if reader.reader.position().saturating_add(compressed) > len {
            return Err(UnityError::Truncated("bundle").into());
        }

        let mut collections = Vec::new();
        let mut objects = Vec::new();
        for entry in reader.serialized_files() {
            let data = reader
                .read_at_entry(entry)
                .map_err(|e| Repr::io("decompress bundle file", Some(path), e))?;

            let file = SerializedFile::from_reader(&mut Cursor::new(&data))
                .map_err(|e| UnityError::corrupted("parse serialized file", e))?;
            let asset_bundle = read_asset_bundle(type_trees, &file, &data);

            objects.extend(list_objects(
                type_trees,
                collections.len(),
                &file,
                &data,
                asset_bundle.as_ref(),
            ));
            collections.push(Collection { data, file });
        }

        Ok(Self { type_trees, reader, collections, objects })
    }

    /// Reads and deserializes an object.
    fn read<T: DeserializeOwned>(&self, object: &Object, what: &'static str) -> Result<T> {
        let not_found = || UnityError::ObjectNotFound(object.path_id);
        let collection = self.collections.get(object.collection).ok_or_else(not_found)?;
        let info = collection.file.get_object_info(object.path_id).ok_or_else(not_found)?;

        collection
            .file
            .read(info, self.type_trees, &mut Cursor::new(&collection.data))
            .map_err(|e| UnityError::corrupted(what, e).into())
    }

    /// Makes sure that the object is of the `expected` class.
    fn check_class(object: &Object, expected: &Class, as_: &'static str) -> Result<()> {
        if object.class == *expected {
            return Ok(());
        }

        Err(UnityError::UnsupportedClass(object.class.to_string(), as_).into())
    }

    /// Reads `size` bytes at `offset` of a non-serialized file, decompressing only the blocks
    /// covering them.
    fn read_resource(&self, name: &str, offset: i64, size: i64) -> Result<Vec<u8>> {
        let entry =
            self.reader.file(name).ok_or_else(|| UnityError::ResourceNotFound(name.to_owned()))?;

        if entry.flags & FileEntry::FLAG_SERIALIZEDFILE != 0 {
            return Err(UnityError::NotAResource(name.to_owned()).into());
        }

        let out_of_range = || UnityError::ResourceOutOfRange;
        if offset < 0 || size < 0 || offset.checked_add(size).is_none_or(|end| end > entry.size) {
            return Err(out_of_range().into());
        }
        let start = entry.offset.checked_add(offset).ok_or_else(out_of_range)?;
        let range = FileEntry { offset: start, size, flags: entry.flags, path: entry.path.clone() };

        let data = self
            .reader
            .read_at_entry(&range)
            .map_err(|e| Repr::io("decompress bundle file", None, e))?;
        if usize::try_from(size).ok() != Some(data.len()) {
            return Err(UnityError::Truncated("streaming data").into());
        }

        Ok(data)
    }
}

impl Bundle for RabexBundle<'_> {
    fn objects(&self) -> &[Object] {
        &self.objects
    }

    fn text(&self, object: &Object) -> Result<Vec<u8>> {
        Self::check_class(object, &Class::TextAsset, "text")?;

        let text: TextAssetData = self.read(object, "read TextAsset")?;

        Ok(text.m_Script.0)
    }

    fn texture(&self, object: &Object) -> Result<DynamicImage> {
        Self::check_class(object, &Class::Texture2D, "texture")?;

        let texture: Texture2DData = self.read(object, "read Texture2D")?;

        let stream = &texture.m_StreamData;
        let streamed;
        let data = if stream.size.0 > 0 {
            let name = stream.path.rsplit('/').next().unwrap_or_default();
            streamed = self.read_resource(name, stream.offset.0, stream.size.0)?;
            &streamed
        } else {
            &texture.image_data.0
        };

        let dimension =
            |v: Int| u32::try_from(v.0).map_err(|e| UnityError::corrupted("read texture size", e));
        let mut image = decode_texture(
            i32::try_from(texture.m_TextureFormat.0)
                .map_err(|e| UnityError::corrupted("read texture format", e))?,
            dimension(texture.m_Width)?,
            dimension(texture.m_Height)?,
            data,
        )
        .map_err(|e| UnityError::corrupted("decode Texture2D", e))?;

        // Unity textures are bottom-up
        image::imageops::flip_vertical_in_place(&mut image);

        Ok(DynamicImage::ImageRgba8(image))
    }

    fn sprite(&self, object: &Object) -> Result<Sprite> {
        Self::check_class(object, &Class::Sprite, "sprite")?;

        let sprite: SpriteData = self.read(object, "read Sprite")?;
        let rect = sprite.m_Rect;

        Ok(Sprite {
            texture_id: sprite.m_RD.texture.m_PathID,
            rect: Rect {
                x: rect.x.to_u32()?,
                y: rect.y.to_u32()?,
                width: rect.width.to_u32()?,
                height: rect.height.to_u32()?,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use rabex::files::unityfile::FileEntry;

    use super::{RabexBundle, RabexExtractor};
    use crate::unity::{Bundle, Class, Extractor, Object};
    use crate::util::init_test_logger;

    struct TestCase {
        path: &'static str,
        object_count: usize,
        object: Object,
    }

    fn test_cases() -> [TestCase; 3] {
        let object = |path_id, class, name: &str, orig_path: &str| {
            Object::new(0, path_id, class, name.to_owned(), Some(orig_path.to_owned()))
        };

        [
            TestCase {
                path: concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_acb.unity3d"),
                object_count: 2,
                object: object(
                    -0x6924_71bf_cbe7_e9c6,
                    Class::TextAsset,
                    "song3_00test.acb",
                    "Assets/imas/resources/adx2/song3/song3_00test.acb.bytes",
                ),
            },
            TestCase {
                path: concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_image.unity3d"),
                object_count: 14,
                object: object(
                    0x5c6b_8a21_bcbc_972e,
                    Class::Texture2D,
                    "014mir0011_0",
                    "Assets/imas/resources/card/014mir0011_0.png",
                ),
            },
            TestCase {
                path: concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_text.unity3d"),
                object_count: 2,
                object: object(
                    0x7aa6_cd6c_f238_30a9,
                    Class::TextAsset,
                    "card_blst_014mir0034_zh.gtx",
                    "Assets/imas/resources/textassets/zh/commu/card/card_blst_014mir0034_zh.gtx.bytes",
                ),
            },
        ]
    }

    fn texture_bundle(extractor: &mut RabexExtractor) -> (RabexBundle<'_>, Object) {
        let case = &test_cases()[1];
        (extractor.load(case.path.as_ref()).unwrap(), case.object.clone())
    }

    #[test]
    fn test_load_multiple_times() {
        init_test_logger();

        // a bundle is unloaded when dropped, so the extractor can load the same file again
        let mut extractor = RabexExtractor::new();
        for case in test_cases().iter().chain(test_cases().iter()) {
            let bundle = extractor.load(case.path.as_ref()).unwrap();
            assert_eq!(bundle.objects().len(), case.object_count);
        }
    }

    #[test]
    fn test_load_truncated() {
        init_test_logger();

        let bytes = std::fs::read(test_cases()[1].path).unwrap();
        let path = std::env::temp_dir().join("mltd_test_truncated.unity3d");

        // inside the streaming resource, which is not decompressed when loading
        for len in [100_000, bytes.len() - 1] {
            std::fs::write(&path, &bytes[..len]).unwrap();

            assert!(RabexExtractor::new().load(&path).is_err());
        }
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_load_not_unityfs() {
        init_test_logger();

        let mut bytes = std::fs::read(test_cases()[1].path).unwrap();
        bytes[..8].copy_from_slice(b"UnityWeb");
        let path = std::env::temp_dir().join("mltd_test_not_unityfs.unity3d");
        std::fs::write(&path, bytes).unwrap();

        let mut extractor = RabexExtractor::new();
        let result = extractor.load(&path).is_err();
        std::fs::remove_file(&path).unwrap();

        assert!(result);
    }

    #[test]
    fn test_load_missing() {
        init_test_logger();

        assert!(RabexExtractor::new().load("tests/does_not_exist.unity3d".as_ref()).is_err());
    }

    #[test]
    fn test_texture_corrupted() {
        init_test_logger();

        let case = &test_cases()[1];
        let mut bytes = std::fs::read(case.path).unwrap();
        // corrupts the streamed texture data, the decoder panics on it (also in release builds)
        bytes[40080] ^= 0xff;
        let path = std::env::temp_dir().join("mltd_test_corrupted_texture.unity3d");
        std::fs::write(&path, bytes).unwrap();

        let mut extractor = RabexExtractor::new();
        let bundle = extractor.load(&path);
        std::fs::remove_file(&path).unwrap();

        assert!(bundle.unwrap().texture(&case.object).is_err());
    }

    #[test]
    fn test_sanitize_path() {
        assert_eq!(super::original_path("assets/a/b.png"), "Assets/a/b.png");
        assert_eq!(super::original_path("assets/../../x"), "Assets/x");
        assert_eq!(super::sanitize_path("C:\\x/./y?z"), "C_/x/y_z");
    }

    #[test]
    fn test_objects() {
        init_test_logger();

        let mut extractor = RabexExtractor::new();
        for case in test_cases() {
            let bundle = extractor.load(case.path.as_ref()).unwrap();

            assert!(bundle.objects().contains(&case.object));
        }
    }

    #[test]
    fn test_text() {
        init_test_logger();

        let mut extractor = RabexExtractor::new();
        let cases = test_cases();
        let expected = [
            (0, concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_acb.acb")),
            (2, concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_text.gtx")),
        ];

        for (i, path) in expected {
            let bundle = extractor.load(cases[i].path.as_ref()).unwrap();

            assert_eq!(bundle.text(&cases[i].object).unwrap(), std::fs::read(path).unwrap());
        }
    }

    #[test]
    fn test_texture() {
        init_test_logger();

        let mut extractor = RabexExtractor::new();
        let (bundle, object) = texture_bundle(&mut extractor);

        let actual = bundle.texture(&object).unwrap();
        let expected =
            image::open(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_image.png")).unwrap();

        assert_eq!(actual.to_rgba8(), expected.to_rgba8());
    }

    #[test]
    fn test_sprite() {
        init_test_logger();

        let mut extractor = RabexExtractor::new();
        let (bundle, texture) = texture_bundle(&mut extractor);

        let object = bundle.objects().iter().find(|o| o.name == "014mir0011_0_6").unwrap();
        let sprite = bundle.sprite(object).unwrap();

        assert_eq!(sprite.texture_id, texture.path_id);
        assert_eq!(sprite.rect.x, 1);
        assert_eq!(sprite.rect.width, 640);
    }

    #[test]
    fn test_wrong_class() {
        init_test_logger();

        let mut extractor = RabexExtractor::new();
        let (bundle, texture) = texture_bundle(&mut extractor);
        let sprite = bundle.objects().iter().find(|o| o.class == Class::Sprite).unwrap();

        assert!(bundle.text(sprite).is_err());
        assert!(bundle.texture(sprite).is_err());
        assert!(bundle.sprite(&texture).is_err());
    }

    #[test]
    fn test_foreign_object() {
        init_test_logger();

        let mut extractor = RabexExtractor::new();
        let (bundle, texture) = texture_bundle(&mut extractor);

        let mut foreign = texture.clone();
        foreign.collection = 99;
        assert!(bundle.texture(&foreign).is_err());

        foreign.collection = 0;
        foreign.path_id += 1;
        assert!(bundle.texture(&foreign).is_err());
    }

    #[test]
    fn test_read_resource() {
        init_test_logger();

        let mut extractor = RabexExtractor::new();
        let (bundle, _) = texture_bundle(&mut extractor);

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
}
