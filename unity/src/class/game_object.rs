use super::PPtr;
use crate::asset::ObjectInfo;
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

    pub fn read<R>(reader: &mut R, object: ObjectInfo) -> Result<Self, Error>
    where
        R: Read,
    {
        todo!()
    }
}

impl_default!(GameObject);
