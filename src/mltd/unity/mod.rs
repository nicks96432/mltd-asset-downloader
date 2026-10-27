#[cfg(feature = "asset-ripper")]
pub mod asset_ripper;

use std::borrow::Cow;
use std::fs::File;
use std::path::Path;

#[cfg(feature = "asset-ripper")]
pub use asset_ripper::AssetRipper;

use crate::error::Result;

/// Information about an unity asset.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
// prevent user from constructing this struct directly
#[non_exhaustive]
pub struct AssetInfo {
    /// Path ID of the asset.
    pub path_id: i64,

    /// Type of the asset, e.g. `Texture2D`, `MonoBehaviour`.
    pub class: Cow<'static, str>,

    /// Name of the asset.
    pub name: Cow<'static, str>,

    /// Original path of the asset before bundled.
    pub orig_path: Option<Cow<'static, str>>,

    /// Name of the original bundle file the asset belongs.
    pub orig_bundle: Cow<'static, str>,

    /// Name of the collection.
    pub collection: Cow<'static, str>,
}

/// A trait for extracting assets from Unity asset bundles.
pub trait Extractor {
    /// Loads an Unity asset bundle file from the specified path.
    ///
    /// # Errors
    ///
    /// This method fails if the path does not exist or is not a valid Unity
    /// asset bundle.
    fn load_file(&mut self, path: &Path) -> Result<()>;

    /// Gets a list of loaded bundle names.
    fn bundles(&self) -> Vec<&str>;

    /// Gets a list of collections in the specified bundle.
    ///
    /// Returns [`None`] if the bundle does not exist.
    fn collections<'a>(&'a self, bundle: &str) -> Option<Vec<&'a str>>;

    /// Gets a list of asset infos in the specified bundle and collection.
    ///
    /// Returns [`None`] if the bundle or collection does not exist.
    fn asset_infos<'a>(&'a self, bundle: &str, collection: &str) -> Option<Vec<&'a AssetInfo>>;

    /// Gets information about a specific asset by its path ID in the specified
    /// bundle and collection.
    ///
    /// Returns [`None`] if the bundle or collection does not exist, or if the
    /// asset with the given path ID is not found.
    fn asset_info<'a>(
        &'a self,
        bundle: &str,
        collection: &str,
        path_id: i64,
    ) -> Option<&'a AssetInfo>;

    /// Gets JSON serialized representation of the asset. See [Unity documentation][doc]
    /// for more details.
    ///
    /// Although the original serialization format is YAML, JSON is used here
    /// for better compatibility with Rust ecosystem.
    ///
    /// # Errors
    ///
    /// This method fails if the asset is not found in the extractor, or the asset
    /// cannot be serialized.
    ///
    /// [doc]: https://docs.unity3d.com/6000.2/Documentation/Manual/FormatDescription.html
    fn asset_serialized<'a>(&'a self, info: &'a AssetInfo) -> Result<serde_json::Value>;

    /// Gets all asset infos across all loaded bundles and collections.
    fn all_asset_infos(&self) -> Vec<&AssetInfo> {
        let mut assets = vec![];

        for bundle in self.bundles() {
            for collection in self.collections(bundle).unwrap_or_default() {
                assets.extend(self.asset_infos(bundle, collection).unwrap_or_default());
            }
        }

        assets
    }

    /// Extracts the raw bytes of the specified asset.
    ///
    /// # Errors
    ///
    /// This method fails if info is not found in the extractor, or the file
    /// cannot be accessed or extracted.
    fn extract(&self, info: &AssetInfo) -> Result<File>;

    /// Clears all loaded bundles and cached data.
    fn clear(&mut self);
}
