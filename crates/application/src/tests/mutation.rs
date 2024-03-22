use anyhow::Context;
use common::{
    knobs::UDF_EXECUTOR_OCC_MAX_RETRIES,
    pause::{
        PauseClient,
        PauseController,
    },
    request_context::RequestId,
    types::{
        AllowedVisibility,
        FunctionCaller,
    },
};
use errors::ErrorMetadataAnyhowExt;
use keybroker::Identity;
use runtime::testing::TestRuntime;
use serde_json::{
    json,
    Value as JsonValue,
};

use crate::{
    test_helpers::ApplicationTestExt,
    Application,
};

async fn insert_object(
    application: &Application<TestRuntime>,
    pause_client: PauseClient,
) -> anyhow::Result<JsonValue> {
    let obj = json!({"an": "object"});
    let result = application
        .mutation_udf(
            RequestId::new(),
            "basic:insertObject".parse()?,
            vec![obj],
            Identity::system(),
            None,
            AllowedVisibility::PublicOnly,
            FunctionCaller::Action {
                parent_scheduled_job: None,
            },
            pause_client,
        )
        .await??;
    Ok(JsonValue::from(result.value))
}

async fn insert_and_count(
    application: &Application<TestRuntime>,
    pause_client: PauseClient,
) -> anyhow::Result<usize> {
    let obj = json!({"an": "object"});
    let result = application
        .mutation_udf(
            RequestId::new(),
            "basic:insertAndCount".parse()?,
            vec![obj],
            Identity::system(),
            None,
            AllowedVisibility::PublicOnly,
            FunctionCaller::Action {
                parent_scheduled_job: None,
            },
            pause_client,
        )
        .await??;
    Ok(JsonValue::from(result.value)
        .as_f64()
        .context("Expected f64 result")? as usize)
}

#[convex_macro::test_runtime]
async fn test_mutation(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;
    let result = insert_object(&application, PauseClient::new()).await?;
    assert_eq!(result["an"], "object");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_mutation_occ_fail(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let (mut pause, pause_client) = PauseController::new(["retry_mutation_loop_start"]);
    let fut1 = insert_and_count(&application, pause_client);
    let fut2 = async {
        for i in 0..*UDF_EXECUTOR_OCC_MAX_RETRIES + 1 {
            let mut guard = pause
                .wait_for_blocked("retry_mutation_loop_start")
                .await
                .context("Didn't hit breakpoint?")?;

            // Do an entire mutation while we're paused - to create an OCC conflict on
            // the original insertion.
            let count = insert_and_count(&application, PauseClient::new()).await?;
            assert_eq!(count, i + 1);

            guard.unpause();
        }
        Ok::<_, anyhow::Error>(())
    };
    let err = futures::try_join!(fut1, fut2).unwrap_err();
    assert!(err.is_occ());
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_mutation_occ_success(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let (mut pause, pause_client) = PauseController::new(["retry_mutation_loop_start"]);
    let fut1 = insert_and_count(&application, pause_client);
    let fut2 = async {
        for i in 0..*UDF_EXECUTOR_OCC_MAX_RETRIES + 1 {
            let mut guard = pause
                .wait_for_blocked("retry_mutation_loop_start")
                .await
                .context("Didn't hit breakpoint?")?;

            // N-1 retries, Nth one allow it to succeed
            if i < *UDF_EXECUTOR_OCC_MAX_RETRIES {
                // Do an entire mutation while we're paused - to create an OCC conflict on
                // the original insertion.
                let count = insert_and_count(&application, PauseClient::new()).await?;
                assert_eq!(count, i + 1);
            }

            guard.unpause();
        }
        Ok::<_, anyhow::Error>(())
    };
    let (count, ()) = futures::try_join!(fut1, fut2)?;

    // one for each of the conflicting transactions + one more for the success at
    // the end
    assert_eq!(count, *UDF_EXECUTOR_OCC_MAX_RETRIES + 1);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_multiple_inserts_dont_occ(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    // Insert an object to create the table (otherwise it'll OCC on table creation).
    insert_object(&application, PauseClient::new()).await?;

    let (mut pause, pause_client) = PauseController::new(["retry_mutation_loop_start"]);
    let fut1 = insert_object(&application, pause_client);
    let fut2 = async {
        let mut guard = pause
            .wait_for_blocked("retry_mutation_loop_start")
            .await
            .context("Didn't hit breakpoint?")?;

        // Do several entire mutations while we're paused. Shouldn't OCC.
        for _ in 0..5 {
            let result = insert_object(&application, PauseClient::new()).await?;
            assert_eq!(result["an"], "object");
        }

        guard.unpause();
        Ok::<_, anyhow::Error>(())
    };
    let (result, ()) = futures::try_join!(fut1, fut2)?;
    assert_eq!(result["an"], "object");
    Ok(())
}
