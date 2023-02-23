use mltd_asset_manifest::ManifestError;
use std::error::Error;
use std::fmt::{Display, Formatter, Result};
use std::io::Error as IOError;

#[derive(Debug)]
pub enum DownloadError {
    FileCreateFailed(IOError),
    ManifestError(ManifestError),
    ThreadPoolError,
}

impl Display for DownloadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::FileCreateFailed(e) => write!(f, "cannot create file or directory: {}", e),
            Self::ManifestError(e) => write!(f, "cannot get manifest: {}", e),
            Self::ThreadPoolError => write!(f, "failed to create thread pool"),
        }
    }
}

impl Error for DownloadError {}
