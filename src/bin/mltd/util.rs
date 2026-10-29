#[cfg(any(feature = "unpack", feature = "decode"))]
use std::path::PathBuf;

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

/// Expands the glob `patterns` to the files they match, and warns about the other matches.
#[cfg(any(feature = "unpack", feature = "decode"))]
pub(crate) fn glob_files(patterns: &[String]) -> impl Iterator<Item = PathBuf> + '_ {
    patterns
        .iter()
        .filter_map(|p| match glob::glob(p) {
            Ok(paths) => Some(paths),
            Err(e) => {
                tracing::warn!(pattern = p, error = ?e, "failed to glob");
                None
            }
        })
        .flatten()
        .filter_map(|r| match r {
            Ok(p) => Some(p),
            Err(e) => {
                tracing::warn!(error = ?e, "failed to read directory entry");
                None
            }
        })
        .filter(|p| match p.is_file() {
            true => true,
            false => {
                tracing::warn!(path = %p.display(), "input path is not a file");
                false
            }
        })
}

#[inline]
pub(crate) fn create_progress_bar() -> ProgressBar {
    let template = "{msg:<60} {bytes:>12} {binary_bytes_per_sec:>12} {eta:>3} [{wide_bar:.cyan/blue}] {percent:>3}%";

    ProgressBar::new(0).with_finish(ProgressFinish::Abandon).with_style(
        ProgressStyle::with_template(template).expect("invalid template").progress_chars("##-"),
    )
}
