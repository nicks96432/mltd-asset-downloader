use std::io::Write;
use std::path::{Path, PathBuf};

use clap::{value_parser, Args};
use futures::lock::Mutex;
use futures::{stream, StreamExt};
use mltd::extract::audio::Encoder;
use mltd::extract::text::decrypt_text;
use mltd::net::{AssetInfo, AssetRipper};
use mltd::Error;
use tokio::fs::create_dir_all;

use crate::util::create_progress_bar;

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
    #[arg(long, value_name = "PATH", display_order = 3)]
    #[arg(default_value_os_t = default_asset_ripper_path())]
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

#[inline]
fn default_asset_ripper_path() -> PathBuf {
    let mut path = std::env::current_exe().expect("failed to get current executable path");
    path.pop();
    path.push("AssetRipper");
    path.push("AssetRipper.GUI.Free");

    path
}

pub async fn extract_files(args: &ExtractorArgs) -> Result<(), Error> {
    ensure_asset_ripper_installed(&args.asset_ripper_path).await?;

    create_dir_all(&args.output).await?;

    log::debug!(
        "creating {} AssetRippers using {}",
        args.parallel,
        args.asset_ripper_path.display()
    );

    let mut port_start = 50000;
    let mut asset_rippers = Vec::new();
    while asset_rippers.len() < args.parallel as usize {
        match AssetRipper::new(&args.asset_ripper_path, port_start) {
            Ok(ripper) => {
                asset_rippers.push(Mutex::new(ripper));
                port_start += 1;
            }
            Err(Error::IO(e)) if e.kind() == std::io::ErrorKind::AddrInUse => port_start += 1,
            Err(e) => return Err(e),
        };
    }

    let files = args
        .input_paths
        .iter()
        .filter_map(|p| match glob::glob(p) {
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

pub async fn ensure_asset_ripper_installed<P>(path: P) -> Result<(), Error>
where
    P: AsRef<Path>,
{
    if path.as_ref().is_file() {
        return Ok(());
    }

    log::info!("AssetRipper is not found at {}", path.as_ref().display());

    println!("Trying to download AssetRipper. This project is not affiliated with, sponsored, or endorsed by AssetRipper.");
    println!("By downloading, you agree to the terms of the license of AssetRipper.");

    print!("Do you want to install it now? (y/N) ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if input.trim().to_ascii_lowercase() != "y" {
        log::error!("User refused to install AssetRipper");
        return Err(Error::Generic("AssetRipper not installed".to_owned()));
    }

    let mut path = path.as_ref().to_path_buf();
    path.pop();
    path.pop();

    let mut progress_bar = create_progress_bar().with_message("Downloading AssetRipper...");
    AssetRipper::download_latest_zip(&path, Some(&mut progress_bar)).await?;

    Ok(())
}

/// Extracts a single .unity3d file.
async fn extract_file<P>(
    path: P,
    asset_ripper: &mut AssetRipper,
    args: &ExtractorArgs,
) -> Result<(), Error>
where
    P: AsRef<Path>,
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

/// Extracts a single asset according to its class.
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

/// Extracts a TextAsset.
///
/// Audio assets are binary TextAsset, so they are handled here as well.
async fn extract_text_asset(
    info: AssetInfo,
    asset_ripper: &mut AssetRipper,
    args: &ExtractorArgs,
) -> Result<(), Error> {
    let asset_original_path =
        info.original_path.as_ref().expect("original path of TextAsset should exist");
    let mut asset_output_dir = args.output.join(asset_original_path);
    asset_output_dir.pop();
    create_dir_all(&asset_output_dir).await?;

    if !info.entry.2.ends_with(".acb")
        && !info.entry.2.ends_with(".awb")
        && !info.entry.2.ends_with(".gtx")
    {
        return Err(Error::Generic(format!("unknown text asset: {}", info.entry.2)));
    }

    let tmpdir = tempfile::tempdir()?;

    // AssetRipper `/Assets/Text` cannot handle binary data, so we have to use export
    // function to get the text data.
    asset_ripper.export_primary(tmpdir.path()).await?;

    let file_path = tmpdir.path().join(asset_original_path);
    match &info.entry.2 {
        // CRI .acb and .awb audio
        n if n.ends_with(".acb") || n.ends_with(".awb") => {
            // remove .bytes extension for vgmstream
            tokio::fs::rename(&file_path, file_path.with_extension("")).await?;
            let file_path = file_path.with_extension("");

            let output_path = args
                .output
                .join(asset_original_path)
                .with_extension("")
                .with_extension(&args.audio_format);

            log::info!("extracting audio to {}", output_path.display());

            let audio_codec = args.audio_codec.clone();
            let args = args.audio_args.clone();

            // turn this into a blocking task to run asynchronously
            tokio::task::spawn_blocking(move || {
                let mut options = ffmpeg_next::Dictionary::new();
                for (key, value) in &args {
                    options.set(key, value);
                }
                if !args.is_empty() {
                    log::trace!("audio options: {:#?}", options);
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
        // AES-192-CBC encrypted plot text
        n if n.ends_with(".gtx") => {
            let output_path = args
                .output
                .join(info.original_path.as_ref().unwrap())
                .with_extension("")
                .with_extension("txt");

            log::info!("extracting text to {}", output_path.display());

            let buf = tokio::fs::read(&file_path).await?;
            tokio::fs::write(&output_path, decrypt_text(&buf)?).await?;
        }
        _ => return Err(Error::Generic(String::from("this shouldn't happen"))),
    };

    Ok(())
}
