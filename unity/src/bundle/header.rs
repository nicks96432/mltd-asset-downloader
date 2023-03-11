use super::{Flags, Signature, Version};
use crate::error::Error;
use crate::macros::{impl_default, impl_try_from_into_vec};
use crate::traits::ReadString;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    pub signature: Signature,
    pub version: u32,
    pub version_player: String,
    pub version_engine: Version,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnityFSHeader {
    pub bundle_size: u64,
    pub compressed_size: u32,
    pub decompressed_size: u32,
    pub flags: Flags,
}

impl Header {
    pub fn read<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: Read,
    {
        let signature = reader.read_string()?;
        log::trace!("signature: {}", signature);

        Ok(Self {
            signature: Signature::from_str(&signature)?,
            version: reader.read_u32::<BigEndian>()?,
            version_player: reader.read_string()?,
            version_engine: Version::from_str(&reader.read_string()?)?,
        })
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
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
            flags: Flags::new(0u32),
        }
    }

    pub fn read<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: Read,
    {
        Ok(Self {
            bundle_size: reader.read_u64::<BigEndian>()?,
            compressed_size: reader.read_u32::<BigEndian>()?,
            decompressed_size: reader.read_u32::<BigEndian>()?,
            flags: Flags::new(reader.read_u32::<BigEndian>()?),
        })
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        writer.write_u64::<BigEndian>(self.bundle_size)?;
        writer.write_u32::<BigEndian>(self.compressed_size)?;
        writer.write_u32::<BigEndian>(self.decompressed_size)?;
        writer.write_u32::<BigEndian>(self.flags.bits)?;

        Ok(())
    }
}

impl_default!(UnityFSHeader);

impl_try_from_into_vec!(Header);
impl_try_from_into_vec!(UnityFSHeader);

#[cfg(test)]
#[ctor::ctor]
fn init() {
    mltd_utils::init_test_logger!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use mltd_utils::{rand_ascii_string, rand_bytes, rand_range};
    use std::io::{copy, Cursor};

    #[test]
    fn test_header_read() {
        let mut buf = Cursor::new(Vec::new());
        let choices = vec![
            Signature::UnityFS,
            Signature::UnityWeb,
            Signature::UnityRaw,
            Signature::UnityArchive,
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
        let version_engine = Version::from_str(&version_engine).unwrap();
        log::trace!("chosen version_engine: {}", version_engine);
        buf.set_position(0);

        let asset_bundle_header = Header::read(&mut buf).unwrap();
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
