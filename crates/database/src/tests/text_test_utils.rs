use std::sync::Arc;

use anyhow::Context;
use common::{
    bootstrap_model::index::{
        text_index::{
            FragmentedTextSegment,
            TextIndexSnapshot,
            TextIndexSnapshotData,
            TextIndexState,
        },
        IndexConfig,
        IndexMetadata,
        TabletIndexMetadata,
    },
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    persistence::PersistenceReader,
    query::{
        Query,
        QueryOperator,
        QuerySource,
        Search,
        SearchFilterExpression,
    },
    runtime::testing::TestRuntime,
    types::{
        GenericIndexName,
        IndexId,
        IndexName,
        TabletIndexName,
    },
    version::MIN_NPM_VERSION_FOR_FUZZY_SEARCH,
};
use maplit::btreeset;
use must_let::must_let;
use search::{
    searcher::InProcessSearcher,
    Searcher,
    SegmentTermMetadataFetcher,
    MAX_CANDIDATE_REVISIONS,
};
use storage::Storage;
use sync_types::Timestamp;
use value::{
    assert_obj,
    FieldPath,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

use crate::{
    index_workers::search_compactor::CompactionConfig,
    test_helpers::{
        DbFixtures,
        DbFixturesArgs,
    },
    text_index_worker::{
        compactor::{
            new_text_compactor_for_tests,
            TextIndexCompactor,
        },
        flusher2::{
            backfill_text_indexes,
            FlusherBuilder,
            TextIndexFlusher2,
        },
        TextIndexMetadataWriter,
    },
    Database,
    IndexModel,
    ResolvedQuery,
    TestFacingModel,
    TextIndexFlusher,
    Transaction,
};

pub struct TextFixtures {
    pub rt: TestRuntime,
    pub storage: Arc<dyn Storage>,
    pub db: Database<TestRuntime>,
    pub reader: Arc<dyn PersistenceReader>,
    searcher: Arc<dyn Searcher>,
    segment_term_metadata_fetcher: Arc<dyn SegmentTermMetadataFetcher>,
    writer: TextIndexMetadataWriter<TestRuntime>,
    namespace: TableNamespace,
    config: CompactionConfig,
}

impl TextFixtures {
    pub async fn new(rt: TestRuntime) -> anyhow::Result<Self> {
        Self::new_with_config(rt, CompactionConfig::default()).await
    }

    pub async fn new_with_config(
        rt: TestRuntime,
        config: CompactionConfig,
    ) -> anyhow::Result<Self> {
        let in_process_searcher = InProcessSearcher::new(rt.clone()).await?;
        let DbFixtures {
            tp,
            db,
            search_storage,
            searcher,
            ..
        } = DbFixtures::new_with_args(
            &rt,
            DbFixturesArgs {
                searcher: Some(Arc::new(in_process_searcher.clone())),
                ..Default::default()
            },
        )
        .await?;
        let segment_term_metadata_fetcher = Arc::new(in_process_searcher);
        let writer = TextIndexMetadataWriter::new(rt.clone(), db.clone(), search_storage.clone());

        Ok(Self {
            rt,
            db,
            reader: tp.reader(),
            storage: search_storage,
            segment_term_metadata_fetcher,
            writer,
            namespace: TableNamespace::test_user(),
            searcher,
            config,
        })
    }

    pub fn new_search_flusher(&self) -> TextIndexFlusher<TestRuntime> {
        TextIndexFlusher::new_with_soft_limit(
            self.rt.clone(),
            self.db.clone(),
            self.storage.clone(),
            2048,
        )
    }

    pub(crate) fn new_search_flusher_builder(&self) -> FlusherBuilder<TestRuntime> {
        FlusherBuilder::new(
            self.rt.clone(),
            self.db.clone(),
            self.reader.clone(),
            self.storage.clone(),
            self.segment_term_metadata_fetcher.clone(),
            self.writer.clone(),
        )
    }

    pub fn new_search_flusher2(&self) -> TextIndexFlusher2<TestRuntime> {
        self.new_search_flusher_builder().set_soft_limit(0).build()
    }

    pub fn new_compactor(&self) -> TextIndexCompactor<TestRuntime> {
        new_text_compactor_for_tests(
            self.rt.clone(),
            self.db.clone(),
            self.storage.clone(),
            self.searcher.clone(),
            self.config.clone(),
        )
    }

    pub async fn enabled_text_index(&self) -> anyhow::Result<IndexData> {
        let index_data = self.backfilled_text_index().await?;
        let mut tx = self.db.begin_system().await?;
        IndexModel::new(&mut tx)
            .enable_index_for_testing(index_data.namespace, &index_data.index_name)
            .await?;
        self.db.commit(tx).await?;
        Ok(index_data)
    }

    pub async fn backfilled_text_index(&self) -> anyhow::Result<IndexData> {
        let index_data = self.insert_backfilling_text_index().await?;
        self.backfill().await?;

        Ok(index_data)
    }

    pub async fn backfill(&self) -> anyhow::Result<()> {
        backfill_text_indexes(
            self.rt.clone(),
            self.db.clone(),
            self.reader.clone(),
            self.storage.clone(),
            self.segment_term_metadata_fetcher.clone(),
        )
        .await
    }

    pub async fn assert_backfilled(&self, index_name: &IndexName) -> anyhow::Result<Timestamp> {
        let mut tx = self.db.begin_system().await?;
        let new_metadata = IndexModel::new(&mut tx)
            .pending_index_metadata(self.namespace, index_name)?
            .context("Index missing or in an unexpected state")?
            .into_value();
        must_let!(let IndexMetadata {
            config: IndexConfig::Search {
                on_disk_state: TextIndexState::Backfilled(TextIndexSnapshot { ts, .. }),
                ..
            },
            ..
        } = new_metadata);
        Ok(ts)
    }

    pub async fn insert_backfilling_text_index(&self) -> anyhow::Result<IndexData> {
        let mut tx = self.db.begin_system().await?;
        let index_metadata = backfilling_text_index()?;
        let index_name = &index_metadata.name;
        let index_id = IndexModel::new(&mut tx)
            .add_application_index(index_metadata.clone())
            .await?;
        let table_id = tx
            .table_mapping()
            .namespace(self.namespace)
            .id(index_name.table())?
            .tablet_id;
        self.db.commit(tx).await?;

        let resolved_index_name = TabletIndexName::new(table_id, index_name.descriptor().clone())?;
        Ok(IndexData {
            index_id: index_id.internal_id(),
            resolved_index_name,
            index_name: index_name.clone(),
            namespace: self.namespace,
        })
    }

    pub async fn insert_backfilling_text_index_with_document(&self) -> anyhow::Result<IndexData> {
        let index_data = self.insert_backfilling_text_index().await?;
        let mut tx = self.db.begin_system().await?;
        add_document(&mut tx, index_data.index_name.table(), "A long text field").await?;
        self.db.commit(tx).await?;
        Ok(index_data)
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
    ) -> anyhow::Result<Vec<FragmentedTextSegment>> {
        let metadata = self.get_index_metadata(index_name).await?;
        must_let!(let IndexConfig::Search { on_disk_state, .. } = &metadata.config);
        let snapshot = match on_disk_state {
            TextIndexState::Backfilling(_) => anyhow::bail!("Still backfilling!"),
            TextIndexState::Backfilled(snapshot) | TextIndexState::SnapshottedAt(snapshot) => {
                snapshot
            },
        };
        must_let!(let TextIndexSnapshotData::MultiSegment(segments) = &snapshot.data);
        Ok(segments.clone())
    }

    pub async fn add_document(&self, text: &str) -> anyhow::Result<ResolvedDocumentId> {
        let table_name = TABLE_NAME.parse::<TableName>()?;
        let mut tx = self.db.begin_system().await?;
        let doc_id = add_document(&mut tx, &table_name, text).await?;
        self.db.commit(tx).await?;
        Ok(doc_id)
    }

    pub async fn enable_index(
        &self,
        index_name: &GenericIndexName<TableName>,
    ) -> anyhow::Result<()> {
        let mut tx = self.db.begin_system().await?;
        IndexModel::new(&mut tx)
            .enable_index_for_testing(self.namespace, index_name)
            .await?;
        self.db.commit(tx).await?;
        Ok(())
    }

    pub async fn search(
        &self,
        index_name: GenericIndexName<TableName>,
        query_string: &str,
    ) -> anyhow::Result<Vec<ResolvedDocument>> {
        let mut tx = self.db.begin_system().await?;
        let filters = vec![SearchFilterExpression::Search(
            SEARCH_FIELD.parse()?,
            query_string.into(),
        )];
        let search = Search {
            table: index_name.table().clone(),
            index_name,
            filters,
        };

        let query = Query {
            source: QuerySource::Search(search),
            operators: vec![QueryOperator::Limit(MAX_CANDIDATE_REVISIONS)],
        };
        let mut query_stream = ResolvedQuery::new_with_version(
            &mut tx,
            TableNamespace::test_user(),
            query,
            Some(MIN_NPM_VERSION_FOR_FUZZY_SEARCH.clone()),
        )?;
        let mut values = vec![];
        while let Some(value) = query_stream.next(&mut tx, None).await? {
            values.push(value);
        }
        Ok(values)
    }
}

const TABLE_NAME: &str = "table";
const SEARCH_FIELD: &str = "text";

pub struct IndexData {
    pub index_id: IndexId,
    pub index_name: IndexName,
    pub resolved_index_name: TabletIndexName,
    pub namespace: TableNamespace,
}

pub fn backfilling_text_index() -> anyhow::Result<IndexMetadata<TableName>> {
    let table_name: TableName = TABLE_NAME.parse()?;
    let index_name = IndexName::new(table_name, "search_index".parse()?)?;
    let search_field: FieldPath = SEARCH_FIELD.parse()?;
    let filter_field: FieldPath = "channel".parse()?;
    let metadata = IndexMetadata::new_backfilling_search_index(
        index_name,
        search_field,
        btreeset![filter_field],
    );
    Ok(metadata)
}

pub async fn add_document(
    tx: &mut Transaction<TestRuntime>,
    table_name: &TableName,
    text: &str,
) -> anyhow::Result<ResolvedDocumentId> {
    let document = assert_obj!(
        "text" => text,
        "channel" => "#general",
    );
    TestFacingModel::new(tx).insert(table_name, document).await
}
