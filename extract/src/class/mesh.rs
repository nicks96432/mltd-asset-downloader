use std::error::Error;
use std::str::FromStr;

use byteorder::ByteOrder;
use rabex::files::SerializedFile;
use rabex::objects::classes::{ChannelInfo, StreamInfo, SubMesh, Vector3f, VertexData, AABB};
use rabex::read_ext::ReadSeekUrexExt;

use crate::utils::ReadUnityTypeExt;
use crate::version::*;

pub fn construct_sub_mesh<R, E>(
    reader: &mut R,
    serialized_file: &SerializedFile,
) -> Result<SubMesh, Box<dyn Error>>
where
    R: ReadSeekUrexExt,
    E: ByteOrder,
{
    let unity_version = Version::from_str(serialized_file.m_UnityVersion.as_ref().unwrap())?;

    Ok(SubMesh {
        firstByte: reader.read_u32::<E>()?,
        indexCount: reader.read_u32::<E>()?,
        isTriStrip: match UNITY_VERSION_3_4_0 <= unity_version
            && unity_version <= UNITY_VERSION_3_5_7
        {
            true => Some(reader.read_u32::<E>()?),
            false => None,
        },
        topology: match UNITY_VERSION_4_0_0 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => Some(reader.read_i32::<E>()?),
            false => None,
        },
        triangleCount: match UNITY_VERSION_3_4_0 <= unity_version
            && unity_version <= UNITY_VERSION_3_5_7
        {
            true => Some(reader.read_u32::<E>()?),
            false => None,
        },
        baseVertex: match UNITY_VERSION_2017_3_0_B1 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => Some(reader.read_u32::<E>()?),
            false => None,
        },
        firstVertex: match UNITY_VERSION_3_0_0 <= unity_version {
            true => reader.read_u32::<E>()?,
            false => u32::default(),
        },
        vertexCount: match UNITY_VERSION_3_0_0 <= unity_version {
            true => reader.read_u32::<E>()?,
            false => u32::default(),
        },
        localAABB: match UNITY_VERSION_3_0_0 <= unity_version {
            true => AABB {
                m_Center: reader.read_vector_3f::<E>()?,
                m_Extent: reader.read_vector_3f::<E>()?,
            },
            false => AABB {
                m_Center: Vector3f { x: f32::default(), y: f32::default(), z: f32::default() },
                m_Extent: Vector3f { x: f32::default(), y: f32::default(), z: f32::default() },
            },
        },
    })
}

pub fn construct_vertex_data<R, E>(
    reader: &mut R,
    serialized_file: &SerializedFile,
) -> Result<VertexData, Box<dyn Error>>
where
    R: ReadSeekUrexExt,
    E: ByteOrder,
{
    let unity_version = Version::from_str(serialized_file.m_UnityVersion.as_ref().unwrap())?;

    Ok(VertexData {
        m_CurrentChannels: if UNITY_VERSION_3_5_0 <= unity_version
            && unity_version <= UNITY_VERSION_5_5_6_F1
        {
            Some(reader.read_u32::<E>()? as i64)
        } else if UNITY_VERSION_5_6_0_B1 <= unity_version
            && unity_version <= UNITY_VERSION_2017_4_40_F1
        {
            Some(reader.read_i32::<E>()? as i64)
        } else {
            None
        },
        m_VertexCount: reader.read_u32::<E>()?,
        m_Channels: match UNITY_VERSION_4_0_0 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => {
                let channels_len = reader.read_array_len::<E>()?;
                let mut channels = Vec::with_capacity(channels_len);

                for _ in 0..channels_len {
                    channels.push(ChannelInfo {
                        stream: reader.read_u8()?,
                        offset: reader.read_u8()?,
                        format: reader.read_u8()?,
                        dimension: reader.read_u8()?,
                    });
                }

                Some(channels)
            }
            false => None,
        },
        m_Streams_0_: match UNITY_VERSION_3_5_0 <= unity_version
            && unity_version <= UNITY_VERSION_3_5_7
        {
            true => Some(construct_stream_info::<_, E>(reader, serialized_file)?),
            false => None,
        },
        m_Streams_1_: match UNITY_VERSION_3_5_0 <= unity_version
            && unity_version <= UNITY_VERSION_3_5_7
        {
            true => Some(construct_stream_info::<_, E>(reader, serialized_file)?),
            false => None,
        },
        m_Streams_2_: match UNITY_VERSION_3_5_0 <= unity_version
            && unity_version <= UNITY_VERSION_3_5_7
        {
            true => Some(construct_stream_info::<_, E>(reader, serialized_file)?),
            false => None,
        },
        m_Streams_3_: match UNITY_VERSION_3_5_0 <= unity_version
            && unity_version <= UNITY_VERSION_3_5_7
        {
            true => Some(construct_stream_info::<_, E>(reader, serialized_file)?),
            false => None,
        },
        m_Streams: match UNITY_VERSION_4_0_0 <= unity_version
            && unity_version <= UNITY_VERSION_4_7_2
        {
            true => {
                let streams_len = reader.read_array_len::<E>()?;
                let mut streams = Vec::with_capacity(streams_len);

                for _ in 0..streams_len {
                    streams.push(construct_stream_info::<_, E>(reader, serialized_file)?);
                }

                Some(streams)
            }
            false => None,
        },
        m_DataSize: {
            let data_size = reader.read_bytes::<E>()?;
            reader.align4()?;

            data_size
        },
    })
}

pub fn construct_stream_info<R, E>(
    reader: &mut R,
    serialized_file: &SerializedFile,
) -> Result<StreamInfo, Box<dyn Error>>
where
    R: ReadSeekUrexExt,
    E: ByteOrder,
{
    let unity_version = Version::from_str(serialized_file.m_UnityVersion.as_ref().unwrap())?;

    Ok(StreamInfo {
        channelMask: reader.read_u32::<E>()?,
        offset: reader.read_u32::<E>()?,
        stride: match unity_version < UNITY_VERSION_4_0_0 {
            true => reader.read_u32::<E>()?,
            false => reader.read_u8()? as u32,
        },
        align: match unity_version < UNITY_VERSION_4_0_0 {
            true => Some(reader.read_u32::<E>()?),
            false => None,
        },
        dividerOp: match UNITY_VERSION_4_0_0 <= unity_version {
            true => Some(reader.read_u8()?),
            false => None,
        },
        frequency: match UNITY_VERSION_4_0_0 <= unity_version {
            true => Some(reader.read_u16::<E>()?),
            false => None,
        },
    })
}
