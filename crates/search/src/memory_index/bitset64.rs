use super::iter_set_bits::iter_set_bits;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Bitset64 {
    bits: u64,
}

impl Bitset64 {
    pub fn new() -> Self {
        Self { bits: 0 }
    }

    pub fn insert(&mut self, i: usize) {
        assert!(i < 64);
        self.bits |= 1 << i;
    }

    pub fn contains(self, i: usize) -> bool {
        assert!(i < 64);
        self.bits & (1 << i) != 0
    }

    pub fn rank(self, i: usize) -> usize {
        assert!(i < 64);
        (self.bits & ((1 << i) - 1)).count_ones() as usize
    }

    pub fn iter_ones(self) -> impl Iterator<Item = usize> {
        iter_set_bits(self.bits)
    }

    pub fn intersect(self, rhs: Self) -> Self {
        Self {
            bits: self.bits & rhs.bits,
        }
    }

    pub fn is_empty(self) -> bool {
        self.bits == 0
    }
}
