mod asset_type;
mod header;
mod metadata;
mod object_info;
mod platform;
mod type_tree;

pub use self::asset_type::*;
pub use self::header::*;
pub use self::metadata::*;
pub use self::object_info::*;
pub use self::platform::*;
pub use self::type_tree::*;

use crate::class::ClassType;
use crate::error::Error;
use crate::traits::ReadIntExt;
use crate::traits::ReadString;
use crate::traits::SeekAlign;

use linked_hash_map::LinkedHashMap;

use std::cell::RefCell;
use std::fmt::Display;
use std::fmt::Formatter;
use std::io::Cursor;
use std::io::Read;
use std::io::Write;
use std::rc::Rc;

#[derive(Debug, Clone, Default)]
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

    pub fn read(asset: &mut Asset) -> Result<Self, Error> {
        let header = &asset.header;
        let reader = &mut asset.reader;

        let mut identifier = Self::new();

        identifier.index = reader.read_i32_by(header.big_endian)?;
        identifier.id = match header.version {
            v if v >= 14 => {
                reader.seek_align(4)?;
                reader.read_i64_by(header.big_endian)?
            }
            _ => reader.read_i32_by(header.big_endian)?.into(),
        };

        Ok(identifier)
    }
}

#[derive(Debug, Clone, Default)]
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

    pub fn read(asset: &mut Asset) -> Result<Self, Error> {
        let header = &asset.header;
        let reader = &mut asset.reader;

        let mut identifier = FileIdentifier::new();

        if header.version >= 6 {
            identifier.temp_empty = reader.read_string()?;
        }
        if header.version >= 5 {
            reader.read_exact(&mut identifier.guid)?;
            identifier.file_type = reader.read_i32_by(header.big_endian)?;
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

#[derive(Debug, Clone, Default)]
pub struct Asset {
    pub header: Header,
    pub metadata: Metadata,
    pub objects: LinkedHashMap<u64, ObjectInfo>,
    pub scripts: Vec<ScriptIdentifier>,
    pub externals: Vec<FileIdentifier>,
    pub ref_types: Vec<AssetType>,
    pub user_information: String,

    reader: Cursor<Vec<u8>>,
}

impl Asset {
    pub fn new() -> Self {
        Self {
            header: Header::new(),
            metadata: Metadata::new(),
            objects: LinkedHashMap::new(),
            scripts: Vec::new(),
            externals: Vec::new(),
            ref_types: Vec::new(),
            user_information: String::new(),
            reader: Cursor::new(Vec::new()),
        }
    }

    pub fn read(reader: Cursor<Vec<u8>>) -> Result<Rc<RefCell<Self>>, Error> {
        let mut asset = Self::new();
        asset.reader = reader;

        log::debug!("reading asset header");
        asset.header = Header::read(&mut asset.reader)?;
        log::trace!("asset header:\n{:#?}", &asset.header);

        log::debug!("reading asset metadata");
        asset.metadata = Metadata::read(&mut asset)?;
        log::trace!("asset metadata:\n{:#?}", &asset.metadata);

        let version = asset.header.version;
        let big_endian = asset.header.big_endian;

        if version >= 11 {
            log::debug!("reading asset scripts");
            let script_count = asset.reader.read_u32_by(big_endian)?;
            log::trace!("{} asset script(s)", script_count);

            for i in 0..script_count {
                let script = ScriptIdentifier::read(&mut asset)?;
                log::trace!("asset script {}:\n{:#?}", i, &script);
                asset.scripts.push(script);
            }
        }

        log::debug!("reading external files");
        let external_count = asset.reader.read_i32_by(big_endian)?;
        log::trace!("{} asset external file(s)", external_count);

        for i in 0..external_count {
            let external = FileIdentifier::read(&mut asset)?;
            log::trace!("asset external {}:\n{:#?}", i, &external);
            asset.externals.push(external);
        }

        if version >= 20 {
            log::debug!("reading asset ref types");
            let ref_type_count = asset.reader.read_i32_by(big_endian)?;
            log::trace!("{} asset ref type(s)", ref_type_count);
            for i in 0..ref_type_count {
                let ref_type = AssetType::read(&mut asset, true)?;
                log::trace!("asset ref type {}:\n{:#?}", i, &ref_type);
                asset.ref_types.push(ref_type);
            }
        }

        if version >= 5 {
            asset.user_information = asset.reader.read_string()?;
        }

        // TODO: object read type tree
        for object in asset.objects.values() {
            if object.class_type == ClassType::AssetBundle {
                todo!()
            }
        }

        Ok(Rc::new(RefCell::new(asset)))
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
        write!(f, "{:indent$}:", self.metadata, indent = indent + 4)?;

        Ok(())
    }
}
