use std::{
    cmp::max,
    future,
    ops::Bound,
    time::Duration,
};

use common::{
    bootstrap_model::index::{
        vector_index::VectorIndexState,
        IndexConfig,
        TabletIndexMetadata,
    },
    document::ParsedDocument,
    errors::report_error,
    knobs::UDF_EXECUTOR_OCC_MAX_RETRIES,
    persistence::{
        RepeatablePersistence,
        TimestampRange,
    },
    persistence_helpers::stream_revision_pairs,
    query::Order,
    runtime::Runtime,
    types::WriteTimestamp,
};
use errors::ErrorMetadataAnyhowExt;
use futures::TryStreamExt;
use imbl::OrdMap;
use indexing::index_registry::IndexRegistry;
use sync_types::{
    backoff::Backoff,
    Timestamp,
};
use value::TableMapping;
use vector::{
    IndexState,
    MemoryVectorIndex,
    QdrantSchema,
    VectorIndexManager,
};

use crate::{
    committer::CommitterClient,
    index_workers::fast_forward::load_metadata_fast_forward_ts,
};

pub struct VectorBootstrapWorker<RT: Runtime> {
    runtime: RT,
    index_registry: IndexRegistry,
    persistence: RepeatablePersistence,
    table_mapping: TableMapping,
    committer_client: CommitterClient<RT>,
    backoff: Backoff,
}

const INITIAL_BACKOFF: Duration = Duration::from_millis(10);
const MAX_BACKOFF: Duration = Duration::from_secs(5);

impl<RT: Runtime> VectorBootstrapWorker<RT> {
    pub(crate) fn new(
        runtime: RT,
        index_registry: IndexRegistry,
        persistence: RepeatablePersistence,
        table_mapping: TableMapping,
        committer_client: CommitterClient<RT>,
    ) -> Self {
        {
            Self {
                runtime,
                index_registry,
                table_mapping,
                persistence,
                committer_client,
                backoff: Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF),
            }
        }
    }

    pub async fn start(mut self) {
        let timer = crate::metrics::vector::bootstrap_timer();
        loop {
            if let Err(e) = self.run().await {
                let delay = self.runtime.with_rng(|rng| self.backoff.fail(rng));
                // Forgive OCC errors < N to match UDF mutation retry behavior.
                if !e.is_occ() || (self.backoff.failures() as usize) > *UDF_EXECUTOR_OCC_MAX_RETRIES
                {
                    report_error(&mut e.context("VectorBootstrapWorker died"));
                    tracing::error!(
                        "VectorBootstrapWorker died, num_failures: {}. Backing off for {}ms",
                        self.backoff.failures(),
                        delay.as_millis()
                    );
                } else {
                    tracing::trace!(
                        "VectorBootstrapWorker occed, retrying. num_failures: {}, backoff: {}ms",
                        self.backoff.failures(),
                        delay.as_millis(),
                    )
                }
                self.runtime.wait(delay).await;
            } else {
                tracing::info!("Vector index bootstrap worker finished!");
                break;
            }
        }
        timer.finish();
    }

    async fn run(&self) -> anyhow::Result<()> {
        let vector_index_manager = self.bootstrap_manager().await?;
        self.finish_bootstrap(vector_index_manager).await
    }

    async fn bootstrap_manager(&self) -> anyhow::Result<VectorIndexManager> {
        // Load all of the fast forward timestamps first to ensure that we stay within
        // the comparatively short valid time for the persistence snapshot
        let snapshot = self
            .persistence
            .read_snapshot(self.persistence.upper_bound())?;
        let mut indexes_with_fast_forward_ts = vec![];
        for index in self.index_registry.all_vector_indexes() {
            let fast_forward_ts = load_metadata_fast_forward_ts(
                &self.index_registry,
                &snapshot,
                &self.table_mapping,
                &index,
            )
            .await?;
            indexes_with_fast_forward_ts.push((index, fast_forward_ts));
        }

        Self::bootstrap(&self.persistence, indexes_with_fast_forward_ts).await
    }

    async fn bootstrap(
        persistence: &RepeatablePersistence,
        vector_indexes_with_fast_forward_timestamps: Vec<(
            ParsedDocument<TabletIndexMetadata>,
            Option<Timestamp>,
        )>,
    ) -> anyhow::Result<VectorIndexManager> {
        let timer = vector::metrics::bootstrap_timer();
        let upper_bound = persistence.upper_bound();

        let mut indexes = IndexState::Ready(OrdMap::new());

        let mut num_revisions = 0;
        let mut total_size = 0;

        for (index_doc, fast_forward_ts) in vector_indexes_with_fast_forward_timestamps {
            let (index_id, index_metadata) = index_doc.into_id_and_value();

            let IndexConfig::Vector {
                ref developer_config,
                ref on_disk_state,
            } = &index_metadata.config
            else {
                continue;
            };
            let snapshot_info = match on_disk_state {
                VectorIndexState::Backfilled(ref snapshot_info) => snapshot_info,
                VectorIndexState::SnapshottedAt(ref snapshot_info) => snapshot_info,
                VectorIndexState::Backfilling(_) => {
                    indexes.insert(
                        index_id.internal_id(),
                        on_disk_state.clone(),
                        MemoryVectorIndex::new(WriteTimestamp::Committed(upper_bound.succ()?)),
                    );
                    continue;
                },
            };

            let ts = max(fast_forward_ts.unwrap_or_default(), snapshot_info.ts);
            tracing::info!(
                "Bootstrapping vector index {:?} from {ts} for fast forward ts: {}, snapshot_ts: \
                 {}",
                index_metadata.name,
                fast_forward_ts.unwrap_or_default(),
                snapshot_info.ts
            );
            let range = (Bound::Excluded(ts), Bound::Included(*upper_bound));
            let document_stream = persistence
                .load_documents(TimestampRange::new(range)?, Order::Asc)
                .try_filter(|(_, id, _)| future::ready(id.table() == index_metadata.name.table()));
            let revision_stream = stream_revision_pairs(document_stream, persistence);
            futures::pin_mut!(revision_stream);

            let qdrant_schema = QdrantSchema::new(developer_config);

            let mut memory_index =
                MemoryVectorIndex::new(WriteTimestamp::Committed(snapshot_info.ts.succ()?));
            while let Some(revision_pair) = revision_stream.try_next().await? {
                memory_index.update(
                    revision_pair.id.internal_id(),
                    WriteTimestamp::Committed(revision_pair.ts()),
                    revision_pair
                        .prev_document()
                        .and_then(|d| qdrant_schema.index(d)),
                    revision_pair
                        .document()
                        .and_then(|d| qdrant_schema.index(d)),
                )?;
                num_revisions += 1;
                total_size += revision_pair.document().map(|d| d.size()).unwrap_or(0);
            }

            indexes.insert(index_id.internal_id(), on_disk_state.clone(), memory_index);
        }

        tracing::info!(
            "Loaded {num_revisions} revisions ({total_size} bytes) in {:?}.",
            timer.elapsed()
        );
        vector::metrics::finish_bootstrap(num_revisions, total_size, timer);
        Ok(VectorIndexManager { indexes })
    }

    async fn finish_bootstrap(
        &self,
        vector_index_manager: VectorIndexManager,
    ) -> anyhow::Result<()> {
        self.committer_client
            .finish_vector_bootstrap(vector_index_manager, self.persistence.upper_bound())
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use common::{
        bootstrap_model::index::IndexMetadata,
        types::IndexName,
    };
    use keybroker::Identity;
    use maplit::btreeset;
    use runtime::testing::TestRuntime;
    use storage::Storage;
    use sync_types::Timestamp;
    use value::{
        assert_obj,
        ConvexValue,
        FieldPath,
        GenericDocumentId,
        InternalId,
        TableName,
        TableNumber,
    };
    use vector::{
        PublicVectorSearchQueryResult,
        VectorSearch,
    };

    use crate::{
        bootstrap_model::index_workers::IndexWorkerMetadataModel,
        test_helpers::{
            index_utils::assert_enabled,
            DbFixtures,
            DbFixturesArgs,
        },
        vector_index_worker::flusher::VectorIndexFlusher,
        Database,
        IndexModel,
    };

    #[convex_macro::test_runtime]
    async fn persisted_vectors_are_not_included(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = DbFixtures::new(&rt).await?;
        let (_, index_metadata) =
            add_and_enable_vector_index(&rt, &fixtures.db, fixtures.search_storage.clone()).await?;

        let db = reopen_db(&rt, &fixtures).await?;
        add_vector(&db, &index_metadata, [1f32, 2f32]).await?;
        backfill_vector_indexes(&rt, &db, 0, fixtures.search_storage).await?;

        // This is a bit of a hack, backfilling with zero size forces all indexes to be
        // written to disk, which causes our boostrapping process to skip our
        // vector. Normally the vector would still be loaded via Searchlight,
        // but in our test setup we use a no-op searcher so the "disk" ends up being
        // excluded.
        let result = query_vectors(&db, &index_metadata).await?;
        assert!(result.is_empty());

        Ok(())
    }

    fn assert_expected_vector(
        vectors: Vec<PublicVectorSearchQueryResult>,
        expected: GenericDocumentId<TableNumber>,
    ) {
        assert_eq!(
            vectors
                .into_iter()
                .map(|result| result.id)
                .collect::<Vec<_>>(),
            vec![expected]
        );
    }

    #[convex_macro::test_runtime]
    async fn vector_added_after_bootstrap_is_included(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = DbFixtures::new(&rt).await?;
        let (_, index_metadata) =
            add_and_enable_vector_index(&rt, &fixtures.db, fixtures.search_storage.clone()).await?;

        let db = reopen_db(&rt, &fixtures).await?;
        let vector_id = add_vector(&db, &index_metadata, [1f32, 2f32]).await?;

        let result = query_vectors(&db, &index_metadata).await?;
        assert_expected_vector(result, vector_id);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn vector_added_before_bootstrap_is_included(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = DbFixtures::new(&rt).await?;
        let (_, index_metadata) =
            add_and_enable_vector_index(&rt, &fixtures.db, fixtures.search_storage.clone()).await?;

        let vector_id = add_vector(&fixtures.db, &index_metadata, [1f32, 2f32]).await?;

        let db = reopen_db(&rt, &fixtures).await?;

        let result = query_vectors(&db, &index_metadata).await?;
        assert_expected_vector(result, vector_id);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn vector_added_before_bootstrap_but_after_fast_forward_is_excluded(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = DbFixtures::new(&rt).await?;
        let (index_id, index_metadata) =
            add_and_enable_vector_index(&rt, &fixtures.db, fixtures.search_storage.clone()).await?;

        add_vector(&fixtures.db, &index_metadata, [1f32, 2f32]).await?;
        let mut tx = fixtures.db.begin_system().await?;
        let mut model = IndexWorkerMetadataModel::new(&mut tx);
        let (metadata_id, mut metadata) = model
            .get_or_create_text_search(index_id)
            .await?
            .into_id_and_value();
        *metadata.index_metadata.mut_fast_forward_ts() = Timestamp::MAX.pred().unwrap();
        tx.replace_system_document(metadata_id, metadata.try_into()?)
            .await?;
        fixtures.db.commit(tx).await?;

        let db = reopen_db(&rt, &fixtures).await?;

        let result = query_vectors(&db, &index_metadata).await?;
        assert!(result.is_empty());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn load_snapshot_with_fast_forward_ts_uses_disk_ts_for_memory_index(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = DbFixtures::new(&rt).await?;
        let (index_id, index_metadata) =
            add_and_backfill_index(&rt, &fixtures.db, fixtures.search_storage.clone()).await?;

        add_vector(&fixtures.db, &index_metadata, [1f32, 2f32]).await?;
        let mut tx = fixtures.db.begin_system().await?;
        let mut model = IndexWorkerMetadataModel::new(&mut tx);
        let (metadata_id, mut metadata) = model
            .get_or_create_text_search(index_id)
            .await?
            .into_id_and_value();
        *metadata.index_metadata.mut_fast_forward_ts() = Timestamp::MAX.pred().unwrap();
        tx.replace_system_document(metadata_id, metadata.try_into()?)
            .await?;
        fixtures.db.commit(tx).await?;

        // If we use the wrong timestamp here (e.g. MAX), then enabling this index will
        // fail because the memory snapshot will have a higher timestamp than
        // our index doc.
        let db = reopen_db(&rt, &fixtures).await?;
        let mut tx = db.begin_system().await?;
        IndexModel::new(&mut tx)
            .enable_index_for_testing(&index_metadata.name)
            .await?;
        db.commit(tx).await?;

        let result = query_vectors(&db, &index_metadata).await?;
        assert!(result.is_empty());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn vector_added_during_bootstrap_is_included(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = DbFixtures::new(&rt).await?;
        let (_, index_metadata) =
            add_and_enable_vector_index(&rt, &fixtures.db, fixtures.search_storage.clone()).await?;

        let db = reopen_db(&rt, &fixtures).await?;
        let worker = db.new_vector_bootstrap_worker_for_testing();

        let manager = worker.bootstrap_manager().await?;
        let vector_id = add_vector(&db, &index_metadata, [3f32, 4f32]).await?;
        worker.finish_bootstrap(manager).await?;

        let result = query_vectors(&db, &index_metadata).await?;
        assert_expected_vector(result, vector_id);

        Ok(())
    }

    async fn add_and_backfill_index(
        rt: &TestRuntime,
        db: &Database<TestRuntime>,
        storage: Arc<dyn Storage>,
    ) -> anyhow::Result<(InternalId, IndexMetadata<TableName>)> {
        let index_metadata = backfilling_vector_index()?;
        let mut tx = db.begin_system().await?;
        IndexModel::new(&mut tx)
            .add_application_index(index_metadata.clone())
            .await?;
        db.commit(tx).await?;
        backfill_vector_indexes(rt, db, 1000, storage.clone()).await?;
        let mut tx = db.begin_system().await?;
        let resolved_index = IndexModel::new(&mut tx)
            .pending_index_metadata(&index_metadata.name)?
            .expect("Missing index metadata!");

        Ok((resolved_index.id().internal_id(), index_metadata))
    }

    async fn add_and_enable_vector_index(
        rt: &TestRuntime,
        db: &Database<TestRuntime>,
        storage: Arc<dyn Storage>,
    ) -> anyhow::Result<(InternalId, IndexMetadata<TableName>)> {
        let (_, index_metadata) = add_and_backfill_index(rt, db, storage.clone()).await?;

        let mut tx = db.begin_system().await?;
        let resolved_index = IndexModel::new(&mut tx)
            .pending_index_metadata(&index_metadata.name)?
            .expect("Missing index metadata!");
        IndexModel::new(&mut tx)
            .enable_backfilled_indexes(vec![resolved_index.clone().into_value()])
            .await?;
        db.commit(tx).await?;
        assert_enabled(
            db,
            index_metadata.name.table(),
            index_metadata.name.descriptor().as_str(),
        )
        .await?;
        Ok((resolved_index.id().internal_id(), index_metadata))
    }

    async fn reopen_db(
        rt: &TestRuntime,
        db_fixtures: &DbFixtures<TestRuntime>,
    ) -> anyhow::Result<Database<TestRuntime>> {
        let DbFixtures { db, .. } = DbFixtures::new_with_args(
            rt,
            DbFixturesArgs {
                tp: Some(db_fixtures.tp.clone()),
                searcher: Some(db_fixtures.searcher.clone()),
                search_storage: Some(db_fixtures.search_storage.clone()),
                ..Default::default()
            },
        )
        .await?;
        Ok(db)
    }

    async fn query_vectors(
        db: &Database<TestRuntime>,
        index_metadata: &IndexMetadata<TableName>,
    ) -> anyhow::Result<Vec<PublicVectorSearchQueryResult>> {
        let query = VectorSearch {
            index_name: index_metadata.name.clone(),
            vector: vec![0.; 2],
            limit: None,
            expressions: btreeset![],
        };
        let (results, _usage_stats) = db.vector_search(Identity::system(), query).await?;
        Ok(results)
    }

    async fn add_vector(
        db: &Database<TestRuntime>,
        index_metadata: &IndexMetadata<TableName>,
        vector: [f32; 2],
    ) -> anyhow::Result<GenericDocumentId<TableNumber>> {
        let mut tx = db.begin_system().await?;
        let values: ConvexValue = vector
            .into_iter()
            .map(|f| ConvexValue::Float64(f as f64))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let document = assert_obj!(
            "vector" => values,
            "channel" => ConvexValue::String("#general".try_into()?),
        );
        let document_id = tx
            .insert_user_facing(index_metadata.name.table().clone(), document)
            .await?;
        db.commit(tx).await?;
        Ok(document_id)
    }

    fn backfilling_vector_index() -> anyhow::Result<IndexMetadata<TableName>> {
        let table_name: TableName = "table".parse()?;
        let index_name = IndexName::new(table_name, "vector_index".parse()?)?;
        let vector_field: FieldPath = "vector".parse()?;
        let filter_field: FieldPath = "channel".parse()?;
        let metadata = IndexMetadata::new_backfilling_vector_index(
            index_name,
            vector_field,
            (2u32).try_into()?,
            btreeset![filter_field],
        );
        Ok(metadata)
    }

    async fn backfill_vector_indexes(
        rt: &TestRuntime,
        db: &Database<TestRuntime>,
        index_size_soft_limit: usize,
        storage: Arc<dyn Storage>,
    ) -> anyhow::Result<()> {
        VectorIndexFlusher::backfill_all_in_test(
            rt.clone(),
            db.clone(),
            storage,
            index_size_soft_limit,
        )
        .await?;
        Ok(())
    }
}
