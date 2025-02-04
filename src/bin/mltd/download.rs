use std::path::{Path, PathBuf};

use clap::Args;
use futures::{stream, StreamExt};
use human_bytes::human_bytes;
use indicatif::{MultiProgress, MultiProgressAlignment, ProgressDrawTarget};
use mltd::asset::{Asset, AssetInfo, Platform};
use mltd::manifest::Manifest;
use mltd::net::AssetVersion;
use mltd::{net, Error};
use tokio::fs::create_dir_all;

use crate::util::create_progress_bar;

#[derive(Debug, Args)]
#[command(about, arg_required_else_help = true, disable_help_flag = true)]
pub struct DownloaderArgs {
    /// The platform variant to download
    #[arg(value_enum, value_name = "Platform")]
    platform: Platform,

    /// The asset version to download, defaults to the latest
    #[arg(long, value_name = "VERSION")]
    asset_version: Option<u64>,

    /// The output path
    #[arg(short, long, value_name = "DIR", default_value_os_t = PathBuf::from("assets"))]
    output_dir: PathBuf,

    /// The number of parallel downloads
    #[arg(short = 'P', long, value_name = "PARALLEL", default_value_t = num_cpus::get())]
    parallel: usize,

    /// The manifest file list to download in MessagePack format
    #[arg(long, value_name = "MANIFEST", requires = "asset_version")]
    manifest: Option<PathBuf>,

    /// Print help
    #[arg(long, action = clap::ArgAction::HelpShort)]
    help: Option<bool>,
}

async fn download_manifest(
    version: Option<u64>,
    platform: Platform,
) -> Result<(Manifest, AssetInfo), Error> {
    let asset_version = match version {
        Some(v) => net::get_asset_version(v).await,
        None => net::latest_asset_version().await,
    }?;

    let manifest_info = AssetInfo {
        filename: asset_version.manifest_filename.clone(),
        platform,
        version: asset_version,
    };

    let asset = Asset::download(manifest_info.clone(), None).await?;
    let manifest: Manifest = asset.try_into()?;

    log::info!(
        "downloaded manifest version {} (updated at {}), manifest file {}, total asset size {}",
        manifest_info.version.version,
        manifest_info.version.updated_at,
        manifest_info.version.manifest_filename,
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
    let mut progress_bar =
        multi_progress.insert_from_back(1, create_progress_bar().with_message(file_name));

    Asset::download_to_file(&asset_info, Some(output_path.as_ref()), Some(&mut progress_bar))
        .await?;

    multi_progress.remove(&progress_bar);
    multi_progress.insert(0, progress_bar);

    Ok(())
}

pub async fn download_assets(args: &DownloaderArgs) -> Result<(), Error> {
    log::debug!("create output directory at {}", args.output_dir.display());
    create_dir_all(&args.output_dir).await?;

    let (manifest, manifest_info) = match &args.manifest {
        Some(path) => {
            let buf = tokio::fs::read(path).await?;
            let manifest: Manifest = Manifest::from_slice(&buf)?;
            drop(buf);

            let filename = path.to_string_lossy().to_string();
            let manifest_info = AssetInfo {
                filename: filename.clone(),
                platform: args.platform,
                version: AssetVersion {
                    manifest_filename: filename,
                    updated_at: String::new(),
                    version: args.asset_version.unwrap(),
                },
            };

            (manifest, manifest_info)
        }
        None => download_manifest(args.asset_version, args.platform).await?,
    };

    log::trace!("create MultiProgress");
    let multi_progress = MultiProgress::with_draw_target(ProgressDrawTarget::stdout_with_hz(5));
    multi_progress.set_alignment(MultiProgressAlignment::Bottom);
    multi_progress.set_move_cursor(true);

    let main_progress_bar = multi_progress.add(create_progress_bar().with_message("total "));
    main_progress_bar.set_length(manifest.asset_size() as u64);
    main_progress_bar.tick();

    log::debug!("start downloading assets");
    let tasks =
        stream::iter(&*manifest).for_each_concurrent(Some(args.parallel), |(name, entry)| {
            let main_progress_bar = main_progress_bar.clone();
            let multi_progress = multi_progress.clone();

            let asset_info = AssetInfo { filename: entry.1.clone(), ..manifest_info.clone() };
            let output_path = args.output_dir.join(name);

            async move {
                let result =
                    tokio::task::spawn(download_task(asset_info, output_path, multi_progress))
                        .await;
                if let Err(e) = &result {
                    log::error!("failed to download {}: {}", entry.1, e);
                }
                if let Err(e) = result.unwrap() {
                    log::warn!("failed to download {}: {}", entry.1, e);
                }

                main_progress_bar.inc(entry.2 as u64);
                main_progress_bar.tick();
            }
        });

    tasks.await;

    if let Err(e) = multi_progress.clear() {
        log::debug!("cannot clear multi_progress, {}", e);
    }

    log::info!("download complete");
    Ok(())
}
