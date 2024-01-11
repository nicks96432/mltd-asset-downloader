use std::collections::HashMap;
use std::error::Error;
use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};
use rabex::read_ext::ReadUrexExt;

#[derive(Debug, Default)]
pub struct Environment {
    /// resource data that loaded from bundles
    resources: HashMap<String, Vec<u8>>,

    /// object data that loaded from resources
    objects: HashMap<i64, Vec<u8>>,
}

impl Environment {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_cab(&mut self, path: &str, buf: Vec<u8>) {
        self.resources.insert(path.to_owned(), buf);
    }

    pub fn get_cab<'a>(&'a self, path: &str) -> Option<&'a [u8]> {
        self.resources.get(path).map(|x| x.as_slice())
    }

    pub fn register_object(&mut self, path_id: i64, buf: Vec<u8>) {
        self.objects.insert(path_id, buf);
    }

    pub fn get_object(&self, path_id: i64) -> Option<&[u8]> {
        self.objects.get(&path_id).map(|x| x.as_slice())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileType {
    AssetsFile = 0,
    BundleFile = 1,
    WebFile = 2,
    ResourceFile = 9,
    Zip = 10,
}

pub fn check_file_type(file: &mut (impl Read + Seek)) -> Result<FileType, Box<dyn Error>> {
    file.seek(SeekFrom::Start(0))?;
    let file_len = file.seek(SeekFrom::End(0))?;
    file.seek(SeekFrom::Start(0))?;

    if file_len < 20 {
        return Ok(FileType::ResourceFile);
    }

    let signature = file.read_bytes_sized(20)?;
    file.seek(SeekFrom::Start(0))?;

    let bundle_signatures: Vec<&[u8]> = vec![b"UnityWeb", b"UnityRaw", &[0xfa; 8], b"UnityFS"];

    if bundle_signatures.iter().any(|&x| signature.starts_with(x)) {
        return Ok(FileType::BundleFile);
    }

    if signature.starts_with(b"UnityWebData1.0") {
        return Ok(FileType::WebFile);
    }

    if signature.starts_with(b"PK\x03\x04") {
        return Ok(FileType::Zip);
    }

    if file_len < 128 {
        return Ok(FileType::ResourceFile);
    }

    const GZIP_SIGNATURE: &[u8; 2] = b"\x1F\x8B";
    if signature.starts_with(GZIP_SIGNATURE) {
        return Ok(FileType::WebFile);
    }

    let signature = file.read_bytes_sized(6)?;
    if signature.starts_with(b"brotli") {
        return Ok(FileType::WebFile);
    }

    file.seek(SeekFrom::Start(0))?;

    // read as if it's an serialized file
    let mut metadata_size = file.read_u32::<BigEndian>()?;
    let file_size = file.read_u32::<BigEndian>()? as i64;
    let version = file.read_u32::<BigEndian>()? as i64;
    let mut data_offset = file.read_u32::<BigEndian>()? as i64;

    if version >= 9 {
        // skip endian and reserved
        file.seek(SeekFrom::Current(4))?;
    }

    if version >= 22 {
        metadata_size = file.read_u32::<BigEndian>()?;
        // skip file size
        file.seek(SeekFrom::Current(8))?;
        data_offset = file.read_i64::<BigEndian>()?;
    }

    file.seek(SeekFrom::Start(0))?;

    if version > 100
        || [file_size, metadata_size as i64, version, data_offset]
            .iter()
            .any(|&x| x < 0 || x > file_len as i64)
        || file_size < metadata_size as i64
        || file_size < data_offset
    {
        return Ok(FileType::ResourceFile);
    }

    Ok(FileType::AssetsFile)
}
