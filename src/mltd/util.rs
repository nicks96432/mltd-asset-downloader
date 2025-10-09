//! Utilities used in this crate.

use std::io::Result;
use std::pin::Pin;
use std::task::{Context, Poll};

use indicatif::ProgressBar;
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, ReadBuf};

pin_project! {
    /// Adapter for [`tokio::io::AsyncRead`] to show progress.
    ///
    /// From [this stackoverflow answer].
    ///
    /// [this stackoverflow answer]: https://stackoverflow.com/a/73809326
    pub struct ProgressReadAdapter<'bar, R: AsyncRead> {
        #[pin]
        inner: R,
        progress_bar: Option<&'bar mut ProgressBar>,
    }
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

        if result.is_ready()
            && let Some(pb) = this.progress_bar
        {
            pb.inc((after - before) as u64);
        }

        result
    }
}

static TEST_LOGGER_INIT: std::sync::Once = std::sync::Once::new();

pub fn init_test_logger() {
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
