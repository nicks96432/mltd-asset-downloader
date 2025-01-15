mod error;
mod sf;

use std::ffi::CStr;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

pub use crate::error::Error;
pub use crate::sf::StreamFile;

#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive)]
pub enum SampleType {
    Pcm16 = vgmstream_sys::libvgmstream_sample_t_LIBVGMSTREAM_SAMPLE_PCM16 as isize,
    Pcm24 = vgmstream_sys::libvgmstream_sample_t_LIBVGMSTREAM_SAMPLE_PCM24 as isize,
    Pcm32 = vgmstream_sys::libvgmstream_sample_t_LIBVGMSTREAM_SAMPLE_PCM32 as isize,
    Float = vgmstream_sys::libvgmstream_sample_t_LIBVGMSTREAM_SAMPLE_FLOAT as isize,
}

impl From<vgmstream_sys::libvgmstream_sample_t> for SampleType {
    fn from(value: vgmstream_sys::libvgmstream_sample_t) -> Self {
        SampleType::from_u32(value as u32).expect("Invalid sample type")
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

    /// downmixing if vgmstream's channels are higher than value
    ///
    /// for players that can only handle N channels
    ///
    /// this type of downmixing is very simplistic and **not** recommended
    pub auto_downmix_channels: i32,

    /// forces output buffer to be remixed into PCM16
    pub force_pcm16: bool,
    /// forces output buffer to be remixed into float
    pub force_float: bool,
}

/// configures how vgmstream opens the format
pub struct Options<'a> {
    /// custom IO streamfile that provides reader info for vgmstream
    ///
    /// not needed after _open and should be closed, as vgmstream re-opens its own SFs internally as needed
    pub libsf: &'a sf::StreamFile<'a>,

    /// target subsong (1..N) or 0 = default/first
    ///
    /// to check if a file has subsongs, _open first + check format->total_subsongs
    /// (then _open 2nd, 3rd, etc)
    pub subsong_index: i32,

    /// force a format (for example when loading new subsong of the same archive)
    pub format_id: i32,

    /// forces vgmstream to decode one 2ch+2ch+2ch... 'track' and discard other channels,
    /// where 0 = disabled, 1..N = Nth track
    pub stereo_track: i32,
}

#[derive(Debug, Clone)]
pub struct Format {
    /* main (always set) */
    /// output channels
    pub channels: i32,
    /// output sample rate
    pub sample_rate: i32,

    /// output buffer's sample type
    pub sample_type: SampleType,
    /// derived from sample_type (pcm16=0x02, float=0x04, etc)
    pub sample_size: i32,

    /* extra info (may be 0 if not known or not relevant) */
    /// standard WAVE bitflags
    pub channel_layout: u32,

    /// 0 = none, N = loaded subsong N (1=first)
    pub subsong_index: i32,
    /// 0 = format has no concept of subsongs, N = has N subsongs
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

            auto_downmix_channels: config.auto_downmix_channels,
            force_pcm16: config.force_pcm16,
            force_float: config.force_float,
        };

        unsafe {
            // libvgmstream_setup expects a mutable pointer, but it actually doesn't modify it
            vgmstream_sys::libvgmstream_setup(vgmstream.inner, &mut config as *mut _)
        };

        Ok(vgmstream)
    }

    pub fn open_song<'a>(&'a mut self, options: &'a mut Options<'a>) -> Result<(), Error> {
        let mut options = vgmstream_sys::libvgmstream_options_t {
            libsf: options.libsf.inner,
            subsong_index: options.subsong_index,
            format_id: options.format_id,
            stereo_track: options.stereo_track,
        };

        if unsafe { vgmstream_sys::libvgmstream_open_song(self.inner, &mut options as *mut _) } != 0
        {
            return Err(Error::Generic);
        }

        Ok(())
    }

    pub(crate) fn as_ref<'a>(&'a self) -> Result<&'a vgmstream_sys::libvgmstream_t, Error> {
        match unsafe { self.inner.as_ref() } {
            Some(i) => Ok(i),
            None => Err(Error::Generic),
        }
    }

    pub(crate) fn as_mut<'a>(&'a mut self) -> Result<&'a mut vgmstream_sys::libvgmstream_t, Error> {
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

        let sample_type = SampleType::from(format.sample_type);

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
            sample_type,
            sample_size: format.sample_size,
            channel_layout: format.channel_layout,
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
        let sf = StreamFile::open(&vgmstream, ACB_PATH).unwrap();
        let mut options = Options { libsf: &sf, subsong_index: 0, format_id: 0, stereo_track: 0 };

        assert!(vgmstream.open_song(&mut options).is_ok());
    }

    #[test]
    fn test_vgmstream_format() {
        let mut vgmstream = VgmStream::new().unwrap();
        let sf = StreamFile::open(&vgmstream, ACB_PATH).unwrap();
        let mut options = Options { libsf: &sf, subsong_index: 0, format_id: 0, stereo_track: 0 };

        vgmstream.open_song(&mut options).unwrap();
        let format = vgmstream.format().unwrap();

        assert_eq!(format.channels, 2);
        assert_eq!(format.sample_rate, 44100);
        assert_eq!(format.sample_type, SampleType::Pcm16);
        assert_eq!(format.sample_size, 2);
        assert_eq!(format.codec_name, "CRI HCA");
        assert_eq!(format.layout_name, "flat");
        assert!(!format.stream_name.is_empty());
    }

    #[test]
    fn test_vgmstream_render() {
        let mut vgmstream = VgmStream::new().unwrap();
        let sf = StreamFile::open(&vgmstream, ACB_PATH).unwrap();
        let mut options = Options { libsf: &sf, subsong_index: 0, format_id: 0, stereo_track: 0 };

        vgmstream.open_song(&mut options).unwrap();

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
