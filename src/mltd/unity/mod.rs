//! Reading Unity asset bundles.
//!
//! An [`Extractor`] loads a bundle file into a [`Bundle`], which lists the [`Object`]s in the
//! bundle and reads their contents. A bundle borrows the extractor, so an extractor holds one
//! bundle at a time, and the bundle is unloaded when it is dropped.

#[cfg(feature = "asset-ripper")]
pub mod asset_ripper;
mod error;
mod float;
#[cfg(feature = "rabex")]
pub mod rabex;
#[cfg(feature = "rabex")]
mod texture;

use std::fmt::{Display, Formatter};
use std::path::Path;

#[cfg(feature = "asset-ripper")]
pub use asset_ripper::AssetRipper;
use image::DynamicImage;
#[cfg(feature = "rabex")]
pub use rabex::RabexExtractor;

pub(crate) use self::error::Error;
use crate::error::Result;

/// Class of an [`Object`].
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[non_exhaustive]
pub enum Class {
    /// Container information of a bundle.
    AssetBundle,

    /// Text or binary data.
    TextAsset,

    /// A texture.
    Texture2D,

    /// A region of a texture.
    Sprite,

    /// Any other class with its name.
    Other(String),
}

impl Class {
    /// Gets the class from the name used by Unity, e.g. `Texture2D`.
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        match name {
            "AssetBundle" => Self::AssetBundle,
            "TextAsset" => Self::TextAsset,
            "Texture2D" => Self::Texture2D,
            "Sprite" => Self::Sprite,
            other => Self::Other(other.to_owned()),
        }
    }

    /// Gets the name used by Unity.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::AssetBundle => "AssetBundle",
            Self::TextAsset => "TextAsset",
            Self::Texture2D => "Texture2D",
            Self::Sprite => "Sprite",
            Self::Other(name) => name,
        }
    }
}

impl Display for Class {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// An object (asset) in a [`Bundle`].
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
// prevent user from constructing this struct directly
#[non_exhaustive]
pub struct Object {
    /// Path ID of the object, which is unique in a serialized file.
    pub path_id: i64,

    /// Class of the object.
    pub class: Class,

    /// Name of the object.
    pub name: String,

    /// Original path of the object before bundled.
    pub orig_path: Option<String>,

    /// Index of the serialized file the object belongs to, which is meaningful to the bundle only.
    pub(crate) collection: usize,
}

impl Object {
    pub(crate) fn new(
        collection: usize,
        path_id: i64,
        class: Class,
        name: String,
        orig_path: Option<String>,
    ) -> Self {
        Self { path_id, class, name, orig_path, collection }
    }
}

/// A rectangle in a texture, whose origin is the bottom left corner.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// A [`Class::Sprite`], which is a region of a [`Class::Texture2D`] in the same serialized file.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Sprite {
    /// Path ID of the texture.
    pub texture_id: i64,

    /// Region of the sprite in the texture.
    pub rect: Rect,
}

/// A trait for loading Unity asset bundles.
pub trait Extractor {
    /// The loaded bundle, which unloads the file when dropped.
    type Bundle<'a>: Bundle
    where
        Self: 'a;

    /// Loads an Unity asset bundle file from the specified path.
    ///
    /// # Errors
    ///
    /// This method fails if the path does not exist or is not a valid Unity
    /// asset bundle.
    fn load(&mut self, path: &Path) -> Result<Self::Bundle<'_>>;
}

/// Abstraction of a loaded Unity asset bundle.
///
/// The methods that read an object fail if the object is not from this bundle, or if its class
/// is not the one the method reads.
pub trait Bundle {
    /// Gets all objects across all serialized files in the bundle.
    fn objects(&self) -> &[Object];

    /// Reads the content of a [`Class::TextAsset`].
    ///
    /// # Errors
    ///
    /// This method fails if the object cannot be read.
    fn text(&self, object: &Object) -> Result<Vec<u8>>;

    /// Reads the image of a [`Class::Texture2D`].
    ///
    /// The image is upright, i.e. the same as the one exported by the Unity editor.
    ///
    /// # Errors
    ///
    /// This method fails if the object cannot be read or the texture cannot be decoded.
    fn texture(&self, object: &Object) -> Result<DynamicImage>;

    /// Reads the texture reference and the region of a [`Class::Sprite`].
    ///
    /// # Errors
    ///
    /// This method fails if the object cannot be read.
    fn sprite(&self, object: &Object) -> Result<Sprite>;
}
