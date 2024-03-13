use std::sync::Arc;

use anyhow::Context;
use common::{
    bootstrap_model::index::{
        search_index::{
            SearchIndexSnapshot,
            SearchIndexState,
        },
        IndexConfig,
        IndexMetadata,
    },
    runtime::testing::TestRuntime,
    types::{
        IndexId,
        IndexName,
        TabletIndexName,
    },
};
use maplit::btreeset;
use must_let::must_let;
use storage::LocalDirStorage;
use sync_types::Timestamp;
use value::{
    assert_obj,
    FieldPath,
    ResolvedDocumentId,
    TableName,
};

use crate::{
    Database,
    IndexModel,
    SearchIndexFlusher,
    TestFacingModel,
    Transaction,
};

pub(crate) struct IndexData {
    pub index_id: IndexId,
    pub index_name: IndexName,
    pub resolved_index_name: TabletIndexName,
}

pub(crate) fn new_search_worker(
    rt: &TestRuntime,
    database: &Database<TestRuntime>,
) -> anyhow::Result<SearchIndexFlusher<TestRuntime>> {
    let storage = LocalDirStorage::new(rt.clone())?;
    Ok(SearchIndexFlusher::new_with_soft_limit(
        rt.clone(),
        database.clone(),
        Arc::new(storage),
        2048,
    ))
}

pub(crate) fn backfilling_search_index() -> anyhow::Result<IndexMetadata<TableName>> {
    let table_name: TableName = "table".parse()?;
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

pub(crate) async fn assert_backfilled(
    database: &Database<TestRuntime>,
    index_name: &IndexName,
) -> anyhow::Result<Timestamp> {
    let mut tx = database.begin_system().await?;
    let new_metadata = IndexModel::new(&mut tx)
        .pending_index_metadata(index_name)?
        .context("Index missing or in an unexpected state")?
        .into_value();
    must_let!(let IndexMetadata {
            config: IndexConfig::Search {
                on_disk_state: SearchIndexState::Backfilled(SearchIndexSnapshot { ts, .. }),
                ..
            },
            ..
        } = new_metadata);
    Ok(ts)
}

pub(crate) async fn add_document(
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

pub(crate) async fn create_search_index_with_document(
    db: &Database<TestRuntime>,
) -> anyhow::Result<IndexData> {
    let index_metadata = backfilling_search_index()?;
    let index_name = &index_metadata.name;
    let mut tx = db.begin_system().await?;
    let index_id = IndexModel::new(&mut tx)
        .add_application_index(index_metadata.clone())
        .await?;
    add_document(&mut tx, index_name.table(), "A long text field").await?;
    let table_id = tx.table_mapping().id(index_name.table())?.table_id;
    db.commit(tx).await?;

    let resolved_index_name = TabletIndexName::new(table_id, index_name.descriptor().clone())?;
    Ok(IndexData {
        index_id: index_id.internal_id(),
        resolved_index_name,
        index_name: index_name.clone(),
    })
}
