use std::error::Error;
use std::fs::File;
use std::io::{Cursor, Write};
use std::mem::size_of_val;
use std::path::Path;
use std::slice::from_raw_parts;
use std::str::FromStr;

use aes::cipher::block_padding::Pkcs7;
use aes::cipher::inout::InOutBufReserved;
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use aes::Aes192;
use anyhow::Result;
use byteorder::{BigEndian, ByteOrder, LittleEndian};
use cbc::{Decryptor, Encryptor};
use rabex::files::SerializedFile;
use rabex::objects::classes::TextAsset;
use rabex::read_ext::ReadUrexExt;
use tempfile::tempdir;

use crate::extract::utils::audio::Encoder;
use crate::extract::utils::ReadAlignedExt;
use crate::extract::version::*;
use crate::extract::ExtractorArgs;

pub(super) fn _construct_text_asset<E>(
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
    let temp_dir = tempdir()?;
    let acb_path = temp_dir.path().join(&text_asset.m_Name).with_extension("acb");
    let mut acb_file = File::create(&acb_path)?;
    acb_file.write_all(text_asset.m_Script.as_bytes())?;

    let output_path =
        output_dir.as_ref().join(&text_asset.m_Name).with_extension(&args.audio_format);

    let mut options = ffmpeg_next::Dictionary::new();
    for (key, value) in &args.audio_args {
        options.set(key, value);
    }
    if !args.image_args.is_empty() {
        log::debug!("audio options: {:#?}", options);
    }

    let mut encoder = Encoder::open(&acb_path, &output_path, &args.audio_codec, Some(options))?;

    encoder.encode()?;

    Ok(())
}

pub const MLTD_TEXT_PBKDF2_HMAC_SHA1_KEY: &[u8; 8] = b"Millicon";
pub const MLTD_TEXT_PBKDF2_HMAC_SHA1_SALT: &[u8; 9] = b"DAISUL___";
pub const MLTD_TEXT_PBKDF2_HMAC_SHA1_ROUNDS: u32 = 1000;

/// The AES-192-CBC key used to decrypt the text asset.
/// 
/// It is derived from [`MLTD_TEXT_PBKDF2_HMAC_SHA1_KEY`] and
/// [`MLTD_TEXT_PBKDF2_HMAC_SHA1_SALT`] using PBKDF2-HMAC-SHA1, where
/// the first 24 bytes of the derived key are used as the actual key.
#[rustfmt::skip]
pub const MLTD_TEXT_DECRYPT_KEY: &[u8; 24] = &[
    0xad, 0x3f, 0x0f, 0x89, 0xee, 0x51, 0xc5, 0x37,
    0x73, 0x1f, 0x17, 0x96, 0xf7, 0x5c, 0x71, 0x84,
    0x01, 0x61, 0x75, 0x6d, 0xa0, 0xd4, 0x86, 0xc9,
];

/// The AES-192-CBC initialization vector used to decrypt the text asset.
/// 
/// It is derived from [`MLTD_TEXT_PBKDF2_HMAC_SHA1_KEY`] and
/// [`MLTD_TEXT_PBKDF2_HMAC_SHA1_SALT`] using PBKDF2-HMAC-SHA1, where
/// the last 16 bytes of the derived key are used as the actual IV.
#[rustfmt::skip]
pub const MLTD_TEXT_DECRYPT_IV: &[u8; 16] = &[
    0x4e, 0x40, 0xb3, 0x8a, 0xeb, 0xf1, 0xa8, 0x53,
    0x12, 0x2c, 0x5f, 0xad, 0xcc, 0xa3, 0x68, 0x5d,
];

pub type MltdTextEncryptor = Encryptor<Aes192>;
pub type MltdTextDecryptor = Decryptor<Aes192>;

pub fn encrypt_text(text: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let encryptor =
        MltdTextEncryptor::new(MLTD_TEXT_DECRYPT_KEY.into(), MLTD_TEXT_DECRYPT_IV.into());
    let mut buf = text.to_owned();

    let buf = InOutBufReserved::from_mut_slice(&mut buf, text.len()).map_err(|e| e.to_string())?;
    let buf = encryptor.encrypt_padded_inout_mut::<Pkcs7>(buf).map_err(|e| e.to_string())?;

    Ok(buf.to_owned())
}

pub fn decrypt_text(cipher: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let decryptor =
        MltdTextDecryptor::new(MLTD_TEXT_DECRYPT_KEY.into(), MLTD_TEXT_DECRYPT_IV.into());
    let mut buf = cipher.to_owned();

    let buf = decryptor
        .decrypt_padded_inout_mut::<Pkcs7>(buf.as_mut_slice().into())
        .map_err(|e| e.to_string())?;

    Ok(buf.to_owned())
}
