// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
// https://github.com/denoland/deno/blob/main/ext/crypto/key.rs

mod ed25519;
mod export_key;
mod generate_key;
mod import_key;
mod shared;
mod x25519;

use std::num::NonZeroU32;

use anyhow::Context;
use aws_lc_rs::{
    aead::{
        self,
        Aad,
        LessSafeKey,
        Nonce,
        UnboundKey,
    },
    agreement::Algorithm as RingAlgorithm,
    digest,
    error::Unspecified,
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
use deno_core::ToJsBuffer;
use openssl::{
    pkey::PKey,
    rsa::Rsa,
    sign::{
        RsaPssSaltlen,
        Verifier,
    },
};
use rand::Rng;
use serde::{
    Deserialize,
    Serialize,
};
use serde_bytes::ByteBuf;
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
        type_error,
        RustRawKeyData,
        V8RawKeyData,
    },
};
use super::OpProvider;
use crate::environment::crypto_rng::CryptoRng;

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
        || provider.crypto_rng(),
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
        args.salt_length,
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
pub fn op_crypto_encrypt<'b, P: OpProvider<'b>>(
    provider: &mut P,
    algorithm: EncryptDecryptAlgorithm,
    key: KeyData,
    data: ByteBuf,
) -> anyhow::Result<ToJsBuffer> {
    Ok(CryptoOps::subtle_encrypt(algorithm, key, data.into_vec())?.into())
}

#[convex_macro::v8_op]
pub fn op_crypto_decrypt<'b, P: OpProvider<'b>>(
    provider: &mut P,
    algorithm: EncryptDecryptAlgorithm,
    key: KeyData,
    data: ByteBuf,
) -> anyhow::Result<Option<ToJsBuffer>> {
    Ok(CryptoOps::subtle_decrypt(algorithm, key, data.into_vec())?.map(ToJsBuffer::from))
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

#[convex_macro::v8_op]
pub fn op_crypto_generate_keypair<'b, P: OpProvider<'b>>(
    provider: &mut P,
    algorithm: GenerateKeypairAlgorithm,
) -> anyhow::Result<GeneratedKeypair> {
    let rng = provider.crypto_rng()?;
    CryptoOps::generate_keypair(rng, algorithm)
}

#[convex_macro::v8_op]
pub fn op_crypto_generate_key_bytes<'b, P: OpProvider<'b>>(
    provider: &mut P,
    length: usize,
) -> anyhow::Result<ToJsBuffer> {
    let rng = provider.crypto_rng()?;
    CryptoOps::generate_key_bytes(rng, length)
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
    pub salt_length: Option<u32>,
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
            CryptoHash::Sha1 => aws_lc_rs::hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
            CryptoHash::Sha256 => aws_lc_rs::hmac::HMAC_SHA256,
            CryptoHash::Sha384 => aws_lc_rs::hmac::HMAC_SHA384,
            CryptoHash::Sha512 => aws_lc_rs::hmac::HMAC_SHA512,
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

impl From<CryptoHash> for openssl::hash::MessageDigest {
    fn from(hash: CryptoHash) -> openssl::hash::MessageDigest {
        match hash {
            CryptoHash::Sha1 => openssl::hash::MessageDigest::sha1(),
            CryptoHash::Sha256 => openssl::hash::MessageDigest::sha256(),
            CryptoHash::Sha384 => openssl::hash::MessageDigest::sha384(),
            CryptoHash::Sha512 => openssl::hash::MessageDigest::sha512(),
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
            CryptoNamedCurve::P256 => &aws_lc_rs::agreement::ECDH_P256,
            CryptoNamedCurve::P384 => &aws_lc_rs::agreement::ECDH_P384,
        }
    }
}

impl From<CryptoNamedCurve> for &EcdsaSigningAlgorithm {
    fn from(curve: CryptoNamedCurve) -> &'static EcdsaSigningAlgorithm {
        match curve {
            CryptoNamedCurve::P256 => &aws_lc_rs::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            CryptoNamedCurve::P384 => &aws_lc_rs::signature::ECDSA_P384_SHA384_FIXED_SIGNING,
        }
    }
}

impl From<CryptoNamedCurve> for &EcdsaVerificationAlgorithm {
    fn from(curve: CryptoNamedCurve) -> &'static EcdsaVerificationAlgorithm {
        match curve {
            CryptoNamedCurve::P256 => &aws_lc_rs::signature::ECDSA_P256_SHA256_FIXED,
            CryptoNamedCurve::P384 => &aws_lc_rs::signature::ECDSA_P384_SHA384_FIXED,
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

#[derive(Deserialize)]
#[serde(tag = "name")]
#[serde(rename_all_fields = "camelCase")]
pub enum EncryptDecryptAlgorithm {
    #[serde(rename = "AES-GCM")]
    AesGcm {
        iv: ByteBuf,
        additional_data: Option<ByteBuf>,
        tag_length: usize,
    },
}

#[derive(Deserialize)]
pub enum Curve25519Algorithm {
    Ed25519,
    X25519,
}

#[derive(Deserialize)]
#[serde(rename_all_fields = "camelCase")]
#[serde(untagged)]
pub enum GenerateKeypairAlgorithm {
    Rsa {
        name: Algorithm,
        modulus_length: usize,
        public_exponent: ByteBuf,
    },
    Ec {
        name: Algorithm,
        named_curve: CryptoNamedCurve,
    },
    Curve25519 {
        name: Curve25519Algorithm,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedKeypair {
    public_raw_data: GeneratedKey,
    private_raw_data: GeneratedKey,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum GeneratedKey {
    KeyData(RustRawKeyData),
    // Ed25519/X25519 store just raw key bytes instead of a structure :/
    RawBytes(ToJsBuffer),
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
        let max_byte_length = 65536;
        anyhow::ensure!(
            byte_length <= max_byte_length,
            type_error(format!(
                "Byte length ({}) exceeds the number of bytes of entropy available via this API \
                 ({})",
                byte_length, max_byte_length
            ))
        );
        let byte_length = byte_length as usize;
        let mut bytes = vec![0u8; byte_length];
        rng.fill(&mut bytes[..]);

        Ok(bytes)
    }

    pub fn sign(
        rng: impl FnOnce() -> anyhow::Result<CryptoRng>,
        key: &[u8],
        data: &[u8],
        algorithm: Algorithm,
        hash: Option<CryptoHash>,
        salt_length: Option<u32>,
        named_curve: Option<CryptoNamedCurve>,
    ) -> anyhow::Result<Vec<u8>> {
        let signature = match algorithm {
            Algorithm::RsassaPkcs1v15 | Algorithm::RsaPss => {
                let key_pair: openssl::pkey::PKey<_> =
                    Rsa::private_key_from_der(key)?.try_into()?;
                let hash_algorithm = hash
                    .ok_or_else(|| type_error("Missing argument hash".to_string()))?
                    .into();
                let padding_algorithm = match algorithm {
                    Algorithm::RsassaPkcs1v15 => openssl::rsa::Padding::PKCS1,
                    Algorithm::RsaPss => {
                        // RSA-PSS uses randomized padding; this requires the
                        // crypto RNG (although openssl-rs doesn't ask for it
                        // explicitly)
                        _ = rng()?;
                        openssl::rsa::Padding::PKCS1_PSS
                    },
                    _ => unreachable!(),
                };
                let mut signer = openssl::sign::Signer::new(hash_algorithm, &key_pair)?;
                signer
                    .set_rsa_padding(padding_algorithm)
                    .context("invalid padding algorithm")?;
                if let Algorithm::RsaPss = algorithm {
                    signer
                        .set_rsa_pss_saltlen(RsaPssSaltlen::custom(
                            salt_length
                                .context("Missing argument saltLength")?
                                .try_into()
                                .context("invalid saltLength")?,
                        ))
                        .context("invalid saltLength")?;
                } else {
                    anyhow::ensure!(salt_length.is_none(), "only PSS takes saltLength");
                }
                let mut signature = vec![0; signer.len()?];
                let actual_len = signer.sign_oneshot(&mut signature, data)?;
                signature.truncate(actual_len);
                signature
            },
            Algorithm::Ecdsa => {
                let curve: &EcdsaSigningAlgorithm = named_curve.ok_or_else(not_supported)?.into();
                // ECDSA uses a nonce that must never be reused or revealed, or
                // else it leaks the private key. So we require a true CryptoRng
                // here.
                // TODO: we could use RFC6979 deterministic signatures instead
                // (but `ring` does not support it).
                let rng = rng()?;

                let key_pair =
                    EcdsaKeyPair::from_pkcs8(curve, key).map_err(|e| anyhow::anyhow!(e))?;
                // We only support P256-SHA256 & P384-SHA384. These are recommended signature
                // pairs. https://briansmith.org/rustdoc/ring/signature/index.html#statics
                if let Some(hash) = hash {
                    match hash {
                        CryptoHash::Sha256 | CryptoHash::Sha384 => (),
                        _ => return Err(type_error("Unsupported algorithm")),
                    }
                };

                let signature = key_pair
                    .sign(&rng.aws_lc(), data)
                    .map_err(|e| anyhow::anyhow!(e))?;

                // Signature data as buffer.
                signature.as_ref().to_vec()
            },
            Algorithm::Hmac => {
                let hash: HmacAlgorithm = hash.ok_or_else(not_supported)?.into();

                let key = HmacKey::new(hash, key);

                let signature = aws_lc_rs::hmac::sign(&key, data);
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
        salt_length: Option<u32>,
        hash: Option<CryptoHash>,
    ) -> anyhow::Result<bool> {
        let verification = match algorithm {
            Algorithm::RsassaPkcs1v15 | Algorithm::RsaPss => {
                let hash_algorithm = hash
                    .ok_or_else(|| type_error("Missing argument hash".to_string()))?
                    .into();
                let (private_key, public_key);
                let mut verifier = match key.r#type {
                    KeyType::Private => {
                        private_key = PKey::try_from(Rsa::private_key_from_der(&key.data)?)?;
                        Verifier::new(hash_algorithm, private_key.as_ref())?
                    },
                    KeyType::Public => {
                        public_key = PKey::try_from(Rsa::public_key_from_der_pkcs1(&key.data)?)?;
                        Verifier::new(hash_algorithm, public_key.as_ref())?
                    },
                    KeyType::Secret => anyhow::bail!("unexpected KeyType::Secret"),
                };
                let padding_algorithm = match algorithm {
                    Algorithm::RsassaPkcs1v15 => openssl::rsa::Padding::PKCS1,
                    Algorithm::RsaPss => openssl::rsa::Padding::PKCS1_PSS,
                    _ => unreachable!(),
                };
                verifier
                    .set_rsa_padding(padding_algorithm)
                    .context("invalid padding algorithm")?;
                if let Algorithm::RsaPss = algorithm {
                    verifier
                        .set_rsa_pss_saltlen(RsaPssSaltlen::custom(
                            salt_length
                                .context("Missing argument saltLength")?
                                .try_into()
                                .context("invalid saltLength")?,
                        ))
                        .context("invalid saltLength")?;
                } else {
                    anyhow::ensure!(salt_length.is_none(), "only PSS takes saltLength");
                }
                verifier.verify_oneshot(signature, data)?
            },
            Algorithm::Hmac => {
                let hash: HmacAlgorithm = hash.ok_or_else(not_supported)?.into();
                let key = HmacKey::new(hash, &key.data);
                aws_lc_rs::hmac::verify(&key, data, signature).is_ok()
            },
            Algorithm::Ecdsa => {
                let signing_alg: &EcdsaSigningAlgorithm =
                    named_curve.ok_or_else(not_supported)?.into();
                let verify_alg: &EcdsaVerificationAlgorithm =
                    named_curve.ok_or_else(not_supported)?.into();

                let private_key;

                let public_key_bytes = match key.r#type {
                    KeyType::Private => {
                        private_key = EcdsaKeyPair::from_pkcs8(signing_alg, &key.data)
                            .map_err(|e| anyhow::anyhow!(e))?;

                        private_key.public_key().as_ref()
                    },
                    KeyType::Public => &*key.data,
                    _ => return Err(type_error("Invalid Key format".to_string())),
                };

                let public_key =
                    aws_lc_rs::signature::UnparsedPublicKey::new(verify_alg, public_key_bytes);

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

    pub fn subtle_encrypt(
        algorithm: EncryptDecryptAlgorithm,
        key: KeyData,
        mut data: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        match algorithm {
            EncryptDecryptAlgorithm::AesGcm {
                iv,
                additional_data,
                tag_length,
            } => {
                anyhow::ensure!(matches!(key.r#type, KeyType::Secret));
                let alg = match key.data.len() {
                    16 => &aead::AES_128_GCM,
                    24 => &aead::AES_192_GCM,
                    32 => &aead::AES_256_GCM,
                    _ => anyhow::bail!("unsupported key length {}", key.data.len()),
                };
                // TODO: consider supporting shorter tag lengths (`ring` does not allow this)
                anyhow::ensure!(
                    tag_length == 8 * alg.tag_len(),
                    "invalid tag len {tag_length}"
                );
                let key = LessSafeKey::new(
                    UnboundKey::new(alg, &key.data)
                        .ok()
                        .context("invalid AES-GCM key")?,
                );
                key.seal_in_place_append_tag(
                    // TODO: consider supporting GHASH construction for nonces (`ring` does not
                    // support this)
                    Nonce::try_assume_unique_for_key(&iv)
                        .ok()
                        .context("wrong AES-GCM IV length")?,
                    Aad::from(additional_data.as_ref().map_or(&[][..], |b| b.as_ref())),
                    &mut data,
                )
                .ok()
                .context("AES-GCM encryption failed")?;
                Ok(data)
            },
        }
    }

    /// Returns `None` if decryption failed due to invalid data (e.g. missing or
    /// incorrect tag)
    pub fn subtle_decrypt(
        algorithm: EncryptDecryptAlgorithm,
        key: KeyData,
        mut data: Vec<u8>,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        match algorithm {
            EncryptDecryptAlgorithm::AesGcm {
                iv,
                additional_data,
                tag_length,
            } => {
                anyhow::ensure!(matches!(key.r#type, KeyType::Secret));
                let alg = match key.data.len() {
                    16 => &aead::AES_128_GCM,
                    24 => &aead::AES_192_GCM,
                    32 => &aead::AES_256_GCM,
                    _ => anyhow::bail!("unsupported key length {}", key.data.len()),
                };
                anyhow::ensure!(
                    tag_length == 8 * alg.tag_len(),
                    "invalid tag len {tag_length}"
                );
                let key = LessSafeKey::new(
                    UnboundKey::new(alg, &key.data)
                        .ok()
                        .context("invalid AES-GCM key")?,
                );
                let plaintext_len = match key.open_in_place(
                    Nonce::try_assume_unique_for_key(&iv)
                        .ok()
                        .context("wrong AES-GCM IV length")?,
                    Aad::from(additional_data.as_ref().map_or(&[][..], |b| b.as_ref())),
                    &mut data,
                ) {
                    Ok(plaintext) => plaintext.len(),
                    Err(Unspecified) => return Ok(None),
                };
                data.truncate(plaintext_len);
                Ok(Some(data))
            },
        }
    }
}
