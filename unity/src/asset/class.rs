use byteorder::ReadBytesExt;

use super::Asset;
use crate::error::Error;
use crate::macros::impl_default;
use crate::traits::ReadIntExt;
use std::any::Any;
use std::io::{Read};

pub struct Class {
    pub id: u32,
    pub stripped: bool,
    pub script_index: i16,
    pub nodes: Vec<Box<dyn Any>>,
}

impl Class {
    pub fn new() -> Self {
        Self {
            id: 0u32,
            stripped: false,
            script_index: 0i16,
            nodes: Vec::new(),
        }
    }

    pub fn read<R: Read>(reader: &mut R, asset: &Asset) -> Result<Self, Error> {
        let mut class = Self::new();

        class.id = reader.read_u32_by(asset.header.endian)?;
        if asset.header.version >= 16 {
            class.stripped = reader.read_u8()? > 0;
        }
        if asset.header.version >= 17 {
            class.script_index = reader.read_i16_by(asset.header.endian)?;
        }

        Ok(class)
    }
}

impl_default!(Class);
