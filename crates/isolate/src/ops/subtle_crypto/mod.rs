use std::str::FromStr;

use aws_lc_rs::digest;
use deno_core::v8;
use indexmap::IndexSet;
use serde::{
    Deserialize,
    Serialize,
};
use serde_bytes::ByteBuf;

use crate::{
    convert_v8::{
        ArrayBuffer,
        DOMException,
        DOMExceptionName,
        FromV8,
        ToV8,
        TypeError,
    },
    ops::{
        errors::throw_uncatchable_developer_error,
        OpProvider,
    },
    strings,
};

mod crypto_key;
mod jwk;
mod serde_helpers;

mod aes;
mod ec;
mod ed25519;
mod hmac;
mod pbkdf2;
mod rsa;
mod x25519;

use self::{
    crypto_key::{
        CryptoKey,
        CryptoKeyKind,
        CryptoKeyOrPair,
        CryptoKeyPair,
    },
    jwk::JsonWebKey,
    serde_helpers::nullary_algorithm,
};

const DERIVE_BITS_MAX: usize = 8 * (1 << 16); // 64KiB in bits
const URL_SAFE_FORGIVING: base64::Config = base64::URL_SAFE_NO_PAD.decode_allow_trailing_bits(true);

const USE_NODE_SUGGESTION: &str = "Consider calling an action defined in Node.js instead (https://docs.convex.dev/functions/actions).";

#[derive(Deserialize, Serialize, Copy, Clone, Eq, PartialEq, Debug)]
enum CryptoHash {
    #[serde(rename = "SHA-1")]
    Sha1,
    #[serde(rename = "SHA-256")]
    Sha256,
    #[serde(rename = "SHA-384")]
    Sha384,
    #[serde(rename = "SHA-512")]
    Sha512,
}
impl CryptoHash {
    fn block_size_bits(&self) -> usize {
        match self {
            CryptoHash::Sha1 => 512,
            CryptoHash::Sha256 => 512,
            CryptoHash::Sha384 => 1024,
            CryptoHash::Sha512 => 1024,
        }
    }

    fn openssl_message_digest(&self) -> openssl::hash::MessageDigest {
        match self {
            CryptoHash::Sha1 => openssl::hash::MessageDigest::sha1(),
            CryptoHash::Sha256 => openssl::hash::MessageDigest::sha256(),
            CryptoHash::Sha384 => openssl::hash::MessageDigest::sha384(),
            CryptoHash::Sha512 => openssl::hash::MessageDigest::sha512(),
        }
    }
}

#[derive(Deserialize, Serialize, Copy, Clone, Eq, PartialEq, Debug, Hash)]
#[serde(rename_all = "camelCase")]
enum KeyUsage {
    Encrypt,
    Decrypt,
    Sign,
    Verify,
    DeriveKey,
    DeriveBits,
    WrapKey,
    UnwrapKey,
}

impl FromStr for KeyUsage {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_owned())).map_err(|_| ())
    }
}

impl KeyUsage {
    #[allow(clippy::inherent_to_string)]
    fn to_string(self) -> String {
        match serde_json::to_value(self) {
            Ok(serde_json::Value::String(s)) => s,
            _ => unreachable!(),
        }
    }
}

#[derive(Deserialize, Serialize, Copy, Clone, Eq, PartialEq, Debug, Hash)]
#[serde(rename_all = "camelCase")]
pub enum KeyType {
    Secret,
    Private,
    Public,
}

fn get_name<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    input: v8::Local<'s, v8::Value>,
) -> anyhow::Result<String> {
    let v8_string = if let Ok(s) = input.try_cast::<v8::String>() {
        s
    } else if let Ok(object) = input.try_cast::<v8::Object>() {
        let name_str = strings::name.create(scope)?;
        object
            .get(scope, name_str.into())
            .ok_or_else(|| anyhow::anyhow!(TypeError::new("'name' missing in algorithm",)))?
            .to_string(scope)
            .ok_or_else(|| anyhow::anyhow!("[TODO: propagate exception]"))?
    } else {
        anyhow::bail!(DOMException::new(
            "Unrecognized or invalid algorithm",
            DOMExceptionName::NotSupportedError
        ))
    };
    let mut string = v8_string.to_rust_string_lossy(scope);
    string.make_ascii_lowercase();
    Ok(string)
}

#[derive(Deserialize, Debug)]
enum KeyFormat {
    #[serde(rename = "raw")]
    Raw,
    #[serde(rename = "pkcs8")]
    Pkcs8,
    #[serde(rename = "spki")]
    Spki,
    #[serde(rename = "jwk")]
    Jwk,
}

#[derive(Debug)]
enum ImportKeyAlgorithm {
    Rsa(rsa::RsaHashedImportParams),
    Ec(ec::EcKeyImportParams),
    Hmac(hmac::HmacImportParams),
    Aes(aes::AesAlgorithm),
    AesKw,
    Pbkdf2,
    Hkdf,
    Ed25519,
    X25519,
}

impl FromV8 for ImportKeyAlgorithm {
    type Output = ImportKeyAlgorithm;

    fn from_v8<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        input: v8::Local<'s, v8::Value>,
    ) -> anyhow::Result<Self::Output> {
        match get_name(scope, input)?.as_str() {
            "rsassa-pkcs1-v1_5" | "rsa-pss" | "rsa-oaep" => Ok(Self::Rsa(
                rsa::RsaHashedImportParams::from_v8(scope, input)?,
            )),
            "ecdsa" | "ecdh" => Ok(Self::Ec(ec::EcKeyAlgorithm::from_v8(scope, input)?)),
            "hmac" => Ok(Self::Hmac(hmac::HmacImportParams::from_v8(scope, input)?)),
            "aes-cbc" => Ok(Self::Aes(aes::AesAlgorithm::AesCbc)),
            "aes-ctr" => Ok(Self::Aes(aes::AesAlgorithm::AesCtr)),
            "aes-gcm" => Ok(Self::Aes(aes::AesAlgorithm::AesGcm)),
            "aes-kw" => Ok(Self::AesKw),
            "pbkdf2" => Ok(Self::Pbkdf2),
            "hkdf" => Ok(Self::Hkdf),
            "ed25519" => Ok(Self::Ed25519),
            "x25519" => Ok(Self::X25519),
            name => anyhow::bail!(DOMException::new(
                format!("Unrecognized or invalid algorithm {name}"),
                DOMExceptionName::NotSupportedError
            )),
        }
    }
}

enum KeyData {
    Raw(Vec<u8>),
    Jwk(JsonWebKey),
}
// Accepts either raw bytes or an object
impl FromV8 for KeyData {
    type Output = KeyData;

    fn from_v8<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        input: v8::Local<'s, v8::Value>,
    ) -> anyhow::Result<Self::Output> {
        // TODO: does data view actually work?
        if input.is_array_buffer() || input.is_typed_array() || input.is_data_view() {
            <serde_bytes::ByteBuf>::from_v8(scope, input).map(|x| Self::Raw(x.into_vec()))
        } else {
            JsonWebKey::from_v8(scope, input).map(Self::Jwk)
        }
    }
}
impl ToV8 for KeyData {
    fn to_v8<'s>(
        self,
        scope: &mut v8::PinScope<'s, '_>,
    ) -> anyhow::Result<v8::Local<'s, v8::Value>> {
        match self {
            KeyData::Raw(bytes) => ArrayBuffer(bytes).to_v8(scope),
            KeyData::Jwk(jwk) => jwk.to_v8(scope),
        }
    }
}

enum ImportKeyInput {
    Raw(Vec<u8>),
    Pkcs8(Vec<u8>),
    Spki(Vec<u8>),
    Jwk(JsonWebKey),
}

#[convex_macro::v8_op]
pub(crate) fn op_crypto_subtle_import_key<'b, P: OpProvider<'b>>(
    provider: &mut P,
    format: KeyFormat,
    key_data: KeyData,
    algorithm: ImportKeyAlgorithm,
    extractable: bool,
    key_usages: IndexSet<KeyUsage>,
) -> anyhow::Result<CryptoKey> {
    let input = match (format, key_data) {
        (KeyFormat::Raw, KeyData::Raw(data)) => ImportKeyInput::Raw(data),
        (KeyFormat::Pkcs8, KeyData::Raw(data)) => ImportKeyInput::Pkcs8(data),
        (KeyFormat::Spki, KeyData::Raw(data)) => ImportKeyInput::Spki(data),
        (KeyFormat::Jwk, KeyData::Jwk(jwk)) => ImportKeyInput::Jwk(jwk),
        (format, _) => anyhow::bail!(TypeError::new(format!(
            "wrong keyData for format {format:?}"
        ))),
    };
    let key = match algorithm {
        ImportKeyAlgorithm::Rsa(algorithm) => {
            rsa::import_key(input, algorithm, extractable, key_usages)?
        },
        ImportKeyAlgorithm::Ec(algorithm) => {
            ec::import_key(input, algorithm, extractable, key_usages)?
        },
        ImportKeyAlgorithm::Hmac(algorithm) => {
            hmac::import_key(input, algorithm, extractable, key_usages)?
        },
        ImportKeyAlgorithm::Aes(algorithm) => {
            aes::import_key(input, algorithm, extractable, key_usages)?
        },
        ImportKeyAlgorithm::AesKw => unimplemented(provider, "importKey", "AES-KW")?,
        ImportKeyAlgorithm::Pbkdf2 => pbkdf2::import_key(input, extractable, key_usages)?,
        ImportKeyAlgorithm::Hkdf => unimplemented(provider, "importKey", "HKDF")?,
        ImportKeyAlgorithm::Ed25519 => ed25519::import_key(input, extractable, key_usages)?,
        ImportKeyAlgorithm::X25519 => x25519::import_key(input, extractable, key_usages)?,
    };
    key.check_useless()?;
    Ok(key)
}

#[derive(Debug)]
enum KeyDeriveParams {
    Pbkdf2(pbkdf2::Pbkdf2Params),
    Ecdh,   // TODO
    Hkdf,   // TODO
    X25519, // TODO
}
impl FromV8 for KeyDeriveParams {
    type Output = KeyDeriveParams;

    fn from_v8<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        input: v8::Local<'s, v8::Value>,
    ) -> anyhow::Result<Self::Output> {
        match get_name(scope, input)?.as_str() {
            "pbkdf2" => Ok(Self::Pbkdf2(pbkdf2::Pbkdf2Params::from_v8(scope, input)?)),
            "ecdh" => Ok(Self::Ecdh),
            "hkdf" => Ok(Self::Hkdf),
            "x25519" => Ok(Self::X25519),
            _ => anyhow::bail!(DOMException::new(
                format!("invalid algorithm for key"),
                DOMExceptionName::InvalidAccessError
            )),
        }
    }
}

fn unimplemented<'b, P: OpProvider<'b>>(
    provider: &mut P,
    operation: &'static str,
    algorithm: &'static str,
) -> anyhow::Result<!> {
    throw_uncatchable_developer_error(
        provider,
        format!(
            "Not implemented: crypto.subtle.{operation} for {algorithm}. {USE_NODE_SUGGESTION}"
        ),
    )
}

fn derive_bits_inner<'b, P: OpProvider<'b>>(
    provider: &mut P,
    operation: &'static str,
    algorithm: KeyDeriveParams,
    key: &CryptoKey,
    length: Option<usize>,
) -> anyhow::Result<Vec<u8>> {
    match algorithm {
        KeyDeriveParams::Pbkdf2(algorithm) => pbkdf2::derive_bits(algorithm, key, length),
        KeyDeriveParams::Ecdh => unimplemented(provider, operation, "ECDH")?,
        KeyDeriveParams::Hkdf => unimplemented(provider, operation, "HKDF")?,
        KeyDeriveParams::X25519 => unimplemented(provider, operation, "X25519")?,
    }
}

#[convex_macro::v8_op]
pub(crate) fn op_crypto_subtle_derive_bits<'b, P: OpProvider<'b>>(
    provider: &mut P,
    algorithm: KeyDeriveParams,
    key: CryptoKey,
    length: Option<usize>,
) -> anyhow::Result<ArrayBuffer> {
    key.check_usage(KeyUsage::DeriveBits)?;
    derive_bits_inner(provider, "deriveBits", algorithm, &key, length).map(ArrayBuffer)
}

enum DerivedKeyAlgorithm {
    Hmac(hmac::HmacImportParams),
    Aes(aes::AesDerivedKeyParams),
    AesKw,
}
impl FromV8 for DerivedKeyAlgorithm {
    type Output = DerivedKeyAlgorithm;

    fn from_v8<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        input: v8::Local<'s, v8::Value>,
    ) -> anyhow::Result<Self::Output> {
        match get_name(scope, input)?.as_str() {
            "hmac" => Ok(Self::Hmac(hmac::HmacImportParams::from_v8(scope, input)?)),
            "aes-ctr" | "aes-cbc" | "aes-gcm" => {
                Ok(Self::Aes(aes::AesKeyGenParams::from_v8(scope, input)?))
            },
            "aes-kw" => Ok(Self::AesKw),
            name => anyhow::bail!(DOMException::new(
                format!("Unrecognized or invalid algorithm {name}"),
                DOMExceptionName::NotSupportedError
            )),
        }
    }
}

#[convex_macro::v8_op]
pub(crate) fn op_crypto_subtle_derive_key<'b, P: OpProvider<'b>>(
    provider: &mut P,
    algorithm: KeyDeriveParams,
    base_key: CryptoKey,
    derived_key_type: DerivedKeyAlgorithm,
    extractable: bool,
    key_usages: IndexSet<KeyUsage>,
) -> anyhow::Result<CryptoKey> {
    base_key.check_usage(KeyUsage::DeriveKey)?;
    let length = match &derived_key_type {
        DerivedKeyAlgorithm::Hmac(alg) => alg.get_key_length()?,
        DerivedKeyAlgorithm::Aes(alg) => alg.get_key_length()?,
        DerivedKeyAlgorithm::AesKw => unimplemented(provider, "deriveKey", "AES-KW")?,
    };
    let key_bits = derive_bits_inner(provider, "deriveKey", algorithm, &base_key, Some(length))?;
    let key_input = ImportKeyInput::Raw(key_bits);
    let key = match derived_key_type {
        DerivedKeyAlgorithm::Hmac(alg) => {
            hmac::import_key(key_input, alg, extractable, key_usages)?
        },
        DerivedKeyAlgorithm::Aes(alg) => {
            aes::import_key(key_input, alg.name, extractable, key_usages)?
        },
        DerivedKeyAlgorithm::AesKw => unreachable!(),
    };
    key.check_useless()?;
    Ok(key)
}

enum KeyGenParams {
    Rsa(rsa::RsaHashedKeyGenParams),
    Ec(ec::EcKeyGenParams),
    Hmac(hmac::HmacKeyGenParams),
    Aes(aes::AesKeyGenParams),
    AesKw,
    Ed25519,
    X25519,
}

impl FromV8 for KeyGenParams {
    type Output = KeyGenParams;

    fn from_v8<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        input: v8::Local<'s, v8::Value>,
    ) -> anyhow::Result<Self::Output> {
        match get_name(scope, input)?.as_str() {
            "rsassa-pkcs1-v1_5" | "rsa-pss" | "rsa-oaep" => Ok(Self::Rsa(
                rsa::RsaHashedKeyGenParams::from_v8(scope, input)?,
            )),
            "ecdsa" | "ecdh" => Ok(Self::Ec(ec::EcKeyGenParams::from_v8(scope, input)?)),
            "hmac" => Ok(Self::Hmac(hmac::HmacKeyGenParams::from_v8(scope, input)?)),
            "aes-ctr" | "aes-cbc" | "aes-gcm" => {
                Ok(Self::Aes(aes::AesKeyGenParams::from_v8(scope, input)?))
            },
            "aes-kw" => Ok(Self::AesKw),
            "ed25519" => Ok(Self::Ed25519),
            "x25519" => Ok(Self::X25519),
            name => anyhow::bail!(DOMException::new(
                format!("Unrecognized or invalid algorithm {name}"),
                DOMExceptionName::NotSupportedError
            )),
        }
    }
}

#[convex_macro::v8_op]
pub(crate) fn op_crypto_subtle_generate_key<'b, P: OpProvider<'b>>(
    provider: &mut P,
    algorithm: KeyGenParams,
    extractable: bool,
    key_usages: IndexSet<KeyUsage>,
) -> anyhow::Result<CryptoKeyOrPair> {
    let rng = provider.crypto_rng()?;
    let result = match algorithm {
        KeyGenParams::Rsa(algorithm) => {
            rsa::generate_keypair(algorithm, &rng, extractable, key_usages)?.into()
        },
        KeyGenParams::Ec(algorithm) => {
            ec::generate_keypair(algorithm, &rng, extractable, key_usages)?.into()
        },
        KeyGenParams::Hmac(algorithm) => {
            hmac::generate_key(algorithm, &rng, extractable, key_usages)?.into()
        },
        KeyGenParams::Aes(algorithm) => {
            aes::generate_key(algorithm, &rng, extractable, key_usages)?.into()
        },
        KeyGenParams::AesKw => unimplemented(provider, "generateKey", "AES-KW")?,
        KeyGenParams::Ed25519 => ed25519::generate_keypair(&rng, extractable, key_usages)?.into(),
        KeyGenParams::X25519 => x25519::generate_keypair(&rng, extractable, key_usages)?.into(),
    };
    match &result {
        CryptoKeyOrPair::Symmetric(key) => key.check_useless()?,
        CryptoKeyOrPair::Asymmetric(keypair) => keypair.private_key.check_useless()?,
    }
    Ok(result)
}

#[convex_macro::v8_op]
pub(crate) fn op_crypto_subtle_export_key<'b, P: OpProvider<'b>>(
    provider: &mut P,
    format: KeyFormat,
    key: CryptoKey,
) -> anyhow::Result<KeyData> {
    anyhow::ensure!(
        key.extractable,
        DOMException::new(
            "key is not extractable",
            DOMExceptionName::InvalidAccessError
        )
    );
    let mut exported = match &key.kind {
        CryptoKeyKind::Pbkdf2 { .. } => {
            anyhow::bail!(DOMException::new(
                "PBKDF2 keys are not exportable",
                DOMExceptionName::NotSupportedError
            ))
        },
        CryptoKeyKind::Hmac { algorithm, key } => key.export_key(algorithm, format)?,
        CryptoKeyKind::Aes { algorithm, key } => key.export_key(algorithm, format)?,
        CryptoKeyKind::RsaPrivate { algorithm, key } => key.export_key(algorithm, format)?,
        CryptoKeyKind::RsaPublic { algorithm, key } => key.export_key(algorithm, format)?,
        CryptoKeyKind::EcPrivate { algorithm, key } => key.export_key(algorithm, format)?,
        CryptoKeyKind::EcPublic { algorithm, key } => key.export_key(algorithm, format)?,
        CryptoKeyKind::Ed25519Private { algorithm: _, key } => key.export_key(format)?,
        CryptoKeyKind::Ed25519Public { algorithm: _, key } => key.export_key(format)?,
        CryptoKeyKind::X25519Private { algorithm: _, key } => key.export_key(format)?,
        CryptoKeyKind::X25519Public { algorithm: _, key } => key.export_key(format)?,
    };
    if let KeyData::Jwk(jwk) = &mut exported {
        jwk.key_ops = Some(key.usages.iter().map(|x| x.to_string()).collect());
        jwk.ext = Some(key.extractable);
    }
    Ok(exported)
}

enum EncryptDecryptAlgorithm {
    RsaOaep(rsa::RsaOaepParams),
    AesCtr(aes::AesCtrParams),
    AesCbc(aes::AesCbcParams),
    AesGcm(aes::AesGcmParams),
}

impl FromV8 for EncryptDecryptAlgorithm {
    type Output = EncryptDecryptAlgorithm;

    fn from_v8<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        input: v8::Local<'s, v8::Value>,
    ) -> anyhow::Result<Self::Output> {
        match get_name(scope, input)?.as_str() {
            "rsa-oaep" => Ok(Self::RsaOaep(rsa::RsaOaepParams::from_v8(scope, input)?)),
            "aes-ctr" => Ok(Self::AesCtr(aes::AesCtrParams::from_v8(scope, input)?)),
            "aes-cbc" => Ok(Self::AesCbc(aes::AesCbcParams::from_v8(scope, input)?)),
            "aes-gcm" => Ok(Self::AesGcm(aes::AesGcmParams::from_v8(scope, input)?)),
            _ => anyhow::bail!(DOMException::new(
                "invalid algorithm for key",
                DOMExceptionName::InvalidAccessError
            )),
        }
    }
}

#[convex_macro::v8_op]
pub(crate) fn op_crypto_subtle_decrypt<'b, P: OpProvider<'b>>(
    provider: &mut P,
    algorithm: EncryptDecryptAlgorithm,
    key: CryptoKey,
    data: ByteBuf,
) -> anyhow::Result<ArrayBuffer> {
    key.check_usage(KeyUsage::Decrypt)?;
    let plaintext = match (algorithm, &key.kind) {
        (
            EncryptDecryptAlgorithm::RsaOaep(params),
            CryptoKeyKind::RsaPrivate { algorithm, key },
        ) => key.decrypt_oaep(params, algorithm, &data)?,
        (EncryptDecryptAlgorithm::AesCtr(params), CryptoKeyKind::Aes { algorithm, key }) => {
            key.crypt_ctr(params, algorithm, data.into_vec())?
        },
        (EncryptDecryptAlgorithm::AesCbc(params), CryptoKeyKind::Aes { algorithm, key }) => {
            key.decrypt_cbc(params, algorithm, data.into_vec())?
        },
        (EncryptDecryptAlgorithm::AesGcm(params), CryptoKeyKind::Aes { algorithm, key }) => {
            key.decrypt_gcm(params, algorithm, data.into_vec())?
        },
        _ => anyhow::bail!(DOMException::new(
            "invalid algorithm for key",
            DOMExceptionName::InvalidAccessError
        )),
    };
    Ok(ArrayBuffer(plaintext))
}

#[convex_macro::v8_op]
pub(crate) fn op_crypto_subtle_encrypt<'b, P: OpProvider<'b>>(
    provider: &mut P,
    algorithm: EncryptDecryptAlgorithm,
    key: CryptoKey,
    data: ByteBuf,
) -> anyhow::Result<ArrayBuffer> {
    key.check_usage(KeyUsage::Encrypt)?;
    let ciphertext = match (algorithm, &key.kind) {
        (EncryptDecryptAlgorithm::RsaOaep(params), CryptoKeyKind::RsaPublic { algorithm, key }) => {
            key.encrypt_oaep(params, algorithm, &provider.crypto_rng()?, &data)?
        },
        (EncryptDecryptAlgorithm::AesCtr(params), CryptoKeyKind::Aes { algorithm, key }) => {
            key.crypt_ctr(params, algorithm, data.into_vec())?
        },
        (EncryptDecryptAlgorithm::AesCbc(params), CryptoKeyKind::Aes { algorithm, key }) => {
            key.encrypt_cbc(params, algorithm, data.into_vec())?
        },
        (EncryptDecryptAlgorithm::AesGcm(params), CryptoKeyKind::Aes { algorithm, key }) => {
            key.encrypt_gcm(params, algorithm, data.into_vec())?
        },
        _ => anyhow::bail!(DOMException::new(
            "invalid algorithm for key",
            DOMExceptionName::InvalidAccessError
        )),
    };
    Ok(ArrayBuffer(ciphertext))
}

#[derive(Deserialize)]
struct DigestAlgorithm(#[serde(with = "nullary_algorithm")] CryptoHash);

#[convex_macro::v8_op]
pub(crate) fn op_crypto_subtle_digest<'b, P: OpProvider<'b>>(
    provider: &mut P,
    algorithm: DigestAlgorithm,
    data: ByteBuf,
) -> anyhow::Result<ArrayBuffer> {
    let algo = match algorithm.0 {
        CryptoHash::Sha1 => &digest::SHA1_FOR_LEGACY_USE_ONLY,
        CryptoHash::Sha256 => &digest::SHA256,
        CryptoHash::Sha384 => &digest::SHA384,
        CryptoHash::Sha512 => &digest::SHA512,
    };
    Ok(ArrayBuffer(digest::digest(algo, &data).as_ref().to_vec()))
}

enum SignVerifyAlgorithm {
    Rsa(rsa::RsaParams),
    Ecdsa(ec::EcdsaParams),
    Hmac,
    Ed25519,
}

impl FromV8 for SignVerifyAlgorithm {
    type Output = SignVerifyAlgorithm;

    fn from_v8<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        input: v8::Local<'s, v8::Value>,
    ) -> anyhow::Result<Self::Output> {
        match get_name(scope, input)?.as_str() {
            "rsassa-pkcs1-v1_5" => Ok(Self::Rsa(rsa::RsaParams::RsaSsaPkcs1v15)),
            "rsa-pss" => Ok(Self::Rsa(rsa::RsaParams::RsaPss(
                rsa::RsaPssParams::from_v8(scope, input)?,
            ))),
            "ecdsa" => Ok(Self::Ecdsa(ec::EcdsaParams::from_v8(scope, input)?)),
            "hmac" => Ok(Self::Hmac),
            "ed25519" => Ok(Self::Ed25519),
            _ => anyhow::bail!(DOMException::new(
                "invalid algorithm for key",
                DOMExceptionName::InvalidAccessError
            )),
        }
    }
}

#[convex_macro::v8_op]
pub(crate) fn op_crypto_subtle_sign<'b, P: OpProvider<'b>>(
    provider: &mut P,
    algorithm: SignVerifyAlgorithm,
    key: CryptoKey,
    data: ByteBuf,
) -> anyhow::Result<ArrayBuffer> {
    key.check_usage(KeyUsage::Sign)?;
    let signature = match (algorithm, &key.kind) {
        (SignVerifyAlgorithm::Rsa(params), CryptoKeyKind::RsaPrivate { algorithm, key }) => {
            key.sign(params, algorithm, || provider.crypto_rng(), &data)?
        },
        (SignVerifyAlgorithm::Ecdsa(params), CryptoKeyKind::EcPrivate { algorithm, key }) => {
            key.sign(params, algorithm, &provider.crypto_rng()?, &data)?
        },
        (SignVerifyAlgorithm::Hmac, CryptoKeyKind::Hmac { algorithm, key }) => {
            key.sign(algorithm, &data)
        },
        (SignVerifyAlgorithm::Ed25519, CryptoKeyKind::Ed25519Private { algorithm: _, key }) => {
            key.sign(&data)
        },
        _ => anyhow::bail!(DOMException::new(
            "invalid algorithm for key",
            DOMExceptionName::InvalidAccessError
        )),
    };
    Ok(ArrayBuffer(signature))
}

#[convex_macro::v8_op]
pub(crate) fn op_crypto_subtle_verify<'b, P: OpProvider<'b>>(
    provider: &mut P,
    algorithm: SignVerifyAlgorithm,
    key: CryptoKey,
    signature: ByteBuf,
    data: ByteBuf,
) -> anyhow::Result<bool> {
    key.check_usage(KeyUsage::Verify)?;
    match (algorithm, &key.kind) {
        (SignVerifyAlgorithm::Rsa(params), CryptoKeyKind::RsaPublic { algorithm, key }) => {
            Ok(key.verify(params, algorithm, &data, &signature)?)
        },
        (SignVerifyAlgorithm::Ecdsa(params), CryptoKeyKind::EcPublic { algorithm, key }) => {
            Ok(key.verify(params, algorithm, &data, &signature)?)
        },
        (SignVerifyAlgorithm::Hmac, CryptoKeyKind::Hmac { algorithm, key }) => {
            Ok(key.verify(algorithm, &data, &signature))
        },
        (SignVerifyAlgorithm::Ed25519, CryptoKeyKind::Ed25519Public { algorithm: _, key }) => {
            Ok(key.verify(&data, &signature))
        },
        _ => anyhow::bail!(DOMException::new(
            "invalid algorithm for key",
            DOMExceptionName::InvalidAccessError
        )),
    }
}

#[derive(Deserialize)]
enum WrapKeyAlgorithm {}

/// Note: this op is never called, JS raises an error directly
#[convex_macro::v8_op]
pub(crate) fn op_crypto_subtle_wrap_key<'b, P: OpProvider<'b>>(
    provider: &mut P,
    _format: KeyFormat,
    _key: CryptoKey,
    _wrapping_key: CryptoKey,
    _wrapping_algorithm: WrapKeyAlgorithm,
) -> anyhow::Result<ByteBuf> {
    anyhow::bail!(DOMException::new(
        "wrapKey not implemented",
        DOMExceptionName::NotSupportedError
    ));
}

/// Note: this op is never called, JS raises an error directly
#[convex_macro::v8_op]
pub(crate) fn op_crypto_subtle_unwrap_key<'b, P: OpProvider<'b>>(
    provider: &mut P,
    _format: KeyFormat,
    _wrapped_key: ByteBuf,
    _unwrapping_key: CryptoKey,
    _wrapping_algorithm: WrapKeyAlgorithm,
    _unwrapped_key_algorithm: ImportKeyAlgorithm, // should be something else
) -> anyhow::Result<CryptoKey> {
    anyhow::bail!(DOMException::new(
        "unwrapKey not implemented",
        DOMExceptionName::NotSupportedError
    ));
}

fn check_usages_subset(
    usages: &IndexSet<KeyUsage>,
    possible_usages: &[KeyUsage],
) -> anyhow::Result<()> {
    anyhow::ensure!(
        usages.iter().all(|usage| possible_usages.contains(usage)),
        DOMException::new("invalid key_usages", DOMExceptionName::SyntaxError)
    );
    Ok(())
}
