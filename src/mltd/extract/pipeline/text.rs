//! Writes text assets.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::error::{Repr, Result};
use crate::extract::ENCRYPTED_TEXT_EXTENSIONS;
use crate::extract::text::decrypt_text;
use crate::unity::{Bundle, Object};

/// Extensions of the audio files, which are decoded after all the files are extracted.
const AUDIO_EXTENSIONS: &[&str] = &["acb", "awb"];

/// Writes a [`Class::TextAsset`](crate::unity::Class::TextAsset) to `output_path`,
/// removes `.bytes` extension, and decrypts it if it's encrypted.
///
/// Returns the output path if it's an audio file.
pub(super) fn extract(
    bundle: &impl Bundle,
    object: &Object,
    output_path: &Path,
) -> Result<Option<PathBuf>> {
    // remove .bytes
    let output_path = output_path.with_extension("");
    let file_ext = output_path.extension().and_then(OsStr::to_str).unwrap_or_default();

    let mut data = bundle.text(object)?;
    let mut audio_path = None;

    if ENCRYPTED_TEXT_EXTENSIONS.contains(&file_ext) {
        tracing::info!(output_path = %output_path.display(), "decrypting text");
        data = decrypt_text(&data)?;
        tracing::trace!(decrypted = %String::from_utf8_lossy(&data));
    } else {
        if AUDIO_EXTENSIONS.contains(&file_ext) {
            audio_path = Some(output_path.clone());
        }

        tracing::info!(output_path = %output_path.display(), "extracting text");
    }

    std::fs::write(&output_path, data)
        .map_err(|e| Repr::io("write text", Some(&output_path), e))?;

    Ok(audio_path)
}
