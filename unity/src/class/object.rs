use crate::asset::{ClassInfo, Platform};
use crate::error::Error;
use crate::traits::ReadIntExt;

use std::fmt::{Display, Formatter};
use std::io::{Read, Seek};

use super::Class;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Object {
    pub hide_flags: u32,

    pub(crate) version: u32,
    pub(crate) big_endian: bool,
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
        object.version = class_info.version;
        object.big_endian = class_info.big_endian;

        if class_info.target_platform == Platform::NoTarget {
            object.hide_flags = reader.read_u32_by(class_info.big_endian)?;
        }

        reader.seek(std::io::SeekFrom::Start(class_info.data_offset))?;

        Ok(object)
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

impl Class for Object {}
