use crate::macros::impl_from;
use crate::macros::impl_from_file_error;
use crate::{CompressionError, FileError};
use std::error::Error;
use std::fmt::{Display, Formatter, Result};
use std::io::Error as IOError;
use std::num::{ParseIntError, TryFromIntError};
use std::str::Utf8Error;

#[derive(Debug)]
pub enum UnityError {
    InvalidSignature,
    InvalidVersion,
    FileError(FileError),
    TryFromIntError(TryFromIntError),
    ParseIntError(ParseIntError),
    UnknownCompressionMethod,
}

impl Error for UnityError {}

impl Display for UnityError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::InvalidSignature => write!(f, "invalid asset bundle signature"),
            Self::InvalidVersion => write!(f, "invalid asset bundle version"),
            Self::FileError(e) => e.fmt(f),
            Self::TryFromIntError(e) => e.fmt(f),
            Self::ParseIntError(e) => e.fmt(f),
            Self::UnknownCompressionMethod => write!(f, "unknown asset bundle compression method"),
        }
    }
}

impl_from_file_error!(IOError);
impl_from_file_error!(Utf8Error);
impl_from_file_error!(CompressionError);

impl_from!(TryFromIntError);
impl_from!(ParseIntError);
