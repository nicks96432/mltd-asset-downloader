use crate::error::UnityError;
use crate::macros::impl_try_from_into_vec;
use crate::traits::{ReadExact, UnityIO};
use crate::{AssetBundleFlags, AssetBundleVersion};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::fmt::Display;
use std::io::{Read, Seek, Write};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AssetBundleSignature {
    UnityFS,
    UnityWeb,
    UnityRaw,
    UnityArchive,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssetBundleHeader {
    pub signature: AssetBundleSignature,
    pub version: u32,
    pub version_player: String,
    pub version_engine: AssetBundleVersion,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnityFSHeader {
    pub bundle_size: u64,
    pub compressed_size: u32,
    pub decompressed_size: u32,
    pub flags: AssetBundleFlags,
}

impl FromStr for AssetBundleSignature {
    type Err = UnityError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "UnityFS" => Ok(Self::UnityFS),
            "UnityWeb" => Ok(Self::UnityWeb),
            "UnityRaw" => Ok(Self::UnityRaw),
            "UnityArchive" => Ok(Self::UnityArchive),
            _ => Err(UnityError::InvalidSignature),
        }
    }
}

impl Display for AssetBundleSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnityFS => write!(f, "UnityFS"),
            Self::UnityWeb => write!(f, "UnityWeb"),
            Self::UnityRaw => write!(f, "UnityRaw"),
            Self::UnityArchive => write!(f, "UnityArchive"),
        }
    }
}

impl UnityIO for AssetBundleHeader {
    fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, UnityError> {
        let signature = reader.read_string()?;
        log::trace!("signature: {}", signature);

        Ok(Self {
            signature: AssetBundleSignature::from_str(&signature)?,
            version: reader.read_u32::<BigEndian>()?,
            version_player: reader.read_string()?,
            version_engine: AssetBundleVersion::from_str(&reader.read_string()?)?,
        })
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), UnityError> {
        writer.write_all(self.signature.to_string().as_bytes())?;
        writer.write_u8(0)?;

        writer.write_u32::<BigEndian>(self.version)?;

        writer.write_all(self.version_player.as_bytes())?;
        writer.write_u8(0)?;

        writer.write_all(self.version_engine.to_string().as_bytes())?;
        writer.write_u8(0)?;

        Ok(())
    }
}

impl UnityIO for UnityFSHeader {
    fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, UnityError> {
        Ok(Self {
            bundle_size: reader.read_u64::<BigEndian>()?,
            compressed_size: reader.read_u32::<BigEndian>()?,
            decompressed_size: reader.read_u32::<BigEndian>()?,
            flags: AssetBundleFlags::new(reader.read_u32::<BigEndian>()?),
        })
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), UnityError> {
        writer.write_u64::<BigEndian>(self.bundle_size)?;
        writer.write_u32::<BigEndian>(self.compressed_size)?;
        writer.write_u32::<BigEndian>(self.decompressed_size)?;
        writer.write_u32::<BigEndian>(self.flags.bits)?;

        Ok(())
    }
}

impl_try_from_into_vec!(AssetBundleHeader);
impl_try_from_into_vec!(UnityFSHeader);

#[cfg(test)]
#[ctor::ctor]
fn init() {
    mltd_utils::init_test_logger!();
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_from_reader() {
    }
}
