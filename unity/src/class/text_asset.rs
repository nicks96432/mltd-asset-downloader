use super::{Class, NamedObject};
use crate::asset::ClassInfo;
use crate::error::Error;
use crate::traits::ReadVecExt;

use std::fmt::{Display, Formatter};
use std::io::{Read, Seek};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextAsset {
    pub named_object: NamedObject,
    pub script: Vec<u8>,
}

impl TextAsset {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut text_asset = TextAsset::new();
        text_asset.named_object = NamedObject::read(reader, class_info)?;
        text_asset.script = reader.read_u8_vec_by(class_info.big_endian)?;

        Ok(text_asset)
    }
}

impl Display for TextAsset {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(f, "{:indent$}Super:", "", indent = indent)?;
        write!(f, "{:indent$}", self.named_object, indent = indent + 4)?;
        writeln!(
            f,
            "{:indent$}Script size: {}",
            "",
            self.script.len(),
            indent = indent
        )?;

        Ok(())
    }
}

impl Class for TextAsset {}
