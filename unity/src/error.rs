use crate::bundle::Signature;

use thiserror::Error;

use std::backtrace::Backtrace;
use std::cell::{BorrowError, BorrowMutError};
use std::io::Error as IOError;
use std::num::{ParseIntError, TryFromIntError};
use std::string::FromUtf8Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{source}")]
    BorrowError {
        #[from]
        source: BorrowError,
        backtrace: Backtrace,
    },

    #[error("{source}")]
    BorrowMutError {
        #[from]
        source: BorrowMutError,
        backtrace: Backtrace,
    },

    #[error("{source}")]
    FromUtf8Error {
        #[from]
        source: FromUtf8Error,
        backtrace: Backtrace,
    },

    #[error("invalid asset bundle signature, expected {expected}, got {got}")]
    InvalidSignature {
        expected: Signature,
        got: Signature,

        #[backtrace]
        backtrace: Backtrace,
    },

    #[error("invalid asset bundle version")]
    InvalidVersion {
        version: String,

        #[backtrace]
        backtrace: Backtrace,
    },

    #[error("{source}")]
    IOError {
        #[from]
        source: IOError,
        backtrace: Backtrace,
    },

    #[error("{source}")]
    ParseIntError {
        #[from]
        source: ParseIntError,
        backtrace: Backtrace,
    },

    #[error("{source}")]
    TryFromIntError {
        #[from]
        source: TryFromIntError,
        backtrace: Backtrace,
    },

    #[error("unknown asset class id type")]
    UnknownClassIDType { class_id: i32, backtrace: Backtrace },

    #[error("unknown asset class common name")]
    UnknownCommonName,

    #[error("unknown asset target platform")]
    UnknownPlatform,

    #[error("unknown asset bundle signature")]
    UnknownSignature {
        signature: String,
        backtrace: Backtrace,
    },

    #[error("unknown texture format")]
    UnknownTextureFormat {
        format: u32,
        backtrace: Backtrace,
    },

    #[cfg(feature = "lz4")]
    #[error("cannot compress: {0}")]
    Lz4CompressError(#[from] lz4_flex::block::CompressError),

    #[cfg(feature = "lz4")]
    #[error("cannot decompress: {0}")]
    Lz4DecompressError(#[from] lz4_flex::block::DecompressError),

    #[cfg(feature = "lzham")]
    #[error("cannot compress: {0}")]
    LzhamCompressError(#[from] lzham::compress::CompressionStatus),

    #[cfg(feature = "lzham")]
    #[error("cannot decompress: {0}")]
    LzhamDecompressError(#[from] lzham::decompress::DecompressionStatus),

    #[cfg(feature = "lzma")]
    #[error("cannot decompress: {0}")]
    LzmaDecompressError(IOError),

    #[error("unknown asset bundle compression method")]
    UnknownCompressionMethod,
}
