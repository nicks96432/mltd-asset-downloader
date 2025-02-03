use std::path::{Path, PathBuf};

use clap::{value_parser, Args};
use futures::lock::Mutex;
use futures::{stream, StreamExt};
use mltd::extract::audio::Encoder;
use mltd::extract::text::decrypt_text;
use tokio::fs::create_dir_all;

use mltd::net::{AssetInfo, AssetRipper};
use mltd::Error;

#[derive(Debug, Args)]
#[command(author, version, about, arg_required_else_help(true))]
pub struct ExtractorArgs {
    /// The input files, supports glob patterns
    #[arg(value_name = "PATH", num_args = 1..)]
    input_paths: Vec<String>,

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
    #[arg(value_parser = parse_key_val, allow_hyphen_values = true)]
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

    /// The path to the asset ripper executable
    #[arg(long, value_name = "PATH", display_order = 2)]
    #[arg(default_value_os_t = std::env::current_dir().unwrap().join("AssetRipper").join("AssetRipper.GUI.Free"))]
    asset_ripper_path: PathBuf,
}

/// Parse a single key-value pair
fn parse_key_val(s: &str) -> Result<(String, String), Error> {
    if !s.starts_with('-') {
        return Err(Error::Generic(format!("invalid -KEY=value: no `-` found in `{}`", s)));
    }
    let pos = s
        .find('=')
        .ok_or_else(|| Error::Generic(format!("invalid -KEY=value: no `=` found in `{}`", s)))?;

    Ok((s[1..pos].to_owned(), s[pos + 1..].to_owned()))
}

pub async fn extract_files(args: &ExtractorArgs) -> Result<(), Error> {
    create_dir_all(&args.output).await?;

    let port_start = 50000;
    log::debug!(
        "creating {} asset rippers using {}",
        args.parallel,
        args.asset_ripper_path.display()
    );
    let asset_rippers = (0..args.parallel as u16)
        .map(|i| AssetRipper::new(&args.asset_ripper_path, port_start + i))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|a| Mutex::new(a))
        .collect::<Vec<_>>();

    let files = args
        .input_paths
        .iter()
        .filter_map(|p| match glob::glob(&p) {
            Ok(paths) => Some(paths),
            Err(e) => {
                log::warn!("failed to glob `{}`: {}", p, e);
                None
            }
        })
        .flatten()
        .filter_map(|r| match r {
            Ok(p) => Some(p),
            Err(e) => {
                log::warn!("failed to read directory entry: {}", e);
                None
            }
        })
        .filter(|p| match p.is_file() {
            true => true,
            false => {
                log::warn!("Input path is not a file");
                false
            }
        });

    stream::iter(files.zip(asset_rippers.iter().cycle()))
        .for_each_concurrent(args.parallel as usize, |(path, asset_ripper)| async move {
            let mut asset_ripper = asset_ripper.lock().await;
            if let Err(e) = extract_file(&path, &mut asset_ripper, args).await {
                log::warn!("failed to extract file: {}", e);
            };
        })
        .await;

    Ok(())
}

async fn extract_file<P>(
    path: &P,
    asset_ripper: &mut AssetRipper,
    args: &ExtractorArgs,
) -> Result<(), Error>
where
    P: AsRef<Path> + ?Sized,
{
    log::debug!("extracting file: {}", path.as_ref().display());

    asset_ripper.load(&path).await?;

    let bundles = asset_ripper.bundles().await?;

    for i in 0..bundles.len() {
        let collections = asset_ripper.collections(i).await?;

        for j in 0..collections.len() {
            let assets = asset_ripper.assets(i, j).await?;

            for asset in assets {
                let info = asset_ripper.asset_info(i, j, asset.0).await?;
                extract_asset(i, j, info, asset_ripper, args).await?;
            }
        }
    }

    Ok(())
}

async fn extract_asset(
    _bundle_no: usize,
    _collection_no: usize,
    info: AssetInfo,
    asset_ripper: &mut AssetRipper,
    args: &ExtractorArgs,
) -> Result<(), Error> {
    match info.entry.1.as_str() {
        "TextAsset" => extract_text_asset(info, asset_ripper, args).await?,
        "Texture2D" => return Err(Error::Generic("Not implemented yet".into())),
        "Sprite" => (),      // sprites should be handled in Texture2D extractor
        "AssetBundle" => (), // asset bundles contains bundle information only
        _ => log::warn!("unknown asset type: {}", info.entry.1),
    };

    Ok(())
}

async fn extract_text_asset(
    info: AssetInfo,
    asset_ripper: &mut AssetRipper,
    args: &ExtractorArgs,
) -> Result<(), Error> {
    let asset_output_dir =
        args.output.join(info.original_path.as_ref().unwrap().to_ascii_lowercase());
    let asset_output_dir = asset_output_dir.parent().unwrap();
    create_dir_all(&asset_output_dir).await?;

    if !info.entry.2.ends_with(".acb") && !info.entry.2.ends_with(".gtx") {
        return Err(Error::Generic(format!("unknown text asset: {}", info.entry.2)));
    }

    let tmpdir = tempfile::tempdir()?;

    // AssetRipper `/Assets/Text` cannot handle binary data, so we have to use export
    // function to get the text data.
    asset_ripper.export_primary(tmpdir.path()).await?;

    let file_path = tmpdir.path().join(info.original_path.as_ref().unwrap());
    match &info.entry.2 {
        n if n.ends_with(".acb") => {
            // remove .bytes extension for vgmstream
            std::fs::rename(&file_path, &file_path.with_extension(""))?;
            let file_path = file_path.with_extension("");

            let output_path = args
                .output
                .join(info.original_path.as_ref().unwrap().to_ascii_lowercase())
                .with_extension("")
                .with_extension(&args.audio_format);

            log::info!("extracting audio to {}", output_path.display());

            let audio_codec = args.audio_codec.to_owned();
            let args = args.audio_args.to_owned();

            // turn this into a blocking task to run asynchronously
            tokio::task::spawn_blocking(move || {
                let mut options = ffmpeg_next::Dictionary::new();
                for (key, value) in &args {
                    options.set(&key, &value);
                }
                if !args.is_empty() {
                    log::debug!("audio options: {:#?}", options);
                }
                let mut encoder = Encoder::open(
                    &file_path.clone(),
                    &output_path.clone(),
                    &audio_codec,
                    Some(options),
                )?;
                encoder.encode()
            })
            .await??;
        }
        n if n.ends_with(".gtx") => {
            let output_path = args
                .output
                .join(info.original_path.as_ref().unwrap().to_ascii_lowercase())
                .with_extension("")
                .with_extension("txt");

            log::info!("extracting text to {}", output_path.display());

            let buf = std::fs::read(&file_path)?;
            std::fs::write(&output_path, decrypt_text(&buf)?)?;
        }
        _ => return Err(Error::Generic("this shouldn't happen".to_owned())),
    };

    Ok(())
}
