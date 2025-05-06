use anyhow::Context as _;
use deno_core::ToJsBuffer;
use openssl::{
    bn::BigNum,
    rsa::Rsa,
};
use ring::{
    rand::SecureRandom,
    signature::{
        EcdsaKeyPair,
        Ed25519KeyPair,
        KeyPair,
    },
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
                let exp = BigNum::from_slice(&public_exponent)?;
                let private_key = Rsa::generate_with_e(
                    modulus_length.try_into().context("bad modulus length")?,
                    &exp,
                )?;
                Ok(GeneratedKeypair {
                    private_raw_data: GeneratedKey::KeyData(RustRawKeyData::Private(
                        private_key.private_key_to_der()?.into(),
                    )),
                    public_raw_data: GeneratedKey::KeyData(RustRawKeyData::Public(
                        private_key.public_key_to_der_pkcs1()?.into(),
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
