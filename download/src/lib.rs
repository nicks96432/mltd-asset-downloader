mod fetch;
mod manifest;

use crate::fetch::{fetch_asset, get_manifest_version};
use indicatif::{
    BinaryBytes, MultiProgress, MultiProgressAlignment, ProgressBar, ProgressFinish, ProgressStyle,
};
use manifest::*;
use mltd_utils::{create_dir, error_exit};
use rayon::current_thread_index;
use rayon::prelude::{ParallelBridge, ParallelIterator};
use rayon::ThreadPoolBuilder;
use std::fs::File;
use std::io::copy;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use ureq::AgentBuilder;

#[derive(Debug, clap::Args)]
#[command(author, version, about, arg_required_else_help(true))]
pub struct DownloaderArgs {
    /// Keep the manifest file in the output directory
    #[arg(long, default_value_t = false)]
    keep_manifest: bool,

    /// The output path
    #[arg(short, long, value_name = "DIR", default_value_os_t = [".", "assets"].iter().collect())]
    output: PathBuf,

    /// The os variant to download
    #[arg(value_enum, value_name = "VARIANT")]
    os_variant: OsVariant,

    /// The number of threads to use
    #[arg(short = 'P', long, value_name = "CPUS", default_value_t = num_cpus::get())]
    parallel: usize,
}

pub fn downloader(args: &DownloaderArgs) {
    log::debug!("getting version from matsurihi.me");

    let (manifest_name, manifest_version) = get_manifest_version().unwrap_or_else(|e| {
        error_exit(
            "cannot get the latest version from matsurihi.me",
            Some(e.as_ref()),
        )
    });

    log::info!(
        "the latest version is {}, manifest file {}",
        manifest_version,
        manifest_name
    );

    log::debug!("reading manifest from MLTD asset server");

    let agent_builder = AgentBuilder::new()
        .https_only(true)
        .max_idle_connections_per_host(args.parallel)
        .user_agent(args.os_variant.user_agent());

    let agent = agent_builder.build();

    let asset_url_base = format!("/{}/production/2018/{}", manifest_version, args.os_variant);

    let manifest_url = format!("{}/{}", asset_url_base, manifest_name);
    let manifest_res = fetch_asset(&agent, &manifest_url)
        .unwrap_or_else(|e| error_exit("cannot get manifest file from MLTD server", Some(&e)));

    log::debug!("creating output directory");

    #[cfg(not(feature = "debug"))]
    create_dir(&args.output);

    let manifest = rmp_serde::from_read::<_, Manifest>(manifest_res.into_reader())
        .unwrap_or_else(|e| error_exit("cannot decode manifest", Some(&e)));

    let asset_count = manifest[0].len();
    let asset_total_size = manifest[0].iter().map(|entry| entry.1.size).sum();

    log::info!(
        "manifest information: {} assets, {}",
        asset_count,
        BinaryBytes(asset_total_size)
    );

    log::debug!("creating asset directory");

    let output_path = args.output.join(manifest_version.to_string());

    #[cfg(not(feature = "debug"))]
    create_dir(&output_path);

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

    let total_progress_bar_style = match ProgressStyle::with_template(template) {
        Ok(style) => style,
        Err(_) => {
            log::debug!("invalid progress bar template, using default style");

            ProgressStyle::default_bar()
        }
    }
    .progress_chars("##-");

    let downloaded_count = AtomicU64::new(0);
    let total_progress_bar =
        multi_progress.add(ProgressBar::new(asset_total_size).with_style(total_progress_bar_style));

    log::debug!("setting the number of threads to use");

    let thread_pool_builder = ThreadPoolBuilder::new().num_threads(args.parallel);
    if let Err(e) = thread_pool_builder.build_global() {
        error_exit("cannot set the number of threads to use", Some(&e));
    }

    log::debug!("start downloading assets");

    let iter = manifest[0].iter().par_bridge();
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
        let asset_file = File::create(&asset_path)
            .unwrap_or_else(|e| error_exit("cannot create file", Some(&e)));

        let mut writer = progress_bar.wrap_write(asset_file);

        if let Err(e) = copy(&mut asset_res.into_reader(), &mut writer) {
            multi_progress
                .suspend(|| log::warn!("cannot write to {}: {}", asset_path.to_string_lossy(), e));
        }

        let cur_dounloaded_count = downloaded_count.fetch_add(1, Ordering::AcqRel);
        total_progress_bar.inc(entry.size);
        total_progress_bar.set_message(format!("Total ({}/{})", cur_dounloaded_count, asset_count));
    });

    if let Err(e) = multi_progress.clear() {
        log::warn!("cannot clear multi_progress, {}", e);
    }

    log::info!("download complete");
}
