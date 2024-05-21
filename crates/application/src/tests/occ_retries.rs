use anyhow::Context;
use common::pause::{
    PauseClient,
    PauseController,
};
use database::{
    SystemMetadataModel,
    Transaction,
    UserFacingModel,
    MAX_OCC_FAILURES,
};
use errors::ErrorMetadataAnyhowExt;
use keybroker::Identity;
use model::deployment_audit_log::types::DeploymentAuditLogEvent;
use runtime::testing::TestRuntime;
use value::{
    obj,
    ConvexValue,
    ResolvedDocumentId,
    TableName,
};

use crate::{
    test_helpers::ApplicationTestExt,
    Application,
};

async fn test_replace_tx(
    tx: &mut Transaction<TestRuntime>,
    id: ResolvedDocumentId,
    value: ConvexValue,
) -> anyhow::Result<((), Vec<DeploymentAuditLogEvent>)> {
    UserFacingModel::new_root_for_test(tx)
        .replace(id.into(), obj!("name" => value)?)
        .await?;
    Ok(((), vec![]))
}

async fn test_replace_with_retries(
    application: &Application<TestRuntime>,
    pause_client: PauseClient,
    id: ResolvedDocumentId,
    value: ConvexValue,
) -> anyhow::Result<()> {
    application
        .execute_with_audit_log_events_and_occ_retries_with_pause_client(
            Identity::system(),
            pause_client,
            "test",
            move |tx| test_replace_tx(tx, id, value.clone()).into(),
        )
        .await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_occ_fails(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let identity = Identity::system();
    let mut tx = application.begin(identity.clone()).await?;
    let table_name: TableName = "_test_table".parse()?;
    tx.create_system_table_testing(&table_name, None).await?;
    let id = SystemMetadataModel::new_global(&mut tx)
        .insert(&table_name, obj!()?)
        .await?;
    application.commit_test(tx).await?;

    let (mut pause, pause_client) = PauseController::new(["retry_tx_loop_start"]);
    let fut1 = test_replace_with_retries(&application, pause_client, id, "value".try_into()?);

    let fut2 = async {
        for _i in 0..MAX_OCC_FAILURES {
            let mut guard = pause
                .wait_for_blocked("retry_tx_loop_start")
                .await
                .context("Didn't hit breakpoint?")?;
            let mut tx = application.begin(identity.clone()).await?;
            test_replace_tx(&mut tx, id, "value2".try_into()?).await?;
            application.commit_test(tx).await?;
            guard.unpause();
        }
        Ok::<_, anyhow::Error>(())
    };
    let err = futures::try_join!(fut1, fut2).unwrap_err();
    assert!(err.is_occ());
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_occ_succeeds(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let identity = Identity::system();
    let mut tx = application.begin(identity.clone()).await?;
    let table_name: TableName = "_test_table".parse()?;
    tx.create_system_table_testing(&table_name, None).await?;
    let id = SystemMetadataModel::new_global(&mut tx)
        .insert(&table_name, obj!()?)
        .await?;
    application.commit_test(tx).await?;

    let (mut pause, pause_client) = PauseController::new(["retry_tx_loop_start"]);
    let fut1 = test_replace_with_retries(&application, pause_client, id, "value".try_into()?);

    let fut2 = async {
        for i in 0..MAX_OCC_FAILURES {
            let mut guard = pause
                .wait_for_blocked("retry_tx_loop_start")
                .await
                .context("Didn't hit breakpoint?")?;
            if i < MAX_OCC_FAILURES - 1 {
                let mut tx = application.begin(identity.clone()).await?;
                test_replace_tx(&mut tx, id, "value2".try_into()?).await?;
                application.commit_test(tx).await?;
            }
            guard.unpause();
        }
        Ok::<_, anyhow::Error>(())
    };
    futures::try_join!(fut1, fut2)?;
    Ok(())
}
