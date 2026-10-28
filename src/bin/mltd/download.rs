use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Result, anyhow};
use clap::{Args, ValueEnum};
use futures_util::StreamExt;
use human_bytes::human_bytes;
use indicatif::{MultiProgress, MultiProgressAlignment, ProgressDrawTarget};
use mltd::Error;
use mltd::asset::{Asset, AssetInfo, Platform};
use mltd::manifest::Manifest;
use mltd::net::{AssetVersion, MatsuriApiClient};
use tokio::fs::create_dir_all;

use crate::util::{BarProgress, create_progress_bar, safe_join};

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

    /// Print help
    #[arg(long, action = clap::ArgAction::HelpShort)]
    help: Option<bool>,
}

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

async fn download_manifest(
    version: Option<u64>,
    platform: Platform,
) -> Result<(Manifest, AssetInfo), Error> {
    let matsuri_client = MatsuriApiClient::new();
    let asset_version = match version {
        Some(v) => matsuri_client.get_asset_version(v).await,
        None => matsuri_client.latest_asset_version().await,
    }?;

    let manifest_info = AssetInfo {
        filename: asset_version.manifest_filename.clone(),
        platform,
        version: asset_version,
    };

    let asset = Asset::download(manifest_info.clone(), None).await?;
    let manifest: Manifest = asset.try_into()?;

    tracing::info!(?manifest_info.version,
        "downloaded manifest, total asset size {}",
        human_bytes(manifest.asset_size() as f64)
    );

    Ok((manifest, manifest_info))
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
        None => download_manifest(args.asset_version, args.platform.into()).await?,
    };

    tracing::trace!("create MultiProgress");
    let multi_progress = MultiProgress::with_draw_target(ProgressDrawTarget::stdout_with_hz(5));
    multi_progress.set_alignment(MultiProgressAlignment::Bottom);
    multi_progress.set_move_cursor(true);

    let main_progress_bar = multi_progress.add(create_progress_bar().with_message("total "));
    main_progress_bar.set_length(manifest.asset_size() as u64);
    main_progress_bar.tick();

    let failed = &AtomicUsize::new(0);

    tracing::debug!("start downloading assets");
    let tasks = futures_util::stream::iter(&*manifest).for_each_concurrent(
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
                    Err(e) => Err(e),
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
