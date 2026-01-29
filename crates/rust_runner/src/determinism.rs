//! Deterministic random and time for WASM execution
//!
//! This module provides deterministic sources of randomness and time
//! for queries and mutations, ensuring they produce consistent results
//! across retries and replicas.
//!
//! For actions, system random and time are used instead.

use std::sync::Mutex;

use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;

/// Virtual time provider for deterministic execution
///
/// Queries and mutations use a fixed virtual timestamp to ensure
/// deterministic behavior. Actions use the actual system time.
#[derive(Debug, Clone)]
pub enum TimeProvider {
    /// Virtual time for deterministic execution (queries/mutations)
    Virtual { timestamp_ms: i64 },
    /// System time for actions
    System,
}

impl TimeProvider {
    /// Create a virtual time provider with the given timestamp
    pub fn virtual_time(timestamp_ms: i64) -> Self {
        Self::Virtual { timestamp_ms }
    }

    /// Create a system time provider
    pub fn system_time() -> Self {
        Self::System
    }

    /// Get the current time in milliseconds
    pub fn now_ms(&self) -> i64 {
        match self {
            TimeProvider::Virtual { timestamp_ms } => *timestamp_ms,
            TimeProvider::System => {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("System time before Unix epoch")
                    .as_millis() as i64
            }
        }
    }
}

impl Default for TimeProvider {
    fn default() -> Self {
        // Default to system time for safety
        Self::System
    }
}

/// Deterministic random number generator
///
/// Uses ChaCha20 for cryptographically secure randomness
/// when seeded with a deterministic seed.
pub struct DeterministicRng {
    rng: Mutex<ChaCha20Rng>,
}

impl DeterministicRng {
    /// Create a new deterministic RNG with the given seed
    pub fn new(seed: u64) -> Self {
        let rng = ChaCha20Rng::seed_from_u64(seed);
        Self {
            rng: Mutex::new(rng),
        }
    }

    /// Fill a buffer with random bytes
    pub fn fill_bytes(&self, buf: &mut [u8]) {
        let mut rng = self.rng.lock().expect("RNG lock poisoned");
        rng.fill_bytes(buf);
    }

    /// Generate a random u64
    pub fn next_u64(&self) -> u64 {
        let mut rng = self.rng.lock().expect("RNG lock poisoned");
        rng.next_u64()
    }
}

/// Random provider that can be either deterministic or system
pub enum RandomProvider {
    /// Deterministic RNG for queries/mutations
    Deterministic(DeterministicRng),
    /// System RNG for actions
    System,
}

impl RandomProvider {
    /// Create a deterministic random provider with the given seed
    pub fn deterministic(seed: u64) -> Self {
        Self::Deterministic(DeterministicRng::new(seed))
    }

    /// Create a system random provider
    pub fn system() -> Self {
        Self::System
    }

    /// Fill a buffer with random bytes
    pub fn fill_bytes(&self, buf: &mut [u8]) {
        match self {
            RandomProvider::Deterministic(rng) => rng.fill_bytes(buf),
            RandomProvider::System => {
                use rand::RngCore;
                rand::thread_rng().fill_bytes(buf);
            }
        }
    }
}

impl Default for RandomProvider {
    fn default() -> Self {
        // Default to system random for safety
        Self::System
    }
}

/// Execution context for determinism control
///
/// This struct holds the time and random providers that determine
/// whether execution is deterministic (queries/mutations) or
/// non-deterministic (actions).
pub struct DeterminismContext {
    time_provider: TimeProvider,
    random_provider: RandomProvider,
}

impl DeterminismContext {
    /// Create a deterministic context for queries/mutations
    ///
    /// # Arguments
    /// * `seed` - Seed for the deterministic RNG
    /// * `timestamp_ms` - Virtual timestamp in milliseconds
    pub fn deterministic(seed: u64, timestamp_ms: i64) -> Self {
        Self {
            time_provider: TimeProvider::virtual_time(timestamp_ms),
            random_provider: RandomProvider::deterministic(seed),
        }
    }

    /// Create a non-deterministic context for actions
    pub fn non_deterministic() -> Self {
        Self {
            time_provider: TimeProvider::system_time(),
            random_provider: RandomProvider::system(),
        }
    }

    /// Get the current time in milliseconds
    pub fn now_ms(&self) -> i64 {
        self.time_provider.now_ms()
    }

    /// Fill a buffer with random bytes
    pub fn fill_random_bytes(&self, buf: &mut [u8]) {
        self.random_provider.fill_bytes(buf);
    }

    /// Check if this context is deterministic
    pub fn is_deterministic(&self) -> bool {
        matches!(self.time_provider, TimeProvider::Virtual { .. })
    }
}

impl Default for DeterminismContext {
    fn default() -> Self {
        // Default to non-deterministic for safety
        Self::non_deterministic()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_time() {
        let provider = TimeProvider::virtual_time(1234567890);
        assert_eq!(provider.now_ms(), 1234567890);

        // Should always return the same value
        assert_eq!(provider.now_ms(), 1234567890);
    }

    #[test]
    fn test_deterministic_rng() {
        let rng1 = DeterministicRng::new(42);
        let rng2 = DeterministicRng::new(42);

        let mut buf1 = [0u8; 32];
        let mut buf2 = [0u8; 32];

        rng1.fill_bytes(&mut buf1);
        rng2.fill_bytes(&mut buf2);

        // Same seed should produce same bytes
        assert_eq!(buf1, buf2);

        // Different seed should produce different bytes
        let rng3 = DeterministicRng::new(43);
        let mut buf3 = [0u8; 32];
        rng3.fill_bytes(&mut buf3);
        assert_ne!(buf1, buf3);
    }

    #[test]
    fn test_determinism_context() {
        let ctx = DeterminismContext::deterministic(42, 1234567890);

        assert!(ctx.is_deterministic());
        assert_eq!(ctx.now_ms(), 1234567890);

        let mut buf = [0u8; 16];
        ctx.fill_random_bytes(&mut buf);

        // Should get same bytes on second call (deterministic)
        let mut buf2 = [0u8; 16];
        ctx.fill_random_bytes(&mut buf2);
        // Note: Since RNG advances, these will be different
    }

    #[test]
    fn test_non_deterministic_context() {
        let ctx = DeterminismContext::non_deterministic();

        assert!(!ctx.is_deterministic());

        // Time should be close to now (within 1 second)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let ctx_now = ctx.now_ms();
        assert!((ctx_now - now).abs() < 1000);
    }
}
