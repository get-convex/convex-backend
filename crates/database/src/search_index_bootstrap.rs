use std::{
    cmp::{
        max,
        min,
    },
    collections::{
        BTreeMap,
        BTreeSet,
    },
    ops::Bound,
    time::Duration,
};

use common::{
    bootstrap_model::index::{
        text_index::{
            TextIndexSnapshot,
            TextIndexState,
        },
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
    persistence_helpers::{
        stream_revision_pairs,
        RevisionPair,
    },
    query::Order,
    runtime::{
        try_join_buffer_unordered,
        Runtime,
    },
    types::{
        IndexId,
        PersistenceVersion,
        RepeatableTimestamp,
        WriteTimestamp,
    },
};
use errors::ErrorMetadataAnyhowExt;
use futures::{
    future,
    Stream,
    TryStreamExt,
};
use indexing::index_registry::IndexRegistry;
use search::{
    DiskIndex,
    MemoryTextIndex,
    SnapshotInfo,
    TantivySearchIndexSchema,
    TextIndex,
    TextIndexManager,
    TextIndexManagerState,
};
use sync_types::{
    backoff::Backoff,
    Timestamp,
};
use value::{
    NamespacedTableMapping,
    TabletId,
};
use vector::{
    IndexState,
    MemoryVectorIndex,
    QdrantSchema,
    VectorIndexManager,
};

use crate::{
    committer::CommitterClient,
    metrics::log_document_skipped,
    search_index_workers::fast_forward::load_metadata_fast_forward_ts,
};

pub const FINISHED_BOOTSTRAP_UPDATES: &str = "finished_bootstrap_updates";

pub struct SearchIndexBootstrapWorker<RT: Runtime> {
    runtime: RT,
    index_registry: IndexRegistry,
    persistence: RepeatablePersistence,
    table_mapping: NamespacedTableMapping,
    committer_client: CommitterClient,
    backoff: Backoff,
}

const INITIAL_BACKOFF: Duration = Duration::from_millis(10);
const MAX_BACKOFF: Duration = Duration::from_secs(5);

struct IndexesToBootstrap {
    table_to_text_indexes: BTreeMap<TabletId, Vec<TextIndexBootstrapData>>,
    table_to_vector_indexes: BTreeMap<TabletId, Vec<VectorIndexBootstrapData>>,
    /// Timestamp to walk the document log from to get all of the revisions
    /// since the last write to disk.
    oldest_index_ts: Timestamp,
}

pub struct BootstrappedSearchIndexes {
    pub text_index_manager: TextIndexManager,
    pub vector_index_manager: VectorIndexManager,
    pub tables_with_indexes: BTreeSet<TabletId>,
}

impl IndexesToBootstrap {
    fn create(
        upper_bound: RepeatableTimestamp,
        indexes_with_fast_forward_ts: Vec<(ParsedDocument<TabletIndexMetadata>, Option<Timestamp>)>,
    ) -> anyhow::Result<Self> {
        let mut table_to_vector_indexes: BTreeMap<_, Vec<_>> = BTreeMap::new();
        let mut table_to_text_indexes: BTreeMap<_, Vec<_>> = BTreeMap::new();
        // We keep track of latest ts we can bootstrap from for each vector index.
        let mut oldest_index_ts = *upper_bound;

        for (index_doc, fast_forward_ts) in indexes_with_fast_forward_ts {
            let is_enabled = index_doc.config.is_enabled();
            let (index_id, index_metadata) = index_doc.into_id_and_value();
            match index_metadata.config {
                IndexConfig::Vector {
                    on_disk_state,
                    ref spec,
                    ..
                } => {
                    let qdrant_schema = QdrantSchema::new(spec);
                    let ts = match on_disk_state {
                        VectorIndexState::Backfilled {
                            snapshot: ref snapshot_info,
                            ..
                        }
                        | VectorIndexState::SnapshottedAt(ref snapshot_info) => {
                            // Use fast forward ts instead of snapshot ts.
                            let current_index_ts =
                                max(fast_forward_ts.unwrap_or_default(), snapshot_info.ts);
                            oldest_index_ts = min(oldest_index_ts, current_index_ts);
                            snapshot_info.ts
                        },
                        VectorIndexState::Backfilling(_) => upper_bound.succ()?,
                    };
                    let vector_index_bootstrap_data = VectorIndexBootstrapData {
                        index_id: index_id.internal_id(),
                        on_disk_state,
                        memory_index: MemoryVectorIndex::new(WriteTimestamp::Committed(ts.succ()?)),
                        qdrant_schema,
                    };
                    if let Some(vector_indexes) =
                        table_to_vector_indexes.get_mut(index_metadata.name.table())
                    {
                        vector_indexes.push(vector_index_bootstrap_data);
                    } else {
                        table_to_vector_indexes.insert(
                            *index_metadata.name.table(),
                            vec![vector_index_bootstrap_data],
                        );
                    }
                },
                IndexConfig::Text {
                    ref spec,
                    on_disk_state,
                } => {
                    let text_index = match on_disk_state {
                        TextIndexState::Backfilling(_) => {
                            // We'll start a new memory search index starting at the next commit
                            // after our persistence upper bound. After
                            // bootstrapping, all commits after
                            // `persistence.upper_bound()` will flow through `Self::update`, so our
                            // memory index contains all revisions `>=
                            // persistence.upper_bound().succ()?`.
                            let memory_index = MemoryTextIndex::new(WriteTimestamp::Committed(
                                upper_bound.succ()?,
                            ));
                            TextIndex::Backfilling { memory_index }
                        },
                        TextIndexState::Backfilled {
                            snapshot:
                                TextIndexSnapshot {
                                    data,
                                    ts: disk_ts,
                                    version,
                                },
                            staged: _,
                        }
                        | TextIndexState::SnapshottedAt(TextIndexSnapshot {
                            data,
                            ts: disk_ts,
                            version,
                        }) => {
                            let current_index_ts =
                                max(disk_ts, fast_forward_ts.unwrap_or_default());
                            oldest_index_ts = min(oldest_index_ts, current_index_ts);
                            let memory_index =
                                MemoryTextIndex::new(WriteTimestamp::Committed(disk_ts.succ()?));
                            let snapshot = SnapshotInfo {
                                disk_index: DiskIndex::try_from(data)?,
                                disk_index_ts: current_index_ts,
                                disk_index_version: version,
                                memory_index,
                            };
                            if is_enabled {
                                TextIndex::Ready(snapshot)
                            } else {
                                TextIndex::Backfilled(snapshot)
                            }
                        },
                    };
                    let tantivy_schema = TantivySearchIndexSchema::new(spec);
                    let text_index_bootstrap_data = TextIndexBootstrapData {
                        index_id: index_id.internal_id(),
                        text_index,
                        tantivy_schema,
                    };
                    if let Some(text_indexes) =
                        table_to_text_indexes.get_mut(index_metadata.name.table())
                    {
                        text_indexes.push(text_index_bootstrap_data);
                    } else {
                        table_to_text_indexes.insert(
                            *index_metadata.name.table(),
                            vec![text_index_bootstrap_data],
                        );
                    }
                },
                _ => continue,
            };
        }
        Ok(Self {
            table_to_text_indexes,
            table_to_vector_indexes,
            oldest_index_ts,
        })
    }

    fn tables_with_indexes(&self) -> BTreeSet<TabletId> {
        self.table_to_text_indexes
            .keys()
            .chain(self.table_to_vector_indexes.keys())
            .copied()
            .collect()
    }

    async fn bootstrap(
        mut self,
        persistence: &RepeatablePersistence,
    ) -> anyhow::Result<BootstrappedSearchIndexes> {
        let timer = crate::metrics::bootstrap_timer();
        let upper_bound = persistence.upper_bound();
        let mut num_revisions = 0;
        let mut total_size = 0;

        let range = TimestampRange::new((
            Bound::Excluded(self.oldest_index_ts),
            Bound::Included(*upper_bound),
        ))?;
        let tables_with_indexes = self.tables_with_indexes();
        let revision_stream =
            stream_revision_pairs_for_indexes(&tables_with_indexes, persistence, range);
        futures::pin_mut!(revision_stream);

        tracing::info!(
            "Starting search index bootstrap at {} with upper bound {}",
            self.oldest_index_ts,
            upper_bound
        );
        while let Some(revision_pair) = revision_stream.try_next().await? {
            num_revisions += 1;
            total_size += revision_pair.document().map(|d| d.size()).unwrap_or(0);
            if let Some(vector_indexes_to_update) = self
                .table_to_vector_indexes
                .get_mut(&revision_pair.id.table())
            {
                for vector_index in vector_indexes_to_update {
                    vector_index.update(&revision_pair)?;
                }
            }
            if let Some(text_indexes_to_update) = self
                .table_to_text_indexes
                .get_mut(&revision_pair.id.table())
            {
                for text_index in text_indexes_to_update {
                    text_index.update(&revision_pair)?;
                }
            }
            if num_revisions % 500 == 0 {
                let percent_progress =
                    (u64::from(revision_pair.ts()) - u64::from(self.oldest_index_ts)) as f64
                        / (u64::from(*upper_bound) - u64::from(self.oldest_index_ts)) as f64
                        * 100.0;
                tracing::info!(
                    "Processed ts {}, estimated progress: ({:.1}%)",
                    revision_pair.ts(),
                    percent_progress
                );
            }
        }

        tracing::info!(
            "Loaded {num_revisions} revisions ({total_size} bytes) in {:?}.",
            timer.elapsed()
        );
        crate::metrics::finish_bootstrap(num_revisions, total_size, timer);

        Ok(self.finish(persistence.version()))
    }

    fn finish(self, persistence_version: PersistenceVersion) -> BootstrappedSearchIndexes {
        let tables_with_indexes = self.tables_with_indexes();
        let text_index_manager = TextIndexManager::new(
            TextIndexManagerState::Ready(
                self.table_to_text_indexes
                    .into_iter()
                    .flat_map(|(_id, text_indexes)| {
                        text_indexes
                            .into_iter()
                            .map(
                                |TextIndexBootstrapData {
                                     index_id,
                                     text_index: search_index,
                                     tantivy_schema: _,
                                 }| (index_id, search_index),
                            )
                            .collect::<Vec<_>>()
                    })
                    .collect(),
            ),
            persistence_version,
        );
        let indexes = IndexState::Ready(
            self.table_to_vector_indexes
                .into_iter()
                .flat_map(|(_id, vector_indexes)| {
                    vector_indexes
                        .into_iter()
                        .map(
                            |VectorIndexBootstrapData {
                                 index_id,
                                 on_disk_state,
                                 memory_index,
                                 qdrant_schema: _,
                             }| {
                                (index_id, (on_disk_state, memory_index))
                            },
                        )
                        .collect::<Vec<_>>()
                })
                .collect(),
        );
        let vector_index_manager = VectorIndexManager { indexes };
        BootstrappedSearchIndexes {
            text_index_manager,
            vector_index_manager,
            tables_with_indexes,
        }
    }
}

#[derive(Clone)]
struct TextIndexBootstrapData {
    index_id: IndexId,
    text_index: TextIndex,
    tantivy_schema: TantivySearchIndexSchema,
}

impl TextIndexBootstrapData {
    fn update(&mut self, revision_pair: &RevisionPair) -> anyhow::Result<()> {
        let memory_index = self.text_index.memory_index_mut();
        match memory_index.min_ts() {
            WriteTimestamp::Pending => {
                anyhow::bail!(
                    "Found a pending write timestamp for search memory index created during \
                     bootstrapping. This should always be a committed timestamp."
                )
            },
            WriteTimestamp::Committed(ts) => {
                // Skip updates for revision pairs that have already been written to disk.
                if ts > revision_pair.ts() {
                    return Ok(());
                }
            },
        }
        memory_index.update(
            revision_pair.id.internal_id(),
            WriteTimestamp::Committed(revision_pair.ts()),
            revision_pair
                .prev_document()
                .map(|d| anyhow::Ok((self.tantivy_schema.index_into_terms(d)?, d.creation_time())))
                .transpose()?,
            revision_pair
                .document()
                .map(|d| anyhow::Ok((self.tantivy_schema.index_into_terms(d)?, d.creation_time())))
                .transpose()?,
        )
    }
}

#[derive(Clone)]
struct VectorIndexBootstrapData {
    index_id: IndexId,
    on_disk_state: VectorIndexState,
    memory_index: MemoryVectorIndex,
    qdrant_schema: QdrantSchema,
}

impl VectorIndexBootstrapData {
    fn update(&mut self, revision_pair: &RevisionPair) -> anyhow::Result<()> {
        match self.memory_index.min_ts() {
            WriteTimestamp::Pending => {
                anyhow::bail!(
                    "Found a pending write timestamp for vector memory index created during \
                     bootstrapping. This should always be a committed timestamp."
                )
            },
            WriteTimestamp::Committed(ts) => {
                // Skip updates for revision pairs that have already been written to disk.
                if ts > revision_pair.ts() {
                    return Ok(());
                }
            },
        }
        self.memory_index.update(
            revision_pair.id.internal_id(),
            WriteTimestamp::Committed(revision_pair.ts()),
            revision_pair
                .prev_document()
                .and_then(|d| self.qdrant_schema.index(d)),
            revision_pair
                .document()
                .and_then(|d| self.qdrant_schema.index(d)),
        )
    }
}

/// Streams revision pairs for documents in the indexed tables.
pub fn stream_revision_pairs_for_indexes<'a>(
    tables_with_indexes: &'a BTreeSet<TabletId>,
    persistence: &'a RepeatablePersistence,
    range: TimestampRange,
) -> impl Stream<Item = anyhow::Result<RevisionPair>> + 'a {
    let document_stream = persistence
        .load_documents(range, Order::Asc)
        .try_filter(|entry| {
            let is_in_indexed_table = tables_with_indexes.contains(&entry.id.table());
            if !is_in_indexed_table {
                log_document_skipped();
            }
            future::ready(tables_with_indexes.contains(&entry.id.table()))
        });
    stream_revision_pairs(document_stream, persistence)
}

impl<RT: Runtime> SearchIndexBootstrapWorker<RT> {
    pub(crate) fn new(
        runtime: RT,
        index_registry: IndexRegistry,
        persistence: RepeatablePersistence,
        table_mapping: NamespacedTableMapping,
        committer_client: CommitterClient,
    ) -> Self {
        Self {
            runtime,
            index_registry,
            table_mapping,
            persistence,
            committer_client,
            backoff: Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF),
        }
    }

    pub async fn start(mut self) {
        let timer = crate::metrics::search_and_vector_bootstrap_timer();
        loop {
            if let Err(e) = self.run().await {
                let delay = self.backoff.fail(&mut self.runtime.rng());
                // Forgive OCC errors < N to match UDF mutation retry behavior.
                if !e.is_occ() || (self.backoff.failures() as usize) > *UDF_EXECUTOR_OCC_MAX_RETRIES
                {
                    report_error(&mut e.context("SearchAndVectorBootstrapWorker died")).await;
                    tracing::error!(
                        "SearchIndexBoostrapWorker died, num_failures: {}. Backing off for {}ms",
                        self.backoff.failures(),
                        delay.as_millis()
                    );
                } else {
                    tracing::trace!(
                        "SearchIndexBoostrapWorker occed, retrying. num_failures: {}, backoff: \
                         {}ms",
                        self.backoff.failures(),
                        delay.as_millis(),
                    )
                }
                self.runtime.wait(delay).await;
            } else {
                tracing::info!("SearchIndexBoostrapWorker finished!");
                break;
            }
        }
        timer.finish();
    }

    async fn run(&mut self) -> anyhow::Result<()> {
        let bootstrapped_indexes = self.bootstrap().await?;
        let pause_client = self.runtime.pause_client();
        pause_client.wait(FINISHED_BOOTSTRAP_UPDATES).await;
        self.committer_client
            .finish_search_and_vector_bootstrap(
                bootstrapped_indexes,
                self.persistence.upper_bound(),
            )
            .await
    }

    async fn bootstrap(&self) -> anyhow::Result<BootstrappedSearchIndexes> {
        // Load all of the fast forward timestamps first to ensure that we stay within
        // the comparatively short valid time for the persistence snapshot
        let snapshot = self
            .persistence
            .read_snapshot(self.persistence.upper_bound())?;
        let registry = self.index_registry.clone();
        let table_mapping = self.table_mapping.clone();
        let get_index_futs = self
            .index_registry
            .all_search_and_vector_indexes()
            .into_iter()
            .map(move |index| {
                let registry = registry.clone();
                let table_mapping = table_mapping.clone();
                let snapshot = snapshot.clone();
                async move {
                    let fast_forward_ts = load_metadata_fast_forward_ts(
                        &registry,
                        &snapshot,
                        &table_mapping,
                        index.id(),
                    )
                    .await?;
                    anyhow::Ok((index, fast_forward_ts))
                }
            });
        let indexes_with_fast_forward_ts =
            try_join_buffer_unordered("get_index_futs", get_index_futs).await?;
        let indexes_to_bootstrap = IndexesToBootstrap::create(
            self.persistence.upper_bound(),
            indexes_with_fast_forward_ts,
        )?;
        indexes_to_bootstrap.bootstrap(&self.persistence).await
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::Arc,
        time::Duration,
    };

    use common::{
        bootstrap_model::index::{
            text_index::TextIndexState,
            IndexConfig,
            IndexMetadata,
            TabletIndexMetadata,
        },
        components::ComponentId,
        document::ParsedDocument,
        persistence::{
            NoopRetentionValidator,
            PersistenceReader,
            RepeatablePersistence,
        },
        runtime::Runtime,
        types::{
            IndexDescriptor,
            IndexId,
            IndexName,
            WriteTimestamp,
        },
    };
    use keybroker::Identity;
    use maplit::btreeset;
    use must_let::must_let;
    use runtime::testing::TestRuntime;
    use search::TextIndex;
    use storage::Storage;
    use sync_types::Timestamp;
    use value::{
        assert_obj,
        ConvexValue,
        DeveloperDocumentId,
        FieldPath,
        InternalId,
        ResolvedDocumentId,
        TableName,
        TableNamespace,
    };
    use vector::{
        PublicVectorSearchQueryResult,
        VectorSearch,
    };

    use crate::{
        bootstrap_model::index_workers::IndexWorkerMetadataModel,
        search_index_workers::fast_forward::load_metadata_fast_forward_ts,
        test_helpers::{
            index_utils::assert_enabled,
            DbFixtures,
            DbFixturesArgs,
        },
        text_index_worker::flusher::new_text_flusher_for_tests,
        vector_index_worker::flusher::backfill_vector_indexes,
        Database,
        IndexModel,
        SystemMetadataModel,
        TableModel,
        TestFacingModel,
        Transaction,
        UserFacingModel,
    };

    #[convex_macro::test_runtime]
    async fn persisted_vectors_are_not_included(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = DbFixtures::new(&rt).await?;
        let (_, index_metadata) = add_and_enable_vector_index(
            &rt,
            &fixtures.db,
            fixtures.tp.reader(),
            fixtures.search_storage.clone(),
        )
        .await?;

        let db = reopen_db(&rt, &fixtures).await?;
        add_vector(&db, &index_metadata, [1f32, 2f32]).await?;
        backfill_vector_indexes(
            rt.clone(),
            db.clone(),
            fixtures.tp.reader(),
            fixtures.search_storage,
        )
        .await?;

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
        expected: DeveloperDocumentId,
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
        let (_, index_metadata) = add_and_enable_vector_index(
            &rt,
            &fixtures.db,
            fixtures.tp.reader(),
            fixtures.search_storage.clone(),
        )
        .await?;

        let db = reopen_db(&rt, &fixtures).await?;
        let vector_id = add_vector(&db, &index_metadata, [1f32, 2f32]).await?;

        let result = query_vectors(&db, &index_metadata).await?;
        assert_expected_vector(result, vector_id);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn vector_added_before_bootstrap_is_included(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = DbFixtures::new(&rt).await?;
        let (_, index_metadata) = add_and_enable_vector_index(
            &rt,
            &fixtures.db,
            fixtures.tp.reader(),
            fixtures.search_storage.clone(),
        )
        .await?;

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
        let (index_id, index_metadata) = add_and_enable_vector_index(
            &rt,
            &fixtures.db,
            fixtures.tp.reader(),
            fixtures.search_storage.clone(),
        )
        .await?;

        add_vector(&fixtures.db, &index_metadata, [1f32, 2f32]).await?;
        let mut tx = fixtures.db.begin_system().await?;
        let mut model = IndexWorkerMetadataModel::new(&mut tx);
        let (metadata_id, mut metadata) = model
            .get_or_create_text_search(index_id)
            .await?
            .into_id_and_value();
        *metadata.index_metadata.mut_fast_forward_ts() = Timestamp::MAX.pred().unwrap();
        SystemMetadataModel::new_global(&mut tx)
            .replace(metadata_id, metadata.try_into()?)
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
        let (index_id, index_metadata) = add_and_backfill_index(
            &rt,
            &fixtures.db,
            fixtures.tp.reader(),
            fixtures.search_storage.clone(),
        )
        .await?;

        add_vector(&fixtures.db, &index_metadata, [1f32, 2f32]).await?;
        let mut tx = fixtures.db.begin_system().await?;
        let mut model = IndexWorkerMetadataModel::new(&mut tx);
        let (metadata_id, mut metadata) = model
            .get_or_create_text_search(index_id)
            .await?
            .into_id_and_value();
        *metadata.index_metadata.mut_fast_forward_ts() = Timestamp::MAX.pred().unwrap();
        SystemMetadataModel::new_global(&mut tx)
            .replace(metadata_id, metadata.try_into()?)
            .await?;
        fixtures.db.commit(tx).await?;

        // If we use the wrong timestamp here (e.g. MAX), then enabling this index will
        // fail because the memory snapshot will have a higher timestamp than
        // our index doc.
        let db = reopen_db(&rt, &fixtures).await?;
        let mut tx = db.begin_system().await?;
        IndexModel::new(&mut tx)
            .enable_index_for_testing(TableNamespace::test_user(), &index_metadata.name)
            .await?;
        db.commit(tx).await?;

        let result = query_vectors(&db, &index_metadata).await?;
        assert!(result.is_empty());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn vector_added_during_bootstrap_is_included(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = DbFixtures::new(&rt).await?;
        let (_, index_metadata) = add_and_enable_vector_index(
            &rt,
            &fixtures.db,
            fixtures.tp.reader(),
            fixtures.search_storage.clone(),
        )
        .await?;

        let db = reopen_db(&rt, &fixtures).await?;
        let worker = db.new_search_and_vector_bootstrap_worker_for_testing();

        let bootstrapped_indexes = worker.bootstrap().await?;
        let vector_id = add_vector(&db, &index_metadata, [3f32, 4f32]).await?;
        worker
            .committer_client
            .finish_search_and_vector_bootstrap(
                bootstrapped_indexes,
                worker.persistence.upper_bound(),
            )
            .await?;

        let result = query_vectors(&db, &index_metadata).await?;
        assert_expected_vector(result, vector_id);

        Ok(())
    }

    // This tests that when the timestamp range is (upper_bound exclusive,
    // upper_bound inclusive), bootstrapping doesn't panic.
    #[convex_macro::test_runtime]
    async fn bootstrap_just_backfilling_index(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = DbFixtures::new(&rt).await?;
        let index_metadata = backfilling_vector_index()?;
        let mut tx = fixtures.db.begin_system().await?;
        IndexModel::new(&mut tx)
            .add_application_index(TableNamespace::test_user(), index_metadata.clone())
            .await?;
        fixtures.db.commit(tx).await?;
        reopen_db(&rt, &fixtures).await?;
        Ok(())
    }

    async fn add_and_backfill_index(
        rt: &TestRuntime,
        db: &Database<TestRuntime>,
        reader: Arc<dyn PersistenceReader>,
        storage: Arc<dyn Storage>,
    ) -> anyhow::Result<(InternalId, IndexMetadata<TableName>)> {
        let index_metadata = backfilling_vector_index()?;
        let mut tx = db.begin_system().await?;
        IndexModel::new(&mut tx)
            .add_application_index(TableNamespace::test_user(), index_metadata.clone())
            .await?;
        db.commit(tx).await?;
        backfill_vector_indexes(rt.clone(), db.clone(), reader, storage.clone()).await?;
        let mut tx = db.begin_system().await?;
        let resolved_index = IndexModel::new(&mut tx)
            .pending_index_metadata(TableNamespace::test_user(), &index_metadata.name)?
            .expect("Missing index metadata!");

        Ok((resolved_index.id().internal_id(), index_metadata))
    }

    async fn add_and_enable_vector_index(
        rt: &TestRuntime,
        db: &Database<TestRuntime>,
        reader: Arc<dyn PersistenceReader>,
        storage: Arc<dyn Storage>,
    ) -> anyhow::Result<(InternalId, IndexMetadata<TableName>)> {
        let (_, index_metadata) = add_and_backfill_index(rt, db, reader, storage.clone()).await?;

        let mut tx = db.begin_system().await?;
        let resolved_index = IndexModel::new(&mut tx)
            .pending_index_metadata(TableNamespace::test_user(), &index_metadata.name)?
            .expect("Missing index metadata!");
        IndexModel::new(&mut tx)
            .enable_backfilled_indexes(vec![resolved_index.clone()])
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
            component_id: ComponentId::Root,
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
    ) -> anyhow::Result<DeveloperDocumentId> {
        add_vector_by_table(db, index_metadata.name.table().clone(), vector).await
    }

    async fn add_vector_by_table(
        db: &Database<TestRuntime>,
        table_name: TableName,
        vector: [f32; 2],
    ) -> anyhow::Result<DeveloperDocumentId> {
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
        let document_id = UserFacingModel::new_root_for_test(&mut tx)
            .insert(table_name, document)
            .await?;
        db.commit(tx).await?;
        Ok(document_id)
    }

    fn table() -> TableName {
        "table".parse().unwrap()
    }

    fn backfilling_vector_index() -> anyhow::Result<IndexMetadata<TableName>> {
        let index_name = IndexName::new(table(), IndexDescriptor::new("vector_index")?)?;
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

    #[convex_macro::test_runtime]
    async fn test_load_snapshot_without_fast_forward(rt: TestRuntime) -> anyhow::Result<()> {
        let db_fixtures = DbFixtures::new(&rt).await?;
        let db = &db_fixtures.db;
        let (index_id, _) = create_new_text_index(&rt, &db_fixtures).await?;

        let mut tx = db.begin_system().await.unwrap();
        add_document(
            &mut tx,
            &"test".parse()?,
            "hello world, this is a message with more than just a few terms in it",
        )
        .await?;
        db.commit(tx).await?;

        let db = reopen_db(&rt, &db_fixtures).await?;
        let snapshot = db.latest_snapshot()?;
        let indexes = snapshot.text_indexes.ready_indexes();

        let index = indexes.get(&index_id).unwrap();
        let TextIndex::Backfilled(snapshot) = index else {
            // Not using must_let because we don't implement Debug for this or nested
            // structs.
            panic!("Not backfilling?")
        };
        assert_eq!(snapshot.memory_index.num_transactions(), 1);

        Ok(())
    }
    #[convex_macro::test_runtime]
    async fn test_load_snapshot_with_fast_forward(rt: TestRuntime) -> anyhow::Result<()> {
        let db_fixtures = DbFixtures::new(&rt).await?;
        let db = &db_fixtures.db;
        let (index_id, _) = create_new_text_index(&rt, &db_fixtures).await?;

        rt.advance_time(Duration::from_secs(10)).await;

        let mut tx = db.begin_system().await.unwrap();
        add_document(
            &mut tx,
            &"test".parse()?,
            "hello world, this is a message with more than just a few terms in it",
        )
        .await?;
        db.commit(tx).await?;

        rt.advance_time(Duration::from_secs(10)).await;

        // We shouldn't ever fast forward across an update in real life, but doing so
        // and verifying we don't read the document is a simple way to verify we
        // actually use the fast forward timestamp.
        let mut tx = db.begin_system().await?;
        let mut model = IndexWorkerMetadataModel::new(&mut tx);
        let (metadata_id, mut metadata) = model
            .get_or_create_text_search(index_id)
            .await?
            .into_id_and_value();
        *metadata.index_metadata.mut_fast_forward_ts() = Timestamp::MAX.pred().unwrap();
        SystemMetadataModel::new_global(&mut tx)
            .replace(metadata_id, metadata.try_into()?)
            .await?;
        db.commit(tx).await?;

        let db = reopen_db(&rt, &db_fixtures).await?;
        let snapshot = db.latest_snapshot()?;
        let indexes = snapshot.text_indexes.ready_indexes();

        let index = indexes.get(&index_id).unwrap();
        let TextIndex::Backfilled(snapshot) = index else {
            panic!("Not backfilling?")
        };
        assert_eq!(snapshot.memory_index.num_transactions(), 0);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_load_snapshot_with_fast_forward_uses_disk_ts_for_memory_index(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let db_fixtures = DbFixtures::new(&rt).await?;
        let db = &db_fixtures.db;
        let (index_id, index_doc) = create_new_text_index(&rt, &db_fixtures).await?;

        // We shouldn't ever fast forward across an update in real life, but doing so
        // and verifying we don't read the document is a simple way to verify we
        // actually use the fast forward timestamp.
        let mut tx = db.begin_system().await?;
        let mut model = IndexWorkerMetadataModel::new(&mut tx);
        let (metadata_id, mut metadata) = model
            .get_or_create_text_search(index_id)
            .await?
            .into_id_and_value();
        *metadata.index_metadata.mut_fast_forward_ts() = Timestamp::MAX.pred().unwrap();
        SystemMetadataModel::new_global(&mut tx)
            .replace(metadata_id, metadata.try_into()?)
            .await?;
        db.commit(tx).await?;

        let db = reopen_db(&rt, &db_fixtures).await?;
        let snapshot = db.latest_snapshot()?;
        let indexes = snapshot.text_indexes.ready_indexes();

        // No must-let because SearchIndex doesn't implement Debug.
        let TextIndex::Backfilled(memory_snapshot) = indexes.get(&index_id).unwrap() else {
            anyhow::bail!("Unexpected index type");
        };
        must_let!(
            let IndexConfig::Text {
                on_disk_state: TextIndexState::Backfilled { snapshot: disk_snapshot, .. }, ..
            } = index_doc.into_value().config
        );

        assert_eq!(
            memory_snapshot.memory_index.min_ts(),
            WriteTimestamp::Committed(disk_snapshot.ts.succ().unwrap())
        );

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_load_fast_forward_ts(rt: TestRuntime) -> anyhow::Result<()> {
        let db_fixtures = DbFixtures::new(&rt).await?;
        let (index_id, index_doc) = create_new_text_index(&rt, &db_fixtures).await?;
        let db = db_fixtures.db;
        let tp = db_fixtures.tp;
        let mut tx = db.begin_system().await?;
        let mut model = IndexWorkerMetadataModel::new(&mut tx);
        let (metadata_id, mut metadata) = model
            .get_or_create_text_search(index_id)
            .await?
            .into_id_and_value();
        *metadata.index_metadata.mut_fast_forward_ts() = Timestamp::MAX;
        SystemMetadataModel::new_global(&mut tx)
            .replace(metadata_id, metadata.try_into()?)
            .await?;
        db.commit(tx).await?;

        let mut tx = db.begin_system().await?;
        let retention_validator = Arc::new(NoopRetentionValidator {});
        let persistence =
            RepeatablePersistence::new(tp.reader(), db.now_ts_for_reads(), retention_validator);
        let persistence_snapshot = persistence.read_snapshot(persistence.upper_bound())?;
        let snapshot = db.snapshot(db.now_ts_for_reads())?;

        let fast_forward_ts = load_metadata_fast_forward_ts(
            &snapshot.index_registry,
            &persistence_snapshot,
            &tx.table_mapping().namespace(TableNamespace::Global),
            index_doc.id(),
        )
        .await?;

        assert_eq!(fast_forward_ts, Some(Timestamp::MAX));

        Ok(())
    }
    #[convex_macro::test_runtime]
    async fn update_vector_memory_index_only_after_disk_ts(rt: TestRuntime) -> anyhow::Result<()> {
        let db_fixtures = DbFixtures::new(&rt).await?;
        let db = &db_fixtures.db;
        let search_storage = db_fixtures.search_storage.clone();
        // Add a search index at t0 to make bootstrapping start at t0
        create_new_text_index(&rt, &db_fixtures).await?;
        // Add a vector index to a table with a vector already in it
        add_vector_by_table(db, table(), [1f32, 2f32]).await?;
        add_and_enable_vector_index(&rt, db, db_fixtures.tp.reader(), search_storage).await?;
        // Bootstrap
        reopen_db(&rt, &db_fixtures).await?;
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn update_search_memory_index_only_after_disk_ts(rt: TestRuntime) -> anyhow::Result<()> {
        let db_fixtures = DbFixtures::new(&rt).await?;
        let db = &db_fixtures.db;
        let search_storage = db_fixtures.search_storage.clone();
        // Add vector index enabled at t0 to make bootstrapping start at t0
        add_and_enable_vector_index(&rt, db, db_fixtures.tp.reader(), search_storage.clone())
            .await?;
        // Add a new search index to a table with pre-existing documents
        let mut tx = db.begin_system().await?;
        add_document(
            &mut tx,
            &"test".parse()?,
            "hello world, this is a message with more than just a few terms in it",
        )
        .await?;
        db.commit(tx).await?;
        create_new_text_index(&rt, &db_fixtures).await?;
        // Bootstrap
        reopen_db(&rt, &db_fixtures).await?;
        Ok(())
    }

    async fn add_document(
        tx: &mut Transaction<TestRuntime>,
        table_name: &TableName,
        text: &str,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let document = assert_obj!(
            "text" => text,
        );
        TestFacingModel::new(tx).insert(table_name, document).await
    }

    async fn create_new_text_index<RT: Runtime>(
        rt: &RT,
        db_fixtures: &DbFixtures<RT>,
    ) -> anyhow::Result<(IndexId, ParsedDocument<TabletIndexMetadata>)> {
        let DbFixtures {
            tp,
            db,
            search_storage,
            build_index_args,
            ..
        } = db_fixtures;
        let table_name: TableName = "test".parse()?;
        let mut tx = db.begin_system().await?;
        TableModel::new(&mut tx)
            .insert_table_metadata_for_test(TableNamespace::test_user(), &table_name)
            .await?;
        let index = IndexMetadata::new_backfilling_text_index(
            "test.by_text".parse()?,
            "searchField".parse()?,
            btreeset! {"filterField".parse()?},
        );
        IndexModel::new(&mut tx)
            .add_application_index(TableNamespace::test_user(), index)
            .await?;
        db.commit(tx).await?;

        let flusher = new_text_flusher_for_tests(
            rt.clone(),
            db.clone(),
            tp.reader(),
            search_storage.clone(),
            build_index_args.segment_term_metadata_fetcher.clone(),
        );
        flusher.step().await?;

        let index_name = IndexName::new(table_name, IndexDescriptor::new("by_text")?)?;
        let mut tx = db.begin_system().await?;
        let mut model = IndexModel::new(&mut tx);
        let index_doc = model
            .pending_index_metadata(TableNamespace::test_user(), &index_name)?
            .unwrap();
        Ok((index_doc.id().internal_id(), index_doc))
    }
}
