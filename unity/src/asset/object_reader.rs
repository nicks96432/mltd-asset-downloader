use super::{Asset, ClassType, SerializedType};
use crate::class::Class;
use crate::error::Error;
use crate::macros::impl_default;
use crate::traits::{ReadIntExt, SeekAlign};
use byteorder::ReadBytesExt;
use num_traits::{FromPrimitive, ToPrimitive};
use std::io::{Read, Seek, Write};

#[derive(Debug, Clone)]
pub struct ObjectReader {
    pub path_id: u64,
    pub start: u64,
    pub size: u32,
    pub r#type: ClassType,
    pub serialized_type: Option<SerializedType>,
    pub is_destroyed: u16,
    pub class_id: u16,
    pub stripped: bool,

    pub(crate) endian: bool,
}

impl ObjectReader {
    pub fn new() -> Self {
        Self {
            path_id: 0u64,
            start: 0u64,
            size: 0u32,
            r#type: ClassType::Unknown,
            serialized_type: None,
            is_destroyed: 0u16,
            class_id: 0u16,
            stripped: false,

            endian: false,
        }
    }

    pub fn read<R>(reader: &mut R, asset: &Asset) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut object = Self::new();
        let endian = asset.header.endian;
        let version = asset.header.version;
        object.endian = endian;

        if asset.big_id_enabled != 0 {
            object.path_id = reader.read_u64_by(endian)?;
        } else if version < 14 {
            object.path_id = reader.read_u32_by(endian)?.into();
        } else {
            reader.seek_align(4)?;
            object.path_id = reader.read_u64_by(endian)?;
        }

        object.start = match version >= 22 {
            true => reader.read_u64_by(endian)?,
            false => reader.read_u32_by(endian)?.into(),
        };
        object.start += asset.header.offset;

        object.size = reader.read_u32_by(endian)?;

        let err = || Error::UnknownClassIDType;
        object.r#type = ClassType::from_i32(reader.read_i32_by(endian)?).ok_or_else(err)?;

        if version < 16 {
            object.class_id = reader.read_u16_by(endian)?;
            object.serialized_type = None;

            for t in asset.types.iter() {
                if ClassType::from_i32(t.class_id).ok_or_else(err)? == object.r#type {
                    object.serialized_type = Some(t.clone());
                    break;
                }
            }
        } else {
            let index = ToPrimitive::to_usize(&object.r#type).ok_or_else(err)?;
            let t = &asset.types[index];
            object.class_id = t.class_id.try_into()?;
            object.serialized_type = Some(t.clone());
        }

        if version < 11 {
            object.is_destroyed = reader.read_u16_by(endian)?;
        }

        if (11..17).contains(&version) {
            let script_type_index = reader.read_i16_by(endian)?;
            if let Some(s) = &mut object.serialized_type {
                s.script_index = script_type_index;
            }
        }

        if version == 15 || version == 16 {
            object.stripped = reader.read_u8()? > 0;
        }

        Ok(object)
    }

    pub fn class<R>(&self, reader: &mut R) -> Result<Class, Error>
    where
        R: Read,
    {
        Class::read(reader, self)
    }

    // TODO: read type tree

    pub fn save<W>(&self, _writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        unimplemented!();
    }
}

impl_default!(ObjectReader);
