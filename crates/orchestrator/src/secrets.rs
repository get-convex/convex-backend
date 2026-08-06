//! Sealing for secrets the orchestrator stores but must never hand back:
//! DNS provider API tokens and the ACME account key.
//!
//! Both are written to Postgres, so a database dump alone shouldn't be enough
//! to take over someone's DNS zone or mint certificates in our name. The key
//! is derived from the orchestrator's service key, which already has to be
//! secret and is supplied out of band.

use anyhow::Context;
use sha2::{
    Digest,
    Sha256,
};

/// Nonce is prepended to the ciphertext, so a sealed blob is
/// `[nonce][ciphertext]` and callers only ever handle one opaque `Vec<u8>`.
#[derive(Clone)]
pub struct SecretSealer {
    key: sodium_secretbox::Key,
}

impl SecretSealer {
    /// Derives the sealing key from the service key.
    ///
    /// Hashing rather than truncating means any length of service key works,
    /// and the sealing key isn't literally the service key — so a leak of one
    /// derived value doesn't hand over the other.
    pub fn from_service_key(service_key: &str) -> anyhow::Result<Self> {
        let mut hasher = Sha256::new();
        hasher.update(b"convex-orchestrator/secret-sealer/v1");
        hasher.update(service_key.as_bytes());
        let digest = hasher.finalize();
        let key = sodium_secretbox::Key::from_slice(&digest)
            .context("deriving secret sealing key")?;
        Ok(Self { key })
    }

    pub fn seal(&self, plaintext: &[u8]) -> Vec<u8> {
        let nonce = sodium_secretbox::gen_nonce();
        let ciphertext = sodium_secretbox::seal(plaintext, &nonce, &self.key);
        let mut out = Vec::with_capacity(sodium_secretbox::NONCEBYTES + ciphertext.len());
        out.extend_from_slice(&nonce.0);
        out.extend_from_slice(&ciphertext);
        out
    }

    pub fn open(&self, sealed: &[u8]) -> anyhow::Result<Vec<u8>> {
        anyhow::ensure!(
            sealed.len() > sodium_secretbox::NONCEBYTES,
            "sealed value is too short to contain a nonce"
        );
        let (nonce_bytes, ciphertext) = sealed.split_at(sodium_secretbox::NONCEBYTES);
        let nonce = sodium_secretbox::Nonce::from_slice(nonce_bytes)
            .context("reading nonce from sealed value")?;
        sodium_secretbox::open(ciphertext, &nonce, &self.key)
            .map_err(|_| anyhow::anyhow!("could not decrypt stored secret — has SERVICE_KEY changed?"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_secret() {
        let sealer = SecretSealer::from_service_key("service-key").unwrap();
        let sealed = sealer.seal(b"cf-token");
        assert_eq!(sealer.open(&sealed).unwrap(), b"cf-token");
    }

    #[test]
    fn ciphertext_does_not_contain_the_plaintext() {
        let sealer = SecretSealer::from_service_key("service-key").unwrap();
        let sealed = sealer.seal(b"cf-token");
        assert!(!sealed.windows(8).any(|w| w == b"cf-token"));
    }

    #[test]
    fn reuses_a_fresh_nonce_per_seal() {
        // Identical plaintexts must not produce identical ciphertexts, or the
        // database leaks which deployments share a token.
        let sealer = SecretSealer::from_service_key("service-key").unwrap();
        assert_ne!(sealer.seal(b"same"), sealer.seal(b"same"));
    }

    #[test]
    fn a_different_service_key_cannot_open_the_secret() {
        let sealed = SecretSealer::from_service_key("one").unwrap().seal(b"secret");
        assert!(SecretSealer::from_service_key("two")
            .unwrap()
            .open(&sealed)
            .is_err());
    }

    #[test]
    fn rejects_truncated_values() {
        let sealer = SecretSealer::from_service_key("service-key").unwrap();
        assert!(sealer.open(b"short").is_err());
    }
}
