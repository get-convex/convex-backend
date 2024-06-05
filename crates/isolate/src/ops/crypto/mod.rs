// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
// https://github.com/denoland/deno/blob/main/ext/crypto/key.rs

mod ed25519;
mod export_key;
mod import_key;
mod shared;
mod x25519;

use std::num::NonZeroU32;

use anyhow::Context;
use deno_core::ToJsBuffer;
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
        EcdsaKeyPair,
        EcdsaSigningAlgorithm,
        EcdsaVerificationAlgorithm,
        KeyPair,
    },
};
use rsa::{
    pkcs1::{
        DecodeRsaPrivateKey,
        DecodeRsaPublicKey,
    },
    signature::{
        RandomizedSigner,
        SignatureEncoding,
        Signer,
        Verifier,
    },
    RsaPrivateKey,
    RsaPublicKey,
};
use serde::{
    Deserialize,
    Serialize,
};
use serde_bytes::ByteBuf;
use sha1::Sha1;
use sha2::{
    Sha256,
    Sha384,
    Sha512,
};
use uuid::Uuid;

use self::{
    export_key::{
        ExportKeyOptions,
        ExportKeyResult,
    },
    import_key::{
        ImportKeyOptions,
        ImportKeyResult,
    },
    shared::{
        not_supported,
        secure_rng_unavailable,
        type_error,
        AnyError,
        V8RawKeyData,
    },
};
use super::OpProvider;
use crate::ops::crypto::shared::crypto_rng_unavailable;

#[convex_macro::v8_op]
pub fn op_crypto_random_uuid<'b, P: OpProvider<'b>>(provider: &mut P) -> anyhow::Result<String> {
    let rng = provider.rng()?;
    let uuid = CryptoOps::random_uuid(rng)?;
    Ok(uuid.to_string())
}

#[convex_macro::v8_op]
pub fn op_crypto_get_random_values<'b, P: OpProvider<'b>>(
    provider: &mut P,
    byte_length: u32,
) -> anyhow::Result<ToJsBuffer> {
    let rng = provider.rng()?;
    let bytes = CryptoOps::get_random_values(rng, byte_length)?;
    Ok(bytes.into())
}

#[convex_macro::v8_op]
pub fn op_crypto_sign<'b, P: OpProvider<'b>>(
    provider: &mut P,
    args: CryptoSignArgs,
) -> anyhow::Result<ToJsBuffer> {
    let signature = CryptoOps::sign(
        &args.key,
        &args.data,
        args.algorithm,
        args.hash,
        args.salt_length,
        args.named_curve,
    )?;
    Ok(signature.into())
}

#[convex_macro::v8_op]
pub fn op_crypto_sign_ed25519<'b, P: OpProvider<'b>>(
    provider: &mut P,
    key: ByteBuf,
    data: ByteBuf,
) -> anyhow::Result<Option<ToJsBuffer>> {
    Ok(CryptoOps::sign_ed25519(&key, &data))
}

#[convex_macro::v8_op]
pub fn op_crypto_verify<'b, P: OpProvider<'b>>(
    provider: &mut P,
    args: CryptoVerifyArgs,
) -> anyhow::Result<bool> {
    CryptoOps::verify(
        args.key,
        &args.data,
        &args.signature,
        args.algorithm,
        args.named_curve,
        args.hash,
    )
}

#[convex_macro::v8_op]
pub fn op_crypto_verify_ed25519<'b, P: OpProvider<'b>>(
    provider: &mut P,
    key: ByteBuf,
    data: ByteBuf,
    signature: ByteBuf,
) -> anyhow::Result<bool> {
    Ok(CryptoOps::verify_ed25519(&key, &data, &signature))
}

#[convex_macro::v8_op]
pub fn op_crypto_derive_bits<'b, P: OpProvider<'b>>(
    provider: &mut P,
    arg: DeriveKeyArg,
    salt: Option<ByteBuf>,
) -> anyhow::Result<ToJsBuffer> {
    CryptoOps::derive_bits(arg, salt.map(|b| b.into_vec()))
}

#[convex_macro::v8_op]
pub fn op_crypto_digest<'b, P: OpProvider<'b>>(
    provider: &mut P,
    algorithm: CryptoHash,
    data: ByteBuf,
) -> anyhow::Result<ToJsBuffer> {
    CryptoOps::subtle_digest(algorithm, data.into_vec())
}

#[convex_macro::v8_op]
pub fn op_crypto_import_key<'b, P: OpProvider<'b>>(
    provider: &mut P,
    opts: ImportKeyOptions,
    key_data: import_key::KeyData,
) -> anyhow::Result<ImportKeyResult> {
    CryptoOps::import_key(opts, key_data)
}

#[convex_macro::v8_op]
pub fn op_crypto_import_spki_ed25519<'b, P: OpProvider<'b>>(
    provider: &mut P,
    key_data: ByteBuf,
) -> anyhow::Result<Option<ToJsBuffer>> {
    Ok(CryptoOps::import_spki_ed25519(&key_data))
}

#[convex_macro::v8_op]
pub fn op_crypto_import_pkcs8_ed25519<'b, P: OpProvider<'b>>(
    provider: &mut P,
    key_data: ByteBuf,
) -> anyhow::Result<Option<ToJsBuffer>> {
    Ok(CryptoOps::import_pkcs8_ed25519(&key_data))
}

#[convex_macro::v8_op]
pub fn op_crypto_import_spki_x25519<'b, P: OpProvider<'b>>(
    provider: &mut P,
    key_data: ByteBuf,
) -> anyhow::Result<Option<ToJsBuffer>> {
    Ok(CryptoOps::import_spki_x25519(key_data.into_vec()))
}

#[convex_macro::v8_op]
pub fn op_crypto_import_pkcs8_x25519<'b, P: OpProvider<'b>>(
    provider: &mut P,
    key_data: ByteBuf,
) -> anyhow::Result<Option<ToJsBuffer>> {
    Ok(CryptoOps::import_pkcs8_x25519(key_data.into_vec()))
}

#[convex_macro::v8_op]
pub fn op_crypto_base64_url_encode<'b, P: OpProvider<'b>>(
    provider: &mut P,
    data: ByteBuf,
) -> anyhow::Result<String> {
    Ok(base64::encode_config(data, base64::URL_SAFE_NO_PAD))
}

#[convex_macro::v8_op]
pub fn op_crypto_base64_url_decode<'b, P: OpProvider<'b>>(
    provider: &mut P,
    data: String,
) -> anyhow::Result<ToJsBuffer> {
    let data: Vec<u8> = base64::decode_config(data, base64::URL_SAFE_NO_PAD)?;
    Ok(data.into())
}

#[convex_macro::v8_op]
pub fn op_crypto_export_key<'b, P: OpProvider<'b>>(
    provider: &mut P,
    opts: ExportKeyOptions,
    key_data: V8RawKeyData,
) -> anyhow::Result<ExportKeyResult> {
    CryptoOps::export_key(opts, key_data)
}

#[convex_macro::v8_op]
pub fn op_crypto_export_spki_ed25519<'b, P: OpProvider<'b>>(
    provider: &mut P,
    pubkey: ByteBuf,
) -> anyhow::Result<ToJsBuffer> {
    CryptoOps::export_spki_ed25519(&pubkey)
}

#[convex_macro::v8_op]
pub fn op_crypto_export_pkcs8_ed25519<'b, P: OpProvider<'b>>(
    provider: &mut P,
    pkey: ByteBuf,
) -> anyhow::Result<ToJsBuffer> {
    CryptoOps::export_pkcs8_ed25519(&pkey)
}

#[convex_macro::v8_op]
pub fn op_crypto_jwk_x_ed25519<'b, P: OpProvider<'b>>(
    provider: &mut P,
    pkey: ByteBuf,
) -> anyhow::Result<String> {
    CryptoOps::jwk_x_ed25519(&pkey)
}

#[convex_macro::v8_op]
pub fn op_crypto_export_spki_x25519<'b, P: OpProvider<'b>>(
    provider: &mut P,
    pubkey: ByteBuf,
) -> anyhow::Result<ToJsBuffer> {
    CryptoOps::export_spki_x25519(&pubkey)
}

#[convex_macro::v8_op]
pub fn op_crypto_export_pkcs8_x25519<'b, P: OpProvider<'b>>(
    provider: &mut P,
    pkey: ByteBuf,
) -> anyhow::Result<ToJsBuffer> {
    CryptoOps::export_pkcs8_x25519(&pkey)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CryptoSignArgs {
    pub key: ByteBuf,
    pub algorithm: Algorithm,
    pub hash: Option<CryptoHash>,
    pub data: ByteBuf,
    pub salt_length: Option<u32>,
    pub named_curve: Option<CryptoNamedCurve>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CryptoVerifyArgs {
    pub key: KeyData,
    pub algorithm: Algorithm,
    pub hash: Option<CryptoHash>,
    pub signature: ByteBuf,
    pub named_curve: Option<CryptoNamedCurve>,
    pub data: ByteBuf,
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
    r#type: KeyType,
    data: ByteBuf,
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
    // info: Option<ByteBuf>,
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
        salt_length: Option<u32>,
        named_curve: Option<CryptoNamedCurve>,
    ) -> anyhow::Result<Vec<u8>> {
        let signature = match algorithm {
            Algorithm::RsassaPkcs1v15 => {
                use rsa::pkcs1v15::SigningKey;
                let private_key = RsaPrivateKey::from_pkcs1_der(key)?;
                match hash.ok_or_else(|| type_error("Missing argument hash".to_string()))? {
                    CryptoHash::Sha1 => {
                        let signing_key = SigningKey::<Sha1>::new(private_key);
                        signing_key.sign(data)
                    },
                    CryptoHash::Sha256 => {
                        let signing_key = SigningKey::<Sha256>::new(private_key);
                        signing_key.sign(data)
                    },
                    CryptoHash::Sha384 => {
                        let signing_key = SigningKey::<Sha384>::new(private_key);
                        signing_key.sign(data)
                    },
                    CryptoHash::Sha512 => {
                        let signing_key = SigningKey::<Sha512>::new(private_key);
                        signing_key.sign(data)
                    },
                }
                .to_vec()
            },
            Algorithm::RsaPss => {
                use rsa::pss::SigningKey;
                let private_key = RsaPrivateKey::from_pkcs1_der(key)?;

                let salt_len = salt_length
                    .ok_or_else(|| type_error("Missing argument saltLength".to_string()))?
                    as usize;

                let rng = crypto_rng_unavailable()?;
                match hash.ok_or_else(|| type_error("Missing argument hash".to_string()))? {
                    CryptoHash::Sha1 => {
                        let signing_key =
                            SigningKey::<Sha1>::new_with_salt_len(private_key, salt_len);
                        signing_key.sign_with_rng(rng, data)
                    },
                    CryptoHash::Sha256 => {
                        let signing_key =
                            SigningKey::<Sha256>::new_with_salt_len(private_key, salt_len);
                        signing_key.sign_with_rng(rng, data)
                    },
                    CryptoHash::Sha384 => {
                        let signing_key =
                            SigningKey::<Sha384>::new_with_salt_len(private_key, salt_len);
                        signing_key.sign_with_rng(rng, data)
                    },
                    CryptoHash::Sha512 => {
                        let signing_key =
                            SigningKey::<Sha512>::new_with_salt_len(private_key, salt_len);
                        signing_key.sign_with_rng(rng, data)
                    },
                }
                .to_vec()
            },
            Algorithm::Ecdsa => {
                let curve: &EcdsaSigningAlgorithm = named_curve.ok_or_else(not_supported)?.into();

                let key_pair = EcdsaKeyPair::from_pkcs8(curve, key, secure_rng_unavailable()?)
                    .map_err(|e| anyhow::anyhow!(e))?;
                // We only support P256-SHA256 & P384-SHA384. These are recommended signature
                // pairs. https://briansmith.org/rustdoc/ring/signature/index.html#statics
                if let Some(hash) = hash {
                    match hash {
                        CryptoHash::Sha256 | CryptoHash::Sha384 => (),
                        _ => return Err(type_error("Unsupported algorithm")),
                    }
                };

                let signature = key_pair
                    .sign(secure_rng_unavailable()?, data)
                    .map_err(|e| anyhow::anyhow!(e))?;

                // Signature data as buffer.
                signature.as_ref().to_vec()
            },
            Algorithm::Hmac => {
                let hash: HmacAlgorithm = hash.ok_or_else(not_supported)?.into();

                let key = HmacKey::new(hash, key);

                let signature = ring::hmac::sign(&key, data);
                signature.as_ref().to_vec()
            },
            _ => return Err(type_error("Unsupported algorithm".to_string())),
        };

        Ok(signature)
    }

    pub fn verify(
        key: KeyData,
        data: &[u8],
        signature: &[u8],
        algorithm: Algorithm,
        named_curve: Option<CryptoNamedCurve>,
        hash: Option<CryptoHash>,
    ) -> anyhow::Result<bool> {
        let verification = match algorithm {
            Algorithm::RsassaPkcs1v15 => {
                use rsa::pkcs1v15::{
                    Signature,
                    VerifyingKey,
                };
                let public_key = read_rsa_public_key(key)?;
                let signature: Signature = signature.as_ref().try_into()?;
                match hash.ok_or_else(|| type_error("Missing argument hash".to_string()))? {
                    CryptoHash::Sha1 => {
                        let verifying_key = VerifyingKey::<Sha1>::new(public_key);
                        verifying_key.verify(data, &signature).is_ok()
                    },
                    CryptoHash::Sha256 => {
                        let verifying_key = VerifyingKey::<Sha256>::new(public_key);
                        verifying_key.verify(data, &signature).is_ok()
                    },
                    CryptoHash::Sha384 => {
                        let verifying_key = VerifyingKey::<Sha384>::new(public_key);
                        verifying_key.verify(data, &signature).is_ok()
                    },
                    CryptoHash::Sha512 => {
                        let verifying_key = VerifyingKey::<Sha512>::new(public_key);
                        verifying_key.verify(data, &signature).is_ok()
                    },
                }
            },
            Algorithm::RsaPss => {
                use rsa::pss::{
                    Signature,
                    VerifyingKey,
                };
                let public_key = read_rsa_public_key(key)?;
                let signature: Signature = signature.as_ref().try_into()?;

                match hash.ok_or_else(|| type_error("Missing argument hash".to_string()))? {
                    CryptoHash::Sha1 => {
                        let verifying_key: VerifyingKey<Sha1> = public_key.into();
                        verifying_key.verify(data, &signature).is_ok()
                    },
                    CryptoHash::Sha256 => {
                        let verifying_key: VerifyingKey<Sha256> = public_key.into();
                        verifying_key.verify(data, &signature).is_ok()
                    },
                    CryptoHash::Sha384 => {
                        let verifying_key: VerifyingKey<Sha384> = public_key.into();
                        verifying_key.verify(data, &signature).is_ok()
                    },
                    CryptoHash::Sha512 => {
                        let verifying_key: VerifyingKey<Sha512> = public_key.into();
                        verifying_key.verify(data, &signature).is_ok()
                    },
                }
            },
            Algorithm::Hmac => {
                let hash: HmacAlgorithm = hash.ok_or_else(not_supported)?.into();
                let key = HmacKey::new(hash, &key.data);
                ring::hmac::verify(&key, data, signature).is_ok()
            },
            Algorithm::Ecdsa => {
                let signing_alg: &EcdsaSigningAlgorithm =
                    named_curve.ok_or_else(not_supported)?.into();
                let verify_alg: &EcdsaVerificationAlgorithm =
                    named_curve.ok_or_else(not_supported)?.into();

                let private_key;

                let public_key_bytes = match key.r#type {
                    KeyType::Private => {
                        private_key = EcdsaKeyPair::from_pkcs8(
                            signing_alg,
                            &key.data,
                            secure_rng_unavailable()?,
                        )
                        .map_err(|e| anyhow::anyhow!(e))?;

                        private_key.public_key().as_ref()
                    },
                    KeyType::Public => &*key.data,
                    _ => return Err(type_error("Invalid Key format".to_string())),
                };

                let public_key =
                    ring::signature::UnparsedPublicKey::new(verify_alg, public_key_bytes);

                public_key.verify(data, signature).is_ok()
            },
            _ => return Err(type_error("Unsupported algorithm".to_string())),
        };

        Ok(verification)
    }

    pub fn derive_bits(args: DeriveKeyArg, salt: Option<Vec<u8>>) -> anyhow::Result<ToJsBuffer> {
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

    pub fn subtle_digest(algorithm: CryptoHash, data: Vec<u8>) -> anyhow::Result<ToJsBuffer> {
        // TODO: Maybe this should be using `spawn_blocking`?
        let output = digest::digest(algorithm.into(), &data)
            .as_ref()
            .to_vec()
            .into();

        Ok(output)
    }
}

fn read_rsa_public_key(key_data: KeyData) -> Result<RsaPublicKey, AnyError> {
    let public_key = match key_data.r#type {
        KeyType::Private => RsaPrivateKey::from_pkcs1_der(&key_data.data)?.to_public_key(),
        KeyType::Public => RsaPublicKey::from_pkcs1_der(&key_data.data)?,
        KeyType::Secret => unreachable!("unexpected KeyType::Secret"),
    };
    Ok(public_key)
}
