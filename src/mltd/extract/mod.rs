//! Functions to extract assets from MLTD.

#[cfg(feature = "extract")]
pub mod pipeline;

#[cfg(feature = "process-image")]
pub mod puzzle;

/// File extensions for audio assets in MLTD, which are decoded after all the files are extracted.
pub const AUDIO_EXTENSIONS: &[&str; 2] = &["acb", "awb"];

/// File extensions for encrypted text assets in MLTD.
pub const ENCRYPTED_TEXT_EXTENSIONS: &[&str; 2] = &["gtx", "mld"];
