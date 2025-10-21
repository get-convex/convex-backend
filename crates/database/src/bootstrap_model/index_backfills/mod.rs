use std::sync::{
    Arc,
    LazyLock,
};

use common::{
    document::{
        ParsedDocument,
        CREATION_TIME_FIELD_PATH,
    },
    runtime::Runtime,
    types::IndexId,
};
use sync_types::Timestamp;
use value::{
    DeveloperDocumentId,
    FieldPath,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
    TabletId,
};

use crate::{
    bootstrap_model::index_backfills::types::{
        BackfillCursor,
        IndexBackfillMetadata,
    },
    system_tables::{
        SystemIndex,
        SystemTable,
    },
    SystemMetadataModel,
    Transaction,
};

pub mod types;

#[cfg(test)]
mod tests;

pub static INDEX_BACKFILLS_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_index_backfills"
        .parse()
        .expect("Invalid built-in index_backfills table")
});

pub static INDEX_BACKFILLS_BY_INDEX_ID: LazyLock<SystemIndex<IndexBackfillTable>> =
    LazyLock::new(|| {
        SystemIndex::new("by_index_id", [&INDEX_ID_FIELD, &CREATION_TIME_FIELD_PATH]).unwrap()
    });

static INDEX_ID_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "indexId".parse().expect("invalid indexId field"));

pub struct IndexBackfillTable;

impl SystemTable for IndexBackfillTable {
    type Metadata = types::IndexBackfillMetadata;

    fn table_name() -> &'static TableName {
        &INDEX_BACKFILLS_TABLE
    }

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![INDEX_BACKFILLS_BY_INDEX_ID.clone()]
    }
}

pub struct IndexBackfillModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> IndexBackfillModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    fn index_id_as_developer_id(&mut self, index_id: IndexId) -> DeveloperDocumentId {
        let index_table_id = self.tx.bootstrap_tables().index_id;
        DeveloperDocumentId::new(index_table_id.table_number, index_id)
    }

    pub(crate) async fn existing_backfill_metadata(
        &mut self,
        index_id: DeveloperDocumentId,
    ) -> anyhow::Result<Option<Arc<ParsedDocument<IndexBackfillMetadata>>>> {
        self.tx
            .query_system(TableNamespace::Global, &*INDEX_BACKFILLS_BY_INDEX_ID)?
            .eq(&[index_id.encode_into(&mut Default::default())])?
            .unique()
            .await
    }

    pub async fn initialize_search_index_backfill(
        &mut self,
        index_id: IndexId,
        total_docs: Option<u64>,
    ) -> anyhow::Result<ResolvedDocumentId> {
        self.initialize_backfill(index_id, total_docs, None).await
    }

    pub async fn initialize_database_index_backfill(
        &mut self,
        index_id: IndexId,
        total_docs: Option<u64>,
        snapshot_ts: Timestamp,
    ) -> anyhow::Result<ResolvedDocumentId> {
        self.initialize_backfill(index_id, total_docs, Some(snapshot_ts))
            .await
    }

    /// Creates a new index backfill entry or reset existing index backfill
    /// entry with 0 progress and the total number of documents, if available.
    /// total_docs may not be available if table summaries have not yet
    /// bootstrapped. We're ok to update it later (which will be approximate).
    async fn initialize_backfill(
        &mut self,
        index_id: IndexId,
        total_docs: Option<u64>,
        snapshot_ts: Option<Timestamp>,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let index_id = self.index_id_as_developer_id(index_id);
        tracing::info!(
            "Initializing index backfill for index developer id {}",
            index_id
        );
        let maybe_existing_backfill_metadata = self.existing_backfill_metadata(index_id).await?;
        let mut system_model = SystemMetadataModel::new_global(self.tx);
        let backfill_metadata = IndexBackfillMetadata {
            index_id,
            num_docs_indexed: 0,
            total_docs,
            cursor: snapshot_ts.map(|ts| BackfillCursor {
                snapshot_ts: ts,
                cursor: None,
            }),
        };
        if let Some(existing_backfill_metadata) = maybe_existing_backfill_metadata {
            system_model
                .replace(
                    existing_backfill_metadata.id(),
                    backfill_metadata.try_into()?,
                )
                .await?;
            Ok(existing_backfill_metadata.id())
        } else {
            system_model
                .insert(&INDEX_BACKFILLS_TABLE, backfill_metadata.try_into()?)
                .await
        }
    }

    pub async fn update_search_index_backfill_progress(
        &mut self,
        index_id: IndexId,
        tablet_id: TabletId,
        num_docs_indexed: u64,
    ) -> anyhow::Result<()> {
        self.update_index_backfill_progress(index_id, tablet_id, num_docs_indexed, None)
            .await
    }

    pub async fn update_database_index_backfill_progress(
        &mut self,
        index_id: IndexId,
        tablet_id: TabletId,
        num_docs_indexed: u64,
        cursor: ResolvedDocumentId,
    ) -> anyhow::Result<()> {
        self.update_index_backfill_progress(
            index_id,
            tablet_id,
            num_docs_indexed,
            Some(cursor.developer_id),
        )
        .await
    }

    /// Upserts progress on index backfills. Only call this during the phase of
    /// the backfill where we walk a snapshot of a table, not the catching up
    /// phase where we walk the revision stream. These metrics don't make sense
    /// in the context of the revision stream.
    /// num_docs_indexed is the number of additional documents indexed since the
    /// last call.
    async fn update_index_backfill_progress(
        &mut self,
        index_id: IndexId,
        tablet_id: TabletId,
        num_docs_indexed: u64,
        cursor: Option<DeveloperDocumentId>,
    ) -> anyhow::Result<()> {
        let index_id = self.index_id_as_developer_id(index_id);
        let maybe_existing_backfill_metadata = self.existing_backfill_metadata(index_id).await?;
        let Some(existing_backfill_metadata) = maybe_existing_backfill_metadata else {
            anyhow::bail!("Index backfill not found for index {}", index_id);
        };
        let cursor = existing_backfill_metadata
            .cursor
            .as_ref()
            .map(|c| BackfillCursor {
                snapshot_ts: c.snapshot_ts,
                cursor,
            });
        if let Some(total_docs) = existing_backfill_metadata.total_docs {
            let new_backfill_metadata = IndexBackfillMetadata {
                index_id,
                num_docs_indexed: existing_backfill_metadata.num_docs_indexed + num_docs_indexed,
                total_docs: Some(total_docs),
                cursor,
            };
            SystemMetadataModel::new_global(self.tx)
                .replace(
                    existing_backfill_metadata.id(),
                    new_backfill_metadata.try_into()?,
                )
                .await?;
        } else {
            // If there is no total_docs, we will approximate it from the current snapshot.
            let table_namespace = self.tx.table_mapping().tablet_namespace(tablet_id)?;
            let table_name = self.tx.table_mapping().tablet_name(tablet_id)?;
            if let Some(count) = self.tx.count(table_namespace, &table_name).await? {
                // Get a maybe-inaccurate total docs count because table summaries were probably
                // still bootstrapping when the backfill began.
                let new_backfill_metadata = IndexBackfillMetadata {
                    index_id,
                    num_docs_indexed: existing_backfill_metadata.num_docs_indexed
                        + num_docs_indexed,
                    total_docs: Some(count),
                    cursor,
                };
                SystemMetadataModel::new_global(self.tx)
                    .replace(
                        existing_backfill_metadata.id(),
                        new_backfill_metadata.try_into()?,
                    )
                    .await?;
            } else {
                // Return early without updating if table summaries are still bootstrapping.
                tracing::info!(
                    "Table summaries are still bootstrapping, skipping index backfill metadata \
                     update."
                );
                return Ok(());
            }
        }
        Ok(())
    }

    pub async fn delete_index_backfill(
        &mut self,
        index_id: ResolvedDocumentId,
    ) -> anyhow::Result<()> {
        if let Some(existing_backfill_metadata) = self
            .existing_backfill_metadata(index_id.developer_id)
            .await?
        {
            SystemMetadataModel::new_global(self.tx)
                .delete(existing_backfill_metadata.id())
                .await?;
        }
        Ok(())
    }
}
