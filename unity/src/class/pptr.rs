use crate::asset::ClassInfo;
use crate::error::Error;
use crate::traits::ReadPrimitiveExt;
use crate::traits::WritePrimitiveExt;

use std::fmt::Display;
use std::fmt::Formatter;
use std::io::{Read, Write};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PPtr {
    pub file_id: u32,
    pub path_id: u64,

    pub(crate) version: u32,
    pub(crate) big_endian: bool,
}

impl PPtr {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut pptr = Self::new();
        pptr.version = class_info.version;
        pptr.big_endian = class_info.big_endian;

        pptr.file_id = reader.read_u32_by(class_info.big_endian)?;
        match class_info.version >= 14 {
            true => pptr.path_id = reader.read_u64_by(class_info.big_endian)?,
            false => pptr.path_id = u64::from(reader.read_u32_by(class_info.big_endian)?),
        };

        Ok(pptr)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        writer.write_u32_by(self.file_id, self.big_endian)?;
        match self.version >= 14 {
            true => writer.write_u64_by(self.path_id, self.big_endian)?,
            false => writer.write_u32_by(u32::try_from(self.path_id)?, self.big_endian)?,
        }

        Ok(())
    }

    // TODO: maybe implement something like Environment in UnityPy so that we can read externals from PPtr in a convenient way?
}

impl Display for PPtr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}file ID: {}",
            "",
            self.file_id,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}path ID: {}",
            "",
            self.path_id,
            indent = indent
        )?;

        Ok(())
    }
}
