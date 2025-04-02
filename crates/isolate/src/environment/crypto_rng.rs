/// Represents access to an RNG suitable for cryptographic operations like key
/// generation, i.e. system randomness.
///
/// This is unavailable in deterministic UDFs (i.e. queries/mutations).
pub struct CryptoRng(());

impl CryptoRng {
    pub fn new() -> Self {
        CryptoRng(())
    }

    /// Returns a `ring`-compatible random number generator
    pub fn ring(&self) -> ring::rand::SystemRandom {
        ring::rand::SystemRandom::new()
    }

    /// Returns an `rsa`-compatible random number generator
    pub fn rsa(&self) -> rsa::rand_core::OsRng {
        rsa::rand_core::OsRng
    }
}
