use std::collections::{
    BTreeMap,
    HashSet,
};

use compact_str::CompactString;
use value::{
    heap_size::{
        HeapSize,
        WithHeapSize,
    },
    FieldPath,
};

use crate::{
    index::IndexKeyBytes,
    query::FilterValue as SearchFilterValue,
    types::TabletIndexName,
};

/// For a given document, contains all the index keys for the indexes on the
/// document’s table.
///
/// This is used in lieu of the full document in the write log. This is most of
/// the time more memory efficient (because we don’t need to store the full
/// document) and faster (because we don’t need to reconstruct the index keys
/// every time we need them).
#[derive(Clone, Debug)]
pub struct DocumentIndexKeys(WithHeapSize<BTreeMap<TabletIndexName, DocumentIndexKeyValue>>);

impl From<BTreeMap<TabletIndexName, DocumentIndexKeyValue>> for DocumentIndexKeys {
    fn from(map: BTreeMap<TabletIndexName, DocumentIndexKeyValue>) -> Self {
        Self(map.into())
    }
}

impl DocumentIndexKeys {
    pub fn get(&self, index_name: &TabletIndexName) -> Option<&DocumentIndexKeyValue> {
        self.0.get(index_name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&TabletIndexName, &DocumentIndexKeyValue)> {
        self.0.iter()
    }

}

impl HeapSize for DocumentIndexKeys {
    fn heap_size(&self) -> usize {
        self.0.heap_size()
    }
}

#[derive(Clone, Debug)]
pub enum DocumentIndexKeyValue {
    Standard(IndexKeyBytes),
    Search(SearchIndexKeyValue),
    // We don’t store index key values for vector indexes because they don’t
    // support subscriptions.
}

#[derive(Clone, Debug)]
pub struct SearchIndexKeyValue {
    /// These are values for the fields present in the must
    /// clauses of the search index.
    pub filter_values: WithHeapSize<BTreeMap<FieldPath, SearchFilterValue>>,
    pub search_field: FieldPath,
    pub search_field_value: Option<SearchValueTokens>,
}

impl HeapSize for DocumentIndexKeyValue {
    fn heap_size(&self) -> usize {
        match self {
            DocumentIndexKeyValue::Standard(index_key) => index_key.heap_size(),
            DocumentIndexKeyValue::Search(SearchIndexKeyValue {
                filter_values,
                search_field,
                search_field_value,
            }) => {
                filter_values.heap_size()
                    + search_field.heap_size()
                    + search_field_value.heap_size()
            },
        }
    }
}

/// The tokens in some textual value (search field of a full-text search index).
///
/// Tokens are not sorted in any particular order, but must be unique.
/// (Uniqueness is not strictly necessary here, but we’d like to avoid
/// iterating over the same token multiple times.)
#[derive(Clone, Debug)]
pub struct SearchValueTokens(WithHeapSize<Box<[CompactString]>>);

impl From<HashSet<CompactString>> for SearchValueTokens {
    fn from(value: HashSet<CompactString>) -> Self {
        let tokens: Box<[CompactString]> = value.into_iter().collect();
        Self(tokens.into())
    }
}

impl SearchValueTokens {
    pub fn for_each_token<F>(&self, prefix: bool, mut for_each: F)
    where
        F: FnMut(&str),
    {
        if prefix {
            // We're inverting prefix match here by constructing all possible prefixes for
            // each term in the document if at least one prefix search exists in
            // the readset (resulting in this method being called with prefix:
            // true).
            //
            // This lets callers search into tries containing the actual search term with
            // dfa prefixes set to false and still match based on prefix.
            // Searching a trie with the document tokens is bounded by the size
            // of the document, which is expected to be significantly smaller
            // than the total number of subscriptions for busy backends.
            for token in self.calculate_prefixes() {
                for_each(token);
            }
        } else {
            for token in self.0.iter() {
                for_each(token);
            }
        }
    }

    fn calculate_prefixes(&self) -> impl Iterator<Item = &str> + '_ {
        let mut set: HashSet<&str> = HashSet::new();

        for token in self.0.iter() {
            if !set.insert(token) {
                continue;
            }
            for (i, _) in token.char_indices()
                // Skip the first index which is always 0
                .skip(1)
            {
                // After that we get all prefixes except for the complete
                // token (because `..i` always skips the last character
                // bytes).
                set.insert(&token[..i]);
            }
        }
        set.into_iter()
    }
}

impl HeapSize for SearchValueTokens {
    fn heap_size(&self) -> usize {
        self.0.heap_size()
    }
}
