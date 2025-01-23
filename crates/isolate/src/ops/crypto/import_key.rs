// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
// https://github.com/denoland/deno/blob/main/ext/crypto/import_key.rs

use deno_core::ToJsBuffer;
use elliptic_curve::pkcs8::PrivateKeyInfo;
use p256::pkcs8::EncodePrivateKey;
use ring::signature::EcdsaKeyPair;
use rsa::{
    pkcs1::UintRef,
    pkcs8::der::Decode as RsaDecode,
};
use serde::{
    Deserialize,
    Serialize,
};
use serde_bytes::ByteBuf;
use spki::{
    der::Encode as SpkiEncode,
    SubjectPublicKeyInfoRef,
};

use super::{
    shared::{
        data_error,
        not_supported_error,
        unsupported_format,
        AnyError,
        EcNamedCurve,
        RustRawKeyData,
        ID_SECP256R1_OID,
        ID_SECP384R1_OID,
        ID_SECP521R1_OID,
        RSA_ENCRYPTION_OID,
    },
    CryptoNamedCurve,
    CryptoOps,
};
use crate::ops::crypto::shared::secure_rng_unavailable;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyData {
    Spki(ByteBuf),
    Pkcs8(ByteBuf),
    Raw(ByteBuf),
    JwkSecret {
        k: String,
    },
    JwkPublicRsa {
        n: String,
        e: String,
    },
    JwkPrivateRsa {
        n: String,
        e: String,
        d: String,
        p: String,
        q: String,
        dp: String,
        dq: String,
        qi: String,
    },
    JwkPublicEc {
        x: String,
        y: String,
    },
    JwkPrivateEc {
        x: String,
        y: String,
        d: String,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", tag = "algorithm")]
pub enum ImportKeyOptions {
    #[serde(rename = "RSASSA-PKCS1-v1_5")]
    RsassaPkcs1v15 {},
    #[serde(rename = "RSA-PSS")]
    RsaPss {},
    #[serde(rename = "RSA-OAEP")]
    RsaOaep {},
    #[serde(rename = "ECDSA", rename_all = "camelCase")]
    Ecdsa { named_curve: EcNamedCurve },
    #[serde(rename = "ECDH", rename_all = "camelCase")]
    Ecdh { named_curve: EcNamedCurve },
    #[serde(rename = "AES", rename_all = "camelCase")]
    Aes {},
    #[serde(rename = "HMAC", rename_all = "camelCase")]
    Hmac {},
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum ImportKeyResult {
    #[serde(rename_all = "camelCase")]
    Rsa {
        raw_data: RustRawKeyData,
        modulus_length: usize,
        public_exponent: ToJsBuffer,
    },
    #[serde(rename_all = "camelCase")]
    Ec { raw_data: RustRawKeyData },
    #[serde(rename_all = "camelCase")]
    #[expect(dead_code)]
    Aes { raw_data: RustRawKeyData },
    #[serde(rename_all = "camelCase")]
    Hmac { raw_data: RustRawKeyData },
}

impl CryptoOps {
    pub fn import_key(
        opts: ImportKeyOptions,
        key_data: KeyData,
    ) -> Result<ImportKeyResult, AnyError> {
        match opts {
            ImportKeyOptions::RsassaPkcs1v15 {} => import_key_rsassa(key_data),
            ImportKeyOptions::RsaPss {} => import_key_rsapss(key_data),
            ImportKeyOptions::RsaOaep {} => import_key_rsaoaep(key_data),
            ImportKeyOptions::Ecdsa { named_curve } | ImportKeyOptions::Ecdh { named_curve } => {
                import_key_ec(key_data, named_curve)
            },
            ImportKeyOptions::Aes {} => import_key_aes(key_data),
            ImportKeyOptions::Hmac {} => import_key_hmac(key_data),
        }
    }
}

const URL_SAFE_FORGIVING: base64::Config = base64::URL_SAFE_NO_PAD.decode_allow_trailing_bits(true);

macro_rules! jwt_b64_int_or_err {
    ($name:ident, $b64:expr, $err:expr) => {
        let bytes =
            base64::decode_config($b64, URL_SAFE_FORGIVING).map_err(|_| data_error($err))?;
        let $name = UintRef::new(&bytes).map_err(|_| data_error($err))?;
    };
}

fn import_key_rsa_jwk(key_data: KeyData) -> anyhow::Result<ImportKeyResult> {
    match key_data {
        KeyData::JwkPublicRsa { n, e } => {
            jwt_b64_int_or_err!(modulus, &n, "invalid modulus");
            jwt_b64_int_or_err!(public_exponent, &e, "invalid public exponent");

            let public_key = rsa::pkcs1::RsaPublicKey {
                modulus,
                public_exponent,
            };

            let mut data = Vec::new();
            public_key
                .encode_to_vec(&mut data)
                .map_err(|_| data_error("invalid rsa public key"))?;
            let public_exponent = public_key.public_exponent.as_bytes().to_vec().into();
            let modulus_length = public_key.modulus.as_bytes().len() * 8;

            Ok(ImportKeyResult::Rsa {
                raw_data: RustRawKeyData::Public(data.into()),
                modulus_length,
                public_exponent,
            })
        },
        KeyData::JwkPrivateRsa {
            n,
            e,
            d,
            p,
            q,
            dp,
            dq,
            qi,
        } => {
            jwt_b64_int_or_err!(modulus, &n, "invalid modulus");
            jwt_b64_int_or_err!(public_exponent, &e, "invalid public exponent");
            jwt_b64_int_or_err!(private_exponent, &d, "invalid private exponent");
            jwt_b64_int_or_err!(prime1, &p, "invalid first prime factor");
            jwt_b64_int_or_err!(prime2, &q, "invalid second prime factor");
            jwt_b64_int_or_err!(exponent1, &dp, "invalid first CRT exponent");
            jwt_b64_int_or_err!(exponent2, &dq, "invalid second CRT exponent");
            jwt_b64_int_or_err!(coefficient, &qi, "invalid CRT coefficient");

            let private_key = rsa::pkcs1::RsaPrivateKey {
                modulus,
                public_exponent,
                private_exponent,
                prime1,
                prime2,
                exponent1,
                exponent2,
                coefficient,
                other_prime_infos: None,
            };

            let mut data = Vec::new();
            private_key
                .encode_to_vec(&mut data)
                .map_err(|_| data_error("invalid rsa private key"))?;

            let public_exponent = private_key.public_exponent.as_bytes().to_vec().into();
            let modulus_length = private_key.modulus.as_bytes().len() * 8;

            Ok(ImportKeyResult::Rsa {
                raw_data: RustRawKeyData::Private(data.into()),
                modulus_length,
                public_exponent,
            })
        },
        _ => unreachable!(),
    }
}

fn import_key_rsassa(key_data: KeyData) -> anyhow::Result<ImportKeyResult> {
    match key_data {
        KeyData::Spki(data) => {
            // 2-3.
            // Note we parse as BitString<'_> instead of Vec<u8> because
            // that's required to infer the write ASN.1 type.
            let pk_info: SubjectPublicKeyInfoRef = spki::SubjectPublicKeyInfo::from_der(&data)
                .map_err(|e| data_error(e.to_string()))?;

            // 4-5.
            let alg = pk_info.algorithm.oid;

            // 6-7. (skipped, only support rsaEncryption for interoperability)
            if alg != RSA_ENCRYPTION_OID {
                return Err(data_error("unsupported algorithm"));
            }

            // 8-9.
            let public_key =
                rsa::pkcs1::RsaPublicKey::from_der(pk_info.subject_public_key.raw_bytes())
                    .map_err(|e| data_error(e.to_string()))?;

            let bytes_consumed = public_key
                .encoded_len()
                .map_err(|e| data_error(e.to_string()))?;

            if bytes_consumed
                != spki::der::Length::new(pk_info.subject_public_key.raw_bytes().len() as u16)
            {
                return Err(data_error("public key is invalid (too long)"));
            }

            let data = pk_info.subject_public_key.raw_bytes().to_vec().into();
            let public_exponent = public_key.public_exponent.as_bytes().to_vec().into();
            let modulus_length = public_key.modulus.as_bytes().len() * 8;

            Ok(ImportKeyResult::Rsa {
                raw_data: RustRawKeyData::Public(data),
                modulus_length,
                public_exponent,
            })
        },
        KeyData::Pkcs8(data) => {
            // 2-3.
            let pk_info = PrivateKeyInfo::from_der(&data).map_err(|e| data_error(e.to_string()))?;

            // 4-5.
            let alg = pk_info.algorithm.oid;

            // 6-7. (skipped, only support rsaEncryption for interoperability)
            if alg != RSA_ENCRYPTION_OID {
                return Err(data_error("unsupported algorithm"));
            }

            // 8-9.
            let private_key = rsa::pkcs1::RsaPrivateKey::from_der(pk_info.private_key)
                .map_err(|e| data_error(e.to_string()))?;

            let bytes_consumed = private_key
                .encoded_len()
                .map_err(|e| data_error(e.to_string()))?;

            if bytes_consumed != spki::der::Length::new(pk_info.private_key.len() as u16) {
                return Err(data_error("private key is invalid (too long)"));
            }

            let data = pk_info.private_key.to_vec().into();
            let public_exponent = private_key.public_exponent.as_bytes().to_vec().into();
            let modulus_length = private_key.modulus.as_bytes().len() * 8;

            Ok(ImportKeyResult::Rsa {
                raw_data: RustRawKeyData::Private(data),
                modulus_length,
                public_exponent,
            })
        },
        KeyData::JwkPublicRsa { .. } | KeyData::JwkPrivateRsa { .. } => {
            import_key_rsa_jwk(key_data)
        },
        _ => Err(unsupported_format()),
    }
}

fn import_key_rsapss(key_data: KeyData) -> anyhow::Result<ImportKeyResult> {
    match key_data {
        KeyData::Spki(data) => {
            // 2-3.
            let pk_info: SubjectPublicKeyInfoRef = spki::SubjectPublicKeyInfo::from_der(&data)
                .map_err(|e| data_error(e.to_string()))?;

            // 4-5.
            let alg = pk_info.algorithm.oid;

            // 6-7. (skipped, only support rsaEncryption for interoperability)
            if alg != RSA_ENCRYPTION_OID {
                return Err(data_error("unsupported algorithm"));
            }

            // 8-9.
            let public_key =
                rsa::pkcs1::RsaPublicKey::from_der(pk_info.subject_public_key.raw_bytes())
                    .map_err(|e| data_error(e.to_string()))?;

            let bytes_consumed = public_key
                .encoded_len()
                .map_err(|e| data_error(e.to_string()))?;

            if bytes_consumed
                != spki::der::Length::new(pk_info.subject_public_key.raw_bytes().len() as u16)
            {
                return Err(data_error("public key is invalid (too long)"));
            }

            let data = pk_info.subject_public_key.raw_bytes().to_vec().into();
            let public_exponent = public_key.public_exponent.as_bytes().to_vec().into();
            let modulus_length = public_key.modulus.as_bytes().len() * 8;

            Ok(ImportKeyResult::Rsa {
                raw_data: RustRawKeyData::Public(data),
                modulus_length,
                public_exponent,
            })
        },
        KeyData::Pkcs8(data) => {
            // 2-3.
            let pk_info = PrivateKeyInfo::from_der(&data).map_err(|e| data_error(e.to_string()))?;

            // 4-5.
            let alg = pk_info.algorithm.oid;

            // 6-7. (skipped, only support rsaEncryption for interoperability)
            if alg != RSA_ENCRYPTION_OID {
                return Err(data_error("unsupported algorithm"));
            }

            // 8-9.
            let private_key = rsa::pkcs1::RsaPrivateKey::from_der(pk_info.private_key)
                .map_err(|e| data_error(e.to_string()))?;

            let bytes_consumed = private_key
                .encoded_len()
                .map_err(|e| data_error(e.to_string()))?;

            if bytes_consumed != spki::der::Length::new(pk_info.private_key.len() as u16) {
                return Err(data_error("private key is invalid (too long)"));
            }

            let data = pk_info.private_key.to_vec().into();
            let public_exponent = private_key.public_exponent.as_bytes().to_vec().into();
            let modulus_length = private_key.modulus.as_bytes().len() * 8;

            Ok(ImportKeyResult::Rsa {
                raw_data: RustRawKeyData::Private(data),
                modulus_length,
                public_exponent,
            })
        },
        KeyData::JwkPublicRsa { .. } | KeyData::JwkPrivateRsa { .. } => {
            import_key_rsa_jwk(key_data)
        },
        _ => Err(unsupported_format()),
    }
}

fn import_key_rsaoaep(key_data: KeyData) -> Result<ImportKeyResult, anyhow::Error> {
    match key_data {
        KeyData::Spki(data) => {
            // 2-3.
            let pk_info: SubjectPublicKeyInfoRef = spki::SubjectPublicKeyInfo::from_der(&data)
                .map_err(|e| data_error(e.to_string()))?;

            // 4-5.
            let alg = pk_info.algorithm.oid;

            // 6-7. (skipped, only support rsaEncryption for interoperability)
            if alg != RSA_ENCRYPTION_OID {
                return Err(data_error("unsupported algorithm"));
            }

            // 8-9.
            let public_key =
                rsa::pkcs1::RsaPublicKey::from_der(pk_info.subject_public_key.raw_bytes())
                    .map_err(|e| data_error(e.to_string()))?;

            let bytes_consumed = public_key
                .encoded_len()
                .map_err(|e| data_error(e.to_string()))?;

            if bytes_consumed
                != spki::der::Length::new(pk_info.subject_public_key.raw_bytes().len() as u16)
            {
                return Err(data_error("public key is invalid (too long)"));
            }

            let data = pk_info.subject_public_key.raw_bytes().to_vec().into();
            let public_exponent = public_key.public_exponent.as_bytes().to_vec().into();
            let modulus_length = public_key.modulus.as_bytes().len() * 8;

            Ok(ImportKeyResult::Rsa {
                raw_data: RustRawKeyData::Public(data),
                modulus_length,
                public_exponent,
            })
        },
        KeyData::Pkcs8(data) => {
            // 2-3.
            let pk_info = PrivateKeyInfo::from_der(&data).map_err(|e| data_error(e.to_string()))?;

            // 4-5.
            let alg = pk_info.algorithm.oid;

            // 6-7. (skipped, only support rsaEncryption for interoperability)
            if alg != RSA_ENCRYPTION_OID {
                return Err(data_error("unsupported algorithm"));
            }

            // 8-9.
            let private_key = rsa::pkcs1::RsaPrivateKey::from_der(pk_info.private_key)
                .map_err(|e| data_error(e.to_string()))?;

            let bytes_consumed = private_key
                .encoded_len()
                .map_err(|e| data_error(e.to_string()))?;

            if bytes_consumed != spki::der::Length::new(pk_info.private_key.len() as u16) {
                return Err(data_error("private key is invalid (too long)"));
            }

            let data = pk_info.private_key.to_vec().into();
            let public_exponent = private_key.public_exponent.as_bytes().to_vec().into();
            let modulus_length = private_key.modulus.as_bytes().len() * 8;

            Ok(ImportKeyResult::Rsa {
                raw_data: RustRawKeyData::Private(data),
                modulus_length,
                public_exponent,
            })
        },
        KeyData::JwkPublicRsa { .. } | KeyData::JwkPrivateRsa { .. } => {
            import_key_rsa_jwk(key_data)
        },
        _ => Err(unsupported_format()),
    }
}

fn decode_b64url_to_field_bytes<C: elliptic_curve::Curve>(
    b64: &str,
) -> Result<elliptic_curve::FieldBytes<C>, anyhow::Error> {
    jwt_b64_int_or_err!(val, b64, "invalid b64 coordinate");

    let mut bytes = elliptic_curve::FieldBytes::<C>::default();
    let original_bytes = val.as_bytes();
    let mut new_bytes: Vec<u8> = vec![];
    if original_bytes.len() < bytes.len() {
        new_bytes = vec![0; bytes.len() - original_bytes.len()];
    }
    new_bytes.extend_from_slice(original_bytes);

    let val = new_bytes.as_slice();

    if val.len() != bytes.len() {
        return Err(data_error("invalid b64 coordinate"));
    }
    bytes.copy_from_slice(val);

    Ok(bytes)
}

fn import_key_ec_jwk_to_point(
    x: String,
    y: String,
    named_curve: EcNamedCurve,
) -> Result<Vec<u8>, anyhow::Error> {
    let point_bytes = match named_curve {
        EcNamedCurve::P256 => {
            let x = decode_b64url_to_field_bytes::<p256::NistP256>(&x)?;
            let y = decode_b64url_to_field_bytes::<p256::NistP256>(&y)?;

            p256::EncodedPoint::from_affine_coordinates(&x, &y, false).to_bytes()
        },
        EcNamedCurve::P384 => {
            let x = decode_b64url_to_field_bytes::<p384::NistP384>(&x)?;
            let y = decode_b64url_to_field_bytes::<p384::NistP384>(&y)?;

            p384::EncodedPoint::from_affine_coordinates(&x, &y, false).to_bytes()
        },
        _ => return Err(not_supported_error("Unsupported named curve")),
    };

    Ok(point_bytes.to_vec())
}

fn import_key_ec_jwk(
    key_data: KeyData,
    named_curve: EcNamedCurve,
) -> anyhow::Result<ImportKeyResult> {
    match key_data {
        KeyData::JwkPublicEc { x, y } => {
            let point_bytes = import_key_ec_jwk_to_point(x, y, named_curve)?;

            Ok(ImportKeyResult::Ec {
                raw_data: RustRawKeyData::Public(point_bytes.into()),
            })
        },
        KeyData::JwkPrivateEc { d, x, y } => {
            jwt_b64_int_or_err!(private_d, &d, "invalid JWK private key");
            let point_bytes = import_key_ec_jwk_to_point(x, y, named_curve)?;
            let pkcs8_der = match named_curve {
                EcNamedCurve::P256 => {
                    let d = decode_b64url_to_field_bytes::<p256::NistP256>(&d)?;
                    let pk = p256::SecretKey::from_bytes(&d)?;

                    pk.to_pkcs8_der().map_err(|e| anyhow::anyhow!(e))?
                },
                EcNamedCurve::P384 => {
                    let d = decode_b64url_to_field_bytes::<p384::NistP384>(&d)?;
                    let pk = p384::SecretKey::from_bytes(&d)?;

                    pk.to_pkcs8_der().map_err(|e| anyhow::anyhow!(e))?
                },
                EcNamedCurve::P521 => return Err(data_error("Unsupported named curve")),
            };

            // Import using ring, to validate key
            let key_alg = match named_curve {
                EcNamedCurve::P256 => CryptoNamedCurve::P256.into(),
                EcNamedCurve::P384 => CryptoNamedCurve::P256.into(),
                EcNamedCurve::P521 => return Err(data_error("Unsupported named curve")),
            };

            let _key_pair = EcdsaKeyPair::from_private_key_and_public_key(
                key_alg,
                private_d.as_bytes(),
                point_bytes.as_ref(),
                secure_rng_unavailable()?,
            );

            Ok(ImportKeyResult::Ec {
                raw_data: RustRawKeyData::Private(pkcs8_der.as_bytes().to_vec().into()),
            })
        },
        _ => unreachable!(),
    }
}

pub struct ECParametersSpki {
    pub named_curve_alg: spki::der::asn1::ObjectIdentifier,
}

impl<'a> TryFrom<spki::der::asn1::AnyRef<'a>> for ECParametersSpki {
    type Error = spki::der::Error;

    fn try_from(any: spki::der::asn1::AnyRef<'a>) -> spki::der::Result<ECParametersSpki> {
        let x = any.try_into()?;

        Ok(Self { named_curve_alg: x })
    }
}

fn import_key_ec(
    key_data: KeyData,
    named_curve: EcNamedCurve,
) -> Result<ImportKeyResult, AnyError> {
    match key_data {
        KeyData::Raw(data) => {
            // The point is parsed and validated, ultimately the original data is
            // returned though.
            match named_curve {
                EcNamedCurve::P256 => {
                    // 1-2.
                    let point = p256::EncodedPoint::from_bytes(&data)
                        .map_err(|_| data_error("invalid P-256 elliptic curve point"))?;
                    // 3.
                    if point.is_identity() {
                        return Err(data_error("invalid P-256 elliptic curve point"));
                    }
                },
                EcNamedCurve::P384 => {
                    // 1-2.
                    let point = p384::EncodedPoint::from_bytes(&data)
                        .map_err(|_| data_error("invalid P-384 elliptic curve point"))?;
                    // 3.
                    if point.is_identity() {
                        return Err(data_error("invalid P-384 elliptic curve point"));
                    }
                },
                _ => return Err(not_supported_error("Unsupported named curve")),
            };
            Ok(ImportKeyResult::Ec {
                raw_data: RustRawKeyData::Public(data.to_vec().into()),
            })
        },
        KeyData::Pkcs8(data) => {
            // 2-7
            // Deserialize PKCS8 - validate structure, extracts named_curve
            let named_curve_alg = match named_curve {
                EcNamedCurve::P256 | EcNamedCurve::P384 => {
                    let pk = PrivateKeyInfo::from_der(data.as_ref())
                        .map_err(|_| data_error("expected valid PKCS#8 data"))?;
                    pk.algorithm.oid
                },
                EcNamedCurve::P521 => return Err(data_error("Unsupported named curve")),
            };

            // 8-9.
            let pk_named_curve = match named_curve_alg {
                // id-secp256r1
                ID_SECP256R1_OID => Some(EcNamedCurve::P256),
                // id-secp384r1
                ID_SECP384R1_OID => Some(EcNamedCurve::P384),
                // id-secp521r1
                ID_SECP521R1_OID => Some(EcNamedCurve::P521),
                _ => None,
            };

            // 10.
            if let Some(pk_named_curve) = pk_named_curve {
                let signing_alg = match pk_named_curve {
                    EcNamedCurve::P256 => CryptoNamedCurve::P256.into(),
                    EcNamedCurve::P384 => CryptoNamedCurve::P384.into(),
                    EcNamedCurve::P521 => return Err(data_error("Unsupported named curve")),
                };

                // deserialize pkcs8 using ring crate, to VALIDATE public key
                let _private_key =
                    EcdsaKeyPair::from_pkcs8(signing_alg, &data, secure_rng_unavailable()?)
                        .map_err(|e| anyhow::anyhow!(e))?;

                // 11.
                if named_curve != pk_named_curve {
                    return Err(data_error("curve mismatch"));
                }
            } else {
                return Err(data_error("Unsupported named curve"));
            }

            Ok(ImportKeyResult::Ec {
                raw_data: RustRawKeyData::Private(data.to_vec().into()),
            })
        },
        KeyData::Spki(data) => {
            // 2-3.
            let pk_info: SubjectPublicKeyInfoRef = spki::SubjectPublicKeyInfo::from_der(&data)
                .map_err(|e| data_error(e.to_string()))?;

            // 4.
            let alg = pk_info.algorithm.oid;
            // id-ecPublicKey
            if alg != elliptic_curve::ALGORITHM_OID {
                return Err(data_error("unsupported algorithm"));
            }

            // 5-7.
            let params = ECParametersSpki::try_from(
                pk_info
                    .algorithm
                    .parameters
                    .ok_or_else(|| data_error("malformed parameters"))?,
            )
            .map_err(|_| data_error("malformed parameters"))?;

            // 8-9.
            let named_curve_alg = params.named_curve_alg;
            let pk_named_curve = match named_curve_alg {
                // id-secp256r1
                ID_SECP256R1_OID => Some(EcNamedCurve::P256),
                // id-secp384r1
                ID_SECP384R1_OID => Some(EcNamedCurve::P384),
                // id-secp521r1
                ID_SECP521R1_OID => Some(EcNamedCurve::P521),
                _ => None,
            };

            // 10.
            let encoded_key;

            if let Some(pk_named_curve) = pk_named_curve {
                let pk = pk_info.subject_public_key.raw_bytes();

                encoded_key = pk.to_vec();

                let bytes_consumed = match named_curve {
                    EcNamedCurve::P256 => {
                        let point = p256::EncodedPoint::from_bytes(&*encoded_key)
                            .map_err(|_| data_error("invalid P-256 elliptic curve SPKI data"))?;
                        if point.is_identity() {
                            return Err(data_error("invalid P-256 elliptic curve point"));
                        }

                        point.as_bytes().len()
                    },
                    EcNamedCurve::P384 => {
                        let point = p384::EncodedPoint::from_bytes(&*encoded_key)
                            .map_err(|_| data_error("invalid P-384 elliptic curve SPKI data"))?;

                        if point.is_identity() {
                            return Err(data_error("invalid P-384 elliptic curve point"));
                        }

                        point.as_bytes().len()
                    },
                    _ => return Err(not_supported_error("Unsupported named curve")),
                };

                if bytes_consumed != pk_info.subject_public_key.raw_bytes().len() {
                    return Err(data_error("public key is invalid (too long)"));
                }

                // 11.
                if named_curve != pk_named_curve {
                    return Err(data_error("curve mismatch"));
                }
            } else {
                return Err(data_error("Unsupported named curve"));
            }

            Ok(ImportKeyResult::Ec {
                raw_data: RustRawKeyData::Public(encoded_key.into()),
            })
        },
        KeyData::JwkPublicEc { .. } | KeyData::JwkPrivateEc { .. } => {
            import_key_ec_jwk(key_data, named_curve)
        },
        _ => Err(unsupported_format()),
    }
}

fn import_key_aes(key_data: KeyData) -> Result<ImportKeyResult, AnyError> {
    Ok(match key_data {
        KeyData::JwkSecret { k } => {
            let data = base64::decode_config(k, URL_SAFE_FORGIVING)
                .map_err(|_| data_error("invalid key data"))?;
            ImportKeyResult::Hmac {
                raw_data: RustRawKeyData::Secret(data.into()),
            }
        },
        _ => return Err(unsupported_format()),
    })
}

fn import_key_hmac(key_data: KeyData) -> Result<ImportKeyResult, AnyError> {
    Ok(match key_data {
        KeyData::JwkSecret { k } => {
            let data = base64::decode_config(k, URL_SAFE_FORGIVING)
                .map_err(|_| data_error("invalid key data"))?;
            ImportKeyResult::Hmac {
                raw_data: RustRawKeyData::Secret(data.into()),
            }
        },
        _ => return Err(unsupported_format()),
    })
}
