pub mod type_tree;

use crate::bundle::Signature;
use crate::error::Error;
use crate::traits::ReadString;
use std::io::{Read, Seek};
use std::str::FromStr;

pub enum FileType {
    AssetBundle(Signature),
    Unknown,
}

impl FileType {
    pub fn parse<T>(value: &mut T) -> Result<Self, Error>
    where
        T: Read + Seek,
    {
        if let Ok(s) = Signature::from_str(&value.read_string()?) {
            return Ok(Self::AssetBundle(s));
        }

        Ok(Self::Unknown)
    }
}
