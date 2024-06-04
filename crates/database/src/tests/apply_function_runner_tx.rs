use keybroker::Identity;
use runtime::testing::TestRuntime;
use usage_tracking::FunctionUsageTracker;
use value::obj;

use crate::{
    test_helpers::new_test_database,
    Transaction,
    UserFacingModel,
};

fn assert_transactions_match(
    backend_tx: Transaction<TestRuntime>,
    function_runner_tx: Transaction<TestRuntime>,
) -> anyhow::Result<()> {
    assert_transaction_reads_match(&backend_tx, &function_runner_tx)?;
    assert_transaction_writes_match(&backend_tx, &function_runner_tx)?;
    Ok(())
}

fn assert_transaction_reads_match(
    backend_tx: &Transaction<TestRuntime>,
    function_runner_tx: &Transaction<TestRuntime>,
) -> anyhow::Result<()> {
    assert_eq!(
        backend_tx.reads.read_set(),
        function_runner_tx.reads.read_set()
    );
    assert_eq!(
        backend_tx.reads.user_tx_size(),
        function_runner_tx.reads.user_tx_size()
    );
    assert_eq!(
        backend_tx.reads.system_tx_size(),
        function_runner_tx.reads.system_tx_size()
    );
    Ok(())
}

fn assert_transaction_writes_match(
    backend_tx: &Transaction<TestRuntime>,
    function_runner_tx: &Transaction<TestRuntime>,
) -> anyhow::Result<()> {
    assert_eq!(backend_tx.writes, function_runner_tx.writes);
    assert_eq!(
        backend_tx.index.index_registry(),
        function_runner_tx.index.index_registry()
    );
    assert_eq!(backend_tx.metadata, function_runner_tx.metadata);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_apply_function_runner_tx_new_table(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt).await;
    let mut backend_tx = db.begin_system().await?;
    let begin_timestamp = backend_tx.begin_timestamp();

    // Create a new tx as though it were in function runner
    let mut function_runner_tx = db
        .begin_with_ts(
            Identity::system(),
            *begin_timestamp,
            FunctionUsageTracker::new(),
        )
        .await?;

    // Insert a document into a new table
    UserFacingModel::new_root_for_test(&mut function_runner_tx)
        .insert("table".parse()?, obj!("field" => "value")?)
        .await?;

    // Apply these writes to the backend_tx
    let num_intervals = function_runner_tx.reads.num_intervals();
    let user_tx_size = function_runner_tx.reads.user_tx_size().clone();
    let system_tx_size = function_runner_tx.reads.system_tx_size().clone();
    let reads = function_runner_tx.reads.clone().into_read_set();
    let rows_read = function_runner_tx
        .stats()
        .iter()
        .map(|(table, stats)| (*table, stats.rows_read))
        .collect();
    let rows_read_by_tablet = function_runner_tx
        .stats_by_tablet()
        .iter()
        .map(|(table, stats)| (*table, stats.rows_read))
        .collect();
    let (updates, generated_ids) = function_runner_tx
        .writes
        .clone()
        .into_updates_and_generated_ids();
    backend_tx.apply_function_runner_tx(
        *begin_timestamp,
        reads,
        num_intervals,
        user_tx_size,
        system_tx_size,
        updates,
        generated_ids,
        rows_read,
        rows_read_by_tablet,
    )?;
    assert_eq!(
        backend_tx.next_creation_time,
        function_runner_tx.next_creation_time
    );
    assert_transactions_match(backend_tx, function_runner_tx)?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_apply_function_runner_tx_read_only(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt).await;
    let mut setup_tx = db.begin_system().await?;
    let id = UserFacingModel::new_root_for_test(&mut setup_tx)
        .insert("table".parse()?, obj!("field" => "value")?)
        .await?;
    db.commit(setup_tx).await?;

    let mut backend_tx = db.begin_system().await?;
    let begin_timestamp = backend_tx.begin_timestamp();

    // Create a new tx as though it were in funrun
    let mut function_runner_tx = db
        .begin_with_ts(
            Identity::system(),
            *begin_timestamp,
            FunctionUsageTracker::new(),
        )
        .await?;

    UserFacingModel::new_root_for_test(&mut function_runner_tx)
        .get_with_ts(id, None)
        .await?;

    // Apply these writes to the backend_tx
    let num_intervals = function_runner_tx.reads.num_intervals();
    let user_tx_size = function_runner_tx.reads.user_tx_size().clone();
    let system_tx_size = function_runner_tx.reads.system_tx_size().clone();
    let reads = function_runner_tx.reads.clone().into_read_set();
    let rows_read = function_runner_tx
        .stats()
        .iter()
        .map(|(table, stats)| (*table, stats.rows_read))
        .collect();
    let rows_read_by_tablet = function_runner_tx
        .stats_by_tablet()
        .iter()
        .map(|(table, stats)| (*table, stats.rows_read))
        .collect();
    let (updates, generated_ids) = function_runner_tx
        .writes
        .clone()
        .into_updates_and_generated_ids();
    backend_tx.apply_function_runner_tx(
        *begin_timestamp,
        reads,
        num_intervals,
        user_tx_size,
        system_tx_size,
        updates,
        generated_ids,
        rows_read,
        rows_read_by_tablet,
    )?;

    assert_transactions_match(backend_tx, function_runner_tx)?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_apply_function_runner_tx_replace(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt).await;
    let mut setup_tx = db.begin_system().await?;
    let id = UserFacingModel::new_root_for_test(&mut setup_tx)
        .insert("table".parse()?, obj!("field" => "value")?)
        .await?;
    db.commit(setup_tx).await?;

    let mut backend_tx = db.begin_system().await?;
    let begin_timestamp = backend_tx.begin_timestamp();

    // Create a new tx as though it were in function runner
    let mut function_runner_tx = db
        .begin_with_ts(
            Identity::system(),
            *begin_timestamp,
            FunctionUsageTracker::new(),
        )
        .await?;

    UserFacingModel::new_root_for_test(&mut function_runner_tx)
        .replace(id, obj!("field" => "value2")?)
        .await?;

    // Apply these writes to the backend_tx
    let num_intervals = function_runner_tx.reads.num_intervals();
    let user_tx_size = function_runner_tx.reads.user_tx_size().clone();
    let system_tx_size = function_runner_tx.reads.system_tx_size().clone();
    let reads = function_runner_tx.reads.clone().into_read_set();
    let rows_read = function_runner_tx
        .stats()
        .iter()
        .map(|(table, stats)| (*table, stats.rows_read))
        .collect();
    let rows_read_by_tablet = function_runner_tx
        .stats_by_tablet()
        .iter()
        .map(|(table, stats)| (*table, stats.rows_read))
        .collect();
    let (updates, generated_ids) = function_runner_tx
        .writes
        .clone()
        .into_updates_and_generated_ids();
    backend_tx.apply_function_runner_tx(
        *begin_timestamp,
        reads,
        num_intervals,
        user_tx_size,
        system_tx_size,
        updates,
        generated_ids,
        rows_read,
        rows_read_by_tablet,
    )?;

    assert_transactions_match(backend_tx, function_runner_tx)?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_apply_function_runner_tx_merge_existing_writes(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let db = new_test_database(rt).await;
    let mut backend_tx = db.begin_system().await?;
    // Make writes before initializing funrun transaction
    UserFacingModel::new_root_for_test(&mut backend_tx)
        .insert("table".parse()?, obj!("field" => "value")?)
        .await?;
    let begin_timestamp = backend_tx.begin_timestamp();

    // Create a new tx as though it were in funrun
    let mut function_runner_tx = db
        .begin_with_ts(
            Identity::system(),
            *begin_timestamp,
            FunctionUsageTracker::new(),
        )
        .await?;
    let (updates, generated_ids) = backend_tx.writes().clone().into_updates_and_generated_ids();
    function_runner_tx.merge_writes(updates, generated_ids)?;

    // Perform writes as if in funrun
    UserFacingModel::new_root_for_test(&mut function_runner_tx)
        .insert("table2".parse()?, obj!("foo" => "bla")?)
        .await?;

    // Apply reads and writes to the backend_tx
    let num_intervals = function_runner_tx.reads.num_intervals();
    let user_tx_size = function_runner_tx.reads.user_tx_size().clone();
    let system_tx_size = function_runner_tx.reads.system_tx_size().clone();
    let reads = function_runner_tx.reads.clone().into_read_set();
    let rows_read = function_runner_tx
        .stats()
        .iter()
        .map(|(table, stats)| (*table, stats.rows_read))
        .collect();
    let rows_read_by_tablet = function_runner_tx
        .stats_by_tablet()
        .iter()
        .map(|(table, stats)| (*table, stats.rows_read))
        .collect();
    let (updates, generated_ids) = function_runner_tx
        .writes
        .clone()
        .into_updates_and_generated_ids();
    backend_tx.apply_function_runner_tx(
        *begin_timestamp,
        reads,
        num_intervals,
        user_tx_size,
        system_tx_size,
        updates,
        generated_ids,
        rows_read,
        rows_read_by_tablet,
    )?;

    assert_transaction_writes_match(&backend_tx, &function_runner_tx)?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_apply_function_runner_tx_merge_existing_writes_bad(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let db = new_test_database(rt).await;
    let mut backend_tx = db.begin_system().await?;
    // Make writes before initializing funrun transaction
    UserFacingModel::new_root_for_test(&mut backend_tx)
        .insert("table".parse()?, obj!("field" => "value")?)
        .await?;
    let begin_timestamp = backend_tx.begin_timestamp();

    // Create a new tx as though it were in funrun
    // but without applying existing writes: BAD!
    let mut function_runner_tx = db
        .begin_with_ts(
            Identity::system(),
            *begin_timestamp,
            FunctionUsageTracker::new(),
        )
        .await?;

    // Perform writes as if in funrun
    UserFacingModel::new_root_for_test(&mut function_runner_tx)
        .insert("table2".parse()?, obj!("foo" => "bla")?)
        .await?;

    // Apply reads and writes to the backend_tx
    let num_intervals = function_runner_tx.reads.num_intervals();
    let user_tx_size = function_runner_tx.reads.user_tx_size().clone();
    let system_tx_size = function_runner_tx.reads.system_tx_size().clone();
    let reads = function_runner_tx.reads.clone().into_read_set();
    let rows_read = function_runner_tx
        .stats()
        .iter()
        .map(|(table, stats)| (*table, stats.rows_read))
        .collect();
    let rows_read_by_tablet = function_runner_tx
        .stats_by_tablet()
        .iter()
        .map(|(table, stats)| (*table, stats.rows_read))
        .collect();
    let (updates, generated_ids) = function_runner_tx
        .writes
        .clone()
        .into_updates_and_generated_ids();
    assert!(backend_tx
        .apply_function_runner_tx(
            *begin_timestamp,
            reads,
            num_intervals,
            user_tx_size,
            system_tx_size,
            updates,
            generated_ids,
            rows_read,
            rows_read_by_tablet,
        )
        .is_err());

    Ok(())
}
