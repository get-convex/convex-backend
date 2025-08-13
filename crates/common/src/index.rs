use std::{
    borrow::Borrow,
    cmp::Ordering,
};

use derive_more::Deref;
use value::{
    id_v6::DeveloperDocumentId,
    ConvexValue,
    InternalId,
    Size,
};

use crate::{
    metrics::log_index_expiration_checked,
    types::Timestamp,
    value::values_to_bytes,
};

// Splits a key into a prefix and suffix, where the prefix is the maximum
// allowed prefix we can store in the primary key Postgres.
pub struct SplitKey {
    pub prefix: Vec<u8>,
    pub suffix: Option<Vec<u8>>,
}

pub const MAX_INDEX_KEY_PREFIX_LEN: usize = 2500;

impl SplitKey {
    pub fn new(key: Vec<u8>) -> Self {
        if key.len() > MAX_INDEX_KEY_PREFIX_LEN {
            Self {
                prefix: key[..MAX_INDEX_KEY_PREFIX_LEN].to_vec(),
                suffix: Some(key[MAX_INDEX_KEY_PREFIX_LEN..].to_vec()),
            }
        } else {
            Self {
                prefix: key,
                suffix: None,
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct IndexEntry {
    // Ordering these fields is important for derived Ord.
    // The first four fields are the primary key in `indexes` table.
    pub index_id: InternalId,
    pub key_prefix: Vec<u8>,
    pub key_sha256: Vec<u8>,
    pub ts: Timestamp,

    pub key_suffix: Option<Vec<u8>>,
    pub deleted: bool,
}

impl IndexEntry {
    /// Is the row outside of retention policy.
    /// next_row must be the next index row in (index_id, index_key, ts)
    /// lexicographic order.
    pub fn is_expired(
        &self,
        min_snapshot_ts: Timestamp,
        next_row: Option<&IndexEntry>,
    ) -> anyhow::Result<bool> {
        if let Some(next_row) = next_row {
            // Check lexicographic order.
            anyhow::ensure!(
                self < next_row,
                "index entries passed out of order - {self:?} before {next_row:?}"
            )
        }
        let result = if self.ts < min_snapshot_ts {
            if self.deleted {
                // Tombstones before min_snapshot_ts are all expired.
                log_index_expiration_checked(true, "tombstone");
                true
            } else {
                match next_row {
                    None => {
                        // Latest for index key because there is no next index row.
                        false
                    },
                    Some(next_row) => {
                        if self.index_id == next_row.index_id
                            && self.key_sha256 == next_row.key_sha256
                        {
                            if next_row.ts <= min_snapshot_ts {
                                // If next_row is before or at min_snapshot_ts, then any snapshot
                                // >= min_snapshot_ts will see next_row or a later revision.
                                // No accessible snapshot can see `self`.
                                log_index_expiration_checked(true, "overwritten_before_retention");
                                true
                            } else {
                                log_index_expiration_checked(false, "overwritten_within_retention");
                                false
                            }
                        } else {
                            // Latest for index key because next has different index key.
                            log_index_expiration_checked(false, "latest_before_retention");
                            false
                        }
                    },
                }
            }
        } else {
            // The row is visible at self.ts, so it's not expired.
            log_index_expiration_checked(false, "within_retention");
            false
        };
        Ok(result)
    }
}

/// An encoded IndexKey, with the same ordering.
/// We don't parse these because we don't need to, it's inefficient, and that
/// would require knowing the encoding format which may depend on DbDriverTag.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Deref)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct IndexKeyBytes(pub Vec<u8>);

impl Borrow<[u8]> for IndexKeyBytes {
    fn borrow(&self) -> &[u8] {
        self
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
/// An IndexKey is what's stored in an index. For an index on `(a, b)`, this
/// will hold `(doc.a, doc.b, doc._id)`.
pub struct IndexKey {
    values_with_id: Vec<Option<ConvexValue>>,
    id: DeveloperDocumentId,
}

impl IndexKey {
    /// Construct an `IndexKey`.
    pub fn new_allow_missing(
        mut index_values: Vec<Option<ConvexValue>>,
        id: DeveloperDocumentId,
    ) -> Self {
        let id_value: ConvexValue = id.into();
        index_values.push(Some(id_value));
        Self {
            values_with_id: index_values,
            id,
        }
    }

    pub fn new(index_values: Vec<ConvexValue>, id: DeveloperDocumentId) -> Self {
        Self::new_allow_missing(index_values.into_iter().map(Some).collect(), id)
    }

    /// For an index key `(doc.a, doc.b, doc._id)`, returns `(doc.a, doc.b)`.
    pub fn indexed_values(&self) -> &[Option<ConvexValue>] {
        &self.values_with_id[..self.values_with_id.len() - 1]
    }

    pub fn to_bytes(&self) -> IndexKeyBytes {
        IndexKeyBytes(values_to_bytes(&self.values_with_id))
    }

    pub fn size(&self) -> usize {
        let mut size = self.id.size();
        for value in self.values_with_id.iter().flatten() {
            size += value.size();
        }
        size
    }
}

impl From<IndexKey> for (Vec<Option<ConvexValue>>, DeveloperDocumentId) {
    fn from(k: IndexKey) -> Self {
        let mut values = k.values_with_id;
        values.pop();
        (values, k.id)
    }
}

impl Ord for IndexKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.values_with_id.cmp(&other.values_with_id)
    }
}
impl PartialOrd for IndexKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(any(test, feature = "testing"))]
mod proptest {
    use proptest::prelude::*;
    use value::{
        id_v6::DeveloperDocumentId,
        ConvexValue,
    };

    use super::IndexKey;

    impl Arbitrary for IndexKey {
        type Parameters = ();

        type Strategy = impl Strategy<Value = IndexKey>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            any::<(Vec<Option<ConvexValue>>, DeveloperDocumentId)>()
                .prop_map(|(values, id)| IndexKey::new_allow_missing(values, id))
        }
    }
}

#[cfg(any(test, feature = "testing"))]
pub mod test_helpers {
    use crate::types::{
        IndexDescriptor,
        IndexName,
    };

    pub fn new_index_name(table_name: &str, index_name: &str) -> anyhow::Result<IndexName> {
        IndexName::new(
            str::parse(table_name)?,
            IndexDescriptor::new(index_name.to_string())?,
        )
    }

    pub fn new_index_descriptor(
        table_name: &str,
        index_name: &str,
    ) -> anyhow::Result<IndexDescriptor> {
        new_index_name(table_name, index_name).map(|name| name.descriptor().clone())
    }
}
