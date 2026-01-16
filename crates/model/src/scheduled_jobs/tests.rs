use database::{
    test_helpers::DbFixtures,
    Database,
    SystemMetadataModel,
};
use keybroker::Identity;
use runtime::testing::TestRuntime;
use usage_tracking::{
    FunctionUsageStats,
    FunctionUsageTracker,
};
use value::{
    assert_obj,
    ConvexValue,
    TableName,
    TableNamespace,
};

use crate::{
    file_storage::FILE_STORAGE_TABLE,
    scheduled_jobs::{
        args::SCHEDULED_JOBS_ARGS_TABLE,
        test_helpers::{
            create_scheduled_job_with_args,
            insert_object_path,
        },
        SchedulerModel,
        SCHEDULED_JOBS_TABLE,
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

#[convex_macro::test_runtime]
async fn scheduled_job_writes_counted_in_db_bandwidth(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
    // Check that database ingress v2 size is non-zero
    let stats = write_to_table_and_get_stats(&db, &SCHEDULED_JOBS_TABLE).await?;
    assert_eq!(stats.database_ingress.values().sum::<u64>(), 0);
    assert_ne!(stats.database_ingress_v2.values().sum::<u64>(), 0);
    Ok(())
}

#[convex_macro::test_runtime]
async fn scheduled_job_arg_writes_counted_in_db_bandwidth(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
    // Check that database ingress v2 size is non-zero
    let stats = write_to_table_and_get_stats(&db, &SCHEDULED_JOBS_ARGS_TABLE).await?;
    assert_eq!(stats.database_ingress.values().sum::<u64>(), 0);
    // TODO(ENG-10171) Track writes to _scheduled_job_args (and other system tables
    // that contribute to virtual tables) toward db bandwidth assert_ne!(stats.
    // database_ingress_v2.values().sum::<u64>(), 0);
    Ok(())
}

#[convex_macro::test_runtime]
async fn scheduled_functions_reads_count_args_in_db_bandwidth(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
    let tx_usage = FunctionUsageTracker::new();
    let size_with_empty_args = 143;
    let mut tx = db
        .begin_with_usage(Identity::system(), tx_usage.clone())
        .await?;
    create_scheduled_job_with_args(&rt, &mut tx, insert_object_path(), vec![]).await?;
    SchedulerModel::new(&mut tx, TableNamespace::Global)
        .read_virtual_table()
        .await?;
    let stats = tx_usage.gather_user_stats();
    assert_eq!(
        stats.database_egress.values().sum::<u64>(),
        size_with_empty_args
    );
    assert_eq!(
        stats.database_egress_v2.values().sum::<u64>(),
        size_with_empty_args
    );
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = db
        .begin_with_usage(Identity::system(), tx_usage.clone())
        .await?;
    create_scheduled_job_with_args(
        &rt,
        &mut tx,
        insert_object_path(),
        vec![ConvexValue::String("hello".try_into()?)],
    )
    .await?;
    SchedulerModel::new(&mut tx, TableNamespace::Global)
        .read_virtual_table()
        .await?;
    let stats = tx_usage.gather_user_stats();
    assert!(stats.database_egress.values().sum::<u64>() > size_with_empty_args);
    assert!(stats.database_egress_v2.values().sum::<u64>() > size_with_empty_args);
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
