use std::collections::BTreeMap;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use clap::{Args, value_parser};
use futures::lock::Mutex;
use futures::{StreamExt, TryStreamExt, stream};
use image::GenericImageView;
use mltd::ErrorKind;
use mltd::extract::audio::{Encoder, EncoderOutputOptions, MLTD_HCA_KEY};
use mltd::extract::puzzle::solve_puzzle;
use mltd::extract::text::{ENCRYPTED_FILE_EXTENSIONS, UNENCRYPTED_FILE_EXTENSIONS, decrypt_text};
use mltd::net::{AssetInfo, AssetRipper};
use tokio::fs::create_dir_all;
use tokio::io::AsyncWriteExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;

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
    #[arg(value_parser = parse_image_format, default_value = "png")]
    image_ext: image::ImageFormat,

    /// The number of asset to extract at the same time
    #[arg(short = 'P', long, value_name = "CPUS", display_order = 2)]
    #[arg(value_parser = value_parser!(u32).range(1..=(num_cpus::get() as i64)))]
    #[arg(default_value_t = num_cpus::get() as u32)]
    parallel: u32,

    /// The path to the asset ripper executable
    #[arg(long, value_name = "PATH", display_order = 3)]
    #[arg(default_value_os_t = default_asset_ripper_path())]
    asset_ripper_path: PathBuf,
}

/// Parses a single key-value pair
fn parse_key_val(s: &str) -> Result<(String, String)> {
    if !s.starts_with('-') {
        return Err(anyhow!("invalid -KEY=value: no `-` found in `{}`", s));
    }
    let pos = match s.find('=') {
        Some(p) => p,
        None => return Err(anyhow!("invalid -KEY=value: no `=` found in `{}`", s))?,
    };

    Ok((s[1..pos].to_owned(), s[pos + 1..].to_owned()))
}

/// Parses string to image format
fn parse_image_format(s: &str) -> Result<image::ImageFormat> {
    let image_format = match image::ImageFormat::from_extension(s) {
        Some(f) => f,
        None => return Err(anyhow!("invalid image format: {}", s)),
    };

    Ok(image_format)
}

#[inline]
fn default_asset_ripper_path() -> PathBuf {
    let mut path = std::env::current_exe().expect("failed to get current executable path");
    path.pop();
    path.push("AssetRipper");

    path.push(match cfg!(windows) {
        true => "AssetRipper.GUI.Free.exe",
        false => "AssetRipper.GUI.Free",
    });

    path
}

pub async fn extract_files(args: &ExtractorArgs) -> Result<()> {
    ensure_asset_ripper_installed(&args.asset_ripper_path).await?;

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

    log::debug!(
        "creating {} AssetRippers using {}",
        args.parallel,
        args.asset_ripper_path.display()
    );

    let mut port = 50000;
    let mut asset_rippers = Vec::new();
    while asset_rippers.len() < args.parallel as usize {
        match AssetRipper::new(&args.asset_ripper_path, port) {
            Ok(ripper) => {
                log::trace!("created AssetRipper on port {}", port);
                asset_rippers.push(Mutex::new(ripper));
                port += 1;
            }
            Err(e) if e.kind() == ErrorKind::Network => port += 1,
            Err(e) => return Err(e.into()),
        };
    }

    create_dir_all(&args.output).await?;

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

pub async fn ensure_asset_ripper_installed<P>(path: P) -> Result<()>
where
    P: AsRef<Path>,
{
    if path.as_ref().is_file() {
        return Ok(());
    }

    log::info!("AssetRipper is not found at {}", path.as_ref().display());

    print!(concat!(
        "Trying to download AssetRipper. This project is not affiliated with, sponsored, or endorsed by AssetRipper.\n",
        "By downloading, you agree to the terms of the license of AssetRipper.\n",
        "Do you want to install it now? (y/N) "
    ));
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if !input.trim().eq_ignore_ascii_case("y") {
        log::error!("User refused to install AssetRipper");
        return Err(anyhow!("AssetRipper not installed"));
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
) -> Result<()>
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

            let mut infos = Vec::new();
            for asset in assets {
                infos.push(asset_ripper.asset_info(i, j, asset.0).await?);
            }

            extract_assets(i, j, infos, asset_ripper, args).await?;
        }
    }

    Ok(())
}

/// Extracts a single asset according to its class.
async fn extract_assets(
    bundle_no: usize,
    collection_no: usize,
    infos: Vec<AssetInfo>,
    asset_ripper: &mut AssetRipper,
    args: &ExtractorArgs,
) -> Result<()> {
    for info in &infos {
        match info.entry.1.as_str() {
            "TextAsset" => {
                let file_name = PathBuf::from(
                    info.original_path
                        .as_ref()
                        .ok_or_else(|| anyhow!("original path should exist"))?,
                )
                .file_stem()
                .ok_or_else(|| anyhow!("file stem should exist"))?
                .to_str()
                .ok_or_else(|| anyhow!("file stem should be string"))?
                .to_owned();

                if ENCRYPTED_FILE_EXTENSIONS
                    .iter()
                    .chain(UNENCRYPTED_FILE_EXTENSIONS)
                    .any(|ext| file_name.ends_with(ext))
                {
                    extract_binary_asset(info, asset_ripper, args).await?;
                } else {
                    extract_text_asset(bundle_no, collection_no, info, asset_ripper, args).await?;
                }
            }

            // Texture2D requires all relavent Sprite infos to be extracted
            "Texture2D" => {
                let texture_infos =
                    infos.iter().filter(|info| info.entry.1 == "Texture2D").collect::<Vec<_>>();
                let sprite_infos =
                    infos.iter().filter(|info| info.entry.1 == "Sprite").collect::<Vec<_>>();
                for texture_info in texture_infos {
                    extract_texture2d_assets(
                        bundle_no,
                        collection_no,
                        texture_info,
                        &sprite_infos,
                        asset_ripper,
                        args,
                    )
                    .await?;
                }
            }
            "Sprite" => (),      // sprites should be handled in Texture2D extractor
            "AssetBundle" => (), // asset bundles contains bundle information only
            _ => log::warn!("unknown asset type: {}", info.entry.1),
        };
    }

    Ok(())
}

async fn extract_texture2d_assets(
    bundle_no: usize,
    collection_no: usize,
    texture_info: &AssetInfo,
    sprite_infos: &[&AssetInfo],
    asset_ripper: &mut AssetRipper,
    args: &ExtractorArgs,
) -> Result<()> {
    let asset_original_path =
        texture_info.original_path.as_ref().expect("original path of Texture2D should exist");
    let mut asset_output_dir = args.output.join(asset_original_path);
    asset_output_dir.pop();

    let mut image = Cursor::new(Vec::new());
    let mut async_reader = asset_ripper
        .asset_image(bundle_no, collection_no, texture_info.entry.0)
        .await?
        .map_err(std::io::Error::other)
        .into_async_read()
        .compat();

    tokio::io::copy(&mut async_reader, &mut image).await?;
    drop(async_reader);

    let image = image::load_from_memory_with_format(
        image.into_inner().as_slice(),
        image::ImageFormat::Png,
    )?
    .flipv();

    create_dir_all(&asset_output_dir).await?;

    let mut rects = BTreeMap::new();
    for info in sprite_infos {
        if info.entry.1 != "Sprite" {
            continue;
        }

        let json = asset_ripper.asset_json(bundle_no, collection_no, info.entry.0).await?;
        let path_id = json.pointer("/m_RD/m_Texture/m_PathID").unwrap().as_i64().unwrap();
        if path_id != texture_info.entry.0 {
            continue;
        }

        let x = json.pointer("/m_Rect/m_X").unwrap().as_u64().unwrap() as u32;
        let y = json.pointer("/m_Rect/m_Y").unwrap().as_u64().unwrap() as u32;
        let width = json.pointer("/m_Rect/m_Width").unwrap().as_u64().unwrap() as u32;
        let height = json.pointer("/m_Rect/m_Height").unwrap().as_u64().unwrap() as u32;

        let sprite_id = info.entry.2.rsplit_once("_").unwrap().1.parse::<u32>().unwrap();
        let piece = image.view(x, y, width, height);
        rects.insert(sprite_id, piece);
    }

    let images = match solve_puzzle(
        &texture_info.entry.2,
        &image,
        rects.into_values().collect::<Vec<_>>().as_slice(),
    ) {
        Ok(images) => images,
        Err(e) => {
            log::warn!("{}, using original image for {}", e, texture_info.entry.2);
            vec![image]
        }
    };

    stream::iter(images.iter().enumerate())
        .for_each_concurrent(args.parallel as usize, |(i, image)| {
            let asset_output_dir = asset_output_dir.clone();
            async move {
                let path = asset_output_dir.join(format!(
                    "{}_{}.{}",
                    texture_info.entry.2,
                    i,
                    args.image_ext.extensions_str()[0]
                ));
                log::info!("writing image to {}", path.display());

                let mut file = std::fs::File::create(&path).unwrap();
                image.write_to(&mut file, args.image_ext).unwrap();
            }
        })
        .await;

    Ok(())
}

/// Extracts a TextAsset with binary content.
///
/// # Panics
///
/// panics if the asset is not a TextAsset with binary content.
async fn extract_binary_asset(
    info: &AssetInfo,
    asset_ripper: &mut AssetRipper,
    args: &ExtractorArgs,
) -> Result<()> {
    let asset_original_path = &PathBuf::from(
        info.original_path.as_ref().expect("original path of TextAsset should exist"),
    );

    // remove .bytes extension
    let asset_original_filename =
        asset_original_path.with_extension("").file_name().unwrap().to_string_lossy().into_owned();
    let asset_original_extension =
        asset_original_path.with_extension("").extension().unwrap().to_string_lossy().into_owned();

    let mut output_dir = args.output.join(asset_original_path);
    output_dir.pop();
    let output_dir = output_dir;

    create_dir_all(&output_dir).await?;

    let tmpdir = tempfile::tempdir()?;

    // AssetRipper `/Assets/Text` cannot handle binary data, so we have to use export
    // function to get the text data.
    asset_ripper.export_primary(tmpdir.path()).await?;

    let extracted_file_path = tmpdir.path().join(asset_original_path);
    match asset_original_extension {
        // CRI .acb and .awb audio
        n if n == "acb" || n == "awb" => {
            let input_file_path = extracted_file_path.with_extension("");
            // remove .bytes extension for vgmstream
            tokio::fs::rename(&extracted_file_path, &input_file_path).await?;

            // According to https://github.com/vgmstream/vgmstream/blob/master/doc/USAGE.md#decryption-keys,
            // we can specify the decryption key in the .hcakey file so that vgmstream doesn't have
            // to brute-force the key.
            let mut key_file =
                tokio::fs::File::create(extracted_file_path.with_file_name(".hcakey")).await?;
            key_file.write_all(MLTD_HCA_KEY.to_string().as_bytes()).await?;

            let output_prefix =
                input_file_path.file_name().expect("file name should exist").to_os_string();

            log::info!("extracting audio to {}", output_dir.display());

            for subsong_index in 0.. {
                let input_file_path = input_file_path.clone();
                let output_dir = output_dir.clone();

                let output_prefix = output_prefix.clone();
                let audio_codec = args.audio_codec.clone();
                let audio_format = args.audio_format.clone();
                let audio_options = args.audio_args.clone();

                // turn this into a blocking task to run asynchronously
                let result = tokio::task::spawn_blocking(move || {
                    let mut options = ffmpeg_next::Dictionary::new();
                    for (key, value) in &audio_options {
                        options.set(key, value);
                    }
                    if !audio_options.is_empty() {
                        log::trace!("audio options: {:#?}", options);
                    }
                    let mut encoder = Encoder::open(
                        &input_file_path,
                        subsong_index,
                        &output_dir,
                        EncoderOutputOptions {
                            prefix: output_prefix.as_os_str().to_str().unwrap(),
                            codec: &audio_codec,
                            format: &audio_format,
                            options: Some(options),
                        },
                    )?;
                    encoder.encode()
                })
                .await?;

                if result.is_err() {
                    break;
                }
            }
        }
        // AES-192-CBC encrypted text
        n if ENCRYPTED_FILE_EXTENSIONS.contains(&n.as_str()) => {
            let output_path = output_dir.join(asset_original_filename);

            log::info!("extracting text to {}", output_path.display());

            let buf = tokio::fs::read(&extracted_file_path).await?;
            tokio::fs::write(&output_path, decrypt_text(&buf)?).await?;
        }
        // MP4 video
        n if n.ends_with(".mp4") => {
            let output_path = output_dir.join(asset_original_filename);

            log::info!("extracting video to {}", output_path.display());

            tokio::fs::copy(&extracted_file_path, &output_path).await?;
        }
        _ => panic!("this is not a TextAsset with binary content"),
    };

    Ok(())
}

async fn extract_text_asset(
    bundle_no: usize,
    collection_no: usize,
    info: &AssetInfo,
    asset_ripper: &mut AssetRipper,
    args: &ExtractorArgs,
) -> Result<()> {
    let asset_original_path = &PathBuf::from(
        info.original_path.as_ref().expect("original path of TextAsset should exist"),
    );

    let output_path = args.output.join(asset_original_path);
    let output_dir = output_path.parent().unwrap();

    create_dir_all(output_dir).await?;
    let mut f = tokio::fs::File::create(&output_path).await?;

    log::info!("extracting text to {}", output_path.display());

    let mut stream = asset_ripper.asset_text(bundle_no, collection_no, info.entry.0).await?;
    while let Some(item) = stream.next().await {
        f.write_all(item?.as_ref()).await?;
    }

    Ok(())
}
