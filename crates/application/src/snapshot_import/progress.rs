use common::{
    components::ComponentPath,
    runtime::Runtime,
    types::TableName,
};
use database::Database;
use keybroker::Identity;
use model::snapshot_imports::SnapshotImportModel;
use usage_tracking::FunctionUsageTracker;
use value::ResolvedDocumentId;

pub async fn best_effort_update_progress_message<RT: Runtime>(
    database: &Database<RT>,
    identity: &Identity,
    import_id: ResolvedDocumentId,
    progress_message: String,
    component_path: &ComponentPath,
    display_table_name: &TableName,
    num_rows_written: i64,
) {
    // Ignore errors because it's not worth blocking or retrying if we can't
    // send a nice progress message on the first try.
    let _result: anyhow::Result<()> = try {
        let mut tx = database.begin(identity.clone()).await?;
        let mut import_model = SnapshotImportModel::new(&mut tx);
        import_model
            .update_progress_message(
                import_id,
                progress_message,
                component_path,
                display_table_name,
                num_rows_written,
            )
            .await?;
        database
            .commit_with_write_source(tx, "snapshot_update_progress_msg")
            .await?;
    };
}

pub async fn add_checkpoint_message<RT: Runtime>(
    database: &Database<RT>,
    identity: &Identity,
    import_id: ResolvedDocumentId,
    checkpoint_message: String,
    component_path: &ComponentPath,
    display_table_name: &TableName,
    num_rows_written: i64,
) -> anyhow::Result<()> {
    database
        .execute_with_overloaded_retries(
            identity.clone(),
            FunctionUsageTracker::new(),
            "snapshot_import_add_checkpoint_message",
            |tx| {
                async {
                    SnapshotImportModel::new(tx)
                        .add_checkpoint_message(
                            import_id,
                            checkpoint_message.clone(),
                            component_path,
                            display_table_name,
                            num_rows_written,
                        )
                        .await
                }
                .into()
            },
        )
        .await?;
    Ok(())
}
