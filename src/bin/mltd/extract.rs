use std::fs::{File, create_dir_all};
use std::io::{BufReader, BufWriter, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Result, anyhow};
use clap::Args;
use mltd::ErrorKind;
use mltd::extract::ENCRYPTED_TEXT_EXTENSIONS;
use mltd::unity::Extractor;
#[cfg(feature = "asset-ripper")]
use mltd::unity::asset_ripper::AssetRipper;
use rayon::iter::{ParallelBridge, ParallelIterator};
#[cfg(feature = "process-image")]
use {
    image::DynamicImage, image::GenericImageView, mltd::extract::puzzle::solve_puzzle,
    mltd::unity::AssetInfo, std::collections::BTreeMap,
};
#[cfg(feature = "decode-audio")]
use {
    mltd::extract::audio::{Encoder, EncoderOutputOptions, MLTD_HCA_KEY},
    std::collections::HashSet,
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

#[derive(Debug, Args)]
#[command(author, version, about, arg_required_else_help(true))]
pub struct ExtractorArgs {
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

    tracing::debug!(
        parallel = args.parallel,
        asset_ripper_path = %args.asset_ripper_path.display(),
        "creating AssetRippers"
    );

    let mut port = 50000;
    let mut unity_extractors = Vec::new();
    while unity_extractors.len() < args.parallel.get() {
        match AssetRipper::new(&args.asset_ripper_path, port) {
            Ok(ripper) => {
                tracing::trace!(port, "created AssetRipper");
                unity_extractors.push(Mutex::new(ripper));
                port += 1;
            }
            Err(e) if e.kind() == ErrorKind::Network => port += 1,
            Err(e) => return Err(e.into()),
        };
    }

    create_dir_all(&args.output)?;

    files.zip(unity_extractors.iter().cycle()).par_bridge().for_each(|(path, extractor)| {
        let mut extractor = extractor.lock().unwrap();
        if let Err(e) = extract_file(&path, &mut *extractor, args) {
            tracing::warn!(error = ?e, "failed to extract file");
        };
    });

    Ok(())
}

/// Extracts a single .unity3d file.
fn extract_file(
    path: &Path,
    unity_extractor: &mut impl Extractor,
    args: &ExtractorArgs,
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

        let asset_original_path = &PathBuf::from(
            info.orig_path.as_deref().expect("original path of TextAsset should exist"),
        );

        let output_path = args.output.join(asset_original_path);
        let output_dir = output_path.parent().unwrap();

        create_dir_all(output_dir)?;

        match &*info.class {
            "TextAsset" => {
                // remove .bytes
                let output_path = output_path.with_extension("");
                let file_ext = output_path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_owned())
                    .ok_or_else(|| anyhow!("file stem should exist"))?;

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
                for texture_info in infos.iter().filter(|info| info.class == "Texture2D") {
                    let images = extract_image(
                        texture_info,
                        infos.iter().cloned().filter(|info| info.class == "Sprite"),
                        unity_extractor,
                    )?;

                    images.into_iter().enumerate().par_bridge().for_each(|(i, image)| {
                        let path = output_dir.join(format!(
                            "{}_{}.{}",
                            texture_info.name,
                            i,
                            args.image_ext.extensions_str()[0]
                        ));
                        tracing::info!(output_path = %path.display(), "writing image");

                        let mut file = match std::fs::File::create(&path) {
                            Ok(f) => f,
                            Err(e) => return tracing::warn!(error = ?e),
                        };
                        if let Err(e) = image.write_to(&mut file, args.image_ext) {
                            tracing::warn!(error = ?e);
                        }
                    });
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
        extract_audio(&output_path, &args.audio)?;
    }

    unity_extractor.clear();

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

        let json = extractor.asset_serialized(info)?;
        let path_id = json.pointer("/m_RD/m_Texture/m_PathID").unwrap().as_i64().unwrap();
        if path_id != texture_info.path_id {
            continue;
        }

        let x = json.pointer("/m_Rect/m_X").unwrap().as_u64().unwrap() as u32;
        let y = json.pointer("/m_Rect/m_Y").unwrap().as_u64().unwrap() as u32;
        let width = json.pointer("/m_Rect/m_Width").unwrap().as_u64().unwrap() as u32;
        let height = json.pointer("/m_Rect/m_Height").unwrap().as_u64().unwrap() as u32;

        let sprite_id: u32 = info.name.rsplit_once("_").unwrap().1.parse().unwrap();
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
fn extract_audio(path: &Path, args: &AudioArgs) -> Result<()> {
    use rayon::iter::IntoParallelIterator;

    // According to https://github.com/vgmstream/vgmstream/blob/master/doc/USAGE.md#decryption-keys,
    // we can specify the decryption key in the .hcakey file so that vgmstream doesn't have
    // to brute-force the key.
    let key_path = path.with_file_name(".hcakey");
    let mut key_file = File::create(&key_path)?;
    tracing::debug!(output_path = %key_path.display() ,"writing temporary .hcakey");
    key_file.write_all(MLTD_HCA_KEY.to_string().as_bytes())?;

    let output_dir = path.parent().expect("output dir should exist");
    tracing::info!(output_path = %output_dir.display(), "extracting audio");

    let mut sf = vgmstream::StreamFile::open(path)?;
    let mut vgmstream = vgmstream::VgmStream::new()?;
    vgmstream.open(&mut sf, 0)?;
    let subsongs = vgmstream.format()?.subsong_count as usize;
    tracing::debug!(output_path = %output_dir.display(), subsongs);

    (0..subsongs).into_par_iter().for_each(|i| {
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

        let mut encoder = match Encoder::open(path, i, output_dir, &options) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(error = ?e);
                return;
            }
        };

        if let Err(e) = encoder.encode() {
            tracing::warn!(error = ?e);
        }
    });

    tracing::debug!(key_path = %key_path.display(), "removing temporary files");
    std::fs::remove_file(key_path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::Path;

    use assert_cmd::Command;
    use assert_fs::TempDir;
    use assert_fs::prelude::*;
    use mltd::util::init_test_logger;
    use predicates::prelude::*;

    struct TestCase {
        pub bundle_path: &'static str,
        pub expected_path: &'static str,
        pub got_relpath: &'static str,
        pub predicate: fn(&TempDir, &Self),
    }

    const TEST_CASES: &[TestCase] = &[
        TestCase {
            bundle_path: "tests/test_acb.unity3d",
            expected_path: "tests/test_acb.acb",
            got_relpath: "Assets/imas/resources/adx2/song3/song3_00test.acb",
            predicate: |dir, case| {
                if cfg!(feature = "decode-audio") {
                    dir.child(Path::new(case.got_relpath).with_file_name("song3_00test_bgm.wav"))
                        .assert(predicate::path::eq_file(
                            Path::new(case.expected_path).with_extension("wav"),
                        ));
                } else {
                    dir.child(case.got_relpath)
                        .assert(predicate::path::eq_file(case.expected_path));
                }
            },
        },
        TestCase {
            bundle_path: "tests/test_text.unity3d",
            expected_path: "tests/test_text.gtx",
            got_relpath: "Assets/imas/resources/textassets/zh/commu/card/card_blst_014mir0034_zh.gtx",
            predicate: |dir, case| {
                let got_path = dir.child(case.got_relpath);
                if cfg!(feature = "decrypt-text") {
                    got_path.assert(predicate::path::eq_file(
                        Path::new(case.expected_path).with_file_name("test_text_decrypted.gtx"),
                    ));
                } else {
                    got_path.assert(predicates::path::eq_file(case.expected_path));
                }
            },
        },
        TestCase {
            bundle_path: "tests/test_image.unity3d",
            expected_path: "tests/test_image.png",
            got_relpath: "Assets/imas/resources/card/014mir0011_0.png",
            predicate: |dir, case| {
                if cfg!(feature = "process-image") {
                    let expected_path =
                        Path::new(case.expected_path).with_file_name("test_image_processed.png");
                    let got_path =
                        dir.child(Path::new(case.got_relpath).with_file_name("014mir0011_0_0.png"));
                    got_path.assert(predicates::path::eq_file(expected_path));
                } else {
                    dir.child(case.got_relpath)
                        .assert(predicates::path::eq_file(case.expected_path));
                }
            },
        },
    ];

    #[test]
    fn test_extract() {
        init_test_logger();

        for case in TEST_CASES {
            let dir = assert_fs::TempDir::new().unwrap();

            let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
            cmd.arg("extract")
                .arg(case.bundle_path)
                .args([OsStr::new("-o"), dir.path().as_os_str()])
                .current_dir(env!("CARGO_MANIFEST_DIR"));

            cmd.assert().success();

            (case.predicate)(&dir, case);
        }
    }
}
