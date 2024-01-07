use std::error::Error;
use std::str::FromStr;

use byteorder::ByteOrder;
use rabex::files::SerializedFile;
use rabex::objects::classes::{ChannelInfo, StreamInfo, SubMesh, Vector3f, VertexData, AABB};
use rabex::read_ext::ReadSeekUrexExt;

use crate::utils::ReadUnityTypeExt;
use crate::version::Version;

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
        isTriStrip: match Version::from_str("3.4.0").unwrap() <= unity_version
            && unity_version <= Version::from_str("3.5.7").unwrap()
        {
            true => Some(reader.read_u32::<E>()?),
            false => None,
        },
        topology: match Version::from_str("4.0.0").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(reader.read_i32::<E>()?),
            false => None,
        },
        triangleCount: match Version::from_str("3.4.0").unwrap() <= unity_version
            && unity_version <= Version::from_str("3.5.7").unwrap()
        {
            true => Some(reader.read_u32::<E>()?),
            false => None,
        },
        baseVertex: match Version::from_str("2017.3.0b1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(reader.read_u32::<E>()?),
            false => None,
        },
        firstVertex: match Version::from_str("3.0.0").unwrap() <= unity_version {
            true => reader.read_u32::<E>()?,
            false => u32::default(),
        },
        vertexCount: match Version::from_str("3.0.0").unwrap() <= unity_version {
            true => reader.read_u32::<E>()?,
            false => u32::default(),
        },
        localAABB: match Version::from_str("3.0.0").unwrap() <= unity_version {
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
        m_CurrentChannels: if Version::from_str("3.5.0").unwrap() <= unity_version
            && unity_version <= Version::from_str("5.5.6f1").unwrap()
        {
            Some(reader.read_u32::<E>()? as i64)
        } else if Version::from_str("5.6.0b1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2017.4.40f1").unwrap()
        {
            Some(reader.read_i32::<E>()? as i64)
        } else {
            None
        },
        m_VertexCount: reader.read_u32::<E>()?,
        m_Channels: match Version::from_str("4.0.0").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
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
        m_Streams_0_: match Version::from_str("3.5.0").unwrap() <= unity_version
            && unity_version <= Version::from_str("3.5.7").unwrap()
        {
            true => Some(construct_stream_info::<_, E>(reader, serialized_file)?),
            false => None,
        },
        m_Streams_1_: match Version::from_str("3.5.0").unwrap() <= unity_version
            && unity_version <= Version::from_str("3.5.7").unwrap()
        {
            true => Some(construct_stream_info::<_, E>(reader, serialized_file)?),
            false => None,
        },
        m_Streams_2_: match Version::from_str("3.5.0").unwrap() <= unity_version
            && unity_version <= Version::from_str("3.5.7").unwrap()
        {
            true => Some(construct_stream_info::<_, E>(reader, serialized_file)?),
            false => None,
        },
        m_Streams_3_: match Version::from_str("3.5.0").unwrap() <= unity_version
            && unity_version <= Version::from_str("3.5.7").unwrap()
        {
            true => Some(construct_stream_info::<_, E>(reader, serialized_file)?),
            false => None,
        },
        m_Streams: match Version::from_str("4.0.0").unwrap() <= unity_version
            && unity_version <= Version::from_str("4.7.2").unwrap()
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
        stride: match unity_version < Version::from_str("4.0.0").unwrap() {
            true => reader.read_u32::<E>()?,
            false => reader.read_u8()? as u32,
        },
        align: match unity_version < Version::from_str("4.0.0").unwrap() {
            true => Some(reader.read_u32::<E>()?),
            false => None,
        },
        dividerOp: match Version::from_str("4.0.0").unwrap() <= unity_version {
            true => Some(reader.read_u8()?),
            false => None,
        },
        frequency: match Version::from_str("4.0.0").unwrap() <= unity_version {
            true => Some(reader.read_u16::<E>()?),
            false => None,
        },
    })
}
