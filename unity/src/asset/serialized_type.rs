use super::{ClassType, Header};
use crate::error::Error;
use crate::traits::{ReadIntExt, ReadString, ReadVecExt};
use crate::utils::type_tree::{CommonString, Name, Node};
use byteorder::ReadBytesExt;
use num_traits::FromPrimitive;
use std::io::{Cursor, Read, Write};

#[derive(Debug, Clone, Default)]
pub struct SerializedType {
    pub class_id: i32,
    pub stripped: bool,
    pub script_index: i16,
    pub script_id: [u8; 16],
    pub hash: [u8; 16],
    pub nodes: Vec<Node>,
    pub string_data: Vec<String>,
    pub type_dependencies: Vec<i32>,
}

impl SerializedType {
    pub fn new() -> Self {
        Self {
            class_id: 0i32,
            stripped: false,
            script_index: 0i16,
            script_id: [0u8; 16],
            hash: [0u8; 16],
            nodes: Vec::new(),
            string_data: Vec::new(),
            type_dependencies: Vec::new(),
        }
    }

    pub fn read<R>(reader: &mut R, header: &Header, is_ref: bool) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut ser_type = Self::new();

        ser_type.class_id = reader.read_i32_by(header.endian)?;
        if header.version >= 16 {
            ser_type.stripped = reader.read_u8()? > 0;
        }
        if header.version >= 17 {
            ser_type.script_index = reader.read_i16_by(header.endian)?;
        }

        if header.version >= 13 {
            if (is_ref && ser_type.script_index >= 0)
                || (header.version < 16 && ser_type.class_id < 0)
                || (header.version >= 16 && ser_type.class_id == ClassType::MonoBehaviour as i32)
            {
                reader.read_exact(&mut ser_type.script_id)?;
            }
            reader.read_exact(&mut ser_type.hash)?;
        }

        if !header.has_type_tree {
            return Ok(ser_type);
        }

        if header.version >= 12 || header.version == 10 {
            ser_type.read_type_tree(reader, header)?;
        }

        if header.version >= 21 {
            ser_type.type_dependencies = reader.read_i32_vec_by(header.endian)?;
        }

        Ok(ser_type)
    }

    fn read_type_tree<R>(&mut self, reader: &mut R, header: &Header) -> Result<(), Error>
    where
        R: Read,
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

    pub fn save<W>(&self, _writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        unimplemented!();
    }
}
