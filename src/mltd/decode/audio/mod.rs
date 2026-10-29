//! Decodes the audio files unpacked from the assets.

pub mod encoder;

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use rayon::iter::{IntoParallelIterator, ParallelIterator};

use self::encoder::{AudioDecodeError, Encoder, EncoderOutputOptions, MLTD_HCA_KEY};
use crate::error::{Repr, Result};

/// Options of the audio output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioOptions {
    /// `FFmpeg` codec format, i.e. the extension of the output files.
    pub format: String,

    /// `FFmpeg` codec name.
    pub codec: String,

    /// `FFmpeg` codec and format options.
    pub ffmpeg_args: Vec<(String, String)>,
}

impl Default for AudioOptions {
    fn default() -> Self {
        Self { format: "wav".to_owned(), codec: "pcm_s16le".to_owned(), ffmpeg_args: Vec::new() }
    }
}

/// Files used to pass the HCA decryption keys to `vgmstream`.
///
/// The files are shared by the workers, and are removed when this is dropped.
#[derive(Debug, Default)]
pub struct KeyFiles(Mutex<BTreeSet<PathBuf>>);

impl KeyFiles {
    /// Makes sure that the key file `path` exists.
    ///
    /// All MLTD audio files use the same key, so each file is written once.
    fn ensure(&self, path: PathBuf) -> Result<()> {
        let mut paths = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if !paths.contains(&path) {
            tracing::debug!(output_path = %path.display(), "writing temporary .hcakey");
            std::fs::write(&path, MLTD_HCA_KEY.to_string())
                .map_err(|e| Repr::io("write key file", Some(&path), e))?;
            paths.insert(path);
        }

        Ok(())
    }
}

impl Drop for KeyFiles {
    fn drop(&mut self) {
        let paths = self.0.get_mut().unwrap_or_else(PoisonError::into_inner);

        for path in paths.iter() {
            tracing::debug!(key_path = %path.display(), "removing temporary files");
            if let Err(e) = std::fs::remove_file(path) {
                tracing::warn!(error = ?e, "failed to remove temporary file");
            }
        }
    }
}

/// Decodes all the subsongs of the audio file `path` into `output_dir`.
///
/// The decoded files are named after `path`. A temporary key file is written next to `path`
/// and removed when `key_files` is dropped.
///
/// # Errors
///
/// This function fails if the file cannot be opened, or any of the subsongs cannot be decoded.
pub fn decode(
    path: &Path,
    output_dir: &Path,
    options: &AudioOptions,
    key_files: &KeyFiles,
) -> Result<()> {
    // According to https://github.com/vgmstream/vgmstream/blob/master/doc/USAGE.md#decryption-keys,
    // we can specify the decryption key in the .hcakey file so that vgmstream doesn't have
    // to brute-force the key.
    key_files.ensure(path.with_file_name(".hcakey"))?;

    let unsafe_path = || Repr::UnsafePath(path.display().to_string());
    let prefix = path.file_stem().and_then(OsStr::to_str).ok_or_else(unsafe_path)?;
    tracing::info!(output_path = %output_dir.display(), "extracting audio");

    let mut sf = vgmstream::StreamFile::open(path).map_err(AudioDecodeError::from)?;
    let mut vgmstream = vgmstream::VgmStream::new().map_err(AudioDecodeError::from)?;
    vgmstream.open(&mut sf, 0).map_err(AudioDecodeError::from)?;
    let subsongs = vgmstream.format().map_err(AudioDecodeError::from)?.subsong_count;
    let subsongs = usize::try_from(subsongs).unwrap_or_default();
    tracing::debug!(output_path = %output_dir.display(), subsongs);

    let results: Vec<Result<()>> = (0..subsongs)
        .into_par_iter()
        .map(|i| -> Result<()> {
            let mut ffmpeg_options = ffmpeg_next::Dictionary::new();
            for (key, value) in &options.ffmpeg_args {
                ffmpeg_options.set(key, value);
            }

            let output_options = EncoderOutputOptions {
                prefix,
                codec: &options.codec,
                format: &options.format,
                ffmpeg_options: Some(ffmpeg_options),
            };

            let mut encoder = Encoder::open(path, i, output_dir, &output_options)?;
            encoder.encode()?;

            Ok(())
        })
        .collect();

    // report every failed subsong, but only fail once
    let mut first_error = None;
    for result in results {
        if let Err(e) = result {
            tracing::warn!(input_path = %path.display(), error = ?e, "failed to extract audio");
            first_error.get_or_insert(e);
        }
    }

    if let Some(e) = first_error {
        return Err(e);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::KeyFiles;

    #[test]
    fn test_key_files_removed_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join(".hcakey");

        let key_files = KeyFiles::default();
        key_files.ensure(key_path.clone()).unwrap();
        // the file is written only once
        std::fs::write(&key_path, "kept").unwrap();
        key_files.ensure(key_path.clone()).unwrap();
        assert_eq!(std::fs::read_to_string(&key_path).unwrap(), "kept");

        drop(key_files);
        assert!(!key_path.exists());
    }
}
