use crate::error::Error;

use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use paste::paste;

use std::io::{Error as IOError, Read, Seek, SeekFrom, Write};

/// Extends [`Read`] with methods for reading strings.
pub(crate) trait ReadString: Read {
    /// Reads a null-terminated string.
    ///
    /// # Errors
    ///
    /// This function will return [`Error::IOError`]if the reader is unavailable.
    #[inline]
    fn read_string(&mut self) -> Result<String, Error> {
        let mut buf = Vec::new();
        loop {
            let byte = self.read_u8()?;
            if byte == 0u8 {
                break;
            }
            buf.push(byte);
        }

        Ok(String::from_utf8(buf)?)
    }
}

impl<R> ReadString for R where R: Read {}

/// Extends [`Seek`] with methods for seeking to byte alignment.
pub(crate) trait SeekAlign: Seek {
    /// Seeks to alignment.
    ///
    /// # Errors
    ///
    /// This function will return [`Error::IOError`]
    /// if the reader is unavailable.
    ///
    /// This function will return [`Error::TryFromIntError`]
    /// if integer conversion is failed.
    #[inline]
    fn seek_align(&mut self, alignment: u64) -> Result<(), Error> {
        let pos = i64::try_from(self.stream_position()?)?;
        let alignment = i64::try_from(alignment)?;
        let alignment = (alignment - pos % alignment) % alignment;
        self.seek(SeekFrom::Current(alignment))?;

        Ok(())
    }
}

impl<S> SeekAlign for S where S: Seek {}

/// Extends [`Write`] [`Seek`] with methods for writing byte alignment.
pub(crate) trait WriteAlign: Write + Seek {
    /// Write bytes alignment.
    ///
    /// # Errors
    ///
    /// This function will return [`Error::IOError`]
    /// if the writer is unavailable.
    ///
    /// This function will return [`Error::TryFromIntError`]
    /// if integer conversion is failed.
    #[inline]
    fn write_align(&mut self, alignment: u64) -> Result<(), Error> {
        let pos = i64::try_from(self.stream_position()?)?;
        let alignment = i64::try_from(alignment)?;
        let alignment = (alignment - pos % alignment) % alignment;

        for _ in 0i64..alignment {
            self.write_u8(0u8)?;
        }

        Ok(())
    }
}

impl<W> WriteAlign for W where W: Write + Seek {}

macro_rules! read_type_by {
    ($reader:ident, $type:ident, $endian:ident) => {
        paste! {
            match $endian {
                true => $reader.[<read_ $type>]::<BigEndian>(),
                false => $reader.[<read_ $type>]::<LittleEndian>(),
            }
        }
    };
}

macro_rules! write_type_by {
    ($writer:ident, $type:ident, $endian:ident, $n:ident) => {
        paste! {
            match $endian {
                true => $writer.[<write_ $type>]::<BigEndian>($n),
                false => $writer.[<write_ $type>]::<LittleEndian>($n),
            }
        }
    };
}

macro_rules! read_array_by {
    ($reader:ident, $type:ident, $endian:ident) => {
        paste! {
            (0u32..$reader.read_u32_by($endian)?)
                .map(|_| $reader.[<read_ $type _by>]($endian)).collect()
        }
    };
}

pub(crate) trait ReadPrimitiveExt: Read {
    #[inline]
    fn read_i16_by(&mut self, endian: bool) -> Result<i16, IOError> {
        read_type_by!(self, i16, endian)
    }

    #[inline]
    fn read_i24_by(&mut self, endian: bool) -> Result<i32, IOError> {
        read_type_by!(self, i24, endian)
    }

    #[inline]
    fn read_i32_by(&mut self, endian: bool) -> Result<i32, IOError> {
        read_type_by!(self, i32, endian)
    }

    #[inline]
    fn read_i48_by(&mut self, endian: bool) -> Result<i64, IOError> {
        read_type_by!(self, i48, endian)
    }

    #[inline]
    fn read_i64_by(&mut self, endian: bool) -> Result<i64, IOError> {
        read_type_by!(self, i64, endian)
    }

    #[inline]
    fn read_u16_by(&mut self, endian: bool) -> Result<u16, IOError> {
        read_type_by!(self, u16, endian)
    }

    #[inline]
    fn read_u24_by(&mut self, endian: bool) -> Result<u32, IOError> {
        read_type_by!(self, u24, endian)
    }

    #[inline]
    fn read_u32_by(&mut self, endian: bool) -> Result<u32, IOError> {
        read_type_by!(self, u32, endian)
    }

    #[inline]
    fn read_u48_by(&mut self, endian: bool) -> Result<u64, IOError> {
        read_type_by!(self, u48, endian)
    }

    #[inline]
    fn read_u64_by(&mut self, endian: bool) -> Result<u64, IOError> {
        read_type_by!(self, u64, endian)
    }

    #[inline]
    fn read_f32_by(&mut self, endian: bool) -> Result<f32, IOError> {
        read_type_by!(self, f32, endian)
    }

    #[inline]
    fn read_f64_by(&mut self, endian: bool) -> Result<f64, IOError> {
        read_type_by!(self, f64, endian)
    }
}

impl<R> ReadPrimitiveExt for R where R: Read {}

pub(crate) trait WritePrimitiveExt: Write {
    #[inline]
    fn write_i16_by(&mut self, n: i16, endian: bool) -> Result<(), IOError> {
        write_type_by!(self, i16, endian, n)
    }

    #[inline]
    fn write_i24_by(&mut self, n: i32, endian: bool) -> Result<(), IOError> {
        write_type_by!(self, i24, endian, n)
    }

    #[inline]
    fn write_i32_by(&mut self, n: i32, endian: bool) -> Result<(), IOError> {
        write_type_by!(self, i32, endian, n)
    }

    #[inline]
    fn write_i48_by(&mut self, n: i64, endian: bool) -> Result<(), IOError> {
        write_type_by!(self, i48, endian, n)
    }

    #[inline]
    fn write_i64_by(&mut self, n: i64, endian: bool) -> Result<(), IOError> {
        write_type_by!(self, i64, endian, n)
    }

    #[inline]
    fn write_u16_by(&mut self, n: u16, endian: bool) -> Result<(), IOError> {
        write_type_by!(self, u16, endian, n)
    }

    #[inline]
    fn write_u24_by(&mut self, n: u32, endian: bool) -> Result<(), IOError> {
        write_type_by!(self, u24, endian, n)
    }

    #[inline]
    fn write_u32_by(&mut self, n: u32, endian: bool) -> Result<(), IOError> {
        write_type_by!(self, u32, endian, n)
    }

    #[inline]
    fn write_u48_by(&mut self, n: u64, endian: bool) -> Result<(), IOError> {
        write_type_by!(self, u48, endian, n)
    }

    #[inline]
    fn write_u64_by(&mut self, n: u64, endian: bool) -> Result<(), IOError> {
        write_type_by!(self, u64, endian, n)
    }

    #[inline]
    fn write_f32_by(&mut self, n: f32, endian: bool) -> Result<(), IOError> {
        write_type_by!(self, f32, endian, n)
    }

    #[inline]
    fn write_f64_by(&mut self, n: f64, endian: bool) -> Result<(), IOError> {
        write_type_by!(self, f64, endian, n)
    }
}

impl<W> WritePrimitiveExt for W where W: Write {}

pub(crate) trait ReadVecExt: ReadPrimitiveExt {
    #[inline]
    fn read_i8_vec_by(&mut self, endian: bool) -> Result<Vec<i8>, IOError> {
        let iter = 0u32..self.read_u32_by(endian)?;
        iter.map(|_| self.read_i8()).collect()
    }

    #[inline]
    fn read_i16_vec_by(&mut self, endian: bool) -> Result<Vec<i16>, IOError> {
        read_array_by!(self, i16, endian)
    }

    #[inline]
    fn read_i24_vec_by(&mut self, endian: bool) -> Result<Vec<i32>, IOError> {
        read_array_by!(self, i24, endian)
    }

    #[inline]
    fn read_i32_vec_by(&mut self, endian: bool) -> Result<Vec<i32>, IOError> {
        read_array_by!(self, i32, endian)
    }

    #[inline]
    fn read_i48_vec_by(&mut self, endian: bool) -> Result<Vec<i64>, IOError> {
        read_array_by!(self, i48, endian)
    }

    #[inline]
    fn read_i64_vec_by(&mut self, endian: bool) -> Result<Vec<i64>, IOError> {
        read_array_by!(self, i64, endian)
    }

    #[inline]
    fn read_u8_vec_by(&mut self, endian: bool) -> Result<Vec<u8>, IOError> {
        let iter = 0u32..self.read_u32_by(endian)?;
        iter.map(|_| self.read_u8()).collect()
    }

    #[inline]
    fn read_u16_vec_by(&mut self, endian: bool) -> Result<Vec<u16>, IOError> {
        read_array_by!(self, u16, endian)
    }

    #[inline]
    fn read_u24_vec_by(&mut self, endian: bool) -> Result<Vec<u32>, IOError> {
        read_array_by!(self, u24, endian)
    }

    #[inline]
    fn read_u32_vec_by(&mut self, endian: bool) -> Result<Vec<u32>, IOError> {
        read_array_by!(self, u32, endian)
    }

    #[inline]
    fn read_u48_vec_by(&mut self, endian: bool) -> Result<Vec<u64>, IOError> {
        read_array_by!(self, u48, endian)
    }

    #[inline]
    fn read_u64_vec_by(&mut self, endian: bool) -> Result<Vec<u64>, IOError> {
        read_array_by!(self, u64, endian)
    }
}

impl<R> ReadVecExt for R where R: Read {}

pub(crate) trait ReadAlignedString:
    Read + SeekAlign + ReadPrimitiveExt + ReadVecExt
{
    #[inline]
    fn read_aligned_string(&mut self, endian: bool, alignment: u64) -> Result<String, Error> {
        let buf = self.read_u8_vec_by(endian)?;
        self.seek_align(alignment)?;

        Ok(String::from_utf8(buf)?)
    }
}

impl<R> ReadAlignedString for R where R: Read + SeekAlign + ReadPrimitiveExt + ReadVecExt {}
