use super::{Asset, AssetType};
use crate::class::Class;
use crate::class::ClassType;
use crate::error::Error;
use crate::macros::impl_default;
use crate::traits::{ReadIntExt, SeekAlign};

use byteorder::ReadBytesExt;
use num_traits::{FromPrimitive, ToPrimitive};

use std::cell::RefCell;
use std::io::{Read, Write};
use std::rc::Rc;
use std::rc::Weak;

#[derive(Debug, Clone)]
pub struct ObjectReader {
    pub path_id: u64,
    pub start: u64,
    pub size: u32,
    pub r#type: ClassType,
    pub asset: Weak<RefCell<Asset>>,
    pub asset_type: Option<AssetType>,
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
            asset: Weak::new(),
            asset_type: None,
            is_destroyed: 0u16,
            class_id: 0u16,
            stripped: false,

            endian: false,
        }
    }

    pub fn read(asset: Rc<RefCell<Asset>>) -> Result<Self, Error> {
        let mut object = Self::new();
        object.asset = Rc::downgrade(&asset);

        let mut asset = asset.try_borrow_mut()?;
        let endian = asset.header.endian;
        let version = asset.header.version;
        object.endian = endian;

        if asset.big_id_enabled {
            object.path_id = asset.reader.read_u64_by(endian)?;
        } else if version < 14 {
            object.path_id = asset.reader.read_u32_by(endian)?.into();
        } else {
            asset.reader.seek_align(4)?;
            object.path_id = asset.reader.read_u64_by(endian)?;
        }

        object.start = match version >= 22 {
            true => asset.reader.read_u64_by(endian)?,
            false => asset.reader.read_u32_by(endian)?.into(),
        };
        object.start += asset.header.offset;

        object.size = asset.reader.read_u32_by(endian)?;

        let err = || Error::UnknownClassIDType;
        object.r#type = ClassType::from_i32(asset.reader.read_i32_by(endian)?).ok_or_else(err)?;

        if version < 16 {
            object.class_id = asset.reader.read_u16_by(endian)?;
            object.asset_type = None;

            for t in asset.types.iter() {
                if ClassType::from_i32(t.class_id).ok_or_else(err)? == object.r#type {
                    object.asset_type = Some(t.clone());
                    break;
                }
            }
        } else {
            let index = ToPrimitive::to_usize(&object.r#type).ok_or_else(err)?;
            let asset_type = &asset.types[index];
            object.class_id = asset_type.class_id.try_into()?;
            object.asset_type = Some(asset_type.clone());
        }

        if version < 11 {
            object.is_destroyed = asset.reader.read_u16_by(endian)?;
        }

        if (11..17).contains(&version) {
            let script_type_index = asset.reader.read_i16_by(endian)?;
            if let Some(s) = &mut object.asset_type {
                s.script_index = script_type_index;
            }
        }

        if version == 15 || version == 16 {
            object.stripped = asset.reader.read_u8()? > 0;
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
