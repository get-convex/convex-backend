// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
// https://github.com/denoland/deno/blob/main/ext/crypto/key.rs

mod ed25519;
mod import_key;
mod shared;
mod x25519;

use std::num::NonZeroU32;

use anyhow::Context;
use common::runtime::Runtime;
use deno_core::{
    JsBuffer,
    ToJsBuffer,
};
use rand::Rng;
use ring::{
    agreement::Algorithm as RingAlgorithm,
    digest,
    hmac::{
        Algorithm as HmacAlgorithm,
        Key as HmacKey,
    },
    pbkdf2,
    signature::{
        EcdsaSigningAlgorithm,
        EcdsaVerificationAlgorithm,
    },
};
use serde::{
    Deserialize,
    Serialize,
};
use uuid::Uuid;

use self::import_key::{
    ImportKeyOptions,
    ImportKeyResult,
};
use crate::{
    environment::IsolateEnvironment,
    execution_scope::ExecutionScope,
};

impl<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> ExecutionScope<'a, 'b, RT, E> {
    #[convex_macro::v8_op]
    pub fn op_crypto_randomUUID(&mut self) -> anyhow::Result<String> {
        let state = self.state_mut();
        let rng = state.environment.rng()?;
        let uuid = CryptoOps::random_uuid(rng)?;
        Ok(uuid.to_string())
    }

    #[convex_macro::v8_op]
    pub fn op_crypto_getRandomValues(&mut self, byte_length: u32) -> anyhow::Result<ToJsBuffer> {
        let state = self.state_mut();
        let rng = state.environment.rng()?;
        let bytes = CryptoOps::get_random_values(rng, byte_length)?;

        Ok(bytes.into())
    }

    #[convex_macro::v8_op]
    pub fn op_crypto_sign(
        &mut self,
        CryptoSignArgs {
            key,
            algorithm,
            hash,
            data,
        }: CryptoSignArgs,
    ) -> anyhow::Result<ToJsBuffer> {
        let signature = CryptoOps::sign(&key, &data, algorithm, hash)?;
        Ok(signature.into())
    }

    #[convex_macro::v8_op]
    pub fn op_crypto_verify(
        &mut self,
        CryptoVerifyArgs {
            key,
            algorithm,
            hash,
            signature,
            data,
        }: CryptoVerifyArgs,
    ) -> anyhow::Result<bool> {
        CryptoOps::verify(&key, &data, &signature, algorithm, hash)
    }

    #[convex_macro::v8_op]
    pub fn op_crypto_deriveBits(
        &mut self,
        arg: DeriveKeyArg,
        salt: Option<JsBuffer>,
    ) -> anyhow::Result<ToJsBuffer> {
        CryptoOps::derive_bits(arg, salt)
    }

    #[convex_macro::v8_op]
    pub fn op_crypto_digest(
        &mut self,
        algorithm: CryptoHash,
        data: JsBuffer,
    ) -> anyhow::Result<ToJsBuffer> {
        CryptoOps::subtle_digest(algorithm, data)
    }

    #[convex_macro::v8_op]
    pub fn op_crypto_importKey(
        &mut self,
        opts: ImportKeyOptions,
        key_data: import_key::KeyData,
    ) -> anyhow::Result<ImportKeyResult> {
        CryptoOps::import_key(opts, key_data)
    }

    #[convex_macro::v8_op]
    pub fn op_crypto_import_spki_ed25519(
        &mut self,
        key_data: JsBuffer,
    ) -> anyhow::Result<Option<ToJsBuffer>> {
        Ok(CryptoOps::import_spki_ed25519(key_data))
    }

    #[convex_macro::v8_op]
    pub fn op_crypto_import_pkcs8_ed25519(
        &mut self,
        key_data: JsBuffer,
    ) -> anyhow::Result<Option<ToJsBuffer>> {
        Ok(CryptoOps::import_pkcs8_ed25519(key_data))
    }

    #[convex_macro::v8_op]
    pub fn op_crypto_import_spki_x25519(
        &mut self,
        key_data: JsBuffer,
    ) -> anyhow::Result<Option<ToJsBuffer>> {
        Ok(CryptoOps::import_spki_x25519(key_data))
    }

    #[convex_macro::v8_op]
    pub fn op_crypto_import_pkcs8_x25519(
        &mut self,
        key_data: JsBuffer,
    ) -> anyhow::Result<Option<ToJsBuffer>> {
        Ok(CryptoOps::import_pkcs8_x25519(key_data))
    }

    #[convex_macro::v8_op]
    pub fn op_crypto_base64_url_decode(&mut self, data: String) -> anyhow::Result<ToJsBuffer> {
        let data: Vec<u8> = base64::decode_config(data, base64::URL_SAFE_NO_PAD)?;
        Ok(data.into())
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CryptoSignArgs {
    pub key: JsBuffer,
    pub algorithm: Algorithm,
    pub hash: Option<CryptoHash>,
    pub data: JsBuffer,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CryptoVerifyArgs {
    pub key: JsBuffer,
    pub algorithm: Algorithm,
    pub hash: Option<CryptoHash>,
    pub signature: JsBuffer,
    pub data: JsBuffer,
}

#[derive(Serialize, Deserialize, Copy, Clone, Eq, PartialEq, Debug)]
pub enum CryptoHash {
    #[serde(rename = "SHA-1")]
    Sha1,
    #[serde(rename = "SHA-256")]
    Sha256,
    #[serde(rename = "SHA-384")]
    Sha384,
    #[serde(rename = "SHA-512")]
    Sha512,
}

impl From<CryptoHash> for HmacAlgorithm {
    fn from(hash: CryptoHash) -> HmacAlgorithm {
        match hash {
            CryptoHash::Sha1 => ring::hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
            CryptoHash::Sha256 => ring::hmac::HMAC_SHA256,
            CryptoHash::Sha384 => ring::hmac::HMAC_SHA384,
            CryptoHash::Sha512 => ring::hmac::HMAC_SHA512,
        }
    }
}

impl From<CryptoHash> for &'static digest::Algorithm {
    fn from(hash: CryptoHash) -> &'static digest::Algorithm {
        match hash {
            CryptoHash::Sha1 => &digest::SHA1_FOR_LEGACY_USE_ONLY,
            CryptoHash::Sha256 => &digest::SHA256,
            CryptoHash::Sha384 => &digest::SHA384,
            CryptoHash::Sha512 => &digest::SHA512,
        }
    }
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub enum CryptoNamedCurve {
    #[serde(rename = "P-256")]
    P256,
    #[serde(rename = "P-384")]
    P384,
}

impl From<CryptoNamedCurve> for &RingAlgorithm {
    fn from(curve: CryptoNamedCurve) -> &'static RingAlgorithm {
        match curve {
            CryptoNamedCurve::P256 => &ring::agreement::ECDH_P256,
            CryptoNamedCurve::P384 => &ring::agreement::ECDH_P384,
        }
    }
}

impl From<CryptoNamedCurve> for &EcdsaSigningAlgorithm {
    fn from(curve: CryptoNamedCurve) -> &'static EcdsaSigningAlgorithm {
        match curve {
            CryptoNamedCurve::P256 => &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            CryptoNamedCurve::P384 => &ring::signature::ECDSA_P384_SHA384_FIXED_SIGNING,
        }
    }
}

impl From<CryptoNamedCurve> for &EcdsaVerificationAlgorithm {
    fn from(curve: CryptoNamedCurve) -> &'static EcdsaVerificationAlgorithm {
        match curve {
            CryptoNamedCurve::P256 => &ring::signature::ECDSA_P256_SHA256_FIXED,
            CryptoNamedCurve::P384 => &ring::signature::ECDSA_P384_SHA384_FIXED,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyType {
    Secret,
    Private,
    Public,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct KeyData {
    // r#type: KeyType,
    data: JsBuffer,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeriveKeyArg {
    key: KeyData,
    algorithm: Algorithm,
    hash: Option<CryptoHash>,
    length: usize,
    iterations: Option<u32>,
    // ECDH
    // public_key: Option<KeyData>,
    // named_curve: Option<CryptoNamedCurve>,
    // HKDF
    // info: Option<JsBuffer>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum Algorithm {
    #[serde(rename = "RSASSA-PKCS1-v1_5")]
    RsassaPkcs1v15,
    #[serde(rename = "RSA-PSS")]
    RsaPss,
    #[serde(rename = "RSA-OAEP")]
    RsaOaep,
    #[serde(rename = "ECDSA")]
    Ecdsa,
    #[serde(rename = "ECDH")]
    Ecdh,
    #[serde(rename = "AES-CTR")]
    AesCtr,
    #[serde(rename = "AES-CBC")]
    AesCbc,
    #[serde(rename = "AES-GCM")]
    AesGcm,
    #[serde(rename = "AES-KW")]
    AesKw,
    #[serde(rename = "HMAC")]
    Hmac,
    #[serde(rename = "PBKDF2")]
    Pbkdf2,
    #[serde(rename = "HKDF")]
    Hkdf,
}

pub struct CryptoOps;

impl CryptoOps {
    pub fn random_uuid(mut rng: impl Rng) -> anyhow::Result<Uuid> {
        let mut bytes = [0u8; 16];
        rng.fill(&mut bytes);
        let uuid = uuid::Builder::from_bytes(bytes)
            .with_version(uuid::Version::Random)
            .into_uuid();
        Ok(uuid)
    }

    pub fn get_random_values(mut rng: impl Rng, byte_length: u32) -> anyhow::Result<Vec<u8>> {
        anyhow::ensure!(byte_length <= 65536);
        let byte_length = byte_length as usize;
        let mut bytes = vec![0u8; byte_length];
        rng.fill(&mut bytes[..]);

        Ok(bytes)
    }

    pub fn sign(
        key: &[u8],
        data: &[u8],
        algorithm: Algorithm,
        hash: Option<CryptoHash>,
    ) -> anyhow::Result<Vec<u8>> {
        match algorithm {
            Algorithm::Hmac => {
                let hash: HmacAlgorithm = hash
                    .ok_or_else(|| {
                        anyhow::anyhow!(format!("Hash algorithm not supported: {hash:?}"))
                    })?
                    .into();
                let key = HmacKey::new(hash, key);
                let signature = ring::hmac::sign(&key, data);
                Ok(signature.as_ref().to_vec())
            },
            Algorithm::RsassaPkcs1v15
            | Algorithm::RsaPss
            | Algorithm::RsaOaep
            | Algorithm::Ecdsa
            | Algorithm::Ecdh
            | Algorithm::AesCtr
            | Algorithm::AesCbc
            | Algorithm::AesGcm
            | Algorithm::AesKw
            | Algorithm::Pbkdf2
            | Algorithm::Hkdf => anyhow::bail!("Signing algorithm not implemented"),
        }
    }

    pub fn verify(
        key: &[u8],
        data: &[u8],
        signature: &[u8],
        algorithm: Algorithm,
        hash: Option<CryptoHash>,
    ) -> anyhow::Result<bool> {
        match algorithm {
            Algorithm::Hmac => {
                let hash: HmacAlgorithm = hash
                    .ok_or_else(|| {
                        anyhow::anyhow!(format!("Hash algorithm not supported: {hash:?}"))
                    })?
                    .into();
                let key = HmacKey::new(hash, key);
                Ok(ring::hmac::verify(&key, data, signature).is_ok())
            },
            Algorithm::RsassaPkcs1v15
            | Algorithm::RsaPss
            | Algorithm::RsaOaep
            | Algorithm::Ecdsa
            | Algorithm::Ecdh
            | Algorithm::AesCtr
            | Algorithm::AesCbc
            | Algorithm::AesGcm
            | Algorithm::AesKw
            | Algorithm::Pbkdf2
            | Algorithm::Hkdf => anyhow::bail!("Verify algorithm not implemented"),
        }
    }

    pub fn derive_bits(args: DeriveKeyArg, salt: Option<JsBuffer>) -> anyhow::Result<ToJsBuffer> {
        let algorithm = args.algorithm;
        match algorithm {
            Algorithm::Pbkdf2 => {
                let salt = salt.ok_or_else(|| anyhow::anyhow!("Not supported"))?;
                // The caller must validate these cases.
                assert!(args.length > 0);
                assert!(args.length % 8 == 0);

                let algorithm = match args.hash.ok_or_else(|| anyhow::anyhow!("Not supported"))? {
                    CryptoHash::Sha1 => pbkdf2::PBKDF2_HMAC_SHA1,
                    CryptoHash::Sha256 => pbkdf2::PBKDF2_HMAC_SHA256,
                    CryptoHash::Sha384 => pbkdf2::PBKDF2_HMAC_SHA384,
                    CryptoHash::Sha512 => pbkdf2::PBKDF2_HMAC_SHA512,
                };

                // This will never panic. We have already checked length earlier.
                let iterations = NonZeroU32::new(
                    args.iterations
                        .ok_or_else(|| anyhow::anyhow!("Not supported"))?,
                )
                .unwrap();
                let secret = args.key.data;
                let mut out = vec![0; args.length / 8];
                pbkdf2::derive(algorithm, iterations, &salt, &secret, &mut out);
                Ok(out.into())
            },
            Algorithm::Ecdh | Algorithm::Hkdf => anyhow::bail!("Signing algorithm not implemented"),
            _ => Err(anyhow::anyhow!("Unsupported algorithm".to_string())),
        }
    }

    pub fn subtle_digest(algorithm: CryptoHash, data: JsBuffer) -> anyhow::Result<ToJsBuffer> {
        // TODO: Maybe this should be using `spawn_blocking`?
        let output = digest::digest(algorithm.into(), &data)
            .as_ref()
            .to_vec()
            .into();

        Ok(output)
    }
}
