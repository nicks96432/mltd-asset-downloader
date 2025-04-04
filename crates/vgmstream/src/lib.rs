mod error;
mod sf;

use std::ffi::{CStr, c_int};

use bitflags::bitflags;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};

pub use crate::error::Error;
pub use crate::sf::StreamFile;

#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive, ToPrimitive)]
pub enum SampleType {
    Pcm16 = vgmstream_sys::libvgmstream_sfmt_t_LIBVGMSTREAM_SFMT_PCM16 as isize,
    // Pcm24 = vgmstream_sys::libvgmstream_sfmt_t_LIBVGMSTREAM_SFMT_PCM24 as isize,
    // Pcm32 = vgmstream_sys::libvgmstream_sfmt_t_LIBVGMSTREAM_SFMT_PCM32 as isize,
    Float = vgmstream_sys::libvgmstream_sfmt_t_LIBVGMSTREAM_SFMT_FLOAT as isize,
}

impl From<vgmstream_sys::libvgmstream_sfmt_t> for SampleType {
    fn from(value: vgmstream_sys::libvgmstream_sfmt_t) -> Self {
        #[allow(clippy::unnecessary_cast)] // libvgmstream_sfmt_t is i32 on windows
        SampleType::from_u32(value as u32).expect("Invalid sample type")
    }
}

impl From<SampleType> for vgmstream_sys::libvgmstream_sfmt_t {
    fn from(value: SampleType) -> Self {
        value.to_u32().expect("Invalid sample type") as vgmstream_sys::libvgmstream_sfmt_t
    }
}

bitflags! {
    /// The `speaker_t` type in vgmstream.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Speaker: u32 {
        /// front left
        const FL = (1 << 0);
        /// front right
        const FR = (1 << 1);
        /// front center
        const FC = (1 << 2);
        /// low frequency effects
        const LFE = (1 << 3);
        /// back left
        const BL = (1 << 4);
        /// back right
        const BR = (1 << 5);
        /// front left center
        const FLC = (1 << 6);
        /// front right center
        const FRC = (1 << 7);
        /// back center
        const BC = (1 << 8);
        /// side left
        const SL = (1 << 9);
        /// side right
        const SR = (1 << 10);

        /// top center
        const TC = (1 << 11);
        /// top front left
        const TFL = (1 << 12);
        /// top front center
        const TFC = (1 << 13);
        /// top front right
        const TFR = (1 << 14);
        /// top back left
        const TBL = (1 << 15);
        /// top back center
        const TBC = (1 << 16);
        /// top back left
        const TBR = (1 << 17);
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ChannelMapping: u32 {
        const MONO              = Speaker::FC.bits();
        const STEREO            = Speaker::FL.bits() | Speaker::FR.bits();
        const _2POINT1          = Speaker::FL.bits() | Speaker::FR.bits() | Speaker::LFE.bits();
        const _2POINT1_XIPH     = Speaker::FL.bits() | Speaker::FR.bits() | Speaker::FC.bits();
        const QUAD              = Speaker::FL.bits() | Speaker::FR.bits() | Speaker::BL.bits()  | Speaker::BR.bits();
        const QUAD_SURROUND     = Speaker::FL.bits() | Speaker::FR.bits() | Speaker::FC.bits()  | Speaker::BC.bits();
        const QUAD_SIDE         = Speaker::FL.bits() | Speaker::FR.bits() | Speaker::SL.bits()  | Speaker::SR.bits();
        const _5POINT0          = Speaker::FL.bits() | Speaker::FR.bits() | Speaker::LFE.bits() | Speaker::BL.bits() | Speaker::BR.bits();
        const _5POINT0_XIPH     = Speaker::FL.bits() | Speaker::FR.bits() | Speaker::FC.bits()  | Speaker::BL.bits() | Speaker::BR.bits();
        const _5POINT0_SURROUND = Speaker::FL.bits() | Speaker::FR.bits() | Speaker::FC.bits()  | Speaker::SL.bits() | Speaker::SR.bits();
        const _5POINT1          = Speaker::FL.bits() | Speaker::FR.bits() | Speaker::FC.bits()  | Speaker::LFE.bits() | Speaker::BL.bits() | Speaker::BR.bits();
        const _5POINT1_SURROUND = Speaker::FL.bits() | Speaker::FR.bits() | Speaker::FC.bits()  | Speaker::LFE.bits() | Speaker::SL.bits() | Speaker::SR.bits();
        const _7POINT0          = Speaker::FL.bits() | Speaker::FR.bits() | Speaker::FC.bits()  | Speaker::LFE.bits() | Speaker::BC.bits() | Speaker::FLC.bits() | Speaker::FRC.bits();
        const _7POINT1          = Speaker::FL.bits() | Speaker::FR.bits() | Speaker::FC.bits()  | Speaker::LFE.bits() | Speaker::BL.bits() | Speaker::BR.bits()  | Speaker::FLC.bits() | Speaker::FRC.bits();
        const _7POINT1_SURROUND = Speaker::FL.bits() | Speaker::FR.bits() | Speaker::FC.bits()  | Speaker::LFE.bits() | Speaker::BL.bits() | Speaker::BR.bits()  | Speaker::SL.bits()  | Speaker::SR.bits();
    }
}

pub struct VgmStream {
    pub(crate) inner: *mut vgmstream_sys::libvgmstream_t,
}

///configures how vgmstream behaves internally when playing a file
#[derive(Debug, Clone)]
pub struct Config {
    // ignore forced (TXTP) config
    pub disable_config_override: bool,
    // must allow manually as some cases a TXTP may set loop forever but client may not handle it
    pub allow_play_forever: bool,

    // keeps looping forever (file must have loop_flag set)
    pub play_forever: bool,
    // ignores loops points
    pub ignore_loop: bool,
    // enables full loops (0..samples) if file doesn't have loop points
    pub force_loop: bool,
    // forces full loops (0..samples) even if file has loop points
    pub really_force_loop: bool,
    // don't fade after N loops and play remaning stream (for files with outros)
    pub ignore_fade: bool,

    /// target loops (values like 1.5 are ok)
    pub loop_count: f64,
    /// fade period after target loops
    pub fade_time: f64,
    /// fade delay after target loops
    pub fade_delay: f64,

    /// forces vgmstream to decode one 2ch+2ch+2ch... 'track' and discard other channels,
    /// where 0 = disabled, 1..N = Nth track
    pub stereo_track: i32,

    /// downmixing if vgmstream's channels are higher than value
    ///
    /// for players that can only handle N channels
    ///
    /// this type of downmixing is very simplistic and **not** recommended
    pub auto_downmix_channels: i32,

    /// forces output buffer to be remixed into some sample format
    pub force_sfmt: SampleType,
}

#[derive(Debug, Clone)]
pub struct Format {
    /* main (always set) */
    /// output channels
    pub channels: i32,
    /// output sample rate
    pub sample_rate: i32,

    /// output buffer's sample type
    pub sample_format: SampleType,
    /// derived from sample_type (pcm16=0x02, float=0x04, etc)
    pub sample_size: i32,

    /* extra info (may be 0 if not known or not relevant) */
    /// standard WAVE bitflags
    pub channel_layout: ChannelMapping,

    /// 0 = none, N = loaded subsong N (1=first)
    pub subsong_index: i32,
    /// 0 = format has no concept of subsongs  
    /// N = has N subsongs  
    /// 1 = format has subsongs, and only 1 for current file
    pub subsong_count: i32,
    /// original file's channels before downmixing (if any)
    pub input_channels: i32,

    /* sample info (may not be used depending on config) */
    /// file's max samples (not final play duration)
    pub stream_samples: i64,
    /// loop start sample
    pub loop_start: i64,
    /// loop end sample
    pub loop_end: i64,
    /// if file loops
    ///
    /// false + defined loops means looping was forcefully disabled
    ///
    /// true + undefined loops means the file loops in a way not representable by loop points
    pub loop_flag: bool,
    /// if file loops forever based on current config (meaning _play never stops)
    pub play_forever: bool,
    /// totals after all calculations (after applying loop/fade/etc config)
    ///
    /// may not be 100% accurate in some cases (check decoder's 'done' flag to stop)
    ///
    /// if `play_forever` is set this is still provided for reference based on non-forever config
    pub play_samples: i64,
    /// average bitrate of the subsong (slightly bloated vs codec_bitrate; incorrect in rare cases)
    ///
    /// not possible / slow to calculate in most cases
    pub stream_bitrate: i32,

    /* descriptions */
    pub codec_name: String,
    pub layout_name: String,
    /// (not internal "tag" metadata)
    pub meta_name: String,
    /// some internal name or representation, not always useful
    pub stream_name: String,

    /* misc */
    /// when reopening subfiles or similar formats without checking other all possible formats
    ///
    /// this value WILL change without warning between vgmstream versions/commits
    pub format_id: i32,
}

impl VgmStream {
    /// Creates a new `libvgmstream_t`.
    ///
    /// # Errors
    ///
    /// Returns `Error::InitializationFailed` if the initialization failed.
    ///
    /// # Example
    ///
    /// Initialize libvgmstream:
    ///
    /// ```no_run
    /// use vgmstream::VgmStream;
    ///
    /// let vgmstream = VgmStream::new().unwrap();
    /// ```
    pub fn new() -> Result<Self, Error> {
        let inner = match unsafe { vgmstream_sys::libvgmstream_init().as_mut() } {
            Some(v) => v,
            None => return Err(Error::InitializationFailed),
        };

        Ok(Self { inner: inner as *mut _ })
    }

    pub fn with_config(config: &Config) -> Result<Self, Error> {
        let vgmstream = Self::new()?;

        let mut config = vgmstream_sys::libvgmstream_config_t {
            disable_config_override: config.disable_config_override,
            allow_play_forever: config.allow_play_forever,

            play_forever: config.play_forever,
            ignore_loop: config.ignore_loop,
            force_loop: config.force_loop,
            really_force_loop: config.really_force_loop,
            ignore_fade: config.ignore_fade,

            loop_count: config.loop_count,
            fade_time: config.fade_time,
            fade_delay: config.fade_delay,

            stereo_track: config.stereo_track,
            auto_downmix_channels: config.auto_downmix_channels,
            force_sfmt: config.force_sfmt.into(),
        };

        unsafe {
            // libvgmstream_setup expects a mutable pointer, but it actually doesn't modify it
            vgmstream_sys::libvgmstream_setup(vgmstream.inner, &mut config as *mut _)
        };

        Ok(vgmstream)
    }

    pub fn open_song<'a>(
        &'a mut self,
        libsf: &'a mut sf::StreamFile<'a>,
        subsong: usize,
    ) -> Result<(), Error> {
        if unsafe {
            vgmstream_sys::libvgmstream_open_stream(self.inner, libsf.inner, subsong as c_int)
        } != 0
        {
            return Err(Error::Generic);
        }

        Ok(())
    }

    pub(crate) fn as_ref(&self) -> Result<&vgmstream_sys::libvgmstream_t, Error> {
        match unsafe { self.inner.as_ref() } {
            Some(i) => Ok(i),
            None => Err(Error::Generic),
        }
    }

    pub(crate) fn as_mut(&mut self) -> Result<&mut vgmstream_sys::libvgmstream_t, Error> {
        match unsafe { self.inner.as_mut() } {
            Some(i) => Ok(i),
            None => Err(Error::Generic),
        }
    }

    pub fn format(&self) -> Result<Format, Error> {
        let format = match unsafe { self.as_ref()?.format.as_ref() } {
            Some(f) => f,
            None => return Err(Error::Generic),
        };

        let sample_format = SampleType::from(format.sample_format);

        let codec_name =
            unsafe { CStr::from_ptr(format.codec_name.as_ptr()) }.to_string_lossy().to_string();
        let layout_name =
            unsafe { CStr::from_ptr(format.layout_name.as_ptr()) }.to_string_lossy().to_string();
        let meta_name =
            unsafe { CStr::from_ptr(format.meta_name.as_ptr()) }.to_string_lossy().to_string();
        let stream_name =
            unsafe { CStr::from_ptr(format.stream_name.as_ptr()) }.to_string_lossy().to_string();

        Ok(Format {
            channels: format.channels,
            sample_rate: format.sample_rate,
            sample_format,
            sample_size: format.sample_size,
            channel_layout: ChannelMapping::from_bits(format.channel_layout)
                .ok_or(Error::InvalidChannelMapping(format.channel_layout))?,
            subsong_index: format.subsong_index,
            subsong_count: format.subsong_count,
            input_channels: format.input_channels,
            stream_samples: format.stream_samples,
            loop_start: format.loop_start,
            loop_end: format.loop_end,
            loop_flag: format.loop_flag,
            play_forever: format.play_forever,
            play_samples: format.play_samples,
            stream_bitrate: format.stream_bitrate,
            codec_name,
            layout_name,
            meta_name,
            stream_name,
            format_id: format.format_id,
        })
    }

    pub fn render(&mut self) -> Result<Vec<u8>, Error> {
        if unsafe { vgmstream_sys::libvgmstream_render(self.inner) } < 0 {
            return Err(Error::Generic);
        }

        let decoder = match unsafe { self.as_mut()?.decoder.as_ref() } {
            Some(d) => d,
            None => return Err(Error::Generic),
        };

        let vgmstream_sys::libvgmstream_decoder_t { buf, buf_bytes, .. } = decoder;

        let buf = unsafe { std::slice::from_raw_parts(*buf as *const u8, *buf_bytes as usize) };

        Ok(buf.to_vec())
    }
}

impl Drop for VgmStream {
    fn drop(&mut self) {
        unsafe { vgmstream_sys::libvgmstream_free(self.inner) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/test.acb");

    #[test]
    fn test_vgmstream() {
        let vgmstream = VgmStream::new().unwrap();
        assert!(!vgmstream.inner.is_null());
    }

    #[test]
    fn test_vgmstream_open_song() {
        let mut vgmstream = VgmStream::new().unwrap();
        let mut sf = StreamFile::open(&vgmstream, ACB_PATH).unwrap();

        assert!(vgmstream.open_song(&mut sf, 0).is_ok());
    }

    #[test]
    fn test_vgmstream_format() {
        let mut vgmstream = VgmStream::new().unwrap();
        let mut sf = StreamFile::open(&vgmstream, ACB_PATH).unwrap();

        vgmstream.open_song(&mut sf, 0).unwrap();
        let format = vgmstream.format().unwrap();

        assert_eq!(format.channels, 2);
        assert_eq!(format.sample_rate, 44100);
        assert_eq!(format.sample_format, SampleType::Pcm16);
        assert_eq!(format.sample_size, 2);
        assert_eq!(format.codec_name, "CRI HCA");
        assert_eq!(format.layout_name, "flat");
        assert!(!format.stream_name.is_empty());
    }

    #[test]
    fn test_vgmstream_render() {
        let mut vgmstream = VgmStream::new().unwrap();
        let mut sf = StreamFile::open(&vgmstream, ACB_PATH).unwrap();

        vgmstream.open_song(&mut sf, 0).unwrap();

        let mut size = 0usize;
        while let Ok(buf) = vgmstream.render() {
            if buf.is_empty() {
                break;
            }

            size += buf.len();
        }

        assert_eq!(size, 3528000);
    }
}
