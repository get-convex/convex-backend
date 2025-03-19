//! Read set tracking for an active transaction
use std::{
    collections::BTreeMap,
    sync::LazyLock,
};

use cmd_util::env::env_config;
use common::{
    bootstrap_model::index::database_index::IndexedFields,
    components::ComponentPath,
    document::{
        IndexKeyBuffer,
        PackedDocument,
    },
    interval::{
        Interval,
        IntervalSet,
    },
    knobs::{
        TRANSACTION_MAX_READ_SET_INTERVALS,
        TRANSACTION_MAX_READ_SIZE_BYTES,
        TRANSACTION_MAX_READ_SIZE_ROWS,
    },
    static_span,
    types::{
        PersistenceVersion,
        TabletIndexName,
        Timestamp,
    },
    value::ResolvedDocumentId,
};
use errors::ErrorMetadata;
use search::QueryReads as SearchQueryReads;
use usage_tracking::FunctionUsageTracker;
use value::{
    heap_size::{
        HeapSize,
        WithHeapSize,
    },
    TableName,
};

#[cfg(doc)]
use crate::Transaction;
use crate::{
    database::{
        ConflictingRead,
        ConflictingReadWithWriteSource,
    },
    stack_traces::StackTrace,
    write_log::{
        PackedDocumentUpdate,
        WriteSource,
    },
};

pub const OVER_LIMIT_HELP: &str = "Consider using smaller limits in your queries, paginating your \
                                   queries, or using indexed queries with a selective index range \
                                   expressions.";

/// If set to 'true', then collect backtraces of every database read in order
/// to help debug OCC errors. Collecting stack traces is expensive and should
/// only be used in development.
static READ_SET_CAPTURE_BACKTRACES: LazyLock<bool> =
    LazyLock::new(|| env_config("READ_SET_CAPTURE_BACKTRACES", false));

#[cfg_attr(any(test, feature = "testing"), derive(PartialEq, Eq))]
#[derive(Debug, Clone)]
pub struct IndexReads {
    pub fields: IndexedFields,
    pub intervals: IntervalSet,
    pub stack_traces: Option<Vec<(Interval, StackTrace)>>,
}

impl HeapSize for IndexReads {
    fn heap_size(&self) -> usize {
        self.fields.heap_size() + self.intervals.heap_size()
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq, Eq))]
pub struct ReadSet {
    indexed: WithHeapSize<BTreeMap<TabletIndexName, IndexReads>>,
    search: WithHeapSize<BTreeMap<TabletIndexName, SearchQueryReads>>,
}

impl HeapSize for ReadSet {
    fn heap_size(&self) -> usize {
        self.indexed.heap_size() + self.search.heap_size()
    }
}

impl ReadSet {
    pub fn empty() -> Self {
        Self {
            indexed: WithHeapSize::default(),
            search: WithHeapSize::default(),
        }
    }

    pub fn new(
        indexed: BTreeMap<TabletIndexName, IndexReads>,
        search: BTreeMap<TabletIndexName, SearchQueryReads>,
    ) -> Self {
        Self {
            indexed: indexed.into(),
            search: search.into(),
        }
    }

    /// Iterate over all range reads for the given index.
    pub fn iter_indexed(&self) -> impl Iterator<Item = (&TabletIndexName, &IndexReads)> {
        self.indexed.iter()
    }

    pub fn iter_search(&self) -> impl Iterator<Item = (&TabletIndexName, &SearchQueryReads)> {
        self.search.iter()
    }

    pub fn consume(
        self,
    ) -> (
        impl Iterator<Item = (TabletIndexName, IndexReads)>,
        impl Iterator<Item = (TabletIndexName, SearchQueryReads)>,
    ) {
        (self.indexed.into_iter(), self.search.into_iter())
    }

    /// Determine whether a mutation to a document overlaps with the read set.
    pub fn overlaps(
        &self,
        document: &PackedDocument,
        persistence_version: PersistenceVersion,
    ) -> Option<ConflictingRead> {
        self.overlaps_with_buffer(document, persistence_version, &mut IndexKeyBuffer::new())
    }

    /// `overlaps` but with a reusable `IndexKeyBuffer` to avoid allocations
    pub fn overlaps_with_buffer(
        &self,
        document: &PackedDocument,
        persistence_version: PersistenceVersion,
        buffer: &mut IndexKeyBuffer,
    ) -> Option<ConflictingRead> {
        for (
            index,
            IndexReads {
                fields,
                intervals,
                stack_traces,
            },
        ) in self.indexed.iter()
        {
            if *index.table() == document.id().tablet_id {
                let index_key = document.index_key(fields, persistence_version, buffer);
                if intervals.contains(index_key) {
                    let stack_traces = stack_traces.as_ref().map(|st| {
                        st.iter()
                            .filter_map(|(interval, trace)| {
                                if interval.contains(index_key) {
                                    Some(trace.clone())
                                } else {
                                    None
                                }
                            })
                            .collect()
                    });
                    return Some(ConflictingRead {
                        index: index.clone(),
                        id: document.id(),
                        stack_traces,
                    });
                }
            }
        }

        for (index, search_reads) in self.search.iter() {
            if *index.table() == document.id().tablet_id && search_reads.overlaps(document) {
                return Some(ConflictingRead {
                    index: index.clone(),
                    id: document.id(),
                    stack_traces: None,
                });
            }
        }
        None
    }

    /// writes_overlap is the core logic for
    /// detecting whether a transaction or subscription intersects a commit.
    /// If a write transaction intersects, it will be retried to maintain
    /// serializability. If a subscription intersects, it will be rerun and the
    /// result sent to all clients.
    #[fastrace::trace]
    pub fn writes_overlap<'a>(
        &self,
        updates: impl Iterator<
            Item = (
                &'a Timestamp,
                impl Iterator<Item = &'a (ResolvedDocumentId, PackedDocumentUpdate)>,
                &'a WriteSource,
            ),
        >,
        persistence_version: PersistenceVersion,
    ) -> Option<ConflictingReadWithWriteSource> {
        let mut buffer = IndexKeyBuffer::new();
        for (_ts, updates, write_source) in updates {
            for (_, update) in updates {
                if let Some(ref document) = update.new_document {
                    if let Some(conflicting_read) =
                        self.overlaps_with_buffer(document, persistence_version, &mut buffer)
                    {
                        return Some(ConflictingReadWithWriteSource {
                            read: conflicting_read,
                            write_source: write_source.clone(),
                        });
                    }
                }
                if let Some(ref prev_value) = update.old_document {
                    if let Some(conflicting_read) =
                        self.overlaps_with_buffer(prev_value, persistence_version, &mut buffer)
                    {
                        return Some(ConflictingReadWithWriteSource {
                            read: conflicting_read,
                            write_source: write_source.clone(),
                        });
                    }
                }
            }
        }
        None
    }
}

/// Tracks the read set for the current transaction. Records successful reads as
/// well as missing documents so we can ensure future reads in this transaction
/// are consistent against the current snapshot.
///
/// [`Transaction`] keeps this read set up to date when accessing documents
/// or the index. We want to minimize the amount of code that updates this state
/// so we avoid missing an update.
#[derive(Debug, Clone)]
pub struct TransactionReadSet {
    read_set: ReadSet,

    // Pre-computed sum of all of the `IntervalSet`'s sizes.
    num_intervals: usize,

    user_tx_size: TransactionReadSize,
    system_tx_size: TransactionReadSize,
}

#[cfg(any(test, feature = "testing"))]
impl PartialEq for TransactionReadSet {
    fn eq(&self, other: &Self) -> bool {
        self.read_set.eq(&other.read_set)
            && self.num_intervals.eq(&other.num_intervals)
            && self.user_tx_size.eq(&other.user_tx_size)
            && self.system_tx_size.eq(&other.system_tx_size)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, derive_more::Add, derive_more::AddAssign)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct TransactionReadSize {
    // Sum of doc.size() for all documents read.
    pub total_document_size: usize,
    // Count of all documents read.
    pub total_document_count: usize,
}

impl TransactionReadSet {
    /// Create a read-set at the given timestamp.
    pub fn new() -> Self {
        Self {
            read_set: ReadSet::empty(),
            num_intervals: 0,
            user_tx_size: TransactionReadSize::default(),
            system_tx_size: TransactionReadSize::default(),
        }
    }

    pub fn into_read_set(self) -> ReadSet {
        self.read_set
    }

    pub fn read_set(&self) -> &ReadSet {
        &self.read_set
    }

    fn _record_indexed(
        &mut self,
        index_name: TabletIndexName,
        fields: IndexedFields,
        intervals: impl IntoIterator<Item = Interval>,
    ) -> (usize, usize) {
        self.read_set.indexed.mutate_entry_or_insert_with(
            index_name,
            || IndexReads {
                fields,
                intervals: IntervalSet::new(),
                stack_traces: (*READ_SET_CAPTURE_BACKTRACES).then_some(vec![]),
            },
            |reads| {
                let IndexReads {
                    intervals: range_set,
                    stack_traces,
                    ..
                } = reads;

                let range_num_intervals_before = range_set.len();
                for interval in intervals {
                    if let Some(stack_traces) = stack_traces.as_mut() {
                        stack_traces.push((interval.clone(), StackTrace::new()));
                    }
                    range_set.add(interval);
                }
                let range_num_intervals_after = range_set.len();

                (range_num_intervals_before, range_num_intervals_after)
            },
        )
    }

    /// Call record_indexed_derived to take a read dependency when the user
    /// didn't directly initiate the read and the read didn't go to persistence,
    /// but we are taking a read dependency anyway.
    /// For example, when writing to a table, take a derived read on the table
    /// to make sure it still exists.
    pub fn record_indexed_derived(
        &mut self,
        index_name: TabletIndexName,
        fields: IndexedFields,
        interval: Interval,
    ) {
        self._record_indexed(index_name, fields, [interval]);
    }

    pub fn merge(
        &mut self,
        reads: ReadSet,
        num_intervals: usize,
        user_tx_size: TransactionReadSize,
        system_tx_size: TransactionReadSize,
    ) {
        let (index_reads, search_reads) = reads.consume();
        for (index_name, index_reads) in index_reads {
            self._record_indexed(index_name, index_reads.fields, index_reads.intervals.iter());
        }
        for (index_name, search_reads) in search_reads {
            self.record_search(index_name, search_reads);
        }
        self.num_intervals += num_intervals;
        self.user_tx_size += user_tx_size;
        self.system_tx_size += system_tx_size;
    }

    pub fn record_read_document(
        &mut self,
        component_path: ComponentPath,
        table_name: TableName,
        document_size: usize,
        usage_tracker: &FunctionUsageTracker,
        is_virtual_table: bool,
    ) -> anyhow::Result<()> {
        // Database bandwidth for document reads
        let is_system_table = table_name.is_system() && !is_virtual_table;
        usage_tracker.track_database_egress_size(
            component_path.clone(),
            table_name.to_string(),
            document_size as u64,
            is_system_table,
        );
        usage_tracker.track_database_egress_rows(
            component_path,
            table_name.to_string(),
            1,
            is_system_table,
        );

        let tx_size = if is_system_table {
            &mut self.system_tx_size
        } else {
            &mut self.user_tx_size
        };

        // We always increment the size first, even if we throw,
        // we want the size to reflect the read, so that
        // we can tell that we threw and not issue a warning.
        tx_size.total_document_count += 1;
        tx_size.total_document_size += document_size;

        if !is_system_table {
            anyhow::ensure!(
                tx_size.total_document_count <= *TRANSACTION_MAX_READ_SIZE_ROWS,
                ErrorMetadata::pagination_limit(
                    "TooManyDocumentsRead",
                    format!(
                        "Too many documents read in a single function execution (limit: {}). \
                         {OVER_LIMIT_HELP}",
                        *TRANSACTION_MAX_READ_SIZE_ROWS,
                    )
                ),
            );
            anyhow::ensure!(
                tx_size.total_document_size <= *TRANSACTION_MAX_READ_SIZE_BYTES,
                ErrorMetadata::pagination_limit(
                    "TooManyBytesRead",
                    format!(
                        "Too many bytes read in a single function execution (limit: {} bytes). \
                         {OVER_LIMIT_HELP}",
                        *TRANSACTION_MAX_READ_SIZE_BYTES,
                    )
                ),
            );
        }
        Ok(())
    }

    pub fn record_indexed_directly(
        &mut self,
        index_name: TabletIndexName,
        fields: IndexedFields,
        interval: Interval,
    ) -> anyhow::Result<()> {
        let _s = static_span!();

        let (num_intervals_before, num_intervals_after) =
            self._record_indexed(index_name, fields, [interval]);

        self.num_intervals = self.num_intervals.saturating_sub(num_intervals_before);
        self.num_intervals += num_intervals_after;
        if self.num_intervals > *TRANSACTION_MAX_READ_SET_INTERVALS {
            anyhow::bail!(
                anyhow::anyhow!("top three: {}", self.top_three_intervals()).context(
                    ErrorMetadata::pagination_limit(
                        "TooManyReads",
                        format!(
                            "Too many reads in a single function execution (limit: {}). \
                             {OVER_LIMIT_HELP}",
                            *TRANSACTION_MAX_READ_SET_INTERVALS,
                        ),
                    )
                )
            );
        }
        Ok(())
    }

    pub fn top_three_intervals(&self) -> String {
        let mut intervals: Vec<_> = self
            .read_set
            .indexed
            .iter()
            .map(|(index, reads)| (reads.intervals.len(), index))
            .collect();
        intervals.sort_by_key(|(len, _)| *len);
        let top_three = intervals
            .iter()
            .rev()
            .take(3)
            .map(|(amt, index)| format!("{index}: {amt}"))
            .collect::<Vec<_>>();
        top_three.join(",")
    }

    pub fn record_search(&mut self, index_name: TabletIndexName, search_reads: SearchQueryReads) {
        self.read_set.search.mutate_entry_or_insert_with(
            index_name,
            SearchQueryReads::empty,
            |existing_reads| existing_reads.merge(search_reads),
        );
    }

    pub fn num_intervals(&self) -> usize {
        self.num_intervals
    }

    pub fn user_tx_size(&self) -> &TransactionReadSize {
        &self.user_tx_size
    }

    pub fn system_tx_size(&self) -> &TransactionReadSize {
        &self.system_tx_size
    }
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for ReadSet {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = ReadSet>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        #[derive(Debug, proptest_derive::Arbitrary)]
        struct GeneratedReads {
            #[proptest(
                strategy = "prop::collection::vec(any::<(TabletIndexName, IndexedFields, \
                            IntervalSet)>(), 0..4)"
            )]
            entries: Vec<(TabletIndexName, IndexedFields, IntervalSet)>,
            #[proptest(strategy = "prop::collection::vec(any::<(TabletIndexName, \
                                   SearchQueryReads)>(), 0..4)")]
            search: Vec<(TabletIndexName, SearchQueryReads)>,
        }

        any::<GeneratedReads>().prop_map(|generated_reads| {
            let indexed = generated_reads
                .entries
                .into_iter()
                .map(|(index_name, fields, intervals)| {
                    (
                        index_name,
                        IndexReads {
                            fields,
                            intervals,
                            stack_traces: None,
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>();
            let search = generated_reads
                .search
                .into_iter()
                .collect::<BTreeMap<_, _>>();
            Self {
                indexed: indexed.into(),
                search: search.into(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use common::{
        assert_obj,
        document::{
            CreationTime,
            PackedDocument,
            ResolvedDocument,
        },
        query::search_value_to_bytes,
        testing::TestIdGenerator,
        types::{
            IndexDescriptor,
            PersistenceVersion,
            TabletIndexName,
        },
        value::{
            ConvexValue,
            FieldPath,
            ResolvedDocumentId,
        },
    };
    use search::{
        query::{
            FuzzyDistance,
            TextQueryTerm,
        },
        FilterConditionRead,
        QueryReads as SearchQueryReads,
        TextQueryTermRead,
    };
    use value::val;

    use super::TransactionReadSet;
    use crate::ReadSet;

    fn create_document_with_one_field(
        id: ResolvedDocumentId,
        field_name: &str,
        value: ConvexValue,
    ) -> anyhow::Result<ResolvedDocument> {
        ResolvedDocument::new(
            id,
            CreationTime::ONE,
            assert_obj!(
                field_name => value
            ),
        )
    }

    fn create_document_with_extra_field(
        id: ResolvedDocumentId,
        field_name: &str,
        value: ConvexValue,
    ) -> anyhow::Result<ResolvedDocument> {
        ResolvedDocument::new(
            id,
            CreationTime::ONE,
            assert_obj!(
                field_name => value,
                "extraField" => ConvexValue::String("word".to_string().try_into()?),
            ),
        )
    }

    #[test]
    fn search_fuzzy_text_no_prefix_0_distance_reads() -> anyhow::Result<()> {
        let mut reads = TransactionReadSet::new();
        let mut id_generator = TestIdGenerator::new();
        let table_name = "mytable".parse()?;
        let table_id = id_generator.user_table_id(&table_name);
        let index_name =
            TabletIndexName::new(table_id.tablet_id, IndexDescriptor::new("search_index")?)?;
        let field_path = "textField";

        let search_reads = SearchQueryReads::new(
            vec![TextQueryTermRead {
                field_path: FieldPath::from_str(field_path)?,
                term: TextQueryTerm::Fuzzy {
                    max_distance: FuzzyDistance::Zero,
                    token: "word".to_string(),
                    prefix: false,
                },
            }]
            .into(),
            vec![].into(),
        );

        reads.record_search(index_name.clone(), search_reads);

        let read_set = reads.into_read_set();
        let id = id_generator.user_generate(&table_name);

        assert!(read_set_overlaps(
            id,
            &read_set,
            field_path,
            // If "word" is a token, it overlaps.
            "Text containing word and other stuff."
        )?);

        assert!(!read_set_overlaps(
            id,
            &read_set,
            field_path,
            // If "word" is just a substring, it does not overlap.
            "This text doesn't have the keyword."
        )?);

        Ok(())
    }

    #[test]
    fn search_fuzzy_text_no_prefix_1_distance_reads() -> anyhow::Result<()> {
        let mut reads = TransactionReadSet::new();
        let mut id_generator = TestIdGenerator::new();
        let table_name = "mytable".parse()?;
        let table_id = id_generator.user_table_id(&table_name);
        let index_name =
            TabletIndexName::new(table_id.tablet_id, IndexDescriptor::new("search_index")?)?;
        let field_path = "textField";

        let search_reads = SearchQueryReads::new(
            vec![TextQueryTermRead {
                field_path: FieldPath::from_str(field_path)?,
                term: TextQueryTerm::Fuzzy {
                    max_distance: FuzzyDistance::One,
                    token: "wod".to_string(),
                    prefix: false,
                },
            }]
            .into(),
            vec![].into(),
        );

        reads.record_search(index_name.clone(), search_reads);

        let read_set = reads.into_read_set();
        let id = id_generator.user_generate(&table_name);

        assert!(!read_set_overlaps(
            id,
            &read_set,
            field_path,
            // If "word" is just a substring, it does not overlap.
            "This text doesn't have the keyword."
        )?);

        Ok(())
    }

    #[test]
    fn search_fuzzy_text_no_prefix_2_distance_reads() -> anyhow::Result<()> {
        let mut reads = TransactionReadSet::new();
        let mut id_generator = TestIdGenerator::new();
        let table_name = "mytable".parse()?;
        let table_id = id_generator.user_table_id(&table_name);
        let index_name =
            TabletIndexName::new(table_id.tablet_id, IndexDescriptor::new("search_index")?)?;
        let field_path = "textField";

        let search_reads = SearchQueryReads::new(
            vec![TextQueryTermRead {
                field_path: FieldPath::from_str(field_path)?,
                term: TextQueryTerm::Fuzzy {
                    max_distance: FuzzyDistance::Two,
                    token: "word".to_string(),
                    prefix: false,
                },
            }]
            .into(),
            vec![].into(),
        );

        reads.record_search(index_name.clone(), search_reads);

        let read_set = reads.into_read_set();
        let id = id_generator.user_generate(&table_name);

        assert!(read_set_overlaps(
            id,
            &read_set,
            field_path,
            "Text containing word and other stuff."
        )?);
        assert!(!read_set_overlaps(
            id,
            &read_set,
            field_path,
            "This text doesn't have the keyword."
        )?);
        Ok(())
    }

    #[test]
    fn search_fuzzy_text_prefix_0_distance_reads() -> anyhow::Result<()> {
        let mut reads = TransactionReadSet::new();
        let mut id_generator = TestIdGenerator::new();
        let table_name = "mytable".parse()?;
        let table_id = id_generator.user_table_id(&table_name);
        let index_name =
            TabletIndexName::new(table_id.tablet_id, IndexDescriptor::new("search_index")?)?;
        let field_path = "textField";

        let search_reads = SearchQueryReads::new(
            vec![TextQueryTermRead {
                field_path: FieldPath::from_str(field_path)?,
                term: TextQueryTerm::Fuzzy {
                    max_distance: FuzzyDistance::Zero,
                    token: "word".to_string(),
                    prefix: true,
                },
            }]
            .into(),
            vec![].into(),
        );

        reads.record_search(index_name.clone(), search_reads);

        let read_set = reads.into_read_set();
        let id = id_generator.user_generate(&table_name);

        assert!(read_set_overlaps(
            id,
            &read_set,
            field_path,
            // If "word.*" is a token, it overlaps.
            "Text containing words and other stuff."
        )?);

        assert!(!read_set_overlaps(
            id,
            &read_set,
            field_path,
            // If "word.*" is just a substring, it does not overlap.
            "This text doesn't have the keyword."
        )?);

        Ok(())
    }

    #[test]
    fn search_fuzzy_text_prefix_1_distance_reads() -> anyhow::Result<()> {
        let mut reads = TransactionReadSet::new();
        let mut id_generator = TestIdGenerator::new();
        let table_name = "mytable".parse()?;
        let table_id = id_generator.user_table_id(&table_name);
        let index_name =
            TabletIndexName::new(table_id.tablet_id, IndexDescriptor::new("search_index")?)?;
        let field_path = "textField";

        let search_reads = SearchQueryReads::new(
            vec![TextQueryTermRead {
                field_path: FieldPath::from_str(field_path)?,
                term: TextQueryTerm::Fuzzy {
                    max_distance: FuzzyDistance::One,
                    token: "wrd".to_string(),
                    prefix: true,
                },
            }]
            .into(),
            vec![].into(),
        );

        reads.record_search(index_name.clone(), search_reads);

        let read_set = reads.into_read_set();
        let id = id_generator.user_generate(&table_name);

        assert!(read_set_overlaps(
            id,
            &read_set,
            field_path,
            // If "wrd.*" is a token, it overlaps.
            "Text containing wrdsythings and other stuff."
        )?);

        assert!(!read_set_overlaps(
            id,
            &read_set,
            field_path,
            // If "word.*" is just a substring, it does not overlap.
            "This text doesn't have keyword."
        )?);

        Ok(())
    }

    #[test]
    fn search_fuzzy_text_prefix_2_distance_reads() -> anyhow::Result<()> {
        let mut reads = TransactionReadSet::new();
        let mut id_generator = TestIdGenerator::new();
        let table_name = "mytable".parse()?;
        let table_id = id_generator.user_table_id(&table_name);
        let index_name =
            TabletIndexName::new(table_id.tablet_id, IndexDescriptor::new("search_index")?)?;
        let field_path = "textField";

        let search_reads = SearchQueryReads::new(
            vec![TextQueryTermRead {
                field_path: FieldPath::from_str(field_path)?,
                term: TextQueryTerm::Fuzzy {
                    max_distance: FuzzyDistance::Two,
                    token: "word".to_string(),
                    prefix: true,
                },
            }]
            .into(),
            vec![].into(),
        );

        reads.record_search(index_name.clone(), search_reads);

        let read_set = reads.into_read_set();
        let id = id_generator.user_generate(&table_name);

        assert!(read_set_overlaps(
            id,
            &read_set,
            field_path,
            // If "word.*" is a token, it overlaps.
            "Text containing wordsythings and other stuff."
        )?);
        // This would fail if prefix s false
        assert!(read_set_overlaps(
            id,
            &read_set,
            field_path,
            "Text containing wordddd and other stuff."
        )?);

        assert!(!read_set_overlaps(
            id,
            &read_set,
            field_path,
            // If "word.*" is just a substring, it does not overlap.
            "This text doesn't have keyword."
        )?);

        Ok(())
    }

    fn read_set_overlaps(
        id: ResolvedDocumentId,
        read_set: &ReadSet,
        field_name: &str,
        document_text: &str,
    ) -> anyhow::Result<bool> {
        let doc_without_word = create_document_with_one_field(id, field_name, val!(document_text))?;
        Ok(read_set
            .overlaps(
                &PackedDocument::pack(doc_without_word),
                PersistenceVersion::default(),
            )
            .is_some())
    }

    #[test]
    fn test_search_exact_text_reads() -> anyhow::Result<()> {
        let mut reads = TransactionReadSet::new();
        let mut id_generator = TestIdGenerator::new();
        let table_name = "mytable".parse()?;
        let table_id = id_generator.user_table_id(&table_name);
        let index_name =
            TabletIndexName::new(table_id.tablet_id, IndexDescriptor::new("search_index")?)?;

        let search_reads = SearchQueryReads::new(
            vec![TextQueryTermRead {
                field_path: FieldPath::from_str("textField")?,
                term: TextQueryTerm::Exact("word".to_string()),
            }]
            .into(),
            vec![].into(),
        );

        reads.record_search(index_name.clone(), search_reads);

        let read_set = reads.into_read_set();

        // If "word" is a token, it overlaps.
        let doc_with_word = create_document_with_one_field(
            id_generator.user_generate(&table_name),
            "textField",
            val!("Text containing word and other stuff."),
        )?;
        assert_eq!(
            read_set
                .overlaps(
                    &PackedDocument::pack(doc_with_word),
                    PersistenceVersion::default()
                )
                .unwrap()
                .index,
            index_name
        );

        // If "word" is just a substring, it does not.
        let doc_without_word = create_document_with_one_field(
            id_generator.user_generate(&table_name),
            "textField",
            val!("This text doesn't have the keyword."),
        )?;
        assert_eq!(
            read_set.overlaps(
                &PackedDocument::pack(doc_without_word),
                PersistenceVersion::default()
            ),
            None
        );

        Ok(())
    }

    #[test]
    fn test_search_filter_reads_empty_query() -> anyhow::Result<()> {
        let mut reads = TransactionReadSet::new();
        let mut id_generator = TestIdGenerator::new();
        let table_name = "mytable".parse()?;
        let table_id = id_generator.user_table_id(&table_name);
        let index_name =
            TabletIndexName::new(table_id.tablet_id, IndexDescriptor::new("search_index")?)?;

        let search_reads = SearchQueryReads::new(
            vec![].into(),
            vec![FilterConditionRead::Must(
                FieldPath::from_str("nullField")?,
                search_value_to_bytes(Some(&ConvexValue::Null)),
            )]
            .into(),
        );

        reads.record_search(index_name.clone(), search_reads);

        let read_set = reads.into_read_set();

        // If "nullField" is Null, it overlaps.
        let doc_with_explicit_null = create_document_with_one_field(
            id_generator.user_generate(&table_name),
            "nullField",
            ConvexValue::Null,
        )?;
        assert_eq!(
            read_set
                .overlaps(
                    &PackedDocument::pack(doc_with_explicit_null),
                    PersistenceVersion::default()
                )
                .unwrap()
                .index,
            index_name
        );

        // If "nullField" is not present, it does not overlap.
        let doc_with_missing_field = create_document_with_one_field(
            id_generator.user_generate(&table_name),
            "unrelatedField",
            ConvexValue::Null,
        )?;
        assert_eq!(
            read_set.overlaps(
                &PackedDocument::pack(doc_with_missing_field),
                PersistenceVersion::default()
            ),
            None
        );

        // If "nullField" is a different type, it does not overlap.
        let doc_with_implicit_null = create_document_with_one_field(
            id_generator.user_generate(&table_name),
            "nullField",
            ConvexValue::Int64(123),
        )?;
        assert_eq!(
            read_set.overlaps(
                &PackedDocument::pack(doc_with_implicit_null),
                PersistenceVersion::default()
            ),
            None
        );

        Ok(())
    }
    #[test]
    fn test_search_filter_reads() -> anyhow::Result<()> {
        let mut reads = TransactionReadSet::new();
        let mut id_generator = TestIdGenerator::new();
        let table_name = "mytable".parse()?;
        let table_id = id_generator.user_table_id(&table_name);
        let index_name =
            TabletIndexName::new(table_id.tablet_id, IndexDescriptor::new("search_index")?)?;

        let search_reads = SearchQueryReads::new(
            vec![TextQueryTermRead {
                field_path: FieldPath::from_str("extraField")?,
                term: TextQueryTerm::Fuzzy {
                    max_distance: FuzzyDistance::Zero,
                    token: "word".to_string(),
                    prefix: false,
                },
            }]
            .into(),
            vec![FilterConditionRead::Must(
                FieldPath::from_str("nullField")?,
                search_value_to_bytes(Some(&ConvexValue::Null)),
            )]
            .into(),
        );

        reads.record_search(index_name.clone(), search_reads);

        let read_set = reads.into_read_set();

        // If "nullField" is Null, it overlaps.
        let doc_with_explicit_null = create_document_with_extra_field(
            id_generator.user_generate(&table_name),
            "nullField",
            ConvexValue::Null,
        )?;
        assert_eq!(
            read_set
                .overlaps(
                    &PackedDocument::pack(doc_with_explicit_null),
                    PersistenceVersion::default()
                )
                .unwrap()
                .index,
            index_name
        );

        // If "nullField" is not present, it does not overlap.
        let doc_with_missing_field = create_document_with_extra_field(
            id_generator.user_generate(&table_name),
            "unrelatedField",
            ConvexValue::Null,
        )?;
        assert_eq!(
            read_set.overlaps(
                &PackedDocument::pack(doc_with_missing_field),
                PersistenceVersion::default()
            ),
            None
        );

        // If "nullField" is a different type, it does not overlap.
        let doc_with_implicit_null = create_document_with_extra_field(
            id_generator.user_generate(&table_name),
            "nullField",
            ConvexValue::Int64(123),
        )?;
        assert_eq!(
            read_set.overlaps(
                &PackedDocument::pack(doc_with_implicit_null),
                PersistenceVersion::default()
            ),
            None
        );

        Ok(())
    }
}
