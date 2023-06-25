use super::{Class, PPtr};
use crate::asset::ClassInfo;
use crate::error::Error;
use crate::traits::{ReadAlignedString, ReadPrimitiveExt};

use std::fmt::{Display, Formatter};
use std::io::{Read, Seek};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GameObject {
    pub layer: i32,
    pub name: String,
    pub components: Vec<PPtr>,

    pub(crate) big_endian: bool,
    pub(crate) version: u32,
}

impl GameObject {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut game_object = Self::new();
        game_object.version = class_info.version;
        game_object.big_endian = class_info.big_endian;

        let component_size = reader.read_u32_by(class_info.big_endian)?;
        for _ in 0u32..component_size {
            let component = PPtr::read(reader, class_info)?;
            game_object.components.push(component);
        }

        game_object.layer = reader.read_i32_by(class_info.big_endian)?;
        game_object.name = reader.read_aligned_string(class_info.big_endian, 4)?;

        Ok(game_object)
    }
}

impl Display for GameObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Layer:            {}",
            "",
            self.layer,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Name:             {}",
            "",
            self.name,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Components count: {}",
            "",
            self.components.len(),
            indent = indent
        )?;

        writeln!(f, "{:indent$}Components:", "", indent = indent)?;
        for (i, component) in self.components.iter().enumerate() {
            writeln!(f, "{:indent$}Components {}:", "", i, indent = indent + 4)?;
            write!(f, "{:indent$}:", component, indent = indent + 8)?;
        }

        Ok(())
    }
}

impl Class for GameObject {}
