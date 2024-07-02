use std::{
    collections::{
        btree_map::Entry,
        BTreeMap,
        BTreeSet,
    },
    fmt::{
        Display,
        Formatter,
    },
    ops::Range,
    sync::Arc,
};

use async_trait::async_trait;
use cmd_util::env::env_config;
use common::{
    bootstrap_model::index::{
        text_index::FragmentedTextSegment,
        vector_index::FragmentedVectorSegment,
        IndexMetadata,
    },
    floating_point::assert_approx_equal,
    pause::PauseController,
    persistence::Persistence,
    query::{
        CursorPosition,
        Query,
        QueryOperator,
        QuerySource,
        Search,
        SearchFilterExpression,
        SearchVersion,
    },
    runtime::SpawnHandle,
    types::{
        IndexName,
        ObjectKey,
        Timestamp,
    },
    value::{
        sorting::{
            sorting_decode::bytes_to_values,
            TotalOrdF64,
        },
        ConvexValue,
        ResolvedDocumentId,
        TableName,
    },
    version::MIN_NPM_VERSION_FOR_FUZZY_SEARCH,
};
use errors::ErrorMetadataAnyhowExt;
use futures::{
    future::BoxFuture,
    pin_mut,
    select_biased,
    FutureExt,
};
use keybroker::Identity;
use maplit::btreeset;
use must_let::must_let;
use pb::searchlight::FragmentedVectorSegmentPaths;
use proptest::prelude::*;
use proptest_derive::Arbitrary;
use runtime::testing::{
    TestDriver,
    TestRuntime,
};
use search::{
    query::{
        CompiledQuery,
        TermShortlist,
    },
    scoring::Bm25StatisticsDiff,
    searcher::{
        Bm25Stats,
        FragmentedTextStorageKeys,
        InProcessSearcher,
        PostingListMatch,
        PostingListQuery,
        Term,
        TokenMatch,
        TokenQuery,
    },
    SearchQueryResult,
    Searcher,
    TantivySearchIndexSchema,
    MAX_CANDIDATE_REVISIONS,
};
use storage::Storage;
use usage_tracking::FunctionUsageTracker;
use value::{
    assert_obj,
    TableNamespace,
};
use vector::{
    CompiledVectorSearch,
    QdrantSchema,
    VectorSearchQueryResult,
    VectorSearcher,
};

use crate::{
    index_workers::{
        search_compactor::CompactionConfig,
        writer::SearchIndexMetadataWriter,
    },
    search_and_vector_bootstrap::FINISHED_BOOTSTRAP_UPDATES,
    test_helpers::{
        DbFixtures,
        DbFixturesArgs,
    },
    text_index_worker::{
        compactor::new_text_compactor,
        flusher::new_text_flusher,
        BuildTextIndexArgs,
    },
    Database,
    IndexModel,
    ResolvedQuery,
    TableModel,
    TestFacingModel,
    UserFacingModel,
};

#[derive(Clone)]
struct Scenario {
    rt: TestRuntime,
    database: Database<TestRuntime>,

    search_storage: Arc<dyn Storage>,
    searcher: Arc<dyn Searcher>,
    build_index_args: BuildTextIndexArgs,
    // Add test persistence here, or just change everything to use db fixtures.
    tp: Arc<dyn Persistence>,

    table_name: TableName,
    namespace: TableNamespace,

    // Store a simple mapping of a test string to an array of test
    // strings (the search field) and a filter field
    model: BTreeMap<String, (ResolvedDocumentId, String, String)>,
}

impl Scenario {
    async fn new(rt: TestRuntime) -> anyhow::Result<Self> {
        Self::new_with_searcher(rt.clone(), InProcessSearcher::new(rt).await?).await
    }

    async fn new_with_searcher(rt: TestRuntime, searcher: impl Searcher) -> anyhow::Result<Self> {
        let DbFixtures {
            db: database,
            search_storage,
            searcher,
            tp,
            build_index_args,
            ..
        } = DbFixtures::new_with_args(
            &rt,
            DbFixturesArgs {
                searcher: Some(Arc::new(searcher)),
                ..Default::default()
            },
        )
        .await?;

        let table_name: TableName = "test".parse()?;
        let namespace = TableNamespace::test_user();
        let mut tx = database.begin(Identity::system()).await?;
        TableModel::new(&mut tx)
            .insert_table_metadata_for_test(TableNamespace::test_user(), &table_name)
            .await?;
        let index = IndexMetadata::new_backfilling_search_index(
            "test.by_text".parse()?,
            "searchField".parse()?,
            btreeset! {"filterField".parse()?},
        );
        IndexModel::new(&mut tx)
            .add_application_index(namespace, index)
            .await?;
        database.commit(tx).await?;

        let mut self_ = Self {
            rt,
            database,
            search_storage,
            searcher,
            tp,
            build_index_args,

            table_name,
            namespace,
            model: BTreeMap::new(),
        };
        self_.backfill().await?;
        self_.enable_index().await?;
        Ok(self_)
    }

    async fn set_bootstrapping(&mut self) -> anyhow::Result<()> {
        let DbFixtures {
            db,
            searcher,
            search_storage,
            tp,
            build_index_args,
            ..
        } = DbFixtures::new_with_args(
            &self.rt,
            DbFixturesArgs {
                tp: Some(self.tp.clone()),
                searcher: Some(self.searcher.clone()),
                search_storage: Some(self.search_storage.clone()),
                bootstrap_search_and_vector_indexes: false,
                ..Default::default()
            },
        )
        .await?;

        self.database = db;
        self.searcher = searcher;
        self.search_storage = search_storage;
        self.tp = tp;
        self.build_index_args = build_index_args;
        Ok(())
    }

    async fn compact(&mut self) -> anyhow::Result<()> {
        let writer = SearchIndexMetadataWriter::new(
            self.rt.clone(),
            self.database.clone(),
            self.tp.reader(),
            self.search_storage.clone(),
            self.build_index_args.clone(),
        );
        new_text_compactor(
            self.database.clone(),
            self.searcher.clone(),
            self.search_storage.clone(),
            CompactionConfig::default(),
            writer,
        )
        .step()
        .await?;
        Ok(())
    }

    async fn backfill(&mut self) -> anyhow::Result<()> {
        let writer = SearchIndexMetadataWriter::new(
            self.rt.clone(),
            self.database.clone(),
            self.tp.reader(),
            self.search_storage.clone(),
            self.build_index_args.clone(),
        );
        let mut flusher = new_text_flusher(
            self.rt.clone(),
            self.database.clone(),
            self.tp.reader(),
            self.search_storage.clone(),
            self.build_index_args.segment_term_metadata_fetcher.clone(),
            writer,
        );
        flusher.step().await?;

        Ok(())
    }

    async fn enable_index(&mut self) -> anyhow::Result<()> {
        let mut txn = self.database.begin_system().await?;
        IndexModel::new(&mut txn)
            .enable_index_for_testing(
                self.namespace,
                &IndexName::new("test".parse()?, "by_text".parse()?)?,
            )
            .await?;
        self.database.commit(txn).await?;
        Ok(())
    }

    async fn _query_with_scores<S: Into<String>>(
        &self,
        query_string: S,
        filter: Option<String>,
        ts: Option<Timestamp>,
        version: SearchVersion,
    ) -> anyhow::Result<Vec<(ResolvedDocumentId, f64)>> {
        let mut filters = vec![SearchFilterExpression::Search(
            "searchField".parse()?,
            query_string.into(),
        )];
        if let Some(filter_field) = filter {
            filters.push(SearchFilterExpression::Eq(
                "filterField".parse()?,
                Some(filter_field.try_into()?),
            ));
        }
        let search = Search {
            index_name: "test.by_text".parse()?,
            table: self.table_name.clone(),
            filters,
        };
        let query = Query {
            source: QuerySource::Search(search),
            operators: vec![QueryOperator::Limit(MAX_CANDIDATE_REVISIONS)],
        };

        let mut tx = if let Some(ts) = ts {
            self.database
                .begin_with_ts(Identity::system(), ts, FunctionUsageTracker::new())
                .await?
        } else {
            self.database.begin(Identity::system()).await?
        };

        let mut query_stream = match version {
            SearchVersion::V1 => ResolvedQuery::new(&mut tx, self.namespace, query)?,
            SearchVersion::V2 => ResolvedQuery::new_with_version(
                &mut tx,
                self.namespace,
                query,
                Some(MIN_NPM_VERSION_FOR_FUZZY_SEARCH.clone()),
            )?,
        };
        let mut returned = Vec::new();
        while let Some(value) = query_stream.next(&mut tx, None).await? {
            must_let!(let Some(cursor) =  query_stream.cursor());
            must_let!(let CursorPosition::After(index_key) = cursor.position);
            let reader = &mut &index_key[..];
            let index_key_values = bytes_to_values(reader)?;
            must_let!(
                let Some(ConvexValue::Float64(negative_score)) = index_key_values[0].clone()
            );
            returned.push((value.id(), -negative_score))
        }
        Ok(returned)
    }

    async fn query_with_scores(
        &self,
        test_query: &TestQuery,
        ts: Option<Timestamp>,
        version: SearchVersion,
    ) -> anyhow::Result<Vec<(ResolvedDocumentId, f64)>> {
        let query_string = test_query
            .search
            .iter()
            .map(|key| key.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let filter_field = test_query.filter.map(|filter| filter.to_string());
        self._query_with_scores(query_string, filter_field, ts, version)
            .await
    }

    // TODO: remove once not needed anymore. I forsee still using this for testing a
    // bit so not removing for now.
    #[allow(unused)]
    async fn query_with_scores_and_verify_version_results_match(
        &self,
        test_query: &TestQuery,
        ts: Option<Timestamp>,
    ) -> anyhow::Result<Vec<(ResolvedDocumentId, f64)>> {
        let left = self
            .query_with_scores(test_query, ts, SearchVersion::V2)
            .await?;
        let right = self
            .query_with_scores(test_query, ts, SearchVersion::V2)
            .await?;
        assert_query_results_approx_equal(&left, &right);
        Ok(left)
    }

    // Box the future to avoid stack overflow.
    fn patch(
        &mut self,
        key: TestKey,
        search_field: Vec<TestValue>,
        filter_field: TestValue,
    ) -> BoxFuture<'_, anyhow::Result<Timestamp>> {
        let future = async move {
            let text = search_field
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            let filter_field = format!("{filter_field:?}");
            let (_, ts) = self._patch(key.to_string(), text, filter_field).await?;
            Ok(ts)
        };
        future.boxed()
    }

    async fn insert<S: Into<String>, F: Into<String>>(
        &self,
        search_field: S,
        filter_field: F,
    ) -> anyhow::Result<Timestamp> {
        let search_field = search_field.into();
        let filter_field = filter_field.into();
        let mut tx = self.database.begin(Identity::system()).await?;
        TestFacingModel::new(&mut tx)
            .insert(
                &self.table_name,
                assert_obj!("searchField" => search_field, "filterField" => filter_field),
            )
            .await?;
        self.database.commit(tx).await
    }

    async fn _patch<K: Into<String>, S: Into<String>, F: Into<String>>(
        &mut self,
        key: K,
        search_field: S,
        filter_field: F,
    ) -> anyhow::Result<(ResolvedDocumentId, Timestamp)> {
        let key = key.into();
        let search_field = search_field.into();
        let filter_field = filter_field.into();
        let mut tx = self.database.begin(Identity::system()).await?;
        let new_document = assert_obj!("searchField" => search_field.clone(), "filterField" => filter_field.clone());
        let document_id = match self.model.entry(key) {
            Entry::Vacant(e) => {
                let document_id = TestFacingModel::new(&mut tx)
                    .insert(&self.table_name, new_document)
                    .await?;
                e.insert((document_id, search_field, filter_field));
                document_id
            },
            Entry::Occupied(mut e) => {
                let (document_id, ..) = e.get();
                UserFacingModel::new_root_for_test(&mut tx)
                    .patch((*document_id).into(), new_document.into())
                    .await?;
                e.get_mut().1 = search_field;
                e.get_mut().2 = filter_field;
                e.get().0
            },
        };
        let ts = self.database.commit(tx).await?;
        Ok((document_id, ts))
    }

    async fn execute(&mut self, action: TestAction) -> anyhow::Result<()> {
        match action {
            TestAction::Backfill => {
                self.backfill().await?;
            },
            TestAction::Update(TestUpdate {
                key,
                search_field,
                filter_field,
            }) => {
                self.patch(key, search_field, filter_field).await?;
            },
            TestAction::Delete(k) => {
                if let Some((id, ..)) = self.model.remove(&k.to_string()) {
                    let mut tx = self.database.begin(Identity::system()).await?;
                    UserFacingModel::new_root_for_test(&mut tx)
                        .delete(id.into())
                        .await?;
                    self.database.commit(tx).await?;
                }
            },
            TestAction::QueryAndCheckResults(query) => {
                let query_set: BTreeSet<_> = query
                    .search
                    .clone()
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect();

                // Find all the documents that contain any of the keywords and
                // match the filter.
                let expected: BTreeSet<_> = self
                    .model
                    .values()
                    .filter(|(_, values, _)| {
                        values.split(' ').any(|value| query_set.contains(value))
                    })
                    .filter(|(_, _, filter_field)| {
                        if let Some(filter) = query.filter {
                            filter_field == &filter.to_string()
                        } else {
                            true
                        }
                    })
                    .map(|(id, ..)| *id)
                    .collect();

                let returned: BTreeSet<_> = self
                    .query_with_scores(&query, None, SearchVersion::V2)
                    .await?
                    .into_iter()
                    .map(|(id, _)| id)
                    .collect();

                assert_eq!(expected, returned);
            },
            TestAction::QueryAndCheckScores(query) => {
                // Get the scores and then do a backfill and retreive them again.
                // This confirms that they are consistent even when they are computed
                // from memory or disk.
                let memory_results = self
                    .query_with_scores(&query, None, SearchVersion::V2)
                    .await?;
                self.backfill().await?;
                let disk_results = self
                    .query_with_scores(&query, None, SearchVersion::V2)
                    .await?;
                assert_query_results_approx_equal(&memory_results, &disk_results);
            },
            TestAction::Compact => {
                self.compact().await?;
            },
        }
        self.database.memory_consistency_check()?;
        Ok(())
    }
}

fn assert_query_results_approx_equal(
    left: &Vec<(ResolvedDocumentId, f64)>,
    right: &Vec<(ResolvedDocumentId, f64)>,
) {
    assert_eq!(left.len(), right.len());
    for ((left_id, left_score), (right_id, right_score)) in left.iter().zip(right.iter()) {
        assert_eq!(left_id, right_id);
        assert_approx_equal(*left_score, *right_score);
    }
}

#[derive(Debug, Arbitrary, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum TestKey {
    A,
    B,
    C,
    D,
}

impl Display for TestKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Arbitrary, Copy, Clone, Eq, Hash, PartialEq, Ord, PartialOrd)]
enum TestValue {
    A,
    B,
    C,
    D,
}

impl Display for TestValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Arbitrary)]
struct TestQuery {
    #[proptest(strategy = "prop::collection::vec(any::<TestValue>(), 0..4)")]
    search: Vec<TestValue>,
    filter: Option<TestValue>,
}

#[derive(Debug, Arbitrary)]
struct TestUpdate {
    key: TestKey,
    search_field: Vec<TestValue>,
    filter_field: TestValue,
}

#[derive(Debug, Arbitrary)]
enum TestAction {
    Backfill,
    Update(TestUpdate),
    Delete(TestKey),
    QueryAndCheckResults(TestQuery),
    QueryAndCheckScores(TestQuery),
    Compact,
}
fn test_search_actions(actions: Vec<TestAction>) {
    let td = TestDriver::new();
    let rt = td.rt();
    let future = async move {
        let mut scenario = Scenario::new(rt).await?;
        for action in actions {
            scenario.execute(action).await?;
        }
        anyhow::Ok(())
    };
    td.run_until(future).unwrap();
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

    /// Randomized search test
    ///
    /// This does a random sequence of updates, deletes, and backfills and checks:
    /// 1. That querying returns the right set of results.
    /// 2. That the scores produced by the in-memory search index match the disk
    /// index scores.
    #[test]
    fn proptest_search_results(actions in prop::collection::vec(any::<TestAction>(), 1..16)) {
        test_search_actions(actions);
    }

    #[test]
    fn proptest_single_query(
        updates in prop::collection::vec(any::<TestUpdate>(), 1..8),
        query in any::<TestQuery>(),
    ) {
        let mut actions: Vec<_> = updates.into_iter().map(TestAction::Update).collect();
        actions.push(TestAction::QueryAndCheckScores(query));
        test_search_actions(actions);
    }
}

/// A non-randomized test to check the BM25 score value.
///
/// Our randomized tests ensure that the scores from the in-memory index
/// match the disk index scores, but don't check the actual score value.
///
/// This makes sure that the scores actually match what BM25 is supposed to
/// produce.
#[convex_macro::test_runtime]
async fn test_search_score(rt: TestRuntime) -> anyhow::Result<()> {
    let mut scenario = Scenario::new(rt).await?;
    scenario
        .patch(TestKey::A, vec![TestValue::A], TestValue::A)
        .await?;
    let query = TestQuery {
        search: vec![TestValue::A],
        filter: None,
    };
    {
        let results = scenario
            .query_with_scores(&query, None, SearchVersion::V1)
            .await?;
        assert_eq!(results.len(), 1);

        assert_approx_equal(results.first().unwrap().1, 0.2876);
    }

    {
        let results = scenario
            .query_with_scores(&query, None, SearchVersion::V2)
            .await?;
        assert_eq!(results.len(), 1);

        assert_approx_equal(results.first().unwrap().1, 0.2876);
    }
    anyhow::Ok(())
}

/// A non-randomized test to test querying our in-memory index at historical
/// timestamps.
///
/// The randomized tests do all the querying at the current time.
#[convex_macro::test_runtime]
async fn test_historical_timestamps(rt: TestRuntime) -> anyhow::Result<()> {
    let mut scenario = Scenario::new(rt).await?;

    let query = TestQuery {
        search: vec![TestValue::A],
        filter: None,
    };

    let ts1 = scenario
        .patch(TestKey::A, vec![TestValue::A], TestValue::A)
        .await?;
    let ts2 = scenario
        .patch(TestKey::B, vec![TestValue::A], TestValue::A)
        .await?;
    let ts3 = scenario
        .patch(TestKey::C, vec![TestValue::A], TestValue::A)
        .await?;
    let ts4 = scenario
        .patch(TestKey::D, vec![TestValue::A], TestValue::A)
        .await?;

    assert_eq!(
        scenario
            .query_with_scores(&query, Some(ts1), SearchVersion::V2)
            .await?
            .len(),
        1
    );
    assert_eq!(
        scenario
            .query_with_scores(&query, Some(ts2), SearchVersion::V2)
            .await?
            .len(),
        2
    );
    assert_eq!(
        scenario
            .query_with_scores(&query, Some(ts3), SearchVersion::V2)
            .await?
            .len(),
        3
    );
    assert_eq!(
        scenario
            .query_with_scores(&query, Some(ts4), SearchVersion::V2)
            .await?
            .len(),
        4
    );
    anyhow::Ok(())
}

/// A test for a query where there is one document in the disk index
/// but it was since deleted.
#[convex_macro::test_runtime]
async fn test_querying_with_zero_documents(rt: TestRuntime) -> anyhow::Result<()> {
    let mut scenario = Scenario::new(rt).await?;
    scenario
        .patch(TestKey::A, vec![TestValue::A], TestValue::A)
        .await?;
    scenario.execute(TestAction::Delete(TestKey::A)).await?;
    scenario
        .execute(TestAction::QueryAndCheckScores(TestQuery {
            search: vec![TestValue::A],
            filter: None,
        }))
        .await?;
    anyhow::Ok(())
}

#[convex_macro::test_runtime]
async fn test_filtering_match(rt: TestRuntime) -> anyhow::Result<()> {
    let mut scenario = Scenario::new(rt).await?;
    scenario
        .patch(TestKey::A, vec![TestValue::A], TestValue::A)
        .await?;
    scenario
        .execute(TestAction::QueryAndCheckScores(TestQuery {
            search: vec![TestValue::A],
            filter: Some(TestValue::A),
        }))
        .await?;
    anyhow::Ok(())
}

#[convex_macro::test_runtime]
async fn test_filtering_no_match(rt: TestRuntime) -> anyhow::Result<()> {
    let mut scenario = Scenario::new(rt).await?;
    scenario
        .patch(TestKey::A, vec![TestValue::A], TestValue::A)
        .await?;
    scenario
        .execute(TestAction::QueryAndCheckScores(TestQuery {
            search: vec![TestValue::A],
            filter: Some(TestValue::B),
        }))
        .await?;
    anyhow::Ok(())
}

#[convex_macro::test_runtime]
async fn test_filtering_match_deleted(rt: TestRuntime) -> anyhow::Result<()> {
    let mut scenario = Scenario::new(rt).await?;
    scenario
        .patch(TestKey::A, vec![TestValue::A], TestValue::A)
        .await?;
    scenario.execute(TestAction::Delete(TestKey::A)).await?;
    scenario
        .execute(TestAction::QueryAndCheckScores(TestQuery {
            search: vec![TestValue::A],
            filter: Some(TestValue::A),
        }))
        .await?;
    anyhow::Ok(())
}

#[convex_macro::test_runtime]
async fn test_filtering_match_updates(rt: TestRuntime) -> anyhow::Result<()> {
    let mut scenario = Scenario::new(rt).await?;
    scenario
        .patch(TestKey::A, vec![TestValue::A], TestValue::A)
        .await?;
    scenario
        .patch(TestKey::A, vec![TestValue::A], TestValue::B)
        .await?;
    scenario
        .execute(TestAction::QueryAndCheckScores(TestQuery {
            search: vec![TestValue::A],
            filter: Some(TestValue::A),
        }))
        .await?;
    anyhow::Ok(())
}

#[convex_macro::test_runtime]
async fn test_bm25_stats_no_underflow(rt: TestRuntime) -> anyhow::Result<()> {
    let mut scenario = Scenario::new(rt).await?;
    scenario
        .patch(TestKey::C, vec![TestValue::D], TestValue::A)
        .await?;
    scenario.execute(TestAction::Backfill).await?;
    scenario.execute(TestAction::Delete(TestKey::C)).await?;
    // This query doens't use the filter field, so the BM25 stats will not include
    // the filter field while the commit statistics will in the memory index from
    // the delete.
    scenario
        .execute(TestAction::QueryAndCheckScores(TestQuery {
            search: vec![TestValue::D],
            filter: None,
        }))
        .await?;
    anyhow::Ok(())
}

// Regression test: We had a bug where we were computing the index of a matching
// union term incorrectly.
//
// This test assigns term ID 0 to a filter term (by creating a document with an
// empty search field first) and then term ID 1 to a term included in the query.
// Then, we perform a matching query, checking that the computed offset for the
// union matching term is in bounds.
#[test]
fn test_union_rank() {
    let actions = vec![
        TestAction::Update(TestUpdate {
            key: TestKey::A,
            search_field: vec![],
            filter_field: TestValue::A,
        }),
        TestAction::Update(TestUpdate {
            key: TestKey::A,
            search_field: vec![TestValue::B],
            filter_field: TestValue::A,
        }),
        TestAction::QueryAndCheckResults(TestQuery {
            search: vec![TestValue::B],
            filter: Some(TestValue::A),
        }),
    ];
    test_search_actions(actions);
}

#[convex_macro::test_runtime]
async fn test_fuzzy_mem(rt: TestRuntime) -> anyhow::Result<()> {
    let mut scenario = Scenario::new(rt).await?;

    scenario._patch("a", "the quick brow fox", "test").await?;
    let results = scenario
        ._query_with_scores("brown", None, None, SearchVersion::V2)
        .await?;
    assert_eq!(results.len(), 1);

    // Exact match w/o prefix will fail
    let results = scenario
        ._query_with_scores("bro test", None, None, SearchVersion::V2)
        .await?;
    assert_eq!(results.len(), 0);

    // Exact match w/ prefix will succeed
    let results = scenario
        ._query_with_scores("bro", None, None, SearchVersion::V2)
        .await?;
    assert_eq!(results.len(), 1);

    // Prefix
    scenario
        ._patch("b", "this is some aghhhhhhhhh... random article", "test")
        .await?;
    let results = scenario
        ._query_with_scores("aghh", None, None, SearchVersion::V2)
        .await?;
    assert_eq!(results.len(), 1);

    // Prefix + fuzzy
    let results = scenario
        ._query_with_scores("ahhhhh", None, None, SearchVersion::V2)
        .await?;
    assert_eq!(results.len(), 1);

    // Edit distance 2
    scenario
        ._patch("c", "my name is bartholomew", "test")
        .await?;
    let results = scenario
        ._query_with_scores("batholmew runs fast", None, None, SearchVersion::V2)
        .await?;
    assert_eq!(results.len(), 1);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_fuzzy_disk(rt: TestRuntime) -> anyhow::Result<()> {
    {
        let mut scenario = Scenario::new(rt).await?;
        scenario._patch("key1", "rakeeb wuz here", "test").await?;
        scenario.backfill().await?;
        scenario
            ._patch("key2", "rakeeb wuz not here", "test")
            .await?;

        let results = scenario
            ._query_with_scores("is rakeem present?", None, None, SearchVersion::V2)
            .await?;
        assert_eq!(results.len(), 2);
    }

    Ok(())
}

// Previous regression
#[convex_macro::test_runtime]
async fn test_fuzzy_disk_snapshot_shortlist_ids_valid_with_empty_memory_index(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut scenario = Scenario::new(rt).await?;
    for i in 0..2048 {
        scenario
            ._patch(format!("{i:}"), "rakeeb wuz here", "test")
            .await?;
    }
    scenario.backfill().await?;

    let results = scenario
        ._query_with_scores("rak", None, None, SearchVersion::V2)
        .await?;
    assert_eq!(results.len(), MAX_CANDIDATE_REVISIONS);
    Ok(())
}

// See https://github.com/get-convex/convex/pull/20649
#[convex_macro::test_runtime]
async fn unrelated_sentences_are_queryable_after_flush(rt: TestRuntime) -> anyhow::Result<()> {
    let mut scenario = Scenario::new(rt).await?;
    scenario._patch("key1", "rakeeb wuz here", "test").await?;
    scenario
        ._patch("key2", "some other sentence", "test")
        .await?;
    scenario.backfill().await?;

    let results = scenario
        ._query_with_scores("rakeem", None, None, SearchVersion::V2)
        .await?;
    assert_eq!(results.len(), 1);

    let results = scenario
        ._query_with_scores("senence", None, None, SearchVersion::V2)
        .await?;
    assert_eq!(results.len(), 1);

    Ok(())
}

#[test]
fn searches_with_duplicate_terms_have_same_memory_disk_score() {
    let action = TestAction::Update(TestUpdate {
        key: TestKey::A,
        search_field: vec![TestValue::D],
        filter_field: TestValue::A,
    });
    let query = TestAction::QueryAndCheckScores(TestQuery {
        search: vec![TestValue::D, TestValue::D],
        filter: None,
    });
    test_search_actions(vec![action, query]);
}

#[convex_macro::test_runtime]
async fn empty_searches_produce_no_results(rt: TestRuntime) -> anyhow::Result<()> {
    let mut scenario = Scenario::new(rt).await?;
    scenario
        ._patch("key1", "rakeeb \t\nwuz here", "test")
        .await?;
    scenario.backfill().await?;
    scenario
        ._patch("key2", "rakeeb     wuz not here", "test")
        .await?;

    for query_string in vec!["", "    ", "\n", "\t"] {
        let results = scenario
            ._query_with_scores(query_string, None, None, SearchVersion::V2)
            .await?;
        assert_eq!(results.len(), 0);
    }
    Ok(())
}

#[convex_macro::test_runtime]
async fn empty_search_works_while_bootstrapping(rt: TestRuntime) -> anyhow::Result<()> {
    let mut scenario = Scenario::new(rt).await?;
    scenario
        ._patch("key1", "rakeeb \t\nwuz here", "test")
        .await?;
    scenario.backfill().await?;
    scenario
        ._patch("key2", "rakeeb     wuz not here", "test")
        .await?;
    scenario.set_bootstrapping().await?;

    for query_string in vec!["", "    ", "\n", "\t"] {
        let results = scenario
            ._query_with_scores(query_string, None, None, SearchVersion::V2)
            .await?;
        assert_eq!(results.len(), 0);
    }
    Ok(())
}

#[convex_macro::test_runtime]
async fn search_fails_while_bootstrapping(rt: TestRuntime) -> anyhow::Result<()> {
    let mut scenario = Scenario::new(rt).await?;
    scenario
        ._patch("key1", "rakeeb \t\nwuz here", "test")
        .await?;
    scenario.backfill().await?;
    scenario.set_bootstrapping().await?;
    let err = scenario
        ._query_with_scores("rakeeb", None, None, SearchVersion::V2)
        .await
        .unwrap_err();
    assert!(err.is_overloaded());

    Ok(())
}

/// Test that search works after bootstrapping has finished when there are
/// writes in between bootstrap ts and the commit ts.
#[convex_macro::test_runtime]
async fn search_works_after_bootstrapping(rt: TestRuntime) -> anyhow::Result<()> {
    let scenario = Scenario::new(rt.clone()).await?;
    let (mut pause_controller, pause_client) =
        PauseController::new(vec![FINISHED_BOOTSTRAP_UPDATES]);
    let mut wait_for_blocked = pause_controller
        .wait_for_blocked(FINISHED_BOOTSTRAP_UPDATES)
        .boxed();
    let bootstrap_fut = scenario
        .database
        .start_search_and_vector_bootstrap(pause_client)
        .into_join_future()
        .fuse();
    pin_mut!(bootstrap_fut);
    select_biased! {
                _ = bootstrap_fut => { panic!("bootstrap completed before pause");},
                pause_guard = wait_for_blocked.as_mut().fuse() => {
                    if let Some(mut pause_guard) = pause_guard {
                        scenario.insert("rakeeb \t\nwuz here", "test").await?;
                        pause_guard.unpause();
                    }
                },
    }
    bootstrap_fut.await?;
    scenario
        ._query_with_scores("rakeeb", None, None, SearchVersion::V2)
        .await?;

    Ok(())
}

struct BrokenSearcher;

#[async_trait]
impl VectorSearcher for BrokenSearcher {
    async fn execute_multi_segment_vector_query(
        &self,
        _: Arc<dyn Storage>,
        _: Vec<FragmentedVectorSegmentPaths>,
        _: QdrantSchema,
        _: CompiledVectorSearch,
        _: u32,
    ) -> anyhow::Result<Vec<VectorSearchQueryResult>> {
        anyhow::bail!("我");
    }

    async fn execute_vector_compaction(
        &self,
        _: Arc<dyn Storage>,
        _: Vec<FragmentedVectorSegmentPaths>,
        _: usize,
    ) -> anyhow::Result<FragmentedVectorSegment> {
        anyhow::bail!("不");
    }
}

#[async_trait]
impl Searcher for BrokenSearcher {
    async fn execute_query(
        &self,
        _: Arc<dyn Storage>,
        _: &ObjectKey,
        _: &TantivySearchIndexSchema,
        _: CompiledQuery,
        _: Bm25StatisticsDiff,
        _: TermShortlist,
        _: usize,
    ) -> anyhow::Result<SearchQueryResult> {
        anyhow::bail!("要");
    }

    async fn query_tokens(
        &self,
        _: Arc<dyn Storage>,
        _: FragmentedTextStorageKeys,
        _: Vec<TokenQuery>,
        _: usize,
    ) -> anyhow::Result<Vec<TokenMatch>> {
        anyhow::bail!("recherche")
    }

    async fn query_bm25_stats(
        &self,
        _: Arc<dyn Storage>,
        _: FragmentedTextStorageKeys,
        _: Vec<Term>,
    ) -> anyhow::Result<Bm25Stats> {
        anyhow::bail!("plein")
    }

    async fn query_posting_lists(
        &self,
        _: Arc<dyn Storage>,
        _: FragmentedTextStorageKeys,
        _: PostingListQuery,
    ) -> anyhow::Result<Vec<PostingListMatch>> {
        anyhow::bail!("texte");
    }

    async fn execute_text_compaction(
        &self,
        _search_storage: Arc<dyn Storage>,
        _segments: Vec<FragmentedTextStorageKeys>,
    ) -> anyhow::Result<FragmentedTextSegment> {
        anyhow::bail!("真不要")
    }
}

#[convex_macro::test_runtime]
async fn empty_searches_with_broken_searcher_return_empty_results(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut scenario = Scenario::new_with_searcher(rt, BrokenSearcher).await?;
    scenario._patch("key1", "rakeeb wuz here", "test").await?;
    scenario.backfill().await?;
    scenario
        ._patch("key2", "rakeeb wuz not here", "test")
        .await?;

    for query_string in vec!["", "    ", "\n", "\t"] {
        let results = scenario
            ._query_with_scores(query_string, None, None, SearchVersion::V2)
            .await?;
        assert_eq!(results.len(), 0);
    }
    Ok(())
}

#[test]
fn test_scores_when_some_but_not_all_term_values_for_a_field_are_deleted() -> anyhow::Result<()> {
    // This is taken directly from a proptest failure. We could simplify it further,
    // but it catches a reasonably small error as is. The main thing to note is
    // that the first Update action adds Document A, which is then replaced by
    // the second update action, changing the term frequency. If the test fails,
    // it means the memory and disk are index are not keeping the same statistics
    // during the replace operation, or are not scoring the same based on the same
    // statistics.
    let actions = vec![
        // 9 As, 2 Ds
        TestAction::Update(TestUpdate {
            key: TestKey::A,
            search_field: vec![
                TestValue::A,
                TestValue::A,
                TestValue::A,
                TestValue::A,
                TestValue::A,
                TestValue::A,
                TestValue::D,
                TestValue::A,
                TestValue::D,
                TestValue::A,
                TestValue::A,
            ],
            filter_field: TestValue::A,
        }),
        TestAction::QueryAndCheckScores(TestQuery {
            search: vec![],
            filter: None,
        }),
        // 4 As, 4 Cs
        TestAction::Update(TestUpdate {
            key: TestKey::A,
            search_field: vec![
                TestValue::A,
                TestValue::C,
                TestValue::A,
                TestValue::C,
                TestValue::C,
                TestValue::A,
                TestValue::A,
                TestValue::C,
            ],
            filter_field: TestValue::A,
        }),
        // 6 As, 3 Bs, 2Cs, 8Ds
        TestAction::Update(TestUpdate {
            key: TestKey::B,
            search_field: vec![
                TestValue::C,
                TestValue::D,
                TestValue::D,
                TestValue::D,
                TestValue::C,
                TestValue::A,
                TestValue::A,
                TestValue::B,
                TestValue::D,
                TestValue::D,
                TestValue::C,
                TestValue::D,
                TestValue::D,
                TestValue::A,
                TestValue::A,
                TestValue::A,
                TestValue::B,
                TestValue::B,
                TestValue::D,
                TestValue::A,
                TestValue::B,
            ],
            filter_field: TestValue::C,
        }),
        TestAction::QueryAndCheckScores(TestQuery {
            search: vec![TestValue::C],
            filter: Some(TestValue::C),
        }),
    ];
    test_search_actions(actions);
    Ok(())
}

/// Generates ASCII alphanumeric strings of length 1..32
fn tokenizable_string_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9]{1,32}").unwrap()
}

fn generate_perturbed_query(query: Vec<String>) -> impl Strategy<Value = Vec<String>> {
    Just(query).prop_shuffle()
}

fn generate_document(
    query: Vec<String>,
    fluff: Range<usize>,
) -> impl Strategy<Value = Vec<String>> {
    (
        prop::collection::vec(tokenizable_string_strategy(), fluff),
        generate_perturbed_query(query),
    )
        .prop_map(|(fluff, perturbed_query)| {
            let mut res = Vec::with_capacity(perturbed_query.len() * fluff.len());
            if let Some(q) = perturbed_query.first() {
                res.push(q.clone());
            }

            for q in perturbed_query.into_iter().skip(1) {
                res.extend_from_slice(&fluff);
                res.push(q.clone());
            }
            res
        })
}

#[derive(Debug, Clone)]
struct FuzzyDeterminismTestCase {
    // Let N be the length of `query`
    // Each document is a list of N-1 sets of fluff terms, a permutation of `query`, a list of
    // operations to apply to the query terms.
    documents: Vec<Vec<String>>,
    query: Vec<String>,
}

struct FuzzyDeterminismArbitraryParams {
    query_size: Range<usize>,
    num_docs: Range<usize>,
    fluff_len: Range<usize>,
}

impl Default for FuzzyDeterminismArbitraryParams {
    fn default() -> Self {
        Self {
            query_size: 2..8,
            // TODO: Since we don't have a deterministic order when querying within a Tantivy
            // segment, we have to query less than 1024 results here.
            num_docs: 2..1023,
            fluff_len: 0..16,
        }
    }
}

impl Arbitrary for FuzzyDeterminismTestCase {
    type Parameters = FuzzyDeterminismArbitraryParams;

    type Strategy = impl Strategy<Value = FuzzyDeterminismTestCase>;

    fn arbitrary_with(
        FuzzyDeterminismArbitraryParams {
            query_size,
            num_docs,
            fluff_len,
        }: Self::Parameters,
    ) -> Self::Strategy {
        // First, generate a query
        prop::collection::vec(tokenizable_string_strategy(), query_size)
            .prop_flat_map(move |query| {
                // Generate up to num_docs documents
                (
                    Just(query.clone()),
                    prop::collection::vec(
                        generate_document(query.clone(), fluff_len.clone()),
                        num_docs.clone(),
                    ),
                )
            })
            .prop_map(|(query, documents)| FuzzyDeterminismTestCase { documents, query })
            .no_shrink()
    }
}

async fn do_search_with_backfill_split(
    mut scenario: Scenario,
    test_case: FuzzyDeterminismTestCase,
    split: usize,
) -> anyhow::Result<Vec<(usize, f64)>> {
    let mut index_by_document_id = BTreeMap::new();
    for (i, doc) in test_case.documents.into_iter().enumerate() {
        if i == split {
            scenario.backfill().await?;
        }
        let key = format!("{i}");
        let doc = doc.join(" ");
        let (document_id, _) = scenario._patch(key, doc, "test").await?;
        index_by_document_id.insert(document_id, i);
    }
    let query = test_case.query.join(" ");
    let results = scenario
        ._query_with_scores(query, None, None, SearchVersion::V2)
        .await?
        .into_iter()
        .map(|(document_id, score)| (index_by_document_id[&document_id], score))
        .collect();
    Ok(results)
}

fn do_search_for_fraction(test_case: FuzzyDeterminismTestCase, num_splits: usize) {
    let td = TestDriver::new();
    let rt = td.rt();
    let future = async move {
        let mut last_result: Option<BTreeSet<(usize, TotalOrdF64)>> = None;
        for split in 0..num_splits {
            let split_idx = (test_case.documents.len() * split) / num_splits;
            let scenario = Scenario::new(rt.clone()).await?;
            let result = do_search_with_backfill_split(scenario, test_case.clone(), split_idx)
                .await?
                .into_iter()
                .map(|(i, score)| (i, TotalOrdF64::from(score)))
                .collect::<BTreeSet<_>>();
            if let Some(last_result) = last_result {
                if result != last_result {
                    let mut msg = format!(
                        "Results differ when doing {} vs. {} splits:",
                        num_splits - 1,
                        num_splits
                    );
                    for added in result.difference(&last_result) {
                        msg.push_str(&format!("\n  added: {added:?}"));
                    }
                    for removed in last_result.difference(&result) {
                        msg.push_str(&format!("\n  removed: {removed:?}"));
                    }
                    panic!("{msg}");
                }
            }
            last_result = Some(result);
        }
        anyhow::Ok(())
    };
    td.run_until(future).unwrap();
}

proptest! {
    // Increase number of cases being run with CONVEX_PROPTEST_MULTIPLIER to test query determinism
    // more rigorously.
    #![proptest_config(ProptestConfig { cases: env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

    #[test]
    fn proptest_mem_disk_determinism(
        test_case in any::<FuzzyDeterminismTestCase>(),
        num_splits in 2_usize..=4_usize,
    ) {
        do_search_for_fraction(test_case, num_splits);
    }
}
