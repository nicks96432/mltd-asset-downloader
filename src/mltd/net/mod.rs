//! Functions related to network requests.

#[cfg(feature = "download")]
pub mod matsuri_api;

use thiserror::Error as ThisError;

pub use self::matsuri_api::*;
use crate::error::Repr;

#[derive(Debug, ThisError)]
#[error("network error: {kind}")]
pub(crate) struct Error {
    pub kind: ErrorKind,
    pub url: reqwest::Url,
    pub source: Option<reqwest::Error>,
}

#[derive(Debug, ThisError)]
pub(crate) enum ErrorKind {
    #[error("failed to send request")]
    Request,

    #[error("failed to decode response body")]
    Decode,
}

impl Error {
    pub const fn request(url: reqwest::Url, source: Option<reqwest::Error>) -> Self {
        Self { kind: ErrorKind::Request, url, source }
    }

    pub const fn decode(url: reqwest::Url, source: Option<reqwest::Error>) -> Self {
        Self { kind: ErrorKind::Decode, url, source }
    }
}

impl From<Error> for crate::Error {
    fn from(err: Error) -> Self {
        Repr::from(err).into()
    }
}
