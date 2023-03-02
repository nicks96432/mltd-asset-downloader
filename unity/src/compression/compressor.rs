use super::method::Method;
use crate::error::Error;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Compressor {
    method: Method,
}

impl Compressor {
    pub fn new(method: Method) -> Self {
        Self { method }
    }

    pub fn compress(&self, buf: &[u8]) -> Result<Vec<u8>, Error> {
        match self.method {
            Method::None => Ok(Vec::from(buf)),

            #[cfg(feature = "lz4")]
            Method::Lz4 | Method::Lz4hc => {
                use lz4_flex::compress;

                Ok(compress(buf))
            }

            #[cfg(feature = "lzma")]
            Method::Lzma => {
                use std::io::Read;
                use xz2::read::XzEncoder;

                let mut output = Vec::new();
                let mut compressor = XzEncoder::new(buf, 6); // TODO: custom compression level
                if let Err(e) = compressor.read_to_end(&mut output) {
                    return Err(Error::LzmaDecompressError(e));
                }

                Ok(output)
            }

            #[cfg(feature = "lzham")]
            Method::Lzham => {
                use lzham::compress;
                use std::io::{BufReader, Cursor};

                let input = Vec::from(buf);
                let mut input = BufReader::new(Cursor::new(input));
                let mut output = Vec::new();

                let status = compress(&mut input, &mut output); // TODO: custom compression level
                if !status.is_success() {
                    return Err(Error::LzhamCompressError(status));
                }

                Ok(output)
            }

            #[cfg(not(all(feature = "lz4", feature = "lzma", feature = "lzham")))]
            _ => Err(Error::UnknownCompressionMethod),
        }
    }

    pub fn decompress(&self, buf: &[u8], output_size: usize) -> Result<Vec<u8>, Error> {
        match self.method {
            Method::None => Ok(Vec::from(buf)),

            #[cfg(feature = "lz4")]
            Method::Lz4 | Method::Lz4hc => {
                use lz4_flex::decompress;

                Ok(decompress(buf, output_size)?)
            }

            #[cfg(feature = "lzma")]
            Method::Lzma => {
                use std::io::Read;
                use xz2::read::XzDecoder;

                let mut output = vec![0u8; output_size];
                let mut decompressor = XzDecoder::new(buf);
                if let Err(e) = decompressor.read_exact(&mut output) {
                    return Err(Error::LzmaDecompressError(e));
                }

                Ok(output)
            }

            #[cfg(feature = "lzham")]
            Method::Lzham => {
                use lzham::decompress;
                use std::io::{BufReader, Cursor};

                let input = Vec::from(buf);
                let mut input = BufReader::new(Cursor::new(input));
                let mut output = Vec::with_capacity(output_size);

                let status = decompress(&mut input, &mut output, output_size);
                if !status.is_success() {
                    return Err(Error::LzhamDecompressError(status));
                }

                Ok(output)
            }

            #[cfg(not(all(feature = "lz4", feature = "lzma", feature = "lzham")))]
            _ => Err(Error::UnknownCompressionMethod),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

    const TESTCASE: [&str; 2] = ["Hello world!", "1234444"];

    fn test_compress(method: Method) -> Result<(), Error> {
        for case in TESTCASE {
            let compressor = Compressor::new(method);
            let output = compressor.compress(case.as_bytes())?;
            let output = compressor.decompress(&output, case.len())?;

            assert_eq!(case, String::from_utf8_lossy(&output));
        }

        Ok(())
    }

    #[test]
    fn test_none() {
        test_compress(Method::None).unwrap();
    }

    #[cfg(feature = "lz4")]
    #[test]
    fn test_lz4() {
        test_compress(Method::Lz4).unwrap();
    }

    #[cfg(feature = "lz4")]
    #[test]
    fn test_lz4hc() {
        test_compress(Method::Lz4hc).unwrap();
    }

    #[cfg(feature = "lzma")]
    #[test]
    fn test_lzma() {
        test_compress(Method::Lzma).unwrap();
    }

    #[cfg(feature = "lzham")]
    #[test]
    fn test_lzham() {
        test_compress(Method::Lzham).unwrap();
    }
}
