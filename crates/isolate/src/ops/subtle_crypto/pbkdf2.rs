use std::num::NonZeroU32;

use aws_lc_rs::pbkdf2;
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
    KeyType,
    KeyUsage,
    DERIVE_BITS_MAX,
};
use crate::convert_v8::{
    DOMException,
    DOMExceptionName,
};

pub(crate) struct Pbkdf2Key {
    secret: Vec<u8>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ImportKeyAlgorithm;

#[derive(Deserialize, Debug)]
pub(crate) struct Pbkdf2Params {
    #[serde(with = "super::nullary_algorithm")]
    hash: CryptoHash,
    iterations: u32,
    salt: serde_bytes::ByteBuf,
}

#[derive(Serialize, Debug)]
#[serde(rename = "PBKDF2")]
#[serde(tag = "name")]
pub(crate) struct Pbkdf2Algorithm {}

pub(crate) fn derive_bits(
    algorithm: Pbkdf2Params,
    key: &CryptoKey,
    length: Option<usize>,
) -> anyhow::Result<Vec<u8>> {
    let Pbkdf2Params {
        hash,
        iterations,
        salt,
    } = algorithm;
    let Some(iterations) = NonZeroU32::new(iterations) else {
        anyhow::bail!(DOMException::new(
            "iterations cannot be zero",
            DOMExceptionName::OperationError
        ))
    };
    let Some(length) = length else {
        anyhow::bail!(DOMException::new(
            "length cannot be null",
            DOMExceptionName::OperationError
        ))
    };
    anyhow::ensure!(
        length % 8 == 0,
        DOMException::new(
            "length must be a multiple of 8",
            DOMExceptionName::OperationError
        )
    );
    anyhow::ensure!(
        length <= DERIVE_BITS_MAX,
        DOMException::new(
            format!("cannot generate more than {DERIVE_BITS_MAX} bits"),
            DOMExceptionName::OperationError
        )
    );
    let CryptoKeyKind::Pbkdf2 { key, .. } = &key.kind else {
        anyhow::bail!(DOMException::new(
            "Key algorithm mismatch",
            DOMExceptionName::InvalidAccessError
        ))
    };
    let algorithm = match hash {
        CryptoHash::Sha1 => pbkdf2::PBKDF2_HMAC_SHA1,
        CryptoHash::Sha256 => pbkdf2::PBKDF2_HMAC_SHA256,
        CryptoHash::Sha384 => pbkdf2::PBKDF2_HMAC_SHA384,
        CryptoHash::Sha512 => pbkdf2::PBKDF2_HMAC_SHA512,
    };
    let secret = &key.secret;
    let mut out = vec![0; length / 8];
    pbkdf2::derive(algorithm, iterations, &salt, secret, &mut out);
    Ok(out)
}

pub(crate) fn import_key(
    format: ImportKeyInput,
    extractable: bool,
    usages: IndexSet<KeyUsage>,
) -> anyhow::Result<CryptoKey> {
    let ImportKeyInput::Raw(secret) = format else {
        anyhow::bail!(DOMException::new(
            "unsupported input format",
            DOMExceptionName::NotSupportedError
        ))
    };
    check_usages_subset(&usages, &[KeyUsage::DeriveKey, KeyUsage::DeriveBits])?;
    anyhow::ensure!(
        !extractable,
        DOMException::new(
            "PBKDF2 keys cannot be extractable",
            DOMExceptionName::SyntaxError
        )
    );
    Ok(CryptoKey {
        kind: CryptoKeyKind::Pbkdf2 {
            algorithm: Pbkdf2Algorithm {},
            key: Pbkdf2Key { secret },
        },
        r#type: KeyType::Secret,
        extractable,
        usages,
    })
}
