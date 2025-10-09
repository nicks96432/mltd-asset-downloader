//! Audio transcoding using vgmstream and ffmpeg.

#![allow(unsafe_code, clippy::cast_possible_wrap, clippy::cast_sign_loss)]

use std::collections::VecDeque;
use std::ffi::{c_int, c_uint};
use std::path::Path;

use ffmpeg_next::Rescale;
use ffmpeg_next::format::Sample;
use ffmpeg_next::packet::Mut;
use thiserror::Error as ThisError;
use vgmstream::{StreamFile, VgmStream};

use crate::error::{Error, Repr, Result};

#[derive(Debug, ThisError)]
pub(crate) enum AudioDecodeError {
    #[error("vgmstream error: {0}")]
    VgmStream(#[from] vgmstream::Error),

    #[error("ffmpeg error: {reason}")]
    FFmpeg { reason: &'static str, source: Option<ffmpeg_next::Error> },

    #[error("unsupported channel layout: {0:?}")]
    UnsupportedChannelLayout(vgmstream::ChannelMapping),
}

impl AudioDecodeError {
    pub fn ffmpeg(reason: &'static str, source: Option<ffmpeg_next::Error>) -> Self {
        Self::FFmpeg { reason, source }
    }
}

impl From<AudioDecodeError> for Error {
    fn from(value: AudioDecodeError) -> Self {
        Repr::from(value).into()
    }
}

/// HCA key used to decrypt MLTD audio asset.
pub const MLTD_HCA_KEY: u64 = 765_765_765_765_765;

/// An encoder that transcodes game audio to the target codec.
pub struct Encoder<'a> {
    /// `VGMStream` stream file.
    pub vgmstream: VgmStream,

    /// Original audio channel layout.
    pub from_channel_layout: ffmpeg_next::ChannelLayout,
    /// Original sample format.
    pub from_sample_format: Sample,
    /// Original sample rate.
    pub from_sample_rate: i32,

    /// `FFmpeg` encoder.
    pub encoder: ffmpeg_next::codec::encoder::audio::Encoder,

    /// `FFmpeg` encoder options.
    pub options: Option<ffmpeg_next::Dictionary<'a>>,

    /// `FFmpeg` output context.
    pub output: ffmpeg_next::format::context::Output,

    /// `FFmpeg` resampler context.
    pub resampler: ffmpeg_next::software::resampling::Context,

    /// `FFmpeg` audio frame.
    pub frame: ffmpeg_next::frame::Audio,

    /// Audio sample count.
    pub sample_count: i64,

    /// Audio next presentation timestamp.
    pub next_pts: i64,

    /// Fifo for audio data.
    fifo: VecDeque<u8>,
}

/// Audio encoder output options.
#[derive(Debug, Clone)]
pub struct EncoderOutputOptions<'a> {
    /// Output filename prefix if stream name is empty.
    pub prefix: &'a str,

    /// `FFmpeg` codec name.
    pub codec: &'a str,

    /// `FFmpeg` codec format.
    pub format: &'a str,

    /// `FFmpeg` codec and format options.
    pub ffmpeg_options: Option<ffmpeg_next::Dictionary<'a>>,
}

impl<'a> Encoder<'a> {
    /// Default audio frame size.
    ///
    /// Some codecs accept variable frame sizes, and this is used for those codecs.
    pub const DEFAULT_FRAME_SIZE: u32 = 4096;

    /// Opens the encoder with the given parameters.
    ///
    /// `VGMStream` will decode the input game audio, and `FFmpeg` will enocde with the given
    /// codec and options. The output file will be truncated if it exists.
    ///
    /// # Errors
    ///
    /// [`Error::VGMStream`]: if vgmstream cannot identify the input file format.
    ///
    /// [`Error::FFmpeg`]: if ffmpeg encoder initialization failed.
    pub fn open<P>(
        input_file: P,
        subsong_index: usize,
        output_dir: P,
        output_options: &EncoderOutputOptions<'a>,
    ) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut vgmstream = VgmStream::new().map_err(AudioDecodeError::from)?;
        let mut sf = StreamFile::open(input_file.as_ref()).map_err(AudioDecodeError::from)?;
        vgmstream.open(&mut sf, subsong_index).map_err(AudioDecodeError::from)?;

        let fmt = vgmstream.format().map_err(AudioDecodeError::from)?;

        if subsong_index >= fmt.subsong_count as usize {
            return Err(Repr::OutOfRange(subsong_index, fmt.subsong_count as usize).into());
        }

        tracing::trace!(?fmt);

        let codec = ffmpeg_next::encoder::find_by_name(output_options.codec)
            .ok_or(AudioDecodeError::ffmpeg("codec not found", None))?;

        let mut encoder = ffmpeg_next::codec::Context::new_with_codec(codec)
            .encoder()
            .audio()
            .map_err(|e| AudioDecodeError::ffmpeg("failed to create encoder", Some(e)))?;

        let supported_formats = get_supported_sample_fmts(&encoder)?;
        tracing::trace!(?supported_formats);

        let from_sample_format = to_ffmpeg_sample(fmt.sample_format);
        let from_channel_layout = to_ffmpeg_channel_layout(fmt.channel_layout)?;

        encoder.set_format(choose_sample_fmt(&supported_formats, from_sample_format));
        encoder.set_bit_rate(320_000);
        encoder.set_compression(Some(12));
        encoder.set_rate(fmt.sample_rate);
        encoder.set_channel_layout(from_channel_layout);

        let output_filename = match fmt.stream_name.as_str() {
            "" => format!("{}_{}.{}", output_options.prefix, subsong_index, output_options.format),
            name => format!("{}.{}", name, output_options.format),
        };
        let output_path = output_dir.as_ref().join(&output_filename);

        tracing::info!(output_path = %output_path.display(), "writing audio");
        if fmt.loop_flag {
            tracing::info!(
                output_path = %output_path.display(),
                "this audio has a loop from sample {} to {}",
                fmt.loop_start,
                fmt.loop_end
            );
        }

        let mut output = match output_options.ffmpeg_options {
            Some(ref o) => ffmpeg_next::format::output_with(&output_path, o.clone()),
            None => ffmpeg_next::format::output(&output_path),
        }
        .map_err(|e| AudioDecodeError::ffmpeg("failed to open output", Some(e)))?;

        if output.format().flags().contains(ffmpeg_next::format::Flags::GLOBAL_HEADER) {
            let flag = ffmpeg_next::codec::Flags::from_bits(
                unsafe { *encoder.as_mut_ptr() }.flags as c_uint,
            )
            .ok_or_else(|| AudioDecodeError::ffmpeg("invalid encoder flags", None))?;
            encoder.set_flags(flag | ffmpeg_next::codec::Flags::GLOBAL_HEADER);
        }

        let encoder = match output_options.ffmpeg_options {
            Some(ref o) => encoder.open_with(o.clone()),
            None => encoder.open(),
        }
        .map_err(|e| AudioDecodeError::ffmpeg("failed to open encoder", Some(e)))?;

        let _ = output
            .add_stream_with(&encoder.0.0.0)
            .map_err(|e| AudioDecodeError::ffmpeg("failed to add stream", Some(e)));

        let frame_size = if encoder
            .codec()
            .ok_or_else(|| AudioDecodeError::ffmpeg("encoder does not have a codec", None))?
            .capabilities()
            .intersects(ffmpeg_next::codec::Capabilities::VARIABLE_FRAME_SIZE)
        {
            tracing::trace!(
                "variable frame size detected, using default frame size ({})",
                Self::DEFAULT_FRAME_SIZE
            );
            Self::DEFAULT_FRAME_SIZE
        } else {
            encoder.frame_size()
        } as usize;

        let mut frame =
            ffmpeg_next::frame::Audio::new(encoder.format(), frame_size, encoder.channel_layout());
        frame.set_pts(Some(0));
        frame.set_rate(encoder.rate());

        let resampler = ffmpeg_next::software::resampler(
            (from_sample_format, from_channel_layout, fmt.sample_rate as u32),
            (encoder.format(), encoder.channel_layout(), encoder.rate()),
        )
        .map_err(|e| AudioDecodeError::ffmpeg("failed to create resampler", Some(e)))?;

        Ok(Self {
            vgmstream,
            from_channel_layout,
            from_sample_format,
            from_sample_rate: fmt.sample_rate,
            encoder,
            options: output_options.ffmpeg_options.clone(),
            output,
            resampler,
            frame,

            sample_count: 0,
            next_pts: 0,

            fifo: VecDeque::new(),
        })
    }

    /// Encodes the next audio frame and writes the encoded packets to the output file.
    ///
    /// Returns `false` if there is more audio data to encode.
    fn write_frame(&mut self, eof: bool) -> Result<bool, ffmpeg_next::Error> {
        if eof { self.encoder.send_eof() } else { self.encoder.send_frame(&self.frame) }?;

        loop {
            let mut packet = ffmpeg_next::Packet::empty();
            if let Err(e) = self.encoder.receive_packet(&mut packet) {
                let errno = c_int::from(e);
                if errno == ffmpeg_next::ffi::AVERROR(ffmpeg_next::ffi::EAGAIN) {
                    return Ok(true);
                }
                if errno == ffmpeg_next::ffi::AVERROR_EOF {
                    return Ok(false);
                }

                return Err(e);
            }

            packet.rescale_ts(
                self.encoder.time_base(),
                self.output.stream_mut(0).unwrap().time_base(),
            );
            packet.set_stream(0);

            // XXX: packet.write() and packet.write_interleved() checks that the packet
            // is not empty, but empty packet with side data is valid.
            match unsafe {
                ffmpeg_next::ffi::av_interleaved_write_frame(
                    self.output.as_mut_ptr(),
                    packet.as_mut_ptr(),
                )
            } {
                0 => Ok(()),
                e => Err(ffmpeg_next::Error::from(e)),
            }?;
        }
    }

    /// Gets the next audio frame from vgmstream decoder.
    ///
    /// Returns `None` if there is no more audio data.
    fn get_audio_frame(&mut self) -> Option<ffmpeg_next::frame::Audio> {
        let needed_len = self.frame.samples()
            * self.from_channel_layout.channels() as usize
            * self.from_sample_format.bytes();

        while let Ok(buf) = self.vgmstream.render() {
            if buf.is_empty() {
                break;
            }

            self.fifo.extend(buf);
            if self.fifo.len() >= needed_len {
                break;
            }
        }

        let samples = match self.fifo.len() {
            0 => return None,
            len if len < needed_len => std::mem::take(&mut self.fifo).into(),
            _ => {
                let mut rest = self.fifo.split_off(needed_len);
                std::mem::swap(&mut rest, &mut self.fifo);

                Vec::from(rest)
            }
        };
        let frame_size = samples.len()
            / (self.from_sample_format.bytes() * self.from_channel_layout.channels() as usize);

        let mut frame = ffmpeg_next::frame::Audio::new(
            self.from_sample_format,
            frame_size,
            self.from_channel_layout,
        );

        frame.data_mut(0)[..samples.len()].copy_from_slice(&samples);
        frame.data_mut(0)[samples.len()..].fill(0);

        frame.set_rate(self.from_sample_rate as u32);
        frame.set_pts(Some(self.next_pts));
        self.next_pts += frame_size as i64;

        Some(frame)
    }

    /// Encodes the next audio frame.
    ///
    /// Returns `false` if there is more audio data to encode.
    fn write_audio_frame(&mut self) -> Result<bool, ffmpeg_next::Error> {
        if let Some(frame) = self.get_audio_frame() {
            debug_assert_eq!(self.resampler.delay(), None, "there should be no delay");

            self.resampler.run(&frame, &mut self.frame)?;

            let pts = self.sample_count.rescale(
                ffmpeg_next::Rational::new(1, self.encoder.rate() as i32),
                self.encoder.time_base(),
            );
            self.frame.set_pts(Some(pts));
            self.frame.set_samples(frame.samples());
            self.sample_count += self.frame.samples() as i64;

            return self.write_frame(false);
        }

        loop {
            if let Ok(None) = self.resampler.flush(&mut self.frame) {
                break;
            }

            let pts = self.frame.pts().unwrap().rescale(
                ffmpeg_next::Rational::new(1, self.encoder.rate() as i32),
                self.encoder.time_base(),
            );
            self.frame.set_pts(Some(pts));
            self.sample_count += self.frame.samples() as i64;

            tracing::trace!(samples = self.frame.samples(), "flushed samples");
            self.write_frame(false)?;
        }

        self.write_frame(true)
    }

    /// Encodes the audio streams.
    ///
    /// # Errors
    ///
    /// [`Error::FFmpeg`]: if encoding failed.
    pub fn encode(&mut self) -> Result<()> {
        match self.options {
            Some(ref o) => {
                let _ = self.output.write_header_with(o.clone()).map_err(|e| {
                    AudioDecodeError::ffmpeg("failed to write audio header", Some(e))
                })?;
            }
            None => self
                .output
                .write_header()
                .map_err(|e| AudioDecodeError::ffmpeg("failed to write audio header", Some(e)))?,
        }

        while self
            .write_audio_frame()
            .map_err(|e| AudioDecodeError::ffmpeg("failed to write audio frame", Some(e)))?
        {
        }

        self.output
            .write_trailer()
            .map_err(|e| AudioDecodeError::ffmpeg("failed to write audio trailer", Some(e)))?;

        Ok(())
    }
}

fn to_ffmpeg_sample(value: vgmstream::SampleFormat) -> Sample {
    match value {
        vgmstream::SampleFormat::Pcm16 => Sample::I16(ffmpeg_next::format::sample::Type::Packed),
        vgmstream::SampleFormat::Pcm24 | vgmstream::SampleFormat::Pcm32 => {
            Sample::I32(ffmpeg_next::format::sample::Type::Packed)
        }
        vgmstream::SampleFormat::Float => Sample::F32(ffmpeg_next::format::sample::Type::Packed),
    }
}

fn to_ffmpeg_channel_layout(
    layout: vgmstream::ChannelMapping,
) -> Result<ffmpeg_next::ChannelLayout> {
    match layout {
        vgmstream::ChannelMapping::MONO => Ok(ffmpeg_next::ChannelLayout::MONO),
        vgmstream::ChannelMapping::STEREO => Ok(ffmpeg_next::ChannelLayout::STEREO),
        vgmstream::ChannelMapping::_2POINT1 => Ok(ffmpeg_next::ChannelLayout::_2POINT1),
        vgmstream::ChannelMapping::_2POINT1_XIPH => Ok(ffmpeg_next::ChannelLayout::SURROUND),
        vgmstream::ChannelMapping::QUAD => Ok(ffmpeg_next::ChannelLayout::QUAD),
        vgmstream::ChannelMapping::QUAD_SURROUND => Ok(ffmpeg_next::ChannelLayout::_4POINT0),
        vgmstream::ChannelMapping::QUAD_SIDE => Ok(ffmpeg_next::ChannelLayout::_2_2),
        vgmstream::ChannelMapping::_5POINT0_XIPH => Ok(ffmpeg_next::ChannelLayout::_5POINT0_BACK),
        vgmstream::ChannelMapping::_5POINT0_SURROUND => Ok(ffmpeg_next::ChannelLayout::_5POINT0),
        vgmstream::ChannelMapping::_5POINT1 => Ok(ffmpeg_next::ChannelLayout::_5POINT1_BACK),
        vgmstream::ChannelMapping::_5POINT1_SURROUND => Ok(ffmpeg_next::ChannelLayout::_5POINT1),
        vgmstream::ChannelMapping::_7POINT1 => Ok(ffmpeg_next::ChannelLayout::_7POINT1_WIDE_BACK),
        vgmstream::ChannelMapping::_7POINT1_SURROUND => {
            Ok(ffmpeg_next::ChannelLayout::_7POINT1_WIDE)
        }
        _ => Err(AudioDecodeError::UnsupportedChannelLayout(layout).into()),
    }
}

/// Returns a list of supported audio formats.
fn get_supported_sample_fmts(
    encoder: &ffmpeg_next::codec::encoder::Encoder,
) -> Result<Vec<Sample>> {
    let mut supported_formats = std::ptr::null();
    let mut num_formats = 0;
    unsafe {
        ffmpeg_next::ffi::avcodec_get_supported_config(
            encoder.as_ptr(),
            std::ptr::null(),
            ffmpeg_next::ffi::AVCodecConfig::AV_CODEC_CONFIG_SAMPLE_FORMAT,
            0,
            &raw mut supported_formats,
            &raw mut num_formats,
        )
    };
    if supported_formats.is_null() {
        return Err(AudioDecodeError::ffmpeg("Failed to get supported configs", None).into());
    }

    Ok(unsafe {
        std::slice::from_raw_parts(
            supported_formats.cast::<ffmpeg_next::ffi::AVSampleFormat>(),
            num_formats as usize,
        )
    }
    .iter()
    .map(|fmt| (*fmt).into())
    .collect())
}

fn choose_sample_fmt(supported: &[Sample], wanted: Sample) -> Sample {
    if supported.contains(&wanted) {
        return wanted;
    }

    // Try to find the closest supported format
    let closest = supported
        .iter()
        .map(|fmt| (wanted.bytes().abs_diff(fmt.bytes()), fmt))
        .min_by_key(|(diff, _)| *diff)
        .unwrap();

    tracing::warn!(original = ?wanted, ?supported, closest = ?closest.1, "original sample format not supported, using closest");

    *closest.1
}

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;
    use assert_fs::prelude::*;

    use crate::util::init_test_logger;

    use super::*;

    #[test]
    fn test_encoder_encode() {
        init_test_logger();

        let dir = TempDir::new().unwrap();

        let mut encoder = Encoder::open(
            Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_acb.acb")),
            0,
            dir.path(),
            &EncoderOutputOptions {
                prefix: "song3_00test",
                codec: "pcm_s16le",
                format: "wav",
                ffmpeg_options: None,
            },
        )
        .unwrap();

        encoder.encode().unwrap();

        assert!(dir.child("song3_00test_bgm.wav").exists());
    }
}
