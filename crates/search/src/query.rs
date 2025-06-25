use std::{
    collections::{
        BTreeMap,
        BTreeSet,
        HashSet,
    },
    ops::Deref,
};

use anyhow::Context;
use bitvec::vec::BitVec;
use common::{
    document::{
        CreationTime,
        PackedDocument,
    },
    index::IndexKeyBytes,
    query::FilterValue,
    types::{
        SubscriberId,
        TabletIndexName,
        WriteTimestamp,
    },
};
use indexing::index_registry::{
    DocumentIndexKeyValue,
    DocumentIndexKeys,
    SearchIndexKeyValue,
};
use itertools::{
    Either,
    Itertools,
};
use maplit::btreemap;
#[cfg(any(test, feature = "testing"))]
use proptest::arbitrary::{
    any,
    Arbitrary,
};
#[cfg(any(test, feature = "testing"))]
use proptest::strategy::Strategy;
use tantivy::{
    schema::Field,
    tokenizer::TextAnalyzer,
    Score,
    Term,
};
use value::{
    heap_size::{
        HeapSize,
        WithHeapSize,
    },
    ConvexString,
    ConvexValue,
    FieldPath,
    InternalId,
    ResolvedDocumentId,
};

use crate::{
    convex_en,
    memory_index::{
        art::ART,
        TermId,
    },
    metrics,
    scoring::term_from_str,
    EditDistance,
};

/// A search query compiled against a particular `SearchIndexSchema`.
#[derive(Debug, Clone)]
pub struct CompiledQuery {
    pub text_query: Vec<QueryTerm>,
    pub filter_conditions: Vec<CompiledFilterCondition>,
}

impl CompiledQuery {
    pub fn num_terms(&self) -> usize {
        self.text_query.len() + self.filter_conditions.len()
    }

    pub fn try_from_text_query_proto(
        value: pb::searchlight::TextQuery,
        search_field: Field,
    ) -> anyhow::Result<CompiledQuery> {
        Ok(Self {
            text_query: value
                .search_terms
                .into_iter()
                .map(|t| QueryTerm::try_from_text_query_term_proto(t, search_field))
                .collect::<anyhow::Result<Vec<_>>>()?,
            filter_conditions: value
                .filter_conditions
                .into_iter()
                // TODO(CX-5481): get rid of this `Term::wrap` call. Need to propagate the Field for these.
                .map(|bytes| CompiledFilterCondition::Must(Term::wrap(bytes)))
                .collect_vec(),
        })
    }
}

impl From<CompiledQuery> for pb::searchlight::TextQuery {
    fn from(value: CompiledQuery) -> Self {
        Self {
            search_terms: value
                .text_query
                .into_iter()
                .map(pb::searchlight::TextQueryTerm::from)
                .collect_vec(),
            filter_conditions: value
                .filter_conditions
                .into_iter()
                .map(|CompiledFilterCondition::Must(term)| term.as_slice().to_vec())
                .collect_vec(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueryTerm {
    term: Term,
    /// If the term is the last in a search query, it can be a prefix match for
    /// typeahead suggestions.
    prefix: bool,
}

impl QueryTerm {
    pub fn new(term: Term, prefix: bool) -> Self {
        QueryTerm { term, prefix }
    }

    pub fn term(&self) -> &Term {
        &self.term
    }

    pub fn into_term(self) -> Term {
        self.term
    }

    pub fn max_distance(&self) -> u32 {
        0
    }

    pub fn prefix(&self) -> bool {
        self.prefix
    }

    pub fn try_from_text_query_term_proto(
        value: pb::searchlight::TextQueryTerm,
        search_field: Field,
    ) -> anyhow::Result<QueryTerm> {
        let qterm = match value.term_type {
            None => anyhow::bail!("No TermType in QueryTerm"),
            Some(pb::searchlight::text_query_term::TermType::Exact(exact)) => QueryTerm {
                term: Term::from_field_text(search_field, &exact.token),
                prefix: false,
            },
            Some(pb::searchlight::text_query_term::TermType::Fuzzy(fuzzy)) => QueryTerm {
                term: Term::from_field_text(search_field, &fuzzy.token),
                prefix: fuzzy.prefix,
            },
        };
        Ok(qterm)
    }
}

impl TryFrom<QueryTerm> for TextQueryTerm {
    type Error = anyhow::Error;

    fn try_from(value: QueryTerm) -> Result<Self, Self::Error> {
        let term = value
            .term
            .as_str()
            .context("Term was not a string")?
            .to_string();
        let text_query_term = if value.prefix {
            TextQueryTerm::Fuzzy {
                token: term,
                max_distance: 0.try_into()?,
                prefix: value.prefix,
            }
        } else {
            TextQueryTerm::Exact(term)
        };
        Ok(text_query_term)
    }
}

impl From<QueryTerm> for pb::searchlight::TextQueryTerm {
    fn from(value: QueryTerm) -> Self {
        let term = value.term();
        let term_str = term.as_str().expect("QueryTerm not a string").to_string();

        let term_type = if value.prefix {
            pb::searchlight::text_query_term::TermType::Fuzzy(pb::searchlight::FuzzyTextTerm {
                token: term_str,
                max_distance: 0,
                prefix: value.prefix,
            })
        } else {
            pb::searchlight::text_query_term::TermType::Exact(pb::searchlight::ExactTextTerm {
                token: term_str,
            })
        };
        Self {
            term_type: Some(term_type),
        }
    }
}

/// An expanded version of CompiledQuery's search terms which expands fuzzy
/// queries. Maps each QueryTerm from a CompiledQuery to a vector of term
/// matches and their distance.
///
/// TermShortlist is the list of terms that will be considered for a search
/// query. `shortlist` and `query_term_shortlist_items` are normalized (with the
/// latter storing indices into the other) to deduplicate terms. This allows
/// `CandidateRevision` to include a list of positions for the document
/// represented as a `ShortlistId`.
///
/// Without this deduplicated structure, `CandidateRevision` would need to store
/// the terms themselves which is potentially a lot of unneeded space in the
/// searchlight RPC.
#[derive(Debug, Clone, PartialEq)]
pub struct TermShortlist {
    shortlist: Vec<Term>,
    pub query_term_shortlist_items: BTreeMap<QueryTerm, Vec<(EditDistance, ShortlistId)>>,
}

pub struct TermShortlistBuilder {
    shortlist: Vec<Term>,
    query_term_shortlist_items: BTreeMap<QueryTerm, Vec<(EditDistance, ShortlistId)>>,

    term_to_shortlist: BTreeMap<Term, ShortlistId>,
}

impl TermShortlistBuilder {
    fn new() -> Self {
        Self {
            shortlist: vec![],
            query_term_shortlist_items: Default::default(),
            term_to_shortlist: Default::default(),
        }
    }

    fn build(self) -> TermShortlist {
        TermShortlist {
            shortlist: self.shortlist,
            query_term_shortlist_items: self.query_term_shortlist_items,
        }
    }

    /// Adds the given set of matches for the given term to the short list.
    /// Returns a vec[] containing a 1-1 mapping between each term in
    /// `matches` and the corresponding shortlist id.
    ///
    /// The returned vec will return Some at each position where the
    /// corresponding term was newly added to the shortlist, and None at
    /// each position where the term already existed in the shortlist.
    fn add_matches(
        &mut self,
        term: QueryTerm,
        matches: BTreeSet<(EditDistance, Term)>,
    ) -> Vec<Option<ShortlistId>> {
        let shortlist_items = self.query_term_shortlist_items.entry(term).or_default();
        let mut shortlist_ids = vec![];

        for (distance, term) in matches {
            let maybe_new_shortlist_id = if !self.term_to_shortlist.contains_key(&term) {
                let shortlist_id = ShortlistId(self.shortlist.len() as u16);
                self.term_to_shortlist.insert(term.clone(), shortlist_id);
                self.shortlist.push(term);
                shortlist_items.push((distance, shortlist_id));

                Some(shortlist_id)
            } else {
                None
            };
            shortlist_ids.push(maybe_new_shortlist_id);
        }
        shortlist_ids
    }
}

/// A pointer to a term in the shortlist.
///
/// For now, ShortlistId fits in a u8 since we will never consider more than 128
/// terms but we use u16 to be generous.
///
/// As an implementation detail that may change in the future, these are
/// currently just the index of the term in the shortlist.
#[derive(PartialOrd, Ord, Clone, Debug, Eq, PartialEq, Copy)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct ShortlistId(u16);

impl TryFrom<u32> for ShortlistId {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(ShortlistId(value.try_into()?))
    }
}

pub(crate) fn shortlist_and_id_mapping(
    term_matches: BTreeMap<QueryTerm, Vec<(EditDistance, Term, TermId)>>,
) -> (TermShortlist, BTreeMap<ShortlistId, TermId>) {
    let mut shortlist_id_to_term_id = BTreeMap::new();
    let mut builder = TermShortlistBuilder::new();
    for (query_term, matches) in term_matches {
        let (matches, term_ids): (BTreeSet<_>, Vec<TermId>) = matches
            .into_iter()
            .map(|(distance, match_term, term_id)| ((distance, match_term), term_id))
            .unzip();
        let shortlist_ids = builder.add_matches(query_term, matches);
        shortlist_id_to_term_id.extend(shortlist_ids.into_iter().zip(term_ids).filter_map(
            |(shortlist_id, term_id)| shortlist_id.map(|shortlist_id| (shortlist_id, term_id)),
        ));
    }
    (builder.build(), shortlist_id_to_term_id)
}

impl TermShortlist {
    pub fn new(term_matches: BTreeMap<QueryTerm, BTreeSet<(EditDistance, Term)>>) -> Self {
        let mut builder = TermShortlistBuilder::new();
        for (query_term, matches) in term_matches {
            builder.add_matches(query_term, matches);
        }
        builder.build()
    }

    pub fn terms(&self) -> Vec<Term> {
        self.shortlist.clone()
    }

    pub fn ids_and_terms(&self) -> impl Iterator<Item = (ShortlistId, &Term)> {
        self.shortlist
            .iter()
            .enumerate()
            .map(|(idx, term)| (ShortlistId(idx as u16), term))
    }

    pub fn get_term(&self, id: ShortlistId) -> anyhow::Result<&Term> {
        self.shortlist
            .get(id.0 as usize)
            .context("Invalid shortlist id, did we mix up ids and shortlists?")
    }

    pub fn get_shortlisted_terms_for_query_term(
        &self,
        query_term: &QueryTerm,
    ) -> impl Iterator<Item = &(EditDistance, ShortlistId)> {
        if let Some(vec) = self.query_term_shortlist_items.get(query_term) {
            Either::Left(vec.iter())
        } else {
            Either::Right(vec![].into_iter())
        }
    }

    pub fn try_from_proto(
        value: pb::searchlight::TermShortlist,
        search_field: Field,
    ) -> anyhow::Result<TermShortlist> {
        Ok(TermShortlist {
            shortlist: value
                .shortlist
                .into_iter()
                .map(|term_str| term_from_str(term_str.as_str()))
                .collect_vec(),
            query_term_shortlist_items: value
                .query_term_shortlist_items
                .into_iter()
                .map(|query_term_shortlist| {
                    anyhow::Ok((
                        QueryTerm::try_from_text_query_term_proto(
                            query_term_shortlist
                                .query_term
                                .context("QueryTerm missing from TermShortlist proto")?,
                            search_field,
                        )?,
                        query_term_shortlist
                            .items
                            .into_iter()
                            .map(|item| {
                                Ok((
                                    item.distance as EditDistance,
                                    ShortlistId::try_from(item.shortlist_id)?,
                                ))
                            })
                            .collect::<anyhow::Result<Vec<_>>>()?,
                    ))
                })
                .collect::<anyhow::Result<BTreeMap<_, _>>>()?,
        })
    }
}

impl From<TermShortlist> for pb::searchlight::TermShortlist {
    fn from(value: TermShortlist) -> Self {
        pb::searchlight::TermShortlist {
            shortlist: value
                .shortlist
                .into_iter()
                .map(|term| {
                    term.as_str()
                        .expect("shortlisted term not a string")
                        .to_string()
                })
                .collect_vec(),
            query_term_shortlist_items: value
                .query_term_shortlist_items
                .into_iter()
                .map(
                    |(qterm, matches)| pb::searchlight::QueryTermShortlistItems {
                        query_term: Some(pb::searchlight::TextQueryTerm::from(qterm)),
                        items: matches
                            .into_iter()
                            .map(|(dist, id)| pb::searchlight::ShortlistItem {
                                distance: dist as u32,
                                shortlist_id: id.0 as u32,
                            })
                            .collect_vec(),
                    },
                )
                .collect_vec(),
        }
    }
}

/// A memory-index specific query that is useful for scoring
#[derive(Debug)]
pub struct TermListBitsetQuery {
    /// Stores a sorted list of term IDs in this query
    pub sorted_terms: Vec<TermId>,
    /// Is `sorted_terms[i]` a filter term?
    pub intersection_terms: BitVec,
    /// Is `union_terms[i]` a search term?
    pub union_terms: BitVec,
    /// Score multiplier for a match of this union term
    pub union_id_boosts: Vec<Score>,
}

impl TermListBitsetQuery {
    pub const NEVER_MATCH: TermListBitsetQuery = TermListBitsetQuery {
        sorted_terms: vec![],
        intersection_terms: BitVec::EMPTY,
        union_terms: BitVec::EMPTY,
        union_id_boosts: vec![],
    };

    pub fn new(
        term_ids: BTreeSet<TermId>,
        intersection_term_ids: BTreeSet<TermId>,
        boosts_by_union_id: BTreeMap<TermId, Score>,
    ) -> Self {
        let sorted_terms = term_ids.into_iter().collect_vec();

        let mut intersection_terms = BitVec::repeat(false, sorted_terms.len());
        let mut union_terms = BitVec::repeat(false, sorted_terms.len());
        let mut union_id_boosts = Vec::with_capacity(sorted_terms.len());

        for (i, term) in sorted_terms.iter().enumerate() {
            intersection_terms.set(i, intersection_term_ids.contains(term));
            if let Some(boost) = boosts_by_union_id.get(term) {
                union_terms.set(i, true);
                union_id_boosts.push(*boost);
            }
        }

        Self {
            sorted_terms,
            intersection_terms,
            union_terms,
            union_id_boosts,
        }
    }

    /// Empty `sorted_terms` indicates that a term cannot match any documents,
    /// either from an empty user query or from a query that is instantiated
    /// with TermListBitsetQuery::NEVER_MATCH
    pub fn never_match(&self) -> bool {
        self.sorted_terms.is_empty()
    }
}

#[derive(Debug, Clone)]
pub enum CompiledFilterCondition {
    Must(Term),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct CandidateRevision {
    pub score: f32,
    pub id: InternalId,
    pub ts: WriteTimestamp,
    pub creation_time: CreationTime,
}

impl From<CandidateRevision> for pb::searchlight::CandidateRevision {
    fn from(revision: CandidateRevision) -> Self {
        let ts: Option<u64> = match revision.ts {
            WriteTimestamp::Committed(ts) => Some(ts.into()),
            WriteTimestamp::Pending => None,
        };
        let internal_id_bytes = &*revision.id;
        pb::searchlight::CandidateRevision {
            score: revision.score,
            internal_id: internal_id_bytes.to_vec(),
            ts,
            creation_time: revision.creation_time.into(),
        }
    }
}

impl TryFrom<pb::searchlight::CandidateRevision> for CandidateRevision {
    type Error = anyhow::Error;

    fn try_from(proto: pb::searchlight::CandidateRevision) -> Result<Self, Self::Error> {
        let ts = match proto.ts {
            Some(ts) => WriteTimestamp::Committed(ts.try_into()?),
            None => WriteTimestamp::Pending,
        };
        Ok(CandidateRevision {
            score: proto.score,
            id: proto.internal_id.try_into()?,
            ts,
            creation_time: proto.creation_time.try_into()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct CandidateRevisionPositions {
    pub revision: CandidateRevision,
    pub positions: BTreeMap<ShortlistId, Vec<u32>>,
}

impl From<CandidateRevisionPositions> for CandidateRevision {
    fn from(value: CandidateRevisionPositions) -> Self {
        value.revision
    }
}

impl TryFrom<pb::searchlight::CandidateRevisionPositions> for CandidateRevisionPositions {
    type Error = anyhow::Error;

    fn try_from(value: pb::searchlight::CandidateRevisionPositions) -> Result<Self, Self::Error> {
        Ok(CandidateRevisionPositions {
            revision: CandidateRevision::try_from(
                value.revision.context("candidate revision missing")?,
            )?,
            positions: value
                .positions
                .into_iter()
                .map(|pos| Ok((ShortlistId::try_from(pos.shortlist_id)?, pos.positions)))
                .collect::<anyhow::Result<BTreeMap<_, _>>>()?,
        })
    }
}

impl From<CandidateRevisionPositions> for pb::searchlight::CandidateRevisionPositions {
    fn from(value: CandidateRevisionPositions) -> Self {
        pb::searchlight::CandidateRevisionPositions {
            revision: Some(pb::searchlight::CandidateRevision::from(value.revision)),
            positions: value
                .positions
                .into_iter()
                .map(|(id, positions)| pb::searchlight::ShortlistPositions {
                    shortlist_id: id.0 as u32,
                    positions,
                })
                .collect_vec(),
        }
    }
}

pub type RevisionWithKeys = Vec<(CandidateRevision, IndexKeyBytes)>;

pub struct QueryResults {
    pub revisions_with_keys: RevisionWithKeys,
    pub reads: QueryReads,
}

impl QueryResults {
    pub fn empty() -> Self {
        Self {
            revisions_with_keys: vec![],
            reads: QueryReads::empty(),
        }
    }
}

/// A read based on a single token extracted from a text query search.
///
/// A single text query will be split into many parts (tokenized), each part
/// will be combined with the constant metadata (path, distance prefix etc) into
/// a term, then we track reads based on individual terms.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct TextQueryTermRead {
    pub field_path: FieldPath,
    pub term: TextQueryTerm,
}

impl TextQueryTermRead {
    pub fn new(field_path: FieldPath, term: TextQueryTerm) -> Self {
        Self { field_path, term }
    }
}

// For proptest we're using lowercase ascii and a filter to generate tokens so
// that we approximately match what the tokenzier we're using allows. The
// would already have run on these terms prior to this point for production
// code.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum TextQueryTerm {
    Exact(
        #[cfg_attr(
            any(test, feature = "testing"),
            proptest(
                regex = "[a-z]+",
                filter = "|token| token.len() > 1 && token.len() < 32"
            )
        )]
        String,
    ),
    Fuzzy {
        #[cfg_attr(
            any(test, feature = "testing"),
            proptest(
                regex = "[a-z]+",
                filter = "|token| token.len() > 1 && token.len() < 32"
            )
        )]
        token: String,
        max_distance: FuzzyDistance,
        prefix: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum FuzzyDistance {
    Zero,
    One,
    Two,
}

impl TryFrom<u8> for FuzzyDistance {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            _ => Err(anyhow::anyhow!("Invalid distance: {value}")),
        }
    }
}

impl From<FuzzyDistance> for u8 {
    fn from(value: FuzzyDistance) -> Self {
        *value
    }
}

impl Deref for FuzzyDistance {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        match self {
            FuzzyDistance::Zero => &0u8,
            FuzzyDistance::One => &1u8,
            FuzzyDistance::Two => &2u8,
        }
    }
}

impl TextQueryTerm {
    /// Convert a term into the parameters necessary to perform a "fuzzy"
    /// search.
    ///
    /// Since exact text search is equivalent to a non-prefixed fuzzy search
    /// with a distance 0, we can hard code those values.
    fn fuzzy_params(&self) -> (&String, u8, bool) {
        match self {
            Self::Fuzzy {
                token,
                max_distance,
                prefix,
            } => (token, **max_distance, *prefix),
            Self::Exact(token) => (token, 0u8, false),
        }
    }
}

impl HeapSize for TextQueryTerm {
    fn heap_size(&self) -> usize {
        match self {
            TextQueryTerm::Exact(token) => token.heap_size(),
            TextQueryTerm::Fuzzy {
                token,
                max_distance,
                prefix,
            } => token.heap_size() + max_distance.heap_size() + prefix.heap_size(),
        }
    }
}

impl HeapSize for TextQueryTermRead {
    fn heap_size(&self) -> usize {
        self.field_path.heap_size() + self.term.heap_size()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum FilterConditionRead {
    Must(FieldPath, FilterValue),
}

impl HeapSize for FilterConditionRead {
    fn heap_size(&self) -> usize {
        match self {
            FilterConditionRead::Must(p, v) => p.heap_size() + v.heap_size(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryReads {
    pub text_queries: WithHeapSize<Vec<TextQueryTermRead>>,
    pub filter_conditions: WithHeapSize<Vec<FilterConditionRead>>,

    // State derived from text_queries for more efficient matching with many
    // fuzzy text subscriptions. Because this is strictly derived, it can always
    // be reconstructed from the simpler text_queries / filter_conditions.
    fuzzy_terms: SearchTermTries<()>,
}

impl QueryReads {
    pub fn new(
        text_queries: WithHeapSize<Vec<TextQueryTermRead>>,
        filter_conditions: WithHeapSize<Vec<FilterConditionRead>>,
    ) -> Self {
        let mut fuzzy_terms = SearchTermTries::new();
        fuzzy_terms.extend((), &text_queries);
        Self {
            text_queries,
            filter_conditions,
            fuzzy_terms,
        }
    }
}

#[cfg(any(test, feature = "testing"))]
impl Arbitrary for QueryReads {
    type Parameters = ();

    type Strategy = impl Strategy<Value = QueryReads>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        any::<(
            WithHeapSize<Vec<TextQueryTermRead>>,
            WithHeapSize<Vec<FilterConditionRead>>,
        )>()
        .prop_map(|(text_queries, filter_conditions)| {
            QueryReads::new(text_queries, filter_conditions)
        })
    }
}

impl PartialEq for QueryReads {
    fn eq(&self, other: &Self) -> bool {
        self.text_queries == other.text_queries && self.filter_conditions == other.filter_conditions
    }
}

impl Eq for QueryReads {}

impl HeapSize for QueryReads {
    // TODO(CX-5459): Include fuzzy_terms in heap size.
    fn heap_size(&self) -> usize {
        self.text_queries.heap_size() + self.filter_conditions.heap_size()
    }
}

#[derive(Debug, Clone)]
struct SearchTermTries<T: Clone + Ord> {
    terms: BTreeMap<FieldPath, Tries<T>>,
}

impl<T: Clone + Ord> SearchTermTries<T> {
    fn new() -> Self {
        Self {
            terms: BTreeMap::new(),
        }
    }

    #[fastrace::trace]
    fn overlaps_document<'a>(
        &'a self,
        document: &'a PackedDocument,
        analyzer: &'a TextAnalyzer,
    ) -> bool {
        let mut result = BTreeSet::new();

        for (path, tries) in self.terms.iter() {
            let Some(ConvexValue::String(document_text)) = document.value().get_path(path) else {
                continue;
            };

            let mut tokens = ValueTokens::new(analyzer, &document_text);
            tries.matching_values(&mut tokens, &mut result);
            if !result.is_empty() {
                return true;
            }
        }

        false
    }

    #[fastrace::trace]
    fn overlaps_index_key_value(
        &self,
        index_key_value: &SearchIndexKeyValue,
        analyzer: &TextAnalyzer,
    ) -> bool {
        let Some(value) = &index_key_value.search_field_value else {
            return false;
        };
        let Some(tries) = self.terms.get(&index_key_value.search_field) else {
            return false;
        };
        let mut tokens = ValueTokens::new(analyzer, value);
        let mut result = BTreeSet::new();
        tries.matching_values(&mut tokens, &mut result);
        !result.is_empty()
    }

    fn extend(&mut self, value: T, queries: &WithHeapSize<Vec<TextQueryTermRead>>) {
        for text_query in queries {
            let path = &text_query.field_path;
            let (token, max_distance, prefix) = text_query.term.fuzzy_params();
            let art = self
                .terms
                .entry(path.clone())
                .or_insert_with(Tries::new)
                .tries
                .entry((prefix, max_distance))
                .or_insert_with(ART::new);

            if let Some(value_to_count) = art.get_mut(token) {
                *value_to_count.entry(value.clone()).or_default() += 1
            } else {
                art.insert(token.clone(), btreemap! { value.clone() => 1});
            }
        }
    }

    fn remove(&mut self, value: T, queries: &WithHeapSize<Vec<TextQueryTermRead>>) {
        for text_query in queries {
            let path = &text_query.field_path;
            let (token, max_distance, prefix) = text_query.term.fuzzy_params();
            let value = value.clone();
            let tries = self
                .terms
                .get_mut(path)
                .unwrap_or_else(|| panic!("Missing tries for {}", path));
            let trie = tries
                .tries
                .get_mut(&(prefix, max_distance))
                .unwrap_or_else(|| panic!("Missing trie for ({}, {})", prefix, max_distance));
            let value_to_count = trie
                .get_mut(token)
                .unwrap_or_else(|| panic!("Missing values for a token of length {}", token.len()));
            let count = value_to_count
                .entry(value.clone())
                .and_modify(|count| {
                    *count = count
                        .checked_sub(1)
                        .expect("Can't remove more values than were added")
                })
                .or_insert_with(|| panic!("Missing count for value!"));

            if *count == 0 {
                value_to_count.remove(&value);
            }
            if value_to_count.is_empty() {
                trie.remove(token);
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Tries<T: Clone> {
    // TODO: Allow ART to store N values:
    // https://github.com/get-convex/convex/pull/20030/files#r1427222221
    tries: BTreeMap<(bool, u8), ART<String, BTreeMap<T, usize>>>,
}

impl<T: Clone> Tries<T> {
    fn new() -> Self {
        Self {
            tries: BTreeMap::new(),
        }
    }
}

impl<T: Clone + Ord> Tries<T> {
    fn matching_values(&self, tokens: &mut ValueTokens, result: &mut BTreeSet<T>) {
        for ((prefix, _max_distance), trie) in self.tries.iter() {
            // Prefixing is handled by constructing prefix tokens in ValueTokens (see the
            // notes there), so we can get away with a symmetric search where the dfa's
            // prefix is always set to false.
            tokens.for_each_token(*prefix, |token| {
                if let Some(value) = trie.get(token) {
                    result.extend(value.keys().cloned());
                }
            });
        }
    }
}

impl QueryReads {
    pub fn empty() -> Self {
        QueryReads {
            text_queries: WithHeapSize::default(),
            filter_conditions: WithHeapSize::default(),
            fuzzy_terms: SearchTermTries::new(),
        }
    }

    pub fn merge(&mut self, other: Self) {
        self.fuzzy_terms.extend((), &other.text_queries);

        self.text_queries.extend(other.text_queries);
        self.filter_conditions.extend(other.filter_conditions);
    }

    #[fastrace::trace]
    pub fn overlaps_document(&self, document: &PackedDocument) -> bool {
        let _timer = metrics::query_reads_overlaps_timer();

        for filter_condition in &self.filter_conditions {
            let FilterConditionRead::Must(field_path, filter_value) = filter_condition;
            let document_value = document.value().get_path(field_path);
            let document_value = FilterValue::from_search_value(document_value.as_ref());
            // If the document doesn't match the filter condition, we can skip checking
            // fuzzy terms
            if document_value != *filter_value {
                metrics::log_query_reads_outcome(false);
                return false;
            }
        }

        // If there are no text queries and all filters match, this counts as an
        // overlap.
        if self.text_queries.is_empty() {
            metrics::log_query_reads_outcome(true);
            return true;
        }
        // If all the filter conditions match and there are text queries, we then check
        // for fuzzy matches.
        let analyzer = convex_en();
        let is_fuzzy_match = self.fuzzy_terms.overlaps_document(document, &analyzer);
        metrics::log_query_reads_outcome(is_fuzzy_match);
        is_fuzzy_match
    }

    #[fastrace::trace]
    pub fn overlaps_search_index_key_value(&self, index_key_value: &SearchIndexKeyValue) -> bool {
        let _timer = metrics::query_reads_overlaps_search_value_timer();

        // Filter out documents that don’t match the filter
        for filter_condition in &self.filter_conditions {
            let FilterConditionRead::Must(field_path, filter_value) = filter_condition;

            let Some(document_value) = index_key_value.filter_values.get(field_path) else {
                // This shouldn’t happen because even if the field doesn’t exist in the
                // document, there is a special `FilterValue` value for
                // undefined. This could happen if the write log entry was created concurrently
                // with index definition changes, but it shouldn’t be a problem.
                metrics::log_missing_filter_value();
                return false;
            };

            if *document_value != *filter_value {
                return false;
            }
        }

        // If there are no text queries and all filters match, this counts as an
        // overlap.
        if self.text_queries.is_empty() {
            metrics::log_query_reads_outcome(true);
            return true;
        }
        // If all the filter conditions match and there are text queries, we then check
        // for fuzzy matches.
        let analyzer = convex_en();
        let is_fuzzy_match = self
            .fuzzy_terms
            .overlaps_index_key_value(index_key_value, &analyzer);
        metrics::log_query_reads_outcome(is_fuzzy_match);
        is_fuzzy_match
    }
}

pub struct TextSearchSubscriptions {
    fuzzy_searches: BTreeMap<TabletIndexName, SearchTermTries<SubscriberId>>,
    // TODO: Filter conditions are inefficiently searched, especially in conjunction with text
    // searches. We should eventually optimize this simpler implementation as well.
    filter_conditions: BTreeMap<TabletIndexName, BTreeMap<SubscriberId, Vec<FilterConditionRead>>>,
}

impl TextSearchSubscriptions {
    pub fn new() -> Self {
        Self {
            fuzzy_searches: BTreeMap::new(),
            filter_conditions: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, id: SubscriberId, index: &TabletIndexName, reads: &QueryReads) {
        self.filter_conditions
            .entry(index.clone())
            .or_default()
            .entry(id)
            .or_default()
            .extend(reads.filter_conditions.to_vec());
        self.fuzzy_searches
            .entry(index.clone())
            .or_insert_with(SearchTermTries::new)
            .extend(id, &reads.text_queries)
    }

    pub fn remove(&mut self, id: SubscriberId, index: &TabletIndexName, reads: &QueryReads) {
        let conditions = self
            .filter_conditions
            .get_mut(index)
            .unwrap_or_else(|| panic!("Missing condition index entry for {}", index));
        assert!(conditions.remove(&id).is_some());
        if conditions.is_empty() {
            self.filter_conditions.remove(index);
        }
        let terms = self
            .fuzzy_searches
            .get_mut(index)
            .unwrap_or_else(|| panic!("Missing fuzzy search index entry for {}", index));
        terms.remove(id, &reads.text_queries);
    }

    pub fn add_matches(
        &self,
        document_id: &ResolvedDocumentId,
        document_index_keys: &DocumentIndexKeys,
        to_notify: &mut BTreeSet<SubscriberId>,
    ) {
        self.add_filter_conditions_matches(document_id, document_index_keys, to_notify);
        self.add_fuzzy_matches(document_id, document_index_keys, to_notify);
    }

    fn add_filter_conditions_matches(
        &self,
        document_id: &ResolvedDocumentId,
        document_index_keys: &DocumentIndexKeys,
        to_notify: &mut BTreeSet<SubscriberId>,
    ) {
        for (index, filter_conditions_map) in &self.filter_conditions {
            if *index.table() != document_id.tablet_id {
                continue;
            }

            let Some(DocumentIndexKeyValue::Search(SearchIndexKeyValue { filter_values, .. })) =
                document_index_keys.get(index)
            else {
                metrics::log_missing_index_key();
                continue;
            };

            for (subscriber_id, filter_conditions) in filter_conditions_map {
                for FilterConditionRead::Must(field_path, filter_value) in filter_conditions {
                    let Some(document_value) = filter_values.get(field_path) else {
                        metrics::log_missing_filter_value();
                        continue;
                    };

                    if document_value == filter_value {
                        metrics::log_query_reads_outcome(true);
                        to_notify.insert(*subscriber_id);
                    }
                }
            }
        }
    }

    /// An inverse search where we search document tokens against a trie of read
    /// query terms instead of the more normal trie of the document tokens
    /// against a dfa for each search term.
    ///
    /// This inverse looking search optimizes for cases where the number of
    /// reads/subscriptions is significantly larger than the number of
    /// tokens in the document.
    fn add_fuzzy_matches(
        &self,
        document_id: &ResolvedDocumentId,
        document_index_keys: &DocumentIndexKeys,
        matches: &mut BTreeSet<SubscriberId>,
    ) {
        let analyzer = convex_en();
        for (index, fuzzy_terms) in self
            .fuzzy_searches
            .iter()
            .filter(|(index, _)| *index.table() == document_id.tablet_id)
        {
            let Some(DocumentIndexKeyValue::Search(index_key_value)) =
                document_index_keys.get(index)
            else {
                continue;
            };

            let Some(value) = &index_key_value.search_field_value else {
                continue;
            };

            let Some(tries) = fuzzy_terms.terms.get(&index_key_value.search_field) else {
                continue;
            };

            let mut tokens = ValueTokens::new(&analyzer, value);
            tries.matching_values(&mut tokens, matches);
        }
    }
}

struct ValueTokens {
    tokens: HashSet<String>,
}

impl ValueTokens {
    fn new(analyzer: &TextAnalyzer, value: &ConvexString) -> Self {
        // Tokenizing the value is expensive, but so is constructing a prefix for
        // every token. So we always keep track of the list of tokens, but we
        // only construct the prefixes for each token if we have at least one search in
        // the read set that uses prefixes.
        let mut token_stream = analyzer.token_stream(value);
        let mut tokens = HashSet::new();
        while token_stream.advance() {
            let text = &token_stream.token().text;
            tokens.insert(text.clone());
        }

        ValueTokens { tokens }
    }

    fn for_each_token<F>(&mut self, prefix: bool, mut for_each: F)
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
            for token in self.tokens.iter() {
                for_each(token);
            }
        }
    }

    fn calculate_prefixes(&self) -> impl Iterator<Item = &str> + '_ {
        let mut set: HashSet<&str> = HashSet::new();

        for token in self.tokens.iter() {
            if !set.insert(token) {
                continue;
            }
            for (i, _) in token.char_indices()
                // Skip the first index because 0 up to but not including the
                // first character index is either the empty String or includes
                // a partial character, neither of which is a valid prefix.
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

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        str::FromStr,
    };

    use common::{
        document::ResolvedDocument,
        types::IndexDescriptor,
    };
    use value::{
        ConvexObject,
        ConvexString,
        ConvexValue,
        ResolvedDocumentId,
        TabletId,
    };

    use super::*;

    #[test]
    fn test_search_term_tries_overlaps() -> anyhow::Result<()> {
        let analyzer = convex_en();
        let mut tries = SearchTermTries::new();

        // Create a document with a text field
        let mut map = BTreeMap::new();
        map.insert(
            "title".parse()?,
            ConvexValue::String(ConvexString::try_from("hello world")?),
        );
        let object = ConvexObject::try_from(map)?;
        let doc = PackedDocument::pack(&ResolvedDocument::new(
            ResolvedDocumentId::MIN,
            CreationTime::ONE,
            object,
        )?);

        // Add a search term that matches the document using extend
        let text_query = TextQueryTermRead::new(
            FieldPath::from_str("title")?,
            TextQueryTerm::Exact("hello".to_string()),
        );
        let text_queries = WithHeapSize::from(vec![text_query]);
        tries.extend((), &text_queries);

        // Test that the document matches
        assert!(tries.overlaps_document(&doc, &analyzer));

        // Add a non-matching term
        let text_query = TextQueryTermRead::new(
            FieldPath::from_str("title")?,
            TextQueryTerm::Exact("goodbye".to_string()),
        );
        let text_queries = WithHeapSize::from(vec![text_query]);
        tries.extend((), &text_queries);

        // Document should still match because it matches at least one term
        assert!(tries.overlaps_document(&doc, &analyzer));

        // Create a document that doesn't match any terms
        let mut map = BTreeMap::new();
        map.insert(
            "title".parse()?,
            ConvexValue::String(ConvexString::try_from("bonjour")?),
        );
        let object = ConvexObject::try_from(map)?;
        let doc = PackedDocument::pack(&ResolvedDocument::new(
            ResolvedDocumentId::MIN,
            CreationTime::ONE,
            object,
        )?);

        // Document should not match
        assert!(!tries.overlaps_document(&doc, &analyzer));
        Ok(())
    }

    #[test]
    fn test_search_term_tries_overlaps_returns_false_if_the_field_does_not_exist(
    ) -> anyhow::Result<()> {
        let analyzer = convex_en();
        let mut tries = SearchTermTries::new();
        let text_query = TextQueryTermRead::new(
            FieldPath::from_str("title")?,
            TextQueryTerm::Exact("hello".to_string()),
        );
        let text_queries = WithHeapSize::from(vec![text_query]);
        tries.extend((), &text_queries);

        let doc = PackedDocument::pack(&ResolvedDocument::new(
            ResolvedDocumentId::MIN,
            CreationTime::ONE,
            ConvexObject::try_from(btreemap! {})?,
        )?);

        assert!(!tries.overlaps_document(&doc, &analyzer));
        Ok(())
    }

    #[test]
    fn test_add_fuzzy_matches() -> anyhow::Result<()> {
        let mut subscriptions = TextSearchSubscriptions::new();
        let tablet_id = TabletId::MIN;
        let index = TabletIndexName::new(tablet_id, IndexDescriptor::new("test_index")?)?;
        let subscriber_id = SubscriberId::MIN;

        // Query that matches the document
        let query_reads = QueryReads::new(
            WithHeapSize::from(vec![TextQueryTermRead::new(
                FieldPath::from_str("text")?,
                TextQueryTerm::Exact("hello".to_string()),
            )]),
            WithHeapSize::default(),
        );
        subscriptions.insert(subscriber_id, &index, &query_reads);

        let keys_matching = DocumentIndexKeys::with_search_index_for_test(
            index.clone(),
            FieldPath::from_str("text")?,
            ConvexString::try_from("hello world")?,
        );

        // Test matching
        let mut matches = BTreeSet::new();
        subscriptions.add_fuzzy_matches(&ResolvedDocumentId::MIN, &keys_matching, &mut matches);
        assert!(matches.contains(&subscriber_id));

        // Test non-matching
        let keys_non_matching = DocumentIndexKeys::with_search_index_for_test(
            index.clone(),
            FieldPath::from_str("text")?,
            ConvexString::try_from("different text")?,
        );

        let mut matches = BTreeSet::new();
        subscriptions.add_fuzzy_matches(&ResolvedDocumentId::MIN, &keys_non_matching, &mut matches);
        assert!(matches.is_empty());

        Ok(())
    }

    #[test]
    fn test_add_fuzzy_matches_returns_false_if_the_field_does_not_exist() -> anyhow::Result<()> {
        let mut subscriptions = TextSearchSubscriptions::new();
        let tablet_id = TabletId::MIN;
        let index = TabletIndexName::new(tablet_id, IndexDescriptor::new("test_index")?)?;
        let subscriber_id = SubscriberId::MIN;

        let query_reads = QueryReads::new(
            WithHeapSize::from(vec![TextQueryTermRead::new(
                FieldPath::from_str("text")?,
                TextQueryTerm::Exact("hello".to_string()),
            )]),
            WithHeapSize::default(),
        );

        subscriptions.insert(subscriber_id, &index, &query_reads);

        let index_keys = DocumentIndexKeys::empty_for_test();

        let mut matches = BTreeSet::new();
        subscriptions.add_fuzzy_matches(&ResolvedDocumentId::MIN, &index_keys, &mut matches);
        assert!(matches.is_empty());

        Ok(())
    }
}
