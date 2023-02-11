use clap::Parser;
use mltd_asset_downloader::*;
use rayon::prelude::{ParallelBridge, ParallelIterator};

const ASSET_URL_BASE: &'static str = "https://td-assets.bn765.com";
const MATSURI_URL: &'static str = "https://api.matsurihi.me/api/mltd/v2";

const UNITY_VERSION: &'static str = "2020.3.32f1";

fn main() {
    let args: Args = Args::parse();
    utils::init_logger(args.verbose.log_level_filter());

    log::debug!("setting the number of threads to use");

    let thread_pool_builder = rayon::ThreadPoolBuilder::new().num_threads(args.parallel);
    if let Err(e) = thread_pool_builder.build_global() {
        utils::error_exit("cannot set the number of threads to use", Some(&e));
    }

    log::debug!("getting version from matsurihi.me");

    let version_url = format!("{}/{}", MATSURI_URL, "version/latest");
    let version_req = ureq::get(&version_url).query("prettyPrint", "false");
    utils::trace_request(&version_req);

    let version_res = version_req.call().unwrap_or_else(|e| {
        utils::error_exit("cannot get the latest version from matsurihi.me", Some(&e))
    });
    log::trace!("");
    utils::trace_response(&version_res);

    let version_json = version_res
        .into_json::<ureq::serde_json::Value>()
        .unwrap_or_else(|e| utils::error_exit("cannot deserialize version json", Some(&e)));

    let manifest_name = version_json["asset"]["indexName"]
        .as_str()
        .unwrap_or_else(|| utils::error_exit("cannot parse asset.indexName", None));
    let manifest_version = version_json["asset"]["version"]
        .as_u64()
        .unwrap_or_else(|| utils::error_exit("cannot parse asset.version", None));

    log::info!(
        "the latest version is {}, manifest file {}",
        manifest_version,
        manifest_name
    );

    log::debug!("creating output directory");

    #[cfg(not(feature = "debug"))]
    utils::create_dir(&args.output);

    log::debug!("reading manifest from MLTD asset server");

    let agent_builder = ureq::AgentBuilder::new()
        .https_only(true)
        .max_idle_connections_per_host(args.parallel)
        .user_agent(args.os_variant.user_agent());

    let agent = agent_builder.build();

    let asset_url_base = format!(
        "{}/{}/production/2018/{}",
        ASSET_URL_BASE, manifest_version, args.os_variant
    );

    let manifest_url = format!("{}/{}", asset_url_base, manifest_name);
    let manifest_req = agent
        .get(&manifest_url)
        .set("Accept", "*/*")
        .set("X-Unity-Version", UNITY_VERSION);
    utils::trace_request(&manifest_req);

    let manifest_res = manifest_req.call().unwrap_or_else(|e| {
        utils::error_exit("cannot get manifest file from MLTD server", Some(&e))
    });
    utils::trace_response(&manifest_res);

    let manifest = rmp_serde::from_read::<_, Manifest>(manifest_res.into_reader())
        .unwrap_or_else(|e| utils::error_exit("cannot decode manifest", Some(&e)));

    let asset_count = manifest[0].len();
    let asset_total_size = manifest[0].iter().map(|entry| entry.1.size).sum();

    log::info!(
        "manifest information: {} assets, {}",
        asset_count,
        indicatif::BinaryBytes(asset_total_size)
    );

    log::debug!("creating asset directory");

    let output_path = args.output.join(manifest_version.to_string());

    #[cfg(not(feature = "debug"))]
    utils::create_dir(&output_path);

    log::debug!("setting progress bar");

    log::trace!("create MultiProgress");
    let multi_progress = indicatif::MultiProgress::new();
    multi_progress.set_alignment(indicatif::MultiProgressAlignment::Bottom);
    multi_progress.set_move_cursor(true);

    log::trace!("create ProgressStyle");
    let template =
        "{msg:60} {bytes:12} {binary_bytes_per_sec:12} {eta:4} [{wide_bar:.cyan/blue}] {percent:3}%";
    let progress_bar_style = match indicatif::ProgressStyle::with_template(template) {
        Ok(style) => style,
        Err(_) => {
            log::debug!("invalid progress bar template, using default style");

            indicatif::ProgressStyle::default_bar()
        }
    }
    .progress_chars("##-");

    log::trace!("create ProgressBar array");
    let mut progress_bars = Vec::<indicatif::ProgressBar>::with_capacity(args.parallel);
    for i in 0..args.parallel {
        log::trace!("create ProgressBar {}", i);
        let progress_bar = multi_progress
            .add(indicatif::ProgressBar::new(0).with_finish(indicatif::ProgressFinish::Abandon));
        progress_bar.set_style(progress_bar_style.clone());
        progress_bars.push(progress_bar);
    }

    let total_progress_bar_style = match indicatif::ProgressStyle::with_template(template) {
        Ok(style) => style,
        Err(_) => {
            log::debug!("invalid progress bar template, using default style");

            indicatif::ProgressStyle::default_bar()
        }
    }
    .progress_chars("##-");

    let downloaded_count = std::sync::atomic::AtomicU64::new(0);
    let total_progress_bar = multi_progress
        .add(indicatif::ProgressBar::new(asset_total_size).with_style(total_progress_bar_style));
    log::debug!("start downloading assets");

    let iter = manifest[0].iter().par_bridge();
    iter.for_each(|(filename, entry)| {
        let tid = rayon::current_thread_index().unwrap_or_default();
        let progress_bar = &progress_bars[tid];
        progress_bar.reset();
        progress_bar.set_length(entry.size);
        progress_bar.set_position(0);
        progress_bar.set_style(progress_bar_style.clone());
        progress_bar.set_message(filename.clone());

        let asset_url = format!("{}/{}", &asset_url_base, entry.filename);
        let asset_req = agent
            .get(&asset_url)
            .set("Accept", "*/*")
            .set("X-Unity-Version", UNITY_VERSION);
        utils::trace_request(&asset_req);

        let result = asset_req.call();
        if let Err(e) = result {
            multi_progress.suspend(|| log::warn!("cannot download {}: {}", filename, e));
            return;
        }

        let asset_res = result.unwrap();
        utils::trace_response(&asset_res);

        let asset_path = output_path.join(filename);

        #[cfg(feature = "debug")]
        let asset_file = Vec::<u8>::new();

        #[cfg(not(feature = "debug"))]
        let asset_file = std::fs::File::create(&asset_path)
            .unwrap_or_else(|e| utils::error_exit("cannot create file", Some(&e)));

        let mut writer = progress_bar.wrap_write(asset_file);

        if let Err(e) = std::io::copy(&mut asset_res.into_reader(), &mut writer) {
            multi_progress
                .suspend(|| log::warn!("cannot write to {}: {}", asset_path.to_string_lossy(), e));
        }

        let cur_dounloaded_count =
            downloaded_count.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        total_progress_bar.inc(entry.size);
        total_progress_bar.set_message(format!("Total ({}/{})", cur_dounloaded_count, asset_count));
    });

    if let Err(e) = multi_progress.clear() {
        log::error!("cannot clear multi_progress, {}", e);
    }

    log::info!("download complete");
}
