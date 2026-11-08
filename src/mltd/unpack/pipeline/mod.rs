//! Unpacks the assets in Unity asset bundles to files.
//!
//! Every [`Class::TextAsset`] and [`Class::Texture2D`] in a bundle is written to the path it had
//! before bundled, under the output directory:
//!
//! - text assets are written as is, with the `.bytes` extension removed
//! - textures are written as images, with the puzzles solved
//!
//! The following steps are also done if their features are enabled, unless they are disabled by
//! the options:
//!
//! - with the `decrypt-text` feature, encrypted text assets are decrypted, unless
//!   `Options::decrypt_text` is `false`
//! - with the `decode-audio` feature, audio assets are decoded after all the files of the bundle
//!   are written, unless `Options::decode_audio` is `false`

mod text;
mod texture;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use image::ImageFormat;

#[cfg(feature = "decode-audio")]
pub use crate::decode::audio::{AudioOptions, decode as decode_audio};
use crate::error::{Error, Repr, Result};
use crate::unity::{Bundle, Class, Extractor};
use crate::util::safe_join;

/// Options of the unpacking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Options {
    /// The output directory.
    pub output: PathBuf,

    /// Format of the output images.
    pub image_format: ImageFormat,

    /// Whether to decrypt the encrypted text assets.
    ///
    /// If it is `false`, texts are written as is.
    #[cfg(feature = "decrypt-text")]
    pub decrypt_text: bool,

    /// Whether to decode the audio assets.
    ///
    /// If it is `false`, audios are written as is.
    #[cfg(feature = "decode-audio")]
    pub decode_audio: bool,

    /// Options of the audio output.
    #[cfg(feature = "decode-audio")]
    pub audio: AudioOptions,
}

impl Options {
    /// Creates options that write to `output` with the default formats.
    ///
    /// # Example
    ///
    /// ```
    /// use image::ImageFormat;
    /// use mltd::unpack::pipeline::Options;
    ///
    /// let mut options = Options::new("output");
    /// assert_eq!(options.image_format, ImageFormat::Png);
    ///
    /// options.image_format = ImageFormat::WebP;
    /// ```
    #[must_use]
    pub fn new(output: impl Into<PathBuf>) -> Self {
        Self {
            output: output.into(),
            image_format: ImageFormat::Png,
            #[cfg(feature = "decrypt-text")]
            decrypt_text: true,
            #[cfg(feature = "decode-audio")]
            decode_audio: true,
            #[cfg(feature = "decode-audio")]
            audio: AudioOptions::default(),
        }
    }

    /// Whether the encrypted text assets should be decrypted.
    #[cfg_attr(not(feature = "decrypt-text"), allow(clippy::unused_self))]
    fn decrypts_text(&self) -> bool {
        #[cfg(feature = "decrypt-text")]
        return self.decrypt_text;

        #[cfg(not(feature = "decrypt-text"))]
        return false;
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Unpacks the files with the given extractors, each of them handles one file at a time.
///
/// Returns the files that failed to unpack, with the reasons.
///
/// # Example
///
/// ```no_run
/// use std::path::PathBuf;
///
/// use mltd::unity::RabexExtractor;
/// use mltd::unpack::pipeline::{Options, unpack_all};
///
/// let files = vec![PathBuf::from("a.unity3d"), PathBuf::from("b.unity3d")];
///
/// // two workers, each of them unpacks one file at a time
/// let failures = unpack_all(RabexExtractor::pool(2), files.into_iter(), &Options::new("output"));
/// for (path, error) in failures {
///     eprintln!("{}: {error}", path.display());
/// }
/// ```
pub fn unpack_all<E>(
    extractors: Vec<E>,
    files: impl Iterator<Item = PathBuf> + Send,
    options: &Options,
) -> Vec<(PathBuf, Error)>
where
    E: Extractor + Send,
{
    tracing::info!(workers = extractors.len(), "unpacking files");

    let files = Mutex::new(files);
    let failures = Mutex::new(Vec::new());

    std::thread::scope(|scope| {
        for mut extractor in extractors {
            let (files, failures) = (&files, &failures);

            scope.spawn(move || {
                loop {
                    let next = lock(files).next();
                    let Some(path) = next else { break };

                    if let Err(e) = unpack_file(&mut extractor, &path, options) {
                        let input_path = path.display();
                        tracing::warn!(%input_path, error = ?e, "failed to unpack file");
                        lock(failures).push((path, e));
                    }
                }
            });
        }
    });

    let failures = failures.into_inner().unwrap_or_else(PoisonError::into_inner);
    tracing::info!(failed = failures.len(), "finished unpacking files");

    failures
}

/// Unpacks a single .unity3d file.
///
/// # Errors
///
/// This method fails if the file cannot be loaded, or any of the assets cannot be unpacked.
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
///
/// use mltd::unity::RabexExtractor;
/// use mltd::unpack::pipeline::{Options, unpack_file};
///
/// let mut extractor = RabexExtractor::new();
/// unpack_file(&mut extractor, Path::new("a.unity3d"), &Options::new("output")).unwrap();
/// ```
pub fn unpack_file<E>(extractor: &mut E, path: &Path, options: &Options) -> Result<()>
where
    E: Extractor,
{
    // the files are unpacked in parallel, so the span tells which file a message is about
    let span = tracing::info_span!("unpack_file", input = %path.display());
    let _entered = span.enter();

    tracing::debug!("loading file");
    let bundle = extractor.load(path)?;

    let mut audio_paths = BTreeSet::new();
    let strips = texture::strip_groups(&bundle);
    tracing::debug!(objects = bundle.objects().len(), strip_groups = strips.len(), "loaded file");

    for object in bundle.objects() {
        match &object.class {
            // asset bundles contains bundle information only, and sprites are handled with
            // their textures
            Class::AssetBundle | Class::Sprite => {
                tracing::trace!(name = object.name, class = %object.class, "skipping object");
                continue;
            }
            Class::TextAsset | Class::Texture2D => (),
            class => {
                tracing::warn!(%class, name = object.name, "unknown asset class");
                continue;
            }
        }

        let Some(orig_path) = object.orig_path.as_deref() else {
            tracing::warn!(name = object.name, class = %object.class, "asset has no original path");
            continue;
        };

        tracing::trace!(
            name = object.name,
            class = %object.class,
            path_id = object.path_id,
            orig_path,
            "unpacking object"
        );
        let output_path = safe_join(&options.output, orig_path)?;
        let output_dir =
            output_path.parent().ok_or_else(|| Repr::UnsafePath(orig_path.to_owned()))?;

        std::fs::create_dir_all(output_dir)
            .map_err(|e| Repr::io("create directory", Some(output_dir), e))?;

        if object.class == Class::TextAsset {
            audio_paths.extend(text::extract(
                &bundle,
                object,
                &output_path,
                options.decrypts_text(),
            )?);
        } else if object.class == Class::Texture2D {
            texture::extract(&bundle, object, output_dir, options.image_format, &strips)?;
        }
    }

    // unload the file before the time-consuming audio decoding
    drop(bundle);

    // we should handle audio files after all of them are unpacked because some files may
    // contain informations (Ex: filename) about others
    #[cfg(feature = "decode-audio")]
    if options.decode_audio {
        tracing::debug!(count = audio_paths.len(), "decoding audio files");
        for audio_path in audio_paths {
            let output_dir = audio_path
                .parent()
                .ok_or_else(|| Repr::UnsafePath(audio_path.display().to_string()))?;
            decode_audio(&audio_path, output_dir, &options.audio)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Options, unpack_all};
    use crate::ErrorKind;
    use crate::unity::RabexExtractor;
    use crate::util::init_test_logger;

    #[test]
    fn test_unpack_all_reports_failures() {
        init_test_logger();

        let dir = tempfile::tempdir().unwrap();
        let bundle = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_text.unity3d");
        let files = vec![bundle.into(), dir.path().join("missing.unity3d")];

        let options = Options::new(dir.path().join("out"));
        let failures = unpack_all(RabexExtractor::pool(2), files.into_iter(), &options);

        // only the missing file fails
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].0, dir.path().join("missing.unity3d"));
        assert_eq!(failures[0].1.kind(), ErrorKind::Io);
        assert!(dir.path().join("out/assets/imas/resources/textassets/zh/commu/card").is_dir());
    }
}
