use aws_lc_rs::hmac;
use indexmap::IndexSet;
use serde::{
    Deserialize,
    Serialize,
};

use super::{
    check_usages_subset,
    CryptoHash,
    CryptoKey,
    CryptoKeyKind,
    ImportKeyInput,
    JsonWebKey,
    KeyData,
    KeyFormat,
    KeyType,
    KeyUsage,
    URL_SAFE_FORGIVING,
};
use crate::{
    convert_v8::{
        DOMException,
        DOMExceptionName,
        TypeError,
    },
    environment::crypto_rng::CryptoRng,
};

#[derive(Deserialize, Debug)]
pub(crate) struct HmacImportParams {
    /// The hash member represents the inner hash function to use.
    #[serde(with = "super::nullary_algorithm")]
    hash: CryptoHash,
    /// The length member represent the length (in bits) of the key.
    length: Option<u32>,
}

#[derive(Serialize)]
#[serde(tag = "name")]
#[serde(rename = "HMAC")]
pub(crate) struct HmacKeyAlgorithm {
    /// The hash member represents the inner hash function to use.
    #[serde(with = "super::nullary_algorithm")]
    hash: CryptoHash,
    /// The length member represent the length (in bits) of the key.
    length: u32,
}
fn jwk_alg(hash: &CryptoHash) -> &'static str {
    match hash {
        CryptoHash::Sha1 => "HS1",
        CryptoHash::Sha256 => "HS256",
        CryptoHash::Sha384 => "HS384",
        CryptoHash::Sha512 => "HS512",
    }
}

pub(crate) type HmacKeyGenParams = HmacImportParams;

pub(crate) struct HmacKey {
    data: Vec<u8>,
    key: hmac::Key,
}

pub(crate) fn generate_key(
    algorithm: HmacKeyGenParams,
    _rng: &CryptoRng,
    extractable: bool,
    usages: IndexSet<KeyUsage>,
) -> anyhow::Result<CryptoKey> {
    check_usages_subset(&usages, &[KeyUsage::Sign, KeyUsage::Verify])?;
    let length = algorithm.get_key_length()?;
    let mut key_bytes = vec![0u8; length.div_ceil(8)];
    aws_lc_rs::rand::fill(&mut key_bytes)?;
    Ok(CryptoKey {
        kind: CryptoKeyKind::Hmac {
            algorithm: HmacKeyAlgorithm {
                hash: algorithm.hash,
                length: length as u32,
            },
            key: HmacKey {
                key: hmac::Key::new(hmac_algorithm(algorithm.hash), &key_bytes),
                data: key_bytes,
            },
        },
        r#type: KeyType::Secret,
        extractable,
        usages,
    })
}

pub(crate) fn import_key(
    input: ImportKeyInput,
    algorithm: HmacImportParams,
    extractable: bool,
    usages: IndexSet<KeyUsage>,
) -> anyhow::Result<CryptoKey> {
    let data = match input {
        ImportKeyInput::Raw(data) => data,
        ImportKeyInput::Jwk(jwk) => {
            jwk.check_kty("oct")?;
            let data = jwk
                .k
                .as_ref()
                .and_then(|k| base64::decode_config(k, URL_SAFE_FORGIVING).ok())
                .ok_or_else(|| {
                    anyhow::anyhow!(DOMException::new(
                        "invalid key data",
                        DOMExceptionName::DataError
                    ))
                })?;
            jwk.check_alg(jwk_alg(&algorithm.hash))?;
            jwk.check_key_ops_and_use(&usages, "sig")?;
            jwk.check_ext(extractable)?;
            data
        },
        ImportKeyInput::Pkcs8(_) | ImportKeyInput::Spki(_) => {
            anyhow::bail!(DOMException::new(
                "unsupported import format",
                DOMExceptionName::NotSupportedError
            ))
        },
    };
    let mut length = data.len() * 8;
    anyhow::ensure!(
        length > 0,
        DOMException::new("provided HMAC key is empty", DOMExceptionName::DataError)
    );
    if algorithm.length.is_some() {
        let requested_len = algorithm.get_key_length()?;
        anyhow::ensure!(
            requested_len <= length,
            DOMException::new(
                "provided HMAC key is shorter than requested length",
                DOMExceptionName::DataError
            )
        );
        anyhow::ensure!(
            requested_len > length - 8,
            DOMException::new(
                "provided HMAC key is longer than requested length",
                DOMExceptionName::DataError
            )
        );
        length = requested_len;
    }
    Ok(CryptoKey {
        kind: CryptoKeyKind::Hmac {
            algorithm: HmacKeyAlgorithm {
                hash: algorithm.hash,
                length: length as u32,
            },
            key: HmacKey {
                key: hmac::Key::new(hmac_algorithm(algorithm.hash), &data),
                data,
            },
        },
        r#type: KeyType::Secret,
        extractable,
        usages,
    })
}

impl HmacKey {
    pub(crate) fn export_key(
        &self,
        algorithm: &HmacKeyAlgorithm,
        format: KeyFormat,
    ) -> anyhow::Result<KeyData> {
        match format {
            KeyFormat::Raw => Ok(KeyData::Raw(self.data.clone())),
            KeyFormat::Jwk => {
                let jwk = JsonWebKey {
                    kty: Some("oct".to_owned()),
                    k: Some(base64::encode_config(&self.data, base64::URL_SAFE_NO_PAD)),
                    alg: Some(jwk_alg(&algorithm.hash).into()),
                    ..Default::default()
                };
                Ok(KeyData::Jwk(jwk))
            },
            KeyFormat::Pkcs8 | KeyFormat::Spki => anyhow::bail!(DOMException::new(
                "unsupported export format",
                DOMExceptionName::NotSupportedError
            )),
        }
    }
}

impl HmacImportParams {
    pub(crate) fn get_key_length(&self) -> anyhow::Result<usize> {
        if let Some(length) = self.length {
            anyhow::ensure!(length > 0, TypeError::new("length must not be zero"));
            // The spec allows any bit length, but node.js, Deno, Bun, Firefox, and Safari
            // don't implement fractional byte lengths; only Chrome does.
            // Node raises a TypeError.
            anyhow::ensure!(
                length % 8 == 0,
                TypeError::new("length must be a multiple of 8")
            );
            Ok(length as usize)
        } else {
            Ok(self.hash.block_size_bits())
        }
    }
}

fn hmac_algorithm(hash: CryptoHash) -> hmac::Algorithm {
    match hash {
        CryptoHash::Sha1 => hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
        CryptoHash::Sha256 => hmac::HMAC_SHA256,
        CryptoHash::Sha384 => hmac::HMAC_SHA384,
        CryptoHash::Sha512 => hmac::HMAC_SHA512,
    }
}

impl HmacKey {
    pub(crate) fn sign(&self, _algorithm: &HmacKeyAlgorithm, data: &[u8]) -> Vec<u8> {
        hmac::sign(&self.key, data).as_ref().to_vec()
    }

    pub(crate) fn verify(
        &self,
        _algorithm: &HmacKeyAlgorithm,
        data: &[u8],
        signature: &[u8],
    ) -> bool {
        hmac::verify(&self.key, data, signature).is_ok()
    }
}
