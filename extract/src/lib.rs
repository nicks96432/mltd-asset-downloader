mod class;
mod environment;
mod utils;
mod version;

use std::error::Error;
use std::fs::{create_dir_all, read_dir, File};
use std::io::{Cursor, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::exit;

use environment::Environment;
use image::ImageFormat;
use rabex::config::ExtractionConfig;
use rabex::files::{BundleFile, SerializedFile};
use rabex::objects::map;
use utils::MyImageFormat;

use crate::class::acb::extract_acb;
use crate::class::asset_bundle::construct_asset_bundle;
use crate::class::texture_2d::extract_texture_2d;
use crate::environment::{check_file_type, FileType};

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

    /// image output format
    #[arg(long, value_name = "FORMAT", value_enum, default_value_t = MyImageFormat(ImageFormat::WebP))]
    image_format: MyImageFormat,

    /// image output quality
    #[arg(long, value_name = "QUALITY", default_value_t = 100, value_parser = number_range)]
    image_quality: u8,
    // TODO: Add option to extract only specific files
}

fn number_range(s: &str) -> Result<u8, String> {
    let n = s.parse::<i32>().map_err(|e| e.to_string())?;
    if n < 0 || n > 100 {
        return Err(format!("{} is out of range [0, 100]", n));
    }

    Ok(n as u8)
}

pub fn extract_media(args: &ExtractorArgs) -> Result<(), Box<dyn Error>> {
    create_dir_all(&args.output)?;

    let input_realpath = args.input.canonicalize()?;

    if input_realpath.is_file() {
        return extract_file(&input_realpath, &args);
    }

    if !input_realpath.is_dir() {
        log::error!("Input path is not a file or directory");
        exit(1);
    }

    for entry in read_dir(&args.input)? {
        extract_file(&entry?.path(), &args)?;
    }

    Ok(())
}

fn extract_file<P>(input_path: P, args: &ExtractorArgs) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
{
    let mut env = Environment::new();

    log::debug!("loading UnityFS bundle: {}", input_path.as_ref().display());
    let mut f = File::open(input_path)?;

    let config = ExtractionConfig::default();
    let mut bundle = BundleFile::from_reader(&mut f, &config)?;

    for dir_info in &bundle.m_DirectoryInfo {
        let reader = &mut bundle.m_BlockReader;
        reader.seek(SeekFrom::Start(dir_info.offset.try_into()?))?;

        let data = reader.get_ref()
            [dir_info.offset.try_into()?..(dir_info.offset + dir_info.size).try_into()?]
            .to_owned();
        env.register_cab(&dir_info.path, data);
    }

    for dir_info in &bundle.m_DirectoryInfo {
        let reader = &mut bundle.m_BlockReader;
        reader.seek(SeekFrom::Start(dir_info.offset.try_into()?))?;

        let data = env.get_cab(&dir_info.path).unwrap();
        let file_type = check_file_type(&mut Cursor::new(data))?;
        log::debug!("file type: {:?},  size: {}", file_type, data.len());

        if file_type != FileType::AssetsFile {
            continue;
        }

        let serialized_file = SerializedFile::from_reader(reader, &config);
        if let Err(e) = serialized_file {
            log::warn!("failed to parse {} as serialized file: {}", dir_info.path, e.to_string());
            continue;
        }

        let mut serialized_file = serialized_file.unwrap();
        log::trace!("serialized file: {:#?}", serialized_file);

        extract_object(reader, args, &mut serialized_file, &mut env)?;
    }

    Ok(())
}

fn extract_object(
    reader: &mut Cursor<Vec<u8>>,
    args: &ExtractorArgs,
    serialized_file: &mut SerializedFile,
    env: &mut Environment,
) -> Result<(), Box<dyn Error>> {
    let asset_bundle = serialized_file
        .m_Objects
        .iter()
        .find(|&object| object.m_ClassID == map::AssetBundle)
        .ok_or("AssetBundle not found")?;
    let mut asset_bundle_handler = serialized_file.get_object_handler(asset_bundle, reader);
    let asset_bundle_data = asset_bundle_handler.get_raw_data()?;

    let asset_bundle = construct_asset_bundle(&asset_bundle_data, serialized_file)?;

    // asserting that all of the assets in the bundle are of the same path
    let asset_path = &asset_bundle.m_Container.first().ok_or("AssetBundle.m_Container is empty")?.0;
    let asset_path = Path::new(asset_path).parent().unwrap();

    let output_dir = args.output.join(asset_path);

    create_dir_all(&output_dir)?;

    for object_info in serialized_file.m_Objects.iter() {
        let mut handler = serialized_file.get_object_handler(object_info, reader);

        let data = handler.get_raw_data()?;
        env.register_object(object_info.m_PathID, data);
    }

    for (i, object_info) in serialized_file.m_Objects.iter().enumerate() {
        log::debug!("extracting object: {} ({})", i, map::CLASS_ID_NAME[&object_info.m_ClassID]);

        match object_info.m_ClassID {
            map::TextAsset => extract_acb(
                env.get_object(object_info.m_PathID).unwrap(),
                &output_dir,
                args,
                serialized_file,
            )?,
            map::Texture2D => extract_texture_2d(
                env.get_object(object_info.m_PathID).unwrap(),
                &output_dir,
                args,
                serialized_file,
                env,
            )?,
            map::AssetBundle => {
                // this class contains some information about the bundle
            }
            map::Sprite => {
                // sprites will be extracted by Texture2D
            }
            c => log::warn!("this type is not implemented yet: {:?}", map::CLASS_ID_NAME[&c]),
        };
    }

    Ok(())
}
