use crate::error::Error;
use crate::utils::Vector3;

use std::fmt::{Display, Formatter};
use std::io::{Read, Write};

/// Axis-aligned Bounding Box
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AABB {
    pub center: Vector3,
    pub extent: Vector3,

    pub endian: bool,
}

impl AABB {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, endian: bool) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut aabb = Self::new();
        aabb.endian = endian;

        aabb.center = Vector3::read(reader, endian)?;
        aabb.extent = Vector3::read(reader, endian)?;

        Ok(aabb)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        self.center.save(writer, self.endian)?;
        self.extent.save(writer, self.endian)?;

        Ok(())
    }
}

impl Display for AABB {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Center: {:?}",
            "",
            self.center,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Extent: {:?}",
            "",
            self.extent,
            indent = indent
        )?;

        Ok(())
    }
}
