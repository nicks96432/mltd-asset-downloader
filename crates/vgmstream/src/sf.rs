use std::ffi::CString;
use std::marker::PhantomData;
use std::path::Path;

use crate::{Error, VgmStream};

pub struct StreamFile<'a> {
    pub(crate) inner: *mut vgmstream_sys::libstreamfile_t,

    phantom: PhantomData<&'a VgmStream>,
}

impl StreamFile<'_> {
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
    /// use vgmstream::{StreamFile, VgmStream};
    ///
    /// let vgmstream = VgmStream::new().unwrap();
    /// let stream = StreamFile::open(&vgmstream, "path/to/file").unwrap();
    /// ```
    pub fn open<P: AsRef<Path>>(_: &VgmStream, path: P) -> Result<Self, Error> {
        let path = CString::new(path.as_ref().to_string_lossy().as_ref()).unwrap();
        let inner = unsafe { vgmstream_sys::libstreamfile_open_from_stdio(path.as_ptr()) };

        if inner.is_null() {
            return Err(Error::InitializationFailed);
        }

        Ok(Self { inner, phantom: PhantomData })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/test.acb");

    #[test]
    fn test_streamfile() {
        let vgmstream = VgmStream::new().unwrap();
        let sf = StreamFile::open(&vgmstream, ACB_PATH).unwrap();
        assert!(!sf.inner.is_null());
    }
}
