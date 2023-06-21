use crate::asset::ObjectInfo;
use crate::error::Error;

use std::io::Read;

#[derive(Debug, Clone)]
pub struct PPtr {}

impl PPtr {
    pub fn read<R>(reader: &mut R, object: &ObjectInfo) -> Result<Self, Error>
    where
        R: Read,
    {
        Ok(Self {})
    }
}
