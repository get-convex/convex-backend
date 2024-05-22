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
    document::ParsedDocument,
    persistence::PersistenceReader,
    runtime::testing::TestRuntime,
    types::{
        GenericIndexName,
        IndexId,
        IndexName,
        TabletIndexName,
    },
};
use maplit::btreeset;
use must_let::must_let;
use search::searcher::InProcessSearcher;
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
    test_helpers::{
        DbFixtures,
        DbFixturesArgs,
    },
    text_index_worker::flusher2::{
        FlusherBuilder,
        TextIndexFlusher2,
    },
    Database,
    IndexModel,
    TestFacingModel,
    TextIndexFlusher,
    Transaction,
};

pub struct TextFixtures {
    pub rt: TestRuntime,
    pub storage: Arc<dyn Storage>,
    pub db: Database<TestRuntime>,
    pub reader: Arc<dyn PersistenceReader>,
    namespace: TableNamespace,
}

impl TextFixtures {
    pub async fn new(rt: TestRuntime) -> anyhow::Result<Self> {
        let DbFixtures {
            tp,
            db,
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

        Ok(Self {
            rt,
            db,
            reader: tp.reader(),
            storage: search_storage,
            namespace: TableNamespace::Global,
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
        )
    }

    pub fn new_search_flusher2(&self) -> TextIndexFlusher2<TestRuntime> {
        self.new_search_flusher_builder().set_soft_limit(0).build()
    }

    pub async fn insert_backfilling_text_index(&self) -> anyhow::Result<IndexMetadata<TableName>> {
        let mut tx = self.db.begin_system().await?;
        let index_metadata = backfilling_text_index()?;
        IndexModel::new(&mut tx)
            .add_application_index(index_metadata.clone())
            .await?;
        self.db.commit(tx).await?;
        Ok(index_metadata)
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

    pub async fn insert_backfilling_text_index_with_document(&self) -> anyhow::Result<IndexData> {
        let index_metadata = backfilling_text_index()?;
        let index_name = &index_metadata.name;
        let mut tx = self.db.begin_system().await?;
        let index_id = IndexModel::new(&mut tx)
            .add_application_index(index_metadata.clone())
            .await?;
        add_document(&mut tx, index_name.table(), "A long text field").await?;
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

    pub async fn add_document(&self, text: &str) -> anyhow::Result<()> {
        let table_name = TABLE_NAME.parse::<TableName>()?;
        let mut tx = self.db.begin_system().await?;
        add_document(&mut tx, &table_name, text).await?;
        self.db.commit(tx).await?;
        Ok(())
    }
}

const TABLE_NAME: &str = "table";

pub struct IndexData {
    pub index_id: IndexId,
    pub index_name: IndexName,
    pub resolved_index_name: TabletIndexName,
    pub namespace: TableNamespace,
}

pub fn backfilling_text_index() -> anyhow::Result<IndexMetadata<TableName>> {
    let table_name: TableName = TABLE_NAME.parse()?;
    let index_name = IndexName::new(table_name, "search_index".parse()?)?;
    let search_field: FieldPath = "text".parse()?;
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
