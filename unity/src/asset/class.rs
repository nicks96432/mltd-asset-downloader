use super::Header;
use crate::error::Error;
use crate::macros::impl_default;
use crate::traits::{ReadArrayExt, ReadIntExt, ReadString};
use crate::utils::type_tree::{CommonString, Name, Node};
use byteorder::ReadBytesExt;
use num_traits::FromPrimitive;
use std::io::{Cursor, Read, Seek};

#[derive(Debug, Clone)]
pub struct Class {
    pub id: i32,
    pub stripped: bool,
    pub script_index: i16,
    pub script_id: [u8; 16],
    pub hash: [u8; 16],
    pub nodes: Vec<Node>,
    pub string_data: Vec<String>,
    pub type_dependencies: Vec<i32>,
}

impl Class {
    pub fn new() -> Self {
        Self {
            id: 0i32,
            stripped: false,
            script_index: 0i16,
            script_id: [0u8; 16],
            hash: [0u8; 16],
            nodes: Vec::new(),
            string_data: Vec::new(),
            type_dependencies: Vec::new(),
        }
    }

    pub fn read<R>(reader: &mut R, header: &Header) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut class = Self::new();

        class.id = reader.read_i32_by(header.endian)?;
        if header.version >= 16 {
            class.stripped = reader.read_u8()? > 0;
        }
        if header.version >= 17 {
            class.script_index = reader.read_i16_by(header.endian)?;
        }

        if header.version >= 13 {
            // TODO: remove magic number 114
            if (header.version < 16 && class.id < 0) || (header.version >= 16 && class.id == 114) {
                reader.read_exact(&mut class.script_id)?;
            }
            reader.read_exact(&mut class.hash)?;
        }

        if !header.has_type_tree {
            return Ok(class);
        }

        if header.version >= 12 || header.version == 10 {
            class.read_type_tree(reader, header)?;
        }

        if header.version >= 21 {
            class.type_dependencies = reader.read_i32_vec_by(header.endian)?;
        }

        Ok(class)
    }

    fn read_type_tree<R>(&mut self, reader: &mut R, header: &Header) -> Result<(), Error>
    where
        R: Read + Seek,
    {
        let node_count = reader.read_u32_by(header.endian)?;
        log::trace!("{} asset class node(s)", node_count);

        let string_buf_size = reader.read_u32_by(header.endian)?;

        self.nodes.clear();
        for _ in 0..node_count {
            self.nodes.push(Node::read(reader, header)?);
        }

        let mut buf = vec![0u8; usize::try_from(string_buf_size)?];
        reader.read_exact(&mut buf)?;
        let mut buf = Cursor::new(buf);

        let mut read_name = |offset: u32| -> Result<Name, Error> {
            if offset & 0x8000_0000 == 0 {
                buf.set_position(offset.into());
                Ok(Name::Custom(buf.read_string()?))
            } else {
                match CommonString::from_u32(offset & 0x7fffffff) {
                    Some(s) => Ok(Name::Common(s)),
                    None => Err(Error::UnknownCommonName),
                }
            }
        };

        for (i, node) in self.nodes.iter_mut().enumerate() {
            node.class = read_name(node.class_offset)?;
            node.name = read_name(node.name_offset)?;

            log::trace!("asset class node {}:\n{:#?}", i, node)
        }

        Ok(())
    }
}

impl_default!(Class);
