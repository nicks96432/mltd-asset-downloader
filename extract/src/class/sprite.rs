use std::error::Error;
use std::io::Cursor;
use std::str::FromStr;

use byteorder::{ByteOrder, ReadBytesExt};
use rabex::files::SerializedFile;
use rabex::objects::classes::{
    SecondarySpriteTexture, Sprite, SpriteBone, SpriteRenderData, SpriteVertex,
};
use rabex::read_ext::{ReadSeekUrexExt, ReadUrexExt};

use crate::utils::{ReadAlignedExt, ReadUnityTypeExt};
use crate::version::Version;

use super::asset_bundle::construct_p_ptr;
use super::mesh::{construct_sub_mesh, construct_vertex_data};

pub fn construct_sprite<E>(
    data: &[u8],
    serialized_file: &SerializedFile,
) -> Result<Sprite, Box<dyn Error>>
where
    E: ByteOrder,
{
    let mut reader = Cursor::new(data);
    let unity_version = Version::from_str(serialized_file.m_UnityVersion.as_ref().unwrap())?;

    Ok(Sprite {
        m_Name: reader.read_aligned_string::<E>()?,
        m_Rect: reader.read_rectf::<E>()?,
        m_Offset: reader.read_vector_2f::<E>()?,
        m_Border: match Version::from_str("4.5.0").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(reader.read_vector_4f::<E>()?),
            false => None,
        },
        m_PixelsToUnits: reader.read_f32::<E>()?,
        m_Pivot: match Version::from_str("5.4.2f2").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(reader.read_vector_2f::<E>()?),
            false => None,
        },
        m_Extrude: reader.read_u32::<E>()?,
        m_IsPolygon: match Version::from_str("5.3.0f1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some({
                let is_polygon = reader.read_bool()?;
                reader.align4()?;

                is_polygon
            }),
            false => None,
        },
        m_RenderDataKey: match Version::from_str("2017.1.0b1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some((reader.read_guid::<E>()?, reader.read_i64::<E>()?)),
            false => None,
        },
        m_AtlasTags: match Version::from_str("2017.1.0b1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => {
                let atlas_tags_len = reader.read_array_len::<E>()?;
                let mut atlas_tags = Vec::with_capacity(atlas_tags_len);

                for _ in 0..atlas_tags_len {
                    atlas_tags.push(reader.read_aligned_string::<E>()?);
                }

                Some(atlas_tags)
            }
            false => None,
        },
        m_SpriteAtlas: match Version::from_str("2017.1.0b1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(construct_p_ptr::<_, E>(&mut reader, serialized_file)?),
            false => None,
        },
        m_RD: construct_sprite_render_data::<_, E>(&mut reader, serialized_file)?,
        m_PhysicsShape: match Version::from_str("2017.1.0b1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => {
                let physics_shape_len = reader.read_array_len::<E>()?;
                let mut physics_shape = Vec::with_capacity(physics_shape_len);

                for _ in 0..physics_shape_len {
                    let vec_len = reader.read_array_len::<E>()?;
                    let mut vec = Vec::with_capacity(vec_len);

                    for _ in 0..vec_len {
                        vec.push(reader.read_vector_2f::<E>()?);
                    }

                    physics_shape.push(vec);
                }

                Some(physics_shape)
            }
            false => None,
        },
        m_Bones: match Version::from_str("2017.1.0b1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => {
                let bones_len = reader.read_array_len::<E>()?;
                let mut bones = Vec::with_capacity(bones_len);

                for _ in 0..bones_len {
                    bones.push(SpriteBone {
                        name: reader.read_aligned_string::<E>()?,
                        guid: match unity_version <= Version::from_str("2021.1.0b1").unwrap() {
                            true => Some(reader.read_aligned_string::<E>()?),
                            false => None,
                        },
                        position: reader.read_vector_3f::<E>()?,
                        rotation: reader.read_quaternionf::<E>()?,
                        length: reader.read_f32::<E>()?,
                        parentId: reader.read_i32::<E>()?,
                        color: match Version::from_str("2021.1.0b1").unwrap() <= unity_version
                            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
                        {
                            true => Some(reader.read_color_rgba_uint()?),
                            false => None,
                        },
                    });
                }

                Some(bones)
            }
            false => None,
        },
    })
}

pub fn construct_sprite_render_data<R, E>(
    reader: &mut R,
    serialized_file: &SerializedFile,
) -> Result<SpriteRenderData, Box<dyn Error>>
where
    R: ReadSeekUrexExt,
    E: ByteOrder,
{
    let unity_version = Version::from_str(serialized_file.m_UnityVersion.as_ref().unwrap())?;

    Ok(SpriteRenderData {
        texture: construct_p_ptr::<_, E>(reader, serialized_file)?,
        alphaTexture: match Version::from_str("5.2.0f2").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(construct_p_ptr::<_, E>(reader, serialized_file)?),
            false => None,
        },
        secondaryTextures: match Version::from_str("2019.1.0b1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => {
                let secondary_textures_len = reader.read_array_len::<E>()?;
                let mut secondary_textures = Vec::with_capacity(secondary_textures_len);

                for _ in 0..secondary_textures_len {
                    secondary_textures.push(SecondarySpriteTexture {
                        texture: construct_p_ptr::<_, E>(reader, serialized_file)?,
                        name: reader.read_aligned_string::<E>()?,
                    });
                }

                Some(secondary_textures)
            }
            false => None,
        },
        vertices: match Version::from_str("4.3.0").unwrap() <= unity_version
            && unity_version <= Version::from_str("5.5.6f1").unwrap()
        {
            true => {
                let vertices_len = reader.read_array_len::<E>()?;
                let mut vertices = Vec::with_capacity(vertices_len);

                for _ in 0..vertices_len {
                    vertices.push(SpriteVertex {
                        pos: reader.read_vector_3f::<E>()?,
                        uv: match Version::from_str("4.3.0").unwrap() <= unity_version
                            && unity_version <= Version::from_str("5.5.6f1").unwrap()
                        {
                            true => Some(reader.read_vector_2f::<E>()?),
                            false => None,
                        },
                    });
                }

                Some(vertices)
            }
            false => None,
        },
        indices: match Version::from_str("4.3.0").unwrap() <= unity_version
            && unity_version <= Version::from_str("5.5.6f1").unwrap()
        {
            true => {
                let indices_len = reader.read_array_len::<E>()?;
                let mut indices = Vec::with_capacity(indices_len);

                for _ in 0..indices_len {
                    indices.push(reader.read_u16::<E>()?);
                }

                reader.align4()?;
                Some(indices)
            }
            false => None,
        },
        m_SubMeshes: match Version::from_str("5.6.0b1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => {
                let sub_meshes_len = reader.read_array_len::<E>()?;
                let mut sub_meshes = Vec::with_capacity(sub_meshes_len);

                for _ in 0..sub_meshes_len {
                    sub_meshes.push(construct_sub_mesh::<_, E>(reader, serialized_file)?);
                }

                Some(sub_meshes)
            }
            false => None,
        },
        m_IndexBuffer: match Version::from_str("5.6.0b1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => {
                let index_buffer = reader.read_bytes::<E>()?;
                reader.align4()?;

                Some(index_buffer)
            }
            false => None,
        },
        m_VertexData: match Version::from_str("5.6.0b1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(construct_vertex_data::<_, E>(reader, serialized_file)?),
            false => None,
        },
        m_Bindpose: match Version::from_str("2018.1.0b2").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => {
                let bindpose_len = reader.read_array_len::<E>()?;
                let mut bindpose = Vec::with_capacity(bindpose_len);

                for _ in 0..bindpose_len {
                    bindpose.push(reader.read_matrix_4x4f::<E>()?);
                }

                Some(bindpose)
            }
            false => None,
        },
        m_SourceSkin: match Version::from_str("2018.1.0b2").unwrap() <= unity_version
            && unity_version <= Version::from_str("2018.1.9b2").unwrap()
        {
            true => {
                let source_skin_len = reader.read_array_len::<E>()?;
                let mut source_skin = Vec::with_capacity(source_skin_len);

                for _ in 0..source_skin_len {
                    source_skin.push(reader.read_bone_weight4::<E>()?);
                }

                Some(source_skin)
            }
            false => None,
        },
        textureRect: reader.read_rectf::<E>()?,
        textureRectOffset: reader.read_vector_2f::<E>()?,
        atlasRectOffset: match Version::from_str("5.4.6f1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2018.1.9b2").unwrap()
        {
            true => Some(reader.read_vector_2f::<E>()?),
            false => None,
        },
        settingsRaw: reader.read_u32::<E>()?,
        uvTransform: match Version::from_str("4.5.0").unwrap() <= unity_version
            && unity_version <= Version::from_str("2018.1.9b2").unwrap()
        {
            true => Some(reader.read_vector_4f::<E>()?),
            false => None,
        },
        downscaleMultiplier: match Version::from_str("2017.1.0b1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2018.1.9b2").unwrap()
        {
            true => Some(reader.read_f32::<E>()?),
            false => None,
        },
    })
}
