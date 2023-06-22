use std::cell::{BorrowError, BorrowMutError};
use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result};
use std::io::Error as IOError;
use std::num::{ParseIntError, TryFromIntError};
use std::string::FromUtf8Error;

#[cfg(feature = "lz4")]
use {
    lz4_flex::block::CompressError as Lz4CompressError,
    lz4_flex::block::DecompressError as Lz4DecompressError,
};

#[derive(Debug)]
pub enum Error {
    BorrowError(BorrowError),
    BorrowMutError(BorrowMutError),
    FromUtf8Error(FromUtf8Error),
    InvalidVersion,
    IOError(IOError),
    ParseIntError(ParseIntError),
    TryFromIntError(TryFromIntError),
    UnknownClassIDType,
    UnknownCommonName,
    UnknownPlatform,
    UnknownSignature,

    #[cfg(feature = "lz4")]
    Lz4CompressError(Lz4CompressError),

    #[cfg(feature = "lz4")]
    Lz4DecompressError(Lz4DecompressError),

    #[cfg(feature = "lzham")]
    LzhamCompressError(lzham::compress::CompressionStatus),

    #[cfg(feature = "lzham")]
    LzhamDecompressError(lzham::decompress::DecompressionStatus),

    #[cfg(feature = "lzma")]
    LzmaDecompressError(IOError),
    UnknownCompressionMethod,
}

impl StdError for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::BorrowError(e) => e.fmt(f),
            Self::BorrowMutError(e) => e.fmt(f),
            Self::FromUtf8Error(e) => e.fmt(f),
            Self::InvalidVersion => write!(f, "invalid asset bundle version"),
            Self::IOError(e) => e.fmt(f),
            Self::ParseIntError(e) => e.fmt(f),
            Self::TryFromIntError(e) => e.fmt(f),
            Self::UnknownClassIDType => write!(f, "unknown asset class id type"),
            Self::UnknownCommonName => write!(f, "unknown asset class common name"),
            Self::UnknownPlatform => write!(f, "unknown asset target platform"),
            Self::UnknownSignature => write!(f, "unknown asset bundle signature"),

            #[cfg(feature = "lz4")]
            Self::Lz4CompressError(e) => e.fmt(f),

            #[cfg(feature = "lz4")]
            Self::Lz4DecompressError(e) => e.fmt(f),

            #[cfg(feature = "lzham")]
            Self::LzhamCompressError(e) => write!(f, "cannot compress: {:?}", e),

            #[cfg(feature = "lzham")]
            Self::LzhamDecompressError(e) => write!(f, "cannot decompress: {:?}", e),

            #[cfg(feature = "lzma")]
            Self::LzmaDecompressError(e) => e.fmt(f),

            Self::UnknownCompressionMethod => write!(f, "unknown asset bundle compression method"),
        }
    }
}

macro_rules! impl_from_for_error {
    ($type:ident) => {
        impl From<$type> for crate::error::Error {
            fn from(value: $type) -> Self {
                crate::error::Error::$type(value)
            }
        }
    };
}

impl_from_for_error!(BorrowError);
impl_from_for_error!(BorrowMutError);
impl_from_for_error!(FromUtf8Error);
impl_from_for_error!(IOError);
impl_from_for_error!(ParseIntError);
impl_from_for_error!(TryFromIntError);

#[cfg(feature = "lz4")]
impl_from_for_error!(Lz4CompressError);

#[cfg(feature = "lz4")]
impl_from_for_error!(Lz4DecompressError);
