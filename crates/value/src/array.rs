use std::{
    convert::TryFrom,
    fmt,
    hash::{
        Hash,
        Hasher,
    },
    iter::IntoIterator,
    ops::Deref,
};

use errors::ErrorMetadata;

use super::size::Size;
use crate::{
    heap_size::{
        HeapSize,
        WithHeapSize,
    },
    size::{
        check_nesting,
        check_system_size,
    },
    utils::display_sequence,
    ConvexValue,
};

const MAX_ARRAY_LEN: usize = 8192;

/// Wrapper on `Vec<Value>` that enforces size limits.
#[derive(Clone)]
pub struct ConvexArray {
    // Precomputed `1 + size(v1) + ... + size(vN) + 1`
    size: usize,
    // Precomputed `1 + max(nesting(v1), ..., nesting(vN))`.
    nesting: usize,

    items: WithHeapSize<Vec<ConvexValue>>,
}

impl ConvexArray {
    pub fn empty() -> Self {
        Self {
            size: 2,
            nesting: 1,
            items: WithHeapSize::default(),
        }
    }
}

impl IntoIterator for ConvexArray {
    type IntoIter = std::vec::IntoIter<ConvexValue>;
    type Item = ConvexValue;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a> IntoIterator for &'a ConvexArray {
    type IntoIter = std::slice::Iter<'a, ConvexValue>;
    type Item = &'a ConvexValue;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl TryFrom<Vec<ConvexValue>> for ConvexArray {
    type Error = anyhow::Error;

    fn try_from(items: Vec<ConvexValue>) -> anyhow::Result<Self> {
        if items.len() > MAX_ARRAY_LEN {
            anyhow::bail!(ErrorMetadata::bad_request(
                "ArrayTooLong",
                format!(
                    "Array length is too long ({} > maximum length {MAX_ARRAY_LEN})",
                    items.len()
                ),
            ));
        }
        let size = 1 + items.iter().map(|v| v.size()).sum::<usize>() + 1;
        check_system_size(size)?;
        let nesting = 1 + items.iter().map(|v| v.nesting()).max().unwrap_or(0);
        check_nesting(nesting)?;
        Ok(Self {
            size,
            nesting,
            items: items.into(),
        })
    }
}

impl From<ConvexArray> for Vec<ConvexValue> {
    fn from(array: ConvexArray) -> Self {
        array.items.into()
    }
}

impl Deref for ConvexArray {
    type Target = [ConvexValue];

    fn deref(&self) -> &[ConvexValue] {
        &self.items[..]
    }
}

impl fmt::Debug for ConvexArray {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.items, f)
    }
}

impl fmt::Display for ConvexArray {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        display_sequence(f, ["[", "]"], self.items.iter())
    }
}

// We only compute the hash on the non-derived state but have an assert in
// `PartialEq` to make sure the derived state always matches.
impl Hash for ConvexArray {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.items.hash(hasher)
    }
}

impl PartialEq for ConvexArray {
    fn eq(&self, other: &Self) -> bool {
        if self.items == other.items {
            // We're just comparing based on items but make sure the derived data always
            // matches.
            assert_eq!(self.size, other.size);
            assert_eq!(self.nesting, other.nesting);
            return true;
        }
        false
    }
}

impl Eq for ConvexArray {}

impl Size for ConvexArray {
    fn size(&self) -> usize {
        self.size
    }

    fn nesting(&self) -> usize {
        self.nesting
    }
}

impl HeapSize for ConvexArray {
    fn heap_size(&self) -> usize {
        self.items.heap_size()
    }
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for ConvexArray {
    type Parameters = proptest::collection::SizeRange;

    type Strategy = impl proptest::strategy::Strategy<Value = ConvexArray>;

    fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        prop::collection::vec(any::<ConvexValue>(), args)
            .prop_filter_map("Vec wasn't a valid Convex value", |s| {
                Self::try_from(s).ok()
            })
    }
}

#[macro_export]
/// Create an array.
///
/// Uses the same syntax as vec![].
macro_rules! array {
    () => (
        $crate::ConvexArray::empty()
    );
    ($($x:expr),+ $(,)?) => (
        $crate::ConvexArray::try_from(
            vec![$($x),+]
        )
    );
}
