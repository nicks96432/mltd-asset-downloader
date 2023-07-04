mod structs;
mod version;

use byteorder::BigEndian;
use byteorder::ReadBytesExt;

pub use self::structs::*;
pub use self::version::*;

use crate::asset::Header;
use crate::bundle::Signature;
use crate::error::Error;
use crate::traits::ReadString;

use std::io::{Read, Seek, SeekFrom};
use std::str::FromStr;

pub mod file_magic {
    pub const GZIP: [u8; 2] = *b"\x1f\x8b";
    pub const BROTLI: [u8; 6] = *b"brotli";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileType {
    #[default]
    Unknown,
    Asset,
    AssetBundle(Signature),
    Resource,
    WebFile,
}

impl FileType {
    pub fn parse<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        if let Ok(string) = reader.read_string() {
            if let Ok(s) = Signature::from_str(&string) {
                return Ok(Self::AssetBundle(s));
            }
        }

        reader.seek(SeekFrom::Start(0u64))?;

        let mut magic = [0u8; 2];
        reader.read_exact(&mut magic)?;
        if magic == file_magic::GZIP {
            return Ok(Self::WebFile);
        }

        reader.seek(SeekFrom::Start(20u64))?;
        let mut magic = [0u8; 6];
        reader.read_exact(&mut magic)?;
        if magic == file_magic::BROTLI {
            return Ok(Self::WebFile);
        }

        reader.seek(SeekFrom::Start(0u64))?;

        let mut header = Header {
            metadata_size: reader.read_i32::<BigEndian>()?,
            asset_size: i64::from(reader.read_i32::<BigEndian>()?),
            version: reader.read_i32::<BigEndian>()?,
            data_offset: i64::from(reader.read_i32::<BigEndian>()?),
            ..Default::default()
        };

        if header.version >= 22 {
            header.big_endian = reader.read_u8()? > 0;
            reader.read_exact(&mut header.padding)?;
            header.metadata_size = reader.read_i32::<BigEndian>()?;
            header.asset_size = reader.read_i64::<BigEndian>()?;
            header.data_offset = reader.read_i64::<BigEndian>()?;
            header.unknown = reader.read_u64::<BigEndian>()?;
        }

        reader.seek(SeekFrom::Start(0u64))?;

        let range = 0i64..=i64::try_from(reader.stream_len()?)?;
        if [
            header.asset_size,
            i64::from(header.metadata_size),
            i64::from(header.version),
            header.data_offset,
        ]
        .iter()
        .any(|i| !range.contains(i))
            || !((0i32..=100i32).contains(&header.version))
            || header.asset_size < i64::from(header.metadata_size)
            || header.asset_size < header.data_offset
        {
            return Ok(Self::Resource);
        }

        Ok(Self::Asset)
    }
}

pub fn bool_to_yes_no(b: bool) -> &'static str {
    match b {
        true => "Yes",
        false => "No",
    }
}
