use crate::error::Error;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, SeekFrom, Write};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Header {
    pub metadata_size: u32,
    pub asset_size: u64,
    pub version: u32,
    pub data_offset: u64,

    pub big_endian: bool,
    pub padding: [u8; 3],
    pub unknown: u64,
}

impl Header {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut header = Self::new();

        header.metadata_size = reader.read_u32::<BigEndian>()?;
        header.asset_size = u64::from(reader.read_u32::<BigEndian>()?);
        header.version = reader.read_u32::<BigEndian>()?;
        header.data_offset = u64::from(reader.read_u32::<BigEndian>()?);

        if header.version >= 9 {
            header.big_endian = reader.read_u8()? > 0;
            reader.read_exact(&mut header.padding)?;

            if header.version >= 22 {
                header.metadata_size = reader.read_u32::<BigEndian>()?;
                header.asset_size = reader.read_u64::<BigEndian>()?;
                header.data_offset = reader.read_u64::<BigEndian>()?;
                header.unknown = reader.read_u64::<BigEndian>()?;
            }
        } else {
            let off = header.asset_size - header.metadata_size as u64;
            reader.seek(SeekFrom::Start(off))?;
            header.big_endian = reader.read_u8()? > 0;
        }

        Ok(header)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        match self.version {
            v if v <= 9 => {
                unimplemented!()
            }
            v if 9 < v && v <= 22 => {
                writer.write_u32::<BigEndian>(self.metadata_size)?;
                writer.write_u32::<BigEndian>(u32::try_from(self.asset_size)?)?;
                writer.write_u32::<BigEndian>(self.version)?;
                writer.write_u32::<BigEndian>(u32::try_from(self.data_offset)?)?;
                writer.write_u8(u8::from(self.big_endian))?;
                writer.write_all(&self.padding)?;
            }
            // v > 22
            _ => {
                writer.write_u32::<BigEndian>(0u32)?;
                writer.write_u32::<BigEndian>(0u32)?;
                writer.write_u32::<BigEndian>(self.version)?;
                writer.write_u32::<BigEndian>(0u32)?;
                writer.write_u8(u8::from(self.big_endian))?;
                writer.write_all(&self.padding)?;
                writer.write_u32::<BigEndian>(self.metadata_size)?;
                writer.write_u64::<BigEndian>(self.asset_size)?;
                writer.write_u64::<BigEndian>(self.data_offset)?;
                writer.write_u64::<BigEndian>(self.unknown)?;
            }
        }

        Ok(())
    }
}

impl Display for Header {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Metadata size: {}",
            "",
            self.metadata_size,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Asset size:    {}",
            "",
            self.asset_size,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Version:       {}",
            "",
            self.version,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Data offset:   {}",
            "",
            self.data_offset,
            indent = indent
        )?;

        let endian = match self.big_endian {
            true => "big",
            false => "little",
        };
        writeln!(
            f,
            "{:indent$}Endian:        {}",
            "",
            endian,
            indent = indent
        )?;

        Ok(())
    }
}
