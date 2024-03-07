use std::{
    collections::BTreeSet,
    convert::TryFrom,
    fmt,
    iter::IntoIterator,
    ops::Deref,
};

use errors::ErrorMetadata;

use super::{
    size::{
        check_nesting,
        Size,
    },
    ConvexValue,
};
use crate::{
    heap_size::{
        HeapSize,
        WithHeapSize,
    },
    size::check_system_size,
    utils::display_sequence,
};

const MAX_SET_LEN: usize = 1024;

/// Wrapper on `BTreeSet<Value>` that enforces size limits.
#[derive(Clone)]
pub struct ConvexSet {
    // Precomputed `1 + (size(v1) + ... + size(vN) + 1`
    size: usize,
    // Precomputed `1 + max(nesting(v1), ..., nesting(vN))`.
    nesting: usize,

    items: WithHeapSize<BTreeSet<ConvexValue>>,
}

impl HeapSize for ConvexSet {
    fn heap_size(&self) -> usize {
        self.items.heap_size()
    }
}

impl IntoIterator for ConvexSet {
    type IntoIter = std::collections::btree_set::IntoIter<ConvexValue>;
    type Item = ConvexValue;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a> IntoIterator for &'a ConvexSet {
    type IntoIter = std::collections::btree_set::Iter<'a, ConvexValue>;
    type Item = &'a ConvexValue;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl TryFrom<BTreeSet<ConvexValue>> for ConvexSet {
    type Error = anyhow::Error;

    fn try_from(items: BTreeSet<ConvexValue>) -> anyhow::Result<Self> {
        if items.len() > MAX_SET_LEN {
            anyhow::bail!(ErrorMetadata::bad_request(
                "SetTooLongError",
                format!(
                    "Set length is too long ({} > maximum length {MAX_SET_LEN})",
                    items.len()
                )
            ));
        }
        let size = 1 + items.iter().map(|v| v.size()).sum::<usize>() + 1;
        check_system_size(size)?;
        let nesting = 1 + items.iter().map(|v| v.nesting()).max().unwrap_or(0);
        check_nesting(nesting)?;
        Ok(ConvexSet {
            size,
            nesting,
            items: items.into(),
        })
    }
}

impl From<ConvexSet> for BTreeSet<ConvexValue> {
    fn from(set: ConvexSet) -> Self {
        set.items.into()
    }
}

impl Deref for ConvexSet {
    type Target = BTreeSet<ConvexValue>;

    fn deref(&self) -> &BTreeSet<ConvexValue> {
        &self.items
    }
}

impl fmt::Debug for ConvexSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.items, f)
    }
}

impl fmt::Display for ConvexSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        display_sequence(f, ["{", "}"], self.items.iter())
    }
}

impl Size for ConvexSet {
    fn size(&self) -> usize {
        self.size
    }

    fn nesting(&self) -> usize {
        self.nesting
    }
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for ConvexSet {
    type Parameters = proptest::collection::SizeRange;

    type Strategy = impl proptest::strategy::Strategy<Value = ConvexSet>;

    fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        prop::collection::btree_set(any::<ConvexValue>(), args)
            .prop_filter_map("BTreeSet wasn't a valid Convex value", |s| {
                ConvexSet::try_from(s).ok()
            })
    }
}
