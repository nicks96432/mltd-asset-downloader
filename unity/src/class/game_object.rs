use super::PPtr;
use crate::asset::ClassInfo;
use crate::error::Error;
use crate::traits::ReadIntExt;

use std::cell::RefCell;
use std::io::Read;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GameObject {
    pub name: String,

    components: Vec<Rc<RefCell<PPtr>>>,
    layer: i32,
    animator: Option<Rc<RefCell<PPtr>>>,
    animation: Option<Rc<RefCell<PPtr>>>,
    transform: Option<Rc<RefCell<PPtr>>>,
    mesh_renderer: Option<Rc<RefCell<PPtr>>>,
    skinned_mesh_renderer: Option<Rc<RefCell<PPtr>>>,
    mesh_filter: Option<Rc<RefCell<PPtr>>>,

    pub(crate) big_endian: bool,
    pub(crate) version: u32,
}

impl GameObject {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, object_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut game_object = Self::new();
        game_object.version = object_info.version;
        game_object.big_endian = object_info.big_endian;

        let component_size = reader.read_i32_by(object_info.big_endian)?;
        for _ in 0..component_size {
            let component = Rc::new(RefCell::new(PPtr::read(reader, object_info)?));
            game_object.components.push(component.clone());
        }

        game_object.layer = reader.read_i32_by(object_info.big_endian)?;

        let length = reader.read_u32_by(object_info.big_endian)?;
        let mut buf = vec![0u8; usize::try_from(length)?];
        reader.read_exact(&mut buf)?;
        game_object.name = String::from_utf8(buf)?;

        Ok(game_object)
    }
}
