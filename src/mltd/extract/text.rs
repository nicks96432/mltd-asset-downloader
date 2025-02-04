//! Encrypt and decrypt text assets in MLTD.

use aes::cipher::block_padding::Pkcs7;
use aes::cipher::inout::InOutBufReserved;
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use aes::Aes192;
use cbc::{Decryptor, Encryptor};

use crate::Error;

/// The key used to derive [`MLTD_TEXT_DECRYPT_KEY`] and
/// [`MLTD_TEXT_DECRYPT_IV`].
pub const MLTD_TEXT_PBKDF2_HMAC_SHA1_KEY: &[u8; 8] = b"Millicon";

/// The salt used to derive [`MLTD_TEXT_DECRYPT_KEY`] and
/// [`MLTD_TEXT_DECRYPT_IV`].
pub const MLTD_TEXT_PBKDF2_HMAC_SHA1_SALT: &[u8; 9] = b"DAISUL___";

/// The number of iterations used to derive [`MLTD_TEXT_DECRYPT_KEY`] and
/// [`MLTD_TEXT_DECRYPT_IV`].
pub const MLTD_TEXT_PBKDF2_HMAC_SHA1_ROUNDS: u32 = 1000;

/// The AES-192-CBC key used to decrypt the text asset.
/// 
/// It is derived from [`MLTD_TEXT_PBKDF2_HMAC_SHA1_KEY`] and
/// [`MLTD_TEXT_PBKDF2_HMAC_SHA1_SALT`] using PBKDF2-HMAC-SHA1, where
/// the first 24 bytes of the derived key are used as the actual key.
#[rustfmt::skip]
pub const MLTD_TEXT_DECRYPT_KEY: &[u8; 24] = &[
    0xad, 0x3f, 0x0f, 0x89, 0xee, 0x51, 0xc5, 0x37,
    0x73, 0x1f, 0x17, 0x96, 0xf7, 0x5c, 0x71, 0x84,
    0x01, 0x61, 0x75, 0x6d, 0xa0, 0xd4, 0x86, 0xc9,
];

/// The AES-192-CBC initialization vector used to decrypt the text asset.
/// 
/// It is derived from [`MLTD_TEXT_PBKDF2_HMAC_SHA1_KEY`] and
/// [`MLTD_TEXT_PBKDF2_HMAC_SHA1_SALT`] using PBKDF2-HMAC-SHA1, where
/// the last 16 bytes of the derived key are used as the actual IV.
#[rustfmt::skip]
pub const MLTD_TEXT_DECRYPT_IV: &[u8; 16] = &[
    0x4e, 0x40, 0xb3, 0x8a, 0xeb, 0xf1, 0xa8, 0x53,
    0x12, 0x2c, 0x5f, 0xad, 0xcc, 0xa3, 0x68, 0x5d,
];

/// AES-192-CBC encryptor for text assets in MLTD.
pub type MltdTextEncryptor = Encryptor<Aes192>;

/// AES-192-CBC decryptor for text assets in MLTD.
pub type MltdTextDecryptor = Decryptor<Aes192>;

/// Encrypts text using AES-192-CBC with MLTD's key and IV.
///
/// The input text is padded with PKCS7 padding.
///
/// # Errors
///
/// [`Error::Aes`]: if encryption failed.
///
/// # Example
///
/// ```no_run
/// use mltd::extract::text::encrypt_text;
///
/// let text = b"Hello, world!";
/// let cipher = encrypt_text(text).unwrap();
/// ```
pub fn encrypt_text(text: &[u8]) -> Result<Vec<u8>, Error> {
    let encryptor =
        MltdTextEncryptor::new(MLTD_TEXT_DECRYPT_KEY.into(), MLTD_TEXT_DECRYPT_IV.into());
    let mut buf = text.to_owned();

    let buf = InOutBufReserved::from_mut_slice(&mut buf, text.len())
        .map_err(|e| Error::Aes(e.to_string()))?;
    let buf =
        encryptor.encrypt_padded_inout_mut::<Pkcs7>(buf).map_err(|e| Error::Aes(e.to_string()))?;

    Ok(buf.to_owned())
}

/// Decrypts text using AES-192-CBC with MLTD's key and IV.
///
/// The output text is unpadded with PKCS7 padding.
///
/// # Errors
///
/// [`Error::Aes`]: if decryption failed.
///
/// # Example
///
/// ```no_run
/// use mltd::extract::text::decrypt_text;
///
/// let cipher = b"Hello, world!";
/// let text = decrypt_text(cipher).unwrap();
/// ```
pub fn decrypt_text(cipher: &[u8]) -> Result<Vec<u8>, Error> {
    let decryptor =
        MltdTextDecryptor::new(MLTD_TEXT_DECRYPT_KEY.into(), MLTD_TEXT_DECRYPT_IV.into());
    let mut buf = cipher.to_owned();

    let buf = decryptor
        .decrypt_padded_inout_mut::<Pkcs7>(buf.as_mut_slice().into())
        .map_err(|e| Error::Aes(e.to_string()))?;

    Ok(buf.to_owned())
}
