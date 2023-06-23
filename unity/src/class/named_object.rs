use super::{Class, EditorExtension};
use crate::asset::ClassInfo;
use crate::error::Error;
use crate::traits::ReadAlignedString;

use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, SeekFrom};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NamedObject {
    pub editor_extension: EditorExtension,
    pub name: String,
}

impl NamedObject {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut named_object = Self::new();
        named_object.editor_extension = EditorExtension::read(reader, class_info)?;

        reader.seek(SeekFrom::Start(class_info.data_offset))?;
        named_object.name = reader.read_aligned_string(class_info.big_endian)?;

        Ok(named_object)
    }
}

impl Display for NamedObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(f, "{:indent$}Super:", "", indent = indent)?;
        write!(f, "{:indent$}", self.editor_extension, indent = indent + 4)?;
        writeln!(f, "{:indent$}Name: {}", "", self.name, indent = indent)?;

        Ok(())
    }
}

impl Class for NamedObject {}
