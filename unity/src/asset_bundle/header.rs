use crate::asset_bundle::ReadExact;
use crate::compression::CompressionMethod;
use crate::error::UnityError;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::fmt::Debug;
use std::io::{Read, Seek, Write};

#[repr(C, packed)]
#[derive(Clone, Copy, PartialEq)]
pub struct AssetBundleHeader {
    pub signature: [u8; 8],
    pub version: u32,
    pub min_version: [u8; 6],
    pub build_version: [u8; 12],
    pub bundle_size: u64,
    pub compressed_info_block_size: u32,
    pub decompressed_info_block_size: u32,
    pub flags: u32,
}

impl Debug for AssetBundleHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let version = self.version;
        let bundle_size = self.bundle_size;
        let compressed_info_block_size = self.compressed_info_block_size;
        let decompressed_info_block_size = self.decompressed_info_block_size;
        let flags = self.flags;

        f.debug_struct("AssetBundleHeader")
            .field("signature", &String::from_utf8_lossy(&self.signature))
            .field("version", &version)
            .field("min_version", &String::from_utf8_lossy(&self.min_version))
            .field(
                "build_version",
                &String::from_utf8_lossy(&self.build_version),
            )
            .field("bundle_size", &bundle_size)
            .field("compressed_info_block_size", &compressed_info_block_size)
            .field(
                "decompressed_info_block_size",
                &decompressed_info_block_size,
            )
            .field("flags", &format!("{:08x}", flags))
            .finish()
    }
}

impl AssetBundleHeader {
    const SIGNATURE: &[u8; 8] = b"UnityFS\0";

    /// Reads the struct from `reader`, assuming that the data start
    /// from current position.
    ///
    /// # Errors
    ///
    /// This function will return [`UnityError::FileError`] if `reader` is unavailable.
    ///
    /// This function will return [`UnityError::InvalidSignature`] if the signature is invalid.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use unity::AssetBundleHeader;
    /// use unity::UnityError;
    ///
    /// fn main() -> Result<(), UnityError> {
    ///     let mut file = File::open("bundle.unity3d")?;
    ///     let header = AssetBundleHeader::from_reader(&mut file)?;
    ///
    ///     assert_eq!(header.signature, *b"UnityFS\0");
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self, UnityError> {
        let signature = reader.read_str::<8>()?;
        log::trace!("signature: {}", String::from_utf8_lossy(&signature));

        if signature != *Self::SIGNATURE {
            return Err(UnityError::InvalidSignature);
        }

        Ok(Self {
            signature,
            version: reader.read_u32::<BigEndian>()?,
            min_version: reader.read_str::<6>()?,
            build_version: reader.read_str::<12>()?,
            bundle_size: reader.read_u64::<BigEndian>()?,
            compressed_info_block_size: reader.read_u32::<BigEndian>()?,
            decompressed_info_block_size: reader.read_u32::<BigEndian>()?,
            flags: reader.read_u32::<BigEndian>()?,
        })
    }

    /// Returns the compression method of this [`AssetBundleHeader`].
    ///
    /// # Errors
    ///
    /// This function will return [`UnityError::UnknownCompressionMethod`] if
    /// the compression method is unknown.
    pub fn compression_method(&self) -> Result<CompressionMethod, UnityError> {
        let value = u32::from(self.flags & 0x3f);
        Ok(CompressionMethod::try_from(value)?)
    }

    /// Returns whether the bundle file has [`InfoBlock`].
    ///
    /// [`InfoBlock`]: crate::InfoBlock
    pub fn has_info_block(&self) -> bool {
        self.flags & 0x40 != 0
    }

    /// Returns whether the [`InfoBlock`] is at the end of this bundle file.
    ///
    /// [`InfoBlock`]: crate::InfoBlock
    pub fn info_block_end(&self) -> bool {
        self.flags & 0x80 != 0
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.signature)?;
        writer.write_u32::<BigEndian>(self.version)?;
        writer.write_all(&self.min_version)?;
        writer.write_all(&self.build_version)?;
        writer.write_u64::<BigEndian>(self.bundle_size)?;
        writer.write_u32::<BigEndian>(self.compressed_info_block_size)?;
        writer.write_u32::<BigEndian>(self.decompressed_info_block_size)?;
        writer.write_u32::<BigEndian>(self.flags)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_from_reader() {
        todo!()
    }

    #[test]
    fn test_compression_method() {
        todo!()
    }
}
