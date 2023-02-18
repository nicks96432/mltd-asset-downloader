use clap::Parser;
use mltd_asset_download::*;
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
    /// Download assets from MLTD asset server
    Download(DownloaderArgs),

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
        Command::Download(d) => downloader(&d),
        Command::Extract(e) => extractor(&e),
    }
}
