mod header;
mod platform;

pub use self::header::*;
pub use self::platform::*;

use crate::error::Error;
use crate::traits::UnityIO;
use std::io::{Read, Seek, Write};

pub struct Asset {
    pub header: AssetHeader,
}

impl UnityIO for Asset {
    fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Error> {
        let header = AssetHeader::read(reader)?;

        Ok(Self { header })
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        todo!()
    }
}
