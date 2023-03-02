use crate::error::Error;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Method {
    None = 0,
    Lzma,
    Lz4,
    Lz4hc,
    Lzham,
}

impl TryFrom<u32> for Method {
    type Error = Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Method::None),
            1 => Ok(Method::Lzma),
            2 => Ok(Method::Lz4),
            3 => Ok(Method::Lz4hc),
            4 => Ok(Method::Lzham),
            _ => Err(Error::UnknownCompressionMethod),
        }
    }
}
