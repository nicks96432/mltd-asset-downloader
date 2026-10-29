#[cfg(feature = "download")]
mod download;

#[cfg(feature = "extract")]
mod extract;

mod manifest;
mod util;

use std::io::{IsTerminal, stdout};

use anyhow::Result;
use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

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

fn main() -> Result<()> {
    let args = Cli::parse();

    let subscriber = tracing_subscriber::registry().with(
        tracing_subscriber::fmt::layer()
            .compact()
            .with_ansi(stdout().is_terminal())
            .with_file(true)
            .with_line_number(true),
    );

    #[cfg(debug_assertions)]
    let subscriber = subscriber.with(
        tracing_subscriber::fmt::layer()
            .compact()
            .with_ansi(false)
            .with_file(true)
            .with_line_number(true)
            .with_writer(std::fs::OpenOptions::new().append(true).create(true).open("mltd.log")?),
    );

    subscriber
        .with(
            tracing_subscriber::filter::Targets::new()
                .with_target("mltd", args.verbose)
                .with_default(LevelFilter::OFF),
        )
        .init();

    let runtime = tokio::runtime::Runtime::new()?;

    match args.command {
        #[cfg(feature = "download")]
        Command::Download(d) => {
            runtime.block_on(async { self::download::download_assets(&d).await })
        }

        #[cfg(feature = "extract")]
        Command::Extract(e) => {
            // AssetRipper requires running outside of tokio runtime because of reqwest blocking
            self::extract::extract_files(&e)
        }

        Command::Manifest(m) => runtime.block_on(async { self::manifest::manifest_main(&m).await }),
    }?;

    Ok(())
}
