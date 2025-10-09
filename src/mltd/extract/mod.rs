//! Functions to extract assets from MLTD.

#[cfg(feature = "decode-audio")]
pub mod audio;

#[cfg(feature = "process-image")]
pub mod puzzle;

#[cfg(feature = "decrypt-text")]
pub mod text;

/// File extensions for encrypted text assets in MLTD.
pub const ENCRYPTED_TEXT_EXTENSIONS: &[&str; 2] = &["gtx", "mld"];
