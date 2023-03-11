use crate::error::Error;
use crate::traits::ReadIntExt;
use crate::{asset::Header, macros::impl_default};
use byteorder::ReadBytesExt;
use num_derive::{FromPrimitive, ToPrimitive};
use std::io::Read;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Name {
    Common(CommonString),
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct Node {
    pub version: u16,
    pub level: u8,
    pub class_flags: u8,
    pub class_offset: u32,
    pub name_offset: u32,
    pub size: i32,
    pub index: i32,
    pub meta_flag: i32,
    pub ref_type_hash: u64,

    pub class: Name,
    pub name: Name,
}

impl Node {
    pub fn new() -> Self {
        Self {
            version: 0u16,
            level: 0u8,
            class_flags: 0u8,
            class_offset: 0u32,
            name_offset: 0u32,
            size: 0i32,
            index: 0i32,
            meta_flag: 0i32,

            /// version > 19
            ref_type_hash: 0u64,

            class: Name::Custom(String::new()),
            name: Name::Custom(String::new()),
        }
    }

    pub fn read<R>(reader: &mut R, header: &Header) -> Result<Self, Error>
    where
        R: Read,
    {
        Ok(Self {
            version: reader.read_u16_by(header.endian)?,
            level: reader.read_u8()?,
            class_flags: reader.read_u8()?,
            class_offset: reader.read_u32_by(header.endian)?,
            name_offset: reader.read_u32_by(header.endian)?,
            size: reader.read_i32_by(header.endian)?,
            index: reader.read_i32_by(header.endian)?,
            meta_flag: reader.read_i32_by(header.endian)?,

            ref_type_hash: match header.version >= 19 {
                true => reader.read_u64_by(header.endian)?,
                false => 0,
            },

            class: Name::Custom(String::new()),
            name: Name::Custom(String::new()),
        })
    }
}

impl_default!(Node);

/// from [UnityPy](
///     https://github.com/K0lb3/UnityPy/blob/master/UnityPy/enums/CommonString.py
/// )
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum CommonString {
    AABB = 0,
    AnimationClip = 5,
    AnimationCurve = 19,
    AnimationState = 34,
    Array = 49,
    Base = 55,
    BitField = 60,
    Bitset = 69,
    Bool = 76,
    Char = 81,
    ColorRGBA = 86,
    Component = 96,
    Data = 106,
    Deque = 111,
    Double = 117,
    DynamicArray = 124,
    FastPropertyName = 138,
    First = 155,
    Float = 161,
    Font = 167,
    GameObject = 172,
    GenericMono = 183,
    GradientNEW = 196,
    GUID = 208,
    GUIStyle = 213,
    Int = 222,
    List = 226,
    LongLong = 231,
    Map = 241,
    Matrix4x4f = 245,
    MdFour = 256,
    MonoBehaviour = 263,
    MonoScript = 277,
    MByteSize = 288,
    MCurve = 299,
    MEditorClassIdentifier = 307,
    MEditorHideFlags = 331,
    MEnabled = 349,
    MExtensionPtr = 359,
    MGameObject = 374,
    MIndex = 387,
    MIsArray = 395,
    MIsStatic = 405,
    MMetaFlag = 416,
    MName = 427,
    MObjectHideFlags = 434,
    MPrefabInternal = 452,
    MPrefabParentObject = 469,
    MScript = 490,
    MStaticEditorFlags = 499,
    MType = 519,
    MVersion = 526,
    Object = 536,
    Pair = 543,
    PPtrComponent = 548,
    PPtrGameObject = 564,
    PPtrMaterial = 581,
    PPtrMonoBehaviour = 596,
    PPtrMonoScript = 616,
    PPtrObject = 633,
    PPtrPrefab = 646,
    PPtrSprite = 659,
    PPtrTextAsset = 672,
    PPtrTexture = 688,
    PPtrTexture2D = 702,
    PPtrTransform = 718,
    Prefab = 734,
    Quaternionf = 741,
    Rectf = 753,
    RectInt = 759,
    RectOffset = 767,
    Second = 778,
    Set = 785,
    Short = 789,
    Size = 795,
    SInt16 = 800,
    SInt32 = 807,
    SInt64 = 814,
    SInt8 = 821,
    Staticvector = 827,
    String = 840,
    TextAsset = 847,
    TextMesh = 857,
    Texture = 866,
    Texture2D = 874,
    Transform = 884,
    TypelessData = 894,
    UInt16 = 907,
    UInt32 = 914,
    UInt64 = 921,
    UInt8 = 928,
    UnsignedInt = 934,
    UnsignedLongLong = 947,
    UnsignedShort = 966,
    Vector = 981,
    Vector2f = 988,
    Vector3f = 997,
    Vector4f = 1006,
    MScriptingClassIdentifier = 1015,
    Gradient = 1042,
    Type = 1051,
    Int2Storage = 1057,
    Int3Storage = 1070,
    BoundsInt = 1083,
    MCorrespondingSourceObject = 1093,
    MPrefabInstance = 1121,
    MPrefabAsset = 1138,
    FileSize = 1152,
}
