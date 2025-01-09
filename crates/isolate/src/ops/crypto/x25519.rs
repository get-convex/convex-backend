// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
// https://github.com/denoland/deno/blob/main/ext/crypto/x25519.rs

use deno_core::ToJsBuffer;
use elliptic_curve::pkcs8::PrivateKeyInfo;
use spki::{
    der::{
        asn1::BitString,
        AnyRef,
        Decode,
        Encode,
    },
    SubjectPublicKeyInfo,
};

use super::{
    shared::{
        custom_error,
        AnyError,
    },
    CryptoOps,
};

// id-X25519 OBJECT IDENTIFIER ::= { 1 3 101 110 }
pub const X25519_OID: const_oid::ObjectIdentifier =
    const_oid::ObjectIdentifier::new_unwrap("1.3.101.110");

impl CryptoOps {
    pub fn import_spki_x25519(key_data: Vec<u8>) -> Option<ToJsBuffer> {
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

    pub fn import_pkcs8_x25519(key_data: Vec<u8>) -> Option<ToJsBuffer> {
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

    pub fn export_spki_x25519(pubkey: &[u8]) -> Result<ToJsBuffer, AnyError> {
        let key_info = spki::SubjectPublicKeyInfo {
            algorithm: spki::AlgorithmIdentifierRef {
                // id-X25519
                oid: X25519_OID,
                parameters: None,
            },
            subject_public_key: BitString::from_bytes(pubkey)?,
        };
        Ok(key_info
            .to_der()
            .map_err(|_| custom_error("DOMExceptionOperationError", "Failed to export key"))?
            .into())
    }

    pub fn export_pkcs8_x25519(pkey: &[u8]) -> Result<ToJsBuffer, AnyError> {
        // This should probably use OneAsymmetricKey instead
        let pk_info = rsa::pkcs8::PrivateKeyInfo {
            public_key: None,
            algorithm: rsa::pkcs8::AlgorithmIdentifierRef {
                // id-X25519
                oid: X25519_OID,
                parameters: None,
            },
            private_key: pkey, // OCTET STRING
        };

        let mut buf = Vec::new();
        pk_info.encode_to_vec(&mut buf)?;
        Ok(buf.into())
    }
}
