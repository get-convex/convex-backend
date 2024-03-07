// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
// https://github.com/denoland/deno/blob/main/ext/crypto/shared.rs

use std::borrow::Cow;

use deno_core::{
    JsBuffer,
    ToJsBuffer,
};
use errors::ErrorMetadata;
use serde::{
    Deserialize,
    Serialize,
};

pub type AnyError = anyhow::Error;

pub const RSA_ENCRYPTION_OID: const_oid::ObjectIdentifier =
    const_oid::ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.1");

pub const ID_SECP256R1_OID: const_oid::ObjectIdentifier =
    const_oid::ObjectIdentifier::new_unwrap("1.2.840.10045.3.1.7");
pub const ID_SECP384R1_OID: const_oid::ObjectIdentifier =
    const_oid::ObjectIdentifier::new_unwrap("1.3.132.0.34");
pub const ID_SECP521R1_OID: const_oid::ObjectIdentifier =
    const_oid::ObjectIdentifier::new_unwrap("1.3.132.0.35");

#[derive(Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
pub enum ShaHash {
    #[serde(rename = "SHA-1")]
    Sha1,
    #[serde(rename = "SHA-256")]
    Sha256,
    #[serde(rename = "SHA-384")]
    Sha384,
    #[serde(rename = "SHA-512")]
    Sha512,
}

#[derive(Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
pub enum EcNamedCurve {
    #[serde(rename = "P-256")]
    P256,
    #[serde(rename = "P-384")]
    P384,
    #[serde(rename = "P-521")]
    P521,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase", tag = "type", content = "data")]
pub enum V8RawKeyData {
    Secret(JsBuffer),
    Private(JsBuffer),
    Public(JsBuffer),
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase", tag = "type", content = "data")]
pub enum RustRawKeyData {
    Secret(ToJsBuffer),
    Private(ToJsBuffer),
    Public(ToJsBuffer),
}

pub fn data_error(msg: impl Into<Cow<'static, str>>) -> AnyError {
    // TODO(CX-5961): throw as a DOMExceptionDataError into js.
    anyhow::anyhow!(ErrorMetadata::bad_request("DOMExceptionDataError", msg))
}

pub fn not_supported_error(msg: impl Into<Cow<'static, str>>) -> AnyError {
    // TODO(CX-5961): throw as a DOMExceptionNotSupportedError into js.
    anyhow::anyhow!(ErrorMetadata::bad_request(
        "DOMExceptionNotSupportedError",
        msg
    ))
}

pub fn unsupported_format() -> AnyError {
    not_supported_error("unsupported format")
}
