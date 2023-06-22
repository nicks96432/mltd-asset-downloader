use crate::asset::ClassInfo;
use crate::error::Error;
use crate::traits::ReadIntExt;
use crate::traits::WriteIntExt;

use std::io::{Read, Write};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PPtr {
    pub file_id: i32,
    pub path_id: i64,

    pub(crate) version: u32,
    pub(crate) big_endian: bool,
}

impl PPtr {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, object_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut pptr = Self::new();
        pptr.version = object_info.version;
        pptr.big_endian = object_info.big_endian;

        pptr.file_id = reader.read_i32_by(object_info.big_endian)?;
        match object_info.version >= 14 {
            true => pptr.path_id = reader.read_i64_by(object_info.big_endian)?,
            false => pptr.path_id = i64::from(reader.read_i32_by(object_info.big_endian)?),
        };

        Ok(pptr)
    }

    pub fn write<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        writer.write_i32_by(self.file_id, self.big_endian)?;
        match self.version >= 14 {
            true => writer.write_i64_by(self.path_id, self.big_endian)?,
            false => writer.write_i32_by(i32::try_from(self.path_id)?, self.big_endian)?,
        }

        Ok(())
    }

    // TODO: maybe implement something like Environment in UnityPy so that we can read externals from PPtr in a convenient way?
}
