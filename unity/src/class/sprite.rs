use super::{BoneWeights4, Class, NamedObject, PPtr, SubMesh, VertexData};
use crate::asset::ClassInfo;
use crate::error::Error;
use crate::traits::{
    ReadAlignedString, ReadPrimitiveExt, ReadString, ReadVecExt, SeekAlign, WriteAlign,
    WritePrimitiveExt,
};
use crate::utils::{
    bool_to_yes_no, Matrix4x4F32, RectangleF32, Vector2, Vector3, Vector4, Version,
};

use byteorder::WriteBytesExt;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;

use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, Write};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SecondaryTexture {
    pub texture: PPtr,
    pub name: String,
}

impl SecondaryTexture {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut secondary_texture = Self::new();

        secondary_texture.texture = PPtr::read(reader, class_info)?;
        secondary_texture.name = reader.read_string()?;

        Ok(secondary_texture)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        self.texture.save(writer)?;
        writer.write_all(self.name.as_bytes())?;
        writer.write_u8(0u8)?;

        Ok(())
    }
}

impl Display for SecondaryTexture {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(f, "{:indent$}Texture PPtr:", "", indent = indent)?;
        write!(f, "{:indent$}", self.texture, indent = indent + 4)?;
        writeln!(f, "{:indent$}Name: {}", "", self.name, indent = indent)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromPrimitive, ToPrimitive)]
pub enum MeshType {
    #[default]
    Unknown = -1,

    FullRect = 0,
    Tight = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromPrimitive, ToPrimitive)]
pub enum PackingMode {
    #[default]
    Unknown = -1,

    Tight = 0,
    Rectangle = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromPrimitive, ToPrimitive)]
pub enum PackingRotation {
    #[default]
    Unknown = -1,

    None = 0,
    Horizontal = 1,
    Vertical = 2,
    Rotate180 = 3,
    Rotate90 = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Settings(u32);

impl Settings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn packed(&self) -> bool {
        self.0 & 1 > 0
    }

    pub fn packing_mode(&self) -> PackingMode {
        match PackingMode::from_u32((self.0 >> 1u32) & 1u32) {
            Some(mode) => mode,
            None => PackingMode::Unknown,
        }
    }

    pub fn packing_rotation(&self) -> PackingRotation {
        match PackingRotation::from_u32((self.0 >> 2u32) & 0xfu32) {
            Some(mode) => mode,
            None => PackingRotation::Unknown,
        }
    }

    pub fn mesh_type(&self) -> MeshType {
        match MeshType::from_u32((self.0 >> 6u32) & 1u32) {
            Some(mode) => mode,
            None => MeshType::Unknown,
        }
    }
}

impl Display for Settings {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Packed?           {:?}",
            "",
            self.packed(),
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Packing mode:     {:?}",
            "",
            self.packing_mode(),
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Packing rotation: {:?}",
            "",
            self.packing_rotation(),
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Mesh type:        {:?}",
            "",
            self.mesh_type(),
            indent = indent
        )?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Vertex {
    pub pos: Vector3,
    pub uv: Vector2,

    pub(crate) big_endian: bool,
    pub(crate) unity_version: Version,
}

impl Vertex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut vertex = Self::new();
        vertex.big_endian = class_info.big_endian;
        vertex.unity_version = class_info.unity_version.clone();

        vertex.pos = Vector3::read(reader, class_info.big_endian)?;
        if class_info.unity_version >= Version::from_str("4.3.0").unwrap() {
            vertex.uv = Vector2::read(reader, class_info.big_endian)?;
        }

        Ok(vertex)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        self.pos.save(writer, self.big_endian)?;
        if self.unity_version >= Version::from_str("4.3.0").unwrap() {
            self.uv.save(writer, self.big_endian)?;
        }

        Ok(())
    }
}

impl Display for Vertex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(f, "{:indent$}Pos: {:?}", "", self.pos, indent = indent)?;
        writeln!(f, "{:indent$}UV:  {:?}", "", self.uv, indent = indent)?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RenderData {
    pub texture: PPtr,
    pub alpha_texture: PPtr,
    pub secondary_textures: Vec<SecondaryTexture>,
    pub sub_meshes: Vec<SubMesh>,
    pub index_buffer: Vec<u8>,
    pub vertex_data: VertexData,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub bindpose: Vec<Matrix4x4F32>,
    pub source_skin_size: u32,
    pub source_skin: Vec<BoneWeights4>,
    pub texture_rect: RectangleF32,
    pub texture_rect_offset: Vector2,
    pub atlas_rect_offset: Vector2,
    pub settings_raw: Settings,
    pub uv_transform: Vector4,
    pub downscale_multiplier: f32,

    pub big_endian: bool,
    pub unity_version: Version,
}

impl RenderData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut render_data = Self::new();

        render_data.texture = PPtr::read(reader, class_info)?;
        if class_info.unity_version >= Version::from_str("5.2.0").unwrap() {
            render_data.alpha_texture = PPtr::read(reader, class_info)?;
        }

        if class_info.unity_version >= Version::from_str("2019.0.0").unwrap() {
            let len = reader.read_u32_by(class_info.big_endian)?;
            for _ in 0u32..len {
                let secondary_texture = SecondaryTexture::read(reader, class_info)?;
                render_data.secondary_textures.push(secondary_texture);
            }
        }

        if class_info.unity_version >= Version::from_str("5.6.0").unwrap() {
            let size = reader.read_u32_by(class_info.big_endian)?;
            for _ in 0u32..size {
                let sub_mesh = SubMesh::read(reader, class_info)?;
                render_data.sub_meshes.push(sub_mesh);
            }

            render_data.index_buffer = reader.read_u8_vec_by(class_info.big_endian)?;
            reader.seek_align(4)?;

            render_data.vertex_data = VertexData::read(reader, class_info)?;
        } else {
            let size = reader.read_u32_by(class_info.big_endian)?;
            for _ in 0u32..size {
                let vertex = Vertex::read(reader, class_info)?;
                render_data.vertices.push(vertex);
            }

            render_data.indices = reader.read_u16_vec_by(class_info.big_endian)?;
            reader.seek_align(4)?;
        }

        if class_info.unity_version >= Version::from_str("2018.0.0").unwrap() {
            let len = reader.read_u32_by(class_info.big_endian)?;
            for _ in 0u32..len {
                let matrix = Matrix4x4F32::read(reader, class_info.big_endian)?;
                render_data.bindpose.push(matrix);
            }

            if class_info.unity_version >= Version::from_str("2018.2.0").unwrap() {
                render_data.source_skin_size = reader.read_u32_by(class_info.big_endian)?;
                render_data.source_skin = vec![BoneWeights4::read(reader, class_info.big_endian)?];
            }
        }

        render_data.texture_rect = RectangleF32::read(reader, class_info.big_endian)?;
        render_data.texture_rect_offset = Vector2::read(reader, class_info.big_endian)?;
        if class_info.unity_version >= Version::from_str("5.6.0").unwrap() {
            render_data.atlas_rect_offset = Vector2::read(reader, class_info.big_endian)?;
        }

        render_data.settings_raw = Settings(reader.read_u32_by(class_info.big_endian)?);
        if class_info.unity_version >= Version::from_str("4.5.0").unwrap() {
            render_data.uv_transform = Vector4::read(reader, class_info.big_endian)?;
        }

        if class_info.unity_version >= Version::from_str("2017.0.0").unwrap() {
            render_data.downscale_multiplier = reader.read_f32_by(class_info.big_endian)?;
        }

        Ok(render_data)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write + Seek,
    {
        self.texture.save(writer)?;

        if self.unity_version >= Version::from_str("5.2.0").unwrap() {
            self.alpha_texture.save(writer)?;
        }

        if self.unity_version >= Version::from_str("2019.0.0").unwrap() {
            writer.write_u32_by(
                u32::try_from(self.secondary_textures.len())?,
                self.big_endian,
            )?;
            for t in self.secondary_textures.iter() {
                t.save(writer)?;
            }
        }

        if self.unity_version >= Version::from_str("5.6.0").unwrap() {
            writer.write_u32_by(u32::try_from(self.sub_meshes.len())?, self.big_endian)?;
            for m in self.sub_meshes.iter() {
                m.save(writer)?;
            }

            writer.write_u32_by(u32::try_from(self.index_buffer.len())?, self.big_endian)?;
            writer.write_all(&self.index_buffer)?;
            writer.write_align(4)?;

            self.vertex_data.save(writer)?;
        } else {
            writer.write_u32_by(u32::try_from(self.vertices.len())?, self.big_endian)?;
            for v in self.vertices.iter() {
                v.save(writer)?;
            }

            writer.write_u32_by(u32::try_from(self.indices.len())?, self.big_endian)?;
            for n in self.indices.iter() {
                writer.write_u16_by(*n, self.big_endian)?;
            }

            writer.write_align(4)?;
        }

        if self.unity_version >= Version::from_str("2018.0.0").unwrap() {
            writer.write_u32_by(u32::try_from(self.bindpose.len())?, self.big_endian)?;
            for m in self.bindpose.iter() {
                m.save(writer, self.big_endian)?;
            }

            if self.unity_version <= Version::from_str("2018.2.0").unwrap() {
                writer.write_u32_by(self.source_skin_size, self.big_endian)?;
                self.source_skin[0].save(writer, self.big_endian)?;
            }
        }

        self.texture_rect.save(writer, self.big_endian)?;
        self.texture_rect_offset.save(writer, self.big_endian)?;
        if self.unity_version >= Version::from_str("5.6.0").unwrap() {
            self.atlas_rect_offset.save(writer, self.big_endian)?;
        }

        writer.write_u32_by(self.settings_raw.0, self.big_endian)?;
        if self.unity_version >= Version::from_str("4.5.0").unwrap() {
            self.uv_transform.save(writer, self.big_endian)?;
        }

        if self.unity_version >= Version::from_str("2017.0.0").unwrap() {
            writer.write_f32_by(self.downscale_multiplier, self.big_endian)?;
        }

        Ok(())
    }
}

impl Display for RenderData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(f, "{:indent$}Texture (PPtr):", "", indent = indent)?;
        write!(f, "{:indent$}", self.texture, indent = indent + 4)?;

        if self.unity_version >= Version::from_str("5.2.0").unwrap() {
            writeln!(f, "{:indent$}Alpha texture (PPtr):", "", indent = indent)?;
            write!(f, "{:indent$}", self.alpha_texture, indent = indent + 4)?;
        }

        if self.unity_version >= Version::from_str("2019.0.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Secondary texture count: {}",
                "",
                self.secondary_textures.len(),
                indent = indent
            )?;
            writeln!(f, "{:indent$}Secondary textures:", "", indent = indent)?;
            for (i, t) in self.secondary_textures.iter().enumerate() {
                writeln!(
                    f,
                    "{:indent$}Secondary texture {}:",
                    "",
                    i,
                    indent = indent + 4
                )?;
                write!(f, "{:indent$}", t, indent = indent + 8)?;
            }
        }

        if self.unity_version >= Version::from_str("5.6.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Submesh count:           {}",
                "",
                self.sub_meshes.len(),
                indent = indent
            )?;
            writeln!(f, "{:indent$}Submeshes:", "", indent = indent)?;
            for (i, m) in self.sub_meshes.iter().enumerate() {
                writeln!(f, "{:indent$}Submesh {}:", "", i, indent = indent + 4)?;
                write!(f, "{:indent$}", m, indent = indent + 8)?;
            }

            writeln!(
                f,
                "{:indent$}Index buffer:            {} bytes",
                "",
                self.index_buffer.len(),
                indent = indent
            )?;
        } else {
            writeln!(
                f,
                "{:indent$}Vertex count:            {}",
                "",
                self.vertices.len(),
                indent = indent
            )?;
            writeln!(f, "{:indent$}Vertices:", "", indent = indent)?;
            for (i, v) in self.vertices.iter().enumerate() {
                writeln!(f, "{:indent$}Vertex {}:", "", i, indent = indent + 4)?;
                write!(f, "{:indent$}", v, indent = indent + 8)?;
            }

            writeln!(
                f,
                "{:indent$}Indices:                 {} number(s)",
                "",
                self.indices.len(),
                indent = indent
            )?;
        }

        if self.unity_version >= Version::from_str("2018.0.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Bindpose matrix count:   {}",
                "",
                self.bindpose.len(),
                indent = indent
            )?;
            writeln!(f, "{:indent$}Bindpose matrices:", "", indent = indent)?;
            for (i, m) in self.bindpose.iter().enumerate() {
                writeln!(f, "{:indent$}Vertex {}:", "", i, indent = indent + 4)?;
                write!(f, "{:indent$}", m, indent = indent + 8)?;
            }

            if self.unity_version >= Version::from_str("2018.2.0").unwrap() {
                writeln!(
                    f,
                    "{:indent$}Source skin size         {}:",
                    "",
                    self.source_skin_size,
                    indent = indent
                )?;
                writeln!(f, "{:indent$}Source skin:", "", indent = indent)?;
                write!(f, "{:indent$}", self.source_skin[0], indent = indent + 4)?;
            }
        }

        writeln!(
            f,
            "{:indent$}Texture rect:            {:?}",
            "",
            self.texture_rect,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Texture rect offset:     {:?}",
            "",
            self.texture_rect_offset,
            indent = indent
        )?;
        if self.unity_version >= Version::from_str("5.6.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Atlas rect offset:       {:?}",
                "",
                self.atlas_rect_offset,
                indent = indent
            )?;
        }

        writeln!(f, "{:indent$}Settings:", "", indent = indent)?;
        write!(f, "{:indent$}", self.settings_raw, indent = indent + 4)?;
        if self.unity_version >= Version::from_str("4.5.0").unwrap() {
            writeln!(
                f,
                "{:indent$}UV transform:            {:?}",
                "",
                self.uv_transform,
                indent = indent
            )?;
        }

        if self.unity_version >= Version::from_str("2017.0.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Downscale multiplier:    {}",
                "",
                self.downscale_multiplier,
                indent = indent
            )?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sprite {
    pub named_object: NamedObject,
    pub rect: RectangleF32,
    pub offset: Vector2,
    pub border: Vector4,
    pub pixals_to_units: f32,
    pub pivot: Vector2,
    pub extrude: u32,
    pub is_polygon: bool,
    pub render_data_key: ([u8; 16], u64),
    pub atlas_tags: Vec<String>,
    pub sprite_atlas: PPtr,
    pub render_data: RenderData,

    pub big_endian: bool,
    pub unity_version: Version,
}

impl Sprite {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, class_info: &ClassInfo) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut sprite = Self::new();
        sprite.big_endian = class_info.big_endian;
        sprite.unity_version = class_info.unity_version.clone();

        sprite.named_object = NamedObject::read(reader, class_info)?;

        sprite.rect = RectangleF32::read(reader, class_info.big_endian)?;
        sprite.offset = Vector2::read(reader, class_info.big_endian)?;

        if class_info.unity_version >= Version::from_str("4.5.0").unwrap() {
            sprite.border = Vector4::read(reader, class_info.big_endian)?;
        }

        sprite.pixals_to_units = reader.read_f32_by(class_info.big_endian)?;
        if class_info.unity_version >= Version::from_str("5.4.2").unwrap()
            || class_info.unity_version >= Version::from_str("5.4.1p3").unwrap()
        {
            sprite.pivot = Vector2::read(reader, class_info.big_endian)?;
        }

        sprite.extrude = reader.read_u32_by(class_info.big_endian)?;
        if class_info.unity_version >= Version::from_str("5.3.0").unwrap() {
            sprite.is_polygon = reader.read_u32_by(class_info.big_endian)? > 0;
        }

        if class_info.unity_version >= Version::from_str("2017.0.0").unwrap() {
            reader.read_exact(&mut sprite.render_data_key.0)?;
            sprite.render_data_key.1 = reader.read_u64_by(class_info.big_endian)?;

            let len = reader.read_u32_by(class_info.big_endian)?;
            for _ in 0u32..len {
                let string = reader.read_aligned_string(class_info.big_endian, 4)?;
                sprite.atlas_tags.push(string);
            }
            sprite.sprite_atlas = PPtr::read(reader, class_info)?;
        }

        sprite.render_data = RenderData::read(reader, class_info)?;

        Ok(sprite)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write + Seek,
    {
        self.named_object.save(writer)?;

        self.rect.save(writer, self.big_endian)?;
        self.offset.save(writer, self.big_endian)?;

        if self.unity_version >= Version::from_str("4.5.0").unwrap() {
            self.border.save(writer, self.big_endian)?;
        }

        writer.write_f32_by(self.pixals_to_units, self.big_endian)?;
        if self.unity_version >= Version::from_str("5.4.2").unwrap()
            || self.unity_version >= Version::from_str("5.4.1p3").unwrap()
        {
            self.pivot.save(writer, self.big_endian)?;
        }

        writer.write_u32_by(self.extrude, self.big_endian)?;
        if self.unity_version >= Version::from_str("5.3.0").unwrap() {
            writer.write_u32_by(u32::from(self.is_polygon), self.big_endian)?;
        }

        if self.unity_version >= Version::from_str("2017.0.0").unwrap() {
            writer.write_all(&self.render_data_key.0)?;
            writer.write_u64_by(self.render_data_key.1, self.big_endian)?;

            writer.write_u32_by(u32::try_from(self.atlas_tags.len())?, self.big_endian)?;
            for string in self.atlas_tags.iter() {
                writer.write_all(string.as_bytes())?;
                writer.write_align(4)?;
            }
            self.sprite_atlas.save(writer)?;
        }

        self.render_data.save(writer)?;

        Ok(())
    }
}

impl Display for Sprite {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(
            f,
            "{:indent$}Super ({}):",
            "",
            self.named_object.name(),
            indent = indent
        )?;
        write!(f, "{:indent$}", self.named_object, indent = indent + 4)?;

        writeln!(
            f,
            "{:indent$}Rect:            {:?}",
            "",
            self.rect,
            indent = indent
        )?;
        writeln!(
            f,
            "{:indent$}Offset:          {:?}",
            "",
            self.offset,
            indent = indent
        )?;

        if self.unity_version >= Version::from_str("4.5.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Border:          {:?}",
                "",
                self.border,
                indent = indent
            )?;
        }
        writeln!(
            f,
            "{:indent$}Pixals to units: {}",
            "",
            self.pixals_to_units,
            indent = indent
        )?;

        if self.unity_version >= Version::from_str("5.4.2").unwrap()
            || self.unity_version >= Version::from_str("5.4.1p3").unwrap()
        {
            writeln!(
                f,
                "{:indent$}Pivot:           {:?}",
                "",
                self.pivot,
                indent = indent
            )?;
        }

        writeln!(
            f,
            "{:indent$}Extrude:         {}",
            "",
            self.extrude,
            indent = indent
        )?;
        if self.unity_version >= Version::from_str("5.3.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Is polygon?      {}",
                "",
                bool_to_yes_no(self.is_polygon),
                indent = indent
            )?;
        }

        if self.unity_version >= Version::from_str("2017.0.0").unwrap() {
            writeln!(
                f,
                "{:indent$}Render data key: {:?}",
                "",
                self.render_data_key,
                indent = indent
            )?;

            writeln!(
                f,
                "{:indent$}Atlas tag count: {}",
                "",
                self.atlas_tags.len(),
                indent = indent
            )?;
            writeln!(f, "{:indent$}Atlas tags:", "", indent = indent)?;
            for tag in self.atlas_tags.iter() {
                write!(f, "{:indent$}{}", "", tag, indent = indent + 4)?;
            }
        }

        writeln!(f, "{:indent$}Render data:", "", indent = indent)?;
        write!(f, "{:indent$}", self.render_data, indent = indent)?;

        Ok(())
    }
}

impl Class for Sprite {}
