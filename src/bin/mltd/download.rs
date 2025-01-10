use std::path::{Path, PathBuf};

use clap::Args;
use futures::{stream, StreamExt};
use human_bytes::human_bytes;
use indicatif::{MultiProgress, MultiProgressAlignment, ProgressDrawTarget};
use mltd::asset::{Asset, AssetInfo, Platform};
use mltd::manifest::Manifest;
use mltd::net::{get_asset_version, latest_asset_version};
use mltd::Error;
use tokio::fs::create_dir_all;

use crate::util::create_progress_bar;

#[derive(Debug, Args)]
#[command(author, version, about, arg_required_else_help(true))]
pub struct DownloaderArgs {
    /// The platform variant to download
    #[arg(value_enum, value_name = "Platform")]
    platform: Platform,

    /// The asset version to download. If not specified, the latest version will be downloaded
    #[arg(long, value_name = "VERSION")]
    asset_version: Option<u64>,

    /// The output path
    #[arg(short, long, value_name = "DIR", default_value_os_t = PathBuf::from("assets"))]
    output_dir: PathBuf,

    /// The number of parallel downloads
    #[arg(short = 'P', long, value_name = "PARALLEL", default_value_t = num_cpus::get())]
    parallel: usize,
}

pub async fn download_task(
    asset_info: AssetInfo,
    output_path: impl AsRef<Path>,
    multi_progress: MultiProgress,
) -> Result<(), Error> {
    let file_name = output_path.as_ref().file_name().unwrap().to_str().unwrap().to_owned();
    let mut progress_bar =
        multi_progress.insert_from_back(1, create_progress_bar().with_message(file_name));

    Asset::download_to_file(&asset_info, Some(output_path.as_ref()), Some(&mut progress_bar))
        .await?;

    multi_progress.remove(&progress_bar);
    multi_progress.insert(0, progress_bar);

    Ok(())
}

pub async fn download_assets(args: &DownloaderArgs) -> Result<(), Error> {
    log::debug!("creating output directory");
    if let Err(e) = create_dir_all(&args.output_dir).await {
        return Err(Error::FileCreate(e));
    }

    let asset_version = match args.asset_version {
        Some(v) => get_asset_version(v).await,
        None => latest_asset_version().await,
    }?;

    let asset_info = AssetInfo {
        filename: asset_version.manifest_filename.clone(),
        platform: args.platform,
        version: asset_version,
    };

    let asset = Asset::download(asset_info.clone(), None).await?;

    let manifest: Manifest = asset.try_into()?;

    log::info!(
        "downloaded manifest version {} (updated at {}), manifest file {}, total asset size {}",
        asset_info.version.version,
        asset_info.version.updated_at,
        asset_info.version.manifest_filename,
        human_bytes(manifest.asset_size() as f64)
    );

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

            let asset_info = AssetInfo { filename: entry.1.clone(), ..asset_info.clone() };
            let output_path = args.output_dir.join(name);

            async move {
                if let Err(e) =
                    tokio::task::spawn(download_task(asset_info, output_path, multi_progress)).await
                {
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
