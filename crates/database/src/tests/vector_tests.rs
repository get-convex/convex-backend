use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    sync::Arc,
};

use cmd_util::env::env_config;
use common::{
    bootstrap_model::index::{
        vector_index::{
            VectorIndexBackfillState,
            VectorIndexSnapshot,
            VectorIndexSnapshotData,
            VectorIndexSpec,
            VectorIndexState,
        },
        IndexConfig,
        IndexMetadata,
    },
    components::ComponentId,
    knobs::{
        MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
        VECTOR_INDEX_SIZE_SOFT_LIMIT,
    },
    runtime::Runtime,
    types::{
        unchecked_repeatable_ts,
        IndexDescriptor,
        IndexName,
    },
};
use itertools::Itertools;
use keybroker::Identity;
use maplit::{
    btreemap,
    btreeset,
};
use must_let::must_let;
use proptest::prelude::*;
use proptest_derive::Arbitrary;
use qdrant_segment::types::VECTOR_ELEMENT_SIZE;
use runtime::{
    prod::ProdRuntime,
    testing::{
        TestDriver,
        TestRuntime,
    },
};
use search::searcher::{
    InProcessSearcher,
    Searcher,
};
use storage::Storage;
use value::{
    assert_obj,
    ConvexObject,
    ConvexValue,
    DeveloperDocumentId,
    TableName,
    TableNamespace,
};
use vector::{
    cosine_similarity,
    PublicVectorSearchQueryResult,
    VectorSearch,
    VectorSearchExpression,
};

use crate::{
    search_index_workers::FlusherType,
    test_helpers::{
        vector_utils::{
            random_vector,
            random_vector_value,
            vector_to_value,
            DIMENSIONS,
        },
        DbFixtures,
        DbFixturesArgs,
    },
    tests::vector_test_utils::{
        IndexData,
        VectorFixtures,
    },
    vector_index_worker::{
        compactor::compact_vector_indexes_in_test,
        flusher::{
            backfill_vector_indexes,
            new_vector_flusher_for_tests,
        },
    },
    Database,
    IndexModel,
    TableModel,
    UserFacingModel,
    VectorIndexFlusher,
};

const TABLE_NAME: &str = "test";
const INDEX_DESCRIPTOR: &str = "by_embedding";
const INDEX_NAME: &str = "test.by_embedding";
const INDEXED_FIELD: &str = "embedding";
const FILTER_FIELDS: &[&str] = &["A", "B", "C", "D"];
const TABLE_NAMESPACE: TableNamespace = TableNamespace::test_user();

struct Scenario<RT: Runtime> {
    rt: RT,
    database: Database<RT>,
    search_storage: Arc<dyn Storage>,
    searcher: Arc<dyn Searcher>,
}

impl<RT: Runtime> Scenario<RT> {
    async fn new(rt: RT) -> anyhow::Result<Self> {
        let DbFixtures {
            db,
            searcher,
            search_storage,
            ..
        } = DbFixtures::new_with_args(
            &rt,
            DbFixturesArgs {
                searcher: Some(Arc::new(InProcessSearcher::new(rt.clone())?)),
                ..Default::default()
            },
        )
        .await?;
        let handle = db.start_search_and_vector_bootstrap();
        handle.join().await?;

        let self_ = Self {
            rt,
            database: db,
            search_storage,
            searcher,
        };

        Ok(self_)
    }

    async fn new_with_enabled_index(rt: RT) -> anyhow::Result<Self> {
        let self_ = Scenario::new(rt).await?;
        self_.add_vector_index(true).await?;
        Ok(self_)
    }

    fn new_backfill_flusher(&self, incremental_index_size: usize) -> VectorIndexFlusher<RT> {
        new_vector_flusher_for_tests(
            self.rt.clone(),
            self.database.clone(),
            self.search_storage.clone(),
            *VECTOR_INDEX_SIZE_SOFT_LIMIT,
            *MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
            incremental_index_size,
            FlusherType::Backfill,
        )
    }

    async fn add_vector_index(&self, should_backfill: bool) -> anyhow::Result<()> {
        let table_name: TableName = TABLE_NAME.parse()?;
        let mut tx = self.database.begin(Identity::system()).await?;
        let namespace = TableNamespace::test_user();
        TableModel::new(&mut tx)
            .insert_table_metadata_for_test(namespace, &table_name)
            .await?;
        let index = IndexMetadata::new_backfilling_vector_index(
            INDEX_NAME.parse()?,
            INDEXED_FIELD.parse()?,
            DIMENSIONS.try_into()?,
            FILTER_FIELDS.iter().map(|f| f.parse()).try_collect()?,
        );
        IndexModel::new(&mut tx)
            .add_application_index(namespace, index)
            .await?;
        self.database.commit(tx).await?;

        if should_backfill {
            self.backfill().await?;
            self.enable_index().await?;
        }
        Ok(())
    }

    async fn enable_index(&self) -> anyhow::Result<()> {
        let mut tx = self.database.begin_system().await?;
        IndexModel::new(&mut tx)
            .enable_index_for_testing(
                TABLE_NAMESPACE,
                &IndexName::new(TABLE_NAME.parse()?, IndexDescriptor::new(INDEX_DESCRIPTOR)?)?,
            )
            .await?;
        self.database.commit(tx).await?;
        Ok(())
    }

    async fn seed_table_with_vector_data(
        &self,
        num_docs: u32,
    ) -> anyhow::Result<Vec<DeveloperDocumentId>> {
        let mut ids = vec![];
        for _ in 0..num_docs {
            let mut tx = self.database.begin(Identity::system()).await?;
            let vector = random_vector_value(&mut self.rt.rng());
            let obj = assert_obj!(INDEXED_FIELD => vector, "A" => ConvexValue::Int64(1017));
            let id = UserFacingModel::new_root_for_test(&mut tx)
                .insert(TABLE_NAME.parse()?, obj)
                .await?;
            ids.push(id);
            self.database.commit(tx).await?;
        }
        Ok(ids)
    }

    async fn compact(&mut self) -> anyhow::Result<()> {
        compact_vector_indexes_in_test(
            self.rt.clone(),
            self.database.clone(),
            self.search_storage.clone(),
            self.searcher.clone(),
        )
        .await
    }

    async fn backfill(&self) -> anyhow::Result<()> {
        backfill_vector_indexes(
            self.rt.clone(),
            self.database.clone(),
            self.search_storage.clone(),
        )
        .await?;
        Ok(())
    }

    async fn search(
        &self,
        vector: Vec<f32>,
        filter_expressions: BTreeSet<VectorSearchExpression>,
    ) -> anyhow::Result<Vec<PublicVectorSearchQueryResult>> {
        self.search_with_limit(vector, filter_expressions, None)
            .await
    }

    async fn search_with_limit(
        &self,
        vector: Vec<f32>,
        filter_expressions: BTreeSet<VectorSearchExpression>,
        limit: Option<u32>,
    ) -> anyhow::Result<Vec<PublicVectorSearchQueryResult>> {
        let (results, _usage_stats) = self
            .database
            .vector_search(
                Identity::system(),
                VectorSearch {
                    index_name: INDEX_NAME.parse()?,
                    component_id: ComponentId::Root,
                    vector,
                    limit,
                    expressions: filter_expressions,
                },
            )
            .await?;
        Ok(results)
    }

    pub async fn get_vector_index_configs(
        &self,
    ) -> anyhow::Result<Vec<(VectorIndexSpec, VectorIndexState)>> {
        let mut tx = self.database.begin_system().await?;
        let mut model = IndexModel::new(&mut tx);
        Ok(model
            .get_all_indexes()
            .await?
            .into_iter()
            .filter_map(|idx| {
                if let IndexConfig::Vector {
                    spec,
                    on_disk_state,
                } = idx.config.clone()
                {
                    Some((spec, on_disk_state))
                } else {
                    None
                }
            })
            .collect_vec())
    }
}

#[derive(Debug, Arbitrary, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct TestKey {
    // At most 8 vectors in our test.
    #[proptest(strategy = "0..8u32")]
    number: u32,
}

#[derive(Debug, Arbitrary, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum FilterKey {
    A,
    B,
    C,
    D,
}

#[derive(Debug, Arbitrary, Copy, Clone, Eq, Hash, PartialEq, Ord, PartialOrd)]
enum FilterValue {
    A,
    B,
    C,
    D,
}

fn vector_strategy() -> impl Strategy<Value = Vec<f32>> {
    // Uniformly sample points in [-1, 1]^4, excluding a small neighborhood of the
    // origin, and normalize to a unit vector.
    let range = prop_oneof![-1.0f32..-1e-4, 1e-4..1.0f32];
    prop::collection::vec(range, DIMENSIONS as usize).prop_map(|v| {
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        v.into_iter().map(|x| x / norm).collect()
    })
}

#[derive(Debug, Arbitrary)]
struct TestUpdate {
    key: TestKey,

    #[proptest(strategy = "vector_strategy()")]
    vector: Vec<f32>,

    #[proptest(
        strategy = "prop::collection::btree_map(any::<FilterKey>(), any::<FilterValue>(), 0..=4)"
    )]
    filter_values: BTreeMap<FilterKey, FilterValue>,
}

#[derive(Debug, Arbitrary)]
struct TestQuery {
    #[proptest(strategy = "vector_strategy()")]
    vector: Vec<f32>,
    #[proptest(strategy = "prop::collection::btree_map(any::<FilterKey>(), \
                           prop::collection::btree_set(any::<Option<FilterValue>>(), 1..=4), \
                           1..=4)")]
    filter: BTreeMap<FilterKey, BTreeSet<Option<FilterValue>>>,
    #[proptest(strategy = "1..16u32")]
    limit: u32,
}

#[derive(Debug, Arbitrary)]
enum TestAction {
    Backfill,
    Update(TestUpdate),
    Delete(TestKey),
    QueryAndCheckResults(TestQuery),
}

struct RandomizedTest<RT: Runtime> {
    scenario: Scenario<RT>,
    model: BTreeMap<TestKey, (DeveloperDocumentId, TestUpdate)>,
}

impl<RT: Runtime> RandomizedTest<RT> {
    async fn new(rt: RT) -> anyhow::Result<Self> {
        Ok(Self {
            scenario: Scenario::new_with_enabled_index(rt).await?,
            model: BTreeMap::new(),
        })
    }

    async fn execute(&mut self, action: TestAction) -> anyhow::Result<()> {
        match action {
            TestAction::Backfill => self.scenario.backfill().await?,
            TestAction::Update(update) => {
                let mut tx = self.scenario.database.begin_system().await?;
                let mut new_obj = BTreeMap::new();
                new_obj.insert(
                    INDEXED_FIELD.parse()?,
                    vector_to_value(update.vector.clone()),
                );
                for (key, value) in &update.filter_values {
                    new_obj.insert(
                        format!("{key:?}").parse()?,
                        ConvexValue::String(format!("{value:?}").try_into()?),
                    );
                }
                let new_obj = ConvexObject::try_from(new_obj)?;
                let id = if let Some((document_id, _)) = self.model.remove(&update.key) {
                    UserFacingModel::new_root_for_test(&mut tx)
                        .replace(document_id, new_obj)
                        .await?;
                    document_id
                } else {
                    UserFacingModel::new_root_for_test(&mut tx)
                        .insert(TABLE_NAME.parse()?, new_obj)
                        .await?
                };
                self.scenario.database.commit(tx).await?;
                self.model.insert(update.key, (id, update));
            },
            TestAction::Delete(key) => {
                if let Some((document_id, _)) = self.model.remove(&key) {
                    let mut tx = self.scenario.database.begin_system().await?;
                    UserFacingModel::new_root_for_test(&mut tx)
                        .delete(document_id)
                        .await?;
                    self.scenario.database.commit(tx).await?;
                }
            },
            TestAction::QueryAndCheckResults(test_query) => {
                let mut expressions = btreeset![];
                for (key, maybe_values) in test_query.filter.clone() {
                    let values: BTreeSet<_> = maybe_values
                        .into_iter()
                        .map(|value| {
                            let value = value
                                .map(|value| {
                                    anyhow::Ok(ConvexValue::String(
                                        format!("{value:?}").try_into()?,
                                    ))
                                })
                                .transpose()?;
                            anyhow::Ok(value)
                        })
                        .try_collect()?;
                    let expression = if values.len() == 1 {
                        let value = values.into_iter().next().unwrap();
                        VectorSearchExpression::Eq(format!("{key:?}").parse()?, value)
                    } else {
                        VectorSearchExpression::In(format!("{key:?}").parse()?, values)
                    };
                    expressions.insert(expression);
                }
                let query = VectorSearch {
                    index_name: INDEX_NAME.parse()?,
                    component_id: ComponentId::Root,
                    vector: test_query.vector.clone(),
                    limit: Some(test_query.limit),
                    expressions,
                };
                let (returned_results, _usage_stats) = self
                    .scenario
                    .database
                    .vector_search(Identity::system(), query)
                    .await?;

                let mut expected_results = vec![];
                for (id, update) in self.model.values() {
                    let mut matches_filter = false;
                    for (key, maybe_values) in &test_query.filter {
                        if maybe_values
                            .iter()
                            .any(|value| update.filter_values.get(key) == value.as_ref())
                        {
                            matches_filter = true;
                            break;
                        }
                    }
                    if !matches_filter {
                        continue;
                    }
                    let score = cosine_similarity(&test_query.vector, &update.vector);
                    expected_results.push(PublicVectorSearchQueryResult { id: *id, score });
                }
                expected_results.sort_by(|a, b| a.cmp(b).reverse());
                expected_results.truncate(test_query.limit as usize);

                assert_eq!(returned_results, expected_results);
            },
        }
        Ok(())
    }
}

#[convex_macro::test_runtime]

async fn test_vector_search(rt: TestRuntime) -> anyhow::Result<()> {
    let scenario = Scenario::new_with_enabled_index(rt.clone()).await?;

    let mut tx = scenario.database.begin(Identity::system()).await?;

    let vector1 = random_vector_value(&mut rt.rng());
    let obj = assert_obj!(INDEXED_FIELD => vector1, "A" => ConvexValue::Int64(1017));
    let id1 = UserFacingModel::new_root_for_test(&mut tx)
        .insert(TABLE_NAME.parse()?, obj)
        .await?;

    let vector2 = random_vector_value(&mut rt.rng());
    let obj = assert_obj!(INDEXED_FIELD => vector2);
    let id2 = UserFacingModel::new_root_for_test(&mut tx)
        .insert(TABLE_NAME.parse()?, obj.clone())
        .await?;

    scenario.database.commit(tx).await?;

    for _ in 0..2 {
        // Check that no filter returns both results.
        let results = scenario.search(vec![0.; 4], btreeset![]).await?;
        assert_eq!(results.len(), 2);

        // Check that filtering for just 1017 gets only the first vector.
        let match_first = VectorSearchExpression::Eq("A".parse()?, Some(ConvexValue::Int64(1017)));
        let results = scenario.search(vec![0.; 4], btreeset![match_first]).await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id.internal_id(), id1.internal_id());

        // Check that filtering for a nonexistent field only gets the second vector.
        let match_second = VectorSearchExpression::Eq("A".parse()?, None);
        let results = scenario
            .search(vec![0.; 4], btreeset![match_second])
            .await?;
        assert_eq!(results[0].id.internal_id(), id2.internal_id());

        // Check that filtering for a field in neither vector returns zero results.
        let match_neither =
            VectorSearchExpression::Eq("A".parse()?, Some(ConvexValue::Int64(1018)));
        let results = scenario
            .search(vec![0.; 4], btreeset![match_neither])
            .await?;
        assert_eq!(results.len(), 0);

        // Backfill and repeat once to check the disk index.
        scenario.backfill().await?;
    }

    // Test deleting the first vector and checking that it doesn't show up in
    // results.
    let mut tx = scenario.database.begin(Identity::system()).await?;
    UserFacingModel::new_root_for_test(&mut tx)
        .delete(id1)
        .await?;
    scenario.database.commit(tx).await?;

    for _ in 0..2 {
        let results = scenario.search(vec![0.; 4], btreeset![]).await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id.internal_id(), id2.internal_id());

        scenario.backfill().await?;
    }

    // Test updating the second vector and checking that it shows up once.
    let mut tx = scenario.database.begin(Identity::system()).await?;
    UserFacingModel::new_root_for_test(&mut tx)
        .replace(id2, obj.clone())
        .await?;
    scenario.database.commit(tx).await?;

    for _ in 0..2 {
        let results = scenario.search(vec![0.; 4], btreeset![]).await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id.internal_id(), id2.internal_id());

        scenario.backfill().await?;
    }

    // Test updating the second vector a second time and checking that it still
    // shows up once.
    let mut tx = scenario.database.begin(Identity::system()).await?;
    UserFacingModel::new_root_for_test(&mut tx)
        .replace(id2, obj)
        .await?;
    scenario.database.commit(tx).await?;

    for _ in 0..2 {
        let results = scenario.search(vec![0.; 4], btreeset![]).await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id.internal_id(), id2.internal_id());

        scenario.backfill().await?;
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_vector_search_compaction(rt: TestRuntime) -> anyhow::Result<()> {
    let mut scenario = Scenario::new_with_enabled_index(rt.clone()).await?;

    let mut ids = vec![];

    let mut rng = rt.rng();

    // Then compact the new round of segments and the original one into a single
    // large segment.
    for _ in 0..3 {
        // Create 3 segments and compact them into one segment
        for _ in 0..3 {
            let mut tx = scenario.database.begin(Identity::system()).await?;
            let vector = random_vector_value(&mut rng);
            let obj = assert_obj!(INDEXED_FIELD => vector, "A" => ConvexValue::Int64(1017));
            let id = UserFacingModel::new_root_for_test(&mut tx)
                .insert(TABLE_NAME.parse()?, obj)
                .await?;
            ids.push(id);
            scenario.database.commit(tx).await?;
            // Backfill to create a new segment
            scenario.backfill().await?;
        }
        scenario.compact().await?;

        let results = scenario.search(vec![0.; 4], btreeset![]).await?;
        assert_eq!(
            results
                .into_iter()
                .map(|result| result.id.internal_id())
                .sorted()
                .collect::<Vec<_>>(),
            ids.iter()
                .map(|id| id.internal_id())
                .sorted()
                .collect::<Vec<_>>()
        );
    }
    scenario.compact().await?;
    let results = scenario.search(vec![0.; 4], btreeset![]).await?;
    assert_eq!(
        results
            .into_iter()
            .map(|result| result.id.internal_id())
            .sorted()
            .collect::<Vec<_>>(),
        ids.iter()
            .map(|id| id.internal_id())
            .sorted()
            .collect::<Vec<_>>()
    );

    Ok(())
}

/// This test will fail flakily if we do not handle MVCC correctly on
/// searchlight. That's reasonably likely because we're downloading caching and
/// re-using some immutable files across different versions of indexes.
#[ignore] // TODO(CX-5143): Re-enable this test after fixing the flake.
#[convex_macro::prod_rt_test]
async fn test_concurrent_index_version_searches(rt: ProdRuntime) -> anyhow::Result<()> {
    let scenario = Arc::new(Scenario::new_with_enabled_index(rt.clone()).await?);

    let mut ids = vec![];
    let mut tx = scenario.database.begin(Identity::system()).await?;
    // Create a segment with N vectors
    for _ in 0..4 {
        let vector = random_vector_value(&mut rt.rng());
        let obj = assert_obj!(INDEXED_FIELD => vector, "A" => ConvexValue::Int64(1017));
        let id = UserFacingModel::new_root_for_test(&mut tx)
            .insert(TABLE_NAME.parse()?, obj)
            .await?;
        ids.push(id);
    }
    scenario.database.commit(tx).await?;
    scenario.backfill().await?;

    // Create N different versions of the index metadata where segment is
    // unmodified, but each one has a different deleted bitset.
    let mut timestamps_and_results = vec![];
    for (index, id) in ids.iter().rev().enumerate() {
        let mut tx = scenario.database.begin(Identity::system()).await?;
        UserFacingModel::new_root_for_test(&mut tx)
            .delete(*id)
            .await?;
        let timestamp = scenario.database.commit(tx).await?;
        timestamps_and_results.push((
            timestamp,
            ids.clone()
                .into_iter()
                .take(ids.len() - 1 - index)
                .collect::<Vec<_>>(),
        ));
        scenario.backfill().await?;
    }

    // Query all of the different versions of the index concurrently to make
    // sure we can handle loading and searching a single segment file with
    // different bitsets.
    let mut handles = vec![];
    for (timestamp, expected_results) in timestamps_and_results {
        let scenario = scenario.clone();
        handles.push(rt.spawn_thread("vector", move || async move {
            let (actual_results, _usage_stats) = scenario
                .database
                .vector_search_at_ts(
                    VectorSearch {
                        index_name: INDEX_NAME.parse().unwrap(),
                        component_id: ComponentId::Root,
                        limit: Some(10),
                        vector: vec![0.; 4],
                        expressions: btreeset![],
                    },
                    unchecked_repeatable_ts(timestamp),
                )
                .await
                .unwrap();
            assert_eq!(
                actual_results
                    .into_iter()
                    .map(|result| result.id.internal_id())
                    .sorted()
                    .collect_vec(),
                expected_results
                    .into_iter()
                    .map(|id| id.internal_id())
                    .sorted()
                    .collect_vec()
            );
        }))
    }
    for handle in handles {
        handle.join().await?;
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_vector_search_compaction_with_deletes(rt: TestRuntime) -> anyhow::Result<()> {
    let mut scenario = Scenario::new_with_enabled_index(rt.clone()).await?;

    let mut ids = vec![];

    // Create 3 segments and compact them into one segment
    for _ in 0..3 {
        let mut tx = scenario.database.begin(Identity::system()).await?;
        let vector = random_vector_value(&mut rt.rng());
        let obj = assert_obj!(INDEXED_FIELD => vector, "A" => ConvexValue::Int64(1017));
        let id = UserFacingModel::new_root_for_test(&mut tx)
            .insert(TABLE_NAME.parse()?, obj)
            .await?;
        ids.push(id);
        scenario.database.commit(tx).await?;
        // Backfill to create a new segment
        scenario.backfill().await?;
    }
    // Then delete some ids and compact.
    let ids_to_delete = ids[0..ids.len() / 2].to_vec();
    let mut tx = scenario.database.begin(Identity::system()).await?;
    for id in &ids_to_delete {
        UserFacingModel::new_root_for_test(&mut tx)
            .delete(*id)
            .await?;
    }
    scenario.database.commit(tx).await?;
    scenario.compact().await?;

    let results = scenario.search(vec![0.; 4], btreeset![]).await?;
    assert_eq!(
        results
            .into_iter()
            .map(|result| result.id.internal_id())
            .sorted()
            .collect::<Vec<_>>(),
        ids.iter()
            .filter(|id| !ids_to_delete.contains(id))
            .map(|id| id.internal_id())
            .sorted()
            .collect::<Vec<_>>()
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_index_backfill_is_incremental(rt: TestRuntime) -> anyhow::Result<()> {
    let scenario = Scenario::new(rt.clone()).await?;
    let num_parts = 12;
    let vectors_per_part = 8;
    let incremental_index_size =
        (DIMENSIONS * (VECTOR_ELEMENT_SIZE as u32) * vectors_per_part) as usize;

    let ids = scenario
        .seed_table_with_vector_data(num_parts * vectors_per_part)
        .await?;

    scenario.add_vector_index(false).await?;

    let flusher = scenario.new_backfill_flusher(incremental_index_size);

    let mut backfill_ts = None;
    for i in 0..num_parts {
        // Do a backfill iteration
        flusher.step().await?;

        // Fetch the current index metadata
        let mut vec_indexes = scenario.get_vector_index_configs().await?;
        assert_eq!(vec_indexes.len(), 1);
        let (_, on_disk_state) = vec_indexes.remove(0);

        // Verify that on_disk_state remains in backfilling until last iteration
        // and each iteration adds a new segment.
        if i < num_parts - 1 {
            must_let!(let VectorIndexState::Backfilling(
                VectorIndexBackfillState {
                    segments,
                    backfill_snapshot_ts,
                    ..
                }) = on_disk_state);
            assert_eq!(segments.len(), (i + 1) as usize);
            backfill_ts = backfill_snapshot_ts;
        } else {
            must_let!(let VectorIndexState::Backfilled {
                snapshot: VectorIndexSnapshot {
                    data,
                    ts,
                },
                ..
            } = on_disk_state);
            // Verify snapshot timestamp matches backfill timestamp
            assert_eq!(backfill_ts.unwrap(), ts);
            must_let!(let VectorIndexSnapshotData::MultiSegment(segments) = data);
            assert_eq!(segments.len(), (num_parts) as usize);
        }
    }

    // Enable the index
    scenario.enable_index().await?;

    // Verify that IDs match
    let results = scenario
        .search_with_limit(vec![0.; 4], btreeset![], Some(256))
        .await?;

    let left = results
        .into_iter()
        .map(|result| result.id.internal_id())
        .sorted()
        .collect::<Vec<_>>();
    let right = ids
        .iter()
        .map(|id| id.internal_id())
        .sorted()
        .collect::<Vec<_>>();
    assert_eq!(left, right);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_incremental_backfill_with_compaction(rt: TestRuntime) -> anyhow::Result<()> {
    let mut scenario = Scenario::new(rt.clone()).await?;
    let num_parts = 3;
    let vectors_per_part = 8;
    let incremental_index_size =
        (DIMENSIONS * (VECTOR_ELEMENT_SIZE as u32) * vectors_per_part) as usize;

    let ids = scenario
        .seed_table_with_vector_data(num_parts * vectors_per_part)
        .await?;

    scenario.add_vector_index(false).await?;

    let flusher = scenario.new_backfill_flusher(incremental_index_size);

    for _ in 0..num_parts {
        // Do a backfill iteration
        flusher.step().await?;
    }
    scenario.compact().await?;

    // There should be 2 parts after compaction
    let mut vec_indexes = scenario.get_vector_index_configs().await?;
    assert_eq!(vec_indexes.len(), 1);
    let (_, on_disk_state) = vec_indexes.remove(0);
    must_let!(let VectorIndexState::Backfilled {
        snapshot: VectorIndexSnapshot { data: VectorIndexSnapshotData::MultiSegment(segments), .. },
        ..
    } = on_disk_state);
    assert_eq!(segments.len(), 1);

    // Enable the index
    scenario.enable_index().await?;

    // Verify that IDs match
    let results = scenario
        .search_with_limit(vec![0.; 4], btreeset![], Some(256))
        .await?;

    let left = results
        .into_iter()
        .map(|result| result.id.internal_id())
        .sorted()
        .collect::<Vec<_>>();
    let right = ids
        .iter()
        .map(|id| id.internal_id())
        .sorted()
        .collect::<Vec<_>>();
    assert_eq!(left, right);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_empty_multi_segment(rt: TestRuntime) -> anyhow::Result<()> {
    let scenario = Scenario::new_with_enabled_index(rt.clone()).await?;
    let query = random_vector(&mut rt.rng());
    let results = scenario
        .search_with_limit(query, btreeset![], Some(10))
        .await?;

    assert!(results.is_empty());
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_recall_multi_segment(rt: TestRuntime) -> anyhow::Result<()> {
    let scenario = Scenario::new_with_enabled_index(rt.clone()).await?;
    let mut tx = scenario.database.begin(Identity::system()).await?;
    let table_number = tx
        .table_mapping()
        .namespace(TABLE_NAMESPACE)
        .name_to_number_user_input()(TABLE_NAME.parse()?)?;

    let mut rng = rt.rng();
    let mut by_id = BTreeMap::new();
    for _ in 0..100 {
        let vector = random_vector(&mut rng);
        let obj = assert_obj!(INDEXED_FIELD => vector_to_value(vector.clone()));
        let id = UserFacingModel::new_root_for_test(&mut tx)
            .insert(TABLE_NAME.parse()?, obj)
            .await?;
        by_id.insert(id.internal_id(), vector);
    }
    scenario.database.commit(tx).await?;

    let limit = 10u32;

    let query = random_vector(&mut rng);
    let mut expected: Vec<_> = by_id
        .iter()
        .map(|(id, vector)| PublicVectorSearchQueryResult {
            id: DeveloperDocumentId::new(table_number, *id),
            score: cosine_similarity(&query, vector),
        })
        .collect();
    expected.sort_by(|a, b| a.cmp(b).reverse());
    expected.truncate(limit as usize);

    for _ in 0..2 {
        let results = scenario
            .search_with_limit(query.clone(), btreeset![], Some(limit))
            .await?;

        assert_eq!(results, expected);

        scenario.backfill().await?;
    }

    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 32 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1),
        failure_persistence: None,
        .. ProptestConfig::default()
    })]
    #[ignore]
    #[test]
    fn proptest_vector_search_results(
        actions in prop::collection::vec(any::<TestAction>(), 1..16),
    ) {
        let td = TestDriver::new();
        let rt = td.rt();
        let future = async move {
        let mut test = RandomizedTest::new(rt).await?;
        for action in actions {
            test.execute(action).await?;
        }
        anyhow::Ok(())
        };
        td.run_until(future).unwrap();
    }
}

#[convex_macro::test_runtime]
async fn test_multi_segment_search_obeys_sorted_order(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = VectorFixtures::new(rt.clone()).await?;

    let IndexData {
        index_name,
        resolved_index_name,
        ..
    } = fixtures.enabled_vector_index().await?;

    let vectors = [[3f64, 4f64], [5f64, 6f64], [6f64, 7f64]];
    let mut ids = vec![];

    for vector in vectors {
        let id = fixtures
            .add_document_vec_array(index_name.table(), vector)
            .await?;
        ids.push(id);
        let worker =
            fixtures.new_index_flusher_with_full_scan_threshold(0, FlusherType::LiveFlush)?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});
    }

    let (results, _usage_stats) = fixtures
        .db
        .vector_search(
            Identity::system(),
            VectorSearch {
                index_name: index_name.clone(),
                component_id: ComponentId::Root,
                vector: [6f64, 7f64].into_iter().map(|value| value as f32).collect(),
                limit: Some(3),
                expressions: btreeset![],
            },
        )
        .await?;

    // Result IDs should match IDs in **reverse order** since input vector
    // corresponds to [6, 7] or ids[2]
    assert_eq!(
        results
            .into_iter()
            .map(|result| result.id.internal_id())
            .collect_vec(),
        ids.into_iter()
            .rev()
            .map(|id| id.internal_id())
            .collect_vec(),
    );

    Ok(())
}
