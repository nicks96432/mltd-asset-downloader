use crate::error::UnityError;
use byteorder::ReadBytesExt;
use std::io::Result as IOResult;
use std::io::{Read, Seek, SeekFrom, Write};

/// Extends [`Read`] with methods for reading exact number of bytes.
pub trait ReadExact: Read {
    /// Read the exact number of bytes.
    ///
    /// # Errors
    ///
    /// This function will return [`UnityError::FileError`] if the reader is unavailable.
    fn read_exact_bytes<const N: usize>(&mut self) -> IOResult<[u8; N]>;

    /// Reads a null-terminated string.
    ///
    /// # Errors
    ///
    /// This function will return [`UnityError::FileError`] if the reader is unavailable.
    fn read_string(&mut self) -> IOResult<String>;
}

impl<R: Read> ReadExact for R {
    #[inline]
    fn read_exact_bytes<const SIZE: usize>(&mut self) -> IOResult<[u8; SIZE]> {
        let mut buf = [0u8; SIZE];
        self.read_exact(&mut buf)?;

        Ok(buf)
    }

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
pub trait SeekAlign: Seek {
    /// Seeks to alignment.
    ///
    /// # Errors
    ///
    /// This function will return [`UnityError::FileError`]
    /// if the reader is unavailable.
    ///
    /// This function will return [`UnityError::TryFromIntError`]
    /// if integer conversion is failed.
    #[inline]
    fn seek_align(&mut self, alignment: u64) -> Result<(), UnityError> {
        let pos = i64::try_from(self.stream_position()?)?;
        let alignment = i64::try_from(alignment)?;
        let align = (alignment - pos % alignment) % alignment;
        self.seek(SeekFrom::Current(align))?;

        Ok(())
    }
}

impl<R: Seek> SeekAlign for R {}

pub(crate) trait UnityIO: Sized {
    /// Reads the struct from `reader`, assuming that the data start
    /// from current position.
    ///
    /// # Errors
    ///
    /// This function will return [`UnityError::FileError`] if `reader`
    /// is unavailable.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::io::Cursor;
    /// use unity::AssetBlockInfo;
    ///
    /// let mut file = Cursor::new(vec![0u8; 10]);
    /// let header = AssetBlockInfo::from_reader(&mut file).unwrap();
    ///
    /// let decompressed_size = header.decompressed_size;
    /// assert_eq!(decompressed_size, 0);
    /// ```
    fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, UnityError>;

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), UnityError>;
}
