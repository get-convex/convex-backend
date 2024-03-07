//! Our own extension traits to add functionality to common types

use std::{
    collections::BTreeSet,
    ops::Bound,
};

/// Small trait for creating a container with a single value in it
pub trait BTreeSetExt {
    /// Element type for the [`BTreeSet`]
    type T;

    /// Create a [`BTreeSet`] from a single value.
    fn one(t: Self::T) -> Self;
}

impl<T: Ord> BTreeSetExt for BTreeSet<T> {
    type T = T;

    fn one(t: T) -> Self {
        let mut out = BTreeSet::new();
        out.insert(t);
        out
    }
}

/// Extension trait for [`Bound`] functionality.
pub trait BoundExt<T> {
    /// Converts a `Bound<K>` to `Bound<fn(K)>`. Generally used when wanting to
    /// convert e.g., `Bound(c, d)` and a tuple `(a, b)` to the corresponding
    /// `Bound(a, b, c, d)`.
    fn map<U>(self, f: impl FnOnce(T) -> U) -> Bound<U>;
}

impl<T> BoundExt<T> for Bound<T> {
    fn map<U>(self, f: impl FnOnce(T) -> U) -> Bound<U> {
        match self {
            Bound::Included(b) => Bound::Included(f(b)),
            Bound::Excluded(b) => Bound::Excluded(f(b)),
            Bound::Unbounded => Bound::Unbounded,
        }
    }
}
