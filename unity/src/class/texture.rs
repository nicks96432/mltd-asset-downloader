use super::{Class, NamedObject};
use crate::asset::ClassInfo;
use crate::error::Error;
use crate::traits::{ReadPrimitiveExt, SeekAlign, WriteAlign, WritePrimitiveExt};
use crate::utils::Version;

use byteorder::{ReadBytesExt, WriteBytesExt};

use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, Write};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Texture {
    pub named_object: NamedObject,
    pub forced_callback_format: u32,
    pub downscale_fallback: bool,
    pub is_alpha_channel_optional: bool,
}

impl Texture {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut texture = Self::new();

        texture.named_object = NamedObject::read(reader, class_info)?;

        if class_info.unity_version >= Version::from_str("2017.3.0").unwrap() {
            texture.forced_callback_format = reader.read_u32_by(class_info.big_endian)?;
            texture.downscale_fallback = reader.read_u8()? > 0;
            if class_info.unity_version >= Version::from_str("2020.2.0").unwrap() {
                texture.is_alpha_channel_optional = reader.read_u8()? > 0;
            }
            reader.seek_align(4)?;
        }

        Ok(texture)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write + Seek,
    {
        let object = &self.named_object.editor_extension.object;
        let big_endian = object.big_endian;
        let unity_version = object.unity_version.clone();

        self.named_object.save(writer)?;

        if unity_version >= Version::from_str("2017.3.0").unwrap() {
            writer.write_u32_by(self.forced_callback_format, big_endian)?;
            writer.write_u8(u8::from(self.downscale_fallback))?;
            if unity_version >= Version::from_str("2020.2.0").unwrap() {
                writer.write_u8(u8::from(self.is_alpha_channel_optional))?;
            }
            writer.write_align(4)?;
        }

        Ok(())
    }
}

impl Display for Texture {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        let object = &self.named_object.editor_extension.object;
        let unity_version = object.unity_version.clone();

        writeln!(
            f,
            "{:indent$}Super ({}):",
            "",
            self.named_object.name(),
            indent = indent
        )?;
        write!(f, "{:indent$}", self.named_object, indent = indent + 4)?;

        if unity_version >= Version::from_str("2017.3.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Forced callback format:     {}",
                "",
                self.forced_callback_format,
                indent = indent
            )?;
            writeln!(
                f,
                "{:indent$}Downscale callback?:        {}",
                "",
                self.downscale_fallback,
                indent = indent
            )?;
            if unity_version >= Version::from_str("2020.2.0").unwrap() {
                writeln!(
                    f,
                    "{:indent$}Alpha channel is optional?: {}",
                    "",
                    self.is_alpha_channel_optional,
                    indent = indent
                )?;
            }
        }

        Ok(())
    }
}

impl Class for Texture {}
