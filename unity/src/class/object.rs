use crate::asset::ObjectReader;
use crate::error::Error;

use std::io::Read;

#[derive(Debug)]
pub struct Object {
    object_hide_flags: u32,
}

impl Object {
    pub fn read<R>(file_reader: &mut R, object_reader: &ObjectReader) -> Result<Self, Error>
    where
        R: Read,
    {
        Ok(Self {
            object_hide_flags: 0u32,
        })
    }
}
