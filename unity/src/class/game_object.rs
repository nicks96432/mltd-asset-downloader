use super::PPtr;
use crate::asset::ObjectReader;
use crate::error::Error;
use crate::macros::impl_default;
use crate::traits::ReadIntExt;

use std::io::Read;

#[derive(Debug, Clone)]
pub struct GameObject {
    pub name: String,

    components: Vec<PPtr>,
    layer: i32,
    animator: Option<PPtr>,
    animation: Option<PPtr>,
    transform: Option<PPtr>,
    mesh_renderer: Option<PPtr>,
    skinned_mesh_renderer: Option<PPtr>,
    mesh_filter: Option<PPtr>,
}

impl GameObject {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            layer: 0i32,
            name: String::new(),
            animator: None,
            animation: None,
            transform: None,
            mesh_renderer: None,
            skinned_mesh_renderer: None,
            mesh_filter: None,
        }
    }

    pub fn read<R>(reader: &mut R, object: ObjectReader) -> Result<Self, Error>
    where
        R: Read,
    {
        let _component_count = reader.read_i32_by(object.endian)?;

        todo!()
    }
}

impl_default!(GameObject);
