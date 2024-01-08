use std::error::Error;
use std::fs::File;
use std::io::Cursor;
use std::io::Write;
use std::mem::size_of_val;
use std::path::Path;
use std::slice::from_raw_parts;

use byteorder::BigEndian;
use byteorder::ByteOrder;
use byteorder::LittleEndian;
use rabex::{files::SerializedFile, read_ext::ReadUrexExt};

use crate::utils::ReadAlignedExt;
use crate::ExtractorArgs;

pub fn _extract_acb<P, E>(
    data: &[u8],
    output_dir: P,
    _args: &ExtractorArgs,
) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
    E: ByteOrder,
{
    let mut reader = Cursor::new(data);
    reader.read_aligned_string::<E>()?;

    let data = reader.read_bytes::<E>()?;

    // assert that there is only one track in an ACB file
    let track = acb::to_tracks(&data)?.swap_remove(0);

    // TODO: Add option to specify output format
    let path = output_dir.as_ref().join(Path::new(&track.name).with_extension("wav"));
    let mut file = File::create(&path)?;

    log::info!("writing audio to {}", path.display());
    file.write_all(&track.data)?;

    Ok(())
}

pub fn extract_acb<P>(
    data: &[u8],
    output_dir: P,
    args: &ExtractorArgs,
    serialized_file: &SerializedFile,
) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
{
    let big_endian = unsafe {
        from_raw_parts(
            (&serialized_file.m_Header as *const _) as *const u8,
            size_of_val(&serialized_file.m_Header),
        )
    }[0x20]
        > 0;

    match big_endian {
        true => _extract_acb::<_, BigEndian>(data, output_dir, args),
        false => _extract_acb::<_, LittleEndian>(data, output_dir, args),
    }
}
