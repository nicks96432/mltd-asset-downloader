use super::{Class, Object, PPtr};
use crate::asset::{ClassInfo, Platform};
use crate::error::Error;

use std::any::type_name;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditorExtension {
    pub object: Object,
    pub prefab_parent_object: PPtr,
    pub prefab_internal: PPtr,
}

impl EditorExtension {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut editor_extension = Self::new();

        editor_extension.object = Object::read(reader, class_info)?;
        if class_info.target_platform == Platform::NoTarget {
            editor_extension.prefab_parent_object = PPtr::read(reader, class_info)?;
            editor_extension.prefab_internal = PPtr::read(reader, class_info)?;
        }

        Ok(editor_extension)
    }
}

impl Display for EditorExtension {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Super ({}):",
            "",
            type_name::<Object>(),
            indent = indent
        )?;
        write!(f, "{:indent$}", self.object, indent = indent + 4)?;

        writeln!(f, "{:indent$}Prefab parent object:", "", indent = indent)?;
        write!(
            f,
            "{:indent$}",
            self.prefab_parent_object,
            indent = indent + 4
        )?;

        writeln!(f, "{:indent$}Prefab internal:", "", indent = indent)?;
        write!(f, "{:indent$}", self.prefab_internal, indent = indent + 4)?;

        Ok(())
    }
}

impl Class for EditorExtension {}
