use crate::asset::{ClassInfo, Platform};
use crate::error::Error;
use crate::traits::ReadIntExt;

use std::io::Read;

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

    pub fn read<R>(reader: &mut R, object_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut object = Self::new();
        object.version = object_info.version;
        object.big_endian = object_info.big_endian;

        if object_info.target_platform == Platform::NoTarget {
            object.hide_flags = reader.read_u32_by(object_info.big_endian)?;
        }

        Ok(object)
    }
}
