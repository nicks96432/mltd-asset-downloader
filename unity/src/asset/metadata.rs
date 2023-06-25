use super::{Asset, ClassType, Platform};
use crate::asset::ClassInfo;
use crate::error::Error;
use crate::traits::{ReadIntExt, ReadString, SeekAlign, WriteIntExt};
use crate::utils::{bool_to_yes_no, Version};

use byteorder::{ReadBytesExt, WriteBytesExt};
use num_traits::{FromPrimitive, ToPrimitive};

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, Write};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScriptInfo {
    pub file_id: i32,
    pub path_id: i64,
}

impl ScriptInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, metadata: &Metadata) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut info = Self::new();

        info.file_id = reader.read_i32_by(metadata.big_endian)?;
        info.path_id = match metadata.version {
            v if v >= 14 => {
                reader.seek_align(4)?;
                reader.read_i64_by(metadata.big_endian)?
            }
            _ => reader.read_i32_by(metadata.big_endian)?.into(),
        };

        Ok(info)
    }

    pub fn save<W>(&self, _writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        unimplemented!()
    }
}

impl Display for ScriptInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}File id: {:8}",
            "",
            self.file_id,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}path id: {:8}",
            "",
            self.path_id,
            indent = indent
        )?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExternalFileInfo {
    pub guid: [u8; 16],
    pub file_type: i32,
    pub path: String,
    pub temp_empty: String,
}

impl ExternalFileInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, metadata: &Metadata) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut info = ExternalFileInfo::new();

        if metadata.version >= 6 {
            info.temp_empty = reader.read_string()?;
        }
        if metadata.version >= 5 {
            reader.read_exact(&mut info.guid)?;
            info.file_type = reader.read_i32_by(metadata.big_endian)?;
        }

        info.path = reader.read_string()?;

        Ok(info)
    }

    pub fn save<W>(&self, _writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        unimplemented!()
    }
}

impl Display for ExternalFileInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(f, "{:indent$}GUID: {}", "", hex::encode(self.guid))?;
        writeln!(f, "{:indent$}type: {}", "", self.file_type)?;
        writeln!(f, "{:indent$}path: {}", "", self.path)?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Metadata {
    pub unity_version: Version,
    pub target_platform: Platform,
    pub has_type_tree: bool,
    pub big_id_enabled: bool,
    pub class_infos: Vec<ClassInfo>,
    pub script_infos: Vec<ScriptInfo>,
    pub externa_file_infos: Vec<ExternalFileInfo>,
    pub ref_types: Vec<ClassType>,
    pub user_information: String,

    pub(crate) big_endian: bool,
    pub(crate) data_offset: u64,
    pub(crate) version: u32,
}

impl Metadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read(asset: &mut Asset) -> Result<Self, Error> {
        let version = asset.header.version;
        let big_endian = asset.header.big_endian;

        let mut metadata = Self::new();
        metadata.big_endian = big_endian;
        metadata.data_offset = asset.header.data_offset;
        metadata.version = version;

        if version >= 7 {
            metadata.unity_version = Version::from_str(&asset.reader.read_string()?)?;
        }
        if version >= 8 {
            metadata.target_platform = Platform::from_u32(asset.reader.read_u32_by(big_endian)?)
                .ok_or_else(|| Error::UnknownPlatform)?;
        }

        if version >= 13 {
            metadata.has_type_tree = asset.reader.read_u8()? > 0;
            if metadata.has_type_tree {
                log::trace!("this asset has type tree");
            }
        }

        log::debug!("reading class types");
        let types_count = asset.reader.read_u32_by(big_endian)?;
        log::trace!("{} class type(s)", types_count);

        let mut class_types = HashMap::new();
        for i in 0usize..usize::try_from(types_count)? {
            let class_type = ClassType::read(&mut asset.reader, &metadata, false)?;
            log::trace!("asset type {}:\n{}", i, class_type);
            class_types.insert(i, class_type);
        }

        if (7u32..14u32).contains(&version) {
            log::trace!("reading big_id_enabled");
            metadata.big_id_enabled = asset.reader.read_i32_by(big_endian)? > 0i32;
        }

        log::debug!("reading object infos");
        let object_count = asset.reader.read_u32_by(big_endian)?;
        log::trace!("{} asset object info(s)", object_count);

        for i in 0usize..usize::try_from(object_count)? {
            let mut object_info = ClassInfo::read(&mut asset.reader, &metadata)?;
            let key = match version >= 16 {
                true => usize::try_from(object_info.type_id)?,
                false => i,
            };

            match class_types.get(&key) {
                Some(class_type) => {
                    if version >= 16 {
                        object_info.class_id = class_type.class_id;
                    }
                    if version >= 17 {
                        object_info.stripped = class_type.stripped;
                        object_info.script_index = class_type.script_index;
                    }
                    object_info.class_type = class_type.clone();
                }
                None => {
                    log::error!("class_type of index {} not found", key)
                }
            };

            log::trace!("asset object {}:\n{}", i, &object_info);
            metadata.class_infos.push(object_info);
        }

        if version >= 11 {
            log::debug!("reading asset scripts");
            let script_count = asset.reader.read_u32_by(big_endian)?;
            log::trace!("{} asset script(s)", script_count);

            for i in 0usize..usize::try_from(script_count)? {
                let script_info = ScriptInfo::read(&mut asset.reader, &metadata)?;
                log::trace!("asset script {}:\n{}", i, &script_info);
                metadata.script_infos.push(script_info);
            }
        }

        log::debug!("reading external files");
        let external_file_count = asset.reader.read_u32_by(big_endian)?;
        log::trace!("{} asset external file(s)", external_file_count);

        for i in 0usize..usize::try_from(external_file_count)? {
            let externa_filel_info = ExternalFileInfo::read(&mut asset.reader, &metadata)?;
            log::trace!("asset external {}:\n{}", i, &externa_filel_info);
            metadata.externa_file_infos.push(externa_filel_info);
        }

        if version >= 20 {
            log::debug!("reading asset ref types");
            let ref_type_count = asset.reader.read_u32_by(big_endian)?;
            log::trace!("{} asset ref type(s)", ref_type_count);
            for i in 0..ref_type_count {
                let ref_type = ClassType::read(&mut asset.reader, &metadata, true)?;
                log::trace!("asset ref type {}:\n{}", i, &ref_type);
                metadata.ref_types.push(ref_type);
            }
        }

        if version >= 5 {
            metadata.user_information = asset.reader.read_string()?;
        }

        Ok(metadata)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        if self.version >= 7 {
            writer.write_all(self.unity_version.to_string().as_bytes())?;
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

        if self.version >= 7 {
            writeln!(
                f,
                "{:indent$}Unity version:            {}",
                "",
                self.unity_version,
                indent = indent
            )?;
        }

        if self.version >= 8 {
            writeln!(
                f,
                "{:indent$}Target platform:          {:?}",
                "",
                self.target_platform,
                indent = indent
            )?;
        }

        if self.version >= 13 {
            writeln!(
                f,
                "{:indent$}Has type tree?            {}",
                "",
                bool_to_yes_no(self.has_type_tree),
                indent = indent
            )?;
        }

        if (7..14).contains(&self.version) {
            writeln!(
                f,
                "{:indent$}Big ID enabled?           {}",
                "",
                bool_to_yes_no(self.big_id_enabled),
                indent = indent
            )?;
        }

        if self.version >= 5 {
            writeln!(
                f,
                "{:indent$}User information: {}",
                "",
                self.user_information,
                indent = indent
            )?;
        }

        writeln!(
            f,
            "{:indent$}Number of class infos:    {}",
            "",
            self.class_infos.len(),
            indent = indent
        )?;

        if self.version >= 11 {
            writeln!(
                f,
                "{:indent$}Number of scripts:        {}",
                "",
                self.script_infos.len(),
                indent = indent
            )?;
        }

        if self.version >= 11 {
            writeln!(
                f,
                "{:indent$}Number of external files: {}",
                "",
                self.script_infos.len(),
                indent = indent
            )?;
        }

        if self.version >= 20 {
            writeln!(
                f,
                "{:indent$}Number of ref types:     {}",
                "",
                self.ref_types.len(),
                indent = indent
            )?;
        }

        writeln!(f, "{:indent$}Class infos:", "", indent = indent)?;
        for (i, class_info) in self.class_infos.iter().enumerate() {
            writeln!(f, "{:indent$}Class info {}:", "", i, indent = indent + 4)?;
            writeln!(f, "{:indent$}", class_info, indent = indent + 8)?;
        }

        if self.version >= 11 {
            writeln!(f, "{:indent$}Script infos:", "", indent = indent)?;
            for (i, script_info) in self.script_infos.iter().enumerate() {
                writeln!(f, "{:indent$}Script info {}:", "", i, indent = indent + 4)?;
                writeln!(f, "{:indent$}", script_info, indent = indent + 8)?;
            }
        }

        writeln!(f, "{:indent$}External file infos:", "", indent = indent)?;
        for (i, external_file_info) in self.externa_file_infos.iter().enumerate() {
            writeln!(
                f,
                "{:indent$}External file info {}:",
                "",
                i,
                indent = indent + 4
            )?;
            writeln!(f, "{:indent$}", external_file_info, indent = indent + 8)?;
        }

        if self.version >= 20 {
            writeln!(f, "{:indent$}Ref types:", "", indent = indent)?;
            for (i, ref_type) in self.ref_types.iter().enumerate() {
                writeln!(f, "{:indent$}Ref type {}:", "", i, indent = indent + 4)?;
                writeln!(f, "{:indent$}", ref_type, indent = indent + 8)?;
            }
        }

        Ok(())
    }
}
