#![cfg(feature = "manifest")]

use std::path::PathBuf;

use mltd_asset_manifest::{Manifest, ManifestError, Platform};

#[derive(Debug, clap::Args)]
#[command(author, version, about, arg_required_else_help(true))]
pub struct ManifestArgs {
    /// The os variant to download
    #[arg(value_enum, value_name = "VARIANT")]
    os_variant: Platform,

    /// The manifest version to download
    #[arg(long, value_name = "VERSION")]
    asset_version: Option<u64>,

    /// The output path
    #[arg(short, long, value_name = "PATH", default_value_os_t = PathBuf::from("manifest.msgpack"))]
    output: PathBuf,
}

pub fn download_manifest(args: &ManifestArgs) -> Result<(), ManifestError> {
    let manifest = Manifest::from_version(&args.os_variant, args.asset_version)?;

    manifest.save(&args.output)
}
