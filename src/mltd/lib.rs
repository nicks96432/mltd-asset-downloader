#![warn(clippy::print_stderr)]
#![warn(clippy::print_stdout)]

//! This module provides functions for downloading and parsing
//! MLTD asset manifests.

pub mod asset;
mod error;
pub mod extract;
pub mod manifest;
pub mod net;
pub mod util;

pub use self::error::Error;
