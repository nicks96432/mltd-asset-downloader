use super::AABB;
use crate::asset::ClassInfo;
use crate::error::Error;
use crate::traits::{ReadPrimitiveExt, ReadVecExt, SeekAlign, WritePrimitiveExt};
use crate::utils::Version;

use byteorder::{ReadBytesExt, WriteBytesExt};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};

use std::backtrace::Backtrace;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, Write};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChannelInfo {
    pub stream: u8,
    pub offset: u8,
    pub format: u8,
    pub dimension: u8,
}

impl ChannelInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut channel_info = Self::new();

        channel_info.stream = reader.read_u8()?;
        channel_info.offset = reader.read_u8()?;
        channel_info.format = reader.read_u8()?;
        channel_info.dimension = reader.read_u8()? & 0xfu8;

        Ok(channel_info)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        writer.write_u8(self.stream)?;
        writer.write_u8(self.offset)?;
        writer.write_u8(self.format)?;
        writer.write_u8(self.dimension)?;

        Ok(())
    }
}

impl Display for ChannelInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Stream:    {}",
            "",
            self.stream,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Offset:    {}",
            "",
            self.offset,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Format:    {}",
            "",
            self.format,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Dimension: {}",
            "",
            self.dimension,
            indent = indent
        )?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StreamInfo {
    pub channel_mask: u32,
    pub offset: u32,
    pub stride: u32,
    pub align: u32,
    pub divider_op: u8,
    pub frequency: u16,

    pub(crate) big_endian: bool,
    pub(crate) unity_version: Version,
}

impl StreamInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut stream_info = Self::new();
        stream_info.big_endian = class_info.big_endian;
        stream_info.unity_version = class_info.unity_version.clone();

        stream_info.channel_mask = reader.read_u32_by(class_info.big_endian)?;
        stream_info.offset = reader.read_u32_by(class_info.big_endian)?;

        if class_info.unity_version < Version::from_str("4.0.0").unwrap() {
            stream_info.stride = reader.read_u32_by(class_info.big_endian)?;
            stream_info.align = reader.read_u32_by(class_info.big_endian)?;
        } else {
            stream_info.stride = u32::from(reader.read_u8()?);
            stream_info.divider_op = reader.read_u8()?;
            stream_info.frequency = reader.read_u16_by(class_info.big_endian)?;
        }

        Ok(stream_info)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        writer.write_u32_by(self.channel_mask, self.big_endian)?;
        writer.write_u32_by(self.offset, self.big_endian)?;

        if self.unity_version < Version::from_str("4.0.0").unwrap() {
            writer.write_u32_by(self.stride, self.big_endian)?;
            writer.write_u32_by(self.align, self.big_endian)?;
        } else {
            writer.write_u8(u8::try_from(self.stride)?)?;
            writer.write_u8(self.divider_op)?;
            writer.write_u16_by(self.frequency, self.big_endian)?;
        }

        Ok(())
    }
}

impl Display for StreamInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Channel mask:      {}",
            "",
            self.channel_mask,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Offset:            {}",
            "",
            self.offset,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Stride:            {}",
            "",
            self.stride,
            indent = indent
        )?;

        if self.unity_version < Version::from_str("4.0.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Align:             {}",
                "",
                self.align,
                indent = indent
            )?;
        } else {
            writeln!(
                f,
                "{:indent$}Divider Operation: {}",
                "",
                self.divider_op,
                indent = indent
            )?;
            writeln!(
                f,
                "{:indent$}Frequency:         {}",
                "",
                self.frequency,
                indent = indent
            )?;
        }

        Ok(())
    }
}

/// From [UnityPy](https://github.com/K0lb3/UnityPy/blob/master/UnityPy/classes/Mesh.py)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromPrimitive, ToPrimitive)]
pub enum VertexChannelFormat {
    #[default]
    Unknown = -1,
    Float = 0,
    Float16 = 1,
    Color = 2,
    Byte = 3,
    UInt32 = 4,
}

/// From [UnityPy](https://github.com/K0lb3/UnityPy/blob/master/UnityPy/classes/Mesh.py)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromPrimitive, ToPrimitive)]
pub enum VertexFormat2017 {
    #[default]
    Unknown = -1,
    Float = 0,
    Float16 = 1,
    Color = 2,
    UNorm8 = 3,
    SNorm8 = 4,
    UNorm16 = 5,
    SNorm16 = 6,
    UInt8 = 7,
    SInt8 = 8,
    UInt16 = 9,
    SInt16 = 10,
    UInt32 = 11,
    SInt32 = 12,
}

/// From [UnityPy](https://github.com/K0lb3/UnityPy/blob/master/UnityPy/classes/Mesh.py)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromPrimitive, ToPrimitive)]
pub enum VertexFormat {
    #[default]
    Unknown = -1,
    Float = 0,
    Float16 = 1,
    UNorm8 = 2,
    SNorm8 = 3,
    UNorm16 = 4,
    SNorm16 = 5,
    UInt8 = 6,
    SInt8 = 7,
    UInt16 = 8,
    SInt16 = 9,
    UInt32 = 10,
    SInt32 = 11,
}

impl VertexFormat {
    pub fn size(&self) -> Result<usize, Error> {
        match self {
            Self::UNorm8 | Self::SNorm8 | Self::UInt8 | Self::SInt8 => Ok(1usize),
            Self::Float16 | Self::UNorm16 | Self::SNorm16 | Self::UInt16 | Self::SInt16 => {
                Ok(2usize)
            }
            Self::Float | Self::UInt32 | Self::SInt32 => Ok(4usize),
            _ => Err(Error::UnknownVertexFormat {
                format: u8::MAX,
                backtrace: Backtrace::capture(),
            }),
        }
    }

    pub fn from_raw(format: u8, version: &Version) -> Result<Self, Error> {
        let err = Err(Error::UnknownVertexFormat {
            format,
            backtrace: Backtrace::capture(),
        });

        match version.major {
            v if v < 2017 => match format {
                f if f == VertexChannelFormat::Float as u8 => Ok(Self::Float),
                f if f == VertexChannelFormat::Color as u8 => Ok(Self::UNorm8),
                f if f == VertexChannelFormat::Byte as u8 => Ok(Self::UInt8),
                f if f == VertexChannelFormat::UInt32 as u8 => Ok(Self::UInt32),
                _ => err,
            },
            v if (2017..2019).contains(&v) => match format {
                f if f == VertexFormat2017::Float as u8 => Ok(Self::Float),
                f if f == VertexFormat2017::Float16 as u8 => Ok(Self::Float16),
                f if f == VertexFormat2017::Color as u8 => Ok(Self::UNorm8),
                f if f == VertexFormat2017::SNorm8 as u8 => Ok(Self::SNorm8),
                f if f == VertexFormat2017::UNorm16 as u8 => Ok(Self::UNorm16),
                f if f == VertexFormat2017::SNorm16 as u8 => Ok(Self::SNorm16),
                f if f == VertexFormat2017::UInt8 as u8 => Ok(Self::UInt8),
                f if f == VertexFormat2017::UInt16 as u8 => Ok(Self::UInt16),
                f if f == VertexFormat2017::SInt16 as u8 => Ok(Self::SInt16),
                f if f == VertexFormat2017::UInt32 as u8 => Ok(Self::UInt32),
                f if f == VertexFormat2017::SInt32 as u8 => Ok(Self::SInt32),
                _ => err,
            },
            _ => err,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VertexData {
    pub current_channels: u32,
    pub vertex_count: u32,
    pub channels_size: u32,
    pub channels: Vec<ChannelInfo>,
    pub streams: Vec<StreamInfo>,
    pub data_size: Vec<u8>,

    pub big_endian: bool,
    pub unity_version: Version,
}

impl VertexData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut vertex_data = Self::new();
        vertex_data.big_endian = class_info.big_endian;
        vertex_data.unity_version = class_info.unity_version.clone();

        if class_info.unity_version < Version::from_str("2018.0.0").unwrap() {
            vertex_data.current_channels = reader.read_u32_by(class_info.big_endian)?;
        }

        vertex_data.vertex_count = reader.read_u32_by(class_info.big_endian)?;

        if class_info.unity_version >= Version::from_str("4.0.0").unwrap() {
            let len = reader.read_u32_by(class_info.big_endian)?;
            for _ in 0u32..len {
                let channel_info = ChannelInfo::read(reader)?;
                vertex_data.channels.push(channel_info);
            }
        }

        if class_info.unity_version < Version::from_str("5.0.0").unwrap() {
            let len = match class_info.unity_version < Version::from_str("4.0.0").unwrap() {
                true => 4u32,
                false => reader.read_u32_by(class_info.big_endian)?,
            };

            for _ in 0u32..len {
                let stream_info = StreamInfo::read(reader, class_info)?;
                vertex_data.streams.push(stream_info);
            }

            if class_info.unity_version < Version::from_str("4.0.0").unwrap() {
                todo!() // TODO: get channels
            }
        } else {
            let stream_count = vertex_data.channels.iter().map(|c| c.stream).max();
            let stream_count = stream_count.unwrap_or(0u8) + 1u8;

            let mut offset = 0u32;
            for s in 0u8..stream_count {
                let mut channel_mask = 0u32;
                let mut stride = 0u32;
                for (i, channel) in vertex_data.channels.iter().enumerate() {
                    if channel.stream == s && channel.dimension > 0 {
                        channel_mask |= 1u32 << i;
                        stride += u32::from(channel.dimension)
                            * u32::try_from(
                                VertexFormat::from_raw(channel.format, &class_info.unity_version)?
                                    .size()?,
                            )?
                    }
                }

                let stream_info = StreamInfo {
                    channel_mask,
                    offset,
                    stride,
                    ..Default::default()
                };
                vertex_data.streams.push(stream_info);

                offset += vertex_data.vertex_count * stride;
                offset = (offset + 0xf) & (!0xf);
            }
        }

        vertex_data.data_size = reader.read_u8_vec_by(class_info.big_endian)?;
        reader.seek_align(4)?;

        Ok(vertex_data)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        if self.unity_version < Version::from_str("2018.0.0").unwrap() {
            writer.write_u32_by(self.current_channels, self.big_endian)?;
        }
        writer.write_u32_by(self.vertex_count, self.big_endian)?;
        if self.unity_version >= Version::from_str("4.0.0").unwrap() {
            writer.write_u32_by(u32::try_from(self.channels.len())?, self.big_endian)?;
            for c in self.channels.iter() {
                c.save(writer)?;
            }
        }

        if self.unity_version >= Version::from_str("4.0.0").unwrap()
            && self.unity_version < Version::from_str("5.0.0").unwrap()
        {
            writer.write_u32_by(u32::try_from(self.streams.len())?, self.big_endian)?;
            for s in self.streams.iter() {
                s.save(writer)?;
            }
        }

        writer.write_u32_by(u32::try_from(self.data_size.len())?, self.big_endian)?;
        writer.write_all(&self.data_size)?;

        Ok(())
    }
}

impl Display for VertexData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        if self.unity_version < Version::from_str("2018.0.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Current channels:   {}",
                "",
                self.current_channels,
                indent = indent
            )?;
        }
        writeln!(
            f,
            "{:indent$}Vertex Count:       {}",
            "",
            self.vertex_count,
            indent = indent
        )?;
        if self.unity_version >= Version::from_str("4.0.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Channel info count: {}",
                "",
                self.channels.len(),
                indent = indent
            )?;
            writeln!(f, "{:indent$}Channel infos:", "", indent = indent)?;
            for (i, c) in self.channels.iter().enumerate() {
                writeln!(f, "{:indent$}Channel info {}", "", i, indent = indent + 4)?;
                write!(f, "{:indent$}", c, indent = indent + 8)?;
            }
        }

        if self.unity_version >= Version::from_str("4.0.0").unwrap()
            && self.unity_version < Version::from_str("5.0.0").unwrap()
        {
            writeln!(
                f,
                "{:indent$}Stream info count:  {}",
                "",
                self.streams.len(),
                indent = indent
            )?;
            writeln!(f, "{:indent$}Stream infos:", "", indent = indent)?;
            for (i, s) in self.streams.iter().enumerate() {
                writeln!(f, "{:indent$}Stream info {}", "", i, indent = indent + 4)?;
                write!(f, "{:indent$}", s, indent = indent + 8)?;
            }
        }

        writeln!(
            f,
            "{:indent$}Data size:          {} bytes",
            "",
            self.data_size.len(),
            indent = indent
        )?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BoneWeights4 {
    pub weight: [f32; 4],
    pub bone_index: [u32; 4],
}

impl BoneWeights4 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, endian: bool) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut bone_weights = Self::new();

        for w in bone_weights.weight.iter_mut() {
            *w = reader.read_f32_by(endian)?;
        }
        for i in bone_weights.bone_index.iter_mut() {
            *i = reader.read_u32_by(endian)?;
        }

        Ok(bone_weights)
    }

    pub fn save<W>(&self, writer: &mut W, endian: bool) -> Result<(), Error>
    where
        W: Write,
    {
        for w in self.weight.iter() {
            writer.write_f32_by(*w, endian)?;
        }
        for i in self.bone_index.iter() {
            writer.write_u32_by(*i, endian)?;
        }

        Ok(())
    }
}

impl Display for BoneWeights4 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Weight: {:?}",
            "",
            self.weight,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Weight: {:?}",
            "",
            self.bone_index,
            indent = indent
        )?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromPrimitive, ToPrimitive)]
pub enum GfxPrimitiveType {
    #[default]
    Unknown = -1,

    Triangles = 0,
    TriangleStrip = 1,
    Quads = 2,
    Lines = 3,
    LineStrip = 4,
    Points = 5,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SubMesh {
    pub first_byte: u32,
    pub index_count: u32,
    pub topology: GfxPrimitiveType,
    pub triangle_count: u32,
    pub base_vertex: u32,
    pub first_vertex: u32,
    pub vertex_count: u32,
    pub local_aabb: AABB,

    pub(crate) big_endian: bool,
    pub(crate) unity_version: Version,
}

impl SubMesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut sub_mesh = Self::new();
        sub_mesh.big_endian = class_info.big_endian;
        sub_mesh.unity_version = class_info.unity_version.clone();

        sub_mesh.first_byte = reader.read_u32_by(class_info.big_endian)?;
        sub_mesh.index_count = reader.read_u32_by(class_info.big_endian)?;

        let topology_raw = reader.read_u32_by(class_info.big_endian)?;
        sub_mesh.topology = match GfxPrimitiveType::from_u32(topology_raw) {
            Some(t) => t,
            None => {
                log::warn!("unknown GfxPrimitiveType: {}, using Unknown", topology_raw);
                GfxPrimitiveType::Unknown
            }
        };

        if class_info.unity_version < Version::from_str("4.0.0").unwrap() {
            sub_mesh.triangle_count = reader.read_u32_by(class_info.big_endian)?;
        }

        if class_info.unity_version >= Version::from_str("2017.3.0").unwrap() {
            sub_mesh.base_vertex = reader.read_u32_by(class_info.big_endian)?;
        }

        if class_info.unity_version >= Version::from_str("3.0.0").unwrap() {
            sub_mesh.first_vertex = reader.read_u32_by(class_info.big_endian)?;
            sub_mesh.vertex_count = reader.read_u32_by(class_info.big_endian)?;
            sub_mesh.local_aabb = AABB::read(reader, class_info.big_endian)?;
        }

        Ok(sub_mesh)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        writer.write_u32_by(self.first_byte, self.big_endian)?;
        writer.write_u32_by(self.index_count, self.big_endian)?;
        writer.write_u32_by(
            ToPrimitive::to_u32(&self.topology).unwrap(),
            self.big_endian,
        )?;

        if self.unity_version < Version::from_str("4.0.0").unwrap() {
            writer.write_u32_by(self.triangle_count, self.big_endian)?;
        }

        if self.unity_version >= Version::from_str("2017.3.0").unwrap() {
            writer.write_u32_by(self.base_vertex, self.big_endian)?;
        }

        if self.unity_version >= Version::from_str("3.0.0").unwrap() {
            writer.write_u32_by(self.first_vertex, self.big_endian)?;
            writer.write_u32_by(self.vertex_count, self.big_endian)?;
            self.local_aabb.save(writer)?;
        }

        Ok(())
    }
}

impl Display for SubMesh {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}First byte:     {}",
            "",
            self.first_byte,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Index count:    {}",
            "",
            self.index_count,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Topology:       {:?}",
            "",
            self.topology,
            indent = indent
        )?;

        if self.unity_version < Version::from_str("4.0.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Triangle count: {}",
                "",
                self.triangle_count,
                indent = indent
            )?;
        }

        if self.unity_version >= Version::from_str("2017.3.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Base vertex:    {}",
                "",
                self.base_vertex,
                indent = indent
            )?;
        }

        if self.unity_version >= Version::from_str("3.0.0").unwrap() {
            writeln!(
                f,
                "{:indent$}First vertex:   {}",
                "",
                self.first_vertex,
                indent = indent
            )?;
            writeln!(
                f,
                "{:indent$}Vertex count:   {}",
                "",
                self.vertex_count,
                indent = indent
            )?;
            writeln!(f, "{:indent$}Local AABB:", "", indent = indent)?;
            write!(f, "{:indent$}", self.local_aabb, indent = indent + 4)?;
        }

        Ok(())
    }
}
