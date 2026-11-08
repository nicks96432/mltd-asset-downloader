use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use clap::Args;
#[cfg(feature = "decode-audio")]
use mltd::decode::audio::{AudioOptions, decode as decode_audio};
#[cfg(feature = "decrypt-text")]
use mltd::decode::text::decrypt_file;
use mltd::decode::{AssetKind, asset_kind};

#[cfg(feature = "decode-audio")]
use crate::audio::AudioArgs;
use crate::util::glob_files;

#[derive(Debug, Args)]
#[command(author, version, about, arg_required_else_help(true))]
pub struct DecoderArgs {
    /// The unpacked assets, supports glob patterns.
    ///
    /// Encrypted text files (.gtx.bytes, .mld.bytes) are decrypted, and audio files (.acb, .awb)
    /// are decoded, if this build supports them. Other files are skipped, and so are audio files
    /// with the .bytes extension, which vgmstream cannot recognize.
    #[arg(value_name = "PATH", num_args = 1..)]
    pub input_paths: Vec<String>,

    /// The output directory, the output files are written next to the inputs if not specified
    #[arg(short, long, value_name = "DIR")]
    pub output: Option<PathBuf>,

    #[cfg(feature = "decode-audio")]
    #[clap(flatten)]
    pub audio: AudioArgs,
}

/// What to do with an input file.
enum Task {
    /// Decrypt a text file to the given file name.
    #[cfg(feature = "decrypt-text")]
    Text(PathBuf),

    /// Decode an audio file.
    #[cfg(feature = "decode-audio")]
    Audio,

    /// Skip the file for the given reason.
    Skip(&'static str),
}

/// Decodes the input files with the same options.
struct Decoder<'a> {
    /// The output directory, the output files are written next to the inputs if [`None`].
    output_dir: Option<&'a Path>,

    #[cfg(feature = "decode-audio")]
    audio: AudioOptions,
}

/// Checks if `a` and `b` are the same file.
///
/// The paths are compared as they are if they cannot be canonicalized, e.g. a file that does not
/// exist yet.
#[cfg(feature = "decrypt-text")]
fn is_same_file(a: &Path, b: &Path) -> bool {
    use std::fs::canonicalize;

    a == b || matches!((canonicalize(a), canonicalize(b)), (Ok(a), Ok(b)) if a == b)
}

impl Decoder<'_> {
    /// Decides what to do with `input`.
    ///
    /// `x.gtx.bytes` is decrypted to `x.gtx`. Text without `.bytes` is decrypted to the same
    /// name, which is refused later unless the output directory is different from the input's,
    /// because the encrypted and decrypted files cannot be told apart.
    fn classify(&self, input: &Path) -> Task {
        let has_bytes = input.extension() == Some(OsStr::new("bytes"));

        match asset_kind(input) {
            Some(AssetKind::Audio) if has_bytes => {
                Task::Skip("vgmstream cannot recognize audio with the .bytes extension, remove it")
            }
            #[cfg(feature = "decode-audio")]
            Some(AssetKind::Audio) => Task::Audio,
            #[cfg(feature = "decrypt-text")]
            Some(AssetKind::EncryptedText) => {
                let text = if has_bytes { input.with_extension("") } else { input.to_owned() };
                let Some(name) = text.file_name() else {
                    return Task::Skip("no file name");
                };

                Task::Text(self.output_dir.map_or_else(|| text.clone(), |dir| dir.join(name)))
            }
            #[cfg(not(all(feature = "decrypt-text", feature = "decode-audio")))]
            Some(_) => Task::Skip("this build cannot decode this kind of file"),
            None => Task::Skip("not an encrypted text or audio file"),
        }
    }

    fn decode_file(&self, input: &Path) -> Result<()> {
        match self.classify(input) {
            #[cfg(feature = "decrypt-text")]
            Task::Text(output) => {
                if is_same_file(input, &output) {
                    return Err(anyhow!(
                        "`{}` would be decrypted in place, use --output to write it elsewhere",
                        input.display()
                    ));
                }

                tracing::info!(input_path = %input.display(), output_path = %output.display(), "decrypting text");
                decrypt_file(input, &output)?;
            }
            #[cfg(feature = "decode-audio")]
            Task::Audio => {
                let output_dir = self
                    .output_dir
                    .or_else(|| input.parent())
                    .ok_or_else(|| anyhow!("`{}` has no parent directory", input.display()))?;

                decode_audio(input, output_dir, &self.audio)?;
            }
            Task::Skip(reason) => {
                tracing::warn!(input_path = %input.display(), reason, "skipped");
            }
        }

        Ok(())
    }
}

pub fn decode_files(args: &DecoderArgs) -> Result<()> {
    if let Some(output) = &args.output {
        std::fs::create_dir_all(output)?;
    }

    let decoder = Decoder {
        output_dir: args.output.as_deref(),
        #[cfg(feature = "decode-audio")]
        audio: AudioOptions::from(&args.audio),
    };
    let mut failed = 0_usize;

    for input in glob_files(&args.input_paths) {
        if let Err(e) = decoder.decode_file(&input) {
            tracing::warn!(input_path = %input.display(), error = ?e, "failed to decode file");
            failed += 1;
        }
    }

    if failed > 0 {
        return Err(anyhow!("failed to decode {failed} file(s)"));
    }

    Ok(())
}
