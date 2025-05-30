use std::io::{
    Cursor,
    Read as _,
};

use anyhow::Context;
use aws_lc_rs::{
    aead,
    kdf,
    rand::{
        SecureRandom,
        SystemRandom,
    },
};
use byteorder::ReadBytesExt;
use prost::Message;

use crate::Secret;

const AEAD_ALGORITHM: aead::Algorithm = aead::AES_128_GCM_SIV;
const KEY_LEN: usize = 16;
#[test]
fn test_key_len() {
    assert_eq!(KEY_LEN, AEAD_ALGORITHM.key_len());
}

#[derive(Clone)]
pub struct Encryptor<const DETERMINISTIC: bool> {
    derived_key: [u8; KEY_LEN],
}
pub type RandomEncryptor = Encryptor<false>;
pub type DeterministicEncryptor = Encryptor<true>;

// These are arbitrary strings; it's only important that we never reuse the
// exact same string for two different logical purposes.
pub struct Purpose<const DETERMINISTIC: bool = false>(&'static str);
pub type DeterministicPurpose = Purpose<true>;
impl Purpose {
    pub const ACTION_CALLBACK_TOKEN: Purpose = Purpose("action callback token");
    pub const ADMIN_KEY: Purpose = Purpose("admin key");
    /// Cursors are issued in UDFs and are also fed back as arguments. As such
    /// we want them to be deterministic to avoid breaking caching.
    /// These do not need to be secret in the first place - only tamper-proof.
    pub const CURSOR: DeterministicPurpose = Purpose("cursor");
    pub const QUERY_JOURNAL: Purpose = Purpose("query journal");
    pub const STORE_FILE_AUTHORIZATION: Purpose = Purpose("store file authorization");
}

impl<const DETERMINISTIC: bool> Purpose<DETERMINISTIC> {
    fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

const KDF_ALGORITHM: &kdf::KbkdfCtrHmacAlgorithm =
    kdf::get_kbkdf_ctr_hmac_algorithm(kdf::KbkdfCtrHmacAlgorithmId::Sha256).unwrap();

impl<const DETERMINISTIC: bool> Encryptor<DETERMINISTIC> {
    pub fn derive_from_secret(
        secret: &Secret,
        purpose: Purpose<DETERMINISTIC>,
    ) -> anyhow::Result<Self> {
        let mut derived_key = [0; KEY_LEN];
        kdf::kbkdf_ctr_hmac(
            KDF_ALGORITHM,
            secret.as_bytes(),
            purpose.as_bytes(),
            &mut derived_key,
        )
        .context("KBKDF failed")?;
        Ok(Self { derived_key })
    }

    // TODO: do not send instance secrets to funrun, only derived keys
    #[allow(unused)]
    pub fn derived_key(&self) -> [u8; KEY_LEN] {
        self.derived_key
    }

    #[allow(unused)]
    pub fn from_derived_key(derived_key: [u8; KEY_LEN]) -> Self {
        Self { derived_key }
    }

    fn key(&self) -> aead::LessSafeKey {
        aead::LessSafeKey::new(
            aead::UnboundKey::new(&AEAD_ALGORITHM, &self.derived_key)
                .expect("KEY_LEN == AEAD_ALGORITHM.key_len()"),
        )
    }

    pub fn encrypt_proto<T: Message>(&self, version: u8, message: &T) -> String {
        let mut nonce = [0; aead::NONCE_LEN];
        // N.B.: AES-GCM-SIV is "nonce-misuse-resistant". When
        // DETERMINISTIC=true we intentionally "misuse" it by using a constant
        // nonce for all messages. This does not break the encryption (unlike
        // AES-GCM) but merely leaks whether messages are identical. That is,
        // anyone can tell whether two encrypted messages correspond to the same
        // plaintext - which is exactly what we want from deterministic
        // encryption.
        if !DETERMINISTIC {
            SystemRandom::new()
                .fill(&mut nonce)
                .expect("SystemRandom failed");
        }
        let mut encoded_message = message.encode_to_vec();
        let tag = self
            .key()
            .seal_in_place_separate_tag(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(&[version]),
                &mut encoded_message,
            )
            .expect("encryption failed");

        let mut buffer = Vec::with_capacity(
            1 + if DETERMINISTIC { 0 } else { nonce.len() }
                + encoded_message.len()
                + AEAD_ALGORITHM.tag_len(),
        );
        buffer.push(version);
        if !DETERMINISTIC {
            buffer.extend_from_slice(&nonce);
        }
        buffer.extend_from_slice(&encoded_message);
        buffer.extend_from_slice(tag.as_ref());
        hex::encode(buffer)
    }

    pub fn decrypt_proto<M: Default + Message>(
        &self,
        version: u8,
        encoded: &str,
    ) -> anyhow::Result<M> {
        let mut bytes = hex::decode(encoded)?;
        let mut reader = Cursor::new(&bytes[..]);
        let message_version = reader.read_u8()?;
        if message_version != version {
            anyhow::bail!("Invalid message version {}", message_version);
        }
        let mut nonce = [0; aead::NONCE_LEN];
        if !DETERMINISTIC {
            reader.read_exact(&mut nonce)?;
        }
        let pos = reader.position() as usize;
        let ciphertext_and_tag = &mut bytes[pos..];
        let plaintext = self
            .key()
            .open_in_place(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(&[version]),
                ciphertext_and_tag,
            )
            .map_err(|_| anyhow::anyhow!("Failed to decrypt ciphertext"))?;
        Ok(M::decode(&*plaintext)?)
    }
}

#[test]
fn test_encryptor() {
    use common::testing::assert_contains;

    let secret = Secret::random();
    let encryptor = RandomEncryptor::derive_from_secret(&secret, Purpose("testing")).unwrap();
    let message = "very cool message".to_owned();
    let encoded = encryptor.encrypt_proto(11, &message);
    // RandomEncryptor is nondeterministic
    assert_ne!(encoded, encryptor.encrypt_proto(11, &message));
    assert_eq!(
        encryptor.decrypt_proto::<String>(11, &encoded).unwrap(),
        message
    );
    // decrypting with the wrong version should fail
    assert_contains(
        &encryptor.decrypt_proto::<String>(12, &encoded).unwrap_err(),
        "Invalid message version",
    );

    // An encryptor with a different purpose should not recognize the message
    let encryptor2 = RandomEncryptor::derive_from_secret(&secret, Purpose("testing2")).unwrap();
    assert_contains(
        &encryptor2
            .decrypt_proto::<String>(11, &encoded)
            .unwrap_err(),
        "Failed to decrypt",
    );
}

#[test]
fn test_deterministic_encryptor() {
    use common::testing::assert_contains;

    let secret = Secret::random();
    let encryptor =
        DeterministicEncryptor::derive_from_secret(&secret, Purpose("testing")).unwrap();
    let message = "very cool message".to_owned();
    let encoded = encryptor.encrypt_proto(11, &message);
    assert_eq!(encoded, encryptor.encrypt_proto(11, &message));
    assert_eq!(
        encryptor.decrypt_proto::<String>(11, &encoded).unwrap(),
        message
    );
    // decrypting with the wrong version should fail
    assert_contains(
        &encryptor.decrypt_proto::<String>(12, &encoded).unwrap_err(),
        "Invalid message version",
    );

    // An encryptor with a different purpose should not recognize the message
    let encryptor2 =
        DeterministicEncryptor::derive_from_secret(&secret, Purpose("testing2")).unwrap();
    assert_contains(
        &encryptor2
            .decrypt_proto::<String>(11, &encoded)
            .unwrap_err(),
        "Failed to decrypt",
    );
}
