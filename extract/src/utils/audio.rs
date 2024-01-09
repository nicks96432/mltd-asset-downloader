use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::Cursor;

use clap::builder::PossibleValue;
use clap::ValueEnum;
use flacenc::bitsink::ByteSink;
use flacenc::component::BitRepr;
use flacenc::config::Encoder;
use flacenc::encode_with_fixed_block_size;
use flacenc::source::MemSource;
use hound::WavReader;

#[derive(Debug, Clone, Copy)]
pub enum AudioFormat {
    Wav,
    Flac,
}

impl ValueEnum for AudioFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Wav, Self::Flac]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            Self::Wav => Some(PossibleValue::new("wav")),
            Self::Flac => Some(PossibleValue::new("flac")),
        }
    }
}

impl Display for AudioFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Wav => write!(f, "wav"),
            Self::Flac => write!(f, "flac"),
        }
    }
}

pub fn wav_to_flac(wav: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let wav = WavReader::new(Cursor::new(wav))?;

    let spec = wav.spec();
    let samples = wav.into_samples::<i16>();

    let source = MemSource::from_samples(
        samples.into_iter().map(|s| s.map(|s| s as i32)).collect::<Result<Vec<_>, _>>()?.as_ref(),
        spec.channels as usize,
        spec.bits_per_sample as usize,
        spec.sample_rate as usize,
    );

    // according to libflac best settings
    let mut flac_config = Encoder::default();
    flac_config.block_sizes = vec![4096];
    flac_config.subframe_coding.prc.max_parameter = 6;

    let flac_stream =
        encode_with_fixed_block_size(&flac_config, source, flac_config.block_sizes[0])
            .map_err(|err| format!("{:?}", err))?;

    let mut flac_sink = ByteSink::new();
    flac_stream.write(&mut flac_sink)?;

    Ok(flac_sink.into_inner())
}
