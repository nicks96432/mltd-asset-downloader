mod class_type;
mod editor_extension;
mod game_object;
mod object;
mod pptr;

pub use self::class_type::*;
pub use self::editor_extension::*;
pub use self::game_object::*;
pub use self::object::*;
pub use self::pptr::*;

use crate::asset::ObjectInfo;
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
    EditorExtension,
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
    Object,
    PPtr,
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
    pub fn read<R>(reader: &mut R, obj: ObjectInfo) -> Result<Self, Error>
    where
        R: Read,
    {
        match obj.class_type {
            ClassType::Animation => Ok(Self::Animation),
            ClassType::AnimationClip => Ok(Self::AnimationClip),
            ClassType::Animator => Ok(Self::Animator),
            ClassType::AnimatorController => Ok(Self::AnimatorController),
            ClassType::AnimatorOverrideController => Ok(Self::AnimatorOverrideController),
            ClassType::AssetBundle => Ok(Self::AssetBundle),
            ClassType::AudioClip => Ok(Self::AudioClip),
            ClassType::Avatar => Ok(Self::Avatar),
            ClassType::Behaviour => Ok(Self::Behaviour),
            ClassType::BuildSettings => Ok(Self::BuildSettings),
            ClassType::Component => Ok(Self::Component),
            ClassType::EditorExtension => Ok(Self::EditorExtension),
            ClassType::Font => Ok(Self::Font),
            ClassType::GameObject => Ok(Self::GameObject(GameObject::read(reader, obj)?)),
            ClassType::Material => Ok(Self::Material),
            ClassType::Mesh => Ok(Self::Mesh),
            ClassType::MeshFilter => Ok(Self::MeshFilter),
            ClassType::MeshRenderer => Ok(Self::MeshRenderer),
            ClassType::MonoBehaviour => Ok(Self::MonoBehaviour),
            ClassType::MonoScript => Ok(Self::MonoScript),
            ClassType::MovieTexture => Ok(Self::MovieTexture),
            ClassType::NamedObject => Ok(Self::NamedObject),
            ClassType::Object => Ok(Self::Object),
            ClassType::PlayerSettings => Ok(Self::PlayerSettings),
            ClassType::RectTransform => Ok(Self::RectTransform),
            ClassType::Renderer => Ok(Self::Renderer),
            ClassType::RuntimeAnimatorController => Ok(Self::RuntimeAnimatorController),
            ClassType::ResourceManager => Ok(Self::ResourceManager),
            ClassType::Shader => Ok(Self::Shader),
            ClassType::SkinnedMeshRenderer => Ok(Self::SkinnedMeshRenderer),
            ClassType::Sprite => Ok(Self::Sprite),
            ClassType::SpriteAtlas => Ok(Self::SpriteAtlas),
            ClassType::TextAsset => Ok(Self::TextAsset),
            ClassType::Texture => Ok(Self::Texture),
            ClassType::Texture2D => Ok(Self::Texture2D),
            ClassType::Transform => Ok(Self::Transform),
            ClassType::VideoClip => Ok(Self::VideoClip),

            _ => Err(Error::UnknownClassIDType),
        }
    }
}
