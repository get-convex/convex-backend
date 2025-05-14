/// This module provides functionality for representing sets of intervals on
/// strings of bytes.
///
/// The set of byte strings has a few useful properties:
/// 1. There is a minimum (the empty string)
/// 2. Every string `s` has a smallest string `t` such that `s < t`, its
///    successor. Note that since we don't put a bound on the length of these
///    strings, strings have successors but not predecessors.
///
/// With these properties, we can simplify our intervals greatly:
/// 1. Every interval can be formed as an inclusive lower bound and exclusive
///    upper bound.
/// 2. We don't need to represent -inf in our system, just +inf.
mod bounds;
mod interval_set;
mod key;

#[cfg(any(test, feature = "testing"))]
pub mod test_helpers;

use std::ops::{
    Bound,
    RangeBounds,
};

pub use self::{
    bounds::{
        End,
        EndRef,
        StartIncluded,
    },
    interval_set::IntervalSet,
    key::BinaryKey,
};
use crate::{
    index::IndexKeyBytes,
    query::{
        CursorPosition,
        Order,
    },
};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct Interval {
    pub start: StartIncluded,
    pub end: End,
}

impl Interval {
    pub fn prefix(key: BinaryKey) -> Self {
        let end = End::after_prefix(&key);
        Self {
            start: StartIncluded(key),
            end,
        }
    }

    pub const fn empty() -> Self {
        Self {
            start: StartIncluded(BinaryKey::min()),
            end: End::Excluded(BinaryKey::min()),
        }
    }

    pub const fn all() -> Self {
        Self {
            start: StartIncluded(BinaryKey::min()),
            end: End::Unbounded,
        }
    }

    pub const fn as_ref(&self) -> IntervalRef<'_> {
        let Self { start, end } = self;
        IntervalRef {
            start: start.0.as_slice(),
            end: end.as_ref(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.as_ref().is_empty()
    }

    pub fn is_superset(&self, other: &Self) -> bool {
        other.is_empty() || self.start <= other.start && other.end <= self.end
    }

    pub fn contains(&self, point: &[u8]) -> bool {
        self.as_ref().contains(point)
    }

    pub fn contains_cursor(&self, cursor: &CursorPosition) -> bool {
        match cursor {
            CursorPosition::After(last_key) => self.contains(last_key),
            CursorPosition::End => true,
        }
    }

    pub fn is_disjoint(&self, other: &Self) -> bool {
        self.is_empty()
            || other.is_empty()
            || other.end.is_disjoint(&self.start)
            || self.end.is_disjoint(&other.start)
    }

    pub fn is_adjacent(&self, other: &Self) -> bool {
        if self.is_empty() || other.is_empty() {
            return false;
        }
        self.end.is_adjacent(&other.start) || other.end.is_adjacent(&self.start)
    }

    /// When reading from self in order `order`, if we've just read `last_key`,
    /// returns (interval read, interval remaining).
    /// If self=[X, Y) and order=Asc, returns [X, last_key] and (last_key, Y).
    /// If self=[X, Y) and order=Desc, returns [last_key, Y) and [X, last_key).
    /// Note last_key must be a full IndexKeyBytes, not an arbitrary BinaryKey,
    /// so we can assume there are no other IndexKeyBytes that have `index_key`
    /// as a prefix.
    pub fn split_after(&self, last_key: IndexKeyBytes, order: Order) -> (Self, Self) {
        let last_key_binary = BinaryKey::from(last_key);
        match order {
            Order::Asc => (
                Self {
                    start: self.start.clone(),
                    end: End::after_prefix(&last_key_binary),
                },
                match last_key_binary.increment() {
                    Some(last_key_incr) => Self {
                        start: StartIncluded(last_key_incr),
                        end: self.end.clone(),
                    },
                    None => Interval::empty(),
                },
            ),
            Order::Desc => (
                Self {
                    start: StartIncluded(last_key_binary.clone()),
                    end: self.end.clone(),
                },
                Self {
                    start: self.start.clone(),
                    end: End::Excluded(last_key_binary),
                },
            ),
        }
    }

    pub fn split(&self, cursor: CursorPosition, order: Order) -> (Self, Self) {
        match cursor {
            CursorPosition::After(last_key) => self.split_after(last_key, order),
            CursorPosition::End => (self.clone(), Interval::empty()),
        }
    }
}

impl RangeBounds<[u8]> for &Interval {
    fn start_bound(&self) -> Bound<&[u8]> {
        let StartIncluded(ref s) = self.start;
        Bound::Included(&s[..])
    }

    fn end_bound(&self) -> Bound<&[u8]> {
        match self.end {
            End::Excluded(ref s) => Bound::Excluded(&s[..]),
            End::Unbounded => Bound::Unbounded,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IntervalRef<'a> {
    pub start: &'a [u8],
    pub end: EndRef<'a>,
}

impl IntervalRef<'_> {
    pub fn all() -> Self {
        static ALL: Interval = Interval::all();
        ALL.as_ref()
    }

    pub fn is_empty(&self) -> bool {
        match (self.start, self.end) {
            (_, EndRef::Unbounded) => false,
            (s, EndRef::Excluded(t)) => s >= t,
        }
    }

    pub fn contains(&self, point: &[u8]) -> bool {
        let after_start = self.start <= point;
        let before_end = match self.end {
            EndRef::Excluded(t) => point < t,
            EndRef::Unbounded => true,
        };
        after_start && before_end
    }

    pub fn to_owned(&self) -> Interval {
        Interval {
            start: StartIncluded(self.start.to_vec().into()),
            end: self.end.to_owned(),
        }
    }
}

impl RangeBounds<[u8]> for IntervalRef<'_> {
    fn start_bound(&self) -> Bound<&[u8]> {
        Bound::Included(self.start)
    }

    fn end_bound(&self) -> Bound<&[u8]> {
        match self.end {
            EndRef::Excluded(s) => Bound::Excluded(s),
            EndRef::Unbounded => Bound::Unbounded,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use cmd_util::env::env_config;
    use proptest::prelude::*;

    use super::{
        bounds::{
            End,
            StartIncluded,
        },
        key::BinaryKey,
        test_helpers::*,
        Interval,
    };

    fn test_bounded_intervals(
        reference: BTreeSet<BinaryKey>,
        interval: Interval,
        other_reference: BTreeSet<BinaryKey>,
        other_interval: Interval,
        query: BinaryKey,
    ) {
        assert_eq!(reference.is_empty(), interval.is_empty());

        assert_eq!(reference.contains(&query), interval.contains(&query[..]));
        assert_eq!(
            reference.is_superset(&other_reference),
            interval.is_superset(&other_interval),
        );
        assert_eq!(
            reference.is_disjoint(&other_reference),
            interval.is_disjoint(&other_interval),
        );
        let mut is_adjacent = false;
        if let Some(end) = other_reference.iter().next_back() {
            let next = end
                .increment()
                .or_else(|| end.is_empty().then_some(vec![0].into()));
            if let Some(next) = next {
                is_adjacent |= Some(&next) == reference.iter().next();
            }
        }
        if let Some(end) = reference.iter().next_back() {
            let next = end
                .increment()
                .or_else(|| end.is_empty().then_some(vec![0].into()));
            if let Some(next) = next {
                is_adjacent |= Some(&next) == other_reference.iter().next();
            }
        }
        assert_eq!(is_adjacent, interval.is_adjacent(&other_interval));
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_prefix(key in small_key(), suffix in any::<u8>(), other in small_key()) {
            let mut with_suffix = Vec::from(key.clone());
            with_suffix.push(suffix);

            let interval = Interval::prefix(key.clone());
            assert!(interval.contains(&with_suffix));
            assert!(interval.contains(&key));

            assert_eq!(interval.contains(&other), other.starts_with(&key));
        }

        #[test]
        fn test_u8_interval(
            (reference, interval) in u8_interval(),
            (other_reference, other_interval) in u8_interval(),
            query in any::<Option<u8>>(),
        ) {
            let query = query.map(|q| vec![q]).unwrap_or_else(Vec::new).into();
            test_bounded_intervals(reference, interval, other_reference, other_interval, query);
        }

        #[test]
        #[ignore = "Too expensive to run in dev builds"]
        fn test_u16_interval(
            (reference, interval) in u16_interval(),
            (other_reference, other_interval) in u16_interval(),
            query in prop::collection::vec(any::<u8>(), 0..2),
        ) {
            test_bounded_intervals(
                reference,
                interval,
                other_reference,
                other_interval,
                query.into(),
            );
        }
    }

    fn assert_interval_eq(
        set: &BTreeSet<BinaryKey>,
        interval: Interval,
        expected: Vec<&BinaryKey>,
    ) {
        let StartIncluded(ref s) = interval.start;
        let r = match interval.end {
            End::Excluded(ref t) => set.range(s..t),
            End::Unbounded => set.range(s..),
        };
        assert_eq!(r.collect::<Vec<_>>(), expected);
    }

    #[test]
    fn test_range_strings() {
        let t1 = key(b"banana\x00drank");
        let t2 = key(b"banana\x00pie");
        let t3 = key(b"bandemic\x00");

        let s = [t1.clone(), t2.clone(), t3.clone()].into_iter().collect();
        assert_interval_eq(&s, Interval::prefix(key(b"ban\x00")), vec![]);
        assert_interval_eq(&s, Interval::prefix(key(b"banana\x00")), vec![&t1, &t2]);
        assert_interval_eq(&s, Interval::prefix(key(b"bananap\x00")), vec![]);
        assert_interval_eq(&s, Interval::prefix(key(b"bandemic\x00")), vec![&t3]);
    }

    #[test]
    fn test_key_or_bound_range() {
        let mut s = BTreeSet::new();
        s.insert(key(b"\x01\x00"));
        s.insert(key(b"\x02\x00"));
        s.insert(key(b"\x02\x00\x01\x00"));
        s.insert(key(b"\x02\x00\x02\x00"));
        s.insert(key(b"\x03\x00"));

        assert_interval_eq(
            &s,
            Interval::prefix(b"\x02".to_vec().into()),
            vec![
                &key(b"\x02\x00"),
                &key(b"\x02\x00\x01\x00"),
                &key(b"\x02\x00\x02\x00"),
            ],
        );
        assert_interval_eq(
            &s,
            Interval {
                start: start(b"\x02\x00"),
                end: end(b"\x02\x00\x02\x00"),
            },
            vec![&key(b"\x02\x00"), &key(b"\x02\x00\x01\x00")],
        );
    }
}
