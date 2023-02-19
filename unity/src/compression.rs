use crate::{error::CompressionError, UnityError};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionMethod {
    None = 0,
    LZMA,
    LZ4,
    LZ4HC,
    LZHAM,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Compressor {
    method: CompressionMethod,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Decompressor {
    method: CompressionMethod,
}

impl TryFrom<u32> for CompressionMethod {
    type Error = UnityError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CompressionMethod::None),
            1 => Ok(CompressionMethod::LZMA),
            2 => Ok(CompressionMethod::LZ4),
            3 => Ok(CompressionMethod::LZ4HC),
            4 => Ok(CompressionMethod::LZHAM),
            _ => Err(UnityError::UnknownCompressionMethod),
        }
    }
}

impl Compressor {
    pub fn new(method: CompressionMethod) -> Self {
        Self { method }
    }

    pub fn compress(&self, buf: &[u8]) -> Result<Vec<u8>, CompressionError> {
        match self.method {
            CompressionMethod::None => Ok(Vec::from(buf)),

            #[cfg(feature = "lz4")]
            CompressionMethod::LZ4 | CompressionMethod::LZ4HC => {
                use lz4_flex::compress;

                Ok(compress(buf))
            }

            #[cfg(feature = "lzma")]
            CompressionMethod::LZMA => {
                use std::io::Read;
                use xz2::read::XzEncoder;

                let mut output = Vec::new();
                let mut compressor = XzEncoder::new(buf, 6); // TODO: custom compression level
                if let Err(e) = compressor.read_to_end(&mut output) {
                    return Err(CompressionError::LZMADecompressError(e));
                }

                Ok(output)
            }

            #[cfg(feature = "lzham")]
            CompressionMethod::LZHAM => {
                use lzham::compress;
                use std::io::{BufReader, Cursor};

                let input = Vec::from(buf);
                let mut input = BufReader::new(Cursor::new(input));
                let mut output = Vec::new();

                let status = compress(&mut input, &mut output); // TODO: custom compression level
                if !status.is_success() {
                    return Err(CompressionError::LZHAMCompressError(status));
                }

                Ok(output)
            }

            #[cfg(not(all(feature = "lz4", feature = "lzma", feature = "lzham")))]
            _ => Err(CompressionError::Disabled),
        }
    }
}

impl Decompressor {
    pub fn new(method: CompressionMethod) -> Self {
        Self { method }
    }

    pub fn decompress(&self, buf: &[u8], output_size: usize) -> Result<Vec<u8>, CompressionError> {
        match self.method {
            CompressionMethod::None => Ok(Vec::from(buf)),

            #[cfg(feature = "lz4")]
            CompressionMethod::LZ4 | CompressionMethod::LZ4HC => {
                use lz4_flex::decompress;

                Ok(decompress(buf, output_size)?)
            }

            #[cfg(feature = "lzma")]
            CompressionMethod::LZMA => {
                use std::io::Read;
                use xz2::read::XzDecoder;

                let mut output = vec![0u8; output_size];
                let mut decompressor = XzDecoder::new(buf);
                if let Err(e) = decompressor.read_exact(&mut output) {
                    return Err(CompressionError::LZMADecompressError(e));
                }

                Ok(output)
            }

            #[cfg(feature = "lzham")]
            CompressionMethod::LZHAM => {
                use lzham::decompress;
                use std::io::{BufReader, Cursor};

                let input = Vec::from(buf);
                let mut input = BufReader::new(Cursor::new(input));
                let mut output = Vec::with_capacity(output_size);

                let status = decompress(&mut input, &mut output, output_size);
                if !status.is_success() {
                    return Err(CompressionError::LZHAMDecompressError(status));
                }

                Ok(output)
            }

            #[cfg(not(all(feature = "lz4", feature = "lzma", feature = "lzham")))]
            _ => Err(CompressionError::Disabled),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UnityError;

    const TESTCASE: [&str; 2] = ["Hello world!", "1234444"];

    fn test_compress(method: CompressionMethod) -> Result<(), UnityError> {
        for case in TESTCASE {
            let compressor = Compressor::new(method);
            let output = compressor.compress(case.as_bytes())?;

            let decompressor = Decompressor::new(method);
            let output = decompressor.decompress(&output, case.len())?;

            assert_eq!(case, String::from_utf8_lossy(&output));
        }

        Ok(())
    }

    #[test]
    fn test_none() -> Result<(), UnityError> {
        test_compress(CompressionMethod::None)
    }

    #[cfg(feature = "lz4")]
    #[test]
    fn test_lz4() -> Result<(), UnityError> {
        test_compress(CompressionMethod::LZ4)
    }

    #[cfg(feature = "lz4")]
    #[test]
    fn test_lz4hc() -> Result<(), UnityError> {
        test_compress(CompressionMethod::LZ4HC)
    }

    #[cfg(feature = "lzma")]
    #[test]
    fn test_lzma() -> Result<(), UnityError> {
        test_compress(CompressionMethod::LZMA)
    }

    #[cfg(feature = "lzham")]
    #[test]
    fn test_lzham() -> Result<(), UnityError> {
        test_compress(CompressionMethod::LZHAM)
    }
}
