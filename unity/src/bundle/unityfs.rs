use super::{InfoBlock, Signature, UnityFSHeader};
use crate::asset::Asset;
use crate::compression::Compressor;
use crate::error::Error;
use crate::macros::impl_default;
use crate::traits::SeekAlign;

use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::mem::size_of;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct UnityFS {
    pub header: UnityFSHeader,
    pub info_block: InfoBlock,
    pub data: Vec<u8>,
    pub assets: Vec<Rc<RefCell<Asset>>>,
}

impl UnityFS {
    pub fn new() -> Self {
        Self {
            header: UnityFSHeader::new(),
            info_block: InfoBlock::new(),
            data: Vec::new(),
            assets: Vec::new(),
        }
    }

    pub fn read<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: Read + Seek,
    {
        let mut unityfs = Self::new();

        log::debug!("reading unityfs header");
        unityfs.header = UnityFSHeader::read(reader)?;
        if unityfs.header.signature != Signature::UnityFS {
            return Err(Error::UnknownSignature);
        }

        if !unityfs.header.flags.info_block_combined() {
            unimplemented!()
        }
        log::trace!("unityfs header:\n{:#?}", unityfs.header);

        if unityfs.header.version_format >= 7 {
            reader.seek_align(16)?;
        }

        log::debug!("finding info block");
        let compressed_size = usize::try_from(unityfs.header.info_block_compressed_size)?;
        let decompressed_size = usize::try_from(unityfs.header.info_block_decompressed_size)?;
        log::trace!(
            "info block size: {}, unccompressed size: {})",
            compressed_size,
            decompressed_size
        );

        let mut buf = vec![0u8; compressed_size];
        reader.read_exact(&mut buf)?;

        let compression_method = unityfs.header.flags.compression_method()?;
        log::trace!("info block compression method: {:?}", compression_method);

        let seek_pos = reader.stream_position()?;
        if unityfs.header.flags.info_block_end() {
            log::trace!("info block is at the end, seeking to the end");
            reader.seek(SeekFrom::End(-i64::try_from(compressed_size)?))?;
        }

        log::debug!("decompressing info block");
        let buf = Compressor::new(compression_method).decompress(&buf, decompressed_size)?;
        unityfs.info_block = InfoBlock::read(&mut Cursor::new(buf))?;

        if unityfs.header.flags.info_block_end() {
            reader.seek(SeekFrom::Start(seek_pos))?;
        }

        if unityfs.header.flags.info_block_padding() {
            reader.seek_align(16)?;
        }

        log::debug!("reading info block");
        let iter = unityfs.info_block.block_infos.iter();
        let len = iter.fold(0u64, |acc, &x| acc + u64::from(x.decompressed_size));
        unityfs.data = Vec::with_capacity(usize::try_from(len)?);

        let iter = unityfs.info_block.block_infos.iter();
        for block in iter {
            let mut buf = vec![0u8; usize::try_from(block.compressed_size)?];
            reader.read_exact(&mut buf)?;

            let decompressor = Compressor::new(block.compression_method()?);
            let buf = decompressor.decompress(&buf, usize::try_from(block.decompressed_size)?)?;

            let expected_size = block.decompressed_size;
            assert_eq!(u32::try_from(buf.len())?, expected_size);

            unityfs.data.write_all(&buf)?;
        }
        log::trace!("data block total size: {}", unityfs.data.len());

        log::debug!("parsing assets");
        for (i, path_info) in unityfs.info_block.path_infos.iter().enumerate() {
            let begin = usize::try_from(path_info.offset)?;
            let end = usize::try_from(path_info.decompressed_size)?;

            log::trace!("asset {}:", i);
            let asset = Asset::read(Cursor::new(unityfs.data[begin..end].to_vec()))?;
            unityfs.assets.push(asset);
        }

        Ok(unityfs)
    }

    pub fn save<W>(&self, writer: &mut W) -> Result<(), Error>
    where
        W: Write,
    {
        // create compressor
        let method = self.header.flags.compression_method()?;
        let compressor = Compressor::new(method);

        // read and compress data block, and modify compressed size in info block accordingly
        let mut cur: usize = 0;
        let mut data_buf = Vec::new();
        let mut info_block = self.info_block.clone();
        for block_info in info_block.block_infos.iter_mut() {
            let end = cur + usize::try_from(block_info.decompressed_size)?;
            let data = compressor.compress(&self.data[cur..end])?;
            block_info.compressed_size = u32::try_from(data.len())?;
            data_buf.push(data);
            cur = end;
        }

        // compress info block
        let mut info_block_buf = Vec::new();
        info_block.save(&mut info_block_buf)?;
        let compressed_info_block = compressor.compress(&info_block_buf)?;

        // modify header according to the compressed size
        let mut unityfs_header = self.header.clone();
        let compressed_len = u32::try_from(compressed_info_block.len())?;
        let decompressed_len = u32::try_from(info_block_buf.len())?;
        unityfs_header.info_block_compressed_size = compressed_len;
        unityfs_header.info_block_decompressed_size = decompressed_len;

        // modify bundle size in the header
        let header_size = u32::try_from(size_of::<UnityFSHeader>())?;
        let data_block_size = u32::try_from(data_buf.iter().map(|d| d.len()).sum::<usize>())?;
        unityfs_header.bundle_size = u64::from(header_size + compressed_len + data_block_size);

        // finally write the data
        unityfs_header.save(writer)?;
        writer.write_all(&compressed_info_block)?;
        for data in data_buf {
            writer.write_all(&data)?;
        }

        Ok(())
    }
}

impl Display for UnityFS {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "================Basic Information=================")?;
        writeln!(f, "{}", self.header)?;

        writeln!(f, "===================Block Infos====================")?;
        for (i, block_info) in self.info_block.block_infos.iter().enumerate() {
            writeln!(
                f,
                "Block {:>2} size: {:<8} (decompressed {})",
                i, block_info.compressed_size, block_info.decompressed_size
            )?;
        }

        writeln!(f, "======================Assets======================")?;
        for (i, path_info) in self.info_block.path_infos.iter().enumerate() {
            writeln!(
                f,
                "Asset {} ({}): data offset {}",
                i, path_info.path, path_info.offset
            )?;
            writeln!(f, "{:4}", self.assets[i].borrow())?;
        }

        Ok(())
    }
}

impl_default!(UnityFS);

#[cfg(test)]
#[ctor::ctor]
fn init() {
    mltd_utils::init_test_logger!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::path::Path;

    #[test]
    fn test_read() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("test.unity3d");
        let mut file = File::open(path).unwrap();

        let bundle = UnityFS::read(&mut file).unwrap();
        println!("{}", bundle);
    }

    #[test]
    fn test_write() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("test.unity3d");
        let mut file = File::open(path).unwrap();
        let expect = UnityFS::read(&mut file).unwrap();

        let mut buf = Vec::new();
        expect.save(&mut buf).unwrap();
        log::trace!(
            "before: {}, after: {}",
            file.metadata().unwrap().len(),
            buf.len()
        );

        let got = UnityFS::read(&mut Cursor::new(&buf)).unwrap();

        assert_eq!(expect.header.signature, got.header.signature);
        assert_eq!(expect.header.flags, got.header.flags);
        assert_eq!(expect.header.version_format, got.header.version_format);
        assert_eq!(expect.header.version_engine, got.header.version_engine);
        assert_eq!(expect.header.version_target, got.header.version_target);

        assert_eq!(expect.info_block.block_count, got.info_block.block_count);
        assert_eq!(expect.info_block.path_count, got.info_block.path_count);

        assert_eq!(expect.data, got.data);
    }
}
