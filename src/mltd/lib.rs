//! Functions for downloading and parsing MLTD assets.
//!
//! [![github]](https://github.com/nicks96432/mltd-asset-downloader)
//!
//! [github]: https://img.shields.io/badge/github-333333?style=for-the-badge&labelColor=555555&logo=github

#![warn(unsafe_code, clippy::all, clippy::pedantic)]
#![allow(
    clippy::uninlined_format_args // I think uninlined args are more readable
)]

pub mod asset;
mod error;

#[cfg(feature = "extract")]
pub mod extract;

pub mod manifest;
pub mod net;
pub mod unity;
pub mod util;

pub use self::error::{Error, ErrorKind};
