use anyhow::Context as _;
use aws_lc_rs::{
    aead::{
        self,
        Aad,
        LessSafeKey,
        Nonce,
        UnboundKey,
    },
    cipher::{
        self,
        DecryptionContext,
        EncryptingKey,
        EncryptionContext,
        PaddedBlockDecryptingKey,
        PaddedBlockEncryptingKey,
        UnboundCipherKey,
    },
    error::Unspecified,
};
use indexmap::IndexSet;
use serde::{
    Deserialize,
    Serialize,
};
use serde_bytes::ByteBuf;

use super::{
    check_usages_subset,
    ensure,
    CryptoKey,
    CryptoKeyKind,
    CryptoRng,
    DOMExceptionName,
    Error,
    ImportKeyInput,
    JsonWebKey,
    KeyData,
    KeyFormat,
    KeyType,
    KeyUsage,
    Result,
    URL_SAFE_FORGIVING,
};

const AES_BLOCK_SIZE: usize = 16;

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum AesAlgorithm {
    /// The "AES-CTR" algorithm identifier is used to perform encryption and
    /// decryption using AES in Counter mode, as described in NIST-SP800-38A.
    #[serde(rename = "AES-CTR")]
    AesCtr,
    /// The "AES-CBC" algorithm identifier is used to perform encryption and
    /// decryption using AES in Cipher Block Chaining mode, as described in
    /// NIST-SP800-38A.
    #[serde(rename = "AES-CBC")]
    AesCbc,
    /// The "AES-GCM" algorithm identifier is used to perform authenticated
    /// encryption and decryption using AES in Galois/Counter Mode mode, as
    /// described in NIST-SP800-38D.
    #[serde(rename = "AES-GCM")]
    AesGcm,
}

#[derive(Deserialize)]
pub struct AesCtrParams {
    /// The counter member contains the initial value of the counter block.
    /// counter MUST be 16 bytes (the AES block size). The counter bits are the
    /// rightmost length bits of the counter block. The rest of the counter
    /// block is for the nonce. The counter bits are incremented using the
    /// standard incrementing function specified in NIST SP 800-38A Appendix
    /// B.1: the counter bits are interpreted as a big-endian integer and
    /// incremented by one.
    pub counter: ByteBuf,
    /// The length member contains the length, in bits, of the rightmost part of
    /// the counter block that is incremented.
    pub length: u8,
}

#[derive(Deserialize)]
pub struct AesCbcParams {
    /// The iv member represents the initialization vector. It MUST be 16 bytes.
    pub iv: ByteBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AesGcmParams {
    /// The iv member represents the initialization vector to use. May be up to
    /// 2^64-1 bytes long.
    pub iv: ByteBuf,
    /// The additionalData member represents the additional authentication data
    /// to include.
    pub additional_data: Option<ByteBuf>,
    /// The tagLength member represents the desired length of the authentication
    /// tag. May be 0 - 128.
    pub tag_length: Option<u8>,
}

#[derive(Deserialize, Serialize)]
pub struct AesKeyAlgorithm {
    pub name: AesAlgorithm,
    /// The length member represents the length, in bits, of the key.
    pub length: u16,
}

impl AesKeyAlgorithm {
    fn jwk_alg(&self) -> String {
        match self.name {
            AesAlgorithm::AesCtr => format!("A{}CTR", self.length),
            AesAlgorithm::AesCbc => format!("A{}CBC", self.length),
            AesAlgorithm::AesGcm => format!("A{}GCM", self.length),
        }
    }
}

pub type AesKeyGenParams = AesKeyAlgorithm;
pub type AesDerivedKeyParams = AesKeyAlgorithm;

impl AesDerivedKeyParams {
    pub fn get_key_length(&self) -> Result<usize> {
        ensure!(
            [128, 192, 256].contains(&self.length),
            Error::dom(
                "AES key length must be 128, 192, or 256 bits",
                DOMExceptionName::OperationError,
            )
        );
        Ok(self.length as usize)
    }
}

pub struct AesKey {
    key: Vec<u8>,
}

pub fn import_key(
    format: ImportKeyInput,
    algorithm: AesAlgorithm,
    extractable: bool,
    usages: IndexSet<KeyUsage>,
) -> Result<CryptoKey> {
    let (data, algorithm) = match format {
        ImportKeyInput::Raw(data) => {
            let length = data.len() * 8;
            ensure!(
                [128, 192, 256].contains(&length),
                Error::dom("invalid key length", DOMExceptionName::DataError)
            );
            (
                data,
                AesKeyAlgorithm {
                    name: algorithm,
                    length: length as u16,
                },
            )
        },
        ImportKeyInput::Jwk(jwk) => {
            jwk.check_kty("oct")?;
            let data = jwk
                .k
                .as_ref()
                .and_then(|k| base64::decode_config(k, URL_SAFE_FORGIVING).ok())
                .ok_or_else(|| Error::dom("invalid key data", DOMExceptionName::DataError))?;
            let length = data.len() * 8;
            ensure!(
                [128, 192, 256].contains(&length),
                Error::dom("invalid key length", DOMExceptionName::DataError)
            );
            let algorithm = AesKeyAlgorithm {
                name: algorithm,
                length: length as u16,
            };
            jwk.check_alg(&algorithm.jwk_alg())?;
            jwk.check_key_ops_and_use(&usages, "enc")?;
            jwk.check_ext(extractable)?;
            (data, algorithm)
        },
        ImportKeyInput::Pkcs8(_) | ImportKeyInput::Spki(_) => {
            return Err(Error::dom(
                "unsupported import format",
                DOMExceptionName::NotSupportedError,
            ))
        },
    };
    Ok(CryptoKey {
        kind: CryptoKeyKind::Aes {
            algorithm,
            key: AesKey { key: data },
        },
        r#type: KeyType::Secret,
        extractable,
        usages,
    })
}

impl AesKey {
    fn aes_key(&self) -> Result<UnboundCipherKey> {
        let alg = match self.key.len() {
            16 => &cipher::AES_128,
            24 => &cipher::AES_192,
            32 => &cipher::AES_256,
            l => {
                return Err(Error::dom(
                    format!("unexpected key length {l}"),
                    DOMExceptionName::OperationError,
                ))
            },
        };
        Ok(UnboundCipherKey::new(alg, &self.key)?)
    }

    fn aes_gcm_key(&self) -> Result<LessSafeKey> {
        let alg = match self.key.len() {
            16 => &aead::AES_128_GCM,
            24 => &aead::AES_192_GCM,
            32 => &aead::AES_256_GCM,
            l => {
                return Err(Error::dom(
                    format!("unexpected key length {l}"),
                    DOMExceptionName::OperationError,
                ))
            },
        };
        let key = LessSafeKey::new(UnboundKey::new(alg, &self.key).context("invalid AES-GCM key")?);
        Ok(key)
    }

    pub fn export_key(&self, algorithm: &AesKeyAlgorithm, format: KeyFormat) -> Result<KeyData> {
        match format {
            KeyFormat::Raw => Ok(KeyData::Raw(self.key.clone())),
            KeyFormat::Jwk => {
                let jwk = JsonWebKey {
                    kty: Some("oct".to_owned()),
                    k: Some(base64::encode_config(&self.key, base64::URL_SAFE_NO_PAD)),
                    alg: Some(algorithm.jwk_alg()),
                    ..Default::default()
                };
                Ok(KeyData::Jwk(jwk))
            },
            KeyFormat::Pkcs8 | KeyFormat::Spki => Err(Error::dom(
                "unsupported export format",
                DOMExceptionName::NotSupportedError,
            )),
        }
    }

    pub fn crypt_ctr(
        &self,
        algorithm: AesCtrParams,
        key_algorithm: &AesKeyAlgorithm,
        mut data: Vec<u8>,
    ) -> Result<Vec<u8>> {
        ensure!(
            key_algorithm.name == AesAlgorithm::AesCtr,
            Error::dom(
                "invalid algorithm for key",
                DOMExceptionName::InvalidAccessError
            )
        );
        let AesCtrParams { counter, length } = algorithm;
        let Ok(counter) = <[u8; 16]>::try_from(&counter[..]) else {
            return Err(Error::dom(
                "counter must be 16 bytes",
                DOMExceptionName::OperationError,
            ));
        };
        ensure!(
            length > 0 && length <= 128,
            Error::dom("invalid counter length", DOMExceptionName::OperationError)
        );
        if let Some(limit) = AES_BLOCK_SIZE.checked_shl(length as u32)
            && data.len() > limit
        {
            return Err(Error::dom(
                "too much data for counter length",
                DOMExceptionName::OperationError,
            ));
        }
        let key = self.aes_key()?;
        let key = EncryptingKey::ctr(key)?;
        // WebCrypto's AES-CTR is defined to only increment the lowest `length`
        // bits of the counter, wrapping on overflow.
        // Most normal implementations of AES-CTR, including aws-lc-rs, use the
        // entire 128 bits of the counter as a big-endian integer (i.e.
        // length==128).
        // So we may need to do two AES-CTR operations to simulate the specified
        // overflow behaviour.
        //
        // counter_overflow_block_index is the minimum number such that `counter
        // + counter_overflow_block_index` changes the `length`th bit (counting
        // from LSB)
        if let Some(bit) = 1u128.checked_shl(length.into())
            && let counter_overflow_block_index = bit - (u128::from_be_bytes(counter) & (bit - 1))
            && let Ok(counter_overflow_block_index) = usize::try_from(counter_overflow_block_index)
            && let Some(counter_overflow_index) =
                counter_overflow_block_index.checked_mul(AES_BLOCK_SIZE)
            && counter_overflow_index < data.len()
        {
            key.less_safe_encrypt(
                &mut data[..counter_overflow_index],
                EncryptionContext::Iv128(counter.into()),
            )?;
            // simulate overflow
            let overflowed_counter = u128::from_be_bytes(counter) & !(bit - 1);
            key.less_safe_encrypt(
                &mut data[counter_overflow_index..],
                EncryptionContext::Iv128(overflowed_counter.to_be_bytes().into()),
            )?;
        } else {
            // overflow would not occur
            key.less_safe_encrypt(&mut data, EncryptionContext::Iv128(counter.into()))?;
        }
        Ok(data)
    }

    pub fn encrypt_cbc(
        &self,
        algorithm: AesCbcParams,
        key_algorithm: &AesKeyAlgorithm,
        mut data: Vec<u8>,
    ) -> Result<Vec<u8>> {
        ensure!(
            key_algorithm.name == AesAlgorithm::AesCbc,
            Error::dom(
                "invalid algorithm for key",
                DOMExceptionName::InvalidAccessError
            )
        );
        let AesCbcParams { iv } = algorithm;
        let Ok(iv) = <[u8; 16]>::try_from(&iv[..]) else {
            return Err(Error::dom(
                "iv must be 16 bytes",
                DOMExceptionName::OperationError,
            ));
        };
        let key = self.aes_key()?;
        let key = PaddedBlockEncryptingKey::cbc_pkcs7(key)?;
        key.less_safe_encrypt(&mut data, EncryptionContext::Iv128(iv.into()))?;
        Ok(data)
    }

    pub fn decrypt_cbc(
        &self,
        algorithm: AesCbcParams,
        key_algorithm: &AesKeyAlgorithm,
        mut data: Vec<u8>,
    ) -> Result<Vec<u8>> {
        ensure!(
            key_algorithm.name == AesAlgorithm::AesCbc,
            Error::dom(
                "invalid algorithm for key",
                DOMExceptionName::InvalidAccessError
            )
        );
        let AesCbcParams { iv } = algorithm;
        let Ok(iv) = <[u8; 16]>::try_from(&iv[..]) else {
            return Err(Error::dom(
                "iv must be 16 bytes",
                DOMExceptionName::OperationError,
            ));
        };
        let key = self.aes_key()?;
        let key = PaddedBlockDecryptingKey::cbc_pkcs7(key)?;
        let len = key
            .decrypt(&mut data, DecryptionContext::Iv128(iv.into()))
            .map_err(|_| Error::dom("invalid ciphertext", DOMExceptionName::OperationError))?
            .len();
        data.truncate(len);
        Ok(data)
    }

    pub fn encrypt_gcm(
        &self,
        algorithm: AesGcmParams,
        key_algorithm: &AesKeyAlgorithm,
        mut data: Vec<u8>,
    ) -> Result<Vec<u8>> {
        ensure!(
            key_algorithm.name == AesAlgorithm::AesGcm,
            Error::dom(
                "invalid algorithm for key",
                DOMExceptionName::InvalidAccessError
            )
        );
        let AesGcmParams {
            iv,
            additional_data,
            tag_length,
        } = algorithm;
        let key = self.aes_gcm_key()?;
        let alg = key.algorithm();
        // TODO: consider supporting shorter tag lengths (`ring` does not allow this)
        ensure!(
            tag_length.is_none_or(|l| usize::from(l) == 8 * alg.tag_len()),
            Error::dom(
                format!("tag length must be {} bits", 8 * alg.tag_len()),
                DOMExceptionName::NotSupportedError
            )
        );
        key.seal_in_place_append_tag(
            // TODO: consider supporting GHASH construction for nonces (`ring` does not
            // support this)
            Nonce::try_assume_unique_for_key(&iv).map_err(|_| {
                Error::dom(
                    format!("AES-GCM IV must be {} bits", 8 * alg.nonce_len()),
                    DOMExceptionName::NotSupportedError,
                )
            })?,
            Aad::from(additional_data.as_ref().map_or(&[][..], |b| b.as_ref())),
            &mut data,
        )
        .context("AES-GCM encryption failed")?;
        Ok(data)
    }

    pub fn decrypt_gcm(
        &self,
        algorithm: AesGcmParams,
        key_algorithm: &AesKeyAlgorithm,
        mut data: Vec<u8>,
    ) -> Result<Vec<u8>> {
        ensure!(
            key_algorithm.name == AesAlgorithm::AesGcm,
            Error::dom(
                "invalid algorithm for key",
                DOMExceptionName::InvalidAccessError
            )
        );
        let AesGcmParams {
            iv,
            additional_data,
            tag_length,
        } = algorithm;
        let key = self.aes_gcm_key()?;
        let alg = key.algorithm();
        // TODO: consider supporting shorter tag lengths (`ring` does not allow this)
        ensure!(
            tag_length.is_none_or(|l| usize::from(l) == 8 * alg.tag_len()),
            Error::dom(
                format!("tag length must be {} bits", 8 * alg.tag_len()),
                DOMExceptionName::NotSupportedError
            )
        );
        ensure!(
            data.len() >= alg.tag_len(),
            Error::dom(
                "The provided data is too small.",
                DOMExceptionName::OperationError
            )
        );
        let plaintext_len = key
            .open_in_place(
                Nonce::try_assume_unique_for_key(&iv).map_err(|_| {
                    Error::dom(
                        format!("AES-GCM IV must be {} bits", 8 * alg.nonce_len()),
                        DOMExceptionName::NotSupportedError,
                    )
                })?,
                Aad::from(additional_data.as_ref().map_or(&[][..], |b| b.as_ref())),
                &mut data,
            )
            .map_err(|Unspecified| {
                Error::dom("Decryption failed", DOMExceptionName::OperationError)
            })?
            .len();
        data.truncate(plaintext_len);
        Ok(data)
    }
}

pub fn generate_key(
    algorithm: AesKeyGenParams,
    _rng: &CryptoRng,
    extractable: bool,
    usages: IndexSet<KeyUsage>,
) -> Result<CryptoKey> {
    check_usages_subset(
        &usages,
        &[
            KeyUsage::Encrypt,
            KeyUsage::Decrypt,
            KeyUsage::WrapKey,
            KeyUsage::UnwrapKey,
        ],
    )?;
    let length = algorithm.get_key_length()?;
    let mut key_bytes = vec![0u8; length / 8];
    aws_lc_rs::rand::fill(&mut key_bytes)?;
    Ok(CryptoKey {
        kind: CryptoKeyKind::Aes {
            algorithm: AesKeyAlgorithm {
                name: algorithm.name,
                length: length as u16,
            },
            key: AesKey { key: key_bytes },
        },
        r#type: KeyType::Secret,
        extractable,
        usages,
    })
}
