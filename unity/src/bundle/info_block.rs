use crate::compression::Method as CompressionMethod;
use crate::error::Error;
use crate::macros::{impl_default, impl_try_from_into_vec};
use crate::traits::ReadString;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::fmt::Debug;
use std::io::{Read, Write};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockInfo {
    pub decompressed_size: u32,
    pub compressed_size: u32,
    pub flags: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathInfo {
    pub offset: u64,
    pub decompressed_size: u64,
    pub flags: u32,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InfoBlock {
    pub decompressed_hash: [u8; 16],
    pub block_count: u32,
    pub block_infos: Vec<BlockInfo>,
    pub path_count: u32,
    pub path_infos: Vec<PathInfo>,
}

impl BlockInfo {
    pub const BASE_SIZE: usize = 10;

    pub fn new() -> Self {
        Self {
            decompressed_size: 0u32,
            compressed_size: 0u32,
            flags: 0u16,
        }
    }

    /// Returns the compression method of the data block of
    /// this [`AssetBlockInfo`].
    ///
    /// # Errors
    ///
    /// This function will return [`UnityError::UnknownCompressionMethod`] if
    /// the compression method is unknown.
    pub fn compression_method(&self) -> Result<CompressionMethod, Error> {
        let value = u32::from(self.flags & 0x3f);

        CompressionMethod::try_from(value)
    }

    pub fn read<R: Read>(reader: &mut R) -> Result<Self, Error> {
        Ok(Self {
            decompressed_size: reader.read_u32::<BigEndian>()?,
            compressed_size: reader.read_u32::<BigEndian>()?,
            flags: reader.read_u16::<BigEndian>()?,
        })
    }

    pub fn save<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_u32::<BigEndian>(self.decompressed_size)?;
        writer.write_u32::<BigEndian>(self.compressed_size)?;
        writer.write_u16::<BigEndian>(self.flags)?;

        Ok(())
    }
}

impl PathInfo {
    pub const BASE_SIZE: usize = 20;

    pub fn new() -> Self {
        Self {
            offset: 0u64,
            decompressed_size: 0u64,
            flags: 0u32,
            path: String::new(),
        }
    }

    pub fn read<R: Read>(reader: &mut R) -> Result<Self, Error> {
        Ok(Self {
            offset: reader.read_u64::<BigEndian>()?,
            decompressed_size: reader.read_u64::<BigEndian>()?,
            flags: reader.read_u32::<BigEndian>()?,
            path: reader.read_string()?,
        })
    }

    pub fn save<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_u64::<BigEndian>(self.offset)?;
        writer.write_u64::<BigEndian>(self.decompressed_size)?;
        writer.write_u32::<BigEndian>(self.flags)?;

        writer.write_all(self.path.as_bytes())?;
        writer.write_u8(0)?;

        Ok(())
    }
}

impl InfoBlock {
    pub fn new() -> Self {
        Self {
            decompressed_hash: [0u8; 16],
            block_count: 1u32,
            block_infos: vec![BlockInfo::new()],
            path_count: 0u32,
            path_infos: Vec::new(),
        }
    }

    pub fn read<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let mut decompressed_hash = [0u8; 16];
        reader.read_exact(&mut decompressed_hash)?;
        log::trace!("hash: {:?}", decompressed_hash);

        let block_count = reader.read_u32::<BigEndian>()?;
        log::trace!("{} asset block info(s)", block_count);

        let mut block_infos = Vec::<BlockInfo>::with_capacity(usize::try_from(block_count)?);
        for i in 0..block_count {
            let block_info = BlockInfo::read(reader)?;
            log::trace!("asset block info {}:\n{:#?}", i, block_info);
            block_infos.push(block_info);
        }

        // asset path info
        let path_count = reader.read_u32::<BigEndian>()?;
        log::trace!("{} asset path info(s)", path_count);

        let mut path_infos = Vec::<PathInfo>::with_capacity(usize::try_from(path_count)?);
        for i in 0..path_count {
            let path_info = PathInfo::read(reader)?;
            log::trace!("asset path info {}:\n{:#?}", i, path_info);
            path_infos.push(path_info);
        }

        Ok(Self {
            decompressed_hash,
            block_count,
            block_infos,
            path_count,
            path_infos,
        })
    }

    pub fn save<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_all(&self.decompressed_hash)?;
        writer.write_u32::<BigEndian>(self.block_count)?;
        for block_info in self.block_infos.iter() {
            block_info.save(writer)?;
        }
        writer.write_u32::<BigEndian>(self.path_count)?;
        for path_info in self.path_infos.iter() {
            path_info.save(writer)?;
        }

        Ok(())
    }
}

impl_default!(BlockInfo);
impl_default!(PathInfo);
impl_default!(InfoBlock);

impl_try_from_into_vec!(BlockInfo);
impl_try_from_into_vec!(PathInfo);
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

        let block_count = rand_range(1u32..5u32);
        reader.write_u32::<BigEndian>(block_count).unwrap();

        let mut block_infos = Vec::new();
        for _ in 0..block_count {
            const SIZE: usize = BlockInfo::BASE_SIZE;
            let mut buf = rand_bytes(SIZE);

            block_infos.push(BlockInfo::read(&mut buf).unwrap());
            buf.set_position(0);

            copy(&mut buf, &mut reader).unwrap();

            assert_eq!(buf.into_inner().len(), SIZE);
        }

        let expected = u64::try_from(BlockInfo::BASE_SIZE).unwrap() * u64::from(block_count);
        assert_eq!(reader.position(), expected + 20);

        let path_count = rand_range(2u32..10u32);
        reader.write_u32::<BigEndian>(path_count).unwrap();

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
        assert_eq!(got.block_count, block_count);
        for i in 0..usize::try_from(block_count).unwrap() {
            assert_eq!(got.block_infos[i], block_infos[i]);
        }
        assert_eq!(got.path_count, path_count);
        for i in 0..usize::try_from(path_count).unwrap() {
            assert_eq!(got.path_infos[i], path_infos[i]);
        }
    }
}