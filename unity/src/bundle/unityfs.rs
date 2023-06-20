use super::{Header, InfoBlock, Signature, UnityFSHeader};
use crate::asset::Asset;
use crate::compression::Compressor;
use crate::error::Error;
use crate::traits::SeekAlign;
use crate::utils::bool_to_yes_no;

use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::mem::size_of;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct UnityFS {
    pub bundle_header: Header,
    pub unityfs_header: UnityFSHeader,
    pub info_block: InfoBlock,
    pub data: Vec<u8>,
    pub assets: Vec<Rc<RefCell<Asset>>>,
}

impl UnityFS {
    pub fn new() -> Self {
        Self {
            bundle_header: Header::new(),
            unityfs_header: UnityFSHeader::new(),
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

        log::debug!("asset bundle header");
        unityfs.bundle_header = Header::read(reader)?;
        log::trace!("bundle header:\n{:#?}", unityfs.bundle_header);
        if unityfs.bundle_header.signature != Signature::UnityFS {
            return Err(Error::UnknownSignature);
        }

        log::debug!("reading unityfs specific header");
        unityfs.unityfs_header = UnityFSHeader::read(reader)?;
        assert!(unityfs.unityfs_header.flags.info_block_combined());
        log::trace!("unityfs header:\n{:#?}", unityfs.unityfs_header);

        unityfs.unityfs_header.flags.new = unityfs.bundle_header.version_engine.is_new();

        if unityfs.bundle_header.version >= 7 {
            reader.seek_align(16)?;
        }

        log::debug!("finding info block");
        let compressed_size = usize::try_from(unityfs.unityfs_header.compressed_size)?;
        let decompressed_size = usize::try_from(unityfs.unityfs_header.decompressed_size)?;
        log::trace!(
            "info block size: {}, unccompressed size: {})",
            compressed_size,
            decompressed_size
        );

        let mut buf = vec![0u8; compressed_size];
        reader.read_exact(&mut buf)?;

        let compression_method = unityfs.unityfs_header.flags.compression_method()?;
        log::trace!("info block compression method: {:?}", compression_method);

        if unityfs.unityfs_header.flags.info_block_end() {
            let offset = -i64::try_from(compressed_size)?;
            reader.seek(SeekFrom::End(offset))?;
        }

        log::debug!("decompressing info block");
        let buf = Compressor::new(compression_method).decompress(&buf, decompressed_size)?;
        unityfs.info_block = InfoBlock::read(&mut Cursor::new(buf))?;

        if unityfs.unityfs_header.flags.info_block_end() {
            let offset = u64::try_from(size_of::<Header>())?;
            reader.seek(SeekFrom::Start(offset))?;
        }

        if unityfs.unityfs_header.flags.info_block_padding() {
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
        self.bundle_header.save(writer)?;

        // create compressor
        let method = self.unityfs_header.flags.compression_method()?;
        let compressor = Compressor::new(method);

        // read and compress data block, and modify compressed size in asset
        // info block accordingly
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
        let mut buf = Vec::new();
        info_block.save(&mut buf)?;
        let compressed_info_block = compressor.compress(&buf)?;

        // modify header according to the sized of compressed size
        let mut unityfs_header = self.unityfs_header;
        let compressed_len = u32::try_from(compressed_info_block.len())?;
        let decompressed_len = u32::try_from(buf.len())?;
        unityfs_header.compressed_size = compressed_len;
        unityfs_header.decompressed_size = decompressed_len;

        // modify bundle size in the header
        let header_size = u32::try_from(size_of::<Header>())?;
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
        writeln!(
            f,
            "Signature:                    {}",
            self.bundle_header.signature
        )?;
        writeln!(
            f,
            "Engine version:               {}",
            self.bundle_header.version_engine
        )?;
        writeln!(
            f,
            "Info block size:              {} (decompressed {})",
            self.unityfs_header.compressed_size, self.unityfs_header.decompressed_size
        )?;

        writeln!(f, "Flags:")?;
        write!(f, "    Compression method:                     ")?;
        if let Ok(compression_method) = self.unityfs_header.flags.compression_method() {
            writeln!(f, "{}", compression_method)?;
        } else {
            writeln!(f, "unknown")?;
        }

        writeln!(
            f,
            "    Block info and path info are combined?  {}",
            bool_to_yes_no(self.unityfs_header.flags.info_block_combined())
        )?;
        writeln!(
            f,
            "    Info block is at the end?               {}",
            bool_to_yes_no(self.unityfs_header.flags.info_block_end())
        )?;
        writeln!(
            f,
            "    Info block has padding at the begining? {}",
            bool_to_yes_no(self.unityfs_header.flags.info_block_padding())
        )?;

        writeln!(f, "====================Block Info====================")?;
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
        log::set_max_level(log::LevelFilter::Info);
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

        assert_eq!(expect.bundle_header, got.bundle_header);

        assert_eq!(
            expect.unityfs_header.decompressed_size,
            got.unityfs_header.decompressed_size
        );
        assert_eq!(expect.unityfs_header.flags, got.unityfs_header.flags);

        assert_eq!(
            expect.info_block.decompressed_hash,
            got.info_block.decompressed_hash
        );
        assert_eq!(expect.info_block.block_count, got.info_block.block_count);
        assert_eq!(expect.info_block.block_count, 3);

        assert_eq!(
            expect.info_block.block_infos[0].decompressed_size,
            got.info_block.block_infos[0].decompressed_size
        );
        assert_eq!(
            expect.info_block.block_infos[0].flags,
            got.info_block.block_infos[0].flags
        );

        assert_eq!(
            expect.info_block.block_infos[1].decompressed_size,
            got.info_block.block_infos[1].decompressed_size
        );
        assert_eq!(
            expect.info_block.block_infos[1].flags,
            got.info_block.block_infos[1].flags
        );

        assert_eq!(
            expect.info_block.block_infos[2].decompressed_size,
            got.info_block.block_infos[2].decompressed_size
        );
        assert_eq!(
            expect.info_block.block_infos[2].flags,
            got.info_block.block_infos[2].flags
        );

        assert_eq!(expect.info_block.path_count, got.info_block.path_count);
        assert_eq!(expect.info_block.path_count, 1);
        assert_eq!(
            expect.info_block.path_infos[0],
            got.info_block.path_infos[0]
        );

        assert_eq!(expect.data, got.data);
    }
}
