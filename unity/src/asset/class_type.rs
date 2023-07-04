use super::{Metadata, TypeTree};
use crate::class::ClassIDType;
use crate::error::Error;
use crate::traits::{ReadPrimitiveExt, ReadString, ReadVecExt};

use byteorder::ReadBytesExt;
use num_traits::{FromPrimitive, ToPrimitive};

use std::backtrace::Backtrace;
use std::fmt::{Display, Formatter};
use std::io::{Read, Write};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClassType {
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

    pub(crate) big_endian: bool,
    pub(crate) version: i32,
}

impl ClassType {
    pub fn new() -> Self {
        Self {
            script_index: -1i16,
            ..Default::default()
        }
    }

    pub fn read<R>(reader: &mut R, metadata: &Metadata, is_ref: bool) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut class_type = Self::new();
        class_type.big_endian = metadata.big_endian;
        class_type.version = metadata.version;

        class_type.class_id = reader.read_i32_by(class_type.big_endian)?;

        if metadata.version >= 16 {
            class_type.stripped = reader.read_u8()? > 0;
        }

        if metadata.version >= 17 {
            class_type.script_index = reader.read_i16_by(class_type.big_endian)?;
        }

        if metadata.version >= 13 {
            if (is_ref && class_type.script_index >= 0)
                || (metadata.version < 16 && class_type.class_id < 0)
                || (metadata.version >= 16
                    // MonoBehavior is a script type
                    && class_type.class_id
                            == ToPrimitive::to_i32(&ClassIDType::MonoBehaviour).ok_or_else(||Error::UnknownClassIDType { class_id:0, backtrace: Backtrace::capture() })?)
            {
                reader.read_exact(&mut class_type.script_hash)?;
            }
            reader.read_exact(&mut class_type.type_hash)?;
        }

        if !metadata.has_type_tree {
            return Ok(class_type);
        }

        log::trace!("reading type tree");
        if metadata.version >= 12 || metadata.version == 10 {
            class_type.type_tree = TypeTree::read(reader, metadata)?;
        } else {
            // TODO: implement old type tree parsing
            unimplemented!();
        }

        if metadata.version >= 21 {
            if is_ref {
                class_type.class_name = reader.read_string()?;
                class_type.namespace = reader.read_string()?;
                class_type.assembly_name = reader.read_string()?;
            } else {
                class_type.type_dependencies = reader.read_i32_vec_by(class_type.big_endian)?;
            }
        }

        Ok(class_type)
    }

    pub fn save<W>(&self, _writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        unimplemented!();
    }
}

impl Display for ClassType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Class ID:       {} ({:?})",
            "",
            self.class_id,
            ClassIDType::from_i32(self.class_id).unwrap_or(ClassIDType::Unknown),
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Stripped?       {}",
            "",
            self.stripped,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Script index:   {}",
            "",
            self.script_index,
            indent = indent
        )?;

        if self.script_index != -1 {
            writeln!(
                f,
                "{:indent$}Script ID:      {}",
                "",
                hex::encode(self.script_hash),
                indent = indent
            )?;
        }
        writeln!(
            f,
            "{:indent$}Hash:           {}",
            "",
            hex::encode(self.type_hash),
            indent = indent
        )?;

        writeln!(
            f,
            "{:indent$}Type tree:      {} node(s)",
            "",
            self.type_tree.nodes.len(),
            indent = indent
        )?;

        if !f.alternate() {
            return Ok(());
        }

        write!(f, "{:indent$}", self.type_tree, indent = indent)?;

        Ok(())
    }
}
