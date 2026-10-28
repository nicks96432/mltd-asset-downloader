use clap::ValueEnum;
use indicatif::{ProgressBar, ProgressFinish, ProgressStyle};
use mltd::asset::Platform;
use mltd::util::Progress;

/// Platform variant accepted on the command line.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum PlatformArg {
    /// Android platform
    Android,

    /// iOS platform
    #[allow(clippy::upper_case_acronyms)]
    IOS,
}

impl From<PlatformArg> for Platform {
    fn from(value: PlatformArg) -> Self {
        match value {
            PlatformArg::Android => Self::Android,
            PlatformArg::IOS => Self::IOS,
        }
    }
}

/// Reports the progress of a transfer to a [`ProgressBar`].
pub(crate) struct BarProgress<'a>(pub &'a ProgressBar);

impl Progress for BarProgress<'_> {
    fn set_length(&mut self, len: u64) {
        self.0.set_length(len);
    }

    fn advance(&mut self, delta: u64) {
        self.0.inc(delta);
    }
}

#[inline]
pub(crate) fn create_progress_bar() -> ProgressBar {
    let template = "{msg:<60} {bytes:>12} {binary_bytes_per_sec:>12} {eta:>3} [{wide_bar:.cyan/blue}] {percent:>3}%";

    ProgressBar::new(0).with_finish(ProgressFinish::Abandon).with_style(
        ProgressStyle::with_template(template).expect("invalid template").progress_chars("##-"),
    )
}
