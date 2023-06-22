use super::{Object, PPtr};
use crate::asset::ClassInfo;
use crate::error::Error;

use std::io::Read;

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

    pub fn read<R>(reader: &mut R, object_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut editor_extension = Self::new();

        editor_extension.object = Object::read(reader, object_info)?;
        editor_extension.prefab_parent_object = PPtr::read(reader, object_info)?;
        editor_extension.prefab_internal = PPtr::read(reader, object_info)?;

        Ok(editor_extension)
    }
}
