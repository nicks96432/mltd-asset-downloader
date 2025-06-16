//! Encrypt and decrypt text assets in MLTD.

use std::sync::LazyLock;

use aes::Aes192;
use cbc::{Decryptor, Encryptor};
use cipher::block_padding::Pkcs7;
use cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use thiserror::Error as ThisError;

use crate::error::{Error, Repr, Result};

#[derive(Debug, ThisError)]
#[error("AES error: {kind}")]
pub(crate) struct AesError {
    pub kind: AesErrorKind,
    pub input: Vec<u8>,
}

impl AesError {
    pub fn unpad(input: Vec<u8>) -> Self {
        Self { kind: AesErrorKind::Unpad, input }
    }
}

impl From<AesError> for Error {
    fn from(value: AesError) -> Self {
        Repr::from(value).into()
    }
}

#[derive(Debug, ThisError)]
pub(crate) enum AesErrorKind {
    #[error("failed to unpad output")]
    Unpad,
}

/// The key used to derive [`TEXT_DECRYPT_KEY`] and
/// [`TEXT_DECRYPT_IV`].
pub const TEXT_PBKDF2_HMAC_SHA1_KEY: &[u8; 8] = b"Millicon";

/// The salt used to derive [`TEXT_DECRYPT_KEY`] and
/// [`TEXT_DECRYPT_IV`].
pub const TEXT_PBKDF2_HMAC_SHA1_SALT: &[u8; 9] = b"DAISUL___";

/// The number of iterations used to derive [`TEXT_DECRYPT_KEY`] and
/// [`TEXT_DECRYPT_IV`].
pub const TEXT_PBKDF2_HMAC_SHA1_ROUNDS: u32 = 1_000;

static TEXT_DECRYPT_KEY_IV: LazyLock<[u8; 40]> = LazyLock::new(|| {
    pbkdf2::pbkdf2_hmac_array::<sha1::Sha1, 40>(
        TEXT_PBKDF2_HMAC_SHA1_KEY,
        TEXT_PBKDF2_HMAC_SHA1_SALT,
        TEXT_PBKDF2_HMAC_SHA1_ROUNDS,
    )
});

/// The AES-192-CBC key used to decrypt the text asset.
///
/// It is derived from [`TEXT_PBKDF2_HMAC_SHA1_KEY`] and
/// [`TEXT_PBKDF2_HMAC_SHA1_SALT`] using PBKDF2-HMAC-SHA1, where
/// the first 24 bytes of the derived key are used as the actual key.
pub static TEXT_DECRYPT_KEY: LazyLock<[u8; 24]> =
    LazyLock::new(|| (&TEXT_DECRYPT_KEY_IV[0..24]).try_into().unwrap());

/// The AES-192-CBC initialization vector used to decrypt the text asset.
/// 
/// It is derived from [`TEXT_PBKDF2_HMAC_SHA1_KEY`] and
/// [`TEXT_PBKDF2_HMAC_SHA1_SALT`] using PBKDF2-HMAC-SHA1, where
/// the last 16 bytes of the derived key are used as the actual IV.
#[rustfmt::skip]
pub static TEXT_DECRYPT_IV: LazyLock<[u8; 16]> =
    LazyLock::new(|| (&TEXT_DECRYPT_KEY_IV[24..40]).try_into().unwrap());

/// AES-192-CBC encryptor for text assets in MLTD.
pub type TextEncryptor = Encryptor<Aes192>;

/// AES-192-CBC decryptor for text assets in MLTD.
pub type TextDecryptor = Decryptor<Aes192>;

/// File extensions for encrypted binary TextAssets in MLTD.
pub const ENCRYPTED_FILE_EXTENSIONS: &[&str; 2] = &["gtx", "mld"];

/// File extensions for unencrypted binary TextAssets in MLTD.
pub const UNENCRYPTED_FILE_EXTENSIONS: &[&str; 3] = &["acb", "awb", "mp4"];

/// Encrypts text using AES-192-CBC with MLTD's key and IV.
///
/// The input text is padded with PKCS7 padding.
///
/// # Example
///
/// ```
/// use mltd::extract::text::encrypt_text;
///
/// let text = b"Hello, world!";
/// let cipher = encrypt_text(text);
/// ```
pub fn encrypt_text(plaintext: &[u8]) -> Vec<u8> {
    let encryptor =
        TextEncryptor::new(TEXT_DECRYPT_KEY.as_ref().into(), TEXT_DECRYPT_IV.as_ref().into());

    encryptor.encrypt_padded_vec_mut::<Pkcs7>(plaintext)
}

/// Decrypts text using AES-192-CBC with MLTD's key and IV.
///
/// The output text is unpadded with PKCS7 padding.
///
/// # Errors
///
/// Returns [`crate::Error`] with [`crate::ErrorKind::Aes`] if decryption failed.
///
/// # Example
///
/// ```
/// use mltd::extract::text::decrypt_text;
///
/// let cipher = [
///     0xca, 0x64, 0x14, 0x8e, 0x80, 0x9e, 0x50, 0xc9, 0xe3, 0x4e, 0x18, 0x6f, 0x1e, 0x9c, 0x3e,
///     0xe2,
/// ];
/// let text = decrypt_text(&cipher).unwrap();
/// assert_eq!(b"Hello, world!", text.as_slice());
/// ```
pub fn decrypt_text(cipher: &[u8]) -> Result<Vec<u8>> {
    let decryptor =
        TextDecryptor::new(TEXT_DECRYPT_KEY.as_ref().into(), TEXT_DECRYPT_IV.as_ref().into());

    let plaintext = match decryptor.decrypt_padded_vec_mut::<Pkcs7>(cipher) {
        Ok(plaintext) => Ok(plaintext),
        Err(_) => Err(AesError::unpad(cipher.to_owned())),
    }?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use cipher::BlockSizeUser;

    use super::*;

    #[test]
    fn test_key_iv() {
        #[rustfmt::skip]
        let expected = [
            0xad, 0x3f, 0x0f, 0x89, 0xee, 0x51, 0xc5, 0x37,
            0x73, 0x1f, 0x17, 0x96, 0xf7, 0x5c, 0x71, 0x84,
            0x01, 0x61, 0x75, 0x6d, 0xa0, 0xd4, 0x86, 0xc9,
            0x4e, 0x40, 0xb3, 0x8a, 0xeb, 0xf1, 0xa8, 0x53,
            0x12, 0x2c, 0x5f, 0xad, 0xcc, 0xa3, 0x68, 0x5d,
        ];

        assert_eq!(TEXT_DECRYPT_KEY_IV.as_ref(), &expected);
    }

    #[test]
    fn test_encrypt_decrypt() {
        let expect = b"Hello, world!";
        let cipher = encrypt_text(expect);
        assert_eq!(cipher.len(), Aes192::block_size());

        let got = decrypt_text(&cipher).unwrap();
        assert_eq!(expect, got.as_slice());
    }
}
