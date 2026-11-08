//! Decodes the audio files unpacked from the assets.

pub mod encoder;

use std::ffi::OsStr;
use std::path::Path;

use rayon::iter::{IntoParallelIterator, ParallelIterator};

use self::encoder::{AudioDecodeError, Encoder, EncoderOutputOptions};
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

/// Checks if `path` is an `.awb` file and has `.acb` next to it.
///
/// The streams of such an `.awb` file are decoded with the `.acb`, which also has their names,
/// so the `.awb` file should not be decoded again.
///
/// # Example
///
/// ```
/// use mltd::decode::audio::is_awb_companion;
///
/// let dir = assert_fs::TempDir::new().unwrap();
/// std::fs::write(dir.path().join("a.acb"), b"").unwrap();
///
/// assert!(is_awb_companion(&dir.path().join("a.awb")));
/// assert!(!is_awb_companion(&dir.path().join("b.awb")));
/// assert!(!is_awb_companion(&dir.path().join("a.acb")));
/// ```
#[must_use]
pub fn is_awb_companion(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "awb") && path.with_extension("acb").is_file()
}

/// Decodes all the subsongs of the audio file `path` into `output_dir`.
///
/// The decoded files are named after `path`.
///
/// # Errors
///
/// This function fails if the file cannot be opened, or any of the subsongs cannot be decoded.
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
///
/// use mltd::decode::audio::{AudioOptions, decode};
///
/// decode(Path::new("test.acb"), Path::new("output"), &AudioOptions::default()).unwrap();
/// ```
pub fn decode(path: &Path, output_dir: &Path, options: &AudioOptions) -> Result<()> {
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
