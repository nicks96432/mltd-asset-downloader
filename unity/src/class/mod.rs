mod class_id_type;
mod editor_extension;
mod game_object;
mod object;
mod pptr;

pub use self::class_id_type::*;
pub use self::editor_extension::*;
pub use self::game_object::*;
pub use self::object::*;
pub use self::pptr::*;

use crate::asset::ClassInfo;
use crate::error::Error;
use std::io::Read;

#[derive(Debug)]
pub enum Class {
    Animation,
    AnimationClip,
    Animator,
    AnimatorController,
    AnimatorOverrideController,
    AssetBundle,
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
    NamedObject,
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
    TextAsset,
    Texture,
    Texture2D,
    Transform,
    VideoClip,
}

impl Class {
    pub fn read<R>(reader: &mut R, object_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read,
    {
        match object_info.object_type()? {
            ClassIDType::Animation => Ok(Self::Animation),
            ClassIDType::AnimationClip => Ok(Self::AnimationClip),
            ClassIDType::Animator => Ok(Self::Animator),
            ClassIDType::AnimatorController => Ok(Self::AnimatorController),
            ClassIDType::AnimatorOverrideController => Ok(Self::AnimatorOverrideController),
            ClassIDType::AssetBundle => Ok(Self::AssetBundle),
            ClassIDType::AudioClip => Ok(Self::AudioClip),
            ClassIDType::Avatar => Ok(Self::Avatar),
            ClassIDType::Behaviour => Ok(Self::Behaviour),
            ClassIDType::BuildSettings => Ok(Self::BuildSettings),
            ClassIDType::Component => Ok(Self::Component),
            ClassIDType::EditorExtension => Ok(Self::EditorExtension(EditorExtension::read(
                reader,
                object_info,
            )?)),
            ClassIDType::Font => Ok(Self::Font),
            ClassIDType::GameObject => Ok(Self::GameObject(GameObject::read(reader, object_info)?)),
            ClassIDType::Material => Ok(Self::Material),
            ClassIDType::Mesh => Ok(Self::Mesh),
            ClassIDType::MeshFilter => Ok(Self::MeshFilter),
            ClassIDType::MeshRenderer => Ok(Self::MeshRenderer),
            ClassIDType::MonoBehaviour => Ok(Self::MonoBehaviour),
            ClassIDType::MonoScript => Ok(Self::MonoScript),
            ClassIDType::MovieTexture => Ok(Self::MovieTexture),
            ClassIDType::NamedObject => Ok(Self::NamedObject),
            ClassIDType::Object => Ok(Self::Object(Object::read(reader, object_info)?)),
            ClassIDType::PlayerSettings => Ok(Self::PlayerSettings),
            ClassIDType::RectTransform => Ok(Self::RectTransform),
            ClassIDType::Renderer => Ok(Self::Renderer),
            ClassIDType::RuntimeAnimatorController => Ok(Self::RuntimeAnimatorController),
            ClassIDType::ResourceManager => Ok(Self::ResourceManager),
            ClassIDType::Shader => Ok(Self::Shader),
            ClassIDType::SkinnedMeshRenderer => Ok(Self::SkinnedMeshRenderer),
            ClassIDType::Sprite => Ok(Self::Sprite),
            ClassIDType::SpriteAtlas => Ok(Self::SpriteAtlas),
            ClassIDType::TextAsset => Ok(Self::TextAsset),
            ClassIDType::Texture => Ok(Self::Texture),
            ClassIDType::Texture2D => Ok(Self::Texture2D),
            ClassIDType::Transform => Ok(Self::Transform),
            ClassIDType::VideoClip => Ok(Self::VideoClip),

            _ => Err(Error::UnknownClassIDType),
        }
    }
}
