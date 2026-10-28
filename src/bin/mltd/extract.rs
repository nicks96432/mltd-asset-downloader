use std::collections::BTreeSet;
use std::fs::{File, create_dir_all};
use std::io::{BufReader, BufWriter, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Result, anyhow};
use clap::{Args, ValueEnum};
use mltd::extract::ENCRYPTED_TEXT_EXTENSIONS;
use mltd::unity::Extractor;
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::util::safe_join;
#[cfg(feature = "decode-audio")]
use {
    crate::util::HashSet,
    ahash::HashSetExt,
    mltd::extract::audio::{Encoder, EncoderOutputOptions, MLTD_HCA_KEY},
};
#[cfg(feature = "process-image")]
use {
    image::DynamicImage, image::GenericImageView, mltd::extract::puzzle::solve_puzzle,
    mltd::unity::AssetInfo, std::collections::BTreeMap,
};
#[cfg(feature = "decrypt-text")]
use {mltd::extract::text::decrypt_text, std::io::Read};

#[derive(Debug, Args)]
#[cfg(feature = "decode-audio")]
pub struct AudioArgs {
    /// Audio output format extension
    #[arg(long, value_name = "FORMAT", display_order = 2)]
    #[arg(default_value_t = String::from("wav"))]
    pub audio_format: String,

    /// Audio output codec
    #[arg(long, value_name = "CODEC", display_order = 2)]
    #[arg(default_value_t = String::from("pcm_s16le"))]
    pub audio_codec: String,

    /// Arguments to pass to ffmpeg encoder for audio output
    ///
    /// Value should be a list of -arg=value pairs separated by commas
    #[arg(long, value_name = "ARGS", display_order = 2)]
    #[arg(value_parser = parse_key_val, allow_hyphen_values = true)]
    pub audio_args: Vec<(String, String)>,
}

/// Library used to read Unity asset bundles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Backend {
    /// External AssetRipper process
    #[cfg(feature = "asset-ripper")]
    AssetRipper,

    /// Built-in pure Rust parser (experimental)
    #[cfg(feature = "rabex")]
    Rabex,
}

impl Default for Backend {
    fn default() -> Self {
        #[cfg(feature = "asset-ripper")]
        return Self::AssetRipper;

        #[cfg(all(feature = "rabex", not(feature = "asset-ripper")))]
        return Self::Rabex;
    }
}

#[derive(Debug, Args)]
#[command(author, version, about, arg_required_else_help(true))]
pub struct ExtractorArgs {
    /// The library used to read the asset bundles
    #[arg(long, value_enum, default_value_t = Backend::default(), display_order = 3)]
    pub backend: Backend,

    /// The input files, supports glob patterns
    #[arg(value_name = "PATH", num_args = 1..)]
    pub input_paths: Vec<String>,

    /// The output directory
    #[arg(short, long, value_name = "DIR", display_order = 1)]
    #[arg(default_value_os_t = PathBuf::from("output"))]
    pub output: PathBuf,

    #[clap(flatten)]
    #[cfg(feature = "decode-audio")]
    pub audio: AudioArgs,

    /// Extension for image output
    #[cfg(feature = "process-image")]
    #[arg(long, value_name = "EXT", display_order = 2)]
    #[arg(value_parser = parse_image_format, default_value = "png")]
    pub image_ext: image::ImageFormat,

    /// The number of asset to extract at the same time
    #[arg(short = 'P', long, value_name = "CPUS", display_order = 2)]
    #[arg(default_value_t = std::thread::available_parallelism().unwrap_or_else(|_|1.try_into().unwrap()))]
    pub parallel: NonZeroUsize,

    /// The path to the asset ripper executable
    #[cfg(feature = "asset-ripper")]
    #[arg(long, value_name = "PATH", display_order = 3)]
    #[arg(default_value_os_t = default_asset_ripper_path())]
    pub asset_ripper_path: PathBuf,
}

/// Parses a single key-value pair
#[cfg(feature = "decode-audio")]
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
#[cfg(feature = "process-image")]
fn parse_image_format(s: &str) -> Result<image::ImageFormat> {
    let image_format = match image::ImageFormat::from_extension(s) {
        Some(f) => f,
        None => return Err(anyhow!("invalid image format: {}", s)),
    };

    Ok(image_format)
}

#[cfg(feature = "asset-ripper")]
#[inline]
fn default_asset_ripper_path() -> PathBuf {
    let mut path = PathBuf::from("AssetRipper");

    path.push(match cfg!(windows) {
        true => "AssetRipper.GUI.Free.exe",
        false => "AssetRipper.GUI.Free",
    });

    path
}

pub fn extract_files(args: &ExtractorArgs) -> Result<()> {
    let files = args
        .input_paths
        .iter()
        .filter_map(|p| match glob::glob(p) {
            Ok(paths) => Some(paths),
            Err(e) => {
                tracing::warn!(pattern = p, error = ?e, "failed to glob");
                None
            }
        })
        .flatten()
        .filter_map(|r| match r {
            Ok(p) => Some(p),
            Err(e) => {
                tracing::warn!(error = ?e, "failed to read directory entry");
                None
            }
        })
        .filter(|p| match p.is_file() {
            true => true,
            false => {
                tracing::warn!("input path is not a file");
                false
            }
        });

    create_dir_all(&args.output)?;

    let failed = match args.backend {
        #[cfg(feature = "asset-ripper")]
        Backend::AssetRipper => {
            tracing::debug!(
                parallel = args.parallel,
                asset_ripper_path = %args.asset_ripper_path.display(),
                "creating AssetRippers"
            );

            // AssetRipper exits immediately when its port is in use, so try the next one
            const MAX_PORT_FAILURES: usize = 100;

            let mut port: u16 = 50000;
            let mut port_failures = 0;
            let mut unity_extractors = Vec::new();
            while unity_extractors.len() < args.parallel.get() {
                match mltd::unity::AssetRipper::new(&args.asset_ripper_path, port) {
                    Ok(ripper) => {
                        tracing::trace!(port, "created AssetRipper");
                        unity_extractors.push(ripper);
                    }
                    Err(e) if e.kind() == mltd::ErrorKind::Process => {
                        port_failures += 1;
                        if port_failures > MAX_PORT_FAILURES {
                            return Err(e.into());
                        }
                        tracing::debug!(port, error = ?e, "cannot start AssetRipper, trying next port");
                    }
                    Err(e) => return Err(e.into()),
                };
                port = port.checked_add(1).ok_or_else(|| anyhow!("no available port"))?;
            }

            extract_parallel(unity_extractors, files, args)
        }

        #[cfg(feature = "rabex")]
        Backend::Rabex => {
            tracing::debug!(parallel = args.parallel, "creating rabex extractors");
            extract_parallel(mltd::unity::RabexExtractor::pool(args.parallel.get()), files, args)
        }
    };

    if failed > 0 {
        return Err(anyhow!("failed to extract {failed} file(s)"));
    }

    Ok(())
}

/// Files used to pass the HCA decryption keys to vgmstream, which are shared by the workers.
type KeyFiles = Mutex<BTreeSet<PathBuf>>;

/// Extracts the files with the given extractors, each of them handles one file at a time.
///
/// Returns the number of files that failed to extract.
fn extract_parallel<E>(
    unity_extractors: Vec<E>,
    files: impl Iterator<Item = PathBuf> + Send,
    args: &ExtractorArgs,
) -> usize
where
    E: Extractor + Send,
{
    let files = &Mutex::new(files);
    let key_files = &KeyFiles::default();
    let failed = &AtomicUsize::new(0);

    std::thread::scope(|scope| {
        for mut extractor in unity_extractors {
            scope.spawn(move || {
                loop {
                    let next = files.lock().unwrap().next();
                    let Some(path) = next else { break };

                    let result = extract_file(&path, &mut extractor, args, key_files);

                    extractor.clear();

                    if let Err(e) = result {
                        tracing::warn!(input_path = %path.display(), error = ?e, "failed to extract file");
                        failed.fetch_add(1, Ordering::Relaxed);
                    };
                }
            });
        }
    });

    // the key files can only be removed after all workers have finished decoding
    for key_path in key_files.lock().unwrap().iter() {
        tracing::debug!(key_path = %key_path.display(), "removing temporary files");
        if let Err(e) = std::fs::remove_file(key_path) {
            tracing::warn!(error = ?e);
        }
    }

    failed.load(Ordering::Relaxed)
}

/// Extracts a single .unity3d file.
#[cfg_attr(not(feature = "decode-audio"), allow(unused_variables))]
fn extract_file(
    path: &Path,
    unity_extractor: &mut impl Extractor,
    args: &ExtractorArgs,
    key_files: &KeyFiles,
) -> Result<()> {
    tracing::debug!(input_path = %path.display(), "loading file");
    unity_extractor.load_file(path.as_ref())?;

    #[cfg(feature = "decode-audio")]
    let mut audio_paths = HashSet::new();

    let infos = unity_extractor.all_asset_infos();
    for info in infos.iter() {
        if info.class == "AssetBundle" {
            // asset bundles contains bundle information only
            continue;
        }

        let Some(orig_path) = info.orig_path.as_deref() else {
            tracing::warn!(name = &*info.name, class = &*info.class, "asset has no original path");
            continue;
        };
        let asset_original_path = &PathBuf::from(orig_path);

        let output_path = safe_join(&args.output, orig_path)?;
        let output_dir = output_path
            .parent()
            .ok_or_else(|| anyhow!("output path has no parent: {}", output_path.display()))?;

        create_dir_all(output_dir)?;

        match &*info.class {
            "TextAsset" => {
                // remove .bytes
                let output_path = output_path.with_extension("");
                let file_ext =
                    output_path.extension().and_then(|s| s.to_str()).unwrap_or_default().to_owned();

                let mut file = BufReader::new(unity_extractor.extract(info)?);
                let mut out_file = BufWriter::new(File::create(&output_path)?);

                if ENCRYPTED_TEXT_EXTENSIONS.contains(&file_ext.as_str()) {
                    #[cfg(feature = "decrypt-text")]
                    {
                        tracing::info!(filename = %asset_original_path.file_name().unwrap().display(), "decrypting text");
                        let mut buf = vec![];
                        file.read_to_end(&mut buf)?;
                        let buf = decrypt_text(&buf)?;
                        tracing::trace!(decrypted = %String::from_utf8_lossy(&buf).as_ref());

                        tracing::info!(
                            output_path = %output_path.display(),
                            "extracting decrypted text"
                        );
                        std::io::copy(&mut std::io::Cursor::new(buf), &mut out_file)?;
                    }

                    #[cfg(not(feature = "decrypt-text"))]
                    {
                        tracing::info!(
                            output_path = %output_path.display(),
                            "extracting encrypted text"
                        );
                        std::io::copy(&mut file, &mut out_file)?;
                    }
                } else {
                    if ["acb", "awb"].contains(&file_ext.as_str()) {
                        // we should handle audio files after all of them are extracted because
                        // some files may contain informations (Ex: filename) about others
                        #[cfg(feature = "decode-audio")]
                        audio_paths.insert(output_path.clone());

                        #[cfg(not(feature = "decode-audio"))]
                        tracing::info!(
                            "feature `decode-audio` is not enabled, saving acb directly"
                        );
                    }

                    tracing::info!(output_path = %output_path.display(), "extracting audio");
                    std::io::copy(&mut file, &mut out_file)?;
                }
            }

            "Texture2D" => {
                #[cfg(feature = "process-image")]
                // Texture2D requires all relavent Sprite infos to be extracted
                {
                    let texture_info = info;
                    let images = extract_image(
                        texture_info,
                        infos.iter().cloned().filter(|info| info.class == "Sprite"),
                        unity_extractor,
                    )?;

                    images.into_iter().enumerate().par_bridge().try_for_each(
                        |(i, image)| -> Result<()> {
                            let path = output_dir.join(format!(
                                "{}_{}.{}",
                                texture_info.name,
                                i,
                                args.image_ext.extensions_str()[0]
                            ));
                            tracing::info!(output_path = %path.display(), "writing image");

                            let mut file = BufWriter::new(File::create(&path)?);
                            image.write_to(&mut file, args.image_ext)?;
                            file.flush()?;

                            Ok(())
                        },
                    )?;
                }

                #[cfg(not(feature = "process-image"))]
                {
                    tracing::info!(output_path = %output_path.display(), "writing unprocessed image");

                    let mut file = BufReader::new(unity_extractor.extract(info)?);
                    let mut output_file = BufWriter::new(File::create(output_path)?);

                    std::io::copy(&mut file, &mut output_file)?;
                }
            }

            "Sprite" => (), // sprites are handled in Texture2D extractor
            _ => tracing::warn!(class = &*info.class, "unknown asset class"),
        };
    }

    #[cfg(feature = "decode-audio")]
    for output_path in audio_paths.into_iter() {
        extract_audio(&output_path, &args.audio, key_files)?;
    }

    Ok(())
}

#[cfg(feature = "process-image")]
fn extract_image<'a>(
    texture_info: &AssetInfo,
    sprite_infos: impl Iterator<Item = &'a AssetInfo>,
    extractor: &impl Extractor,
) -> Result<Vec<DynamicImage>> {
    use std::io::BufReader;

    let image =
        image::load(BufReader::new(extractor.extract(texture_info)?), image::ImageFormat::Png)?
            .flipv();

    let mut rects = BTreeMap::new();
    for info in sprite_infos {
        if info.class != "Sprite" {
            continue;
        }

        // AssetRipper names the fields `m_X`, `m_Texture`, ... while the type tree based
        // extractors use the names written by Unity (`x`, `texture`, ...) and floats.
        let json = extractor.asset_serialized(info)?;
        let pointer = |paths: &[&str]| paths.iter().find_map(|p| json.pointer(p)).cloned();
        let number =
            |paths: &[&str]| pointer(paths).and_then(|v| v.as_f64()).map(|v| v.round() as u32);

        let Some(path_id) = pointer(&["/m_RD/m_Texture/m_PathID", "/m_RD/texture/m_PathID"])
            .and_then(|v| v.as_i64())
        else {
            tracing::warn!(name = &*info.name, "sprite has no texture reference, skipping");
            continue;
        };
        if path_id != texture_info.path_id {
            continue;
        }

        let (Some(x), Some(y), Some(width), Some(height)) = (
            number(&["/m_Rect/m_X", "/m_Rect/x"]),
            number(&["/m_Rect/m_Y", "/m_Rect/y"]),
            number(&["/m_Rect/m_Width", "/m_Rect/width"]),
            number(&["/m_Rect/m_Height", "/m_Rect/height"]),
        ) else {
            tracing::warn!(name = &*info.name, "sprite has no rect, skipping");
            continue;
        };
        if u64::from(x) + u64::from(width) > u64::from(image.width())
            || u64::from(y) + u64::from(height) > u64::from(image.height())
        {
            tracing::warn!(name = &*info.name, "sprite rect is out of the texture, skipping");
            continue;
        }

        let Some(sprite_id) = info.name.rsplit_once('_').and_then(|(_, n)| n.parse::<u32>().ok())
        else {
            tracing::warn!(name = &*info.name, "sprite name has no piece number, skipping");
            continue;
        };
        let piece = image.view(x, y, width, height);
        rects.insert(sprite_id, piece);
    }

    let images =
        match solve_puzzle(&texture_info.name, &image, &rects.into_values().collect::<Vec<_>>()) {
            Ok(images) => images,
            Err(e) => {
                tracing::warn!(error = ?e, filename = &*texture_info.name, "using original image");
                vec![image]
            }
        };

    Ok(images)
}

#[cfg(feature = "decode-audio")]
fn extract_audio(path: &Path, args: &AudioArgs, key_files: &KeyFiles) -> Result<()> {
    use rayon::iter::IntoParallelIterator;

    // According to https://github.com/vgmstream/vgmstream/blob/master/doc/USAGE.md#decryption-keys,
    // we can specify the decryption key in the .hcakey file so that vgmstream doesn't have
    // to brute-force the key.
    //
    // All MLTD audio files use the same key, so the file is written once per directory and
    // shared by all workers. It is removed after all of them have finished.
    let key_path = path.with_file_name(".hcakey");
    {
        let mut key_files = key_files.lock().unwrap();
        if !key_files.contains(&key_path) {
            tracing::debug!(output_path = %key_path.display() ,"writing temporary .hcakey");
            File::create(&key_path)?.write_all(MLTD_HCA_KEY.to_string().as_bytes())?;
            key_files.insert(key_path);
        }
    }

    let output_dir = path.parent().expect("output dir should exist");
    tracing::info!(output_path = %output_dir.display(), "extracting audio");

    let mut sf = vgmstream::StreamFile::open(path)?;
    let mut vgmstream = vgmstream::VgmStream::new()?;
    vgmstream.open(&mut sf, 0)?;
    let subsongs = vgmstream.format()?.subsong_count as usize;
    tracing::debug!(output_path = %output_dir.display(), subsongs);

    let results: Vec<Result<()>> = (0..subsongs)
        .into_par_iter()
        .map(|i| -> Result<()> {
            let mut options = ffmpeg_next::Dictionary::new();
            for (key, value) in &args.audio_args {
                options.set(key, value);
            }

            let output_prefix = path.with_extension("");
            let options = EncoderOutputOptions {
                prefix: output_prefix.file_name().unwrap().to_str().unwrap(),
                codec: &args.audio_codec,
                format: &args.audio_format,
                ffmpeg_options: Some(options),
            };

            let mut encoder = Encoder::open(path, i, output_dir, &options)?;
            encoder.encode()?;

            Ok(())
        })
        .collect();

    // report every failed subsong, but only fail once
    let mut first_error = None;
    for result in results {
        if let Err(e) = result {
            tracing::warn!(input_path = %path.display(), error = ?e, "failed to extract audio");
            first_error.get_or_insert(e);
        }
    }

    if let Some(e) = first_error {
        return Err(e);
    }

    Ok(())
}
