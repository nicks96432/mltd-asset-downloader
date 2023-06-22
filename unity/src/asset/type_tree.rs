use super::Metadata;
use crate::error::Error;
use crate::macros::impl_default;
use crate::traits::{ReadIntExt, ReadString};

use byteorder::ReadBytesExt;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;

use std::fmt::{Display, Formatter};
use std::io::{Cursor, Read};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Name {
    Common(CommonString),
    Custom(String),
}

impl ToString for Name {
    fn to_string(&self) -> String {
        match self {
            Name::Common(c) => format!("{:?}", c),
            Name::Custom(s) => s.to_owned(),
        }
    }
}

/// Global string buffer, from [UnityPy](
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub version: u16,

    /// The depth in the tree of the current node.
    ///
    /// Nodes of the tree are serialized depth-first, so this number will increase when the current
    /// node is a child of the previousnode, will stay the same when the current node is a sibling
    /// of the previous node, and will decrease when the current node is a sibling of one of the
    /// previous node’s parents.
    pub depth: u8,

    /// When true, this node is a special array node -- its first child (size) in the tree is
    /// the size in elements of the array, and its next child (data) is serialized in a loop
    /// for each element of the array.
    pub is_array: bool,
    pub class_offset: u32,
    pub name_offset: u32,

    /// The expected size in bytes when this node (including children) is serialized.
    ///
    /// This is -1 for variable-sized fields, such as arrays or structs that have arrays as
    /// children.
    pub size: i32,

    /// This is just an index of the node in the flat depth-first list of nodes.
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
            depth: 0u8,
            is_array: false,
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

    pub fn align(&self) -> bool {
        self.meta_flag & 0x4000i32 != 0
    }

    pub fn read<R>(reader: &mut R, metadata: &Metadata) -> Result<Self, Error>
    where
        R: Read,
    {
        Ok(Self {
            version: reader.read_u16_by(metadata.big_endian)?,
            depth: reader.read_u8()?,
            is_array: reader.read_u8()? > 0,
            class_offset: reader.read_u32_by(metadata.big_endian)?,
            name_offset: reader.read_u32_by(metadata.big_endian)?,
            size: reader.read_i32_by(metadata.big_endian)?,
            index: reader.read_i32_by(metadata.big_endian)?,
            meta_flag: reader.read_i32_by(metadata.big_endian)?,

            ref_type_hash: match metadata.version >= 19 {
                true => reader.read_u64_by(metadata.big_endian)?,
                false => 0,
            },

            class: Name::Custom(String::new()),
            name: Name::Custom(String::new()),
        })
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Class:        {}",
            "",
            self.class.to_string(),
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Name:         {}",
            "",
            self.name.to_string(),
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Class offset: {}",
            "",
            self.index,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Name offset:  {}",
            "",
            self.index,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Index:        {}",
            "",
            self.index,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Depth:        {}",
            "",
            self.depth,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Size:         {}",
            "",
            self.size,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Is array?     {}",
            "",
            self.is_array,
            indent = indent
        )?;

        Ok(())
    }
}

impl_default!(Node);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TypeTree {
    pub string_buffer_size: i32,
    pub nodes: Vec<Node>,
}

impl TypeTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, metadata: &Metadata) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut type_tree = Self::new();

        let node_count = reader.read_i32_by(metadata.big_endian)?;
        log::trace!("{} asset class node(s)", node_count);

        type_tree.string_buffer_size = reader.read_i32_by(metadata.big_endian)?;

        for _ in 0..node_count {
            type_tree.nodes.push(Node::read(reader, metadata)?);
        }

        let mut buf = vec![0u8; usize::try_from(type_tree.string_buffer_size)?];
        reader.read_exact(&mut buf)?;
        let mut buf = Cursor::new(buf);

        let mut read_name = |offset: u32| -> Result<Name, Error> {
            match offset & 0x8000_0000u32 {
                0u32 => {
                    buf.set_position(offset.into());
                    Ok(Name::Custom(buf.read_string()?))
                }
                _ => match CommonString::from_u32(offset & 0x7fff_ffffu32) {
                    Some(s) => Ok(Name::Common(s)),
                    None => Err(Error::UnknownCommonName),
                },
            }
        };

        for (i, node) in type_tree.nodes.iter_mut().enumerate() {
            node.class = read_name(node.class_offset)?;
            node.name = read_name(node.name_offset)?;

            log::trace!("asset class node {}:\n{:#?}", i, node)
        }

        Ok(type_tree)
    }
}

impl Display for TypeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        for (i, node) in self.nodes.iter().enumerate() {
            writeln!(f, "{:indent$}Node {}:", "", i, indent = indent + 4)?;
            write!(f, "{:indent$}", node, indent = indent + 8)?;
        }

        Ok(())
    }
}
