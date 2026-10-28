use std::path::{Component, Path, PathBuf};

use ahash::RandomState;
use anyhow::{Result, anyhow};
use indicatif::{ProgressBar, ProgressFinish, ProgressStyle};

pub(crate) type HashSet<T> = std::collections::HashSet<T, RandomState>;

#[inline]
pub(crate) fn create_progress_bar() -> ProgressBar {
    let template = "{msg:<60} {bytes:>12} {binary_bytes_per_sec:>12} {eta:>3} [{wide_bar:.cyan/blue}] {percent:>3}%";

    ProgressBar::new(0).with_finish(ProgressFinish::Abandon).with_style(
        ProgressStyle::with_template(template).expect("invalid template").progress_chars("##-"),
    )
}

/// Joins path that is read from a manifest or an asset bundle to `base`, and makes
/// sure that the result stays inside `base`.
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
