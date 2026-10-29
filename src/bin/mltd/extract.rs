use std::num::NonZeroUsize;
use std::path::PathBuf;

use anyhow::{Result, anyhow};
use clap::{Args, ValueEnum};
use mltd::extract::pipeline::{AudioOptions, Options, extract_all};

use crate::util::glob_files;

#[derive(Debug, Args)]
pub struct AudioArgs {
    /// Audio output format extension
    #[arg(long, value_name = "FORMAT", display_order = 2)]
    #[arg(default_value_t = String::from("wav"))]
    pub audio_format: String,

    /// Audio output codec
    #[arg(long, value_name = "CODEC", display_order = 2)]
    #[arg(default_value_t = String::from("pcm_s16le"))]
    pub audio_codec: String,

    /// Arguments to pass to ffmpeg encoder for audio output
    ///
    /// Value should be a list of -arg=value pairs separated by commas
    #[arg(long, value_name = "ARGS", display_order = 2)]
    #[arg(value_parser = parse_key_val, allow_hyphen_values = true)]
    pub audio_args: Vec<(String, String)>,
}

/// Library used to read Unity asset bundles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Backend {
    /// External AssetRipper process
    #[cfg(feature = "asset-ripper")]
    AssetRipper,

    /// Built-in pure Rust parser (experimental)
    #[cfg(feature = "rabex")]
    Rabex,
}

impl Default for Backend {
    fn default() -> Self {
        #[cfg(feature = "rabex")]
        return Self::Rabex;

        #[cfg(all(feature = "asset-ripper", not(feature = "rabex")))]
        return Self::AssetRipper;
    }
}

#[derive(Debug, Args)]
#[command(author, version, about, arg_required_else_help(true))]
pub struct ExtractorArgs {
    /// The library used to read the asset bundles
    #[arg(long, value_enum, default_value_t = Backend::default(), display_order = 3)]
    pub backend: Backend,

    /// The input files, supports glob patterns
    #[arg(value_name = "PATH", num_args = 1..)]
    pub input_paths: Vec<String>,

    /// The output directory
    #[arg(short, long, value_name = "DIR", display_order = 1)]
    #[arg(default_value_os_t = PathBuf::from("output"))]
    pub output: PathBuf,

    /// Write encrypted text and audio assets as they are, they can be decoded later by `mltd decode`
    #[arg(long, alias = "no-decrypt", display_order = 2)]
    pub raw: bool,

    #[clap(flatten)]
    pub audio: AudioArgs,

    /// Extension for image output
    #[arg(long, value_name = "EXT", display_order = 2)]
    #[arg(value_parser = parse_image_format, default_value = "png")]
    pub image_ext: image::ImageFormat,

    /// The number of asset to extract at the same time
    #[arg(short = 'P', long, value_name = "CPUS", display_order = 2)]
    #[arg(default_value_t = std::thread::available_parallelism().unwrap_or_else(|_|1.try_into().unwrap()))]
    pub parallel: NonZeroUsize,

    /// The path to the asset ripper executable
    #[cfg(feature = "asset-ripper")]
    #[arg(long, value_name = "PATH", display_order = 3)]
    #[arg(default_value_os_t = default_asset_ripper_path())]
    pub asset_ripper_path: PathBuf,
}

/// Parses a single key-value pair
fn parse_key_val(s: &str) -> Result<(String, String)> {
    if !s.starts_with('-') {
        return Err(anyhow!("invalid -KEY=value: no `-` found in `{}`", s));
    }
    let pos = match s.find('=') {
        Some(p) => p,
        None => return Err(anyhow!("invalid -KEY=value: no `=` found in `{}`", s))?,
    };

    Ok((s[1..pos].to_owned(), s[pos + 1..].to_owned()))
}

/// Parses string to image format
fn parse_image_format(s: &str) -> Result<image::ImageFormat> {
    let image_format = match image::ImageFormat::from_extension(s) {
        Some(f) => f,
        None => return Err(anyhow!("invalid image format: {}", s)),
    };

    Ok(image_format)
}

#[cfg(feature = "asset-ripper")]
#[inline]
fn default_asset_ripper_path() -> PathBuf {
    let mut path = PathBuf::from("AssetRipper");

    path.push(match cfg!(windows) {
        true => "AssetRipper.GUI.Free.exe",
        false => "AssetRipper.GUI.Free",
    });

    path
}

impl From<&ExtractorArgs> for Options {
    fn from(args: &ExtractorArgs) -> Self {
        Self {
            output: args.output.clone(),
            image_format: args.image_ext,
            decrypt_text: !args.raw,
            decode_audio: !args.raw,
            audio: AudioOptions::from(&args.audio),
        }
    }
}

impl From<&AudioArgs> for AudioOptions {
    fn from(args: &AudioArgs) -> Self {
        Self {
            format: args.audio_format.clone(),
            codec: args.audio_codec.clone(),
            ffmpeg_args: args.audio_args.clone(),
        }
    }
}

pub fn extract_files(args: &ExtractorArgs) -> Result<()> {
    let files = glob_files(&args.input_paths);

    let options = Options::from(args);

    let failures = match args.backend {
        #[cfg(feature = "asset-ripper")]
        Backend::AssetRipper => {
            tracing::debug!(
                parallel = args.parallel,
                asset_ripper_path = %args.asset_ripper_path.display(),
                "creating AssetRippers"
            );

            let rippers =
                mltd::unity::AssetRipper::pool(&args.asset_ripper_path, args.parallel.get())?;
            extract_all(rippers, files, &options)
        }

        #[cfg(feature = "rabex")]
        Backend::Rabex => {
            tracing::debug!(parallel = args.parallel, "creating rabex extractors");

            extract_all(mltd::unity::RabexExtractor::pool(args.parallel.get()), files, &options)
        }
    };

    if !failures.is_empty() {
        return Err(anyhow!("failed to extract {} file(s)", failures.len()));
    }

    Ok(())
}
