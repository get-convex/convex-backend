use std::ops::{
    Deref,
    DerefMut,
};

use value::heap_size::HeapSize;

use crate::index::IndexKeyBytes;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct BinaryKey {
    key: Vec<u8>,
}

impl HeapSize for BinaryKey {
    fn heap_size(&self) -> usize {
        self.key.heap_size()
    }
}

impl From<IndexKeyBytes> for BinaryKey {
    fn from(key: IndexKeyBytes) -> Self {
        key.0.into()
    }
}

impl From<Vec<u8>> for BinaryKey {
    fn from(key: Vec<u8>) -> Self {
        Self { key }
    }
}

impl From<BinaryKey> for IndexKeyBytes {
    fn from(b: BinaryKey) -> Self {
        IndexKeyBytes(b.into())
    }
}

impl From<BinaryKey> for Vec<u8> {
    fn from(b: BinaryKey) -> Self {
        b.key
    }
}

impl Deref for BinaryKey {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.key[..]
    }
}

impl DerefMut for BinaryKey {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.key[..]
    }
}

impl BinaryKey {
    pub const fn min() -> Self {
        Self { key: Vec::new() }
    }

    /// For any key `k`, `increment(k)` is the minimum key such that for
    /// all keys `s` where `k.is_prefix(s)`, we have `s < increment(k)`.
    pub fn increment(&self) -> Option<Self> {
        let mut incremented = self.clone();
        while let Some(byte) = incremented.last_mut() {
            if *byte < 255 {
                *byte += 1;
                return Some(incremented);
            }
            incremented.key.pop();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use proptest::prelude::*;

    use super::BinaryKey;

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_increment(
            key in any::<BinaryKey>(),
            mut suffix in prop::collection::vec(any::<u8>(), 0..=2),
        ) {
            if let Some(incr) = key.increment() {
                let mut bytes_with_suffix = key.key;
                bytes_with_suffix.append(&mut suffix);
                let with_suffix = BinaryKey{ key: bytes_with_suffix };
                assert!(incr > with_suffix);
            } else {
                assert!(key.iter().all(|b| *b == 255));
            }
        }

        #[test]
        fn test_increment_minimum(
            key in prop::collection::vec(any::<u8>(), 0..=2),
            other_key in prop::collection::vec(any::<u8>(), 0..=2),
        ) {
            let key = BinaryKey { key };
            let other_key = BinaryKey { key: other_key };
            if let Some(incr) = key.increment() {
                if key < other_key && other_key < incr {
                    assert!(other_key.starts_with(&key));
                }
            } else {
                assert!(key.iter().all(|b| *b == 255));
            }
        }
    }

    #[test]
    fn test_increment_samples() {
        let key: BinaryKey = vec![5, 6].into();
        assert_eq!(key.increment(), Some(vec![5, 7].into()));
        let key: BinaryKey = vec![5, 255, 255].into();
        assert_eq!(key.increment(), Some(vec![6].into()));
        let key: BinaryKey = vec![255, 255, 255].into();
        assert_eq!(key.increment(), None);
        let key: BinaryKey = vec![].into();
        assert_eq!(key.increment(), None);
    }
}
