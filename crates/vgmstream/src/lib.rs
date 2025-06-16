//! Rust bindings for low level [`vgmstream_sys`] crate.

mod error;
mod sf;

use std::ffi::{CStr, c_int};
use std::ptr::NonNull;

use bitflags::bitflags;

pub use crate::error::Error;
pub use crate::sf::StreamFile;

pub use vgmstream_sys;

/// Rust version of [`vgmstream_sys::libvgmstream_sfmt_t`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SampleFormat {
    Pcm16 = vgmstream_sys::libvgmstream_sfmt_t_LIBVGMSTREAM_SFMT_PCM16 as isize,
    Pcm24 = vgmstream_sys::libvgmstream_sfmt_t_LIBVGMSTREAM_SFMT_PCM24 as isize,
    Pcm32 = vgmstream_sys::libvgmstream_sfmt_t_LIBVGMSTREAM_SFMT_PCM32 as isize,
    Float = vgmstream_sys::libvgmstream_sfmt_t_LIBVGMSTREAM_SFMT_FLOAT as isize,
}

impl TryFrom<vgmstream_sys::libvgmstream_sfmt_t> for SampleFormat {
    type Error = Error;
    fn try_from(value: vgmstream_sys::libvgmstream_sfmt_t) -> Result<Self, Self::Error> {
        match value {
            vgmstream_sys::libvgmstream_sfmt_t_LIBVGMSTREAM_SFMT_PCM16 => Ok(Self::Pcm16),
            vgmstream_sys::libvgmstream_sfmt_t_LIBVGMSTREAM_SFMT_PCM24 => Ok(Self::Pcm24),
            vgmstream_sys::libvgmstream_sfmt_t_LIBVGMSTREAM_SFMT_PCM32 => Ok(Self::Pcm32),
            vgmstream_sys::libvgmstream_sfmt_t_LIBVGMSTREAM_SFMT_FLOAT => Ok(Self::Float),
            _ => Err(Error::UnknownSampleFormat(value as _)),
        }
    }
}

impl From<SampleFormat> for vgmstream_sys::libvgmstream_sfmt_t {
    fn from(value: SampleFormat) -> Self {
        value as vgmstream_sys::libvgmstream_sfmt_t
    }
}

bitflags! {
    /// The `speaker_t` type in vgmstream.
    ///
    /// Standard WAVEFORMATEXTENSIBLE speaker positions.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    /// The `channel_layout_t` type in vgmstream.
    ///
    /// Typical mappings that metas may use to set channel_layout (but plugin must actually use it)
    /// (in order, so 3ch file could be mapped to FL FR FC or FL FR LFE but not LFE FL FR)
    /// Not too sure about names but no clear standards.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// A wrapper around the [`vgmstream_sys::libvgmstream_t`] pointer.
///
/// vgmstream context/handle.
pub struct VgmStream {
    pub(crate) inner: NonNull<vgmstream_sys::libvgmstream_t>,
}

/// Rust version of [`vgmstream_sys::libvgmstream_config_t`].
///
/// Configures how vgmstream behaves internally when playing a file.
#[derive(Debug, Clone, PartialEq)]
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
    pub force_sfmt: SampleFormat,
}

impl From<Config> for vgmstream_sys::libvgmstream_config_t {
    fn from(value: Config) -> Self {
        Self {
            disable_config_override: value.disable_config_override,
            allow_play_forever: value.allow_play_forever,

            play_forever: value.play_forever,
            ignore_loop: value.ignore_loop,
            force_loop: value.force_loop,
            really_force_loop: value.really_force_loop,
            ignore_fade: value.ignore_fade,

            loop_count: value.loop_count,
            fade_time: value.fade_time,
            fade_delay: value.fade_delay,

            stereo_track: value.stereo_track,
            auto_downmix_channels: value.auto_downmix_channels,
            force_sfmt: value.force_sfmt.into(),
        }
    }
}

/// Rust version of [`vgmstream_sys::libvgmstream_format_t`].
///
/// Current song info, may be copied around (values are info-only)
#[derive(Debug, Clone, PartialEq)]
pub struct Format {
    /* main (always set) */
    /// output channels
    pub channels: i32,
    /// output sample rate
    pub sample_rate: i32,
    /// output buffer's sample type
    pub sample_format: SampleFormat,
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

impl TryFrom<vgmstream_sys::libvgmstream_format_t> for Format {
    type Error = Error;
    fn try_from(value: vgmstream_sys::libvgmstream_format_t) -> Result<Self, Error> {
        let codec_name =
            unsafe { CStr::from_ptr(value.codec_name.as_ptr()) }.to_string_lossy().to_string();
        let layout_name =
            unsafe { CStr::from_ptr(value.layout_name.as_ptr()) }.to_string_lossy().to_string();
        let meta_name =
            unsafe { CStr::from_ptr(value.meta_name.as_ptr()) }.to_string_lossy().to_string();
        let stream_name =
            unsafe { CStr::from_ptr(value.stream_name.as_ptr()) }.to_string_lossy().to_string();

        Ok(Self {
            channels: value.channels,
            sample_rate: value.sample_rate,
            sample_format: SampleFormat::try_from(value.sample_format)?,
            sample_size: value.sample_size,
            channel_layout: ChannelMapping::from_bits(value.channel_layout)
                .ok_or(Error::UnknownChannelMapping(value.channel_layout))?,
            subsong_index: value.subsong_index,
            subsong_count: value.subsong_count,
            input_channels: value.input_channels,
            stream_samples: value.stream_samples,
            loop_start: value.loop_start,
            loop_end: value.loop_end,
            loop_flag: value.loop_flag,
            play_forever: value.play_forever,
            play_samples: value.play_samples,
            stream_bitrate: value.stream_bitrate,
            codec_name,
            layout_name,
            meta_name,
            stream_name,
            format_id: value.format_id,
        })
    }
}

impl VgmStream {
    /// Inits the vgmstream context.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InitializationFailed`] if the initialization failed.
    ///
    /// # Example
    ///
    /// ```
    /// use vgmstream::VgmStream;
    ///
    /// let vgmstream = VgmStream::new().unwrap();
    /// ```
    pub fn new() -> Result<Self, Error> {
        let inner = match unsafe { vgmstream_sys::libvgmstream_init().as_mut() } {
            Some(v) => v,
            None => return Err(Error::VgmStream("libvgmstream_init".to_string())),
        };

        Ok(Self { inner: inner.into() })
    }

    /// Pass config to apply to next [`Self::open`], or current stream if
    /// already loaded and not setup previously.
    ///
    /// * some settings may be ignored in invalid or complex cases
    ///   (ex. TXTP with pre-configured options)
    /// * once config is applied to current stream new [`Self::setup`] calls
    ///   only apply to next [`Self::open`]
    /// * pass [`None`] to clear current config
    /// * remember config may change format info like channels or output format
    ///   (recheck if calling after loading song)
    pub fn setup(&mut self, config: Option<&Config>) {
        match config {
            Some(cfg) => unsafe {
                vgmstream_sys::libvgmstream_setup(
                    self.inner.as_mut(),
                    &mut cfg.clone().into() as *mut _,
                );
            },
            None => unsafe {
                vgmstream_sys::libvgmstream_setup(self.inner.as_mut(), std::ptr::null_mut())
            },
        };
    }

    /// Opens file based on config and prepares it to play if supported.
    ///
    /// * returns < 0 on error (file not recognised, invalid subsong index, etc)
    /// * will close currently loaded song if needed
    /// * libsf (custom IO) is not needed after [`Self::open`] and should be closed,
    ///   as vgmstream re-opens as needed
    /// * subsong can be 1..N or 0 = default/first
    /// * to check if a file has subsongs, [`Self::open`] default and check
    ///   [`Format::subsong_count`]
    pub fn open(&mut self, libsf: &mut StreamFile, subsong: usize) -> Result<(), Error> {
        if unsafe {
            vgmstream_sys::libvgmstream_open_stream(
                self.inner.as_mut(),
                libsf.inner.as_ptr(),
                subsong as c_int,
            )
        } != 0
        {
            return Err(Error::VgmStream("libvgmstream_open_stream".to_string()));
        }

        Ok(())
    }

    /// Closes current song.
    ///
    /// Can still use `self` to open other songs.
    pub fn close(&mut self) {
        unsafe {
            vgmstream_sys::libvgmstream_close_stream(self.inner.as_mut());
        }
    }

    pub(crate) fn as_ref(&self) -> &vgmstream_sys::libvgmstream_t {
        unsafe { self.inner.as_ref() }
    }

    pub(crate) fn as_mut(&mut self) -> &mut vgmstream_sys::libvgmstream_t {
        unsafe { self.inner.as_mut() }
    }

    /// Returns the current file format.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NullPointer`] if `vgmstream->format` is null.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use vgmstream::{StreamFile, VgmStream};
    ///
    /// let mut vgmstream = VgmStream::new().unwrap();
    /// let mut sf = StreamFile::open("path/to/file").unwrap();
    ///
    /// vgmstream.open(&mut sf, 0).unwrap();
    /// println!("{:?}", vgmstream.format().unwrap());
    /// ```
    pub fn format(&self) -> Result<Format, Error> {
        match unsafe { self.as_ref().format.as_ref() } {
            Some(f) => (*f).try_into(),
            None => Err(Error::NullPointer("vgmstream->format".to_string())),
        }
    }

    /// Decodes next batch of samples.
    ///
    /// vgmstream supplies its own buffer, updated on `vgmstream->decoder` attributes
    ///
    /// # Errors
    ///
    /// Returns [`Error::VgmStream`] if decoding failed.
    ///
    /// Returns [`Error::NullPointer`] if `vgmstream->decoder` is null.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use vgmstream::{StreamFile, VgmStream};
    ///
    /// let mut vgmstream = VgmStream::new().unwrap();
    /// let mut sf = StreamFile::open("path/to/file").unwrap();
    ///
    /// vgmstream.open(&mut sf, 0).unwrap();
    /// while let Ok(buf) = vgmstream.render() {
    ///     println!("{}", buf.len());
    /// }
    /// ```
    pub fn render(&mut self) -> Result<&[u8], Error> {
        if unsafe { vgmstream_sys::libvgmstream_render(self.inner.as_mut()) } < 0 {
            return Err(Error::VgmStream("libvgmstream_render".to_string()));
        }

        let decoder = match unsafe { self.as_mut().decoder.as_ref() } {
            Some(d) => d,
            None => return Err(Error::NullPointer("vgmstream->decoder".to_string())),
        };

        let vgmstream_sys::libvgmstream_decoder_t { buf, buf_bytes, .. } = decoder;

        let buf: &[u8] =
            unsafe { std::slice::from_raw_parts(*buf as *const _, *buf_bytes as usize) };

        Ok(buf)
    }

    /// Same as [`Self::render`], but fills some external buffer (also updates lib->decoder attributes)
    ///
    /// It decodes `buf.len() / sample_size / input_channels` samples. Note that may return less than
    /// requested samples. (such as near EOF)
    ///
    /// # Errors
    ///
    /// Returns [`Error::NullPointer`] if `vgmstream->decoder` is null.
    ///
    /// Returns [`Error::VgmStream`] if `libvgmstream_fill` fails.
    ///
    /// # Performance
    ///
    /// This function needs copying around from internal bufs so may be slightly slower;
    /// mainly for cases when you have buf constraints.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use vgmstream::{StreamFile, VgmStream};
    ///
    /// let mut vgmstream = VgmStream::new().unwrap();
    /// let mut sf = StreamFile::open("path/to/file").unwrap();
    ///
    /// vgmstream.open(&mut sf, 0).unwrap();
    /// let mut buf = vec![0u8; 1024];
    /// while let Ok(len) = vgmstream.fill(&mut buf) {
    ///     println!("{}", len);
    /// }
    /// ```
    pub fn fill(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let Format { sample_size, input_channels, .. } = self.format()?;
        if unsafe {
            vgmstream_sys::libvgmstream_fill(
                self.inner.as_mut(),
                buf.as_mut_ptr() as *mut _,
                (buf.len() / sample_size as usize / input_channels as usize) as c_int,
            )
        } < 0
        {
            return Err(Error::VgmStream("libvgmstream_fill".to_string()));
        }

        let decoder = match unsafe { self.as_mut().decoder.as_ref() } {
            Some(d) => d,
            None => return Err(Error::NullPointer("vgmstream->decoder".to_string())),
        };

        let vgmstream_sys::libvgmstream_decoder_t { buf_bytes, .. } = decoder;

        Ok(*buf_bytes as usize)
    }

    /// Gets current position within the song.
    ///
    /// # Errors
    ///
    /// Returns `Error::VgmStream` if getting position failed.
    pub fn get_pos(&mut self) -> Result<i64, Error> {
        let pos = unsafe { vgmstream_sys::libvgmstream_get_play_position(self.inner.as_mut()) };
        if pos < 0 {
            return Err(Error::VgmStream("libvgmstream_get_play_position".to_string()));
        }

        Ok(pos)
    }

    /// Seeks to absolute position.
    ///
    /// Will clamp incorrect values such as seeking before/past playable length.
    pub fn seek(&mut self, pos: i64) {
        unsafe { vgmstream_sys::libvgmstream_seek(self.inner.as_mut(), pos) };
    }

    /// Reset current song.
    pub fn reset(&mut self) {
        unsafe { vgmstream_sys::libvgmstream_reset(self.inner.as_mut()) };
    }
}

impl Drop for VgmStream {
    fn drop(&mut self) {
        unsafe { vgmstream_sys::libvgmstream_free(self.inner.as_mut()) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    All = vgmstream_sys::libvgmstream_loglevel_t_LIBVGMSTREAM_LOG_LEVEL_ALL as isize,
    Debug = vgmstream_sys::libvgmstream_loglevel_t_LIBVGMSTREAM_LOG_LEVEL_DEBUG as isize,
    Info = vgmstream_sys::libvgmstream_loglevel_t_LIBVGMSTREAM_LOG_LEVEL_INFO as isize,
    None = vgmstream_sys::libvgmstream_loglevel_t_LIBVGMSTREAM_LOG_LEVEL_NONE as isize,
}

impl From<LogLevel> for vgmstream_sys::libvgmstream_loglevel_t {
    fn from(level: LogLevel) -> Self {
        level as vgmstream_sys::libvgmstream_loglevel_t
    }
}

/// Defines a global log callback, as vgmstream sometimes communicates format issues to the user.
///
/// * Note that log is currently set globally rather than per [`VgmStream`].
/// * Call with [`LogLevel::None`] to disable current callback.
/// * Call with [`None`] callback to use default stdout callback.
///
pub fn set_log(level: LogLevel, callback: Option<unsafe extern "C" fn(c_int, *const i8)>) {
    unsafe { vgmstream_sys::libvgmstream_set_log(level.into(), callback) };
}

/// Returns a list of supported extensions, such as "adx", "dsp", etc.
/// Mainly for plugins that want to know which extensions are supported.
pub fn extensions() -> &'static [&'static str] {
    static EXTENSIONS: std::sync::LazyLock<Vec<&'static str>> = std::sync::LazyLock::new(|| {
        let mut ext_count = 0;

        unsafe {
            let exts = vgmstream_sys::libvgmstream_get_extensions(&mut ext_count);
            std::slice::from_raw_parts(exts, ext_count as usize)
        }
        .iter()
        .map(|p| unsafe { CStr::from_ptr(*p) }.to_str().unwrap())
        .collect()
    });

    &EXTENSIONS
}

/// Same as [`extensions`], buf returns a list what vgmstream considers "common" formats,
/// such as "wav", "ogg", which usually one doesn't want to associate to vgmstream.
pub fn common_extensions() -> &'static [&'static str] {
    static COMMON_EXTENSIONS: std::sync::LazyLock<Vec<&'static str>> =
        std::sync::LazyLock::new(|| {
            let mut ext_count = 0;

            unsafe {
                let exts = vgmstream_sys::libvgmstream_get_common_extensions(&mut ext_count);
                std::slice::from_raw_parts(exts, ext_count as usize)
            }
            .iter()
            .map(|p| unsafe { CStr::from_ptr(*p) }.to_str().unwrap())
            .collect()
        });

    &COMMON_EXTENSIONS
}

#[cfg(test)]
mod tests {
    use std::cell::LazyCell;

    use super::*;

    const ACB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/test.acb");
    const ACB_FORMAT: LazyCell<Format> = LazyCell::new(|| Format {
        channel_layout: ChannelMapping::STEREO,
        codec_name: "CRI HCA".to_string(),
        input_channels: 2,
        channels: 2,
        play_samples: 882000,
        play_forever: false,
        loop_start: 0,
        loop_end: 0,
        layout_name: "flat".to_string(),
        loop_flag: false,
        meta_name: "CRI HCA header".to_string(),
        sample_format: SampleFormat::Float,
        sample_rate: 44100,
        sample_size: 4,
        stream_bitrate: 117615,
        stream_name: "song3_00test_bgm".to_string(),
        stream_samples: 882000,
        subsong_count: 1,
        subsong_index: 0,
        format_id: 378,
    });

    fn open_vgmstream() -> VgmStream {
        let mut vgmstream = VgmStream::new().unwrap();
        let mut sf = StreamFile::open(ACB_PATH).unwrap();

        vgmstream.open(&mut sf, 0).unwrap();

        vgmstream
    }

    #[test]
    fn test_vgmstream_format() {
        let vgmstream = open_vgmstream();
        let format = vgmstream.format().unwrap();

        assert_eq!(format, ACB_FORMAT.to_owned());
    }

    #[test]
    fn test_vgmstream_setup() {
        let mut vgmstream = VgmStream::new().unwrap();
        let mut sf = StreamFile::open(ACB_PATH).unwrap();

        let config = Config {
            disable_config_override: false,
            allow_play_forever: false,
            play_forever: false,
            ignore_loop: false,
            force_loop: false,
            really_force_loop: false,
            ignore_fade: false,
            loop_count: 0f64,
            fade_time: 0f64,
            fade_delay: 0f64,
            stereo_track: 0,
            auto_downmix_channels: 0,
            force_sfmt: SampleFormat::Float,
        };
        vgmstream.setup(Some(&config));

        vgmstream.open(&mut sf, 0).unwrap();
        assert_eq!(vgmstream.format().unwrap(), ACB_FORMAT.to_owned());
    }

    #[test]
    fn test_vgmstream_render() {
        let mut vgmstream = open_vgmstream();
        let mut size = 0usize;
        while let Ok(buf) = vgmstream.render() {
            if buf.is_empty() {
                break;
            }

            size += buf.len();
        }

        assert_eq!(
            size,
            ACB_FORMAT.sample_size as usize
                * ACB_FORMAT.channels as usize
                * ACB_FORMAT.stream_samples as usize
        );
    }

    #[test]
    fn test_vgmstream_fill() {
        let mut f =
            std::fs::File::create(concat!(env!("CARGO_MANIFEST_DIR"), "/test.wav")).unwrap();
        let mut vgmstream = open_vgmstream();
        let mut buf = [0u8; 4096];
        let mut total_size = 0usize;
        while let Ok(size) = vgmstream.fill(&mut buf) {
            if size == 0 {
                break;
            }

            std::io::Write::write_all(&mut f, &buf).unwrap();
            total_size += size;
        }

        assert_eq!(
            total_size,
            ACB_FORMAT.sample_size as usize
                * ACB_FORMAT.channels as usize
                * ACB_FORMAT.stream_samples as usize
        );
    }
}
