use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use ahash::RandomState;
use anyhow::{Result, anyhow};
use clap::Args;
use futures_util::StreamExt;
use human_bytes::human_bytes;
use indicatif::{MultiProgress, MultiProgressAlignment, ProgressDrawTarget};
use mltd::Error;
use mltd::asset::{Asset, AssetInfo};
use mltd::manifest::{Manifest, ManifestEntry};
use mltd::net::AssetVersion;
use mltd::util::safe_join;
use tokio::fs::create_dir_all;

use crate::util::{BarProgress, ManifestSource, PlatformArg, create_progress_bar, fetch_manifest};

type HashSet<T> = std::collections::HashSet<T, RandomState>;

#[derive(Debug, Args)]
#[command(about, arg_required_else_help = true, disable_help_flag = true)]
pub struct DownloaderArgs {
    /// The platform variant to download
    #[arg(value_enum, value_name = "Platform")]
    platform: PlatformArg,

    /// The asset version to download, defaults to the latest
    #[arg(long, value_name = "VERSION")]
    asset_version: Option<u64>,

    /// The output path
    #[arg(short, long, value_name = "DIR", default_value_os_t = PathBuf::from("assets"))]
    output_dir: PathBuf,

    /// The number of parallel downloads
    #[arg(
        short = 'P',
        long,
        value_name = "CPUS",
        default_value_t = std::thread::available_parallelism().unwrap_or_else(|_| 1.try_into().unwrap())
    )]
    parallel: NonZeroUsize,

    /// The manifest file list to download in MessagePack format
    #[arg(long, value_name = "MANIFEST", requires = "asset_version")]
    manifest: Option<PathBuf>,

    /// Save the downloaded manifest to this path
    #[arg(long, value_name = "PATH", conflicts_with = "manifest")]
    save_manifest: Option<PathBuf>,

    /// Only download assets newer than this asset version or manifest file
    #[arg(long, value_name = "VERSION|MANIFEST")]
    since: Option<ManifestSource>,

    /// Only download assets that matches this glob pattern, can be specified multiple times
    #[arg(short = 'I', long, value_name = "PATTERN")]
    include: Vec<glob::Pattern>,

    /// Print help
    #[arg(long, action = clap::ArgAction::HelpShort)]
    help: Option<bool>,
}

/// Selects the manifest entries to download according to `--since` and `--include`.
async fn select_entries<'a>(
    manifest: &'a Manifest,
    args: &DownloaderArgs,
) -> Result<Vec<(&'a String, &'a ManifestEntry)>> {
    let changed = match &args.since {
        Some(since) => {
            let old = since.load(Some(args.platform.into())).await?;

            let diff = old.diff(manifest);
            tracing::info!(
                added = diff.added.len(),
                updated = diff.updated.len(),
                removed = diff.removed.len(),
                "compared with {since}",
            );

            Some(
                diff.added
                    .keys()
                    .chain(diff.updated.keys())
                    .map(|name| (*name).clone())
                    .collect::<HashSet<String>>(),
            )
        }
        None => None,
    };

    Ok(manifest
        .iter()
        .filter(|(name, _)| changed.as_ref().is_none_or(|c| c.contains(*name)))
        .filter(|(name, _)| args.include.is_empty() || args.include.iter().any(|p| p.matches(name)))
        .collect())
}

async fn download_task<P>(
    asset_info: AssetInfo,
    output_path: P,
    multi_progress: MultiProgress,
) -> Result<(), Error>
where
    P: AsRef<Path>,
{
    let file_name = String::from(output_path.as_ref().file_name().unwrap().to_str().unwrap());
    let progress_bar =
        multi_progress.insert_from_back(1, create_progress_bar().with_message(file_name));

    Asset::download_to_file(
        &asset_info,
        Some(output_path.as_ref()),
        Some(&mut BarProgress(&progress_bar)),
    )
    .await?;

    multi_progress.remove(&progress_bar);
    multi_progress.insert(0, progress_bar);

    Ok(())
}

pub async fn download_assets(args: &DownloaderArgs) -> Result<()> {
    tracing::debug!(output_dir = %args.output_dir.display(), "creating output directory");
    create_dir_all(&args.output_dir).await?;

    let (manifest, manifest_info) = match &args.manifest {
        Some(path) => {
            let buf = tokio::fs::read(path).await?;
            let manifest: Manifest = Manifest::from_slice(&buf)?;
            drop(buf);

            let filename = path.to_string_lossy().to_string();
            let manifest_info = AssetInfo {
                filename: filename.clone(),
                platform: args.platform.into(),
                version: AssetVersion {
                    manifest_filename: filename,
                    updated_at: String::new(),
                    version: args.asset_version.unwrap(),
                },
            };

            (manifest, manifest_info)
        }
        None => {
            fetch_manifest(args.asset_version, args.platform.into(), args.save_manifest.as_deref())
                .await?
        }
    };

    let entries = select_entries(&manifest, args).await?;
    let total_size: usize = entries.iter().map(|(_, entry)| entry.2).sum();

    if entries.is_empty() {
        tracing::info!("no asset to download");
        return Ok(());
    }

    tracing::info!(
        "downloading {} of {} asset(s), size {}",
        entries.len(),
        manifest.len(),
        human_bytes(total_size as f64)
    );

    tracing::trace!("create MultiProgress");
    let multi_progress = MultiProgress::with_draw_target(ProgressDrawTarget::stdout_with_hz(5));
    multi_progress.set_alignment(MultiProgressAlignment::Bottom);
    multi_progress.set_move_cursor(true);

    let main_progress_bar = multi_progress.add(create_progress_bar().with_message("total "));
    main_progress_bar.set_length(total_size as u64);
    main_progress_bar.tick();

    let failed = &AtomicUsize::new(0);

    tracing::debug!("start downloading assets");
    let tasks = futures_util::stream::iter(entries).for_each_concurrent(
        Some(args.parallel.get()),
        |(name, entry)| {
            let main_progress_bar = main_progress_bar.clone();
            let multi_progress = multi_progress.clone();

            let asset_info = AssetInfo { filename: entry.1.clone(), ..manifest_info.clone() };

            async move {
                let result = match safe_join(&args.output_dir, name) {
                    Ok(output_path) => {
                        match tokio::task::spawn(download_task(
                            asset_info,
                            output_path,
                            multi_progress,
                        ))
                        .await
                        {
                            Ok(r) => r.map_err(anyhow::Error::from),
                            Err(e) => Err(anyhow::Error::from(e)),
                        }
                    }
                    Err(e) => Err(anyhow::Error::from(e)),
                };

                if let Err(e) = result {
                    tracing::warn!(name = entry.1, error = ?e, "failed to download asset");
                    failed.fetch_add(1, Ordering::Relaxed);
                }

                main_progress_bar.inc(entry.2 as u64);
                main_progress_bar.tick();
            }
        },
    );

    tasks.await;

    if let Err(e) = multi_progress.clear() {
        tracing::debug!(error = ?e, "cannot clear multi_progress");
    }

    let failed = failed.load(Ordering::Relaxed);
    if failed > 0 {
        return Err(anyhow!("failed to download {failed} asset(s)"));
    }

    tracing::info!("download complete");

    Ok(())
}
