use super::{Asset, AssetType, Platform};
use crate::asset::ObjectInfo;
use crate::error::Error;
use crate::macros::impl_default;
use crate::traits::{ReadIntExt, ReadString, WriteIntExt};
use crate::utils::bool_to_yes_no;

use byteorder::{ReadBytesExt, WriteBytesExt};
use num_traits::{FromPrimitive, ToPrimitive};

use std::fmt::{Display, Formatter};
use std::io::Write;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub unity_version: String,
    pub target_platform: Platform,
    pub has_type_tree: bool,
    pub types_count: u32,
    pub types: Vec<AssetType>,
    pub big_id_enabled: bool,
    pub object_count: u32,
    pub object_infos: Vec<ObjectInfo>,

    pub(super) big_endian: bool,
    pub(super) data_offset: u64,
    pub(super) version: u32,
}

impl Metadata {
    pub fn new() -> Self {
        Self {
            unity_version: String::new(),
            target_platform: Platform::UnknownPlatform,
            has_type_tree: true,
            types_count: 0u32,
            types: Vec::new(),
            big_id_enabled: false,
            object_count: 0u32,
            object_infos: Vec::new(),

            big_endian: false,
            data_offset: 0u64,
            version: 0u32,
        }
    }

    pub fn read(asset: &mut Asset) -> Result<Self, Error> {
        let version = asset.header.version;
        let big_endian = asset.header.big_endian;

        let mut metadata = Self::new();
        metadata.big_endian = big_endian;
        metadata.data_offset = asset.header.data_offset;
        metadata.version = version;

        if version >= 7 {
            metadata.unity_version = asset.reader.read_string()?;
        }
        if version >= 8 {
            metadata.target_platform = Platform::from_u32(asset.reader.read_u32_by(big_endian)?)
                .ok_or_else(|| Error::UnknownSignature)?;
        }

        if version >= 13 {
            metadata.has_type_tree = asset.reader.read_u8()? > 0;
        }

        log::debug!("reading asset types");
        metadata.types_count = asset.reader.read_u32_by(big_endian)?;
        log::trace!("{} asset serized type(s)", metadata.types_count);

        for i in 0..metadata.types_count {
            let asset_type = AssetType::read(asset, false)?;
            log::trace!("asset type {}:\n{:#?}", i, asset_type);
            metadata.types.push(asset_type);
        }

        if (7..14).contains(&version) {
            log::trace!("reading big_id_enabled");
            metadata.big_id_enabled = asset.reader.read_i32_by(big_endian)? > 0i32;
        }

        log::debug!("reading objects");
        metadata.object_count = asset.reader.read_u32_by(big_endian)?;
        log::trace!("{} asset object(s)", metadata.object_count);

        for i in 0..metadata.object_count {
            let object_info = ObjectInfo::read(&mut asset.reader, &metadata)?;
            log::trace!("asset object {}:\n{:#?}", i, &object_info);

            metadata.object_infos.push(object_info);
        }

        Ok(metadata)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        if self.version >= 7 {
            writer.write_all(self.unity_version.as_bytes())?;
        }

        if self.version >= 8 {
            writer.write_u32_by(
                ToPrimitive::to_u32(&self.target_platform).ok_or_else(|| Error::UnknownPlatform)?,
                self.big_endian,
            )?;
        }

        if self.version >= 13 {
            writer.write_u8(u8::from(self.has_type_tree))?;
        }

        Ok(())
    }
}

impl Display for Metadata {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Unity version:     {}",
            "",
            self.unity_version,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Target platform:   {:?}",
            "",
            self.target_platform,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Has type tree?     {}",
            "",
            bool_to_yes_no(self.has_type_tree),
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Big ID enabled?    {}",
            "",
            bool_to_yes_no(self.big_id_enabled),
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Number of types:   {}",
            "",
            self.types_count,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Number of objects: {}",
            "",
            self.object_count,
            indent = indent
        )?;

        writeln!(f, "{:indent$}Types:", "", indent = indent)?;
        for (i, asset_type) in self.types.iter().enumerate() {
            writeln!(f, "{:indent$}Type {}:", "", i, indent = indent + 4)?;
            writeln!(f, "{:indent$}", asset_type, indent = indent + 8)?;
        }

        writeln!(f, "{:indent$}Object infos:", "", indent = indent)?;
        for (i, object_info) in self.object_infos.iter().enumerate() {
            writeln!(f, "{:indent$}Object {}:", "", i, indent = indent + 4)?;
            writeln!(f, "{:indent$}", object_info, indent = indent + 8)?;
        }

        Ok(())
    }
}

impl_default!(Metadata);
