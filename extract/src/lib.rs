use unity::bundle::UnityFS;
use unity::class::{ClassIDType, TextAsset};

use std::error::Error;
use std::fs::File;
use std::path::PathBuf;

#[derive(Debug, clap::Args)]
#[command(author, version, about, arg_required_else_help(true))]
pub struct ExtractorArgs {
    /// The input path
    #[arg(value_name = "PATH")]
    input: PathBuf,

    /// The output directory
    #[arg(short, long, value_name = "DIR", default_value_os_t = [".", "output"].iter().collect())]
    output: PathBuf,

    /// The number of threads to use
    #[arg(short = 'P', long, value_name = "CPUS", default_value_t = num_cpus::get())]
    parallel: usize,
}

pub fn extract_media(args: &ExtractorArgs) -> Result<(), Box<dyn Error>> {
    let mut f = File::open(&args.input)?;

    log::info!("loading UnityFS bundle: {}", args.input.to_string_lossy());
    let bundle = UnityFS::read(&mut f)?;
    for asset in bundle.assets.iter() {
        for class in asset.classes.iter() {
            if class.class_id() == ClassIDType::TextAsset {
                if let Some(_text_asset) = class.as_any().downcast_ref::<TextAsset>() {
                    todo!()
                }
            }
        }
    }

    Ok(())
}
