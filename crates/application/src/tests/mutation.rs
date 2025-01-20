use std::sync::Arc;

use anyhow::Context;
use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentPath,
        PublicFunctionPath,
    },
    knobs::UDF_EXECUTOR_OCC_MAX_RETRIES,
    pause::PauseController,
    types::FunctionCaller,
    RequestId,
};
use errors::ErrorMetadataAnyhowExt;
use events::{
    testing::BasicTestUsageEventLogger,
    usage::{
        FunctionCallUsageFields,
        UsageEvent,
    },
};
use keybroker::Identity;
use runtime::testing::TestRuntime;
use serde_json::{
    json,
    Value as JsonValue,
};

use crate::{
    test_helpers::{
        ApplicationFixtureArgs,
        ApplicationTestExt,
    },
    Application,
};

async fn insert_object(application: &Application<TestRuntime>) -> anyhow::Result<JsonValue> {
    let obj = json!({"an": "object"});
    let result = application
        .mutation_udf(
            RequestId::new(),
            PublicFunctionPath::Component(CanonicalizedComponentFunctionPath {
                component: ComponentPath::test_user(),
                udf_path: "basic:insertObject".parse()?,
            }),
            vec![obj],
            Identity::system(),
            None,
            FunctionCaller::Action {
                parent_scheduled_job: None,
            },
        )
        .await??;
    Ok(JsonValue::from(result.value))
}

async fn insert_and_count(application: &Application<TestRuntime>) -> anyhow::Result<usize> {
    let obj = json!({"an": "object"});
    let result = application
        .mutation_udf(
            RequestId::new(),
            PublicFunctionPath::Component(CanonicalizedComponentFunctionPath {
                component: ComponentPath::test_user(),
                udf_path: "basic:insertAndCount".parse()?,
            }),
            vec![obj],
            Identity::system(),
            None,
            FunctionCaller::Action {
                parent_scheduled_job: None,
            },
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
    let result = insert_object(&application).await?;
    assert_eq!(result["an"], "object");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_mutation_occ_fail(rt: TestRuntime, pause: PauseController) -> anyhow::Result<()> {
    let logger = BasicTestUsageEventLogger::new();
    let application = Application::new_for_tests_with_args(
        &rt,
        ApplicationFixtureArgs::with_event_logger(Arc::new(logger.clone())),
    )
    .await?;
    application.load_udf_tests_modules().await?;

    let hold_guard = pause.hold("retry_mutation_loop_start");
    let fut1 = insert_and_count(&application);
    let fut2 = async {
        let mut hold_guard = hold_guard;
        for i in 0..*UDF_EXECUTOR_OCC_MAX_RETRIES + 1 {
            let guard = hold_guard
                .wait_for_blocked()
                .await
                .context("Didn't hit breakpoint?")?;

            // Do an entire mutation while we're paused - to create an OCC conflict on
            // the original insertion.
            let count = insert_and_count(&application).await?;
            assert_eq!(count, i + 1);

            hold_guard = pause.hold("retry_mutation_loop_start");
            guard.unpause();
        }
        Ok::<_, anyhow::Error>(())
    };
    let err = futures::try_join!(fut1, fut2).unwrap_err();
    assert!(err.is_occ());

    // Test that the usage events look good.
    let function_call_events: Vec<FunctionCallUsageFields> = logger
        .collect()
        .into_iter()
        .filter_map(|event| {
            if let UsageEvent::FunctionCall { fields } = event {
                if fields.udf_id == "basic.js:insertAndCount" {
                    Some(fields)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    // One for each of the conflicting transactions.
    assert_eq!(
        function_call_events.len(),
        (*UDF_EXECUTOR_OCC_MAX_RETRIES + 1) * 2,
    );
    for (index, event) in function_call_events.iter().enumerate() {
        if index % 2 == 0 {
            // The first event, and every other event after that should not be an OCC.
            assert!(!event.is_occ);
            assert!(event.is_tracked);
            assert!(event.occ_table_name.is_none());
            assert!(event.occ_document_id.is_none());
            assert!(event.occ_retry_count.is_none());
        } else {
            // The second event, and every other event after that should be an OCC.
            assert!(event.is_occ);
            assert!(event.is_tracked);
            // Only the second OCC will have a table name and document id.
            if index > 1 {
                assert!(event.occ_table_name.is_some());
                assert!(event.occ_document_id.is_some());
            } else {
                assert!(event.occ_table_name.is_none());
                assert!(event.occ_document_id.is_none());
            }
            assert_eq!(event.occ_retry_count.unwrap() as usize, index / 2);
        }
    }
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_mutation_occ_success(rt: TestRuntime, pause: PauseController) -> anyhow::Result<()> {
    let logger = BasicTestUsageEventLogger::new();
    let application = Application::new_for_tests_with_args(
        &rt,
        ApplicationFixtureArgs::with_event_logger(Arc::new(logger.clone())),
    )
    .await?;
    application.load_udf_tests_modules().await?;

    let hold_guard = pause.hold("retry_mutation_loop_start");
    let fut1 = insert_and_count(&application);
    let fut2 = async {
        let mut hold_guard = hold_guard;
        for i in 0..*UDF_EXECUTOR_OCC_MAX_RETRIES + 1 {
            let guard = hold_guard
                .wait_for_blocked()
                .await
                .context("Didn't hit breakpoint?")?;

            // N-1 retries, Nth one allow it to succeed
            if i < *UDF_EXECUTOR_OCC_MAX_RETRIES {
                // Do an entire mutation while we're paused - to create an OCC conflict on
                // the original insertion.
                let count = insert_and_count(&application).await?;
                assert_eq!(count, i + 1);
            }

            hold_guard = pause.hold("retry_mutation_loop_start");
            guard.unpause();
        }
        Ok::<_, anyhow::Error>(())
    };
    let (count, ()) = futures::try_join!(fut1, fut2)?;

    // one for each of the conflicting transactions + one more for the success at
    // the end
    assert_eq!(count, *UDF_EXECUTOR_OCC_MAX_RETRIES + 1);

    // Test that the usage events look good.
    let function_call_events: Vec<FunctionCallUsageFields> = logger
        .collect()
        .into_iter()
        .filter_map(|event| {
            if let UsageEvent::FunctionCall { fields } = event {
                if fields.udf_id == "basic.js:insertAndCount" {
                    Some(fields)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        function_call_events.len(),
        *UDF_EXECUTOR_OCC_MAX_RETRIES * 2 + 1,
    );
    for (index, event) in function_call_events.iter().enumerate() {
        if index % 2 == 0 {
            // The first event, and every other event after that should not be an OCC.
            assert!(!event.is_occ);
            assert!(event.is_tracked);
            assert!(event.occ_table_name.is_none());
            assert!(event.occ_document_id.is_none());
            assert!(event.occ_retry_count.is_none());
        } else {
            // The second event, and every other event after that should be an OCC.
            assert!(event.is_occ);
            assert!(event.is_tracked);
            // Only the second OCC will have a table name and document id.
            if index > 1 {
                assert!(event.occ_table_name.is_some());
                assert!(event.occ_document_id.is_some());
            } else {
                assert!(event.occ_table_name.is_none());
                assert!(event.occ_document_id.is_none());
            }
            assert_eq!(event.occ_retry_count.unwrap() as usize, index / 2);
        }
    }
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_multiple_inserts_dont_occ(
    rt: TestRuntime,
    pause: PauseController,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    // Insert an object to create the table (otherwise it'll OCC on table creation).
    insert_object(&application).await?;

    let hold_guard = pause.hold("retry_mutation_loop_start");
    let fut1 = insert_object(&application);
    let fut2 = async {
        let guard = hold_guard
            .wait_for_blocked()
            .await
            .context("Didn't hit breakpoint?")?;

        // Do several entire mutations while we're paused. Shouldn't OCC.
        for _ in 0..5 {
            let result = insert_object(&application).await?;
            assert_eq!(result["an"], "object");
        }

        guard.unpause();
        Ok::<_, anyhow::Error>(())
    };
    let (result, ()) = futures::try_join!(fut1, fut2)?;
    assert_eq!(result["an"], "object");
    Ok(())
}
