use super::{ClassType, Metadata, Platform};
use crate::class::ClassIDType;
use crate::error::Error;
use crate::macros::impl_default;
use crate::traits::{ReadIntExt, SeekAlign, WriteIntExt};
use crate::utils::bool_to_yes_no;

use byteorder::{ReadBytesExt, WriteBytesExt};
use num_traits::FromPrimitive;

use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, Write};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassInfo {
    pub id: u64,

    /// This is added to the `data_offset` in the [`Header`][Header] to determine the file
    /// offset to the object data.
    ///
    /// [Header]: super::Header
    pub data_offset: u64,
    pub data_size: u32,

    pub class_id: i32,

    /// Starting in version 16, this is an index to the array of type information given by looping
    /// over `types` in [`Metadata`][Metadata].
    ///
    /// [Metadata]: super::Metadata
    pub type_id: u32,

    pub is_destroyed: bool,

    /// Starting in version 17, this is the field `script_index` read in the loop over `types` in
    /// [`Metadata`][Metadata].
    ///
    /// [Metadata]: super::Metadata
    pub script_index: i16,

    /// Starting in version 17, this is the field `stripped` read in the loop over `types` in
    /// [`Metadata`][Metadata].
    ///
    /// [Metadata]: super::Metadata
    pub stripped: bool,

    // TODO: merge this class into ClassType so that we don't need this
    pub(crate) class_type: Option<Rc<RefCell<ClassType>>>,
    pub(crate) big_endian: bool,
    pub(crate) big_id_enabled: bool,
    pub(crate) target_platform: Platform,
    pub(crate) version: u32,
}

impl ClassInfo {
    pub fn new() -> Self {
        Self {
            id: 0u64,
            data_offset: 0u64,
            data_size: 0u32,
            type_id: 0u32,
            class_id: 0i32,
            is_destroyed: false,
            script_index: -1i16,
            stripped: false,

            class_type: None,
            big_endian: false,
            big_id_enabled: false,
            target_platform: Platform::UnknownPlatform,
            version: 0,
        }
    }

    pub fn object_type(&self) -> Result<ClassIDType, Error> {
        ClassIDType::from_i32(self.class_id).ok_or_else(|| Error::UnknownClassIDType)
    }

    pub fn read<R>(reader: &mut R, metadata: &Metadata) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let big_endian = metadata.big_endian;
        let version = metadata.version;

        let mut object_info = Self::new();
        object_info.big_endian = big_endian;
        object_info.big_id_enabled = metadata.big_id_enabled;
        object_info.target_platform = metadata.target_platform;
        object_info.version = version;

        if metadata.big_id_enabled {
            object_info.id = reader.read_u64_by(big_endian)?;
        } else if version < 14 {
            object_info.id = reader.read_u32_by(big_endian)?.into();
        } else {
            reader.seek_align(4)?;
            object_info.id = reader.read_u64_by(big_endian)?;
        }

        object_info.data_offset = match version >= 22 {
            true => reader.read_u64_by(big_endian)?,
            false => reader.read_u32_by(big_endian)?.into(),
        };
        object_info.data_offset += metadata.data_offset;
        object_info.data_size = reader.read_u32_by(big_endian)?;

        object_info.type_id = reader.read_u32_by(big_endian)?;
        if version <= 15 {
            object_info.class_id = i32::from(reader.read_u16_by(big_endian)?);
        } else {
            let class_type = &metadata.class_types[usize::try_from(object_info.type_id)?];
            object_info.class_type = Some(class_type.clone());
            object_info.class_id = class_type.try_borrow()?.class_id;
        }

        if version <= 10 {
            object_info.is_destroyed = reader.read_u16_by(big_endian)? > 0;
        }

        if (11..=16).contains(&version) {
            object_info.script_index = reader.read_i16_by(big_endian)?;
        }

        if version == 15 || version == 16 {
            object_info.stripped = reader.read_u8()? > 0;
        }

        Ok(object_info)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write + Seek,
    {
        if self.big_id_enabled {
            writer.write_u64_by(self.id, self.big_endian)?;
        } else if self.version < 14 {
            writer.write_u32_by(u32::try_from(self.id)?, self.big_endian)?;
        } else {
            writer.seek_align(4)?;
            writer.write_u64_by(self.id, self.big_endian)?;
        }

        match self.version >= 22 {
            true => writer.write_u64_by(self.data_offset, self.big_endian)?,
            false => writer.write_u32_by(u32::try_from(self.data_offset)?, self.big_endian)?,
        };

        writer.write_u32_by(self.data_size, self.big_endian)?;
        writer.write_u32_by(self.type_id, self.big_endian)?;
        if self.version < 16 {
            writer.write_u16_by(u16::try_from(self.class_id)?, self.big_endian)?;
        }

        if self.version <= 10 {
            writer.write_u16_by(u16::from(self.is_destroyed), self.big_endian)?;
        }

        if (11..=16).contains(&self.version) {
            writer.write_i16_by(self.script_index, self.big_endian)?;
        }

        if self.version == 15 || self.version == 16 {
            writer.write_u8(u8::from(self.stripped))?;
        }

        Ok(())
    }
}

impl Display for ClassInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}ID:             {}",
            "",
            self.id,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Data offset:    {}",
            "",
            self.data_offset,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Data size:      {}",
            "",
            self.data_size,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Asset class id: {}",
            "",
            self.type_id,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Object type:    {:?}",
            "",
            self.object_type().unwrap_or(ClassIDType::Unknown),
            indent = indent
        )?;

        if self.version <= 10 {
            writeln!(
                f,
                "{:indent$}Is destroyed?   {}",
                "",
                bool_to_yes_no(self.is_destroyed),
                indent = indent
            )?;
        }

        if (11..=16).contains(&self.version) {
            writeln!(
                f,
                "{:indent$}Script index:   {}",
                "",
                self.script_index,
                indent = indent
            )?;
        }

        if self.version == 15 || self.version == 16 {
            writeln!(
                f,
                "{:indent$}Stripped?      {}",
                "",
                bool_to_yes_no(self.stripped),
                indent = indent
            )?;
        }

        Ok(())
    }
}

impl_default!(ClassInfo);
