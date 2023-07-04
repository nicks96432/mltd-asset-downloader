use crate::traits::ReadPrimitiveExt;
use crate::{error::Error, traits::WritePrimitiveExt};

use std::fmt::{Display, Formatter};
use std::io::{Read, Write};

/// x, y, width, height
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RectangleI32(i32, i32, i32, i32);

impl RectangleI32 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, endian: bool) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut rectangle = Self::new();

        rectangle.0 = reader.read_i32_by(endian)?;
        rectangle.1 = reader.read_i32_by(endian)?;
        rectangle.2 = reader.read_i32_by(endian)?;
        rectangle.3 = reader.read_i32_by(endian)?;

        Ok(rectangle)
    }

    pub fn save<W>(&self, writer: &mut W, endian: bool) -> Result<(), Error>
    where
        W: Write,
    {
        writer.write_i32_by(self.0, endian)?;
        writer.write_i32_by(self.1, endian)?;
        writer.write_i32_by(self.2, endian)?;
        writer.write_i32_by(self.3, endian)?;

        Ok(())
    }

    pub fn to_i32(&self) -> RectangleF32 {
        RectangleF32(self.0 as f32, self.1 as f32, self.2 as f32, self.3 as f32)
    }
}

/// x, y, width, height
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RectangleF32(f32, f32, f32, f32);

impl RectangleF32 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, endian: bool) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut rectangle = Self::new();

        rectangle.0 = reader.read_f32_by(endian)?;
        rectangle.1 = reader.read_f32_by(endian)?;
        rectangle.2 = reader.read_f32_by(endian)?;
        rectangle.3 = reader.read_f32_by(endian)?;

        Ok(rectangle)
    }

    pub fn save<W>(&self, writer: &mut W, endian: bool) -> Result<(), Error>
    where
        W: Write,
    {
        writer.write_f32_by(self.0, endian)?;
        writer.write_f32_by(self.1, endian)?;
        writer.write_f32_by(self.2, endian)?;
        writer.write_f32_by(self.3, endian)?;

        Ok(())
    }

    pub fn round(&self) -> Self {
        Self(
            self.0.round(),
            self.1.round(),
            self.2.round(),
            self.3.round(),
        )
    }

    pub fn to_i32(&self) -> RectangleI32 {
        RectangleI32(
            self.0.round() as i32,
            self.1.round() as i32,
            self.2.round() as i32,
            self.3.round() as i32,
        )
    }
}

/// x, y
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector2(f32, f32);

impl Vector2 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, endian: bool) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut vector2 = Self::new();

        vector2.0 = reader.read_f32_by(endian)?;
        vector2.1 = reader.read_f32_by(endian)?;

        Ok(vector2)
    }

    pub fn save<W>(&self, writer: &mut W, endian: bool) -> Result<(), Error>
    where
        W: Write,
    {
        writer.write_f32_by(self.0, endian)?;
        writer.write_f32_by(self.1, endian)?;

        Ok(())
    }
}

/// x, y, z
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector3(f32, f32, f32);

impl Vector3 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, endian: bool) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut vector3 = Self::new();

        vector3.0 = reader.read_f32_by(endian)?;
        vector3.1 = reader.read_f32_by(endian)?;
        vector3.2 = reader.read_f32_by(endian)?;

        Ok(vector3)
    }

    pub fn save<W>(&self, writer: &mut W, endian: bool) -> Result<(), Error>
    where
        W: Write,
    {
        writer.write_f32_by(self.0, endian)?;
        writer.write_f32_by(self.1, endian)?;
        writer.write_f32_by(self.2, endian)?;

        Ok(())
    }
}

/// x, y, z, w
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector4(f32, f32, f32, f32);

impl Vector4 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, endian: bool) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut vector4 = Self::new();

        vector4.0 = reader.read_f32_by(endian)?;
        vector4.1 = reader.read_f32_by(endian)?;
        vector4.2 = reader.read_f32_by(endian)?;
        vector4.3 = reader.read_f32_by(endian)?;

        Ok(vector4)
    }

    pub fn save<W>(&self, writer: &mut W, endian: bool) -> Result<(), Error>
    where
        W: Write,
    {
        writer.write_f32_by(self.0, endian)?;
        writer.write_f32_by(self.1, endian)?;
        writer.write_f32_by(self.2, endian)?;
        writer.write_f32_by(self.3, endian)?;

        Ok(())
    }
}

/// x, y, z, w
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Matrix4x4F32([[f32; 4]; 4]);

impl Matrix4x4F32 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R>(reader: &mut R, endian: bool) -> Result<Self, Error>
    where
        R: Read,
    {
        let mut matrix = Self::new();

        for i in 0usize..4usize {
            for j in 0usize..4usize {
                matrix.0[i][j] = reader.read_f32_by(endian)?;
            }
        }

        Ok(matrix)
    }

    pub fn save<W>(&self, writer: &mut W, endian: bool) -> Result<(), Error>
    where
        W: Write,
    {
        for i in 0usize..4usize {
            for j in 0usize..4usize {
                writer.write_f32_by(self.0[i][j], endian)?;
            }
        }

        Ok(())
    }
}

impl Display for Matrix4x4F32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        for row in self.0.iter() {
            writeln!(
                f,
                "{:indent$}{:.3} {:.3} {:.3} {:.3}",
                "", row[0], row[1], row[2], row[3]
            )?;
        }

        Ok(())
    }
}
