use std::io::Result;

use byteorder::ByteOrder;
use rabex::objects::classes::{
    BoneWeights4, ColorRGBA, Matrix4x4f, Quaternionf, Rectf, Vector2f, Vector3f, Vector4f, GUID,
};
use rabex::read_ext::ReadSeekUrexExt;

pub trait ReadAlignedExt: ReadSeekUrexExt {
    fn read_aligned_string<E>(&mut self) -> Result<String>
    where
        E: ByteOrder,
    {
        let s = self.read_string::<E>()?;
        self.align4()?;

        Ok(s)
    }
}

impl<R> ReadAlignedExt for R where R: ReadSeekUrexExt + ?Sized {}

pub trait ReadUnityTypeExt: ReadSeekUrexExt {
    fn read_bone_weight4<E>(&mut self) -> Result<BoneWeights4>
    where
        E: ByteOrder,
    {
        Ok(BoneWeights4 {
            weight_0_: Some(self.read_f32::<E>()?),
            weight_1_: Some(self.read_f32::<E>()?),
            weight_2_: Some(self.read_f32::<E>()?),
            weight_3_: Some(self.read_f32::<E>()?),
            boneIndex_0_: Some(self.read_i32::<E>()?),
            boneIndex_1_: Some(self.read_i32::<E>()?),
            boneIndex_2_: Some(self.read_i32::<E>()?),
            boneIndex_3_: Some(self.read_i32::<E>()?),
        })
    }

    fn read_rectf<E>(&mut self) -> Result<Rectf>
    where
        E: ByteOrder,
    {
        Ok(Rectf {
            x: self.read_f32::<E>()?,
            y: self.read_f32::<E>()?,
            width: self.read_f32::<E>()?,
            height: self.read_f32::<E>()?,
        })
    }

    fn read_guid<E>(&mut self) -> Result<GUID>
    where
        E: ByteOrder,
    {
        Ok(GUID {
            data_0_: Some(self.read_u32::<E>()?),
            data_1_: Some(self.read_u32::<E>()?),
            data_2_: Some(self.read_u32::<E>()?),
            data_3_: Some(self.read_u32::<E>()?),
        })
    }

    fn read_vector_2f<E>(&mut self) -> Result<Vector2f>
    where
        E: ByteOrder,
    {
        Ok(Vector2f { x: self.read_f32::<E>()?, y: self.read_f32::<E>()? })
    }

    fn read_vector_3f<E>(&mut self) -> Result<Vector3f>
    where
        E: ByteOrder,
    {
        Ok(Vector3f {
            x: self.read_f32::<E>()?,
            y: self.read_f32::<E>()?,
            z: self.read_f32::<E>()?,
        })
    }

    fn read_quaternionf<E>(&mut self) -> Result<Quaternionf>
    where
        E: ByteOrder,
    {
        Ok(Quaternionf {
            x: self.read_f32::<E>()?,
            y: self.read_f32::<E>()?,
            z: self.read_f32::<E>()?,
            w: self.read_f32::<E>()?,
        })
    }

    fn read_color_rgba_uint(&mut self) -> Result<ColorRGBA> {
        Ok(ColorRGBA {
            r: Some(self.read_u8()? as f32 / 255.0),
            g: Some(self.read_u8()? as f32 / 255.0),
            b: Some(self.read_u8()? as f32 / 255.0),
            a: Some(self.read_u8()? as f32 / 255.0),
            rgba: None,
        })
    }

    fn read_color_rgba<E>(&mut self) -> Result<ColorRGBA>
    where
        E: ByteOrder,
    {
        Ok(ColorRGBA {
            r: Some(self.read_f32::<E>()?),
            g: Some(self.read_f32::<E>()?),
            b: Some(self.read_f32::<E>()?),
            a: Some(self.read_f32::<E>()?),
            rgba: None,
        })
    }

    fn read_vector_4f<E>(&mut self) -> Result<Vector4f>
    where
        E: ByteOrder,
    {
        Ok(Vector4f {
            x: self.read_f32::<E>()?,
            y: self.read_f32::<E>()?,
            z: self.read_f32::<E>()?,
            w: self.read_f32::<E>()?,
        })
    }

    fn read_matrix_4x4f<E>(&mut self) -> Result<Matrix4x4f>
    where
        E: ByteOrder,
    {
        Ok(Matrix4x4f {
            e00: self.read_f32::<E>()?,
            e01: self.read_f32::<E>()?,
            e02: self.read_f32::<E>()?,
            e03: self.read_f32::<E>()?,
            e10: self.read_f32::<E>()?,
            e11: self.read_f32::<E>()?,
            e12: self.read_f32::<E>()?,
            e13: self.read_f32::<E>()?,
            e20: self.read_f32::<E>()?,
            e21: self.read_f32::<E>()?,
            e22: self.read_f32::<E>()?,
            e23: self.read_f32::<E>()?,
            e30: self.read_f32::<E>()?,
            e31: self.read_f32::<E>()?,
            e32: self.read_f32::<E>()?,
            e33: self.read_f32::<E>()?,
        })
    }
}

impl<R> ReadUnityTypeExt for R where R: ReadSeekUrexExt + ?Sized {}
