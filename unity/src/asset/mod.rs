mod header;
mod object;
mod platform;
mod serialized_type;

pub use self::header::*;
pub use self::object::*;
pub use self::platform::*;
pub use self::serialized_type::*;

use crate::error::Error;
use crate::macros::impl_default;
use crate::traits::ReadIntExt;
use linked_hash_map::LinkedHashMap;
use std::io::{Read, Seek};

pub struct Asset {
    pub header: Header,
    pub types: Vec<SerializedType>,
    pub big_id_enabled: i32,
    pub objects: LinkedHashMap<u64, Object>,
}

impl Asset {
    fn new() -> Self {
        Self {
            header: Header::new(),
            types: Vec::new(),
            big_id_enabled: 0i32,
            objects: LinkedHashMap::new(),
        }
    }

    pub fn read<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut asset = Self::new();

        asset.header = Header::read(reader)?;
        log::trace!("asset header:\n{:#?}", &asset.header);

        let endian = asset.header.endian;
        let version = asset.header.version;

        let class_count = reader.read_u32_by(endian)?;
        log::trace!("{} asset class(es)", class_count);

        for i in 0..class_count {
            let ser_type = SerializedType::read(reader, &asset.header)?;
            log::trace!("asset class {}:\n{:#?}", i, ser_type);
            asset.types.push(ser_type);
        }

        if (7..14).contains(&version) {
            asset.big_id_enabled = reader.read_i32_by(endian)?;
        }

        let object_count = reader.read_i32_by(endian)?;
        for i in 0..object_count {
            let object = Object::read(reader, &asset)?;
            log::trace!("asset object {}:\n{:#?}", i, object);
            asset.objects.insert(object.path_id, object);
        }

        Ok(asset)
    }
}

impl_default!(Asset);
