//! Utilities used in this crate.

use std::io::{Result, Write};
use std::pin::Pin;
use std::task::{Context, Poll};

use env_logger::fmt::Formatter;
use indicatif::ProgressBar;
use log::Record;
use pin_project::pin_project;
use tokio::io::{AsyncRead, ReadBuf};

/// Custom log formatter used in this crate.
pub fn log_formatter(buf: &mut Formatter, record: &Record) -> Result<()> {
    let color_code = match record.level() {
        log::Level::Error => 1, // red
        log::Level::Warn => 3,  // yellow
        log::Level::Info => 4,  // blue
        log::Level::Debug => 2, // green
        log::Level::Trace => 7, // white
    };
    let space = match record.level().as_str().len() {
        4 => " ",
        _ => "",
    };
    let level = record.level().as_str();
    let target = record.target();
    let timestamp = buf.timestamp_micros();
    let body = record.args();

    writeln!(
        buf,
        "[\x1b[3{}m{}\x1b[0m]{} {} {} - {}",
        color_code, level, space, timestamp, target, body
    )
}

/// Adapter for [`tokio::io::AsyncRead`] to show progress.
///
/// From [this stackoverflow answer].
///
/// [this stackoverflow answer]: https://stackoverflow.com/a/73809326
#[pin_project]
pub struct ProgressReadAdapter<'bar, R: AsyncRead> {
    #[pin]
    inner: R,
    progress_bar: Option<&'bar mut ProgressBar>,
}

impl<'bar, R: AsyncRead> ProgressReadAdapter<'bar, R> {
    /// Create a new [`ProgressReadAdapter`] from a [`tokio::io::AsyncRead`]
    /// reader and an optional [`indicatif::ProgressBar`].
    pub fn new(inner: R, progress_bar: Option<&'bar mut ProgressBar>) -> Self {
        Self { inner, progress_bar }
    }
}

impl<R: AsyncRead> AsyncRead for ProgressReadAdapter<'_, R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        let this = self.project();

        let before = buf.filled().len();
        let result = this.inner.poll_read(cx, buf);
        let after = buf.filled().len();

        if result.is_ready() {
            if let Some(pb) = this.progress_bar {
                pb.inc((after - before) as u64);
            }
        }

        result
    }
}

#[cfg(test)]
macro_rules! init_test_logger {
    () => {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_module(env!("CARGO_PKG_NAME"), log::LevelFilter::Debug)
            .format(crate::util::log_formatter)
            .try_init();
    };
}

#[cfg(test)]
pub(crate) use init_test_logger;

#[cfg(test)]
pub(crate) mod test_util {
    use std::io::Cursor;

    use rand::distributions::uniform::{SampleRange, SampleUniform};
    use rand::{thread_rng, Rng, SeedableRng};
    use rand_xoshiro::Xoshiro256PlusPlus as MyRng;

    pub fn rand_ascii_string(len: usize) -> Cursor<Vec<u8>> {
        let mut rng = MyRng::from_rng(thread_rng()).unwrap();
        let mut buf = vec![0u8; len];
        for byte in buf.iter_mut().take(len) {
            *byte = u8::try_from(rng.gen_range(0x33..0x7f)).unwrap(); // printable ascii
        }
        buf.push(0u8);

        Cursor::new(buf)
    }

    pub fn rand_range<T, R>(range: R) -> T
    where
        T: SampleUniform,
        R: SampleRange<T>,
    {
        let mut rng = MyRng::from_rng(thread_rng()).unwrap();

        rng.gen_range(range)
    }
}
