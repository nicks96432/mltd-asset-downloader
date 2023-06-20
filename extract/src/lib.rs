use unity::bundle::UnityFS;

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
    let bundle = UnityFS::read(&mut f)?;

    for (i, asset) in bundle.assets.iter().enumerate() {
        for (j, object) in asset.borrow().objects.values().enumerate() {
            log::debug!("asset {} object {}:\n{:#?}", i, j, object.class(&mut f)?);
        }
    }

    Ok(())
}
