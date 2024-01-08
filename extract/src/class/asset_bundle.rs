use std::error::Error;
use std::io::{Cursor, Read, Seek};
use std::mem::size_of_val;
use std::slice::from_raw_parts;
use std::str::FromStr;

use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use rabex::files::SerializedFile;
use rabex::objects::classes::{AssetBundle, AssetBundleScriptInfo, AssetInfo};
use rabex::objects::PPtr;
use rabex::read_ext::ReadUrexExt;

use crate::utils::ReadAlignedExt;
use crate::version::*;

pub fn _construct_p_ptr<R, E>(
    reader: &mut R,
    serialized_file: &SerializedFile,
) -> Result<PPtr, Box<dyn Error>>
where
    R: Read + Seek,
    E: ByteOrder,
{
    // XXX: This is a hack to get the version from the serialized file header.
    let version = Cursor::new(
        &unsafe {
            from_raw_parts(
                (&serialized_file.m_Header as *const _) as *const u8,
                size_of_val(&serialized_file.m_Header),
            )
        }[0x1c..0x20],
    )
    .read_u32::<E>()?;

    Ok(PPtr {
        m_FileID: reader.read_i32::<E>()? as i64,
        m_PathID: if version < 14 {
            reader.read_i32::<E>()? as i64
        } else {
            reader.read_i64::<E>()?
        },
    })
}

pub fn construct_p_ptr<R>(
    reader: &mut R,
    serialized_file: &SerializedFile,
) -> Result<PPtr, Box<dyn Error>>
where
    R: Read + Seek,
{
    let big_endian = unsafe {
        from_raw_parts(
            (&serialized_file.m_Header as *const _) as *const u8,
            size_of_val(&serialized_file.m_Header),
        )
    }[0x20] > 0;

    match big_endian {
        true => _construct_p_ptr::<_, BigEndian>(reader, serialized_file),
        false => _construct_p_ptr::<_, LittleEndian>(reader, serialized_file),
    }
}

pub fn construct_asset_info<R>(
    reader: &mut R,
    serialized_file: &SerializedFile,
) -> Result<AssetInfo, Box<dyn Error>>
where
    R: Read + Seek,
{
    let big_endian = unsafe {
        from_raw_parts(
            (&serialized_file.m_Header as *const _) as *const u8,
            size_of_val(&serialized_file.m_Header),
        )
    }[0x20]
        > 0;

    let asset_info = match big_endian {
        true => AssetInfo {
            preloadIndex: reader.read_i32::<BigEndian>()?,
            preloadSize: reader.read_i32::<BigEndian>()?,
            asset: construct_p_ptr(reader, serialized_file)?,
        },
        false => AssetInfo {
            preloadIndex: reader.read_i32::<LittleEndian>()?,
            preloadSize: reader.read_i32::<LittleEndian>()?,
            asset: construct_p_ptr(reader, serialized_file)?,
        },
    };

    Ok(asset_info)
}

pub fn _construct_asset_bundle<E>(
    data: &[u8],
    serialized_file: &SerializedFile,
) -> Result<AssetBundle, Box<dyn Error>>
where
    E: ByteOrder,
{
    let mut reader = Cursor::new(data);
    let unity_version = Version::from_str(serialized_file.m_UnityVersion.as_ref().unwrap())?;

    Ok(AssetBundle {
        m_Name: reader.read_aligned_string::<E>()?,
        m_PreloadTable: match UNITY_VERSION_3_4_0 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => {
                let preload_table_len = reader.read_array_len::<E>()?;
                let mut preload_table = Vec::with_capacity(preload_table_len);

                for _ in 0..preload_table_len {
                    preload_table.push(_construct_p_ptr::<_, E>(&mut reader, serialized_file)?);
                }

                preload_table
            }
            false => Vec::new(),
        },
        m_Container: {
            let container_len = reader.read_array_len::<E>()?;
            let mut container = Vec::with_capacity(container_len);

            for _ in 0..container_len {
                let key = reader.read_aligned_string::<E>()?;

                let value = construct_asset_info(&mut reader, serialized_file)?;
                container.push((key, value));
            }

            container
        },
        m_MainAsset: construct_asset_info(&mut reader, serialized_file)?,
        m_ScriptCompatibility: match UNITY_VERSION_3_4_0 <= unity_version
            && unity_version <= UNITY_VERSION_4_7_2
        {
            true => {
                let script_compatibility_len = reader.read_array_len::<E>()?;
                let mut script_compatibility = Vec::with_capacity(script_compatibility_len);

                for _ in 0..script_compatibility_len {
                    script_compatibility.push(AssetBundleScriptInfo {
                        className: reader.read_aligned_string::<E>()?,
                        nameSpace: reader.read_aligned_string::<E>()?,
                        assemblyName: reader.read_aligned_string::<E>()?,
                        hash: reader.read_u32::<E>()?,
                    });
                }

                Some(script_compatibility)
            }
            false => None,
        },
        m_ClassCompatibility: match UNITY_VERSION_3_5_0 <= unity_version
            && unity_version <= UNITY_VERSION_4_7_2
        {
            true => {
                let class_compatibility_len = reader.read_array_len::<E>()?;
                let mut class_compatibility = Vec::with_capacity(class_compatibility_len);

                for _ in 0..class_compatibility_len {
                    class_compatibility.push((reader.read_i32::<E>()?, reader.read_u32::<E>()?));
                }

                Some(class_compatibility)
            }
            false => None,
        },
        m_ClassVersionMap: match UNITY_VERSION_5_4_0_F3 <= unity_version
            && unity_version <= UNITY_VERSION_5_4_6_F3
        {
            true => {
                let version_map_len = reader.read_array_len::<E>()?;
                let mut version_map = Vec::with_capacity(version_map_len);

                for _ in 0..version_map_len {
                    version_map.push((reader.read_i32::<E>()?, reader.read_i32::<E>()?));
                }

                Some(version_map)
            }
            false => None,
        },
        m_RuntimeCompatibility: match UNITY_VERSION_4_2_0 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => Some(reader.read_u32::<E>()?),
            false => None,
        },
        m_AssetBundleName: match UNITY_VERSION_5_0_0_F4 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => Some(reader.read_aligned_string::<E>()?),
            false => None,
        },
        m_Dependencies: match UNITY_VERSION_5_0_0_F4 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => {
                let dependencies_len = reader.read_array_len::<E>()?;
                let mut dependencies = Vec::with_capacity(dependencies_len);

                for _ in 0..dependencies_len {
                    dependencies.push(reader.read_aligned_string::<E>()?);
                }

                Some(dependencies)
            }
            false => None,
        },
        m_IsStreamedSceneAssetBundle: match UNITY_VERSION_5_0_0_F4 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => Some(reader.read_bool()?),
            false => None,
        },
        m_ExplicitDataLayout: match UNITY_VERSION_2017_3_0_B1 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => Some(reader.read_i32::<E>()?),
            false => None,
        },
        m_PathFlags: match UNITY_VERSION_2017_1_0_B2 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => Some(reader.read_i32::<E>()?),
            false => None,
        },
        m_SceneHashes: match UNITY_VERSION_2017_3_0_B1 <= unity_version
            && unity_version <= UNITY_VERSION_2022_3_2_F1
        {
            true => {
                let scene_hashes_len = reader.read_array_len::<E>()?;
                let mut scene_hashes = Vec::with_capacity(scene_hashes_len);

                for _ in 0..scene_hashes_len {
                    scene_hashes.push((
                        reader.read_aligned_string::<E>()?,
                        reader.read_aligned_string::<E>()?,
                    ));
                }

                Some(scene_hashes)
            }
            false => None,
        },
    })
}

pub fn construct_asset_bundle(
    data: &[u8],
    serialized_file: &SerializedFile,
) -> Result<AssetBundle, Box<dyn Error>> {
    let big_endian = unsafe {
        from_raw_parts(
            (&serialized_file.m_Header as *const _) as *const u8,
            size_of_val(&serialized_file.m_Header),
        )
    }[0x20] > 0;

    match big_endian {
        true => _construct_asset_bundle::<BigEndian>(data, serialized_file),
        false => _construct_asset_bundle::<LittleEndian>(data, serialized_file),
    }
}
