use aws_lc_rs::{
    agreement::{
        self,
        X25519,
    },
    encoding::{
        AsBigEndian,
        Curve25519SeedBin,
    },
};
use indexmap::IndexSet;
use serde::Serialize;
use spki::der::{
    asn1::{
        BitStringRef,
        OctetStringRef,
    },
    AnyRef,
    Decode as _,
    Encode,
};

use super::{
    check_usages_subset,
    CryptoKey,
    CryptoKeyKind,
    CryptoKeyPair,
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
    },
    environment::crypto_rng::CryptoRng,
};

// id-X25519 OBJECT IDENTIFIER ::= { 1 3 101 110 }
const X25519_OID: const_oid::ObjectIdentifier =
    const_oid::ObjectIdentifier::new_unwrap("1.3.101.110");

#[derive(Serialize)]
#[serde(tag = "name")]
#[serde(rename = "X25519")]
pub(crate) struct X25519Algorithm {}

pub(crate) struct X25519PrivateKey {
    private_key: agreement::PrivateKey,
}

pub(crate) struct X25519PublicKey {
    public_key: agreement::UnparsedPublicKey<[u8; 32]>,
}

pub(crate) fn generate_keypair(
    _rng: &CryptoRng,
    extractable: bool,
    usages: IndexSet<KeyUsage>,
) -> anyhow::Result<CryptoKeyPair> {
    check_usages_subset(&usages, &[KeyUsage::DeriveKey, KeyUsage::DeriveBits])?;
    let private_key = agreement::PrivateKey::generate(&X25519)?;
    let public_key = private_key.compute_public_key()?;
    Ok(CryptoKeyPair {
        private_key: CryptoKey {
            kind: CryptoKeyKind::X25519Private {
                algorithm: X25519Algorithm {},
                key: X25519PrivateKey { private_key },
            },
            r#type: KeyType::Private,
            extractable,
            usages,
        },
        public_key: CryptoKey {
            kind: CryptoKeyKind::X25519Public {
                algorithm: X25519Algorithm {},
                key: X25519PublicKey {
                    public_key: agreement::UnparsedPublicKey::new(
                        &X25519,
                        public_key.as_ref().try_into()?,
                    ),
                },
            },
            r#type: KeyType::Public,
            extractable: true, // N.B.: public key is always extractable
            usages: IndexSet::new(),
        },
    })
}

pub(crate) fn import_key(
    format: ImportKeyInput,
    extractable: bool,
    usages: IndexSet<KeyUsage>,
) -> anyhow::Result<CryptoKey> {
    match format {
        ImportKeyInput::Spki(der) => {
            check_usages_subset(&usages, &[])?;
            let spki = spki::SubjectPublicKeyInfo::<AnyRef, BitStringRef<'_>>::from_der(&der)
                .map_err(|_| {
                    DOMException::new(
                        "invalid SubjectPublicKeyInfo document",
                        DOMExceptionName::DataError,
                    )
                })?;
            anyhow::ensure!(
                spki.algorithm.oid == X25519_OID,
                DOMException::new(
                    "SubjectPublicKeyInfo algorithm is not id-X25519",
                    DOMExceptionName::DataError
                )
            );
            anyhow::ensure!(
                spki.algorithm.parameters.is_none(),
                DOMException::new(
                    "SubjectPublicKeyInfo parameters must not be present",
                    DOMExceptionName::DataError
                )
            );
            let x = spki
                .subject_public_key
                .as_bytes()
                .and_then(|x| <[u8; 32]>::try_from(x).ok())
                .ok_or_else(|| {
                    DOMException::new(
                        "SubjectPublicKeyInfo public key has wrong length",
                        DOMExceptionName::DataError,
                    )
                })?;
            Ok(CryptoKey {
                kind: CryptoKeyKind::X25519Public {
                    algorithm: X25519Algorithm {},
                    key: X25519PublicKey {
                        public_key: agreement::UnparsedPublicKey::new(&X25519, x),
                    },
                },
                r#type: KeyType::Public,
                extractable,
                usages,
            })
        },
        ImportKeyInput::Pkcs8(der) => {
            check_usages_subset(&usages, &[KeyUsage::DeriveKey, KeyUsage::DeriveBits])?;
            let pki = pkcs8::PrivateKeyInfo::from_der(&der).map_err(|_| {
                DOMException::new("invalid X25519 PrivateKeyInfo", DOMExceptionName::DataError)
            })?;
            anyhow::ensure!(
                pki.algorithm.oid == X25519_OID,
                DOMException::new(
                    "PrivateKeyInfo algorithm is not id-X25519",
                    DOMExceptionName::DataError,
                )
            );
            anyhow::ensure!(
                pki.algorithm.parameters.is_none(),
                DOMException::new(
                    "PrivateKeyInfo parameters must not be present",
                    DOMExceptionName::DataError
                )
            );
            // X25519 PKCS#8 PrivateKeyInfo is a CurvePrivateKey, which is an OCTET STRING
            let private_key = OctetStringRef::from_der(pki.private_key)
                .ok()
                .and_then(|pk| agreement::PrivateKey::from_private_key(&X25519, pk.as_bytes()).ok())
                .ok_or_else(|| {
                    DOMException::new("invalid X25519 private key", DOMExceptionName::DataError)
                })?;
            Ok(CryptoKey {
                kind: CryptoKeyKind::X25519Private {
                    algorithm: X25519Algorithm {},
                    key: X25519PrivateKey { private_key },
                },
                r#type: KeyType::Private,
                extractable,
                usages,
            })
        },
        ImportKeyInput::Jwk(jwk) => {
            if jwk.d.is_some() {
                check_usages_subset(&usages, &[KeyUsage::DeriveKey, KeyUsage::DeriveBits])?;
            } else {
                check_usages_subset(&usages, &[])?;
            }
            jwk.check_kty("OKP")?;
            jwk.check_crv("X25519")?;
            jwk.check_key_ops_and_use(&usages, "enc")?;
            jwk.check_ext(extractable)?;
            let x = jwk
                .x
                .as_ref()
                .and_then(|k| base64::decode_config(k, URL_SAFE_FORGIVING).ok())
                .and_then(|x| <[u8; 32]>::try_from(x).ok())
                .ok_or_else(|| {
                    anyhow::anyhow!(DOMException::new(
                        "invalid key `x`",
                        DOMExceptionName::DataError
                    ))
                })?;
            if let Some(d) = jwk.d {
                let private_key = base64::decode_config(&d, URL_SAFE_FORGIVING)
                    .ok()
                    .and_then(|d| agreement::PrivateKey::from_private_key(&X25519, &d).ok())
                    .ok_or_else(|| {
                        anyhow::anyhow!(DOMException::new(
                            "invalid key `d`",
                            DOMExceptionName::DataError
                        ))
                    })?;
                let public_key = private_key.compute_public_key()?;
                anyhow::ensure!(
                    x == public_key.as_ref(),
                    DOMException::new("JWT `d` and `x` do not match", DOMExceptionName::DataError)
                );
                Ok(CryptoKey {
                    kind: CryptoKeyKind::X25519Private {
                        algorithm: X25519Algorithm {},
                        key: X25519PrivateKey { private_key },
                    },
                    r#type: KeyType::Private,
                    extractable,
                    usages,
                })
            } else {
                Ok(CryptoKey {
                    kind: CryptoKeyKind::X25519Public {
                        algorithm: X25519Algorithm {},
                        key: X25519PublicKey {
                            public_key: agreement::UnparsedPublicKey::new(&X25519, x),
                        },
                    },
                    r#type: KeyType::Public,
                    extractable,
                    usages,
                })
            }
        },
        ImportKeyInput::Raw(raw) => {
            check_usages_subset(&usages, &[])?;
            let raw = <[u8; 32]>::try_from(raw).map_err(|_| {
                DOMException::new(
                    "X25519 public key must be 256 bits",
                    DOMExceptionName::DataError,
                )
            })?;
            Ok(CryptoKey {
                kind: CryptoKeyKind::X25519Public {
                    algorithm: X25519Algorithm {},
                    key: X25519PublicKey {
                        public_key: agreement::UnparsedPublicKey::new(&X25519, raw),
                    },
                },
                r#type: KeyType::Public,
                extractable,
                usages,
            })
        },
    }
}

impl X25519PrivateKey {
    pub(crate) fn export_key(&self, format: KeyFormat) -> anyhow::Result<KeyData> {
        match format {
            KeyFormat::Pkcs8 => Ok(KeyData::Raw(
                pkcs8::PrivateKeyInfo {
                    algorithm: spki::AlgorithmIdentifier {
                        oid: X25519_OID,
                        parameters: None,
                    },
                    private_key: &OctetStringRef::new(
                        AsBigEndian::<Curve25519SeedBin>::as_be_bytes(&self.private_key)?.as_ref(),
                    )?
                    .to_der()?,
                    public_key: None,
                }
                .to_der()?,
            )),
            KeyFormat::Jwk => {
                let b64 =
                    |bytes: &[u8]| Some(base64::encode_config(bytes, base64::URL_SAFE_NO_PAD));
                let jwk = JsonWebKey {
                    kty: Some("OKP".to_owned()),
                    crv: Some("X25519".to_owned()),
                    x: b64(self.private_key.compute_public_key()?.as_ref()),
                    d: b64(
                        AsBigEndian::<Curve25519SeedBin>::as_be_bytes(&self.private_key)?.as_ref(),
                    ),
                    ..Default::default()
                };
                Ok(KeyData::Jwk(jwk))
            },
            KeyFormat::Raw | KeyFormat::Spki => anyhow::bail!(DOMException::new(
                "invalid export format for X25519 private key",
                DOMExceptionName::InvalidAccessError
            )),
        }
    }
}

impl X25519PublicKey {
    pub(crate) fn export_key(&self, format: KeyFormat) -> anyhow::Result<KeyData> {
        match format {
            KeyFormat::Spki => Ok(KeyData::Raw(
                spki::SubjectPublicKeyInfo {
                    algorithm: spki::AlgorithmIdentifierOwned {
                        oid: X25519_OID,
                        parameters: None,
                    },
                    subject_public_key: BitStringRef::from_bytes(self.public_key.bytes())?,
                }
                .to_der()?,
            )),
            KeyFormat::Jwk => {
                let b64 =
                    |bytes: &[u8]| Some(base64::encode_config(bytes, base64::URL_SAFE_NO_PAD));
                let jwk = JsonWebKey {
                    kty: Some("OKP".to_owned()),
                    crv: Some("X25519".to_owned()),
                    x: b64(self.public_key.bytes()),
                    ..Default::default()
                };
                Ok(KeyData::Jwk(jwk))
            },
            KeyFormat::Raw => Ok(KeyData::Raw(self.public_key.bytes().to_vec())),
            KeyFormat::Pkcs8 => anyhow::bail!(DOMException::new(
                "invalid export format for X25519 public key",
                DOMExceptionName::InvalidAccessError
            )),
        }
    }
}
