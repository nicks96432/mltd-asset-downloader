use std::collections::VecDeque;
use std::ffi::{c_int, c_uint};
use std::path::Path;

use ffmpeg_next::packet::Mut;
use ffmpeg_next::Rescale;
use vgmstream::{Options, StreamFile, VgmStream};

use crate::Error;

pub struct Encoder<'a> {
    pub options: Option<ffmpeg_next::Dictionary<'a>>,

    pub vgmstream: VgmStream,
    pub from_channel_layout: ffmpeg_next::ChannelLayout,
    pub from_sample_format: ffmpeg_next::format::Sample,
    pub from_sample_rate: i32,

    pub encoder: ffmpeg_next::codec::encoder::audio::Encoder,

    pub output: ffmpeg_next::format::context::Output,

    pub resampler: ffmpeg_next::software::resampling::Context,

    pub frame: ffmpeg_next::frame::Audio,

    pub sample_count: i64,
    pub next_pts: i64,

    queue: VecDeque<u8>,
}

impl<'a> Encoder<'a> {
    pub const DEFAULT_FRAME_SIZE: u32 = 4096;

    pub fn open<P>(
        input: &P,
        output: &P,
        codec: &str,
        options: Option<ffmpeg_next::Dictionary<'a>>,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path> + ?Sized,
    {
        let mut vgmstream = VgmStream::new()?;
        let sf = StreamFile::open(&vgmstream, input)?;
        vgmstream.open_song(&mut Options {
            libsf: &sf,
            format_id: 0,
            stereo_track: 0,
            // FIXME: there are more subsongs
            subsong_index: 0,
        })?;

        let acb_fmt = vgmstream.format()?;

        log::trace!("audio format: {:#?}", acb_fmt);

        let mut output = match options {
            Some(ref o) => ffmpeg_next::format::output_with(output, o.clone()),
            None => ffmpeg_next::format::output(output.as_ref()),
        }?;

        let codec = ffmpeg_next::encoder::find_by_name(codec)
            .ok_or(Error::Generic("Failed to find encoder".to_owned()))?;

        let mut encoder = ffmpeg_next::codec::Context::new_with_codec(codec).encoder().audio()?;

        let supported_formats = get_supported_formats(&encoder)?;
        log::trace!("supported formats: {:?}", supported_formats);

        let from_sample_format = to_ffmpeg_sample_format(acb_fmt.sample_type)?;
        let from_channel_layout = to_ffmpeg_channel_layout(acb_fmt.channel_layout)?;

        encoder.set_format(choose_format(&supported_formats, from_sample_format));
        encoder.set_bit_rate(320000);
        encoder.set_compression(Some(12));
        encoder.set_rate(acb_fmt.sample_rate);
        encoder.set_channel_layout(from_channel_layout);

        if output.format().flags().contains(ffmpeg_next::format::Flags::GLOBAL_HEADER) {
            let flag = ffmpeg_next::codec::Flags::from_bits(
                unsafe { *encoder.as_mut_ptr() }.flags as c_uint,
            )
            .unwrap();
            encoder.set_flags(flag | ffmpeg_next::codec::Flags::GLOBAL_HEADER);
        }

        let encoder = match options {
            Some(ref o) => encoder.open_with(o.clone()),
            None => encoder.open(),
        }?;

        let _ = output.add_stream_with(&encoder.0 .0 .0)?;

        let frame_size = match encoder
            .codec()
            .unwrap()
            .capabilities()
            .intersects(ffmpeg_next::codec::Capabilities::VARIABLE_FRAME_SIZE)
        {
            true => {
                log::trace!("variable frame size detected, using default frame size");
                Self::DEFAULT_FRAME_SIZE
            }
            false => encoder.frame_size(),
        } as usize;

        let mut frame =
            ffmpeg_next::frame::Audio::new(encoder.format(), frame_size, encoder.channel_layout());
        frame.set_pts(Some(0));
        frame.set_rate(encoder.rate());

        let resampler = ffmpeg_next::software::resampler(
            (from_sample_format, from_channel_layout, acb_fmt.sample_rate as u32),
            (encoder.format(), encoder.channel_layout(), encoder.rate()),
        )?;

        Ok(Self {
            options,
            vgmstream,
            from_channel_layout,
            from_sample_format,
            from_sample_rate: acb_fmt.sample_rate,
            encoder,
            output,
            resampler,
            frame,

            sample_count: 0,
            next_pts: 0,

            queue: VecDeque::new(),
        })
    }

    fn write_frame(&mut self, eof: bool) -> Result<bool, Error> {
        match eof {
            false => self.encoder.send_frame(&self.frame),
            true => self.encoder.send_eof(),
        }?;

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

                return Err(Error::FFmpeg(e));
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

            self.queue.extend(buf);
            if self.queue.len() >= needed_len {
                break;
            }
        }

        let samples = match self.queue.len() {
            0 => return None,
            len if len < needed_len => std::mem::take(&mut self.queue).into(),
            _ => {
                let mut rest = self.queue.split_off(needed_len);
                std::mem::swap(&mut rest, &mut self.queue);

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
    fn write_audio_frame(&mut self) -> Result<bool, Error> {
        if let Some(frame) = self.get_audio_frame() {
            assert_eq!(self.resampler.delay(), None, "there should be no delay");

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

            log::trace!("flushed {} samples", self.frame.samples());
            self.write_frame(false)?;
        }

        self.write_frame(true)
    }

    pub fn encode(&mut self) -> Result<(), Error> {
        match self.options {
            Some(ref o) => {
                let _ = self.output.write_header_with(o.clone())?;
            }
            None => self.output.write_header()?,
        };

        while self.write_audio_frame()? {}

        self.output.write_trailer()?;

        Ok(())
    }
}

fn to_ffmpeg_sample_format(
    value: vgmstream::SampleType,
) -> Result<ffmpeg_next::format::Sample, Error> {
    match value {
        vgmstream::SampleType::Pcm16 => {
            Ok(ffmpeg_next::format::Sample::I16(ffmpeg_next::format::sample::Type::Packed))
        }
        vgmstream::SampleType::Pcm32 => {
            Ok(ffmpeg_next::format::Sample::I32(ffmpeg_next::format::sample::Type::Packed))
        }
        vgmstream::SampleType::Float => {
            Ok(ffmpeg_next::format::Sample::F32(ffmpeg_next::format::sample::Type::Packed))
        }
        _ => Err(Error::Generic(format!("Unsupported sample type: {:?}", value))),
    }
}

fn to_ffmpeg_channel_layout(
    value: vgmstream::ChannelMapping,
) -> Result<ffmpeg_next::ChannelLayout, Error> {
    match value {
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
        _ => Err(Error::Generic(format!("Unsupported channel layout: {:?}", value))),
    }
}

/// Returns a list of supported audio formats.
///
/// XXX: In the next version of FFmpeg, this function will be removed. Use
/// `get_supported_formats_new` below.
fn get_supported_formats(
    encoder: &ffmpeg_next::codec::encoder::Encoder,
) -> Result<Vec<ffmpeg_next::format::Sample>, Error> {
    match encoder.codec().unwrap().audio()?.formats() {
        Some(f) => Ok(f.collect()),
        None => Err(Error::Generic("no supported audio formats found".to_owned())),
    }
}

/*
fn get_supported_formats_new(
    encoder: &ffmpeg_next::codec::encoder::Encoder,
) -> Result<Vec<ffmpeg_next::format::Sample>, Error> {
    let mut supported_formats = std::ptr::null();
    let mut num_formats = 0;
    unsafe {
        ffmpeg_next::ffi::avcodec_get_supported_config(
            encoder.as_ptr(),
            std::ptr::null(),
            ffmpeg_next::ffi::AVCodecConfig::AV_CODEC_CONFIG_SAMPLE_FORMAT,
            0,
            &mut supported_formats,
            &mut num_formats,
        )
    };
    if supported_formats.is_null() {
        return Err(Error::Generic("Failed to get supported configs".to_owned()));
    }

    Ok(unsafe {
        std::slice::from_raw_parts(
            supported_formats as *const ffmpeg_next::ffi::AVSampleFormat,
            num_formats as usize,
        )
    }
    .iter()
    .map(|fmt| (*fmt).into())
    .collect())
}
*/

fn choose_format(
    supported_formats: &[ffmpeg_next::format::Sample],
    wanted_format: ffmpeg_next::format::Sample,
) -> ffmpeg_next::format::Sample {
    if supported_formats.contains(&wanted_format) {
        return wanted_format;
    }

    // Try to find the closest supported format
    let closest = supported_formats
        .iter()
        .map(|fmt| ((wanted_format.bytes() as i32 - fmt.bytes() as i32).abs(), fmt))
        .min_by_key(|(diff, _)| *diff)
        .unwrap();

    log::trace!("original sample format not supported, using closest: {:?}", closest.1);

    *closest.1
}
