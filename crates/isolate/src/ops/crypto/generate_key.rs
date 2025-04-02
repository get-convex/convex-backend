use anyhow::Context as _;
use deno_core::ToJsBuffer;
use ring::{
    rand::SecureRandom,
    signature::{
        EcdsaKeyPair,
        Ed25519KeyPair,
        KeyPair,
    },
};
use rsa::pkcs1::{
    EncodeRsaPrivateKey,
    EncodeRsaPublicKey,
};

use super::{
    shared::RustRawKeyData,
    Algorithm,
    CryptoOps,
    Curve25519Algorithm,
    GenerateKeypairAlgorithm,
    GeneratedKey,
    GeneratedKeypair,
};
use crate::environment::crypto_rng::CryptoRng;

impl CryptoOps {
    pub fn generate_keypair(
        rng: CryptoRng,
        algorithm: GenerateKeypairAlgorithm,
    ) -> anyhow::Result<GeneratedKeypair> {
        match algorithm {
            GenerateKeypairAlgorithm::Rsa {
                name: Algorithm::RsaPss | Algorithm::RsaOaep | Algorithm::RsassaPkcs1v15,
                modulus_length,
                public_exponent,
            } => {
                let exp = rsa::BigUint::from_bytes_be(&public_exponent);
                let private_key =
                    rsa::RsaPrivateKey::new_with_exp(&mut rng.rsa(), modulus_length, &exp)?;
                let public_key = private_key.to_public_key();
                Ok(GeneratedKeypair {
                    private_raw_data: GeneratedKey::KeyData(RustRawKeyData::Private(
                        private_key.to_pkcs1_der()?.as_bytes().to_vec().into(),
                    )),
                    public_raw_data: GeneratedKey::KeyData(RustRawKeyData::Public(
                        public_key.to_pkcs1_der()?.into_vec().into(),
                    )),
                })
            },
            GenerateKeypairAlgorithm::Ec {
                name: Algorithm::Ecdsa | Algorithm::Ecdh,
                named_curve,
            } => {
                let private_key_pkcs8 =
                    EcdsaKeyPair::generate_pkcs8(named_curve.into(), &rng.ring())
                        .ok()
                        .context("failed to generate ecdsa keypair")?;
                let keypair = EcdsaKeyPair::from_pkcs8(
                    named_curve.into(),
                    private_key_pkcs8.as_ref(),
                    &rng.ring(),
                )
                .ok()
                .context("failed to parse ecdsa pkcs8 that we just generated")?;
                Ok(GeneratedKeypair {
                    private_raw_data: GeneratedKey::KeyData(RustRawKeyData::Private(
                        // private key is PKCS#8-encoded
                        private_key_pkcs8.as_ref().to_vec().into(),
                    )),
                    public_raw_data: GeneratedKey::KeyData(RustRawKeyData::Public(
                        // public key is just the elliptic curve point
                        keypair.public_key().as_ref().to_vec().into(),
                    )),
                })
            },
            GenerateKeypairAlgorithm::Curve25519 {
                name: Curve25519Algorithm::Ed25519,
            } => {
                let pkcs8_keypair = Ed25519KeyPair::generate_pkcs8(&rng.ring())
                    .ok()
                    .context("failed to generate ed25519 key")?;
                // ring is really annoying and needs to jump through hoops to get the public key
                // that we just generated
                let public_key = Ed25519KeyPair::from_pkcs8(pkcs8_keypair.as_ref())
                    .ok()
                    .context("failed to parse ed25519 pkcs8 that we just generated")?
                    .public_key()
                    .as_ref()
                    .to_vec();
                // ring is really really annoying and doesn't export the raw
                // seed at all, so use RustCrypto instead
                let private_key = Self::import_pkcs8_ed25519(pkcs8_keypair.as_ref())
                    .context("failed to import ed25519 pkcs8 that we just generated")?;
                Ok(GeneratedKeypair {
                    private_raw_data: GeneratedKey::RawBytes(private_key),
                    public_raw_data: GeneratedKey::RawBytes(public_key.into()),
                })
            },
            GenerateKeypairAlgorithm::Curve25519 {
                name: Curve25519Algorithm::X25519,
            } => {
                // ring refuses to generate exportable X25519 keys
                // (this should be unreachable as X25519 is rejected in the UDF runtime as well)
                anyhow::bail!("TODO: not yet supported");
            },
            _ => anyhow::bail!("invalid algorithm"),
        }
    }

    pub fn generate_key_bytes(rng: CryptoRng, length: usize) -> anyhow::Result<ToJsBuffer> {
        anyhow::ensure!(length <= 1024, "key too long");
        let mut buf = vec![0; length];
        rng.ring()
            .fill(&mut buf)
            .ok()
            .context("failed to generate random bytes")?;
        Ok(buf.into())
    }
}
