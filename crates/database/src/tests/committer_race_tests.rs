use common::pause::PauseController;
use keybroker::Identity;
use runtime::testing::TestRuntime;
use value::{
    assert_obj,
    TableName,
    TableNamespace,
};

use crate::{
    committer::AFTER_PENDING_WRITE_SNAPSHOT,
    test_helpers::{
        DbFixtures,
        DbFixturesArgs,
    },
    TestFacingModel,
};

#[convex_macro::test_runtime]
async fn test_table_summary_recompute_pending_writes(
    rt: TestRuntime,
    pause: PauseController,
) -> anyhow::Result<()> {
    // Create a database without table summaries bootstrapped
    let DbFixtures { db, .. } = DbFixtures::new_with_args(
        &rt,
        DbFixturesArgs {
            bootstrap_table_summaries: false,
            ..Default::default()
        },
    )
    .await?;

    // Insert a document so that table summaries should have content.
    let mut tx = db.begin(Identity::system()).await?;
    let table_name: TableName = "test_table".parse()?;
    let _ = TestFacingModel::new(&mut tx)
        .insert_and_get(table_name.clone(), assert_obj!("field" => "value"))
        .await?;
    db.commit(tx).await?;

    // Start a new transaction that doesn't have the table summaries in the snapshot
    // when it is first added to pending_writes.
    let mut tx_without_summaries = db.begin_system().await?;
    let _ = TestFacingModel::new(&mut tx_without_summaries)
        .insert_and_get(table_name.clone(), assert_obj!("field" => "value"))
        .await?;
    let db_clone = db.clone();
    let commit_fut = async move {
        db_clone.commit(tx_without_summaries).await?;
        anyhow::Ok(())
    };

    let hold_guard = pause.hold(AFTER_PENDING_WRITE_SNAPSHOT);
    let db_clone = db.clone();
    let finish_bootstrap_fut = async move {
        let pause_guard = hold_guard.wait_for_blocked().await;
        db_clone.finish_table_summary_bootstrap().await?;
        if let Some(pause_guard) = pause_guard {
            pause_guard.unpause();
        }
        anyhow::Ok(())
    };
    futures::try_join!(commit_fut, finish_bootstrap_fut)?;

    // Start another transaction to verify table summaries were included after
    // recomputing pending_writes.
    let mut tx2 = db.begin(Identity::system()).await?;
    let count = tx2
        .must_count(TableNamespace::root_component(), &table_name)
        .await?;
    assert_eq!(count, 2);
    Ok(())
}
