//! Writes text assets.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::error::{Repr, Result};
use crate::extract::text::decrypt_text;
use crate::extract::{AUDIO_EXTENSIONS, ENCRYPTED_TEXT_EXTENSIONS};
use crate::unity::{Bundle, Object};

/// Writes a [`Class::TextAsset`](crate::unity::Class::TextAsset) to `output_path`,
/// removes `.bytes` extension, and decrypts it if it's encrypted.
///
/// If `decrypt` is `false`, encrypted text is written as it is, and keeps the `.bytes` extension
/// so that it can be told apart from the decrypted one by [`decrypt_file`].
///
/// Returns the output path if it's an audio file.
///
/// [`decrypt_file`]: crate::extract::text::decrypt_file
pub(super) fn extract(
    bundle: &impl Bundle,
    object: &Object,
    output_path: &Path,
    decrypt: bool,
) -> Result<Option<PathBuf>> {
    // remove .bytes
    let stripped_path = output_path.with_extension("");
    let file_ext = stripped_path.extension().and_then(OsStr::to_str).unwrap_or_default();

    let mut data = bundle.text(object)?;
    let mut audio_path = None;

    let output_path = if !ENCRYPTED_TEXT_EXTENSIONS.contains(&file_ext) {
        if AUDIO_EXTENSIONS.contains(&file_ext) {
            audio_path = Some(stripped_path.clone());
        }

        tracing::info!(output_path = %stripped_path.display(), "extracting text");
        stripped_path
    } else if decrypt {
        tracing::info!(output_path = %stripped_path.display(), "decrypting text");
        data = decrypt_text(&data)?;
        tracing::trace!(decrypted = %String::from_utf8_lossy(&data));
        stripped_path
    } else {
        tracing::info!(output_path = %output_path.display(), "extracting encrypted text");
        output_path.to_path_buf()
    };

    std::fs::write(&output_path, data)
        .map_err(|e| Repr::io("write text", Some(&output_path), e))?;

    Ok(audio_path)
}
