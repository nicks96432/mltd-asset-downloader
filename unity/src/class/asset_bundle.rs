use super::{Class, NamedObject, PPtr};
use crate::asset::ClassInfo;
use crate::error::Error;
use crate::traits::{ReadAlignedString, ReadIntExt};

use std::any::type_name;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AssetInfo {
    pub preload_index: u32,
    pub preload_size: u32,
    pub asset: PPtr,
}

impl AssetInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut asset_info = AssetInfo::new();

        asset_info.preload_index = reader.read_u32_by(class_info.big_endian)?;
        asset_info.preload_size = reader.read_u32_by(class_info.big_endian)?;
        asset_info.asset = PPtr::read(reader, class_info)?;

        Ok(asset_info)
    }
}

impl Display for AssetInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);
        writeln!(
            f,
            "{:indent$}Preload index: {}",
            "",
            self.preload_index,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Preload size: {}",
            "",
            self.preload_size,
            indent = indent
        )?;
        writeln!(f, "{:indent$}Asset (pptr):", "", indent = indent)?;
        write!(f, "{:indent$}", self.asset, indent = indent + 4)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AssetBundle {
    pub named_object: NamedObject,
    pub preload_table: Vec<PPtr>,
    pub container: HashMap<String, Vec<AssetInfo>>,
}

impl AssetBundle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut asset_bundle = Self::new();
        asset_bundle.named_object = NamedObject::read(reader, class_info)?;

        log::trace!("cursor is now at {}", reader.stream_position()?);

        let preload_table_size = reader.read_u32_by(class_info.big_endian)?;
        for _ in 0..preload_table_size {
            asset_bundle
                .preload_table
                .push(PPtr::read(reader, class_info)?);
        }

        let container_size = reader.read_u32_by(class_info.big_endian)?;
        for _ in 0..container_size {
            let key = reader.read_aligned_string(class_info.big_endian, 4)?;
            let asset_info = AssetInfo::read(reader, class_info)?;

            match asset_bundle.container.get_mut(&key) {
                Some(v) => {
                    v.push(asset_info);
                }
                None => {
                    asset_bundle.container.insert(key, vec![asset_info]);
                }
            };
        }

        println!("{}", reader.stream_position()?);

        Ok(asset_bundle)
    }
}

impl Display for AssetBundle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Super ({}):",
            "",
            type_name::<NamedObject>(),
            indent = indent
        )?;
        write!(f, "{:indent$}", self.named_object, indent = indent + 4)?;

        writeln!(f, "{:indent$}Preload table:", "", indent = indent)?;
        for (i, pptr) in self.preload_table.iter().enumerate() {
            writeln!(f, "{:indent$}PPtr {}:", "", i, indent = indent + 4)?;
            write!(f, "{:indent$}", pptr, indent = indent + 8)?;
        }

        writeln!(f, "{:indent$}Asset infos:", "", indent = indent)?;
        let mut asset_info_count = 0u32;
        for (key, values) in self.container.iter() {
            for value in values.iter() {
                writeln!(
                    f,
                    "{:indent$}Asset info {} ({}):",
                    "",
                    asset_info_count,
                    key,
                    indent = indent + 4
                )?;
                write!(f, "{:indent$}", value, indent = indent + 8)?;
                asset_info_count += 1;
            }
        }

        Ok(())
    }
}

impl Class for AssetBundle {}
