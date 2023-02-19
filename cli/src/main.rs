use clap::Parser;

#[cfg(feature = "download")]
use mltd_asset_download::*;

#[cfg(feature = "extract")]
use mltd_asset_extract::*;

#[derive(Parser)]
#[command(author, version, about, arg_required_else_help(true))]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity<clap_verbosity_flag::InfoLevel>,
}

#[derive(clap::Subcommand)]
enum Command {
    #[cfg(feature = "download")]
    /// Download assets from MLTD asset server
    Download(DownloaderArgs),

    #[cfg(feature = "extract")]
    /// Extract media from MLTD assets
    Extract(ExtractorArgs),
}

fn main() {
    let args = Cli::parse();

    env_logger::Builder::new()
        .filter_module(env!("CARGO_PKG_NAME"), args.verbose.log_level_filter())
        .format(mltd_core::utils::log_formatter)
        .init();

    match args.command {
        #[cfg(feature = "download")]
        Command::Download(d) => downloader(&d),

        #[cfg(feature = "extract")]
        Command::Extract(e) => extractor(&e),
    }
}
