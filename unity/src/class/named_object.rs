use super::{Class, EditorExtension};
use crate::asset::ClassInfo;
use crate::error::Error;
use crate::traits::{ReadAlignedString, WriteAlign, WritePrimitiveExt};

use byteorder::WriteBytesExt;

use std::any::type_name;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, SeekFrom, Write};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NamedObject {
    pub editor_extension: EditorExtension,
    pub name: String,

    pub(crate) big_endian: bool,
    pub(crate) data_offset: u64,
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
        named_object.big_endian = class_info.big_endian;
        named_object.data_offset = class_info.data_offset;

        named_object.editor_extension = EditorExtension::read(reader, class_info)?;

        reader.seek(SeekFrom::Start(class_info.data_offset))?;
        named_object.name = reader.read_aligned_string(class_info.big_endian, 4)?;
        log::trace!("name: {}", named_object.name);

        Ok(named_object)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write + Seek,
    {
        self.editor_extension.save(writer)?;

        // XXX: maybe there are some data in this gap?
        let gap = self.data_offset - writer.stream_position()?;
        for _ in 0u64..gap {
            writer.write_u8(0u8)?;
        }

        writer.write_u32_by(u32::try_from(self.name.len())?, self.big_endian)?;
        writer.write_all(self.name.as_bytes())?;
        writer.write_align(4)?;

        Ok(())
    }
}

impl Display for NamedObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Super ({}):",
            "",
            type_name::<EditorExtension>(),
            indent = indent
        )?;
        write!(f, "{:indent$}", self.editor_extension, indent = indent + 4)?;
        writeln!(f, "{:indent$}Name: {}", "", self.name, indent = indent)?;

        Ok(())
    }
}

impl Class for NamedObject {}
