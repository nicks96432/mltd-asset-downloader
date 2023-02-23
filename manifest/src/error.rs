use std::error::Error;
use std::fmt::{Display, Formatter, Result};
use std::io::Error as IOError;

#[derive(Debug)]
pub enum ManifestError {
    DeserializeFailed,
    FileCreateFailed(IOError),
    FileWriteFailed(IOError),
    RequestFailed,
    SerializeFailed,
    StructFieldNotFound,
    UnknownVariant,
}

impl Error for ManifestError {}

impl Display for ManifestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::DeserializeFailed => write!(f, "deserialize manifest failed"),
            Self::FileCreateFailed(e) => write!(f, "cannot create output file: {}", e),
            Self::FileWriteFailed(e) => write!(f, "cannot write to output file: {}", e),
            Self::RequestFailed => write!(f, "fail to send request"),
            Self::SerializeFailed => write!(f, "serialize manifest failed"),
            Self::StructFieldNotFound => write!(f, "struct field not found"),
            Self::UnknownVariant => write!(f, "unknown os variant"),
        }
    }
}
