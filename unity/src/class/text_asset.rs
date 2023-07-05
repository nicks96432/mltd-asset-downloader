use super::{Class, NamedObject};
use crate::asset::ClassInfo;
use crate::error::Error;
use crate::traits::{ReadVecExt, WritePrimitiveExt};

use std::any::Any;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, Write};

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

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write + Seek,
    {
        self.named_object.save(writer)?;
        writer.write_u32_by(
            u32::try_from(self.script.len())?,
            self.named_object.editor_extension.object.big_endian,
        )?;
        writer.write_all(&self.script)?;

        Ok(())
    }
}

impl Display for TextAsset {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Super ({}):",
            "",
            self.named_object.name(),
            indent = indent
        )?;
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

impl Class for TextAsset {
    fn class_id(&self) -> super::ClassIDType {
        super::ClassIDType::TextAsset
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
