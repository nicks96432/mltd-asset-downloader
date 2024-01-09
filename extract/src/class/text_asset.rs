use std::error::Error;
use std::fs::write;
use std::io::Cursor;
use std::mem::size_of_val;
use std::path::Path;
use std::slice::from_raw_parts;
use std::str::FromStr;

use byteorder::BigEndian;
use byteorder::ByteOrder;
use byteorder::LittleEndian;
use rabex::files::SerializedFile;
use rabex::objects::classes::TextAsset;
use rabex::read_ext::ReadUrexExt;

use crate::utils::ffmpeg;
use crate::utils::ReadAlignedExt;
use crate::version::*;
use crate::ExtractorArgs;

pub fn _construct_text_asset<E>(
    data: &[u8],
    serialized_file: &SerializedFile,
) -> Result<TextAsset, Box<dyn Error>>
where
    E: ByteOrder,
{
    let mut reader = Cursor::new(data);
    let unity_version = Version::from_str(serialized_file.m_UnityVersion.as_ref().unwrap())?;

    Ok(TextAsset {
        m_Name: reader.read_aligned_string::<E>()?,
        m_PathName: match UNITY_VERSION_3_4_0 <= unity_version
            && unity_version <= UNITY_VERSION_2017_1_0_B1
        {
            true => Some(reader.read_aligned_string::<E>()?),
            false => None,
        },
        m_Script: unsafe { String::from_utf8_unchecked(reader.read_bytes::<E>()?) },
    })
}

pub fn construct_text_asset(
    data: &[u8],
    serialized_file: &SerializedFile,
) -> Result<TextAsset, Box<dyn Error>> {
    let big_endian = unsafe {
        from_raw_parts(
            (&serialized_file.m_Header as *const _) as *const u8,
            size_of_val(&serialized_file.m_Header),
        )
    }[0x20]
        > 0;

    match big_endian {
        true => _construct_text_asset::<BigEndian>(data, serialized_file),
        false => _construct_text_asset::<LittleEndian>(data, serialized_file),
    }
}

pub fn extract_acb<P>(
    data: &[u8],
    output_dir: P,
    args: &ExtractorArgs,
    serialized_file: &SerializedFile,
) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
{
    let text_asset = construct_text_asset(data, serialized_file)?;
    let wav = acb::to_tracks(text_asset.m_Script.as_bytes())?.swap_remove(0);
    let output_path = output_dir.as_ref().join(text_asset.m_Name).with_extension(&args.audio_ext);

    log::info!("writing audio to: {}", output_path.display());

    if args.audio_ext == "wav" && args.audio_args.is_empty() {
        write(&output_path, wav.data)?;
        return Ok(());
    }

    ffmpeg(&wav.data, num_cpus::get() / args.parallel, &args.audio_args, output_path)
}
