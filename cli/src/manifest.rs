#![cfg(feature = "manifest")]

use std::fs::read;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};
use mltd_asset_manifest::{get_all_asset_versions, Manifest, Platform, RawManifest};

#[derive(Args)]
#[command(author, version, about, arg_required_else_help(true))]
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

pub fn download_manifest(args: &ManifestDownloadArgs) -> Result<()> {
    let manifest = Manifest::from_version(&args.platform, args.asset_version)?;

    let output = match &args.output {
        Some(output) => output,
        None => &PathBuf::from(&manifest.asset_version.filename),
    };
    manifest.save(output)?;

    Ok(())
}

pub fn diff_manifest(args: &ManifestDiffArgs) -> Result<()> {
    let first_manifest = RawManifest::from_slice(&read(&args.first)?)?;
    let second_manifest = RawManifest::from_slice(&read(&args.second)?)?;

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

pub fn list_manifests() -> Result<()> {
    let versions = get_all_asset_versions()?;

    for version in versions {
        println!(
            "version {:>7}: filename {}, updated at {}",
            version.version, version.filename, version.updated_at
        );
    }

    Ok(())
}

pub fn manifest_main(args: &ManifestArgs) -> Result<()> {
    match &args.command {
        ManifestCommand::Diff(args) => diff_manifest(args),
        ManifestCommand::Download(args) => download_manifest(args),
        ManifestCommand::List => list_manifests(),
    }
}
