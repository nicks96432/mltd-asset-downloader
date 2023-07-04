use crate::error::Error;

use std::backtrace::Backtrace;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signature {
    UnityFS,
    UnityWeb,
    UnityRaw,
    UnityArchive,
}

impl FromStr for Signature {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "UnityFS" => Ok(Self::UnityFS),
            "UnityWeb" => Ok(Self::UnityWeb),
            "UnityRaw" => Ok(Self::UnityRaw),
            "UnityArchive" => Ok(Self::UnityArchive),
            _ => Err(Error::UnknownSignature {
                signature: s.to_string(),
                backtrace: Backtrace::capture(),
            }),
        }
    }
}

impl Display for Signature {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnityFS => write!(f, "UnityFS"),
            Self::UnityWeb => write!(f, "UnityWeb"),
            Self::UnityRaw => write!(f, "UnityRaw"),
            Self::UnityArchive => write!(f, "UnityArchive"),
        }
    }
}
