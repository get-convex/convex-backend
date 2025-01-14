use std::mem::size_of;

use common::{
    document::ResolvedDocument,
    index::IndexEntry,
    knobs::{
        MYSQL_CHUNK_SIZE,
        MYSQL_MAX_CHUNK_BYTES,
        MYSQL_MAX_DYNAMIC_SMART_CHUNK_SIZE,
    },
    persistence::DatabaseDocumentUpdate,
    types::{
        DatabaseIndexUpdate,
        Timestamp,
    },
    value::{
        InternalDocumentId,
        ResolvedDocumentId,
    },
};

pub trait ApproxSize {
    fn approx_size(&self) -> usize;
}

impl<T: ApproxSize> ApproxSize for Option<T> {
    fn approx_size(&self) -> usize {
        self.as_ref().map_or("null".len(), |v| v.approx_size())
    }
}

impl ApproxSize for Timestamp {
    fn approx_size(&self) -> usize {
        size_of::<Timestamp>()
    }
}

impl ApproxSize for InternalDocumentId {
    fn approx_size(&self) -> usize {
        self.size()
    }
}

impl ApproxSize for ResolvedDocument {
    fn approx_size(&self) -> usize {
        self.size()
    }
}

impl<T: ApproxSize, U: ApproxSize> ApproxSize for (T, U) {
    fn approx_size(&self) -> usize {
        self.0.approx_size() + self.1.approx_size()
    }
}

impl<T: ApproxSize, U: ApproxSize, V: ApproxSize> ApproxSize for (T, U, V) {
    fn approx_size(&self) -> usize {
        self.0.approx_size() + self.1.approx_size() + self.2.approx_size()
    }
}

impl ApproxSize for DatabaseDocumentUpdate {
    fn approx_size(&self) -> usize {
        self.ts.approx_size()
            + self.id.approx_size()
            + self.value.approx_size()
            + self.prev_ts.approx_size()
    }
}

impl ApproxSize for DatabaseIndexUpdate {
    fn approx_size(&self) -> usize {
        self.index_id.size() + self.key.clone().into_bytes().len() + ResolvedDocumentId::MIN.size()
    }
}

impl ApproxSize for &[u8] {
    fn approx_size(&self) -> usize {
        self.len()
    }
}

impl ApproxSize for bool {
    fn approx_size(&self) -> usize {
        "false".len()
    }
}

impl ApproxSize for IndexEntry {
    fn approx_size(&self) -> usize {
        self.index_id.size()
            + (&*self.key_prefix).approx_size()
            + (&*self.key_sha256).approx_size()
            + self.ts.approx_size()
            + self.key_suffix.as_deref().approx_size()
            + self.deleted.approx_size()
    }
}

struct SmartChunkIter<'a, T: ApproxSize> {
    items: &'a [T],
    max_dynamic_size: usize,
    max_chunk_size: usize,
    max_bytes: usize,
}

impl<'a, T: ApproxSize> Iterator for SmartChunkIter<'a, T> {
    type Item = &'a [T];

    fn next(&mut self) -> Option<Self::Item> {
        if self.items.is_empty() {
            return None;
        }
        let mut len = 0;
        let mut total_bytes = 0;
        for item in self.items {
            total_bytes += item.approx_size();
            if len > self.max_chunk_size || (len > 0 && total_bytes > self.max_bytes) {
                break;
            }
            len += 1;
        }
        let chunk_size = if len <= self.max_dynamic_size {
            len
        } else if len >= self.max_chunk_size {
            self.max_chunk_size
        } else {
            // Round down to power of 2.
            1 << (len.ilog2() as usize)
        };
        let next_chunk = &self.items[0..chunk_size];
        self.items = &self.items[chunk_size..];
        Some(next_chunk)
    }
}

pub fn smart_chunks<T: ApproxSize>(items: &[T]) -> impl Iterator<Item = &[T]> {
    SmartChunkIter {
        items,
        max_dynamic_size: *MYSQL_MAX_DYNAMIC_SMART_CHUNK_SIZE,
        max_chunk_size: *MYSQL_CHUNK_SIZE,
        max_bytes: *MYSQL_MAX_CHUNK_BYTES,
    }
}

/// Possible lengths of chunks returned by smart_chunks.
pub fn smart_chunk_sizes() -> impl Iterator<Item = usize> {
    (1..=*MYSQL_CHUNK_SIZE)
        .filter(|len| *len <= *MYSQL_MAX_DYNAMIC_SMART_CHUNK_SIZE || len.is_power_of_two())
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::ApproxSize;
    use crate::chunks::SmartChunkIter;

    impl ApproxSize for i32 {
        fn approx_size(&self) -> usize {
            4
        }
    }

    #[test]
    fn test_small_batch() {
        // Small batch => single chunk.
        let items = (1..).take(6).collect_vec();
        let iter = SmartChunkIter {
            items: &items,
            max_dynamic_size: 8,
            max_chunk_size: 16,
            max_bytes: 1 << 20,
        };
        assert_eq!(iter.collect_vec(), vec![(1..=6).collect_vec()]);
    }

    #[test]
    fn test_huge_batch() {
        // Huge batch => big chunks.
        let items = (1..).take(100).collect_vec();
        let iter = SmartChunkIter {
            items: &items,
            max_dynamic_size: 4,
            max_chunk_size: 50,
            max_bytes: 1 << 20,
        };
        assert_eq!(
            iter.collect_vec(),
            vec![(1..=50).collect_vec(), (51..=100).collect_vec()]
        );
    }

    #[test]
    fn test_medium_batch_power_of_two() {
        // Medium batch => powers of two chunks.
        let items = (1..).take(42).collect_vec();
        let iter = SmartChunkIter {
            items: &items,
            max_dynamic_size: 4,
            max_chunk_size: 50,
            max_bytes: 1 << 20,
        };
        assert_eq!(
            iter.collect_vec(),
            vec![
                (1..=32).collect_vec(),
                (33..=40).collect_vec(),
                (41..=42).collect_vec()
            ]
        );
    }

    #[test]
    fn test_too_many_bytes() {
        // Too many bytes for max chunk.
        let items = (1..).take(42).collect_vec();
        let iter = SmartChunkIter {
            items: &items,
            max_dynamic_size: 4,
            max_chunk_size: 50,
            max_bytes: 70,
        };
        assert_eq!(
            iter.collect_vec(),
            vec![
                (1..=16).collect_vec(),
                (17..=32).collect_vec(),
                (33..=40).collect_vec(),
                (41..=42).collect_vec()
            ]
        );
    }
}
