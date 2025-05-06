use anyhow::Context as _;
use aws_lc_rs::{
    encoding::AsBigEndian as _,
    rand::SecureRandom,
    signature::{
        EcdsaKeyPair,
        Ed25519KeyPair,
        KeyPair,
    },
};
use deno_core::ToJsBuffer;
use openssl::{
    bn::BigNum,
    rsa::Rsa,
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
        _rng: CryptoRng,
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
                let keypair = EcdsaKeyPair::generate(named_curve.into())
                    .context("failed to generate ecdsa keypair")?;
                Ok(GeneratedKeypair {
                    private_raw_data: GeneratedKey::KeyData(RustRawKeyData::Private(
                        // private key is PKCS#8-encoded
                        keypair
                            .to_pkcs8v1()
                            .context("failed to serialize ecdsa keypair")?
                            .as_ref()
                            .to_vec()
                            .into(),
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
                let keypair =
                    Ed25519KeyPair::generate().context("failed to generate ed25519 key")?;
                let public_key = keypair.public_key().as_ref().to_vec();
                let private_key = keypair
                    .seed()
                    .context("failed to get generated ed25519 seed")?
                    .as_be_bytes()
                    .context("failed to get generated ed25519 seed")?;
                Ok(GeneratedKeypair {
                    private_raw_data: GeneratedKey::RawBytes(private_key.as_ref().to_vec().into()),
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
        rng.aws_lc()
            .fill(&mut buf)
            .ok()
            .context("failed to generate random bytes")?;
        Ok(buf.into())
    }
}
