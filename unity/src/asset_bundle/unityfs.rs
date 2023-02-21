use crate::compression::{Compressor, Decompressor};
use crate::macros::impl_try_from_into_vec;
use crate::traits::{SeekAlign, UnityIO};
use crate::UnityError;
use crate::{AssetBundleHeader, AssetBundleSignature, InfoBlock, UnityFSHeader};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::mem::size_of;

#[derive(Debug, Clone)]
pub struct UnityFSBundle {
    pub bundle_header: AssetBundleHeader,
    pub unityfs_header: UnityFSHeader,
    pub info_block: InfoBlock,
    pub data: Vec<u8>,
}

impl UnityIO for UnityFSBundle {
    fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, UnityError> {
        // asset bundle header
        let bundle_header = AssetBundleHeader::read(reader)?;
        log::trace!("bundle header:\n{:#?}", bundle_header);
        if bundle_header.signature != AssetBundleSignature::UnityFS {
            return Err(UnityError::InvalidSignature);
        }

        // unityfs specific header
        let mut unityfs_header = UnityFSHeader::read(reader)?;
        assert!(unityfs_header.flags.info_block_combined());
        log::trace!("unityfs header:\n{:#?}", unityfs_header);

        unityfs_header.flags.new = bundle_header.version_engine.is_new();
        let unityfs_header = unityfs_header;

        // alignment
        if bundle_header.version >= 7 {
            reader.seek_align(16)?;
        }

        // find info block
        let compressed_size = usize::try_from(unityfs_header.compressed_size)?;
        let decompressed_size = usize::try_from(unityfs_header.decompressed_size)?;
        log::trace!(
            "info block size: {}, unccompressed size: {})",
            compressed_size,
            decompressed_size
        );

        let mut buf = vec![0u8; compressed_size];
        reader.read_exact(&mut buf)?;

        let compression_method = unityfs_header.flags.compression_method()?;
        log::trace!("info block compression method: {:?}", compression_method);

        if unityfs_header.flags.info_block_end() {
            let offset = -i64::try_from(compressed_size)?;
            reader.seek(SeekFrom::End(offset))?;
        }

        // decompress info block
        let buf = Decompressor::new(compression_method).decompress(&buf, decompressed_size)?;
        let info_block = InfoBlock::read(&mut Cursor::new(buf))?;

        if unityfs_header.flags.info_block_end() {
            let offset = u64::try_from(size_of::<AssetBundleHeader>())?;
            reader.seek(SeekFrom::Start(offset))?;
        }

        if unityfs_header.flags.info_block_padding() {
            reader.seek_align(16)?;
        }

        // data block
        let iter = info_block.block_infos.iter();
        let len = iter.fold(0u64, |acc, &x| acc + u64::from(x.decompressed_size));
        let mut data = Vec::with_capacity(usize::try_from(len)?);

        let iter = info_block.block_infos.iter();
        for block in iter {
            let mut buf = vec![0u8; usize::try_from(block.compressed_size)?];
            reader.read_exact(&mut buf)?;

            let decompressor = Decompressor::new(block.compression_method()?);
            let buf = decompressor.decompress(&buf, usize::try_from(block.decompressed_size)?)?;

            let expected_size = block.decompressed_size;
            assert_eq!(u32::try_from(buf.len())?, expected_size);

            data.write_all(&buf)?;
        }
        log::trace!("data block total size: {}", data.len());

        Ok(UnityFSBundle {
            bundle_header,
            unityfs_header,
            info_block,
            data,
        })
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), UnityError> {
        self.bundle_header.write(writer)?;

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
            cur += end;
        }

        // compress info block
        let mut buf = Vec::new();
        info_block.write(&mut buf)?;
        let compressed_info_block = compressor.compress(&buf)?;

        // modify header according to the sized of compressed size
        let mut unityfs_header = self.unityfs_header.clone();
        let compressed_len = u32::try_from(compressed_info_block.len())?;
        let decompressed_len = u32::try_from(buf.len())?;
        unityfs_header.compressed_size = compressed_len;
        unityfs_header.decompressed_size = decompressed_len;

        // modify bundle size in the header
        let header_size = u32::try_from(size_of::<AssetBundleHeader>())?;
        let data_block_size = u32::try_from(data_buf.iter().map(|d| d.len()).sum::<usize>())?;
        unityfs_header.bundle_size = u64::from(header_size + compressed_len + data_block_size);

        // finally write the data
        unityfs_header.write(writer)?;
        writer.write_all(&compressed_info_block)?;
        for data in data_buf {
            writer.write_all(&data)?;
        }

        Ok(())
    }
}

impl_try_from_into_vec!(UnityFSBundle);

#[cfg(test)]
#[ctor::ctor]
fn init() {
    mltd_utils::init_test_logger!();
}

#[cfg(test)]
mod tests {
    use crate::traits::UnityIO;
    use crate::UnityFSBundle;
    use std::fs::File;
    use std::io::Cursor;
    use std::path::Path;

    #[test]
    fn test_read() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("test.unity3d");
        let mut file = File::open(path).unwrap();
        UnityFSBundle::read(&mut file).unwrap();
    }

    #[test]
    fn test_write() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("test.unity3d");
        let mut file = File::open(path).unwrap();
        let expect = UnityFSBundle::read(&mut file).unwrap();

        let mut buf = Vec::new();
        expect.write(&mut buf).unwrap();
        log::trace!(
            "before: {}, after: {}",
            file.metadata().unwrap().len(),
            buf.len()
        );

        let got = UnityFSBundle::read(&mut Cursor::new(&buf)).unwrap();

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
        assert_eq!(expect.info_block.block_count, 1);
        assert_eq!(
            expect.info_block.block_infos[0].decompressed_size,
            got.info_block.block_infos[0].decompressed_size
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
    }
}
