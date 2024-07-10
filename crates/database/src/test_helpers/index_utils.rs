use std::{
    assert_matches::assert_matches,
    str::FromStr,
};

use anyhow::Context;
use common::{
    bootstrap_model::index::{
        database_index::DatabaseIndexState,
        text_index::TextIndexState,
        vector_index::VectorIndexState,
        IndexConfig,
        IndexMetadata,
        TabletIndexMetadata,
    },
    document::ParsedDocument,
    types::{
        IndexDescriptor,
        IndexDiff,
        IndexName,
        IndexTableIdentifier,
    },
};
use runtime::testing::TestRuntime;
use value::TableNamespace;

use crate::{
    Database,
    IndexModel,
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
        .context(format!("Missing index: {}", expected))
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
    index_name: &str,
) -> anyhow::Result<()> {
    let mut tx = db.begin_system().await?;
    let index_metadata: common::bootstrap_model::index::IndexMetadata<value::TabletId> =
        get_recent_index_metadata(&mut tx, table_name, index_name)?;
    match index_metadata.config {
        IndexConfig::Database { on_disk_state, .. } => {
            assert_matches!(on_disk_state, DatabaseIndexState::Backfilled { .. })
        },
        IndexConfig::Text { on_disk_state, .. } => {
            assert_matches!(on_disk_state, TextIndexState::Backfilled(_))
        },
        IndexConfig::Vector { on_disk_state, .. } => {
            assert_matches!(on_disk_state, VectorIndexState::Backfilled(_))
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

pub fn new_index_name(table_name: &str, index_name: &str) -> anyhow::Result<IndexName> {
    IndexName::new(str::parse(table_name)?, index_name.parse()?)
}

pub fn new_index_descriptor(table_name: &str, index_name: &str) -> anyhow::Result<IndexDescriptor> {
    new_index_name(table_name, index_name).map(|name| name.descriptor().clone())
}

pub fn index_descriptors_and_fields(diff: &IndexDiff) -> Vec<Vec<(IndexDescriptor, Vec<String>)>> {
    let IndexDiff {
        added,
        identical: _,
        dropped,
    } = diff.clone();
    let dropped = values(dropped);

    vec![added, dropped]
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
    metadata
        .iter()
        .map(|index| (descriptor(index), get_index_fields(index.clone())))
        .collect()
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
        IndexConfig::Database {
            developer_config, ..
        } => developer_config
            .fields
            .iter()
            .flat_map(|field_path| field_path.fields().iter().map(|field| field.to_string()))
            .collect(),
        IndexConfig::Text {
            developer_config, ..
        } => developer_config
            .search_field
            .fields()
            .iter()
            .map(|field| field.to_string())
            .collect(),
        IndexConfig::Vector {
            developer_config, ..
        } => developer_config
            .vector_field
            .fields()
            .iter()
            .map(|field| field.to_string())
            .collect(),
    }
}
