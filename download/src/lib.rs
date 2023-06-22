mod error;

pub use self::error::*;

use indicatif::{MultiProgress, MultiProgressAlignment};
use indicatif::{ProgressBar, ProgressFinish, ProgressStyle};
use mltd_asset_manifest::{Manifest, OsVariant};
use mltd_utils::fetch_asset;
use rayon::current_thread_index;
use rayon::prelude::{ParallelBridge, ParallelIterator};
use rayon::ThreadPoolBuilder;
use std::fs::{create_dir_all, File};
use std::io::copy;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use ureq::AgentBuilder;

#[derive(Debug, clap::Args)]
#[command(author, version, about, arg_required_else_help(true))]
pub struct DownloaderArgs {
    /// The os variant to download
    #[arg(value_enum, value_name = "VARIANT")]
    os_variant: OsVariant,

    /// The output path
    #[arg(short, long, value_name = "DIR", default_value_os_t = Path::new("assets").to_path_buf())]
    output: PathBuf,

    /// The number of threads to use
    #[arg(short = 'P', long, value_name = "CPUS", default_value_t = num_cpus::get())]
    parallel: usize,
}

pub fn download_assets(args: &DownloaderArgs) -> Result<(), DownloadError> {
    log::debug!("getting manifest");
    let manifest = match Manifest::download(&args.os_variant) {
        Ok(m) => m,
        Err(e) => return Err(DownloadError::ManifestError(e)),
    };

    let output_path = args.output.join(manifest.version.to_string());

    log::debug!("creating output directory");
    if let Err(e) = create_dir_all(&args.output) {
        return Err(DownloadError::FileCreateFailed(e));
    }

    log::debug!("creating asset directory");
    if let Err(e) = create_dir_all(&output_path) {
        return Err(DownloadError::FileCreateFailed(e));
    }

    log::debug!("setting progress bar");

    log::trace!("create MultiProgress");
    let multi_progress = MultiProgress::new();
    multi_progress.set_alignment(MultiProgressAlignment::Bottom);
    multi_progress.set_move_cursor(true);

    log::trace!("create ProgressStyle");
    let template =
        "{msg:60} {bytes:12} {binary_bytes_per_sec:12} {eta:4} [{wide_bar:.cyan/blue}] {percent:3}%";
    let progress_bar_style = match ProgressStyle::with_template(template) {
        Ok(style) => style,
        Err(_) => {
            log::debug!("invalid progress bar template, using default style");

            ProgressStyle::default_bar()
        }
    }
    .progress_chars("##-");

    log::trace!("create ProgressBar array");
    let mut progress_bars = Vec::with_capacity(args.parallel);
    for i in 0..args.parallel {
        log::trace!("create ProgressBar {}", i);
        let progress_bar =
            multi_progress.add(ProgressBar::new(0).with_finish(ProgressFinish::Abandon));
        progress_bar.set_style(progress_bar_style.clone());
        progress_bars.push(progress_bar);
    }

    let downloaded_count = AtomicU64::new(0);
    let total_progress_bar = multi_progress.add(
        ProgressBar::new(u64::try_from(manifest.len()).unwrap())
            .with_style(progress_bar_style.clone()),
    );

    log::debug!("setting the number of threads to use");

    let thread_pool_builder = ThreadPoolBuilder::new().num_threads(args.parallel);
    if thread_pool_builder.build_global().is_err() {
        return Err(DownloadError::ThreadPoolError);
    }

    log::debug!("building request agent");
    let agent_builder = AgentBuilder::new()
        .https_only(true)
        .user_agent(args.os_variant.user_agent());
    let agent = agent_builder.build();

    log::debug!("start downloading assets");
    let asset_url_base = format!("/{}/production/2018/{}", manifest.version, args.os_variant);
    let iter = manifest.data[0].iter().par_bridge();

    iter.for_each(|(filename, entry)| {
        let tid = current_thread_index().unwrap_or_default();
        let progress_bar = &progress_bars[tid];
        progress_bar.reset();
        progress_bar.set_length(entry.size);
        progress_bar.set_position(0);
        progress_bar.set_style(progress_bar_style.clone());
        progress_bar.set_message(filename.clone());

        let asset_url = format!("{}/{}", asset_url_base, entry.filename);
        let asset_res = match fetch_asset(&agent, &asset_url) {
            Ok(res) => res,
            Err(e) => {
                multi_progress.suspend(|| log::warn!("cannot download {}: {}", filename, e));
                return;
            }
        };

        let asset_path = output_path.join(filename);

        #[cfg(feature = "debug")]
        let asset_file = Vec::new();

        #[cfg(not(feature = "debug"))]
        let mut asset_file = match File::create(&asset_path) {
            Ok(f) => f,
            Err(e) => {
                multi_progress.suspend(|| {
                    log::warn!("cannot create {}: {}", asset_path.to_string_lossy(), e)
                });
                return;
            }
        };

        let mut writer = progress_bar.wrap_write(&mut asset_file);
        if let Err(e) = copy(&mut asset_res.into_reader(), &mut writer) {
            multi_progress
                .suspend(|| log::warn!("cannot write to {}: {}", asset_path.to_string_lossy(), e));
        }

        let cur_dounloaded_count = downloaded_count.fetch_add(1, Ordering::AcqRel);
        total_progress_bar.inc(entry.size);
        total_progress_bar.set_message(format!(
            "Total ({}/{})",
            cur_dounloaded_count,
            manifest.len()
        ));
    });

    if let Err(e) = multi_progress.clear() {
        log::warn!("cannot clear multi_progress, {}", e);
    }

    log::info!("download complete");
    Ok(())
}
