use anyhow::{Result, anyhow};
use clap::Args;
use mltd::decode::audio::AudioOptions;

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

impl From<&AudioArgs> for AudioOptions {
    fn from(args: &AudioArgs) -> Self {
        Self {
            format: args.audio_format.clone(),
            codec: args.audio_codec.clone(),
            ffmpeg_args: args.audio_args.clone(),
        }
    }
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
