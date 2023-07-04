use num_derive::{FromPrimitive, ToPrimitive};

use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum Method {
    None = 0,
    Lzma,
    Lz4,
    Lz4hc,
    Lzham,
}

impl Display for Method {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}",
            match self {
                Self::None => "None",
                Self::Lzma => "LZMA",
                Self::Lz4 => "LZ4",
                Self::Lz4hc => "LZ4HC",
                Self::Lzham => "LZHAM",
            }
        )
    }
}
