#[allow(unused)]
// Adapted from https://lemire.me/blog/2018/02/21/iterating-over-set-bits-quickly/
pub fn iter_set_bits(mut bitset: u64) -> impl Iterator<Item = u32> {
    std::iter::from_coroutine(move || {
        while bitset != 0 {
            // The main trick for this algorithm is that `t` has all bits off except for the
            // least significant bit of `bitset`. The intuition here is that
            //
            //   bitset.wrapping_neg() + bitset = 2^64 (mod 2^64),
            //
            // so we can fully determine `bitset.wrapping_neg()` by working from
            // right-to-left.
            //
            // bitset:                1 0 ... 0 0 1 0 0 1 0 1 1 0 0 0 0
            // bitset.wrapping_neg(): ? ? ... ? ? ? ? ? ? ? ? ? ? ? ? ?
            // 2^64 mod 2^64 = 0:     0 0 ... 0 0 0 0 0 0 0 0 0 0 0 0 0
            //
            // For all bits to the right of the least significant bit, we must set them to
            // zero.
            //
            // bitset:                1 0 ... 0 0 1 0 0 1 0 1 1 0 0 0 0
            // bitset.wrapping_neg(): ? ? ... ? ? ? ? ? ? ? ? ? 0 0 0 0
            // 2^64 mod 2^64 = 0:     0 0 ... 0 0 0 0 0 0 0 0 0 0 0 0 0
            //
            // Then, we have to set the position of the least significant bit to one to
            // cancel it out, which then causes a carry.
            //
            // carry:                                       1
            // bitset:                1 0 ... 0 0 1 0 0 1 0 1 1 0 0 0 0
            // bitset.wrapping_neg(): ? ? ... ? ? ? ? ? ? ? ? 0 0 0 0 0
            // 2^64 mod 2^64 = 0:     0 0 ... 0 0 0 0 0 0 0 0 0 0 0 0 0
            //
            // There are two cases: If the current bit is set, our negation must be off
            // since the sum is already zero with the carry.
            //
            // carry:                                       1
            // bitset:                1 0 ... 0 0 1 0 0 1 0 1 1 0 0 0 0
            // bitset.wrapping_neg(): ? ? ... ? ? ? ? ? ? ? 0 0 0 0 0 0
            // 2^64 mod 2^64 = 0:     0 0 ... 0 0 0 0 0 0 0 0 0 0 0 0 0
            //
            // This then causes the carry to propagate to the next bit, and if it's zero in
            // `bitset`, we have to turn it on in the negation.
            //
            // carry:                                     1 1
            // bitset:                1 0 ... 0 0 1 0 0 1 0 1 1 0 0 0 0
            // bitset.wrapping_neg(): ? ? ... ? ? ? ? ? ? 1 0 0 0 0 0 0
            // 2^64 mod 2^64 = 0:     0 0 ... 0 0 0 0 0 0 0 0 0 0 0 0 0
            //
            // The end result is that we continue to carry until the end of the u64, and all
            // of the bits are flipped past the least significant bit.
            // Therefore, `AND`ing the two together only leaves the least
            // significant bit on.
            let t = bitset & bitset.wrapping_neg();
            yield bitset.trailing_zeros();
            bitset ^= t;
        }
    })
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig { cases: 65536 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

        #[test]
        fn test_iter_set_bits(bitset in any::<u64>()) {
            let set_bits = (0..64).filter(|i| bitset & (1 << i) != 0);
            assert!(iter_set_bits(bitset).eq(set_bits));
        }
    }

    #[test]
    fn test_iter_set_bits_exhaustive_u16() {
        for bitset in 0u16..=65535 {
            let set_bits = (0..16).filter(|i| bitset & (1 << i) != 0);
            assert!(iter_set_bits(bitset as u64).eq(set_bits), "{bitset}");
        }
    }
}
