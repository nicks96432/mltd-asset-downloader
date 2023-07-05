use unity::bundle::UnityFS;
use unity::class::{ClassIDType, TextAsset};

use std::error::Error;
use std::fs::{create_dir_all, read_dir, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::exit;

#[derive(Debug, clap::Args)]
#[command(author, version, about, arg_required_else_help(true))]
pub struct ExtractorArgs {
    /// The input directory or file
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
    create_dir_all(&args.output)?;
    let input_realpath = args.input.canonicalize()?;

    if input_realpath.is_file() {
        log::debug!("loading UnityFS bundle: {}", input_realpath.display());
        return extract_file(&input_realpath, &args.output);
    }

    if !input_realpath.is_dir() {
        log::error!("Input path is not a file or directory");
        exit(1);
    }

    for entry in read_dir(&args.input)? {
        let entry = entry?;
        log::debug!("loading UnityFS bundle: {}", entry.path().display());
        extract_file(&entry.path(), &args.output)?;
    }

    Ok(())
}

fn extract_file<P>(input_path: P, output_dir: P) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
{
    let mut f = File::open(input_path)?;
    let bundle = UnityFS::read(&mut f)?;
    for asset in bundle.assets.iter() {
        for class in asset.classes.iter() {
            match class.class_id() {
                ClassIDType::TextAsset => {
                    if let Some(text_asset) = class.as_any().downcast_ref::<TextAsset>() {
                        if let Ok(tracks) = acb::to_wav(&text_asset.script) {
                            for track in tracks.iter() {
                                let path = output_dir
                                    .as_ref()
                                    .join(Path::new(&track.name).with_extension("wav"));

                                let mut file = File::create(&path)?;
                                file.write_all(&track.data)?;
                            }
                        }
                    }
                }
                c => {
                    log::warn!("this type is not implemented yet: {:?}", c);
                }
            }
        }
    }

    Ok(())
}
