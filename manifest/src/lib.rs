#![forbid(unsafe_code)]
#![warn(clippy::print_stderr)]
#![warn(clippy::print_stdout)]

//! `mltd manifest` subcommand

mod error;
mod manifest;
mod matsuri_api;

pub use self::error::*;
pub use self::manifest::*;
pub use self::matsuri_api::*;
