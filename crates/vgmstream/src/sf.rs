use std::ffi::CString;
use std::path::Path;
use std::ptr::NonNull;

use crate::Error;

pub struct StreamFile {
    pub(crate) inner: NonNull<vgmstream_sys::libstreamfile_t>,
}

impl StreamFile {
    /// Creates a new `libstreamfile_t`.
    ///
    /// # Errors
    ///
    /// Returns `Error::InitializationFailed` if the initialization failed.
    ///
    /// # Example
    ///
    /// Initialize libvgmstream:
    ///
    /// ```no_run
    /// use vgmstream::StreamFile;
    ///
    /// let stream = StreamFile::open(&vgmstream, "path/to/file").unwrap();
    /// ```
    pub fn open<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let path = CString::new(path.as_ref().to_string_lossy().as_ref()).unwrap();
        let inner = unsafe { vgmstream_sys::libstreamfile_open_from_stdio(path.as_ptr()) };

        if inner.is_null() {
            return Err(Error::VgmStream("libstreamfile_open_from_stdio".to_string()));
        }

        Ok(Self { inner: unsafe { NonNull::new_unchecked(inner) } })
    }

    pub fn buffered(mut self) -> Result<Self, Error> {
        let inner = unsafe { vgmstream_sys::libstreamfile_open_buffered(self.inner.as_ptr()) };
        if inner.is_null() {
            return Err(Error::VgmStream("libstreamfile_open_buffered".to_string()));
        }

        self.inner = unsafe { NonNull::new_unchecked(inner) };

        Ok(self)
    }
}

impl Drop for StreamFile {
    fn drop(&mut self) {
        unsafe { self.inner.as_ref().close.unwrap()(self.inner.as_ptr()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/test.acb");

    #[test]
    fn test_streamfile() {
        StreamFile::open(ACB_PATH).unwrap().buffered().unwrap();
    }
}
