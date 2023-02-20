mod header;
mod info_block;

pub use header::*;
pub use info_block::*;

use crate::{
    compression::{Compressor, Decompressor},
    error::UnityError,
};
use std::{
    io::{Cursor, Read, Seek, SeekFrom, Write},
    mem::size_of,
};

/// Extends [`Read`] with methods for reading exact number of bytes.
pub(crate) trait ReadExact: Read {
    /// Read the exact number of bytes.
    ///
    /// # Errors
    ///
    /// This function will return [`UnityError::FileError`] if the reader is unavailable.
    #[inline]
    fn read_str<const SIZE: usize>(&mut self) -> Result<[u8; SIZE], UnityError> {
        let mut buf = [0u8; SIZE];
        self.read_exact(&mut buf)?;

        Ok(buf)
    }
}

impl<R: Read> ReadExact for R {}

#[derive(Debug)]
pub struct AssetBundle {
    pub header: AssetBundleHeader,
    pub info_block: InfoBlock,
    pub data: Vec<u8>,
}

impl AssetBundle {
    pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self, UnityError> {
        // asset bundle header
        let header = AssetBundleHeader::from_reader(reader)?;
        assert!(header.has_info_block());
        log::trace!("header:\n{:#?}", header);

        // decompress info block
        let compressed_size: usize = header.compressed_info_block_size.try_into()?;
        let decompressed_size: usize = header.decompressed_info_block_size.try_into()?;
        log::trace!(
            "info block size: {}, unccompressed size: {})",
            compressed_size,
            decompressed_size
        );

        let mut buf = vec![0u8; compressed_size];
        reader.read_exact(&mut buf)?;

        let compression_method = header.compression_method()?;
        log::trace!("info block compression method: {:?}", compression_method);

        if header.info_block_end() {
            let offset = -i64::try_from(compressed_size)?;
            reader.seek(SeekFrom::End(offset))?;
        }

        // info block
        let buf = Decompressor::new(compression_method).decompress(&buf, decompressed_size)?;
        let info_block = InfoBlock::from_reader(&mut Cursor::new(buf))?;

        if header.info_block_end() {
            let offset = u64::try_from(size_of::<AssetBundleHeader>())?;
            reader.seek(SeekFrom::Start(offset))?;
        }

        // data block
        let iter = info_block.block_infos.iter();
        let len = iter.fold(0u64, |acc, &x| acc + u64::from(x.decompressed_size));
        let mut data = Vec::with_capacity(len.try_into()?);

        let iter = info_block.block_infos.iter();
        for block in iter {
            let mut buf = vec![0u8; block.compressed_size.try_into()?];
            reader.read_exact(&mut buf)?;

            let decompressor = Decompressor::new(block.compression_method()?);
            let buf = decompressor.decompress(&buf, block.decompressed_size.try_into()?)?;

            let expected_size = block.decompressed_size;
            assert_eq!(u32::try_from(buf.len())?, expected_size);

            data.write_all(&buf)?;
        }
        log::trace!("data block total size: {}", data.len());

        Ok(AssetBundle {
            header,
            info_block,
            data,
        })
    }

    /// Writes this struct into a writer in unity asset bundle format.
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn write<W: Write>(&self, writer: &mut W) -> Result<(), UnityError> {
        // create compressor
        let method = self.header.compression_method()?;
        let compressor = Compressor::new(method);

        // read and compress data block, and modify compressed size in asset
        // info block accordingly
        let mut cur: usize = 0;
        let mut data_buf = Vec::new();
        let mut info_block = self.info_block.clone();
        for block_info in info_block.block_infos.iter_mut() {
            let end = cur + usize::try_from(block_info.decompressed_size)?;
            let data = compressor.compress(&self.data[cur..end])?;
            block_info.compressed_size = data.len().try_into()?;
            data_buf.push(data);
            cur += end;
        }

        // compress info block
        let mut buf = Vec::new();
        info_block.write(&mut buf)?;
        let compressed_info_block = compressor.compress(&buf)?;

        // modify header according to the sized of compressed size
        let mut header = self.header.clone();
        let compressed_len = u32::try_from(compressed_info_block.len())?;
        let decompressed_len = u32::try_from(buf.len())?;
        header.compressed_info_block_size = compressed_len;
        header.decompressed_info_block_size = decompressed_len;

        // modify bundle size in the header
        let header_size = u32::try_from(size_of::<AssetBundleHeader>())?;
        let data_block_size = u32::try_from(data_buf.iter().map(|d| d.len()).sum::<usize>())?;
        header.bundle_size = u64::from(header_size + compressed_len + data_block_size);

        // finally write the data
        header.write(writer)?;
        writer.write_all(&compressed_info_block)?;
        for data in data_buf {
            writer.write_all(&data)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::error::UnityError;
    use crate::AssetBundle;
    use mltd_utils::log_formatter;
    use std::fs::File;
    use std::io::Cursor;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn setup() {
        INIT.call_once(|| {
            env_logger::builder()
                .is_test(true)
                .filter_module(env!("CARGO_PKG_NAME"), log::LevelFilter::Trace)
                .format(log_formatter)
                .init()
        })
    }

    #[test]
    fn test_from_reader() -> Result<(), UnityError> {
        setup();

        let mut file = File::open("/mnt/e/MLTD/assets/875600/001har_name.unity3d")?;
        AssetBundle::from_reader(&mut file)?;

        Ok(())
    }

    #[test]
    fn test_write() -> Result<(), UnityError> {
        setup();

        let mut file = File::open("/mnt/e/MLTD/assets/875600/001har_name.unity3d")?;
        let expect = AssetBundle::from_reader(&mut file)?;

        let mut buf = Vec::new();
        expect.write(&mut buf)?;
        log::trace!("before: {}, after: {}", file.metadata()?.len(), buf.len());

        let got = AssetBundle::from_reader(&mut Cursor::new(buf))?;

        assert_eq!(expect.header.signature, got.header.signature);
        assert!(expect.header.version == got.header.version);
        assert_eq!(expect.header.min_version, got.header.min_version);
        assert_eq!(expect.header.build_version, got.header.build_version);
        assert!(expect.header.flags == got.header.flags);

        assert_eq!(expect.info_block.hash, got.info_block.hash);
        assert_eq!(expect.info_block.block_count, got.info_block.block_count);
        assert_eq!(expect.info_block.block_count, 1);
        assert!(
            expect.info_block.block_infos[0].decompressed_size
                == got.info_block.block_infos[0].decompressed_size
        );
        assert!(expect.info_block.block_infos[0].flags == got.info_block.block_infos[0].flags);

        assert_eq!(expect.info_block.path_count, got.info_block.path_count);
        assert_eq!(expect.info_block.path_count, 2);
        assert_eq!(
            expect.info_block.path_infos[0],
            got.info_block.path_infos[0]
        );
        assert_eq!(
            expect.info_block.path_infos[1],
            got.info_block.path_infos[1]
        );

        assert_eq!(expect.data, got.data);

        Ok(())
    }
}
