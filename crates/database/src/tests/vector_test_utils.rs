use std::sync::Arc;

use anyhow::Context;
use async_trait::async_trait;
use common::{
    bootstrap_model::index::{
        text_index::FragmentedTextSegment,
        vector_index::{
            FragmentedVectorSegment,
            VectorIndexBackfillState,
            VectorIndexSnapshot,
            VectorIndexSnapshotData,
            VectorIndexState,
        },
        IndexConfig,
        IndexMetadata,
        TabletIndexMetadata,
    },
    document::ParsedDocument,
    knobs::{
        MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
        VECTOR_INDEX_SIZE_SOFT_LIMIT,
    },
    pause::PauseController,
    persistence::PersistenceReader,
    runtime::Runtime,
    types::{
        GenericIndexName,
        IndexDescriptor,
        IndexName,
        TabletIndexName,
    },
};
use events::testing::TestUsageEventLogger;
use futures::try_join;
use maplit::btreeset;
use must_let::must_let;
use pb::searchlight::FragmentedVectorSegmentPaths;
use qdrant_segment::{
    segment::Segment,
    types::VECTOR_ELEMENT_SIZE,
};
use runtime::testing::TestRuntime;
use search::{
    disk_index::{
        download_single_file_original,
        download_single_file_zip,
    },
    searcher::{
        Bm25Stats,
        FragmentedTextStorageKeys,
        InProcessSearcher,
        PostingListMatch,
        PostingListQuery,
        Searcher,
        Term,
        TokenMatch,
        TokenQuery,
    },
};
use storage::Storage;
use sync_types::Timestamp;
use tempfile::TempDir;
use value::{
    assert_obj,
    ConvexValue,
    FieldPath,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};
use vector::{
    qdrant_segments::{
        unsafe_load_disk_segment,
        VectorDiskSegmentPaths,
    },
    CompiledVectorSearch,
    QdrantSchema,
    VectorSearchQueryResult,
    VectorSearcher,
};

use super::DbFixtures;
use crate::{
    index_workers::{
        search_compactor::CompactionConfig,
        search_flusher::FLUSH_RUNNING_LABEL,
    },
    test_helpers::DbFixturesArgs,
    vector_index_worker::{
        compactor::{
            new_vector_compactor_for_tests,
            VectorIndexCompactor,
        },
        flusher::{
            backfill_vector_indexes,
            new_vector_flusher_for_tests,
            VectorIndexFlusher,
        },
    },
    Database,
    IndexModel,
    TestFacingModel,
    Transaction,
    UserFacingModel,
};

pub struct VectorFixtures {
    pub rt: TestRuntime,
    pub storage: Arc<dyn Storage>,
    pub db: Database<TestRuntime>,
    pub reader: Arc<dyn PersistenceReader>,
    searcher: Arc<dyn Searcher>,
    config: CompactionConfig,
    namespace: TableNamespace,
    pub test_usage_logger: TestUsageEventLogger,
}

/// The size of the vectors these fixtures use [f32; 2]. We actually require f64
/// arrays, but these are converted to f32 before use.
pub const VECTOR_SIZE_BYTES: u64 = 2 * VECTOR_ELEMENT_SIZE as u64;

impl VectorFixtures {
    pub async fn new(rt: TestRuntime) -> anyhow::Result<Self> {
        Self::new_with_config(rt, CompactionConfig::default()).await
    }

    pub(crate) async fn new_with_config(
        rt: TestRuntime,
        config: CompactionConfig,
    ) -> anyhow::Result<Self> {
        let DbFixtures {
            tp,
            db,
            searcher,
            search_storage,
            test_usage_logger,
            ..
        } = DbFixtures::new_with_args(
            &rt,
            DbFixturesArgs {
                searcher: Some(Arc::new(InProcessSearcher::new(rt.clone()).await?)),
                ..Default::default()
            },
        )
        .await?;

        Ok(Self {
            rt,
            db,
            reader: tp.reader(),
            storage: search_storage,
            searcher,
            config,
            namespace: TableNamespace::test_user(),
            test_usage_logger,
        })
    }

    pub async fn backfill(&self) -> anyhow::Result<()> {
        backfill_vector_indexes(
            self.rt.clone(),
            self.db.clone(),
            self.reader.clone(),
            self.storage.clone(),
        )
        .await
    }

    pub async fn enabled_vector_index(&self) -> anyhow::Result<IndexData> {
        let index_data = backfilled_vector_index(
            self.rt.clone(),
            self.db.clone(),
            self.reader.clone(),
            self.storage.clone(),
        )
        .await?;
        let mut tx = self.db.begin_system().await?;
        IndexModel::new(&mut tx)
            .enable_index_for_testing(index_data.namespace, &index_data.index_name)
            .await?;
        self.db.commit(tx).await?;
        Ok(index_data)
    }

    pub async fn backfilled_vector_index(&self) -> anyhow::Result<IndexData> {
        backfilled_vector_index(
            self.rt.clone(),
            self.db.clone(),
            self.reader.clone(),
            self.storage.clone(),
        )
        .await
    }

    pub async fn backfilled_vector_index_with_doc(&self) -> anyhow::Result<IndexData> {
        backfilled_vector_index_with_doc(
            self.rt.clone(),
            self.db.clone(),
            self.reader.clone(),
            self.storage.clone(),
        )
        .await
    }

    pub async fn backfilling_vector_index_with_doc(&self) -> anyhow::Result<IndexData> {
        backfilling_vector_index_with_doc(&self.db).await
    }

    pub async fn backfilling_vector_index(&self) -> anyhow::Result<IndexData> {
        backfilling_vector_index(&self.db).await
    }

    pub async fn add_document_vec_array(
        &self,
        table_name: &TableName,
        vector: [f64; 2],
    ) -> anyhow::Result<ResolvedDocumentId> {
        let mut tx = self.db.begin_system().await?;
        let result = add_document_vec_array(&mut tx, table_name, vector).await?;
        self.db.commit(tx).await?;
        Ok(result)
    }

    pub async fn new_compactor(&self) -> anyhow::Result<VectorIndexCompactor<TestRuntime>> {
        self.new_compactor_with_searchlight(self.searcher.clone())
            .await
    }

    pub async fn new_compactor_delete_on_compact(
        &self,
        id_to_delete: ResolvedDocumentId,
    ) -> anyhow::Result<VectorIndexCompactor<TestRuntime>> {
        let searcher = DeleteOnCompactSearchlight {
            rt: self.rt.clone(),
            db: self.db.clone(),
            reader: self.reader.clone(),
            searcher: self.searcher.clone(),
            to_delete: id_to_delete,
            storage: self.storage.clone(),
        };

        self.new_compactor_with_searchlight(Arc::new(searcher))
            .await
    }

    async fn new_compactor_with_searchlight(
        &self,
        searcher: Arc<dyn Searcher>,
    ) -> anyhow::Result<VectorIndexCompactor<TestRuntime>> {
        Ok(new_vector_compactor_for_tests(
            self.rt.clone(),
            self.db.clone(),
            self.reader.clone(),
            self.storage.clone(),
            searcher,
            self.config.clone(),
        ))
    }

    pub fn new_index_flusher(&self) -> anyhow::Result<VectorIndexFlusher<TestRuntime>> {
        self.new_index_flusher_with_full_scan_threshold(*MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB)
    }

    pub async fn run_compaction_during_flush(&self, pause: PauseController) -> anyhow::Result<()> {
        let mut flusher = new_vector_flusher_for_tests(
            self.rt.clone(),
            self.db.clone(),
            self.reader.clone(),
            self.storage.clone(),
            // Force indexes to always be built.
            0,
            *MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
            8,
        );
        let hold_guard = pause.hold(FLUSH_RUNNING_LABEL);
        let flush = flusher.step();
        let compactor = self.new_compactor().await?;
        let compact_during_flush = async move {
            if let Some(pause_guard) = hold_guard.wait_for_blocked().await {
                compactor.step().await?;
                pause_guard.unpause();
            };
            Ok::<(), anyhow::Error>(())
        };
        try_join!(flush, compact_during_flush)?;
        Ok(())
    }

    pub fn new_index_flusher_with_full_scan_threshold(
        &self,
        full_scan_threshold_kb: usize,
    ) -> anyhow::Result<VectorIndexFlusher<TestRuntime>> {
        Ok(new_vector_flusher_for_tests(
            self.rt.clone(),
            self.db.clone(),
            self.reader.clone(),
            self.storage.clone(),
            // Force indexes to always be built.
            0,
            full_scan_threshold_kb,
            *VECTOR_INDEX_SIZE_SOFT_LIMIT,
        ))
    }

    pub fn new_index_flusher_with_incremental_part_threshold(
        &self,
        incremental_part_threshold: usize,
    ) -> anyhow::Result<VectorIndexFlusher<TestRuntime>> {
        Ok(new_vector_flusher_for_tests(
            self.rt.clone(),
            self.db.clone(),
            self.reader.clone(),
            self.storage.clone(),
            // Force indexes to always be built.
            0,
            *MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
            incremental_part_threshold,
        ))
    }

    pub async fn get_index_metadata(
        &self,
        index_name: GenericIndexName<TableName>,
    ) -> anyhow::Result<ParsedDocument<TabletIndexMetadata>> {
        let mut tx = self.db.begin_system().await?;
        let mut index_model = IndexModel::new(&mut tx);
        let mut metadata = index_model.enabled_index_metadata(self.namespace, &index_name)?;
        if metadata.is_none() {
            metadata = index_model.pending_index_metadata(self.namespace, &index_name)?;
        }
        let metadata = metadata.context("Index is neither pending nor enabled!")?;
        Ok(metadata)
    }

    pub async fn get_segments_metadata(
        &self,
        index_name: GenericIndexName<TableName>,
    ) -> anyhow::Result<Vec<FragmentedVectorSegment>> {
        let metadata = self.get_index_metadata(index_name).await?;
        must_let!(let IndexConfig::Vector { on_disk_state, .. } = &metadata.config);
        let snapshot = match on_disk_state {
            VectorIndexState::Backfilling(_) => anyhow::bail!("Still backfilling!"),
            VectorIndexState::Backfilled(snapshot) | VectorIndexState::SnapshottedAt(snapshot) => {
                snapshot
            },
        };
        must_let!(let VectorIndexSnapshotData::MultiSegment(segments) = &snapshot.data);
        Ok(segments.clone())
    }

    pub async fn get_segments_from_backfilling_index(
        &self,
        index_name: GenericIndexName<TableName>,
    ) -> anyhow::Result<Vec<FragmentedVectorSegment>> {
        let metadata = self.get_index_metadata(index_name).await?;
        must_let!(let IndexConfig::Vector { on_disk_state, .. } = &metadata.config);
        must_let!(let VectorIndexState::Backfilling(VectorIndexBackfillState
            {
                segments,
                ..
            }) = on_disk_state);
        Ok(segments.clone())
    }

    pub async fn load_segment(&self, segment: &FragmentedVectorSegment) -> anyhow::Result<Segment> {
        let tmp_dir = TempDir::new()?;
        let segment_path = tmp_dir.path().join("segment_tmp.tar");
        download_single_file_original(
            &segment.segment_key,
            segment_path.clone(),
            self.storage.clone(),
        )
        .await?;
        let bitset_path = tmp_dir.path().join("bitset_tmp");
        download_single_file_zip(
            &segment.deleted_bitset_key,
            bitset_path.clone(),
            self.storage.clone(),
        )
        .await?;
        let id_tracker_path = tmp_dir.path().join("id_tracker_tmp");
        download_single_file_zip(
            &segment.id_tracker_key,
            id_tracker_path.clone(),
            self.storage.clone(),
        )
        .await?;
        // Our usage is safe here because we're always fetching the data into
        // a new temp dir, so it's not possible to load a segment from these
        // specific paths multiple times.
        unsafe_load_disk_segment(&VectorDiskSegmentPaths {
            segment: segment_path,
            uuids: id_tracker_path,
            deleted_bitset: bitset_path,
        })
        .await
    }
}

pub async fn add_document_vec_array(
    tx: &mut Transaction<TestRuntime>,
    table_name: &TableName,
    vector: [f64; 2],
) -> anyhow::Result<ResolvedDocumentId> {
    let values: Vec<ConvexValue> = vector.into_iter().map(ConvexValue::Float64).collect();
    add_document_vec(tx, table_name, values).await
}

pub async fn add_document_vec(
    tx: &mut Transaction<TestRuntime>,
    table_name: &TableName,
    vector: Vec<ConvexValue>,
) -> anyhow::Result<ResolvedDocumentId> {
    add_document_with_value(tx, table_name, ConvexValue::Array(vector.try_into()?)).await
}

pub async fn add_document_with_value(
    tx: &mut Transaction<TestRuntime>,
    table_name: &TableName,
    value: ConvexValue,
) -> anyhow::Result<ResolvedDocumentId> {
    let document = assert_obj!(
        "vector" => value,
        "channel" => "#general",
    );
    TestFacingModel::new(tx).insert(table_name, document).await
}

pub struct IndexData {
    pub index_id: ResolvedDocumentId,
    pub index_name: IndexName,
    pub resolved_index_name: TabletIndexName,
    pub namespace: TableNamespace,
}

fn new_backfilling_vector_index() -> anyhow::Result<IndexMetadata<TableName>> {
    let table_name: TableName = "table".parse()?;
    let index_name = IndexName::new(table_name, IndexDescriptor::new("vector_index")?)?;
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

pub async fn backfilled_vector_index(
    rt: TestRuntime,
    db: Database<TestRuntime>,
    reader: Arc<dyn PersistenceReader>,
    storage: Arc<dyn Storage>,
) -> anyhow::Result<IndexData> {
    let index_data = backfilling_vector_index(&db).await?;
    backfill_vector_indexes(rt, db.clone(), reader, storage).await?;

    Ok(index_data)
}

pub async fn backfilling_vector_index(db: &Database<TestRuntime>) -> anyhow::Result<IndexData> {
    let index_metadata = new_backfilling_vector_index()?;
    let index_name = &index_metadata.name;
    let mut tx = db.begin_system().await?;
    let namespace = TableNamespace::test_user();
    let index_id = IndexModel::new(&mut tx)
        .add_application_index(namespace, index_metadata.clone())
        .await?;
    let table_id = tx
        .table_mapping()
        .namespace(namespace)
        .id(index_name.table())?
        .tablet_id;
    db.commit(tx).await?;
    let resolved_index_name = TabletIndexName::new(table_id, index_name.descriptor().clone())?;
    Ok(IndexData {
        index_id,
        resolved_index_name,
        index_name: index_name.clone(),
        namespace,
    })
}

pub async fn backfilling_vector_index_with_doc(
    db: &Database<TestRuntime>,
) -> anyhow::Result<IndexData> {
    let index_data = backfilling_vector_index(db).await?;
    let mut tx = db.begin_system().await?;
    add_document_vec_array(&mut tx, index_data.index_name.table(), [1f64, 2f64]).await?;
    db.commit(tx).await?;

    Ok(index_data)
}

pub async fn backfilled_vector_index_with_doc(
    rt: TestRuntime,
    db: Database<TestRuntime>,
    reader: Arc<dyn PersistenceReader>,
    storage: Arc<dyn Storage>,
) -> anyhow::Result<IndexData> {
    let result = backfilled_vector_index(rt, db.clone(), reader, storage).await?;
    let mut tx = db.begin_system().await?;
    add_document_vec_array(&mut tx, result.index_name.table(), [1f64, 2f64]).await?;
    db.commit(tx).await?;

    Ok(result)
}

pub(crate) async fn assert_backfilled(
    database: &Database<TestRuntime>,
    namespace: TableNamespace,
    index_name: &IndexName,
) -> anyhow::Result<Timestamp> {
    let mut tx = database.begin_system().await?;
    let new_metadata = IndexModel::new(&mut tx)
        .pending_index_metadata(namespace, index_name)?
        .context("Index missing or in an unexpected state")?
        .into_value();
    must_let!(let IndexMetadata {
            config: IndexConfig::Vector {
                on_disk_state: VectorIndexState::Backfilled(VectorIndexSnapshot { ts, .. }),
                ..
            },
            ..
        } = new_metadata);
    Ok(ts)
}

/// A hack that lets us racily delete vectors during compaction by accepting
/// a specific document id of a vector to delete when
/// execute_vector_compaction is called.
///
/// All functionality comes from the delegated searcher.
#[derive(Clone)]
struct DeleteOnCompactSearchlight<RT: Runtime> {
    rt: RT,
    searcher: Arc<dyn Searcher>,
    db: Database<RT>,
    reader: Arc<dyn PersistenceReader>,
    storage: Arc<dyn Storage>,
    to_delete: ResolvedDocumentId,
}

#[async_trait]
impl<RT: Runtime> Searcher for DeleteOnCompactSearchlight<RT> {
    async fn query_tokens(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_keys: FragmentedTextStorageKeys,
        queries: Vec<TokenQuery>,
        max_results: usize,
    ) -> anyhow::Result<Vec<TokenMatch>> {
        self.searcher
            .query_tokens(search_storage, storage_keys, queries, max_results)
            .await
    }

    async fn query_bm25_stats(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_keys: FragmentedTextStorageKeys,
        terms: Vec<Term>,
    ) -> anyhow::Result<Bm25Stats> {
        self.searcher
            .query_bm25_stats(search_storage, storage_keys, terms)
            .await
    }

    async fn query_posting_lists(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_keys: FragmentedTextStorageKeys,
        query: PostingListQuery,
    ) -> anyhow::Result<Vec<PostingListMatch>> {
        self.searcher
            .query_posting_lists(search_storage, storage_keys, query)
            .await
    }

    async fn execute_text_compaction(
        &self,
        search_storage: Arc<dyn Storage>,
        segments: Vec<FragmentedTextStorageKeys>,
    ) -> anyhow::Result<FragmentedTextSegment> {
        self.searcher
            .execute_text_compaction(search_storage, segments)
            .await
    }
}

#[async_trait]
impl<RT: Runtime> VectorSearcher for DeleteOnCompactSearchlight<RT> {
    async fn execute_multi_segment_vector_query(
        &self,
        search_storage: Arc<dyn Storage>,
        segments: Vec<FragmentedVectorSegmentPaths>,
        schema: QdrantSchema,
        search: CompiledVectorSearch,
        overfetch_delta: u32,
    ) -> anyhow::Result<Vec<VectorSearchQueryResult>> {
        self.searcher
            .execute_multi_segment_vector_query(
                search_storage,
                segments,
                schema,
                search,
                overfetch_delta,
            )
            .await
    }

    async fn execute_vector_compaction(
        &self,
        search_storage: Arc<dyn Storage>,
        segments: Vec<pb::searchlight::FragmentedVectorSegmentPaths>,
        dimension: usize,
    ) -> anyhow::Result<FragmentedVectorSegment> {
        let mut tx: Transaction<RT> = self.db.begin_system().await?;
        UserFacingModel::new_root_for_test(&mut tx)
            .delete(self.to_delete.into())
            .await?;
        self.db.commit(tx).await?;
        backfill_vector_indexes(
            self.rt.clone(),
            self.db.clone(),
            self.reader.clone(),
            self.storage.clone(),
        )
        .await?;

        self.searcher
            .execute_vector_compaction(search_storage, segments, dimension)
            .await
    }
}
