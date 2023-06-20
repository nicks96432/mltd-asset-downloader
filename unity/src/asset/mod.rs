mod asset_type;
mod header;
mod object_reader;
mod platform;

pub use self::asset_type::*;
pub use self::header::*;
pub use self::object_reader::*;
pub use self::platform::*;

use crate::class::ClassType;
use crate::error::Error;
use crate::traits::ReadIntExt;
use crate::traits::ReadString;
use crate::traits::SeekAlign;
use crate::utils::bool_to_yes_no;

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

#[derive(Debug, Clone, Default)]
pub struct Asset {
    pub header: Header,
    pub types: Vec<AssetType>,
    pub big_id_enabled: bool,
    pub objects: LinkedHashMap<u64, ObjectReader>,
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
            types: Vec::new(),
            big_id_enabled: false,
            objects: LinkedHashMap::new(),
            scripts: Vec::new(),
            externals: Vec::new(),
            ref_types: Vec::new(),
            user_information: String::new(),

            reader: Cursor::new(Vec::new()),
        }
    }

    pub fn read(reader: Cursor<Vec<u8>>) -> Result<Rc<RefCell<Self>>, Error> {
        let asset_rc = Rc::new(RefCell::new(Self::new()));

        let object_count: i32;

        {
            let mut asset = asset_rc.try_borrow_mut()?;

            asset.reader = reader;

            log::debug!("reading asset header");
            asset.header = Header::read(&mut asset.reader)?;
            log::trace!("asset header:\n{:#?}", &asset.header);

            let endian = asset.header.endian;

            log::debug!("reading asset types");
            let type_count = asset.reader.read_u32_by(endian)?;
            log::trace!("{} asset serized type(s)", type_count);

            for i in 0..type_count {
                let r#type = AssetType::read(&mut asset, false)?;
                log::trace!("asset class {}:\n{:#?}", i, r#type);
                asset.types.push(r#type);
            }

            if (7..14).contains(&asset.header.version) {
                log::debug!("reading big_id_enabled");
                asset.big_id_enabled = asset.reader.read_i32_by(endian)? > 0i32;
            }

            log::debug!("reading objects");
            object_count = asset.reader.read_i32_by(endian)?;
            log::trace!("{} asset object(s)", object_count);
        }

        for i in 0..object_count {
            let object = ObjectReader::read(asset_rc.clone())?;
            log::trace!("asset object {}:\n{:#?}", i, &object);

            let mut asset = asset_rc.try_borrow_mut()?;
            asset.objects.insert(object.path_id, object);
        }

        {
            let mut asset = asset_rc.try_borrow_mut()?;

            let version = asset.header.version;
            let endian = asset.header.endian;

            if version >= 11 {
                log::debug!("reading asset scripts");
                let script_count = asset.reader.read_u32_by(endian)?;
                log::trace!("{} asset script(s)", script_count);

                for i in 0..script_count {
                    let script = ScriptIdentifier::read(&mut asset)?;
                    log::trace!("asset script {}:\n{:#?}", i, &script);
                    asset.scripts.push(script);
                }
            }

            log::debug!("reading external files");
            let external_count = asset.reader.read_i32_by(endian)?;
            log::trace!("{} asset external file(s)", external_count);

            for i in 0..external_count {
                let external = FileIdentifier::read(&mut asset)?;
                log::trace!("asset external {}:\n{:#?}", i, &external);
                asset.externals.push(external);
            }

            if version >= 20 {
                log::debug!("reading asset ref types");
                let ref_type_count = asset.reader.read_i32_by(endian)?;
                log::trace!("{} asset ref type(s)", ref_type_count);
                for i in 0..ref_type_count {
                    let r#type = AssetType::read(&mut asset, true)?;
                    log::trace!("asset ref type {}:\n{:#?}", i, &r#type);
                    asset.ref_types.push(r#type);
                }
            }

            if version >= 5 {
                asset.user_information = asset.reader.read_string()?;
            }

            // TODO: object read type tree
            for object in asset.objects.values() {
                if object.r#type == ClassType::AssetBundle {
                    todo!()
                }
            }
        }

        Ok(asset_rc)
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
        let indent = f.width().unwrap_or(0);
        writeln!(f, "{:indent$}Basic information:", "", indent = indent)?;
        writeln!(
            f,
            "{:indent$}Metadata size:   {}",
            "",
            self.header.metadata_size,
            indent = indent + 4
        )?;
        writeln!(
            f,
            "{:indent$}Asset Size:      {}",
            "",
            self.header.asset_size,
            indent = indent + 4
        )?;
        writeln!(
            f,
            "{:indent$}Content offset:  {}",
            "",
            self.header.offset,
            indent = indent + 4
        )?;
        let endian_str = if self.header.endian { "big" } else { "little" };
        writeln!(
            f,
            "{:indent$}Endian:          {}",
            "",
            endian_str,
            indent = indent + 4
        )?;
        writeln!(
            f,
            "{:indent$}Unity version:   {}",
            "",
            self.header.unity_version,
            indent = indent + 4
        )?;
        writeln!(
            f,
            "{:indent$}Target platform: {:?}",
            "",
            self.header.target_platform,
            indent = indent + 4
        )?;
        writeln!(
            f,
            "{:indent$}Has type tree?   {}",
            "",
            bool_to_yes_no(self.header.has_type_tree),
            indent = indent + 4
        )?;
        writeln!(
            f,
            "{:indent$}Big ID enabled?  {}",
            "",
            bool_to_yes_no(self.big_id_enabled),
            indent = indent + 4
        )?;

        writeln!(f, "{:indent$}Types:", "", indent = indent)?;
        for (i, r#type) in self.types.iter().enumerate() {
            writeln!(f, "{:indent$}Type {}:", "", i, indent = indent + 4)?;
            writeln!(f, "{:indent$}", r#type, indent = indent + 8)?;
        }

        Ok(())
    }
}
