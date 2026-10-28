//! Utilities used in this crate.

use std::io::Result;
use std::pin::Pin;
use std::task::{Context, Poll};

use ahash::RandomState;
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, ReadBuf};

pub(crate) type HashMap<K, V> = std::collections::HashMap<K, V, RandomState>;

#[cfg(feature = "asset-ripper")]
pub(crate) type HashSet<T> = std::collections::HashSet<T, RandomState>;

/// Receiver of the progress of a transfer, e.g. a progress bar.
pub trait Progress: Send {
    /// Sets the total length of the transfer.
    fn set_length(&mut self, len: u64);

    /// Advances the progress by `delta` units.
    fn advance(&mut self, delta: u64);
}

pin_project! {
    /// Adapter for [`tokio::io::AsyncRead`] to report progress.
    ///
    /// From [this stackoverflow answer].
    ///
    /// [this stackoverflow answer]: https://stackoverflow.com/a/73809326
    pub struct ProgressReadAdapter<'p, R: AsyncRead> {
        #[pin]
        inner: R,
        progress: Option<&'p mut dyn Progress>,
    }
}

impl<'p, R: AsyncRead> ProgressReadAdapter<'p, R> {
    /// Create a new [`ProgressReadAdapter`] from a [`tokio::io::AsyncRead`]
    /// reader and an optional [`Progress`] receiver.
    pub fn new(inner: R, progress: Option<&'p mut dyn Progress>) -> Self {
        Self { inner, progress }
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

        if result.is_ready()
            && let Some(progress) = this.progress
        {
            progress.advance((after - before) as u64);
        }

        result
    }
}

#[cfg(test)]
static TEST_LOGGER_INIT: std::sync::Once = std::sync::Once::new();

#[cfg(test)]
pub(crate) fn init_test_logger() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    TEST_LOGGER_INIT.call_once(|| {
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().compact().with_file(true).with_line_number(true))
            .with(
                tracing_subscriber::filter::Targets::new()
                    .with_target("mltd", tracing::Level::DEBUG)
                    .with_default(tracing::Level::INFO),
            )
            .init();
    });
}
