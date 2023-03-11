mod class;
mod header;
mod platform;

pub use self::class::*;
pub use self::header::*;
pub use self::platform::*;

use crate::error::Error;
use crate::macros::impl_default;
use crate::traits::ReadIntExt;
use std::io::{Read, Seek};

pub struct Asset {
    pub header: Header,
    pub classes: Vec<Class>,
    pub big_id_enabled: i32,
}

impl Asset {
    fn new() -> Self {
        Self {
            header: Header::new(),
            classes: Vec::new(),
            big_id_enabled: 0i32,
        }
    }

    pub fn read<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut asset = Self::new();

        asset.header = Header::read(reader)?;
        log::trace!("asset header:\n{:#?}", &asset.header);

        let Header {
            endian, version, ..
        } = asset.header;

        let class_count = reader.read_u32_by(endian)?;
        log::trace!("{} asset class(es)", class_count);

        for i in 0..class_count {
            let class = Class::read(reader, &asset.header)?;
            log::trace!("asset class {}:\n{:#?}", i, class);
            asset.classes.push(class);
        }

        if (7..14).contains(&version) {
            asset.big_id_enabled = reader.read_i32_by(endian)?;
        }

        Ok(asset)
    }
}

impl_default!(Asset);
