use std::fs::read;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use mltd::asset::{Asset, AssetInfo, Platform};
use mltd::manifest::Manifest;
use mltd::net::{get_all_asset_versions, get_asset_version, latest_asset_version};
use mltd::Error;

use crate::util::create_progress_bar;

#[derive(Args)]
#[command(about, arg_required_else_help(true))]
pub struct ManifestArgs {
    #[command(subcommand)]
    pub command: ManifestCommand,
}

#[derive(Subcommand)]
pub enum ManifestCommand {
    /// Compute the difference between two manifests
    Diff(ManifestDiffArgs),

    /// Download a manifest
    Download(ManifestDownloadArgs),

    /// List available manifests
    List,
}

#[derive(Args)]
pub struct ManifestDiffArgs {
    /// The first manifest
    #[arg(value_name = "PATH")]
    first: PathBuf,

    /// The second manifest
    #[arg(value_name = "PATH")]
    second: PathBuf,
}

#[derive(Args)]
pub struct ManifestDownloadArgs {
    /// The platform variant to download
    #[arg(value_enum, value_name = "VARIANT")]
    pub platform: Platform,

    /// The manifest version to download, defaults to the latest version
    #[arg(long, value_name = "VERSION")]
    pub asset_version: Option<u64>,

    /// The output path, defaults to the original manifest file name
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<PathBuf>,
}

pub async fn download_manifest(args: &ManifestDownloadArgs) -> Result<(), Error> {
    let asset_version = match args.asset_version {
        None => latest_asset_version().await,
        Some(v) => get_asset_version(v).await,
    }?;

    let asset_info = AssetInfo {
        filename: asset_version.manifest_filename.clone(),
        platform: args.platform,
        version: asset_version,
    };

    let mut progress_bar = create_progress_bar().with_message(asset_info.filename.clone());

    Asset::download_to_file(&asset_info, args.output.as_deref(), Some(&mut progress_bar)).await?;

    Ok(())
}

pub fn diff_manifest(args: &ManifestDiffArgs) -> Result<(), Error> {
    let first_manifest = Manifest::from_slice(&read(&args.first)?)?;
    let second_manifest = Manifest::from_slice(&read(&args.second)?)?;

    let diff = first_manifest.diff(&second_manifest);

    let added_bytes = diff.added.values().fold(0, |acc, v| acc + v.2);
    let updated_bytes = diff.updated.values().fold(0, |acc, v| acc + v.2);
    let removed_bytes = diff.removed.values().fold(0, |acc, v| acc + v.2);

    let added_files_count = diff.added.len();
    let updated_files_count = diff.updated.len();
    let removed_files_count = diff.removed.len();

    println!("added {} files, {} bytes", added_files_count, added_bytes);
    println!("updated {} files, {} bytes", updated_files_count, updated_bytes);
    println!("removed {} files, {} bytes", removed_files_count, removed_bytes);

    Ok(())
}

pub async fn list_manifests() -> Result<(), Error> {
    let versions = get_all_asset_versions().await?;

    for version in versions {
        println!(
            "version {:>7}: filename {}, updated at {}",
            version.version, version.manifest_filename, version.updated_at
        );
    }

    Ok(())
}

pub async fn manifest_main(args: &ManifestArgs) -> Result<(), Error> {
    match &args.command {
        ManifestCommand::Diff(args) => diff_manifest(args),
        ManifestCommand::Download(args) => download_manifest(args).await,
        ManifestCommand::List => list_manifests().await,
    }
}
