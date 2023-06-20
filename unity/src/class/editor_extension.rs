use super::{Object, PPtr};
use crate::asset::{ObjectReader, Platform};
use crate::error::Error;

use std::io::Read;

#[derive(Debug)]
pub struct EditorExtension {
    object: Object,
    pub(crate) prefab_parent_object: PPtr,
    pub(crate) prefab_internal: PPtr,
}

impl EditorExtension {
    pub fn read<R>(file_reader: &mut R, object_reader: &ObjectReader) -> Result<Self, Error>
    where
        R: Read,
    {
        Ok(Self {
            object: Object::read(file_reader, object_reader)?,
            prefab_parent_object: PPtr::read(file_reader, object_reader)?,
            prefab_internal: PPtr::read(file_reader, object_reader)?,
        })
    }
}
