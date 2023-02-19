use crate::asset_bundle::ReadExact;
use crate::compression::CompressionMethod;
use crate::error::UnityError;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::fmt::Debug;
use std::io::{Read, Seek, Write};

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssetBlockInfo {
    pub decompressed_size: u32,
    pub compressed_size: u32,
    pub flags: u16,
}

#[repr(C, packed)]
#[derive(Clone, Copy, PartialEq)]
pub struct AssetPathInfo {
    pub offset: u64,
    pub decompressed_size: u64,
    pub flags: u32,
    pub path: [u8; 37],
}

#[derive(Debug, Clone, PartialEq)]
pub struct InfoBlock {
    pub hash: [u8; 16],
    pub block_count: u32,
    pub block_infos: Vec<AssetBlockInfo>,
    pub path_count: u32,
    pub path_infos: Vec<AssetPathInfo>,
}

impl Debug for AssetPathInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let offset = self.offset;
        let decompressed_size = self.decompressed_size;
        let flags = self.flags;

        f.debug_struct("AssetPathInfo")
            .field("offset", &offset)
            .field("decompressed_size", &decompressed_size)
            .field("flags", &flags)
            .field("path", &String::from_utf8_lossy(&self.path))
            .finish()
    }
}

impl AssetBlockInfo {
    pub fn new() -> Self {
        Self {
            decompressed_size: 0u32,
            compressed_size: 0u32,
            flags: 0u16,
        }
    }

    /// Reads the struct from `reader`, assuming that the data start
    /// from current position.
    ///
    /// # Errors
    ///
    /// This function will return [`UnityError::FileError`] if `reader`
    /// is unavailable.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::io::Cursor;
    /// use unity::AssetBlockInfo;
    ///
    /// let mut file = Cursor::new(vec![0u8; 10]);
    /// let header = AssetBlockInfo::from_reader(&mut file).unwrap();
    ///
    /// let decompressed_size = header.decompressed_size;
    /// assert_eq!(decompressed_size, 0);
    /// ```
    pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self, UnityError> {
        Ok(Self {
            decompressed_size: reader.read_u32::<BigEndian>()?,
            compressed_size: reader.read_u32::<BigEndian>()?,
            flags: reader.read_u16::<BigEndian>()?,
        })
    }
    pub fn write<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_u32::<BigEndian>(self.decompressed_size)?;
        writer.write_u32::<BigEndian>(self.compressed_size)?;
        writer.write_u16::<BigEndian>(self.flags)?;

        Ok(())
    }

    /// Returns the compression method of the data block of
    /// this [`AssetBlockInfo`].
    ///
    /// # Errors
    ///
    /// This function will return [`UnityError::UnknownCompressionMethod`] if
    /// the compression method is unknown.
    pub fn compression_method(&self) -> Result<CompressionMethod, UnityError> {
        let value = u32::from(self.flags & 0x3f);
        Ok(CompressionMethod::try_from(value)?)
    }
}

impl AssetPathInfo {
    pub fn new() -> Self {
        Self {
            offset: 0u64,
            decompressed_size: 0u64,
            flags: 0u32,
            path: [0u8; 37],
        }
    }

    /// Reads the struct from `reader`, assuming that the data start
    /// from current position.
    ///
    /// # Errors
    ///
    /// This function will return [`UnityError::FileError`] if `reader`
    /// is unavailable.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::io::Cursor;
    /// use unity::AssetPathInfo;
    ///
    /// let mut file = Cursor::new(vec![0u8; 57]);
    /// let header = AssetPathInfo::from_reader(&mut file).unwrap();
    ///
    /// let decompressed_size = header.decompressed_size;
    /// assert_eq!(decompressed_size, 0);
    /// ```
    pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self, UnityError> {
        Ok(Self {
            offset: reader.read_u64::<BigEndian>()?,
            decompressed_size: reader.read_u64::<BigEndian>()?,
            flags: reader.read_u32::<BigEndian>()?,
            path: reader.read_str::<37>()?,
        })
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_u64::<BigEndian>(self.offset)?;
        writer.write_u64::<BigEndian>(self.decompressed_size)?;
        writer.write_u32::<BigEndian>(self.flags)?;
        writer.write_all(&self.path)?;

        Ok(())
    }
}

impl InfoBlock {
    pub fn new() -> Self {
        Self {
            hash: [0u8; 16],
            block_count: 1u32,
            block_infos: vec![AssetBlockInfo::new()],
            path_count: 0u32,
            path_infos: Vec::new(),
        }
    }

    /// Reads the struct from `reader`, assuming that the **decompressed** data
    /// start from current position.
    ///
    /// # Errors
    ///
    /// This function will return [`UnityError::FileError`] if `reader`
    /// is unavailable.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::io::Cursor;
    /// use unity::InfoBlock;
    ///
    /// let mut file = Cursor::new(vec![0u8; 24]);
    /// let header = InfoBlock::from_reader(&mut file).unwrap();
    ///
    /// let path_count = header.path_count;
    /// assert_eq!(path_count, 0);
    /// ```
    pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self, UnityError> {
        let hash = reader.read_str::<16>()?;
        log::trace!("hash: {:x?}", hash);

        let block_count = reader.read_u32::<BigEndian>()?;
        log::trace!("{} asset block info(s)", block_count);

        let mut block_infos = Vec::<AssetBlockInfo>::with_capacity(block_count.try_into()?);
        for i in 0..block_count {
            let block_info = AssetBlockInfo::from_reader(reader)?;
            log::trace!("asset block info {}:\n{:#?}", i, block_info);
            block_infos.push(block_info);
        }

        // asset path info
        let path_count = reader.read_u32::<BigEndian>()?;
        log::trace!("{} asset path info(s)", path_count);

        let mut path_infos = Vec::<AssetPathInfo>::with_capacity(path_count.try_into()?);
        for i in 0..path_count {
            let path_info = AssetPathInfo::from_reader(reader)?;
            log::trace!("asset path info {}:\n{:#?}", i, path_info);
            path_infos.push(path_info);
        }

        Ok(Self {
            hash,
            block_count,
            block_infos,
            path_count,
            path_infos,
        })
    }

    pub fn write<W>(&self, writer: &mut W) -> std::io::Result<()>
    where
        W: Write,
    {
        writer.write_all(&self.hash)?;
        writer.write_u32::<BigEndian>(self.block_count)?;
        for block_info in self.block_infos.iter() {
            block_info.write(writer)?;
        }
        writer.write_u32::<BigEndian>(self.path_count)?;
        for path_info in self.path_infos.iter() {
            path_info.write(writer)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::UnityError;

    #[test]
    fn test_asset_block_info() -> Result<(), UnityError> {
        todo!()
    }

    #[test]
    fn test_asset_path_info() -> Result<(), UnityError> {
        todo!()
    }

    #[test]
    fn test_info_block() -> Result<(), UnityError> {
        todo!()
    }
}
