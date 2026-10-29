use std::convert::Infallible;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use human_bytes::human_bytes;
use indicatif::{ProgressBar, ProgressFinish, ProgressStyle};
use mltd::asset::{Asset, AssetInfo, Platform};
use mltd::manifest::Manifest;
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

/// Downloads the manifest of `version` (the latest if `None`), optionally saving the raw file.
pub(crate) async fn fetch_manifest(
    version: Option<u64>,
    platform: Platform,
    save_path: Option<&Path>,
) -> Result<(Manifest, AssetInfo)> {
    let manifest_info = AssetInfo::manifest(platform, version).await?;

    let asset = Asset::download(manifest_info.clone(), None).await?;

    if let Some(path) = save_path {
        tracing::debug!(path = %path.display(), "saving manifest");
        tokio::fs::write(path, &asset.data)
            .await
            .with_context(|| format!("cannot save manifest to {}", path.display()))?;
    }

    let manifest: Manifest = asset.try_into()?;

    tracing::info!(?manifest_info.version,
        "downloaded manifest, total asset size {}",
        human_bytes(manifest.asset_size() as f64)
    );

    Ok((manifest, manifest_info))
}

/// A manifest given on the command line, either an asset version or a manifest file.
///
/// A plain number is treated as a version, use `./` to refer to a file.
#[derive(Debug, Clone)]
pub(crate) enum ManifestSource {
    Version(u64),
    Path(PathBuf),
}

impl ManifestSource {
    /// Loads the manifest, downloading it if it is a version.
    pub(crate) async fn load(&self, platform: Option<Platform>) -> Result<Manifest> {
        match self {
            Self::Version(version) => match platform {
                Some(platform) => Ok(fetch_manifest(Some(*version), platform, None).await?.0),
                None => bail!("platform is required to download manifest of version {version}"),
            },
            Self::Path(path) => {
                let buf = tokio::fs::read(path)
                    .await
                    .with_context(|| format!("cannot read manifest {}", path.display()))?;

                Ok(Manifest::from_slice(&buf)?)
            }
        }
    }
}

impl FromStr for ManifestSource {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.parse().map_or_else(|_| Self::Path(PathBuf::from(s)), Self::Version))
    }
}

impl Display for ManifestSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Version(version) => write!(f, "version {version}"),
            Self::Path(path) => write!(f, "{}", path.display()),
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
