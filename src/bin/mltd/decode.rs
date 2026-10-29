use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use clap::Args;
use mltd::decode::audio::{AudioOptions, KeyFiles, decode as decode_audio};
use mltd::decode::text::decrypt_file;
use mltd::extract::{AUDIO_EXTENSIONS, ENCRYPTED_TEXT_EXTENSIONS};

use crate::audio::AudioArgs;
use crate::util::glob_files;

#[derive(Debug, Args)]
#[command(author, version, about, arg_required_else_help(true))]
pub struct DecoderArgs {
    /// The files extracted with `extract --raw`, supports glob patterns.
    ///
    /// Encrypted text files (.gtx.bytes, .mld.bytes) are decrypted, and audio files (.acb, .awb)
    /// are decoded. Other files are skipped.
    #[arg(value_name = "PATH", num_args = 1..)]
    pub input_paths: Vec<String>,

    /// The output directory, the output files are written next to the inputs if not specified
    #[arg(short, long, value_name = "DIR")]
    pub output: Option<PathBuf>,

    #[clap(flatten)]
    pub audio: AudioArgs,
}

/// What to do with an input file.
enum Task {
    /// Decrypt a text file to the given file name.
    Text(PathBuf),

    /// Decode an audio file.
    Audio,
}

/// Returns the extension of `path` if it is valid UTF-8.
fn extension(path: &Path) -> &str {
    path.extension().and_then(OsStr::to_str).unwrap_or_default()
}

/// Decides what to do with `input`.
///
/// `x.gtx.bytes` is decrypted to `x.gtx`. Text without `.bytes` is not decrypted unless the
/// output directory is different from the input's, because the encrypted and decrypted files
/// cannot be told apart.
fn classify(input: &Path, output_dir: Option<&Path>) -> Option<Task> {
    let ext = extension(input);

    if AUDIO_EXTENSIONS.contains(&ext) {
        return Some(Task::Audio);
    }

    let text = if ext == "bytes" { input.with_extension("") } else { input.to_owned() };
    if !ENCRYPTED_TEXT_EXTENSIONS.contains(&extension(&text)) {
        return None;
    }

    let name = text.file_name()?;
    let output = match output_dir {
        Some(dir) => dir.join(name),
        None => text,
    };

    Some(Task::Text(output))
}

fn decode_file(
    input: &Path,
    output_dir: Option<&Path>,
    audio: &AudioOptions,
    key_files: &KeyFiles,
) -> Result<()> {
    let Some(task) = classify(input, output_dir) else {
        tracing::warn!(input_path = %input.display(), "not an encrypted text or audio file, skipped");
        return Ok(());
    };

    match task {
        Task::Text(output) => {
            if output == input {
                return Err(anyhow!(
                    "`{}` would be decrypted in place, use --output to write it elsewhere",
                    input.display()
                ));
            }

            tracing::info!(input_path = %input.display(), output_path = %output.display(), "decrypting text");
            decrypt_file(input, &output)?;
        }
        Task::Audio => {
            let output_dir = output_dir
                .or_else(|| input.parent())
                .ok_or_else(|| anyhow!("`{}` has no parent directory", input.display()))?;

            decode_audio(input, output_dir, audio, key_files)?;
        }
    }

    Ok(())
}

pub fn decode_files(args: &DecoderArgs) -> Result<()> {
    if let Some(output) = &args.output {
        std::fs::create_dir_all(output)?;
    }

    let audio = AudioOptions::from(&args.audio);
    // dropped after all the files are decoded
    let key_files = KeyFiles::default();
    let mut failed = 0_usize;

    for input in glob_files(&args.input_paths) {
        if let Err(e) = decode_file(&input, args.output.as_deref(), &audio, &key_files) {
            tracing::warn!(input_path = %input.display(), error = ?e, "failed to decode file");
            failed += 1;
        }
    }

    if failed > 0 {
        return Err(anyhow!("failed to decode {failed} file(s)"));
    }

    Ok(())
}
