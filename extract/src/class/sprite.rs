use std::error::Error;
use std::io::Cursor;
use std::mem::size_of_val;
use std::slice::from_raw_parts;
use std::str::FromStr;

use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use rabex::files::SerializedFile;
use rabex::objects::classes::{
    SecondarySpriteTexture, Sprite, SpriteBone, SpriteRenderData, SpriteVertex,
};
use rabex::objects::PPtr;
use rabex::read_ext::{ReadSeekUrexExt, ReadUrexExt};

use crate::utils::{ReadAlignedExt, ReadUnityTypeExt};
use crate::version::*;

use super::asset_bundle::_construct_p_ptr;
use super::mesh::{construct_sub_mesh, construct_vertex_data};

pub(super) fn _construct_sprite<E>(
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
        m_Border: match UNITY_VERSION_4_5_0 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => Some(reader.read_vector_4f::<E>()?),
            false => None,
        },
        m_PixelsToUnits: reader.read_f32::<E>()?,
        m_Pivot: match UNITY_VERSION_5_4_2_F2 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => Some(reader.read_vector_2f::<E>()?),
            false => None,
        },
        m_Extrude: reader.read_u32::<E>()?,
        m_IsPolygon: match UNITY_VERSION_5_3_0_F1 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => Some({
                let is_polygon = reader.read_bool()?;
                reader.align4()?;

                is_polygon
            }),
            false => None,
        },
        m_RenderDataKey: match UNITY_VERSION_2017_1_0_B1 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => Some((reader.read_guid::<E>()?, reader.read_i64::<E>()?)),
            false => None,
        },
        m_AtlasTags: match UNITY_VERSION_2017_1_0_B1 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
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
        m_SpriteAtlas: match UNITY_VERSION_2017_1_0_B1 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => Some(_construct_p_ptr::<_, E>(&mut reader, serialized_file)?),
            false => None,
        },
        m_RD: construct_sprite_render_data::<_, E>(&mut reader, serialized_file)?,
        m_PhysicsShape: match UNITY_VERSION_2017_1_0_B1 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
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
        m_Bones: match UNITY_VERSION_2017_1_0_B1 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => {
                let bones_len = reader.read_array_len::<E>()?;
                let mut bones = Vec::with_capacity(bones_len);

                for _ in 0..bones_len {
                    bones.push(SpriteBone {
                        name: reader.read_aligned_string::<E>()?,
                        guid: match unity_version <= UNITY_VERSION_2021_1_0_B1 {
                            true => Some(reader.read_aligned_string::<E>()?),
                            false => None,
                        },
                        position: reader.read_vector_3f::<E>()?,
                        rotation: reader.read_quaternionf::<E>()?,
                        length: reader.read_f32::<E>()?,
                        parentId: reader.read_i32::<E>()?,
                        color: match UNITY_VERSION_2021_1_0_B1 <= unity_version
                            && unity_version <= UNITY_VERSION_2022_3_2_F1
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

pub fn construct_sprite(
    data: &[u8],
    serialized_file: &SerializedFile,
) -> Result<Sprite, Box<dyn Error>> {
    let big_endian = unsafe {
        from_raw_parts(
            (&serialized_file.m_Header as *const _) as *const u8,
            size_of_val(&serialized_file.m_Header),
        )
    }[0x20]
        > 0;

    match big_endian {
        true => _construct_sprite::<BigEndian>(data, serialized_file),
        false => _construct_sprite::<LittleEndian>(data, serialized_file),
    }
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
        texture: match UNITY_VERSION_4_3_0 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => _construct_p_ptr::<_, E>(reader, serialized_file)?,
            false => PPtr { m_FileID: i64::default(), m_PathID: i64::default() },
        },
        alphaTexture: match UNITY_VERSION_5_2_0_F2 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => Some(_construct_p_ptr::<_, E>(reader, serialized_file)?),
            false => None,
        },
        secondaryTextures: match UNITY_VERSION_2019_1_0_B1 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => {
                let secondary_textures_len = reader.read_array_len::<E>()?;
                let mut secondary_textures = Vec::with_capacity(secondary_textures_len);

                for _ in 0..secondary_textures_len {
                    secondary_textures.push(SecondarySpriteTexture {
                        texture: _construct_p_ptr::<_, E>(reader, serialized_file)?,
                        name: reader.read_aligned_string::<E>()?,
                    });
                }

                Some(secondary_textures)
            }
            false => None,
        },
        vertices: match UNITY_VERSION_4_3_0 <= unity_version
            && unity_version <= UNITY_VERSION_5_5_6_F1
        {
            true => {
                let vertices_len = reader.read_array_len::<E>()?;
                let mut vertices = Vec::with_capacity(vertices_len);

                for _ in 0..vertices_len {
                    vertices.push(SpriteVertex {
                        pos: reader.read_vector_3f::<E>()?,
                        uv: match UNITY_VERSION_4_3_0 <= unity_version
                            && unity_version <= UNITY_VERSION_5_5_6_F1
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
        indices: match UNITY_VERSION_4_3_0 <= unity_version
            && unity_version <= UNITY_VERSION_5_5_6_F1
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
        m_SubMeshes: match UNITY_VERSION_5_6_0_B1 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
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
        m_IndexBuffer: match UNITY_VERSION_5_6_0_B1 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => {
                let index_buffer = reader.read_bytes::<E>()?;
                reader.align4()?;

                Some(index_buffer)
            }
            false => None,
        },
        m_VertexData: match UNITY_VERSION_5_6_0_B1 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => Some(construct_vertex_data::<_, E>(reader, serialized_file)?),
            false => None,
        },
        m_Bindpose: match UNITY_VERSION_2018_1_0_B2 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
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
        m_SourceSkin: match UNITY_VERSION_2018_1_0_B2 <= unity_version
            && unity_version <= UNITY_VERSION_2018_1_9_F2
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
        atlasRectOffset: match UNITY_VERSION_5_4_6_F1 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => Some(reader.read_vector_2f::<E>()?),
            false => None,
        },
        settingsRaw: reader.read_u32::<E>()?,
        uvTransform: match UNITY_VERSION_4_5_0 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => Some(reader.read_vector_4f::<E>()?),
            false => None,
        },
        downscaleMultiplier: match UNITY_VERSION_2017_1_0_B1 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => Some(reader.read_f32::<E>()?),
            false => None,
        },
    })
}
