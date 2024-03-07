// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
// https://github.com/denoland/deno/blob/main/ext/crypto/x25519.rs

use deno_core::{
    JsBuffer,
    ToJsBuffer,
};
use elliptic_curve::pkcs8::PrivateKeyInfo;
use p256::pkcs8::der::Decode as _;
use spki::{
    der::{
        AnyRef,
        Decode,
    },
    SubjectPublicKeyInfo,
};

use super::CryptoOps;

// id-X25519 OBJECT IDENTIFIER ::= { 1 3 101 110 }
pub const X25519_OID: const_oid::ObjectIdentifier =
    const_oid::ObjectIdentifier::new_unwrap("1.3.101.110");

impl CryptoOps {
    pub fn import_spki_x25519(key_data: JsBuffer) -> Option<ToJsBuffer> {
        // 2-3.
        let pk_info: SubjectPublicKeyInfo<AnyRef, Vec<u8>> =
            match spki::SubjectPublicKeyInfo::from_der(&key_data) {
                Ok(pk_info) => pk_info,
                Err(_) => return None,
            };
        // 4.
        let alg = pk_info.algorithm.oid;
        if alg != X25519_OID {
            return None;
        }
        // 5.
        if pk_info.algorithm.parameters.is_some() {
            return None;
        }
        Some(pk_info.subject_public_key.into())
    }

    pub fn import_pkcs8_x25519(key_data: JsBuffer) -> Option<ToJsBuffer> {
        // 2-3.
        // This should probably use OneAsymmetricKey instead
        let pk_info = match PrivateKeyInfo::from_der(&key_data) {
            Ok(pk_info) => pk_info,
            Err(_) => return None,
        };
        // 4.
        let alg = pk_info.algorithm.oid;
        if alg != X25519_OID {
            return None;
        }
        // 5.
        if pk_info.algorithm.parameters.is_some() {
            return None;
        }
        // 6.
        // CurvePrivateKey ::= OCTET STRING
        if pk_info.private_key.len() != 34 {
            return None;
        }
        Some(pk_info.private_key[2..].to_vec().into())
    }
}
