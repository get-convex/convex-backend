//! Bindings for libsodium's secretbox_xsalsa20poly1305 construction

use std::os::raw::c_ulonglong;

use libsodium_sys::{
    crypto_secretbox_KEYBYTES,
    crypto_secretbox_MACBYTES,
    crypto_secretbox_NONCEBYTES,
    crypto_secretbox_easy,
    crypto_secretbox_open_easy,
    randombytes_buf,
    sodium_init,
};

pub const KEYBYTES: usize = crypto_secretbox_KEYBYTES as usize;
#[derive(Clone)]
pub struct Key([u8; KEYBYTES]);
impl Key {
    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        Some(Key(bytes.try_into().ok()?))
    }
}

pub const NONCEBYTES: usize = crypto_secretbox_NONCEBYTES as usize;
pub struct Nonce(pub [u8; NONCEBYTES]);
impl Nonce {
    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        Some(Nonce(bytes.try_into().ok()?))
    }
}

fn init() {
    unsafe {
        assert_ne!(sodium_init(), -1, "libsodium failed to initialize");
    }
}

pub fn gen_nonce() -> Nonce {
    init();
    unsafe {
        let mut nonce = Nonce([0; NONCEBYTES]);
        randombytes_buf(nonce.0.as_mut_ptr().cast(), NONCEBYTES);
        nonce
    }
}

pub fn seal(plaintext: &[u8], nonce: &Nonce, secret: &Key) -> Vec<u8> {
    init();
    unsafe {
        let mut ciphertext = vec![0; plaintext.len() + crypto_secretbox_MACBYTES as usize];
        assert_eq!(
            crypto_secretbox_easy(
                ciphertext.as_mut_ptr(),
                plaintext.as_ptr(),
                plaintext.len() as c_ulonglong,
                nonce.0.as_ptr(),
                secret.0.as_ptr(),
            ),
            0,
            "crypto_secretbox_easy failed"
        );
        ciphertext
    }
}

/// Indicates that decryption failed for an unspecified reason.
pub struct OpenError;
pub fn open(ciphertext: &[u8], nonce: &Nonce, secret: &Key) -> Result<Vec<u8>, OpenError> {
    init();
    unsafe {
        let mut plaintext = vec![
            0;
            ciphertext
                .len()
                .checked_sub(crypto_secretbox_MACBYTES as usize)
                .ok_or(OpenError)?
        ];
        if crypto_secretbox_open_easy(
            plaintext.as_mut_ptr(),
            ciphertext.as_ptr(),
            ciphertext.len() as c_ulonglong,
            nonce.0.as_ptr(),
            secret.0.as_ptr(),
        ) != 0
        {
            return Err(OpenError);
        }
        Ok(plaintext)
    }
}
