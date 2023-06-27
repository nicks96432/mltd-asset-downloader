mod class_info;
mod class_type;
mod header;
mod metadata;
mod platform;
mod type_tree;

pub use self::class_info::*;
pub use self::class_type::*;
pub use self::header::*;
pub use self::metadata::*;
pub use self::platform::*;
pub use self::type_tree::*;

use crate::class::{Class, ClassReader};
use crate::error::Error;

use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, Write};

#[derive(Debug, Default)]
pub struct Asset {
    pub header: Header,
    pub metadata: Metadata,
    pub classes: Vec<Box<dyn Class>>,
}

impl Asset {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut asset = Self::new();

        log::debug!("reading asset header");
        asset.header = Header::read(reader)?;
        log::trace!("asset header:\n{}", &asset.header);

        log::debug!("reading asset metadata");
        asset.metadata = Metadata::read(reader, &asset.header)?;
        log::trace!("asset metadata:\n{}", &asset.metadata);

        for (i, class_info) in asset.metadata.class_infos.iter().enumerate() {
            let class = ClassReader::read(reader, class_info)?;
            log::trace!("class {}:\n{}", i, class);
            asset.classes.push(class);
        }

        Ok(asset)
    }

    pub fn save<W>(&self, _writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        unimplemented!()
    }
}

impl Display for Asset {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(f, "{:indent$}Basic information:", "", indent = indent)?;
        write!(f, "{:indent$}", self.header, indent = indent + 4)?;

        writeln!(f, "{:indent$}Metadata:", "", indent = indent)?;
        write!(f, "{:indent$}", self.metadata, indent = indent + 4)?;

        writeln!(f, "{:indent$}Classes:", "", indent = indent)?;
        for (i, class) in self.classes.iter().enumerate() {
            writeln!(
                f,
                "{:indent$}Class {} ({}):",
                "",
                i,
                class.name(),
                indent = indent + 4
            )?;
            write!(f, "{:indent$}", class, indent = indent + 8)?;
        }

        Ok(())
    }
}
