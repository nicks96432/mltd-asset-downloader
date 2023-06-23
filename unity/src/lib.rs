// thiserror need these
#![feature(error_generic_member_access, provide_any)]

pub mod asset;
pub mod bundle;
pub mod class;
pub mod compression;
pub mod error;
pub mod macros;
pub mod utils;

pub(crate) mod traits;
