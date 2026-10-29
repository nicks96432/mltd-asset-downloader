//! Unpacks `TextAssets`.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::decode::{AUDIO_EXTENSIONS, ENCRYPTED_TEXT_EXTENSIONS};
use crate::error::{Repr, Result};
use crate::unity::{Bundle, Object};

/// Writes a [`Class::TextAsset`](crate::unity::Class::TextAsset) to `output_path`,
/// removes `.bytes` extension, and decrypts it if it's encrypted and `decrypt` is `true`.
///
/// Encrypted text that is not decrypted is written as it is, and keeps the `.bytes` extension
/// so that it can be told apart from the decrypted one.
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
    let file_ext = stripped_path.extension().and_then(OsStr::to_str).unwrap_or_default();

    let data = bundle.text(object)?;
    let mut audio_path = None;

    let (output_path, data) = if !ENCRYPTED_TEXT_EXTENSIONS.contains(&file_ext) {
        if AUDIO_EXTENSIONS.contains(&file_ext) {
            audio_path = Some(stripped_path.clone());
        }

        tracing::info!(output_path = %stripped_path.display(), "unpacking text");
        (stripped_path, data)
    } else if decrypt {
        tracing::info!(output_path = %stripped_path.display(), "decrypting text");
        (stripped_path, decrypt_data(&data)?)
    } else {
        tracing::info!(output_path = %output_path.display(), "unpacking encrypted text");
        (output_path.to_path_buf(), data)
    };

    std::fs::write(&output_path, data)
        .map_err(|e| Repr::io("write text", Some(&output_path), e))?;

    Ok(audio_path)
}

#[cfg(feature = "decode")]
fn decrypt_data(data: &[u8]) -> Result<Vec<u8>> {
    let decrypted = crate::decode::text::decrypt_text(data)?;
    tracing::trace!(decrypted = %String::from_utf8_lossy(&decrypted));

    Ok(decrypted)
}

/// Text is never decrypted without the `decode` feature, so this is never called.
#[cfg(not(feature = "decode"))]
#[allow(clippy::unnecessary_wraps)]
fn decrypt_data(data: &[u8]) -> Result<Vec<u8>> {
    Ok(data.to_vec())
}
