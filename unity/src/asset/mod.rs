mod class;
mod header;
mod platform;

pub use self::class::*;
pub use self::header::*;
pub use self::platform::*;

use crate::error::Error;
use crate::macros::impl_default;
use crate::traits::{ReadIntExt, ReadString};
use byteorder::ReadBytesExt;
use num_traits::FromPrimitive;
use std::io::{Read, Seek};

pub struct Asset {
    pub header: Header,
    pub unknown: u64,
    pub unity_version: String,
    pub target_platform: Platform,
    pub type_tree: bool,
    pub classes: Vec<Class>,
}

impl Asset {
    fn new() -> Self {
        Self {
            header: Header::new(),
            unknown: 0u64,
            unity_version: String::new(),
            target_platform: Platform::UnknownPlatform,
            type_tree: false,
            classes: Vec::new(),
        }
    }

    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Error> {
        let mut asset = Self::new();

        asset.header = Header::read(reader)?;
        log::trace!("asset header:\n{:#?}", &asset.header);

        if asset.header.version >= 7 {
            asset.unity_version = reader.read_string()?;
        }
        if asset.header.version >= 8 {
            asset.target_platform = Platform::from_u32(reader.read_u32_by(asset.header.endian)?)
                .ok_or(Error::UnknownSignature)?;
        }

        if asset.header.version >= 13 {
            asset.type_tree = reader.read_u8()? > 0;
        }

        let class_count = reader.read_u32_by(asset.header.endian)?;
        for _ in 0..class_count {
            let class = Class::read(reader, &asset)?;
            asset.classes.push(class);
        }

        Ok(asset)
    }
}

impl_default!(Asset);
