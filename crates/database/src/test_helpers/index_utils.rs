use std::{
    assert_matches::assert_matches,
    str::FromStr,
    sync::Arc,
};

use anyhow::Context;
pub use common::index::test_helpers::{
    new_index_descriptor,
    new_index_name,
};
use common::{
    bootstrap_model::index::{
        database_index::{
            DatabaseIndexState,
            IndexedFields,
        },
        text_index::TextIndexState,
        vector_index::VectorIndexState,
        IndexConfig,
        IndexMetadata,
        TabletIndexMetadata,
    },
    document::ParsedDocument,
    persistence::{
        NoopRetentionValidator,
        Persistence,
    },
    runtime::Runtime,
    types::{
        IndexDescriptor,
        IndexDiff,
        IndexName,
        IndexTableIdentifier,
    },
};
use runtime::testing::TestRuntime;
use value::{
    ResolvedDocumentId,
    TableNamespace,
};

use crate::{
    Database,
    IndexModel,
    IndexWorker,
    Transaction,
};

pub fn get_recent_index_metadata(
    tx: &mut Transaction<TestRuntime>,
    table_name: &str,
    index_name: &str,
) -> anyhow::Result<TabletIndexMetadata> {
    assert_at_most_one_definition(tx, table_name, index_name)?;

    let expected = IndexName::from_str(&format!("{table_name}.{index_name}"))?;
    IndexModel::new(tx)
        .pending_index_metadata(TableNamespace::test_user(), &expected)?
        .or(IndexModel::new(tx).enabled_index_metadata(TableNamespace::test_user(), &expected)?)
        .map(|doc| doc.into_value())
        .context(format!("Missing index: {expected}"))
}

fn assert_at_most_one_definition(
    tx: &mut Transaction<TestRuntime>,
    table_name: &str,
    index_name: &str,
) -> anyhow::Result<()> {
    let index_name = new_index_name(table_name, index_name)?;
    let mut model = IndexModel::new(tx);
    let enabled = model.enabled_index_metadata(TableNamespace::test_user(), &index_name)?;
    let pending = model.pending_index_metadata(TableNamespace::test_user(), &index_name)?;
    assert!(enabled.is_none() || pending.is_none());
    Ok(())
}

pub fn assert_backfilling(
    mut tx: Transaction<TestRuntime>,
    table_name: &str,
    index_name: &str,
) -> anyhow::Result<()> {
    let index_metadata: common::bootstrap_model::index::IndexMetadata<value::TabletId> =
        get_recent_index_metadata(&mut tx, table_name, index_name)?;
    match index_metadata.config {
        IndexConfig::Database { on_disk_state, .. } => {
            assert_matches!(on_disk_state, DatabaseIndexState::Backfilling(_))
        },
        IndexConfig::Text { on_disk_state, .. } => {
            assert_matches!(on_disk_state, TextIndexState::Backfilling(_))
        },
        IndexConfig::Vector { on_disk_state, .. } => {
            assert_matches!(on_disk_state, VectorIndexState::Backfilling(_))
        },
    }
    Ok(())
}

pub async fn assert_backfilled(
    db: &Database<TestRuntime>,
    table_name: &str,
    index_name: &'static str,
) -> anyhow::Result<()> {
    let mut tx = db.begin_system().await?;
    let index_metadata: common::bootstrap_model::index::IndexMetadata<value::TabletId> =
        get_recent_index_metadata(&mut tx, table_name, index_name)?;
    match index_metadata.config {
        IndexConfig::Database { on_disk_state, .. } => {
            assert_matches!(on_disk_state, DatabaseIndexState::Backfilled { .. })
        },
        IndexConfig::Text { on_disk_state, .. } => {
            assert_matches!(on_disk_state, TextIndexState::Backfilled { .. })
        },
        IndexConfig::Vector { on_disk_state, .. } => {
            assert_matches!(on_disk_state, VectorIndexState::Backfilled { .. })
        },
    }
    Ok(())
}

pub async fn assert_enabled(
    db: &Database<TestRuntime>,
    table_name: &str,
    index_name: &str,
) -> anyhow::Result<()> {
    let mut tx = db.begin_system().await?;
    let index_metadata: common::bootstrap_model::index::IndexMetadata<value::TabletId> =
        get_recent_index_metadata(&mut tx, table_name, index_name)?;
    match index_metadata.config {
        IndexConfig::Database { on_disk_state, .. } => {
            assert_eq!(on_disk_state, DatabaseIndexState::Enabled)
        },
        IndexConfig::Text { on_disk_state, .. } => {
            assert_matches!(on_disk_state, TextIndexState::SnapshottedAt(_))
        },
        IndexConfig::Vector { on_disk_state, .. } => {
            assert_matches!(on_disk_state, VectorIndexState::SnapshottedAt(_))
        },
    }
    Ok(())
}

pub fn index_descriptors_and_fields(diff: &IndexDiff) -> Vec<Vec<(IndexDescriptor, Vec<String>)>> {
    let IndexDiff {
        added,
        identical: _,
        dropped,
        enabled,
        disabled,
    } = diff.clone();
    let dropped = values(dropped);
    let enabled = values(enabled);
    let disabled = values(disabled);

    vec![added, dropped, enabled, disabled]
        .into_iter()
        .map(descriptors_and_fields)
        .collect()
}

pub fn values<T: IndexTableIdentifier>(
    docs: Vec<ParsedDocument<IndexMetadata<T>>>,
) -> Vec<IndexMetadata<T>> {
    docs.into_iter().map(|doc| doc.into_value()).collect()
}

pub fn descriptors_and_fields<T: IndexTableIdentifier>(
    metadata: Vec<IndexMetadata<T>>,
) -> Vec<(IndexDescriptor, Vec<String>)> {
    let mut descriptors: Vec<_> = metadata
        .iter()
        .map(|index| (descriptor(index), get_index_fields(index.clone())))
        .collect();
    descriptors.sort();
    descriptors
}

pub fn descriptors<T: IndexTableIdentifier>(
    metadata: Vec<IndexMetadata<T>>,
) -> Vec<IndexDescriptor> {
    metadata.iter().map(|index| descriptor(index)).collect()
}

fn descriptor<T: IndexTableIdentifier>(metadata: &IndexMetadata<T>) -> IndexDescriptor {
    metadata.name.descriptor().clone()
}

pub fn get_index_fields<T: IndexTableIdentifier>(index_metadata: IndexMetadata<T>) -> Vec<String> {
    match index_metadata.config {
        IndexConfig::Database { spec, .. } => spec
            .fields
            .into_iter()
            .map(|field_path| field_path.into())
            .collect(),
        IndexConfig::Text { spec, .. } => vec![spec.search_field.into()],
        IndexConfig::Vector { spec, .. } => vec![spec.vector_field.into()],
    }
}

impl<RT: Runtime> Database<RT> {
    pub async fn create_backfilled_index_for_test(
        &self,
        tp: Arc<dyn Persistence>,
        namespace: TableNamespace,
        index_name: IndexName,
        fields: IndexedFields,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let mut tx = self.begin_system().await?;
        let begin_ts = tx.begin_timestamp();
        let id = IndexModel::new(&mut tx)
            .add_application_index(
                namespace,
                IndexMetadata::new_backfilling(*begin_ts, index_name.clone(), fields),
            )
            .await?;
        self.commit(tx).await?;

        let retention_validator = Arc::new(NoopRetentionValidator);

        let index_backfill_fut = IndexWorker::new_terminating(
            self.runtime.clone(),
            tp,
            retention_validator,
            self.clone(),
        );
        index_backfill_fut.await?;

        let mut tx = self.begin_system().await?;
        IndexModel::new(&mut tx)
            .enable_index_for_testing(namespace, &index_name)
            .await?;
        self.commit(tx).await?;
        Ok(id)
    }
}
