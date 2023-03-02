use super::Platform;
use crate::error::Error;
use crate::macros::impl_default;
use crate::traits::UnityIO;
use crate::ReadExact;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use num_traits::FromPrimitive;
use std::io::{Read, Seek, SeekFrom, Write};

#[derive(Debug, Clone)]
pub struct AssetHeader {
    pub metadata_size: u32,
    pub asset_size: u32,
    pub version: u32,
    pub offset: u32,
    pub endian: u32,
    pub unity_version: String,
    pub target_platform: Platform,
}

impl UnityIO for AssetHeader {
    fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Error> {
        let mut header = Self::new();

        header.metadata_size = reader.read_u32::<BigEndian>()?;
        header.asset_size = reader.read_u32::<BigEndian>()?;
        header.version = reader.read_u32::<BigEndian>()?;
        header.offset = reader.read_u32::<BigEndian>()?;

        if header.version >= 9 {
            header.endian = reader.read_u32::<BigEndian>()?;
            if header.version >= 22 {
                todo!()
            }
        } else {
            let off = u64::from(header.asset_size - header.metadata_size);
            reader.seek(SeekFrom::Start(off))?;
            header.endian = reader.read_u32::<BigEndian>()?;
        }

        let read_u32 = |reader: &mut R, e: u32| match e {
            0 => reader.read_u32::<LittleEndian>(),
            _ => reader.read_u32::<BigEndian>(),
        };

        if header.version >= 7 {
            header.unity_version = reader.read_string()?;
        }
        if header.version >= 8 {
            header.target_platform = Platform::from_u32(read_u32(reader, header.endian)?)
                .ok_or(Error::UnknownSignature)?;
        }

        Ok(header)
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_u32::<BigEndian>(self.metadata_size)?;
        writer.write_u32::<BigEndian>(self.asset_size)?;
        writer.write_u32::<BigEndian>(self.version)?;
        writer.write_u32::<BigEndian>(self.offset)?;

        Ok(())
    }
}

impl AssetHeader {
    pub fn new() -> Self {
        Self {
            metadata_size: 0u32,
            asset_size: 0u32,
            version: 0u32,
            offset: 0u32,
            endian: 0u32,
            unity_version: String::new(),
            target_platform: Platform::UnknownPlatform,
        }
    }
}

impl_default!(AssetHeader);
