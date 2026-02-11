use common::{
    components::ComponentPath,
    document::{
        ParseDocument,
        ParsedDocument,
    },
};
use database::{
    system_tables::SystemIndex,
    test_helpers::DbFixtures,
    Database,
    SystemMetadataModel,
    Transaction,
    UserFacingModel,
};
use keybroker::Identity;
use runtime::testing::TestRuntime;
use usage_tracking::{
    FunctionUsageStats,
    FunctionUsageTracker,
};
use value::{
    assert_obj,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

use crate::{
    file_storage::FILE_STORAGE_TABLE,
    scheduled_jobs::{
        args::{
            ScheduledJobArgsTable,
            SCHEDULED_JOBS_ARGS_TABLE,
        },
        test_helpers::{
            create_scheduled_job_with_args,
            insert_object_path,
        },
        types::ScheduledJobMetadata,
        SchedulerModel,
        SCHEDULED_JOBS_TABLE,
        SCHEDULED_JOBS_VIRTUAL_TABLE,
    },
    session_requests::SESSION_REQUESTS_TABLE,
    test_helpers::DbFixturesWithModel,
};

async fn write_to_table_and_get_stats(
    db: &Database<TestRuntime>,
    table: &TableName,
) -> anyhow::Result<FunctionUsageStats> {
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = db
        .begin_with_usage(Identity::system(), tx_usage.clone())
        .await?;
    let mut model = SystemMetadataModel::new(&mut tx, TableNamespace::Global);
    // Write a bogus document to the table
    model.insert(table, assert_obj!("hello" => "world")).await?;
    db.commit(tx).await?;
    Ok(tx_usage.gather_user_stats())
}

async fn get_scheduled_job_metadata_and_args_docs_sizes(
    tx: &mut Transaction<TestRuntime>,
    id: ResolvedDocumentId,
) -> anyhow::Result<(u64, u64)> {
    let scheduled_job_metadata_doc = tx.get(id).await?.unwrap();
    let scheduled_job_size = scheduled_job_metadata_doc.size();
    let scheduled_job_metadata: ParsedDocument<ScheduledJobMetadata> =
        scheduled_job_metadata_doc.parse()?;
    let scheduled_job_args = UserFacingModel::new(tx, TableNamespace::Global)
        .get(scheduled_job_metadata.args_id.unwrap(), None)
        .await?
        .unwrap();
    Ok((scheduled_job_size as u64, scheduled_job_args.size() as u64))
}

#[convex_macro::test_runtime]
async fn scheduled_job_writes_counted_in_db_bandwidth(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
    // Check that database ingress v2 size is non-zero
    let stats = write_to_table_and_get_stats(&db, &SCHEDULED_JOBS_TABLE).await?;
    assert_eq!(stats.database_ingress.values().sum::<u64>(), 0);
    // TODO: Check the amount matches the number of indexes
    assert_ne!(stats.database_ingress_v2.values().sum::<u64>(), 0);
    Ok(())
}

#[convex_macro::test_runtime]
async fn scheduled_job_arg_writes_counted_in_db_bandwidth(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
    // Check that database ingress v2 size is non-zero
    let stats = write_to_table_and_get_stats(&db, &SCHEDULED_JOBS_ARGS_TABLE).await?;
    assert_eq!(stats.database_ingress.values().sum::<u64>(), 0);
    // TODO: Check the amount matches the number of indexes
    assert_ne!(stats.database_ingress_v2.values().sum::<u64>(), 0);
    Ok(())
}

#[convex_macro::test_runtime]
async fn scheduled_job_arg_reads_counted_in_db_bandwidth(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
    let mut tx = db.begin_system().await?;
    let (id, ..) =
        create_scheduled_job_with_args(&rt, &mut tx, insert_object_path(), vec![]).await?;
    db.commit(tx).await?;

    // Read the scheduled job args table
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = db
        .begin_with_usage(Identity::system(), tx_usage.clone())
        .await?;
    tx.query_system(
        TableNamespace::Global,
        &SystemIndex::<ScheduledJobArgsTable>::by_creation_time(),
    )?
    .all()
    .await?;
    let stats = tx_usage.gather_user_stats();
    let (_, scheduled_job_args_size) =
        get_scheduled_job_metadata_and_args_docs_sizes(&mut tx, id).await?;
    assert_eq!(
        stats.database_egress.values().sum::<u64>(),
        scheduled_job_args_size
    );
    assert!(!stats
        .database_egress_v2
        .contains_key(&(ComponentPath::root(), "_scheduled_job_args".to_string())));
    assert_eq!(
        *stats
            .database_egress_v2
            .get(&(ComponentPath::root(), "_scheduled_functions".to_string()))
            .unwrap(),
        scheduled_job_args_size
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn scheduled_functions_reads_count_args_in_db_bandwidth(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
    let mut tx = db.begin_system().await?;
    let (id, ..) =
        create_scheduled_job_with_args(&rt, &mut tx, insert_object_path(), vec![]).await?;
    db.commit(tx).await?;
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = db
        .begin_with_usage(Identity::system(), tx_usage.clone())
        .await?;
    SchedulerModel::new(&mut tx, TableNamespace::Global)
        .read_virtual_table()
        .await?;
    let stats = tx_usage.gather_user_stats();

    // Fetch the two system tables separately - as that's what we charge
    let (scheduled_job_metadata_size, scheduled_job_args_size) =
        get_scheduled_job_metadata_and_args_docs_sizes(&mut tx, id).await?;

    // Assert we charge the sum of the tables - attributed to the virtual table
    for egress in [stats.database_egress, stats.database_egress_v2] {
        assert_eq!(
            egress.values().sum::<u64>(),
            scheduled_job_metadata_size + scheduled_job_args_size
        );
        assert!(egress
            .keys()
            .all(|(_, t)| *t == SCHEDULED_JOBS_VIRTUAL_TABLE.to_string()));
    }
    Ok(())
}

#[convex_macro::test_runtime]
async fn file_storage_writes_counted_in_db_bandwidth(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
    // Check that database ingress v2 size is non-zero
    let stats = write_to_table_and_get_stats(&db, &FILE_STORAGE_TABLE).await?;
    assert_eq!(stats.database_ingress.values().sum::<u64>(), 0);
    assert_ne!(stats.database_ingress_v2.values().sum::<u64>(), 0);
    Ok(())
}

#[convex_macro::test_runtime]
async fn system_table_writes_do_not_count_in_db_bandwidth(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
    let stats = write_to_table_and_get_stats(&db, &SESSION_REQUESTS_TABLE).await?;
    // Check that database ingress size is zero
    assert_eq!(stats.database_ingress.values().sum::<u64>(), 0);
    assert_eq!(stats.database_ingress_v2.values().sum::<u64>(), 0);
    Ok(())
}

#[convex_macro::test_runtime]
async fn virtual_table_storage_accumulates_across_system_tables(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;

    // Write to both _scheduled_jobs and _scheduled_job_args that map to
    // _scheduled_functions virtual table
    let mut tx = db.begin(Identity::system()).await?;
    let mut model = SystemMetadataModel::new(&mut tx, TableNamespace::Global);
    model
        .insert(&SCHEDULED_JOBS_TABLE, assert_obj!("hello" => "world"))
        .await?;
    model
        .insert(&SCHEDULED_JOBS_ARGS_TABLE, assert_obj!("foo" => "bar"))
        .await?;
    db.commit(tx).await?;

    let snapshot = db.latest_snapshot()?;
    let tables_usage = snapshot.get_document_and_index_storage()?;
    let jobs_usage = &tables_usage
        .system_tables
        .get(&(TableNamespace::Global, SCHEDULED_JOBS_TABLE.clone()))
        .expect("_scheduled_jobs should exist");
    let args_usage = &tables_usage
        .system_tables
        .get(&(TableNamespace::Global, SCHEDULED_JOBS_ARGS_TABLE.clone()))
        .expect("_scheduled_job_args should exist");

    // The virtual table _scheduled_functions should have the sum of both jobs_usage
    // and args_usage
    let virtual_usage = &tables_usage
        .virtual_tables
        .get(&(TableNamespace::Global, SCHEDULED_JOBS_VIRTUAL_TABLE.clone()))
        .expect("_scheduled_functions virtual table should exist");
    assert_eq!(
        virtual_usage.0,
        jobs_usage.0 + args_usage.0,
        "Virtual table usage should be sum of both system tables"
    );

    Ok(())
}
