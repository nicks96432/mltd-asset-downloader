use crate::CompressionError;
use std::error::Error;
use std::fmt::{Display, Formatter, Result};
use std::io::Error as IOError;
use std::str::Utf8Error;

#[derive(Debug)]
pub enum FileError {
    CompressionError(CompressionError),
    IOError(IOError),
    Utf8Error(Utf8Error),
}

impl Error for FileError {}

impl Display for FileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::CompressionError(e) => e.fmt(f),
            Self::IOError(e) => e.fmt(f),
            Self::Utf8Error(e) => e.fmt(f),
        }
    }
}
