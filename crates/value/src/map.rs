use std::{
    cmp,
    collections::BTreeMap,
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
    utils::display_map,
};

const MAX_MAP_LEN: usize = 1024;

/// Wrapper on `BTreeMap<Value, Value>` that enforces size limits.
#[derive(Clone)]
pub struct ConvexMap {
    // Precomputed `1 + size(k1) + size(v1) + ... + size(kN) + size(vN) + 1`
    size: usize,
    // Precomputed `1 + max(nesting(k1), nesting(v1), ..., nesting(kN), nesting(vN))`.
    nesting: usize,

    items: WithHeapSize<BTreeMap<ConvexValue, ConvexValue>>,
}

impl HeapSize for ConvexMap {
    fn heap_size(&self) -> usize {
        self.items.heap_size()
    }
}

impl IntoIterator for ConvexMap {
    type IntoIter = std::collections::btree_map::IntoIter<ConvexValue, ConvexValue>;
    type Item = (ConvexValue, ConvexValue);

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a> IntoIterator for &'a ConvexMap {
    type IntoIter = std::collections::btree_map::Iter<'a, ConvexValue, ConvexValue>;
    type Item = (&'a ConvexValue, &'a ConvexValue);

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl TryFrom<BTreeMap<ConvexValue, ConvexValue>> for ConvexMap {
    type Error = anyhow::Error;

    fn try_from(items: BTreeMap<ConvexValue, ConvexValue>) -> anyhow::Result<Self> {
        if items.len() > MAX_MAP_LEN {
            anyhow::bail!(ErrorMetadata::bad_request(
                "MapTooLongError",
                format!(
                    "Map length is too long ({} > maximum length {MAX_MAP_LEN})",
                    items.len()
                )
            ));
        }
        let size = 1
            + items
                .iter()
                .map(|(k, v)| k.size() + v.size())
                .sum::<usize>()
            + 1;
        check_system_size(size)?;
        let nesting = 1 + items
            .iter()
            .map(|(k, v)| cmp::max(k.nesting(), v.nesting()))
            .max()
            .unwrap_or(0);
        check_nesting(nesting)?;
        Ok(Self {
            size,
            nesting,
            items: items.into(),
        })
    }
}

impl From<ConvexMap> for BTreeMap<ConvexValue, ConvexValue> {
    fn from(map: ConvexMap) -> Self {
        map.items.into()
    }
}

impl Deref for ConvexMap {
    type Target = BTreeMap<ConvexValue, ConvexValue>;

    fn deref(&self) -> &BTreeMap<ConvexValue, ConvexValue> {
        &self.items
    }
}

impl fmt::Debug for ConvexMap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.items, f)
    }
}

impl fmt::Display for ConvexMap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        display_map(f, ["{", "}"], self.items.iter())
    }
}

impl Size for ConvexMap {
    fn size(&self) -> usize {
        self.size
    }

    fn nesting(&self) -> usize {
        self.nesting
    }
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for ConvexMap {
    type Parameters = proptest::collection::SizeRange;

    type Strategy = impl proptest::strategy::Strategy<Value = ConvexMap>;

    fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        prop::collection::btree_map(any::<ConvexValue>(), any::<ConvexValue>(), args)
            .prop_filter_map("Map wasn't a valid Convex value", |s| {
                ConvexMap::try_from(s).ok()
            })
    }
}
