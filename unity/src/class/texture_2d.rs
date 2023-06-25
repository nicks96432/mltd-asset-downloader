use super::{Class, Texture};
use crate::asset::ClassInfo;
use crate::error::Error;
use crate::traits::{
    ReadAlignedString, ReadPrimitiveExt, ReadVecExt, SeekAlign, WriteAlign, WritePrimitiveExt,
};
use crate::utils::{bool_to_yes_no, Version};

use byteorder::{ReadBytesExt, WriteBytesExt};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};

use std::any::type_name;
use std::backtrace::Backtrace;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, Write};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StreamingInfo {
    pub offset: u64,
    pub size: u32,
    pub path: String,

    pub(crate) unity_version: Version,
    pub(crate) big_endian: bool,
}

impl StreamingInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut streaming_info = Self::new();
        streaming_info.unity_version = class_info.unity_version.clone();
        streaming_info.big_endian = class_info.big_endian;

        streaming_info.offset =
            match class_info.unity_version >= Version::from_str("2020.0.0").unwrap() {
                true => reader.read_u64_by(class_info.big_endian)?,
                false => u64::from(reader.read_u32_by(class_info.big_endian)?),
            };

        streaming_info.size = reader.read_u32_by(class_info.big_endian)?;
        streaming_info.path = reader.read_aligned_string(class_info.big_endian, 4)?;

        Ok(streaming_info)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write + Seek,
    {
        match self.unity_version >= Version::from_str("2020.0.0").unwrap() {
            true => writer.write_u64_by(self.offset, self.big_endian)?,
            false => writer.write_u32_by(u32::try_from(self.offset)?, self.big_endian)?,
        };

        writer.write_u32_by(self.size, self.big_endian)?;

        writer.write_u32_by(u32::try_from(self.path.len())?, self.big_endian)?;
        writer.write_all(self.path.as_bytes())?;
        writer.write_align(4)?;

        Ok(())
    }
}

impl Display for StreamingInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(f, "{:indent$}Offset: {}", "", self.offset, indent = indent)?;
        writeln!(f, "{:indent$}Size:   {}", "", self.size, indent = indent)?;
        writeln!(f, "{:indent$}Path:   {}", "", self.path, indent = indent)?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GLTextureSettings {
    pub filter_mode: u32,
    pub aniso: u32,
    pub mip_bias: f32,
    pub wrap_mode: u32,
    pub wrap_v: u32,
    pub wrap_w: u32,

    pub(crate) unity_version: Version,
    pub(crate) big_endian: bool,
}

impl GLTextureSettings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut gl_texture_settings = Self::new();
        gl_texture_settings.unity_version = class_info.unity_version.clone();
        gl_texture_settings.big_endian = class_info.big_endian;

        gl_texture_settings.filter_mode = reader.read_u32_by(class_info.big_endian)?;
        gl_texture_settings.aniso = reader.read_u32_by(class_info.big_endian)?;
        gl_texture_settings.mip_bias = reader.read_f32_by(class_info.big_endian)?;
        gl_texture_settings.wrap_mode = reader.read_u32_by(class_info.big_endian)?;

        if class_info.unity_version >= Version::from_str("2017.0.0").unwrap() {
            gl_texture_settings.wrap_v = reader.read_u32_by(class_info.big_endian)?;
            gl_texture_settings.wrap_w = reader.read_u32_by(class_info.big_endian)?;
        }

        Ok(gl_texture_settings)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        writer.write_u32_by(self.filter_mode, self.big_endian)?;
        writer.write_u32_by(self.aniso, self.big_endian)?;
        writer.write_f32_by(self.mip_bias, self.big_endian)?;
        writer.write_u32_by(self.wrap_mode, self.big_endian)?;

        if self.unity_version >= Version::from_str("2017.0.0").unwrap() {
            writer.write_u32_by(self.wrap_v, self.big_endian)?;
            writer.write_u32_by(self.wrap_w, self.big_endian)?;
        }

        Ok(())
    }
}

impl Display for GLTextureSettings {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Filter mode: {}",
            "",
            self.filter_mode,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Aniso:       {}",
            "",
            self.aniso,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Mip bias:    {}",
            "",
            self.mip_bias,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Wrap mode:   {}",
            "",
            self.wrap_mode,
            indent = indent
        )?;

        if self.unity_version >= Version::from_str("2017.0.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Wrap V:      {}",
                "",
                self.wrap_v,
                indent = indent
            )?;
            writeln!(
                f,
                "{:indent$}Wrap W:      {}",
                "",
                self.wrap_w,
                indent = indent
            )?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, FromPrimitive, ToPrimitive)]
pub enum TextureFormat {
    #[default]
    Unknown = -1,

    Alpha8 = 1,
    ARGB4444 = 2,
    RGB24 = 3,
    RGBA32 = 4,
    ARGB32 = 5,
    RGB565 = 7,
    R16 = 9,
    DXT1 = 10,
    DXT5 = 12,
    RGBA4444 = 13,
    BGRA32 = 14,
    RHalf = 15,
    RGHalf = 16,
    RGBAHalf = 17,
    RFloat = 18,
    RGFloat = 19,
    RGBAFloat = 20,
    YUY2 = 21,
    RGB9e5Float = 22,
    BC4 = 26,
    BC5 = 27,
    BC6H = 24,
    BC7 = 25,
    DXT1Crunched = 28,
    DXT5Crunched = 29,
    PVRTCRGB2 = 30,
    PVRTCRGBA2 = 31,
    PVRTCRGB4 = 32,
    PVRTCRGBA4 = 33,
    ETCRGB4 = 34,
    ATCRGB4 = 35,
    ATCRGBA8 = 36,
    EACR = 41,
    EACRSigned = 42,
    EACRG = 43,
    EACRGSigned = 44,
    ETC2RGB = 45,
    ETC2RGBA1 = 46,
    ETC2RGBA8 = 47,
    ASTCRGB4x4 = 48,
    ASTCRGB5x5 = 49,
    ASTCRGB6x6 = 50,
    ASTCRGB8x8 = 51,
    ASTCRGB10x10 = 52,
    ASTCRGB12x12 = 53,
    ASTCRGBA4x4 = 54,
    ASTCRGBA5x5 = 55,
    ASTCRGBA6x6 = 56,
    ASTCRGBA8x8 = 57,
    ASTCRGBA10x10 = 58,
    ASTCRGBA12x12 = 59,
    ETCRGB4_3DS = 60,
    ETCRGBA8_3DS = 61,
    RG16 = 62,
    R8 = 63,
    ETCRGB4Crunched = 64,
    ETC2RGBA8Crunched = 65,
    ASTCHDR4x4 = 66,
    ASTCHDR5x5 = 67,
    ASTCHDR6x6 = 68,
    ASTCHDR8x8 = 69,
    ASTCHDR10x10 = 70,
    ASTCHDR12x12 = 71,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Texture2D {
    pub texture: Texture,
    pub width: u32,
    pub height: u32,
    pub complete_image_size: u32,
    pub mips_stripped: u32,
    pub texture_format: TextureFormat,
    pub mip_map: bool,
    pub mip_count: u32,
    pub is_readable: bool,
    pub is_preprocessed: bool,
    pub ignore_master_texture_limit: bool,
    pub read_allowed: bool,
    pub streaming_mipmaps: bool,
    pub streaming_mipmaps_priority: u32,
    pub image_count: u32,
    pub texture_dimension: u32,
    pub texture_settings: GLTextureSettings,
    pub lightmap_format: u32,
    pub color_space: u32,
    pub platform_blob: Vec<u8>,
    pub image_data: Vec<u8>,
    pub stream_data: StreamingInfo,

    pub(crate) big_endian: bool,
    pub(crate) unity_version: Version,
}

impl Texture2D {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut texture_2d = Self::new();
        texture_2d.big_endian = class_info.big_endian;
        texture_2d.unity_version = class_info.unity_version.clone();

        texture_2d.texture = Texture::read(reader, class_info)?;

        texture_2d.width = reader.read_u32_by(class_info.big_endian)?;
        texture_2d.height = reader.read_u32_by(class_info.big_endian)?;
        texture_2d.complete_image_size = reader.read_u32_by(class_info.big_endian)?;

        if class_info.unity_version >= Version::from_str("2020.0.0").unwrap() {
            texture_2d.mips_stripped = reader.read_u32_by(class_info.big_endian)?;
        }

        let texture_format = reader.read_u32_by(class_info.big_endian)?;
        texture_2d.texture_format = match TextureFormat::from_u32(texture_format) {
            Some(f) => f,
            None => {
                return Err(Error::UnknownTextureFormat {
                    format: texture_format,
                    backtrace: Backtrace::capture(),
                })
            }
        };

        if class_info.unity_version < Version::from_str("5.2.0").unwrap() {
            texture_2d.mip_map = reader.read_u8()? > 0;
        } else {
            texture_2d.mip_count = reader.read_u32_by(class_info.big_endian)?;
        }

        if class_info.unity_version >= Version::from_str("2.6.0").unwrap() {
            texture_2d.is_readable = reader.read_u8()? > 0;
        }
        if class_info.unity_version >= Version::from_str("2020.0.0").unwrap() {
            texture_2d.is_preprocessed = reader.read_u8()? > 0;
        }
        if class_info.unity_version >= Version::from_str("2019.3.0").unwrap() {
            texture_2d.ignore_master_texture_limit = reader.read_u8()? > 0;
        }
        if class_info.unity_version >= Version::from_str("3.0.0").unwrap()
            && class_info.unity_version <= Version::from_str("5.4.0").unwrap()
        {
            texture_2d.read_allowed = reader.read_u8()? > 0;
        }
        if class_info.unity_version >= Version::from_str("2018.2.0").unwrap() {
            texture_2d.streaming_mipmaps = reader.read_u8()? > 0;
        }

        reader.seek_align(4)?;

        if class_info.unity_version >= Version::from_str("2018.2.0").unwrap() {
            texture_2d.streaming_mipmaps_priority = reader.read_u32_by(class_info.big_endian)?;
        }
        texture_2d.image_count = reader.read_u32_by(class_info.big_endian)?;
        texture_2d.texture_dimension = reader.read_u32_by(class_info.big_endian)?;
        texture_2d.texture_settings = GLTextureSettings::read(reader, class_info)?;

        if class_info.unity_version >= Version::from_str("3.0.0").unwrap() {
            texture_2d.lightmap_format = reader.read_u32_by(class_info.big_endian)?;
        }
        if class_info.unity_version >= Version::from_str("3.5.0").unwrap() {
            texture_2d.color_space = reader.read_u32_by(class_info.big_endian)?;
        }
        if class_info.unity_version >= Version::from_str("2020.2.0").unwrap() {
            texture_2d.platform_blob = reader.read_u8_vec_by(class_info.big_endian)?;
            reader.seek_align(4)?;
        }

        texture_2d.image_data = reader.read_u8_vec_by(class_info.big_endian)?;

        if class_info.unity_version >= Version::from_str("5.3.0").unwrap() {
            texture_2d.stream_data = StreamingInfo::read(reader, class_info)?;
        }

        Ok(texture_2d)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write + Seek,
    {
        self.texture.save(writer)?;

        writer.write_u32_by(self.width, self.big_endian)?;
        writer.write_u32_by(self.height, self.big_endian)?;
        writer.write_u32_by(self.complete_image_size, self.big_endian)?;

        if self.unity_version >= Version::from_str("2020.0.0").unwrap() {
            writer.write_u32_by(self.mips_stripped, self.big_endian)?;
        }

        writer.write_u32_by(
            ToPrimitive::to_u32(&self.texture_format).ok_or(Error::UnknownTextureFormat {
                format: 0,
                backtrace: Backtrace::capture(),
            })?,
            self.big_endian,
        )?;

        if self.unity_version < Version::from_str("5.2.0").unwrap() {
            writer.write_u8(u8::from(self.mip_map))?;
        } else {
            writer.write_u32_by(self.mip_count, self.big_endian)?;
        }

        if self.unity_version >= Version::from_str("2.6.0").unwrap() {
            writer.write_u8(u8::from(self.is_readable))?;
        }
        if self.unity_version >= Version::from_str("2020.0.0").unwrap() {
            writer.write_u8(u8::from(self.is_readable))?;
        }
        if self.unity_version >= Version::from_str("2019.3.0").unwrap() {
            writer.write_u8(u8::from(self.ignore_master_texture_limit))?;
        }
        if self.unity_version >= Version::from_str("3.0.0").unwrap()
            && self.unity_version <= Version::from_str("5.4.0").unwrap()
        {
            writer.write_u8(u8::from(self.read_allowed))?;
        }
        if self.unity_version >= Version::from_str("2018.2.0").unwrap() {
            writer.write_u8(u8::from(self.streaming_mipmaps))?;
        }

        writer.write_align(4)?;

        if self.unity_version >= Version::from_str("2018.2.0").unwrap() {
            writer.write_u32_by(self.streaming_mipmaps_priority, self.big_endian)?;
        }
        writer.write_u32_by(self.image_count, self.big_endian)?;
        writer.write_u32_by(self.texture_dimension, self.big_endian)?;
        self.texture_settings.save(writer)?;

        if self.unity_version >= Version::from_str("3.0.0").unwrap() {
            writer.write_u32_by(self.lightmap_format, self.big_endian)?;
        }
        if self.unity_version >= Version::from_str("3.5.0").unwrap() {
            writer.write_u32_by(self.color_space, self.big_endian)?;
        }
        if self.unity_version >= Version::from_str("2020.2.0").unwrap() {
            writer.write_u32_by(u32::try_from(self.platform_blob.len())?, self.big_endian)?;
            writer.write_all(&self.platform_blob)?;
            writer.write_align(4)?;
        }

        writer.write_u32_by(u32::try_from(self.image_data.len())?, self.big_endian)?;
        writer.write_all(&self.image_data)?;

        if self.unity_version >= Version::from_str("5.3.0").unwrap() {
            self.stream_data.save(writer)?;
        }

        Ok(())
    }
}

impl Display for Texture2D {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Super ({}):",
            "",
            type_name::<Texture>(),
            indent = indent
        )?;
        write!(f, "{:indent$}", self.texture, indent = indent + 4)?;

        writeln!(
            f,
            "{:indent$}Width:                           {}",
            "",
            self.width,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Height:                          {}",
            "",
            self.height,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Complete image size:             {}",
            "",
            self.complete_image_size,
            indent = indent
        )?;
        if self.unity_version >= Version::from_str("2020.0.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Mips stripped:               {}",
                "",
                self.mips_stripped,
                indent = indent
            )?;
        }
        writeln!(
            f,
            "{:indent$}Texture format:                  {:?}",
            "",
            self.texture_format,
            indent = indent
        )?;

        if self.unity_version < Version::from_str("5.2.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Mip map?                     {}",
                "",
                self.mip_map,
                indent = indent
            )?;
        } else {
            writeln!(
                f,
                "{:indent$}Mip count:                   {}",
                "",
                self.mip_count,
                indent = indent
            )?;
        }

        if self.unity_version >= Version::from_str("2.6.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Is readable?                 {}",
                "",
                bool_to_yes_no(self.is_readable),
                indent = indent
            )?;
        }
        if self.unity_version >= Version::from_str("2020.0.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Is preprocessed?             {}",
                "",
                bool_to_yes_no(self.is_preprocessed),
                indent = indent
            )?;
        }
        if self.unity_version >= Version::from_str("2019.3.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Ignore master texture limit? {}",
                "",
                bool_to_yes_no(self.ignore_master_texture_limit),
                indent = indent
            )?;
        }
        if self.unity_version >= Version::from_str("3.0.0").unwrap()
            && self.unity_version <= Version::from_str("5.4.0").unwrap()
        {
            writeln!(
                f,
                "{:indent$}Read allowed?                {}",
                "",
                bool_to_yes_no(self.read_allowed),
                indent = indent
            )?;
        }
        if self.unity_version >= Version::from_str("2018.2.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Streaming mipmaps?           {}",
                "",
                bool_to_yes_no(self.streaming_mipmaps),
                indent = indent
            )?;
        }

        if self.unity_version >= Version::from_str("2018.2.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Streaming mipmaps priority:  {}",
                "",
                self.streaming_mipmaps_priority,
                indent = indent
            )?;
        }
        writeln!(
            f,
            "{:indent$}Image count:                 {}",
            "",
            self.image_count,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Texture dimension:           {}",
            "",
            self.image_count,
            indent = indent
        )?;
        writeln!(f, "{:indent$}Texture settings:", "", indent = indent)?;
        write!(f, "{:indent$}:", self.texture_settings, indent = indent + 4)?;

        if self.unity_version >= Version::from_str("3.0.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Lightmap format:             {}",
                "",
                self.lightmap_format,
                indent = indent
            )?;
        }
        if self.unity_version >= Version::from_str("3.5.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Color space:                 {}",
                "",
                self.color_space,
                indent = indent
            )?;
        }
        if self.unity_version >= Version::from_str("2020.2.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Platform blob:               {} byte(s)",
                "",
                self.platform_blob.len(),
                indent = indent
            )?;
        }

        writeln!(
            f,
            "{:indent$}Image data:                  {} byte(s)",
            "",
            self.image_data.len(),
            indent = indent
        )?;

        if self.unity_version >= Version::from_str("5.3.0").unwrap() {
            writeln!(f, "{:indent$}Stream data:", "", indent = indent)?;
            writeln!(f, "{:indent$}", self.stream_data, indent = indent + 4)?;
        }
        Ok(())
    }
}

impl Class for Texture2D {}
