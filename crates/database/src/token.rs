//! Externalizable tokens that record the currently-observed state within a
//! transaction.

#[cfg(any(test, feature = "testing"))]
use common::types::TabletIndexName;
use common::types::Timestamp;
#[cfg(any(test, feature = "testing"))]
use search::query::TextQueryTerm;
use value::heap_size::HeapSize;
#[cfg(any(test, feature = "testing"))]
use value::FieldPath;

use crate::reads::ReadSet;

/// Serialized representation of [`Token`].
pub type SerializedToken = String;

/// A token is a base64 serializable representation of the current read-state
/// for a transaction. This can be externalized to a user and used to represent
/// current transaction state.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct Token {
    read_set: ReadSet,
    ts: Timestamp,
}

impl Token {
    #[allow(unused)]
    #[cfg(any(test, feature = "testing"))]
    pub fn new_for_testing(read_set: ReadSet, ts: Timestamp) -> Self {
        Self::new(read_set, ts)
    }

    #[allow(unused)]
    #[cfg(any(test, feature = "testing"))]
    pub fn text_search_token(
        index_name: TabletIndexName,
        field_path: FieldPath,
        terms: Vec<TextQueryTerm>,
    ) -> Self {
        use std::time::Duration;

        use common::types::TabletIndexName;
        use pb::searchlight::TextQueryTerm;
        use search::{
            QueryReads,
            TextQueryTermRead,
        };
        use value::heap_size::WithHeapSize;

        use crate::TransactionReadSet;

        let mut read_set = TransactionReadSet::new();
        let mut text_queries: WithHeapSize<Vec<TextQueryTermRead>> = WithHeapSize::default();

        for term in terms {
            text_queries.push(TextQueryTermRead::new(field_path.clone(), term));
        }

        let query_reads = QueryReads::new(text_queries, WithHeapSize::default());

        read_set.record_search(index_name, query_reads);
        let read_set = read_set.into_read_set();
        Token::new_for_testing(
            read_set,
            Timestamp::MIN.add(Duration::from_secs(1)).unwrap(),
        )
    }

    pub(crate) fn new(read_set: ReadSet, ts: Timestamp) -> Self {
        Self { read_set, ts }
    }

    pub fn empty(ts: Timestamp) -> Self {
        Self {
            read_set: ReadSet::empty(),
            ts,
        }
    }

    pub fn ts(&self) -> Timestamp {
        self.ts
    }

    pub fn reads(&self) -> &ReadSet {
        &self.read_set
    }

    pub fn into_reads(self) -> ReadSet {
        self.read_set
    }

    /// Advance the token's timestamp to a new timestamp.
    pub fn advance_ts(&mut self, ts: Timestamp) {
        assert!(self.ts < ts);
        self.ts = ts;
    }
}

impl HeapSize for Token {
    fn heap_size(&self) -> usize {
        self.read_set.heap_size()
    }
}
