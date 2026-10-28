#[cfg(any(feature = "download", feature = "extract"))]
use std::path::{Component, Path, PathBuf};

#[cfg(feature = "extract")]
use ahash::RandomState;
#[cfg(any(feature = "download", feature = "extract"))]
use anyhow::{Result, anyhow};
use clap::ValueEnum;
use indicatif::{ProgressBar, ProgressFinish, ProgressStyle};
use mltd::asset::Platform;
use mltd::util::Progress;

#[cfg(feature = "extract")]
pub(crate) type HashSet<T> = std::collections::HashSet<T, RandomState>;

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

/// Joins path that is read from a manifest or an asset bundle to `base`, and makes
/// sure that the result stays inside `base`.
#[cfg(any(feature = "download", feature = "extract"))]
pub(crate) fn safe_join(base: &Path, path: &str) -> Result<PathBuf> {
    let mut joined = base.to_path_buf();
    for component in Path::new(path).components() {
        match component {
            Component::Normal(part) => joined.push(part),
            Component::CurDir => (),
            _ => return Err(anyhow!("unsafe path: {path}")),
        }
    }

    Ok(joined)
}

#[cfg(test)]
#[cfg(any(feature = "download", feature = "extract"))]
mod tests {
    use super::*;

    #[test]
    fn test_safe_join() {
        let base = Path::new("out");

        assert_eq!(safe_join(base, "a/./b.png").unwrap(), Path::new("out/a/b.png"));
        assert!(safe_join(base, "../x").is_err());
        assert!(safe_join(base, "a/../../x").is_err());
        assert!(safe_join(base, "/etc/passwd").is_err());
    }
}
