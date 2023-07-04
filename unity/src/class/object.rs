use super::Class;
use crate::asset::{ClassInfo, Platform};
use crate::error::Error;
use crate::traits::{ReadPrimitiveExt, WritePrimitiveExt};
use crate::utils::Version;

use byteorder::WriteBytesExt;

use std::any::Any;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, Write};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Object {
    pub hide_flags: u32,

    pub big_endian: bool,
    pub data_offset: i64,
    pub target_platform: Platform,
    pub unity_version: Version,
}

impl Object {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut object = Self::new();
        object.target_platform = class_info.target_platform;
        object.big_endian = class_info.big_endian;
        object.data_offset = class_info.data_offset;
        object.unity_version = class_info.unity_version.clone();

        if class_info.target_platform == Platform::NoTarget {
            object.hide_flags = reader.read_u32_by(class_info.big_endian)?;
        }

        reader.seek(std::io::SeekFrom::Start(u64::try_from(
            class_info.data_offset,
        )?))?;

        Ok(object)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        if self.target_platform == Platform::NoTarget {
            writer.write_u32_by(self.hide_flags, self.big_endian)?;
        }

        for _ in 0i64..self.data_offset {
            writer.write_u8(0u8)?;
        }

        Ok(())
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Hide flags: {}",
            "",
            self.hide_flags,
            indent = indent
        )?;

        Ok(())
    }
}

impl Class for Object {
    fn class_id(&self) -> super::ClassIDType {
        super::ClassIDType::Object
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
