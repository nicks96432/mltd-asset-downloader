//! Decrypts unpacked assets.

#[cfg(feature = "decode-audio")]
pub mod audio;

#[cfg(feature = "decrypt-text")]
pub mod text;

use std::ffi::OsStr;
use std::path::Path;

/// File extensions for audio assets in MLTD, which are decoded after all the files are unpacked.
pub const AUDIO_EXTENSIONS: &[&str; 2] = &["acb", "awb"];

/// File extensions for encrypted text assets in MLTD.
pub const ENCRYPTED_TEXT_EXTENSIONS: &[&str; 2] = &["gtx", "mld"];

/// Kinds of assets that can be decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    /// An audio file, see [`AUDIO_EXTENSIONS`].
    Audio,

    /// An encrypted text file, see [`ENCRYPTED_TEXT_EXTENSIONS`].
    EncryptedText,
}

/// Returns the extension of `path` if it is valid UTF-8.
fn extension(path: &Path) -> &str {
    path.extension().and_then(OsStr::to_str).unwrap_or_default()
}

/// Tells the kind of the asset at `path` from its extension.
///
/// The `.bytes` extension that Unity adds to text assets is ignored, so `x.gtx.bytes` is
/// encrypted text and `x.acb.bytes` is audio.
///
/// # Example
///
/// ```
/// use std::path::Path;
///
/// use mltd::decode::{AssetKind, asset_kind};
///
/// assert_eq!(asset_kind(Path::new("bgm.acb")), Some(AssetKind::Audio));
/// assert_eq!(asset_kind(Path::new("bgm.awb.bytes")), Some(AssetKind::Audio));
/// assert_eq!(asset_kind(Path::new("commu.gtx.bytes")), Some(AssetKind::EncryptedText));
/// assert_eq!(asset_kind(Path::new("card.png")), None);
/// ```
#[must_use]
pub fn asset_kind(path: &Path) -> Option<AssetKind> {
    let ext = match extension(path) {
        "bytes" => path.file_stem().map(Path::new).map(extension).unwrap_or_default(),
        ext => ext,
    };

    if AUDIO_EXTENSIONS.contains(&ext) {
        Some(AssetKind::Audio)
    } else if ENCRYPTED_TEXT_EXTENSIONS.contains(&ext) {
        Some(AssetKind::EncryptedText)
    } else {
        None
    }
}
