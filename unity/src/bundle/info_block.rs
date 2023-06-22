use crate::compression::Method as CompressionMethod;
use crate::error::Error;
use crate::macros::impl_try_from_into_vec;
use crate::traits::ReadString;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use num_traits::FromPrimitive;

use std::fmt::{Debug, Display, Formatter};
use std::io::{Read, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlockInfo {
    pub decompressed_size: u32,
    pub compressed_size: u32,
    pub flags: u16,
}

impl BlockInfo {
    pub const BASE_SIZE: usize = 10;

    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the compression method of the data block of
    /// this [`BlockInfo`].
    ///
    /// # Errors
    ///
    /// This function will return [`Error::UnknownCompressionMethod`] if
    /// the compression method is unknown.
    pub fn compression_method(&self) -> Result<CompressionMethod, Error> {
        let value = u32::from(self.flags & 0x3f);

        CompressionMethod::from_u32(value).ok_or_else(|| Error::UnknownCompressionMethod)
    }

    pub fn read<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: Read,
    {
        Ok(Self {
            decompressed_size: reader.read_u32::<BigEndian>()?,
            compressed_size: reader.read_u32::<BigEndian>()?,
            flags: reader.read_u16::<BigEndian>()?,
        })
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        writer.write_u32::<BigEndian>(self.decompressed_size)?;
        writer.write_u32::<BigEndian>(self.compressed_size)?;
        writer.write_u16::<BigEndian>(self.flags)?;

        Ok(())
    }
}

impl Display for BlockInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Block size: {:<8} (decompressed {})",
            "",
            self.compressed_size,
            self.decompressed_size,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Flags:      {:#04x}",
            "",
            self.flags,
            indent = indent
        )?;

        Ok(())
    }
}

impl_try_from_into_vec!(BlockInfo);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PathInfo {
    pub offset: u64,
    pub decompressed_size: u64,
    pub flags: u32,
    pub path: String,
}

impl PathInfo {
    pub const BASE_SIZE: usize = 20;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: Read,
    {
        Ok(Self {
            offset: reader.read_u64::<BigEndian>()?,
            decompressed_size: reader.read_u64::<BigEndian>()?,
            flags: reader.read_u32::<BigEndian>()?,
            path: reader.read_string()?,
        })
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        writer.write_u64::<BigEndian>(self.offset)?;
        writer.write_u64::<BigEndian>(self.decompressed_size)?;
        writer.write_u32::<BigEndian>(self.flags)?;

        writer.write_all(self.path.as_bytes())?;
        writer.write_u8(0)?;

        Ok(())
    }
}

impl Display for PathInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Offset:            {}",
            "",
            self.offset,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Decompressed size: {}",
            "",
            self.decompressed_size,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Flags:             {:#08x}",
            "",
            self.flags,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Path:              {}",
            "",
            self.path,
            indent = indent
        )?;

        Ok(())
    }
}

impl_try_from_into_vec!(PathInfo);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InfoBlock {
    pub decompressed_hash: [u8; 16],
    pub block_infos: Vec<BlockInfo>,
    pub path_infos: Vec<PathInfo>,
}

impl InfoBlock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut info_block = Self::new();

        reader.read_exact(&mut info_block.decompressed_hash)?;
        log::trace!("hash: {}", hex::encode(info_block.decompressed_hash));

        let block_count = reader.read_u32::<BigEndian>()?;
        log::trace!("{} asset block info(s)", block_count);

        for i in 0..block_count {
            let block_info = BlockInfo::read(reader)?;
            log::trace!("asset block info {}:\n{}", i, block_info);
            info_block.block_infos.push(block_info);
        }

        // asset path info
        let path_count = reader.read_u32::<BigEndian>()?;
        log::trace!("{} asset path info(s)", path_count);

        for i in 0..path_count {
            let path_info = PathInfo::read(reader)?;
            log::trace!("asset path info {}:\n{}", i, path_info);
            info_block.path_infos.push(path_info);
        }

        Ok(info_block)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        writer.write_all(&self.decompressed_hash)?;
        writer.write_u32::<BigEndian>(u32::try_from(self.block_infos.len())?)?;
        for block_info in self.block_infos.iter() {
            block_info.save(writer)?;
        }
        writer.write_u32::<BigEndian>(u32::try_from(self.path_infos.len())?)?;
        for path_info in self.path_infos.iter() {
            path_info.save(writer)?;
        }

        Ok(())
    }
}

impl Display for InfoBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Decompressed hash: {}",
            "",
            hex::encode(self.decompressed_hash)
        )?;
        writeln!(
            f,
            "{:indent$}Block info count:  {}",
            "",
            self.block_infos.len()
        )?;
        writeln!(
            f,
            "{:indent$}Path info count:   {}",
            "",
            self.path_infos.len()
        )?;

        writeln!(f, "{:indent$}Block infos:", "", indent = indent)?;
        for (i, block_info) in self.block_infos.iter().enumerate() {
            writeln!(f, "{:indent$}Block info {}:", "", i, indent = indent + 4)?;
            write!(f, "{:indent$}", block_info, indent = indent + 8)?;
        }

        writeln!(f, "{:indent$}Path infos:", "", indent = indent)?;
        for (i, path_info) in self.path_infos.iter().enumerate() {
            writeln!(f, "{:indent$}Path info {}:", "", i, indent = indent + 4)?;
            write!(f, "{:indent$}", path_info, indent = indent + 8)?;
        }

        Ok(())
    }
}

impl_try_from_into_vec!(InfoBlock);

#[cfg(test)]
#[ctor::ctor]
fn init() {
    mltd_utils::init_test_logger!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use mltd_utils::{rand_ascii_string, rand_bytes, rand_range};
    use std::io::{copy, Seek, SeekFrom};
    use std::iter::zip;

    #[test]
    fn test_block_info_read() {
        const SIZE: usize = BlockInfo::BASE_SIZE;
        let mut reader = rand_bytes(SIZE);
        let got = BlockInfo::read(&mut reader).unwrap();
        reader.set_position(0);

        assert_eq!(
            reader.read_u32::<BigEndian>().unwrap(),
            got.decompressed_size
        );
        assert_eq!(reader.read_u32::<BigEndian>().unwrap(), got.compressed_size);
        assert_eq!(reader.read_u16::<BigEndian>().unwrap(), got.flags);
    }

    #[test]
    fn test_path_info_read() {
        const SIZE: usize = PathInfo::BASE_SIZE;
        let mut reader = rand_bytes(SIZE);
        reader.seek(SeekFrom::End(0)).unwrap();
        copy(&mut rand_ascii_string(40), &mut reader).unwrap();
        reader.set_position(0);

        let got = PathInfo::read(&mut reader).unwrap();
        reader.set_position(0);

        assert_eq!(got.offset, reader.read_u64::<BigEndian>().unwrap());
        assert_eq!(
            got.decompressed_size,
            reader.read_u64::<BigEndian>().unwrap()
        );
        assert_eq!(got.flags, reader.read_u32::<BigEndian>().unwrap());
        assert_eq!(got.path, reader.read_string().unwrap());
    }

    #[test]
    fn test_info_block_read() {
        let mut reader = rand_bytes(16); // uncompressed hash
        reader.set_position(16);

        let block_count = rand_range(1usize..5usize);
        reader
            .write_u32::<BigEndian>(u32::try_from(block_count).unwrap())
            .unwrap();

        let mut block_infos = Vec::new();
        for _ in 0..block_count {
            let mut buf = rand_bytes(BlockInfo::BASE_SIZE);

            block_infos.push(BlockInfo::read(&mut buf).unwrap());
            buf.set_position(0);

            copy(&mut buf, &mut reader).unwrap();

            assert_eq!(buf.into_inner().len(), BlockInfo::BASE_SIZE);
        }

        let expected =
            u64::try_from(BlockInfo::BASE_SIZE).unwrap() * u64::try_from(block_count).unwrap();
        assert_eq!(reader.position(), expected + 20);

        let path_count = rand_range(2usize..10usize);
        reader
            .write_u32::<BigEndian>(u32::try_from(path_count).unwrap())
            .unwrap();

        let mut path_infos = Vec::new();
        for _ in 0..path_count {
            const SIZE: usize = PathInfo::BASE_SIZE;
            let mut buf = rand_bytes(SIZE);
            buf.set_position(u64::try_from(SIZE).unwrap());

            let mut path = rand_ascii_string(rand_range(30..40));
            copy(&mut path, &mut buf).unwrap();
            buf.set_position(0);

            path_infos.push(PathInfo::read(&mut buf).unwrap());
            buf.set_position(0);

            copy(&mut buf, &mut reader).unwrap();

            assert_eq!(buf.into_inner().len(), SIZE + path.into_inner().len());
        }
        reader.set_position(0);

        let got = InfoBlock::read(&mut reader).unwrap();
        reader.set_position(0);

        let mut buf = [0u8; 16];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(got.decompressed_hash, buf);

        assert_eq!(got.block_infos.len(), block_count);
        assert_eq!(got.path_infos.len(), path_count);

        for (expected, got) in zip(block_infos.iter(), got.block_infos.iter()) {
            assert_eq!(expected, got);
        }

        for (expected, got) in zip(path_infos.iter(), got.path_infos.iter()) {
            assert_eq!(expected, got);
        }
    }
}
