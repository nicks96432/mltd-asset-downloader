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

impl UnityFSHeader {
    pub const BASE_SIZE: usize = 20;

    pub fn new() -> Self {
        Self {
            bundle_size: 0u64,
            compressed_size: 0u32,
            decompressed_size: 0u32,
            flags: AssetBundleFlags::new(0u32),
        }
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
    use crate::traits::{ReadExact, UnityIO};
    use crate::{AssetBundleHeader, AssetBundleSignature, AssetBundleVersion, UnityFSHeader};
    use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
    use mltd_utils::{rand_ascii_string, rand_bytes, rand_range};
    use std::io::{copy, Cursor, Write};
    use std::str::FromStr;

    #[test]
    fn test_asset_bundle_header_read() {
        let mut buf = Cursor::new(Vec::new());
        let choices = vec![
            AssetBundleSignature::UnityFS,
            AssetBundleSignature::UnityWeb,
            AssetBundleSignature::UnityRaw,
            AssetBundleSignature::UnityArchive,
        ];
        let signature = choices[rand_range(0..choices.len())];
        log::trace!("chosen signature: {}", signature);

        buf.write_all(signature.to_string().as_bytes()).unwrap();
        buf.write_u8(0u8).unwrap();

        let version = rand_range(6u32..=7u32);
        buf.write_u32::<BigEndian>(version).unwrap(); // version
        log::trace!("chosen version: {}", version);

        let mut version_player = rand_ascii_string(10);

        copy(&mut version_player, &mut buf).unwrap(); // version_player
        version_player.set_position(0);

        let version_player = version_player.read_string().unwrap();

        let choices: Vec<&[u8]> = vec![
            b"5.6.7\0",
            b"2017.4.40f1\0",
            b"2018.3.0f2\0",
            b"2021.3.18f1\0",
            b"2023.1.0a4\0",
        ];
        let mut version_engine = choices[rand_range(0..choices.len())];
        buf.write_all(version_engine).unwrap();
        let version_engine = version_engine.read_string().unwrap();
        let version_engine = AssetBundleVersion::from_str(&version_engine).unwrap();
        log::trace!("chosen version_engine: {}", version_engine);
        buf.set_position(0);

        let asset_bundle_header = AssetBundleHeader::read(&mut buf).unwrap();
        buf.set_position(u64::try_from(signature.to_string().len() + 1).unwrap());

        assert_eq!(asset_bundle_header.signature, signature);
        assert_eq!(asset_bundle_header.version, version);
        assert_eq!(asset_bundle_header.version_player, version_player);
        assert_eq!(asset_bundle_header.version_engine, version_engine);
    }

    #[test]
    fn test_unityfs_header_read() {
        let mut buf = rand_bytes(UnityFSHeader::BASE_SIZE);

        let unityfs_header = UnityFSHeader::read(&mut buf).unwrap();
        buf.set_position(0);
        assert_eq!(
            unityfs_header.bundle_size,
            buf.read_u64::<BigEndian>().unwrap()
        );
        assert_eq!(
            unityfs_header.compressed_size,
            buf.read_u32::<BigEndian>().unwrap()
        );
        assert_eq!(
            unityfs_header.decompressed_size,
            buf.read_u32::<BigEndian>().unwrap()
        );
        assert_eq!(
            unityfs_header.flags.bits,
            buf.read_u32::<BigEndian>().unwrap()
        );
    }
}
