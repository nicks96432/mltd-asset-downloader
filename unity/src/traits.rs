use crate::error::Error;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::Result as IOResult;
use std::io::{Read, Seek, SeekFrom};

/// Extends [`Read`] with methods for reading strings.
pub(crate) trait ReadString: Read {
    /// Reads a null-terminated string.
    ///
    /// # Errors
    ///
    /// This function will return [`UnityError::FileError`] if the reader is unavailable.
    fn read_string(&mut self) -> IOResult<String>;
}

impl<R: Read> ReadString for R {
    #[inline]
    fn read_string(&mut self) -> IOResult<String> {
        let mut buf = Vec::new();
        loop {
            let byte = self.read_u8()?;
            if byte == 0u8 {
                break;
            }
            buf.push(byte);
        }

        Ok(String::from_utf8_lossy(&buf).into_owned())
    }
}

/// Extends [`Seek`] with methods for reading exact number of bytes.
pub(crate) trait SeekAlign: Seek {
    /// Seeks to alignment.
    ///
    /// # Errors
    ///
    /// This function will return [`UnityError::FileError`]
    /// if the reader is unavailable.
    ///
    /// This function will return [`UnityError::TryFromIntError`]
    /// if integer conversion is failed.
    fn seek_align(&mut self, alignment: u64) -> Result<(), Error>;
}

impl<S: Seek> SeekAlign for S {
    #[inline]
    fn seek_align(&mut self, alignment: u64) -> Result<(), Error> {
        let pos = i64::try_from(self.stream_position()?)?;
        let alignment = i64::try_from(alignment)?;
        let align = (alignment - pos % alignment) % alignment;
        self.seek(SeekFrom::Current(align))?;

        Ok(())
    }
}

pub(crate) trait ReadIntExt: Read {
    fn read_i16_by(&mut self, endian: bool) -> Result<i16, std::io::Error>;
    fn read_i24_by(&mut self, endian: bool) -> Result<i32, std::io::Error>;
    fn read_i32_by(&mut self, endian: bool) -> Result<i32, std::io::Error>;
    fn read_i48_by(&mut self, endian: bool) -> Result<i64, std::io::Error>;
    fn read_i64_by(&mut self, endian: bool) -> Result<i64, std::io::Error>;
    fn read_u16_by(&mut self, endian: bool) -> Result<u16, std::io::Error>;
    fn read_u24_by(&mut self, endian: bool) -> Result<u32, std::io::Error>;
    fn read_u32_by(&mut self, endian: bool) -> Result<u32, std::io::Error>;
    fn read_u48_by(&mut self, endian: bool) -> Result<u64, std::io::Error>;
    fn read_u64_by(&mut self, endian: bool) -> Result<u64, std::io::Error>;
}

impl<R: Read> ReadIntExt for R {
    #[inline]
    fn read_i16_by(&mut self, endian: bool) -> Result<i16, std::io::Error> {
        match endian {
            true => self.read_i16::<BigEndian>(),
            false => self.read_i16::<LittleEndian>(),
        }
    }

    #[inline]
    fn read_i24_by(&mut self, endian: bool) -> Result<i32, std::io::Error> {
        match endian {
            true => self.read_i24::<BigEndian>(),
            false => self.read_i24::<LittleEndian>(),
        }
    }

    #[inline]
    fn read_i32_by(&mut self, endian: bool) -> Result<i32, std::io::Error> {
        match endian {
            true => self.read_i32::<BigEndian>(),
            false => self.read_i32::<LittleEndian>(),
        }
    }

    #[inline]
    fn read_i48_by(&mut self, endian: bool) -> Result<i64, std::io::Error> {
        match endian {
            true => self.read_i48::<BigEndian>(),
            false => self.read_i48::<LittleEndian>(),
        }
    }

    #[inline]
    fn read_i64_by(&mut self, endian: bool) -> Result<i64, std::io::Error> {
        match endian {
            true => self.read_i64::<BigEndian>(),
            false => self.read_i64::<LittleEndian>(),
        }
    }

    #[inline]
    fn read_u16_by(&mut self, endian: bool) -> Result<u16, std::io::Error> {
        match endian {
            true => self.read_u16::<BigEndian>(),
            false => self.read_u16::<LittleEndian>(),
        }
    }

    #[inline]
    fn read_u24_by(&mut self, endian: bool) -> Result<u32, std::io::Error> {
        match endian {
            true => self.read_u24::<BigEndian>(),
            false => self.read_u24::<LittleEndian>(),
        }
    }

    #[inline]
    fn read_u32_by(&mut self, endian: bool) -> Result<u32, std::io::Error> {
        match endian {
            true => self.read_u32::<BigEndian>(),
            false => self.read_u32::<LittleEndian>(),
        }
    }

    #[inline]
    fn read_u48_by(&mut self, endian: bool) -> Result<u64, std::io::Error> {
        match endian {
            true => self.read_u48::<BigEndian>(),
            false => self.read_u48::<LittleEndian>(),
        }
    }

    #[inline]
    fn read_u64_by(&mut self, endian: bool) -> Result<u64, std::io::Error> {
        match endian {
            true => self.read_u64::<BigEndian>(),
            false => self.read_u64::<LittleEndian>(),
        }
    }
}
