use super::{Asset, TypeTree};
use crate::class::ClassType;
use crate::error::Error;
use crate::macros::impl_default;
use crate::traits::{ReadIntExt, ReadString, ReadVecExt};

use byteorder::ReadBytesExt;
use num_traits::{FromPrimitive, ToPrimitive};

use std::fmt::{Display, Formatter};
use std::io::{Read, Write};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetType {
    /// Negative for script types
    pub class_id: i32,
    pub stripped: bool,
    pub script_index: i16,
    pub script_hash: [u8; 16],
    pub type_hash: [u8; 16],
    pub type_tree: TypeTree,
    pub type_dependencies: Vec<i32>,
    pub class_name: String,
    pub namespace: String,
    pub assembly_name: String,
}

impl AssetType {
    pub fn new() -> Self {
        Self {
            class_id: 0i32,
            stripped: false,
            script_index: -1i16,
            script_hash: [0u8; 16],
            type_hash: [0u8; 16],
            type_tree: TypeTree::new(),
            type_dependencies: Vec::new(),
            class_name: String::new(),
            namespace: String::new(),
            assembly_name: String::new(),
        }
    }

    pub fn read(asset: &mut Asset, is_ref: bool) -> Result<Self, Error> {
        let version = asset.header.version;
        let big_endian = asset.header.big_endian;

        let mut asset_type = Self::new();

        asset_type.class_id = asset.reader.read_i32_by(big_endian)?;

        if version >= 16 {
            asset_type.stripped = asset.reader.read_u8()? > 0;
        }

        if version >= 17 {
            asset_type.script_index = asset.reader.read_i16_by(big_endian)?;
        }

        if version >= 13 {
            if (is_ref && asset_type.script_index >= 0)
                || (version < 16 && asset_type.class_id < 0)
                || (version >= 16
                    // MonoBehavior is a script type
                    && asset_type.class_id
                            == ToPrimitive::to_i32(&ClassType::MonoBehaviour).unwrap_or(0))
            {
                asset.reader.read_exact(&mut asset_type.script_hash)?;
            }
            asset.reader.read_exact(&mut asset_type.type_hash)?;
        }

        if !asset.metadata.has_type_tree {
            return Ok(asset_type);
        }

        if version >= 12 || version == 10 {
            asset_type.type_tree = TypeTree::read(asset)?;
        } else {
            // TODO: implement old type tree parsing
            unimplemented!();
        }

        if version >= 21 {
            if is_ref {
                asset_type.class_name = asset.reader.read_string()?;
                asset_type.namespace = asset.reader.read_string()?;
                asset_type.assembly_name = asset.reader.read_string()?;
            } else {
                asset_type.type_dependencies = asset.reader.read_i32_vec_by(big_endian)?;
            }
        }

        Ok(asset_type)
    }

    pub fn save<W>(&self, _writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        unimplemented!();
    }
}

impl Display for AssetType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Class ID:     {} ({:?})",
            "",
            self.class_id,
            ClassType::from_i32(self.class_id).unwrap_or(ClassType::Unknown),
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Stripped?     {}",
            "",
            self.stripped,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Script index: {}",
            "",
            self.script_index,
            indent = indent
        )?;

        if self.script_index != -1 {
            writeln!(
                f,
                "{:indent$}Script ID:    {}",
                "",
                hex::encode(self.script_hash),
                indent = indent
            )?;
        }
        writeln!(
            f,
            "{:indent$}Hash:         {}",
            "",
            hex::encode(self.type_hash),
            indent = indent
        )?;

        writeln!(
            f,
            "{:indent$}Type tree:    {} node(s)",
            "",
            self.type_tree.nodes.len(),
            indent = indent
        )?;

        if !f.alternate() {
            return Ok(());
        }

        for (i, node) in self.type_tree.nodes.iter().enumerate() {
            writeln!(f, "{:indent$}Node {}:", "", i, indent = indent + 4)?;
            writeln!(
                f,
                "{:indent$}Name: {:?}",
                "",
                node.name,
                indent = indent + 8
            )?;
        }

        Ok(())
    }
}

impl_default!(AssetType);
