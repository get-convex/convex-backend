use std::{
    mem,
    ops::Deref,
    sync::{
        Arc,
        LazyLock,
    },
};

use ref_cast::RefCast;
use tantivy::{
    schema::Type,
    Term,
};

use crate::{
    levenshtein_dfa::build_fuzzy_dfa,
    memory_index::{
        art::ART,
        slab::{
            Slab,
            SlabKey,
        },
        small_slice::SmallSlice,
    },
    scoring::term_from_str,
    EditDistance,
};

/// Used to skip the Term metadata bits Tantivy does not publicly expose
/// in Terms of type String.
static TERM_STRING_METADATA_BITS: LazyLock<Vec<u8>> =
    LazyLock::new(|| term_from_str("").as_slice().to_vec());

pub type TermId = SlabKey;

#[derive(Debug, Clone, RefCast)]
#[repr(transparent)]
struct TermRef(Term);

impl AsRef<[u8]> for TermRef {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

#[derive(Clone, Debug)]
pub struct TermEntry {
    term: SmallSlice,
    refcount: u32,
}

/// Stores filter and search terms. Cheap to Clone via copy-on-write data
/// structures.
#[derive(Clone, Debug)]
pub struct TermTable {
    terms: Slab<TermEntry>,
    index: ART<TermRef, SlabKey>,
    size: usize,
}

impl TermTable {
    pub(crate) fn new() -> Self {
        Self {
            terms: Slab::new(),
            index: ART::new(),
            size: 0,
        }
    }

    pub fn incref(&mut self, term: &Term) -> TermId {
        if let Some(term_id) = self.index.get_mut(TermRef::ref_cast(term)) {
            let entry = self
                .terms
                .get_mut(*term_id)
                .expect("Invalid search term ID");
            entry.refcount += 1;
            return *term_id;
        }
        let term_ref = TermRef(term.clone());
        let term_slice = SmallSlice::from(term_ref.as_ref());
        let entry = TermEntry {
            term: term_slice,
            refcount: 1,
        };

        self.size += entry.term.heap_allocations();
        self.size += mem::size_of::<TermEntry>();
        self.size += mem::size_of::<(SmallSlice, SlabKey)>();

        let term_id = self.terms.alloc(entry);
        self.index.insert(term_ref, term_id);
        term_id
    }

    pub fn decref(&mut self, term_id: TermId, count: u32) {
        let entry = self.terms.get_mut(term_id).expect("Invalid search term ID");
        assert!(entry.refcount >= count);
        entry.refcount -= count;
        if entry.refcount == 0 {
            let entry = self.terms.free(term_id);
            let term_bytes = entry.term.deref();
            let term = Term::wrap(Vec::from(term_bytes));
            assert_eq!(self.index.remove(&TermRef(term)), Some(term_id));

            self.size -= entry.term.heap_allocations();
            self.size -= mem::size_of::<TermEntry>();
            self.size -= mem::size_of::<(Arc<[u8]>, SlabKey)>();
        }
    }

    pub fn get(&self, term: &Term) -> Option<TermId> {
        self.index.get(TermRef::ref_cast(term)).cloned()
    }

    pub fn get_fuzzy(
        &self,
        term: &Term,
        max_distance: u8,
        prefix: bool,
    ) -> impl Iterator<Item = (TermId, EditDistance, Term)> + '_ {
        assert!(max_distance <= 2);
        let term = term.as_str().expect("Term must be string for get_fuzzy");
        let dfa = build_fuzzy_dfa(term, max_distance, prefix);

        self.index
            .intersect(dfa, Some(&TERM_STRING_METADATA_BITS))
            .map(|(key, dist, bytes)| {
                let term = Term::wrap(bytes);
                debug_assert_eq!(term.typ(), Type::Str);
                (*key, dist, term)
            })
    }

    pub fn refcount(&self, term_id: TermId) -> u32 {
        self.terms.get(term_id).expect("Invalid term ID").refcount
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn consistency_check(&self) -> anyhow::Result<()> {
        anyhow::ensure!(self.terms.len() == self.index.len());
        let mut expected_size = 0;
        for term_id in self.index.iter_values() {
            let Some(entry) = self.terms.get(*term_id) else {
                anyhow::bail!("Missing term for {term_id}");
            };
            anyhow::ensure!(entry.refcount > 0);
            expected_size += entry.term.heap_allocations();
            expected_size += mem::size_of::<TermEntry>();
            expected_size += mem::size_of::<(Arc<[u8]>, SlabKey)>();
        }
        anyhow::ensure!(self.size == expected_size);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::{
        memory_index::term_table::TermTable,
        scoring::term_from_str,
    };

    #[test]
    fn test_get_fuzzy() {
        let mut tt = TermTable::new();
        tt.incref(&term_from_str("brow"));

        let results = tt.get_fuzzy(&term_from_str("brow"), 0, false).collect_vec();
        assert_eq!(results.len(), 1);

        let results = tt
            .get_fuzzy(&term_from_str("brown"), 0, false)
            .collect_vec();
        assert_eq!(results.len(), 0);

        let results = tt
            .get_fuzzy(&term_from_str("brown"), 1, false)
            .collect_vec();
        assert_eq!(results.len(), 1);

        tt.incref(&term_from_str("aaaaaaaaaaa"));
        let results = tt.get_fuzzy(&term_from_str("aaa"), 0, true).collect_vec();
        assert_eq!(results.len(), 1);
        let results = tt.get_fuzzy(&term_from_str("baa"), 0, true).collect_vec();
        assert_eq!(results.len(), 0);
        let results = tt.get_fuzzy(&term_from_str("baa"), 1, true).collect_vec();
        assert_eq!(results.len(), 1);
        let results = tt.get_fuzzy(&term_from_str("bab"), 1, true).collect_vec();
        assert_eq!(results.len(), 0);

        // This actually also matches brow since bro <-> bab are distance 2
        let results = tt.get_fuzzy(&term_from_str("bab"), 2, true).collect_vec();
        assert_eq!(results.len(), 2);
    }
}
