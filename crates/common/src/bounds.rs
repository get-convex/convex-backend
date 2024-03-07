//! Lower and upper bound wrappers on [`std::ops::Bound`].
//!
//! Bounds can't directly implement `Ord` because `Unbounded` will either be the
//! minimum value or the maximum value depending on whether it's used as a lower
//! bound or an upper bound. Instead, we define dedicated wrappers for lower
//! bounds and upper bounds that can be ordered.
use std::{
    cmp::Ordering,
    ops::Bound,
};

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct LowerBound<K>(pub Bound<K>);

impl<K: Ord> Ord for LowerBound<K> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.0, &other.0) {
            (Bound::Included(l), Bound::Included(r)) => l.cmp(r),
            (Bound::Excluded(l), Bound::Excluded(r)) => l.cmp(r),
            (Bound::Included(l), Bound::Excluded(r)) => {
                if l == r {
                    Ordering::Less
                } else {
                    l.cmp(r)
                }
            },
            (Bound::Excluded(l), Bound::Included(r)) => {
                if l == r {
                    Ordering::Greater
                } else {
                    l.cmp(r)
                }
            },
            (Bound::Unbounded, Bound::Unbounded) => Ordering::Equal,
            (_, Bound::Unbounded) => Ordering::Greater,
            (Bound::Unbounded, _) => Ordering::Less,
        }
    }
}

impl<K: Clone + Ord> LowerBound<&K> {
    pub fn cloned(&self) -> LowerBound<K> {
        LowerBound(self.0.map(|b| b.clone()))
    }
}

impl<K: Ord> LowerBound<K> {
    pub fn as_ref(&self) -> LowerBound<&K> {
        LowerBound(self.0.as_ref())
    }
}

impl<K: Ord> PartialOrd for LowerBound<K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct UpperBound<K>(pub Bound<K>);

impl<K: Ord> Ord for UpperBound<K> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.0, &other.0) {
            (Bound::Included(l), Bound::Included(r)) => l.cmp(r),
            (Bound::Excluded(l), Bound::Excluded(r)) => l.cmp(r),
            (Bound::Included(l), Bound::Excluded(r)) => {
                if l == r {
                    Ordering::Less
                } else {
                    l.cmp(r)
                }
            },
            (Bound::Excluded(l), Bound::Included(r)) => {
                if l == r {
                    Ordering::Greater
                } else {
                    l.cmp(r)
                }
            },
            (Bound::Unbounded, Bound::Unbounded) => Ordering::Equal,
            (_, Bound::Unbounded) => Ordering::Less,
            (Bound::Unbounded, _) => Ordering::Greater,
        }
    }
}

impl<K: Clone + Ord> UpperBound<&K> {
    pub fn cloned(&self) -> UpperBound<K> {
        UpperBound(self.0.map(|b| b.clone()))
    }
}

impl<K: Ord> UpperBound<K> {
    pub fn as_ref(&self) -> UpperBound<&K> {
        UpperBound(self.0.as_ref())
    }
}

impl<K: Ord> PartialOrd for UpperBound<K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
