//! Unpacks `TextAssets`.

use std::path::{Path, PathBuf};

use crate::decode::{AssetKind, asset_kind};
use crate::error::{Repr, Result};
use crate::unity::{Bundle, Object};

/// Writes a [`Class::TextAsset`](crate::unity::Class::TextAsset) to `output_path`,
/// removes `.bytes` extension, and decrypts it if it's encrypted and `decrypt` is `true`.
///
/// Encrypted text that is not decrypted is written as it is, and keeps the `.bytes` extension.
///
/// Returns the output path if it's an audio file.
pub(super) fn extract(
    bundle: &impl Bundle,
    object: &Object,
    output_path: &Path,
    decrypt: bool,
) -> Result<Option<PathBuf>> {
    // remove .bytes
    let stripped_path = output_path.with_extension("");
    let kind = asset_kind(&stripped_path);

    let data = bundle.text(object)?;
    let mut audio_path = None;

    let (output_path, data) = match kind {
        Some(AssetKind::EncryptedText) if decrypt => {
            tracing::info!(output_path = %stripped_path.display(), "decrypting text");
            (stripped_path, decrypt_data(&data)?)
        }
        Some(AssetKind::EncryptedText) => {
            tracing::info!(output_path = %output_path.display(), "unpacking encrypted text");
            (output_path.to_path_buf(), data)
        }
        Some(AssetKind::Audio) | None => {
            if kind == Some(AssetKind::Audio) {
                tracing::debug!(path = %stripped_path.display(), "found audio file to decode later");
                audio_path = Some(stripped_path.clone());
            }

            tracing::info!(output_path = %stripped_path.display(), "unpacking text");
            (stripped_path, data)
        }
    };

    std::fs::write(&output_path, data)
        .map_err(|e| Repr::io("write text", Some(&output_path), e))?;

    Ok(audio_path)
}

#[cfg(feature = "decrypt-text")]
fn decrypt_data(data: &[u8]) -> Result<Vec<u8>> {
    let decrypted = crate::decode::text::decrypt_text(data)?;
    tracing::trace!(decrypted = %String::from_utf8_lossy(&decrypted));

    Ok(decrypted)
}

#[cfg(not(feature = "decrypt-text"))]
#[allow(clippy::unnecessary_wraps)]
fn decrypt_data(data: &[u8]) -> Result<Vec<u8>> {
    Ok(data.to_vec())
}
