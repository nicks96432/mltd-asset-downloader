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
use crate::traits::ReadString;
use crate::traits::SeekAlign;
use linked_hash_map::LinkedHashMap;
use std::io::Read;
use std::io::Seek;
use std::io::Write;

#[derive(Debug, Clone)]
pub struct ScriptIdentifier {
    pub index: i32,
    pub id: i64,
}

impl ScriptIdentifier {
    pub fn new() -> Self {
        Self {
            index: 0i32,
            id: 0i64,
        }
    }

    pub fn read<R>(reader: &mut R, header: &Header) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut identifier = Self::new();

        identifier.index = reader.read_i32_by(header.endian)?;
        identifier.id = match header.version {
            v if v >= 14 => {
                reader.seek_align(4)?;
                reader.read_i64_by(header.endian)?
            }
            _ => reader.read_i32_by(header.endian)?.into(),
        };

        Ok(identifier)
    }
}

#[derive(Debug, Clone)]
pub struct FileIdentifier {
    pub guid: [u8; 16],
    pub file_type: i32,
    pub path: String,
    pub temp_empty: String,
}

impl FileIdentifier {
    pub fn new() -> Self {
        Self {
            guid: [0u8; 16],
            file_type: 0i32,
            path: String::new(),
            temp_empty: String::new(),
        }
    }

    pub fn read<R>(reader: &mut R, header: &Header) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut identifier = FileIdentifier::new();

        if header.version >= 6 {
            identifier.temp_empty = reader.read_string()?;
        }
        if header.version >= 5 {
            reader.read_exact(&mut identifier.guid)?;
            identifier.file_type = reader.read_i32_by(header.endian)?;
        }

        identifier.path = reader.read_string()?;

        Ok(identifier)
    }

    pub fn save<W>(&self, _writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
pub struct Asset {
    pub header: Header,
    pub types: Vec<SerializedType>,
    pub big_id_enabled: i32,
    pub objects: LinkedHashMap<u64, Object>,
    pub scripts: Vec<ScriptIdentifier>,
    pub externals: Vec<FileIdentifier>,
    pub user_information: String,
}

impl Asset {
    pub fn new() -> Self {
        Self {
            header: Header::new(),
            types: Vec::new(),
            big_id_enabled: 0i32,
            objects: LinkedHashMap::new(),
            scripts: Vec::new(),
            externals: Vec::new(),
            user_information: String::new(),
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

        let type_count = reader.read_u32_by(endian)?;
        log::trace!("{} asset serized type(s)", type_count);

        for i in 0..type_count {
            let serialized_type = SerializedType::read(reader, &asset.header)?;
            log::trace!("asset class {}:\n{:#?}", i, serialized_type);
            asset.types.push(serialized_type);
        }

        if (7..14).contains(&version) {
            asset.big_id_enabled = reader.read_i32_by(endian)?;
        }

        let object_count = reader.read_i32_by(endian)?;
        log::trace!("{} asset object(s)", object_count);

        for i in 0..object_count {
            let object = Object::read(reader, &asset)?;
            log::trace!("asset object {}:\n{:#?}", i, &object);
            asset.objects.insert(object.path_id, object);
        }

        if version >= 11 {
            let script_count = reader.read_u32_by(endian)?;
            log::trace!("{} asset script(s)", script_count);

            for i in 0..script_count {
                let script = ScriptIdentifier::read(reader, &asset.header)?;
                log::trace!("asset script {}:\n{:#?}", i, &script);
                asset.scripts.push(script);
            }
        }

        let external_count = reader.read_i32_by(endian)?;
        log::trace!("{} asset external file(s)", external_count);

        for i in 0..external_count {
            let external = FileIdentifier::read(reader, &asset.header)?;
            log::trace!("asset external {}:\n{:#?}", i, &external);
            asset.externals.push(external);
        }

        // TODO: SerializedType ref type
        if version >= 20 {
            todo!();
        }

        if version >= 5 {
            asset.user_information = reader.read_string()?;
        }

        // TODO: object read type tree
        for object in asset.objects.values() {
            if object.type_id == ClassType::AssetBundle {
                todo!()
            }
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

impl_default!(ScriptIdentifier);
impl_default!(FileIdentifier);
impl_default!(Asset);
