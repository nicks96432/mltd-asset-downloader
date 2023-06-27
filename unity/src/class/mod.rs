mod animation_clip;
mod asset_bundle;
mod class_id_type;
mod editor_extension;
mod game_object;
mod mesh;
mod named_object;
mod object;
mod pptr;
mod sprite;
mod text_asset;
mod texture;
mod texture_2d;

pub use self::animation_clip::*;
pub use self::asset_bundle::*;
pub use self::class_id_type::*;
pub use self::editor_extension::*;
pub use self::game_object::*;
pub use self::mesh::*;
pub use self::named_object::*;
pub use self::object::*;
pub use self::pptr::*;
pub use self::sprite::*;
pub use self::text_asset::*;
pub use self::texture::*;
pub use self::texture_2d::*;

use crate::asset::ClassInfo;
use crate::error::Error;

use std::any::type_name;
use std::fmt::{Debug, Display};
use std::io::{Read, Seek};

#[derive(Debug, Clone, PartialEq)]
pub struct ClassReader {}

impl ClassReader {
    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Box<dyn Class>, Error>
    where
        R: Read + Seek,
    {
        match class_info.object_type()? {
            ClassIDType::AssetBundle => Ok(Box::new(AssetBundle::read(reader, class_info)?)),
            ClassIDType::EditorExtension => {
                Ok(Box::new(EditorExtension::read(reader, class_info)?))
            }
            ClassIDType::GameObject => Ok(Box::new(GameObject::read(reader, class_info)?)),
            ClassIDType::NamedObject => Ok(Box::new(NamedObject::read(reader, class_info)?)),
            ClassIDType::Object => Ok(Box::new(Object::read(reader, class_info)?)),
            ClassIDType::Sprite => Ok(Box::new(Sprite::read(reader, class_info)?)),
            ClassIDType::TextAsset => Ok(Box::new(TextAsset::read(reader, class_info)?)),
            ClassIDType::Texture => Ok(Box::new(Texture::read(reader, class_info)?)),
            ClassIDType::Texture2D => Ok(Box::new(Texture2D::read(reader, class_info)?)),

            c => {
                log::error!("the class type {:?} is not implemented yet", c);
                unimplemented!()
            }
        }
    }
}

pub trait Class: Debug + Display {
    /// Gets the type name of this class.
    fn name(&self) -> &'static str {
        return type_name::<Self>();
    }
}
