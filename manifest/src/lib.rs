mod error;
mod format;
mod manifest;
mod os_variant;

pub use self::error::*;
pub use self::format::*;
pub use self::manifest::*;
pub use self::os_variant::*;

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, clap::Args)]
#[command(author, version, about, arg_required_else_help(true))]
pub struct ManifestArgs {
    /// Output format of the manifest
    #[arg(short, long, value_enum, default_value_t = ManifestFormat::Msgpack)]
    format: ManifestFormat,

    /// The os variant to download
    #[arg(value_enum, value_name = "VARIANT")]
    os_variant: OsVariant,

    /// The output path
    #[arg(short, long, value_name = "PATH", default_value_os_t = Path::new("manifest").to_path_buf())]
    output: PathBuf,
}

pub fn download_manifest(args: &ManifestArgs) -> Result<(), ManifestError> {
    let manifest = Manifest::download(&args.os_variant)?;
    let buf = match args.format {
        ManifestFormat::Msgpack => manifest.msgpack()?,
        ManifestFormat::JSON => manifest.json()?,
        ManifestFormat::YAML => manifest.yaml()?,
    };

    let mut file = match File::create(&args.output) {
        Ok(f) => f,
        Err(e) => return Err(ManifestError::FileCreateFailed(e)),
    };

    match file.write_all(&buf) {
        Ok(()) => Ok(()),
        Err(e) => Err(ManifestError::FileWriteFailed(e)),
    }
}
