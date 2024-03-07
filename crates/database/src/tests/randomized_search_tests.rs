use std::{
    collections::{
        btree_map::Entry,
        BTreeMap,
        BTreeSet,
    },
    ops::Range,
    sync::Arc,
};

use cmd_util::env::env_config;
use common::{
    bootstrap_model::index::IndexMetadata,
    floating_point::assert_approx_equal,
    query::{
        CursorPosition,
        Query,
        QueryOperator,
        QuerySource,
        Search,
        SearchFilterExpression,
        SearchVersion,
    },
    types::{
        IndexName,
        TabletIndexName,
        Timestamp,
    },
    value::{
        sorting::sorting_decode::bytes_to_values,
        ConvexValue,
        ResolvedDocumentId,
        TableName,
    },
    version::MIN_NPM_VERSION_FOR_FUZZY_SEARCH,
};
use futures::{
    future::BoxFuture,
    FutureExt,
};
use keybroker::Identity;
use maplit::btreeset;
use must_let::must_let;
use proptest::prelude::*;
use proptest_derive::Arbitrary;
use runtime::testing::{
    TestDriver,
    TestRuntime,
};
use search::{
    searcher::InProcessSearcher,
    MAX_CANDIDATE_REVISIONS,
};
use storage::Storage;
use usage_tracking::FunctionUsageTracker;
use value::assert_obj;

use crate::{
    test_helpers::{
        DbFixtures,
        DbFixturesArgs,
    },
    Database,
    IndexModel,
    ResolvedQuery,
    SearchIndexFlusher,
    TableModel,
};

struct Scenario {
    rt: TestRuntime,
    database: Database<TestRuntime>,

    search_storage: Arc<dyn Storage>,

    table_name: TableName,

    // Store a simple mapping of a test string to an array of test
    // strings (the search field) and a filter field
    model: BTreeMap<String, (ResolvedDocumentId, String, String)>,
}

impl Scenario {
    async fn new(rt: TestRuntime) -> anyhow::Result<Self> {
        let DbFixtures {
            db: database,
            search_storage,
            ..
        } = DbFixtures::new_with_args(
            &rt,
            DbFixturesArgs {
                searcher: Some(Arc::new(InProcessSearcher::new(rt.clone()).await?)),
                ..Default::default()
            },
        )
        .await?;

        let table_name: TableName = "test".parse()?;
        let mut tx = database.begin(Identity::system()).await?;
        TableModel::new(&mut tx)
            .insert_table_metadata_for_test(&table_name)
            .await?;
        let index = IndexMetadata::new_backfilling_search_index(
            "test.by_text".parse()?,
            "searchField".parse()?,
            btreeset! {"filterField".parse()?},
        );
        IndexModel::new(&mut tx)
            .add_application_index(index)
            .await?;
        database.commit(tx).await?;

        let mut self_ = Self {
            rt,
            database,
            search_storage,

            table_name,
            model: BTreeMap::new(),
        };
        self_.backfill().await?;
        self_.enable_index().await?;
        Ok(self_)
    }

    async fn backfill(&mut self) -> anyhow::Result<()> {
        let snapshot = self.database.latest_snapshot()?;
        let table_id = snapshot.table_mapping().id(&"test".parse()?)?.table_id;
        SearchIndexFlusher::build_index_in_test(
            TabletIndexName::new(table_id, "by_text".parse()?)?,
            "test".parse()?,
            self.rt.clone(),
            self.database.clone(),
            self.search_storage.clone(),
        )
        .await?;

        Ok(())
    }

    async fn enable_index(&mut self) -> anyhow::Result<()> {
        let mut txn = self.database.begin_system().await?;
        IndexModel::new(&mut txn)
            .enable_index_for_testing(&IndexName::new("test".parse()?, "by_text".parse()?)?)
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
            SearchVersion::V1 => ResolvedQuery::new(&mut tx, query)?,
            SearchVersion::V2 => ResolvedQuery::new_with_version(
                &mut tx,
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
            returned.push((*value.id(), -negative_score))
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
        let text = search_field
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let filter_field = format!("{filter_field:?}");
        self._patch(key.to_string(), text, filter_field).boxed()
    }

    async fn _patch<K: Into<String>, S: Into<String>, F: Into<String>>(
        &mut self,
        key: K,
        search_field: S,
        filter_field: F,
    ) -> anyhow::Result<Timestamp> {
        let key = key.into();
        let search_field = search_field.into();
        let filter_field = filter_field.into();
        let mut tx = self.database.begin(Identity::system()).await?;
        let new_document = assert_obj!("searchField" => search_field.clone(), "filterField" => filter_field.clone());
        match self.model.entry(key) {
            Entry::Vacant(e) => {
                let document_id = tx.insert_for_test(&self.table_name, new_document).await?;
                e.insert((document_id, search_field, filter_field));
            },
            Entry::Occupied(mut e) => {
                let (document_id, ..) = e.get();
                tx.patch_user_facing((*document_id).into(), new_document.into())
                    .await?;
                e.get_mut().1 = search_field;
                e.get_mut().2 = filter_field;
            },
        }
        self.database.commit(tx).await
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
                    tx.delete_user_facing(id.into()).await?;
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

impl ToString for TestKey {
    fn to_string(&self) -> String {
        format!("{self:?}")
    }
}

#[derive(Debug, Arbitrary, Copy, Clone, Eq, Hash, PartialEq, Ord, PartialOrd)]
enum TestValue {
    A,
    B,
    C,
    D,
}

impl ToString for TestValue {
    fn to_string(&self) -> String {
        format!("{self:?}")
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
}
fn test_search_actions(actions: Vec<TestAction>) {
    let mut td = TestDriver::new();
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

        // Constant taken from https://github.com/quickwit-oss/tantivy/blob/main/src/query/term_query/mod.rs#L20
        assert_approx_equal(results.first().unwrap().1, 1.);
    }
    {
        let results = scenario
            .query_with_scores(&query, None, SearchVersion::V2)
            .await?;
        assert_eq!(results.len(), 1);
        assert_approx_equal(results.first().unwrap().1, 1.);
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
            num_docs: 128..2048,
            fluff_len: 8..16,
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
) -> anyhow::Result<Vec<(ResolvedDocumentId, f64)>> {
    let docs = test_case.documents;

    for (i, doc) in docs.into_iter().enumerate() {
        if i == split {
            scenario.backfill().await?;
        }
        let doc = doc.join(" ");
        scenario._patch(format!("{i:}"), doc, "test").await?;
    }

    let query = test_case.query.join(" ");
    let results = scenario
        ._query_with_scores(query, None, None, SearchVersion::V2)
        .await?;

    Ok(results)
}

fn do_search_for_fraction(test_case: FuzzyDeterminismTestCase, num_splits: usize) {
    let mut td = TestDriver::new();
    let rt = td.rt();
    let future = async move {
        let mut last_result: Option<Vec<(ResolvedDocumentId, f64)>> = None;
        for split in 0..num_splits {
            let split_idx = (test_case.documents.len() * split) / num_splits;
            let scenario = Scenario::new(rt.clone()).await?;
            let result =
                do_search_with_backfill_split(scenario, test_case.clone(), split_idx).await?;
            if let Some(last_result) = last_result {
                assert_eq!(result.len(), last_result.len());
                for i in 0..result.len() {
                    assert_approx_equal(result[i].1, last_result[i].1);
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
