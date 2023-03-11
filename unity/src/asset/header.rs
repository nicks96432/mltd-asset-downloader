use crate::error::Error;
use crate::macros::impl_default;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Seek, SeekFrom, Write};

#[derive(Debug, Clone)]
pub struct Header {
    pub metadata_size: u32,
    pub asset_size: u64,
    pub version: u32,
    pub offset: u64,

    /// true: big endian, false: little endian
    pub endian: bool,
    pub reserved: u32,
    pub unknown: u64,
}

impl Header {
    pub fn new() -> Self {
        Self {
            metadata_size: 0u32,
            asset_size: 0u64,
            version: 0u32,
            offset: 0u64,
            endian: true,
            reserved: 0u32,
            unknown: 0u64,
        }
    }

    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Error> {
        let mut header = Self::new();

        header.metadata_size = reader.read_u32::<BigEndian>()?;
        header.asset_size = reader.read_u32::<BigEndian>()? as u64;
        header.version = reader.read_u32::<BigEndian>()?;
        header.offset = reader.read_u32::<BigEndian>()? as u64;

        if header.version >= 9 {
            header.endian = reader.read_u8()? > 0;
            header.reserved = reader.read_u24::<BigEndian>()?;

            if header.version >= 22 {
                header.metadata_size = reader.read_u32::<BigEndian>()?;
                header.asset_size = reader.read_u64::<BigEndian>()?;
                header.offset = reader.read_u64::<BigEndian>()?;
                header.unknown = reader.read_u64::<BigEndian>()?;
            }
        } else {
            let off = header.asset_size - header.metadata_size as u64;
            reader.seek(SeekFrom::Start(off))?;
            header.endian = reader.read_u8()? > 0;
        }

        Ok(header)
    }

    pub fn save<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        match self.version {
            v if v <= 9 => {
                unimplemented!()
            }
            v if 9 < v && v <= 22 => {
                writer.write_u32::<BigEndian>(self.metadata_size)?;
                writer.write_u32::<BigEndian>(self.asset_size as u32)?;
                writer.write_u32::<BigEndian>(self.version)?;
                writer.write_u32::<BigEndian>(self.offset as u32)?;
                writer.write_u8(u8::from(self.endian))?;
                writer.write_u24::<BigEndian>(self.reserved)?;
            }
            // v > 22
            _ => {
                writer.write_u32::<BigEndian>(0u32)?;
                writer.write_u32::<BigEndian>(0u32)?;
                writer.write_u32::<BigEndian>(self.version)?;
                writer.write_u32::<BigEndian>(0u32)?;
                writer.write_u8(u8::from(self.endian))?;
                writer.write_u24::<BigEndian>(self.reserved)?;
                writer.write_u32::<BigEndian>(self.metadata_size)?;
                writer.write_u64::<BigEndian>(self.asset_size)?;
                writer.write_u64::<BigEndian>(self.offset)?;
                writer.write_u64::<BigEndian>(self.unknown)?;
            }
        }

        Ok(())
    }
}

impl_default!(Header);
