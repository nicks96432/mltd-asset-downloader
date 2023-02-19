use std::error::Error;
use std::fmt::{Display, Formatter, Result};
use std::io::Error as IOError;
use std::num::TryFromIntError;
use std::str::Utf8Error;

use crate::{CompressionError, FileError};

#[derive(Debug)]
pub enum UnityError {
    InvalidSignature,
    FileError(FileError),
    TryFromIntError(TryFromIntError),
    UnknownCompressionMethod,
}

impl Error for UnityError {}

impl Display for UnityError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::InvalidSignature => write!(f, "invalid asset bundle signature"),
            Self::FileError(e) => e.fmt(f),
            Self::TryFromIntError(e) => e.fmt(f),
            Self::UnknownCompressionMethod => write!(f, "unknown asset bundle compression method"),
        }
    }
}

impl From<IOError> for UnityError {
    fn from(value: IOError) -> Self {
        UnityError::FileError(FileError::ReadError(value))
    }
}

impl From<Utf8Error> for UnityError {
    fn from(value: Utf8Error) -> Self {
        UnityError::FileError(FileError::Utf8Error(value))
    }
}

impl From<TryFromIntError> for UnityError {
    fn from(value: TryFromIntError) -> Self {
        UnityError::TryFromIntError(value)
    }
}

impl From<CompressionError> for UnityError {
    fn from(value: CompressionError) -> Self {
        UnityError::FileError(FileError::CompressionError(value))
    }
}
