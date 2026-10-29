//! Decrypts unpacked assets.

#[cfg(feature = "decode-audio")]
pub mod audio;
#[cfg(feature = "decrypt-text")]
pub mod text;

/// File extensions for audio assets in MLTD, which are decoded after all the files are unpacked.
pub const AUDIO_EXTENSIONS: &[&str; 2] = &["acb", "awb"];

/// File extensions for encrypted text assets in MLTD.
pub const ENCRYPTED_TEXT_EXTENSIONS: &[&str; 2] = &["gtx", "mld"];
