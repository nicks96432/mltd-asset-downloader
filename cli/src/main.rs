mod manifest;

use anyhow::Result;
use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};
#[cfg(feature = "download")]
use mltd_asset_download::*;
#[cfg(feature = "extract")]
use mltd_asset_extract::*;
use mltd_utils::log_formatter;

#[cfg(feature = "manifest")]
use crate::manifest::*;

#[derive(Parser)]
#[command(author, version, about, arg_required_else_help(true))]
struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[command(flatten)]
    pub verbose: Verbosity<InfoLevel>,
}

#[derive(Subcommand)]
enum Command {
    #[cfg(feature = "download")]
    /// Download assets from MLTD asset server
    Download(DownloaderArgs),

    #[cfg(feature = "extract")]
    /// Extract media from MLTD assets
    Extract(ExtractorArgs),

    #[cfg(feature = "manifest")]
    /// Download manifest from MLTD asset server
    Manifest(ManifestArgs),
}

fn main() -> Result<()> {
    let args = Cli::parse();

    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .format(log_formatter)
        .init();

    match args.command {
        #[cfg(feature = "download")]
        Command::Download(d) => {
            if let Err(e) = download_assets(&d) {
                log::error!("asset download failed: {}", e);
            }
        }

        #[cfg(feature = "extract")]
        Command::Extract(e) => {
            if let Err(e) = extract_media(&e) {
                log::error!("asset extract failed: {}", e);
            }
        }

        #[cfg(feature = "manifest")]
        Command::Manifest(m) => manifest_main(&m)?,
    }

    Ok(())
}
