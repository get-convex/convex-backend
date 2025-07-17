use common::{
    runtime::Runtime,
    types::IndexId,
};
use runtime::testing::TestRuntime;
use value::{
    DeveloperDocumentId,
    InternalId,
    ResolvedDocumentId,
    TableNamespace,
    TabletId,
};

use crate::{
    bootstrap_model::index_backfills::{
        types::IndexBackfillMetadata,
        IndexBackfillModel,
    },
    test_helpers::new_test_database,
    TableModel,
    Transaction,
};

async fn setup_test_tx<RT: Runtime>(rt: RT) -> anyhow::Result<Transaction<RT>> {
    let db = new_test_database(rt).await;
    db.begin_system().await
}

fn create_test_index_id() -> IndexId {
    InternalId::MIN
}

fn create_test_tablet_id() -> TabletId {
    TabletId::MIN
}

#[convex_macro::test_runtime]
async fn test_initialize_backfill_creates_new_entry(rt: TestRuntime) -> anyhow::Result<()> {
    let mut tx = setup_test_tx(rt).await?;

    let index_id = create_test_index_id();
    let total_docs = Some(1000u64);

    let backfill_id = IndexBackfillModel::new(&mut tx)
        .initialize_backfill(index_id, total_docs)
        .await?;

    // Verify the backfill was created
    let backfill_doc = tx.get(backfill_id).await?;
    assert!(backfill_doc.is_some());

    let backfill_metadata: IndexBackfillMetadata =
        backfill_doc.unwrap().into_value().into_value().try_into()?;
    assert_eq!(backfill_metadata.num_docs_indexed, 0);
    assert_eq!(backfill_metadata.total_docs, total_docs);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_initialize_backfill_with_none_total_docs(rt: TestRuntime) -> anyhow::Result<()> {
    let mut tx = setup_test_tx(rt).await?;

    let index_id = create_test_index_id();
    let total_docs = None;

    let backfill_id = IndexBackfillModel::new(&mut tx)
        .initialize_backfill(index_id, total_docs)
        .await?;

    // Verify the backfill was created with None total_docs
    let backfill_doc = tx.get(backfill_id).await?;
    assert!(backfill_doc.is_some());

    let backfill_metadata: IndexBackfillMetadata =
        backfill_doc.unwrap().into_value().into_value().try_into()?;
    assert_eq!(backfill_metadata.num_docs_indexed, 0);
    assert_eq!(backfill_metadata.total_docs, None);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_initialize_backfill_resets_existing_entry(rt: TestRuntime) -> anyhow::Result<()> {
    let mut tx = setup_test_tx(rt).await?;

    let index_id = create_test_index_id();

    // Create initial backfill
    let first_backfill_id = IndexBackfillModel::new(&mut tx)
        .initialize_backfill(index_id, Some(500))
        .await?;

    // Update progress
    IndexBackfillModel::new(&mut tx)
        .update_index_backfill_progress(index_id, create_test_tablet_id(), 100)
        .await?;

    // Verify progress was updated
    let backfill_doc = tx.get(first_backfill_id).await?;
    let backfill_metadata: IndexBackfillMetadata =
        backfill_doc.unwrap().into_value().into_value().try_into()?;
    assert_eq!(backfill_metadata.num_docs_indexed, 100);

    // Initialize again with different total_docs - should reset progress
    let second_backfill_id = IndexBackfillModel::new(&mut tx)
        .initialize_backfill(index_id, Some(1000))
        .await?;

    // Should return the same ID
    assert_eq!(first_backfill_id, second_backfill_id);

    // Verify the backfill was reset
    let backfill_doc = tx.get(second_backfill_id).await?;
    let backfill_metadata: IndexBackfillMetadata =
        backfill_doc.unwrap().into_value().into_value().try_into()?;
    assert_eq!(backfill_metadata.num_docs_indexed, 0);
    assert_eq!(backfill_metadata.total_docs, Some(1000));

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_update_index_backfill_progress_with_total_docs(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut tx = setup_test_tx(rt).await?;

    let index_id = create_test_index_id();
    let tablet_id = create_test_tablet_id();
    let total_docs = Some(1000u64);

    // Initialize backfill
    let backfill_id = IndexBackfillModel::new(&mut tx)
        .initialize_backfill(index_id, total_docs)
        .await?;

    // Update progress
    IndexBackfillModel::new(&mut tx)
        .update_index_backfill_progress(index_id, tablet_id, 250)
        .await?;

    // Verify progress was updated
    let backfill_doc = tx.get(backfill_id).await?;
    let backfill_metadata: IndexBackfillMetadata =
        backfill_doc.unwrap().into_value().into_value().try_into()?;
    assert_eq!(backfill_metadata.num_docs_indexed, 250);
    assert_eq!(backfill_metadata.total_docs, total_docs);

    // Update progress again
    IndexBackfillModel::new(&mut tx)
        .update_index_backfill_progress(index_id, tablet_id, 150)
        .await?;

    // Verify progress was accumulated
    let backfill_doc = tx.get(backfill_id).await?;
    let backfill_metadata: IndexBackfillMetadata =
        backfill_doc.unwrap().into_value().into_value().try_into()?;
    assert_eq!(backfill_metadata.num_docs_indexed, 400);
    assert_eq!(backfill_metadata.total_docs, total_docs);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_update_index_backfill_progress_nonexistent_backfill(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut tx = setup_test_tx(rt).await?;

    let index_id = create_test_index_id();
    let tablet_id = create_test_tablet_id();

    // Try to update progress for non-existent backfill
    let result = IndexBackfillModel::new(&mut tx)
        .update_index_backfill_progress(index_id, tablet_id, 100)
        .await;

    // Should return an error
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Index backfill not found"));

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_update_index_backfill_progress_with_none_total_docs(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut tx = setup_test_tx(rt).await?;

    let index_id = create_test_index_id();

    let tablet_id = TableModel::new(&mut tx)
        .insert_table_metadata_for_test(TableNamespace::Global, &"table".parse()?)
        .await?
        .tablet_id;
    // Initialize backfill without total_docs
    let backfill_id = IndexBackfillModel::new(&mut tx)
        .initialize_backfill(index_id, None)
        .await?;

    // In a test environment, table mapping for a non-existent tablet will fail,
    // so this tests that the method handles the error gracefully
    IndexBackfillModel::new(&mut tx)
        .update_index_backfill_progress(index_id, tablet_id, 100)
        .await?;

    let backfill_doc = tx.get(backfill_id).await?;
    let backfill_metadata: IndexBackfillMetadata =
        backfill_doc.unwrap().into_value().into_value().try_into()?;
    assert_eq!(backfill_metadata.total_docs, Some(0));

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_delete_index_backfill_existing(rt: TestRuntime) -> anyhow::Result<()> {
    let mut tx = setup_test_tx(rt).await?;

    let index_id = create_test_index_id();
    let total_docs = Some(1000u64);

    // Initialize backfill
    let backfill_id = IndexBackfillModel::new(&mut tx)
        .initialize_backfill(index_id, total_docs)
        .await?;

    // Verify it exists
    let backfill_doc = tx.get(backfill_id).await?;
    assert!(backfill_doc.is_some());

    // For delete_index_backfill, we need to pass the index's ResolvedDocumentId,
    // not the backfill document's ID. Create a fake index document ID.
    let index_table_id = tx.bootstrap_tables().index_id;
    let index_developer_id = DeveloperDocumentId::new(index_table_id.table_number, index_id);
    let index_resolved_id = ResolvedDocumentId::new(index_table_id.tablet_id, index_developer_id);

    // Delete the backfill
    IndexBackfillModel::new(&mut tx)
        .delete_index_backfill(index_resolved_id)
        .await?;

    // Verify it was deleted
    let backfill_doc = tx.get(backfill_id).await?;
    assert!(backfill_doc.is_none());

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_delete_index_backfill_nonexistent(rt: TestRuntime) -> anyhow::Result<()> {
    let mut tx = setup_test_tx(rt).await?;

    // Create a fake index document ID that doesn't exist
    let index_table_id = tx.bootstrap_tables().index_id;
    let fake_developer_id = DeveloperDocumentId::new(index_table_id.table_number, InternalId::MAX);
    let fake_resolved_id = ResolvedDocumentId::new(index_table_id.tablet_id, fake_developer_id);

    // Delete non-existent backfill - should not error (method handles missing
    // gracefully)
    IndexBackfillModel::new(&mut tx)
        .delete_index_backfill(fake_resolved_id)
        .await?;

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_multiple_backfills_different_indexes(rt: TestRuntime) -> anyhow::Result<()> {
    let mut tx = setup_test_tx(rt).await?;

    let index_id1 = InternalId::MIN;
    let index_id2 = InternalId::MAX;
    let tablet_id = create_test_tablet_id();

    // Initialize backfills for different indexes
    let mut model = IndexBackfillModel::new(&mut tx);
    let backfill_id1 = model.initialize_backfill(index_id1, Some(1000)).await?;
    let backfill_id2 = model.initialize_backfill(index_id2, Some(2000)).await?;

    // Update progress for both
    model
        .update_index_backfill_progress(index_id1, tablet_id, 100)
        .await?;
    model
        .update_index_backfill_progress(index_id2, tablet_id, 200)
        .await?;

    // Verify both backfills exist with correct progress
    let backfill_doc1 = tx.get(backfill_id1).await?;
    let backfill_metadata1: IndexBackfillMetadata = backfill_doc1
        .unwrap()
        .into_value()
        .into_value()
        .try_into()?;
    assert_eq!(backfill_metadata1.num_docs_indexed, 100);
    assert_eq!(backfill_metadata1.total_docs, Some(1000));

    let backfill_doc2 = tx.get(backfill_id2).await?;
    let backfill_metadata2: IndexBackfillMetadata = backfill_doc2
        .unwrap()
        .into_value()
        .into_value()
        .try_into()?;
    assert_eq!(backfill_metadata2.num_docs_indexed, 200);
    assert_eq!(backfill_metadata2.total_docs, Some(2000));

    // Delete one backfill using the index's ResolvedDocumentId
    let index_table_id = tx.bootstrap_tables().index_id;
    let index1_developer_id = DeveloperDocumentId::new(index_table_id.table_number, index_id1);
    let index1_resolved_id = ResolvedDocumentId::new(index_table_id.tablet_id, index1_developer_id);

    IndexBackfillModel::new(&mut tx)
        .delete_index_backfill(index1_resolved_id)
        .await?;

    // Verify only one was deleted
    let backfill_doc1 = tx.get(backfill_id1).await?;
    assert!(backfill_doc1.is_none());

    let backfill_doc2 = tx.get(backfill_id2).await?;
    assert!(backfill_doc2.is_some());

    Ok(())
}
