#[cfg(feature = "download")]
mod download;

#[cfg(feature = "extract")]
mod extract;

mod manifest;
mod util;

use anyhow::Result;
use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use mltd::util::log_formatter;

#[derive(Parser)]
#[command(author, version = env!("VERGEN_GIT_DESCRIBE"), long_version = env!("MLTD_VERSION_LONG"))]
#[command(about, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[command(flatten)]
    pub verbose: Verbosity<InfoLevel>,
}

#[derive(Subcommand)]
enum Command {
    /// Download assets from MLTD asset server
    #[cfg(feature = "download")]
    Download(self::download::DownloaderArgs),

    /// Extract media from MLTD assets
    #[cfg(feature = "extract")]
    Extract(self::extract::ExtractorArgs),

    /// Download manifest from MLTD asset server
    Manifest(self::manifest::ManifestArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    env_logger::Builder::new()
        .filter_module(env!("CARGO_PKG_NAME"), args.verbose.log_level_filter())
        .format(log_formatter)
        .init();

    match args.command {
        #[cfg(feature = "download")]
        Command::Download(d) => self::download::download_assets(&d).await?,

        #[cfg(feature = "extract")]
        Command::Extract(e) => self::extract::extract_files(&e).await?,

        Command::Manifest(m) => self::manifest::manifest_main(&m).await?,
    }

    Ok(())
}
