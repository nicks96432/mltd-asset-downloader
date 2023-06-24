mod asset_bundle;
mod class_id_type;
mod editor_extension;
mod game_object;
mod named_object;
mod object;
mod pptr;
mod text_asset;

use num_traits::ToPrimitive;

pub use self::asset_bundle::*;
pub use self::class_id_type::*;
pub use self::editor_extension::*;
pub use self::game_object::*;
pub use self::named_object::*;
pub use self::object::*;
pub use self::pptr::*;
pub use self::text_asset::*;

use crate::asset::ClassInfo;
use crate::error::Error;

use std::backtrace::Backtrace;
use std::fmt::{Debug, Display};
use std::io::{Read, Seek};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassReader {
    Animation,
    AnimationClip,
    Animator,
    AnimatorController,
    AnimatorOverrideController,
    AssetBundle(AssetBundle),
    AudioClip,
    Avatar,
    Behaviour,
    BuildSettings,
    Component,
    EditorExtension(EditorExtension),
    Font,
    GameObject(GameObject),
    Material,
    Mesh,
    MeshFilter,
    MeshRenderer,
    MonoBehaviour,
    MonoScript,
    MovieTexture,
    NamedObject(NamedObject),
    Object(Object),
    PPtr(PPtr),
    PlayerSettings,
    RectTransform,
    Renderer,
    RuntimeAnimatorController,
    ResourceManager,
    Shader,
    SkinnedMeshRenderer,
    Sprite,
    SpriteAtlas,
    TextAsset(TextAsset),
    Texture,
    Texture2D,
    Transform,
    VideoClip,
}

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
            ClassIDType::TextAsset => Ok(Box::new(TextAsset::read(reader, class_info)?)),

            c => Err(Error::UnknownClassIDType {
                class_id: ToPrimitive::to_i32(&c).unwrap_or(-1),
                backtrace: Backtrace::capture(),
            }),
        }
    }
}

pub trait Class: Debug + Display {}
