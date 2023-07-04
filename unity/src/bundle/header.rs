use super::{Flags, Signature};
use crate::error::Error;
use crate::macros::{impl_default, impl_try_from_into_vec};
use crate::traits::ReadString;
use crate::utils::Version;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use std::fmt::Display;
use std::io::{Read, Write};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnityFSHeader {
    pub signature: Signature,
    pub version_format: u32,
    pub version_target: Version,
    pub version_engine: Version,
    pub bundle_size: u64,
    pub info_block_compressed_size: u32,
    pub info_block_decompressed_size: u32,
    pub flags: Flags,
}

impl UnityFSHeader {
    pub fn new() -> Self {
        Self {
            signature: Signature::UnityFS,
            version_format: 0u32,
            version_target: Version::new(),
            version_engine: Version::new(),
            bundle_size: 0u64,
            info_block_compressed_size: 0u32,
            info_block_decompressed_size: 0u32,
            flags: Flags::new(),
        }
    }

    pub fn read<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: Read,
    {
        let signature = reader.read_string()?;
        log::trace!("signature: {}", signature);

        Ok(Self {
            signature: Signature::from_str(&signature)?,
            version_format: reader.read_u32::<BigEndian>()?,
            version_target: Version::from_str(&reader.read_string()?)?,
            version_engine: Version::from_str(&reader.read_string()?)?,
            bundle_size: reader.read_u64::<BigEndian>()?,
            info_block_compressed_size: reader.read_u32::<BigEndian>()?,
            info_block_decompressed_size: reader.read_u32::<BigEndian>()?,
            flags: Flags(reader.read_u32::<BigEndian>()?),
        })
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        writer.write_all(self.signature.to_string().as_bytes())?;
        writer.write_u8(0)?;

        writer.write_u32::<BigEndian>(self.version_format)?;

        writer.write_all(self.version_target.to_string().as_bytes())?;
        writer.write_u8(0)?;

        writer.write_all(self.version_engine.to_string().as_bytes())?;
        writer.write_u8(0)?;

        writer.write_u64::<BigEndian>(self.bundle_size)?;
        writer.write_u32::<BigEndian>(self.info_block_compressed_size)?;
        writer.write_u32::<BigEndian>(self.info_block_decompressed_size)?;
        writer.write_u32::<BigEndian>(self.flags.0)?;

        Ok(())
    }
}

impl Display for UnityFSHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Signature:       {}",
            "",
            self.signature,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Format version:  {}",
            "",
            self.version_format,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Target version:  {}",
            "",
            self.version_target,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Engine version:  {}",
            "",
            self.version_engine,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Bundle size:     {}",
            "",
            self.bundle_size,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Info block size: {} (decompressed {})",
            "",
            self.info_block_compressed_size,
            self.info_block_decompressed_size,
            indent = indent
        )?;

        write!(f, "{:indent$}{}", "", self.flags, indent = indent)?;

        Ok(())
    }
}

impl_default!(UnityFSHeader);
impl_try_from_into_vec!(UnityFSHeader);

#[cfg(test)]
#[ctor::ctor]
fn init() {
    mltd_utils::init_test_logger!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use mltd_utils::{rand_bytes, rand_range};
    use std::io::{Cursor, Seek};

    #[test]
    fn test_unityfs_header_read() {
        let mut buf = Cursor::new(Vec::new());

        let signature = Signature::UnityFS;
        buf.write_all(signature.to_string().as_bytes()).unwrap();
        buf.write_u8(0u8).unwrap();

        let version = rand_range(1u32..=7u32);
        buf.write_u32::<BigEndian>(version).unwrap();
        log::trace!("chosen version: {}", version);

        let version_player = "5.x.x";
        buf.write_all(version_player.as_bytes()).unwrap(); // version_player
        buf.write_u8(0u8).unwrap();

        let choices = vec![
            "5.6.7",
            "2017.4.40f1",
            "2018.3.0f2",
            "2021.3.18f1",
            "2023.1.0a4",
        ];
        let version_engine = choices[rand_range(0usize..choices.len())];
        log::trace!("chosen version_engine: {}", version_engine);
        buf.write_all(version_engine.as_bytes()).unwrap();
        buf.write_u8(0u8).unwrap();

        buf.write_all(rand_bytes(20).into_inner().as_slice())
            .unwrap();
        buf.set_position(0);

        let unityfs_header = UnityFSHeader::read(&mut buf).unwrap();
        buf.seek(std::io::SeekFrom::End(-20)).unwrap();

        assert_eq!(unityfs_header.signature, signature);
        assert_eq!(unityfs_header.version_format, version);
        assert_eq!(unityfs_header.version_target.to_string(), version_player);
        assert_eq!(unityfs_header.version_engine.to_string(), version_engine);

        assert_eq!(
            unityfs_header.bundle_size,
            buf.read_u64::<BigEndian>().unwrap()
        );
        assert_eq!(
            unityfs_header.info_block_compressed_size,
            buf.read_u32::<BigEndian>().unwrap()
        );
        assert_eq!(
            unityfs_header.info_block_decompressed_size,
            buf.read_u32::<BigEndian>().unwrap()
        );
        assert_eq!(unityfs_header.flags.0, buf.read_u32::<BigEndian>().unwrap());
    }
}
