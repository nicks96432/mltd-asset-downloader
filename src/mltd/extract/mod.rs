pub mod class;
pub mod environment;
pub mod utils;
pub mod version;

use std::error::Error;
#[cfg(not(feature = "debug"))]
use std::fs::{create_dir_all, write};
use std::fs::{read_dir, File};
use std::io::{Cursor, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use clap::{value_parser, Args};
use environment::Environment;
use indicatif::{ProgressBar, ProgressStyle};
use rabex::config::ExtractionConfig;
use rabex::files::{BundleFile, SerializedFile};
use rabex::objects::map;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rayon::ThreadPoolBuilder;

use self::class::asset_bundle::construct_asset_bundle;
use self::class::text_asset::{construct_text_asset, decrypt_text, extract_acb};
use self::class::texture_2d::extract_texture_2d;
use self::environment::{check_file_type, FileType};

#[derive(Debug, Args)]
#[command(author, version, about, arg_required_else_help(true))]
pub struct ExtractorArgs {
    /// The input directory or file
    #[arg(value_name = "PATH", num_args = 1..)]
    input_paths: Vec<PathBuf>,

    /// The output directory
    #[arg(short, long, value_name = "DIR", display_order = 1)]
    #[arg(default_value_os_t = [".", "output"].iter().collect())]
    output: PathBuf,

    /// Audio output format extension
    #[arg(long, value_name = "FORMAT", display_order = 2)]
    #[arg(default_value_t = String::from("wav"))]
    audio_format: String,

    /// Audio output codec
    #[arg(long, value_name = "CODEC", display_order = 2)]
    #[arg(default_value_t = String::from("pcm_s16le"))]
    audio_codec: String,

    /// Arguments to pass to ffmpeg encoder for audio output
    ///
    /// Value should be a list of -arg=value pairs separated by commas
    #[arg(long, value_name = "ARGS", display_order = 2)]
    #[arg(value_parser = parse_key_val::<String, String>, allow_hyphen_values = true)]
    audio_args: Vec<(String, String)>,

    /// Extension for image output
    #[arg(long, value_name = "EXT", display_order = 2)]
    #[arg(default_value_t = String::from("png"))]
    image_ext: String,

    /// Arguments to pass to ffmpeg for image output
    #[arg(long, value_name = "ARGS", display_order = 2, hide_default_value = true)]
    #[arg(default_value_t = String::from(""))]
    image_args: String,

    /// The number of threads to use
    #[arg(short = 'P', long, value_name = "CPUS", display_order = 2)]
    #[arg(value_parser = value_parser!(u32).range(1..=(num_cpus::get() as i64)))]
    #[arg(default_value_t = num_cpus::get() as u32)]
    parallel: u32,
    // TODO: Add option to extract only specific files
}

/// Parse a single key-value pair
fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    if !s.starts_with('-') {
        return Err(format!("invalid -KEY=value: no `-` found in `{s}`").into());
    }
    let pos = s.find('=').ok_or_else(|| format!("invalid -KEY=value: no `=` found in `{s}`"))?;

    Ok((s[1..pos].parse()?, s[pos + 1..].parse()?))
}

pub fn extract_media(args: &ExtractorArgs) -> Result<(), Box<dyn Error>> {
    #[cfg(not(feature = "debug"))]
    create_dir_all(&args.output)?;

    let mut entries = Vec::new();

    for p in &args.input_paths {
        let input_realpath = p.canonicalize()?;

        if input_realpath.is_file() {
            entries.push(input_realpath);
        } else if !input_realpath.is_dir() {
            log::warn!("Input path is not a file or directory");
        } else {
            let input_paths: Vec<_> = read_dir(input_realpath)?.collect();
            let mut input_paths = input_paths
                .into_iter()
                .filter(|r| {
                    if let Err(e) = r {
                        log::warn!("failed to read directory entry: {}", e);
                    }
                    r.is_ok()
                })
                .map(|e| e.unwrap().path())
                .collect();

            entries.append(&mut input_paths);
        }
    }

    extract_files(&entries, args)
}

fn extract_files(input_paths: &[PathBuf], args: &ExtractorArgs) -> Result<(), Box<dyn Error>> {
    log::debug!("setting progress bar");

    let template = "{msg:60} {eta:4} [{wide_bar:.cyan/blue}] {percent:3}%";
    let progress_bar_style = match ProgressStyle::with_template(template) {
        Ok(style) => style,
        Err(_) => {
            log::debug!("invalid progress bar template, using default style");

            ProgressStyle::default_bar()
        }
    }
    .progress_chars("##-");

    let progress_bar = ProgressBar::new(input_paths.len() as u64).with_style(progress_bar_style);
    let finished_count = AtomicU64::new(0);

    log::debug!("setting thread pool");

    let thread_pool_builder = ThreadPoolBuilder::new().num_threads(args.parallel as usize);
    thread_pool_builder.build_global()?;

    input_paths.par_iter().for_each(|entry| {
        if let Err(e) = extract_file(entry, args) {
            log::warn!("failed to extract file: {}", e);
        };

        let cur_finished_count = finished_count.fetch_add(1, Ordering::AcqRel);
        progress_bar.inc(1);
        progress_bar.set_message(format!("{}/{}", cur_finished_count, input_paths.len()));
    });

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
        log::debug!("file type: {:?}, size: {}", file_type, data.len());

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

    #[cfg(not(feature = "debug"))]
    create_dir_all(&output_dir)?;

    for object_info in serialized_file.m_Objects.iter() {
        let mut handler = serialized_file.get_object_handler(object_info, reader);

        let data = handler.get_raw_data()?;
        env.register_object(object_info.m_PathID, data);
    }

    for (i, object_info) in serialized_file.m_Objects.iter().enumerate() {
        log::debug!("extracting object: {} ({})", i, map::CLASS_ID_NAME[&object_info.m_ClassID]);

        let data = env.get_object(object_info.m_PathID).unwrap();

        match object_info.m_ClassID {
            map::TextAsset => {
                let text_asset = construct_text_asset(data, serialized_file)?;
                match text_asset.m_Name.contains(".acb") {
                    true => extract_acb(data, &output_dir, args, serialized_file).unwrap(),
                    false => {
                        let output_path = output_dir.join(text_asset.m_Name).with_extension("txt");
                        log::info!("writing text to {}", output_path.display());

                        #[cfg(not(feature = "debug"))]
                        write(output_path, decrypt_text(text_asset.m_Script.as_bytes())?)?;
                    }
                }
            }
            map::Texture2D => extract_texture_2d(data, &output_dir, args, serialized_file, env)?,
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
