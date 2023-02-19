use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result};

#[cfg(feature = "lz4")]
use {
    lz4_flex::block::CompressError as LZ4CompressError,
    lz4_flex::block::DecompressError as LZ4DecompressError,
};

#[derive(Debug)]
pub enum CompressionError {
    #[cfg(feature = "lz4")]
    LZ4CompressError(LZ4CompressError),

    #[cfg(feature = "lz4")]
    LZ4DecompressError(LZ4DecompressError),

    #[cfg(feature = "lzma")]
    LZMADecompressError(std::io::Error),

    #[cfg(feature = "lzham")]
    LZHAMCompressError(lzham::compress::CompressionStatus),

    #[cfg(feature = "lzham")]
    LZHAMDecompressError(lzham::decompress::DecompressionStatus),
    Disabled,
}

impl StdError for CompressionError {}

impl Display for CompressionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            #[cfg(feature = "lz4")]
            Self::LZ4CompressError(e) => e.fmt(f),

            #[cfg(feature = "lz4")]
            Self::LZ4DecompressError(e) => e.fmt(f),

            #[cfg(feature = "lzma")]
            Self::LZMADecompressError(e) => e.fmt(f),

            #[cfg(feature = "lzham")]
            Self::LZHAMCompressError(e) => write!(f, "cannot compress: {:?}", e),

            #[cfg(feature = "lzham")]
            Self::LZHAMDecompressError(e) => write!(f, "cannot decompress: {:?}", e),

            Self::Disabled => {
                write!(f, "the compression method is disabled by the feature setting, enable it in Cargo.toml")
            }
        }
    }
}

#[cfg(feature = "lz4")]
impl From<LZ4CompressError> for CompressionError {
    fn from(value: LZ4CompressError) -> Self {
        CompressionError::LZ4CompressError(value)
    }
}

#[cfg(feature = "lz4")]
impl From<LZ4DecompressError> for CompressionError {
    fn from(value: LZ4DecompressError) -> Self {
        CompressionError::LZ4DecompressError(value)
    }
}
