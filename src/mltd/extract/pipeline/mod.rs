//! Extracts the assets in Unity asset bundles to files.
//!
//! Every [`Class::TextAsset`] and [`Class::Texture2D`] in a bundle is written to the path it had
//! before bundled, under the output directory:
//!
//! - text assets are decrypted if they are encrypted, unless [`Options::decrypt_text`] is `false`
//! - audio assets are decoded after all the files of the bundle are written, unless
//!   [`Options::decode_audio`] is `false`
//! - textures are written as images, with the puzzles solved

mod text;
mod texture;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use image::ImageFormat;

pub use crate::decode::audio::{AudioOptions, KeyFiles, decode as decode_audio};
use crate::error::{Error, Repr, Result};
use crate::unity::{Bundle, Class, Extractor};
use crate::util::safe_join;

/// Options of the extraction.
#[derive(Debug, Clone)]
pub struct Options {
    /// The output directory.
    pub output: PathBuf,

    /// Format of the output images.
    pub image_format: ImageFormat,

    /// Whether to decrypt the encrypted text assets.
    ///
    /// If it is `false`, they are written as they are, and can be decrypted later by
    /// [`decrypt_file`](crate::decode::text::decrypt_file).
    pub decrypt_text: bool,

    /// Whether to decode the audio assets.
    ///
    /// If it is `false`, they are written as they are, and can be decoded later by
    /// [`decode_audio`].
    pub decode_audio: bool,

    /// Options of the audio output.
    pub audio: AudioOptions,
}

impl Options {
    /// Creates options that write to `output` with the default formats.
    #[must_use]
    pub fn new(output: impl Into<PathBuf>) -> Self {
        Self {
            output: output.into(),
            image_format: ImageFormat::Png,
            decrypt_text: true,
            decode_audio: true,
            audio: AudioOptions::default(),
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Extracts the files with the given extractors, each of them handles one file at a time.
///
/// Returns the files that failed to extract, with the reasons. The temporary files made
/// for decoding audio are removed before returning, even if a worker panics.
pub fn extract_all<E>(
    extractors: Vec<E>,
    files: impl Iterator<Item = PathBuf> + Send,
    options: &Options,
) -> Vec<(PathBuf, Error)>
where
    E: Extractor + Send,
{
    let files = Mutex::new(files);
    let failures = Mutex::new(Vec::new());
    // dropped after all the workers have finished decoding
    let key_files = KeyFiles::default();

    std::thread::scope(|scope| {
        for mut extractor in extractors {
            let (files, failures, key_files) = (&files, &failures, &key_files);

            scope.spawn(move || {
                loop {
                    let next = lock(files).next();
                    let Some(path) = next else { break };

                    if let Err(e) = extract_file(&mut extractor, &path, options, key_files) {
                        tracing::warn!(input_path = %path.display(), error = ?e, "failed to extract file");
                        lock(failures).push((path, e));
                    }
                }
            });
        }
    });

    failures.into_inner().unwrap_or_else(PoisonError::into_inner)
}

/// Extracts a single .unity3d file.
///
/// # Errors
///
/// This method fails if the file cannot be loaded, or any of the assets cannot be extracted.
pub fn extract_file<E>(
    extractor: &mut E,
    path: &Path,
    options: &Options,
    key_files: &KeyFiles,
) -> Result<()>
where
    E: Extractor,
{
    tracing::debug!(input_path = %path.display(), "loading file");
    let bundle = extractor.load(path)?;

    let mut audio_paths = BTreeSet::new();

    for object in bundle.objects() {
        match &object.class {
            // asset bundles contains bundle information only, and sprites are handled with
            // their textures
            Class::AssetBundle | Class::Sprite => continue,
            Class::TextAsset | Class::Texture2D => (),
            class => {
                tracing::warn!(%class, "unknown asset class");
                continue;
            }
        }

        let Some(orig_path) = object.orig_path.as_deref() else {
            tracing::warn!(name = object.name, class = %object.class, "asset has no original path");
            continue;
        };

        let output_path = safe_join(&options.output, orig_path)?;
        let output_dir =
            output_path.parent().ok_or_else(|| Repr::UnsafePath(orig_path.to_owned()))?;

        std::fs::create_dir_all(output_dir)
            .map_err(|e| Repr::io("create directory", Some(output_dir), e))?;

        if object.class == Class::TextAsset {
            audio_paths.extend(text::extract(&bundle, object, &output_path, options.decrypt_text)?);
        } else {
            texture::extract(&bundle, object, output_dir, options.image_format)?;
        }
    }

    // unload the file before the time-consuming audio decoding
    drop(bundle);

    // we should handle audio files after all of them are extracted because some files may
    // contain informations (Ex: filename) about others
    if options.decode_audio {
        for audio_path in audio_paths {
            let output_dir = audio_path
                .parent()
                .ok_or_else(|| Repr::UnsafePath(audio_path.display().to_string()))?;
            decode_audio(&audio_path, output_dir, &options.audio, key_files)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Options, extract_all};
    use crate::ErrorKind;
    use crate::unity::RabexExtractor;
    use crate::util::init_test_logger;

    #[test]
    fn test_extract_all_reports_failures() {
        init_test_logger();

        let dir = tempfile::tempdir().unwrap();
        let bundle = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_text.unity3d");
        let files = vec![bundle.into(), dir.path().join("missing.unity3d")];

        let options = Options::new(dir.path().join("out"));
        let failures = extract_all(RabexExtractor::pool(2), files.into_iter(), &options);

        // only the missing file fails
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].0, dir.path().join("missing.unity3d"));
        assert_eq!(failures[0].1.kind(), ErrorKind::Io);
        assert!(dir.path().join("out/Assets/imas/resources/textassets/zh/commu/card").is_dir());
    }
}
