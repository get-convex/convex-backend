use std::{
    str::FromStr,
    time::Duration,
};

use common::{
    components::{
        ComponentFunctionPath,
        ComponentId,
    },
    execution_context::ExecutionContext,
    pause::{
        PauseClient,
        PauseController,
    },
    runtime::Runtime,
    types::FunctionCaller,
    RequestId,
};
use database::{
    TableModel,
    Transaction,
};
use isolate::parse_udf_args;
use keybroker::Identity;
use model::{
    backend_state::{
        types::BackendState,
        BackendStateModel,
    },
    scheduled_jobs::{
        types::ScheduledJobState,
        SchedulerModel,
    },
};
use runtime::testing::TestRuntime;
use serde_json::Value as JsonValue;
use sync_types::UdfPath;
use value::{
    GenericDocumentId,
    TabletIdAndTableNumber,
};

use crate::{
    scheduled_jobs::SCHEDULED_JOB_EXECUTED,
    test_helpers::{
        ApplicationFixtureArgs,
        ApplicationTestExt,
        OBJECTS_TABLE,
    },
    Application,
};

fn function_path() -> ComponentFunctionPath {
    ComponentFunctionPath {
        component: ComponentId::Root,
        udf_path: UdfPath::from_str("basic:insertObject").unwrap(),
    }
}

async fn create_scheduled_job<'a>(
    rt: &'a TestRuntime,
    tx: &'a mut Transaction<TestRuntime>,
) -> anyhow::Result<(
    GenericDocumentId<TabletIdAndTableNumber>,
    SchedulerModel<'a, TestRuntime>,
)> {
    let mut map = serde_json::Map::new();
    map.insert(
        "key".to_string(),
        serde_json::Value::String("value".to_string()),
    );
    let mut model = SchedulerModel::new(tx);
    let path = function_path();
    let job_id = model
        .schedule(
            path.clone(),
            parse_udf_args(&path, vec![JsonValue::Object(map)])?,
            rt.unix_timestamp(),
            ExecutionContext::new_for_test(),
        )
        .await?;
    let state = model.check_status(job_id).await?.unwrap();
    assert_eq!(state, ScheduledJobState::Pending);
    Ok((job_id, model))
}

/// Waits for scheduled job to execute and unpauses the scheduled job executor.
async fn wait_for_scheduled_job_execution(mut pause_controller: PauseController) {
    if let Some(mut pause_guard) = pause_controller
        .wait_for_blocked(SCHEDULED_JOB_EXECUTED)
        .await
    {
        pause_guard.unpause();
    }
}

#[convex_macro::test_runtime]
async fn test_scheduled_jobs_success(rt: TestRuntime) -> anyhow::Result<()> {
    let (args, pause_controller) = ApplicationFixtureArgs::with_scheduled_jobs_pause_client();
    let application = Application::new_for_tests_with_args(&rt, args).await?;
    application.load_udf_tests_modules().await?;

    let mut tx = application.begin(Identity::system()).await?;
    let (job_id, _model) = create_scheduled_job(&rt, &mut tx).await?;
    assert!(
        TableModel::new(&mut tx)
            .table_is_empty(&OBJECTS_TABLE)
            .await?
    );

    application.commit_test(tx).await?;

    wait_for_scheduled_job_execution(pause_controller).await;
    tx = application.begin(Identity::system()).await?;
    let mut model = SchedulerModel::new(&mut tx);
    let state = model.check_status(job_id).await?.unwrap();
    assert_eq!(state, ScheduledJobState::Success);
    assert!(
        !TableModel::new(&mut tx)
            .table_is_empty(&OBJECTS_TABLE)
            .await?
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_scheduled_jobs_canceled(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let mut tx = application.begin(Identity::system()).await?;

    let (_job_id, mut model) = create_scheduled_job(&rt, &mut tx).await?;
    let jobs = model.list().await?;
    assert_eq!(jobs.len(), 1);
    let (job_id, job) = jobs[0].clone().into_id_and_value();
    assert_eq!(job.state, ScheduledJobState::Pending);
    assert!(job.next_ts.is_some());

    // Cancel the scheduled job
    let path = function_path();
    model.cancel_all(Some(path.canonicalize()), 1).await?;
    let state = model.check_status(job_id).await?.unwrap();
    assert_eq!(state, ScheduledJobState::Canceled);
    application.commit_test(tx).await?;

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_scheduled_jobs_race_condition(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let mut tx = application.begin(Identity::system()).await?;

    let (_job_id, mut model) = create_scheduled_job(&rt, &mut tx).await?;
    let jobs = model.list().await?;
    assert_eq!(jobs.len(), 1);
    let (job_id, job) = jobs[0].clone().into_id_and_value();

    // Cancel the scheduled job
    let path = function_path();
    model.cancel_all(Some(path.canonicalize()), 1).await?;

    application.commit_test(tx).await?;

    // This simulates the race condition where the job executor picks up a job to
    // execute after the job was created but before it was canceled. We should
    // handle the race condition gracefully.
    application
        .test_one_off_scheduled_job_executor_run(job, job_id)
        .await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_scheduled_jobs_garbage_collection(rt: TestRuntime) -> anyhow::Result<()> {
    std::env::set_var("SCHEDULED_JOB_RETENTION", "30");
    let (args, pause_controller) = ApplicationFixtureArgs::with_scheduled_jobs_pause_client();
    let application = Application::new_for_tests_with_args(&rt, args).await?;
    application.load_udf_tests_modules().await?;

    let mut tx = application.begin(Identity::system()).await?;

    let (job_id, _model) = create_scheduled_job(&rt, &mut tx).await?;
    assert!(
        TableModel::new(&mut tx)
            .table_is_empty(&OBJECTS_TABLE)
            .await?
    );

    application.commit_test(tx).await?;

    wait_for_scheduled_job_execution(pause_controller).await;
    tx = application.begin(Identity::system()).await?;
    let mut model = SchedulerModel::new(&mut tx);
    let state = model.check_status(job_id).await?.unwrap();
    assert_eq!(state, ScheduledJobState::Success);
    assert!(
        !TableModel::new(&mut tx)
            .table_is_empty(&OBJECTS_TABLE)
            .await?
    );

    // Wait for garbage collector to clean up the job
    rt.wait(Duration::from_secs(60)).await;
    tx = application.begin(Identity::system()).await?;
    let state = SchedulerModel::new(&mut tx).check_status(job_id).await?;
    assert!(state.is_none());

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_pause_scheduled_jobs(rt: TestRuntime) -> anyhow::Result<()> {
    test_scheduled_jobs_helper(rt, BackendState::Paused).await?;

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_disable_scheduled_jobs(rt: TestRuntime) -> anyhow::Result<()> {
    test_scheduled_jobs_helper(rt, BackendState::Disabled).await?;

    Ok(())
}

/// Helper for testing the functionality of changing the backend state
async fn test_scheduled_jobs_helper(
    rt: TestRuntime,
    backend_state: BackendState,
) -> anyhow::Result<()> {
    let (args, pause_controller) = ApplicationFixtureArgs::with_scheduled_jobs_pause_client();
    let application = Application::new_for_tests_with_args(&rt, args).await?;
    application.load_udf_tests_modules().await?;

    let mut tx = application.begin(Identity::system()).await?;
    let mut backend_state_model = BackendStateModel::new(&mut tx);
    backend_state_model
        .toggle_backend_state(backend_state)
        .await?;
    let (job_id, _model) = create_scheduled_job(&rt, &mut tx).await?;
    assert!(
        TableModel::new(&mut tx)
            .table_is_empty(&OBJECTS_TABLE)
            .await?
    );

    application.commit_test(tx).await?;

    tx = application.begin(Identity::system()).await?;
    let mut model = SchedulerModel::new(&mut tx);
    let state = model.check_status(job_id).await?.unwrap();
    assert_eq!(state, ScheduledJobState::Pending);
    assert!(
        TableModel::new(&mut tx)
            .table_is_empty(&OBJECTS_TABLE)
            .await?
    );

    // Resuming the backend should allow the job to be executed.
    let mut model = BackendStateModel::new(&mut tx);
    model.toggle_backend_state(BackendState::Running).await?;
    application.commit_test(tx).await?;
    wait_for_scheduled_job_execution(pause_controller).await;
    tx = application.begin(Identity::system()).await?;
    let mut model = SchedulerModel::new(&mut tx);
    let state = model.check_status(job_id).await?.unwrap();
    assert_eq!(state, ScheduledJobState::Success);
    assert!(
        !TableModel::new(&mut tx)
            .table_is_empty(&OBJECTS_TABLE)
            .await?
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_cancel_recursively_scheduled_job(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    // Schedule and cancel a job
    let mut tx = application.begin(Identity::system()).await?;
    let (job_id, mut model) = create_scheduled_job(&rt, &mut tx).await?;
    model.complete(job_id, ScheduledJobState::Canceled).await?;
    application.commit_test(tx).await?;

    // Run a mutation that has a canceled parent job and schedules
    let parent_scheduled_job = Some(job_id.into());
    application
        .mutation_udf(
            RequestId::new(),
            ComponentFunctionPath {
                component: ComponentId::Root,
                udf_path: UdfPath::from_str("scheduler:scheduleWithArbitraryJson")?,
            },
            vec![],
            Identity::system(),
            None,
            FunctionCaller::Action {
                parent_scheduled_job,
            },
            PauseClient::new(),
        )
        .await??;

    // Run an action in v8 that has a canceled parent job and schedules
    application
        .action_udf(
            RequestId::new(),
            ComponentFunctionPath {
                component: ComponentId::Root,
                udf_path: UdfPath::from_str("action:schedule")?,
            },
            vec![],
            Identity::system(),
            FunctionCaller::Action {
                parent_scheduled_job,
            },
        )
        .await??;

    let mut tx = application.begin(Identity::system()).await?;
    let mut model = SchedulerModel::new(&mut tx);
    let list = model.list().await?;
    assert_eq!(list.len(), 3);
    assert!(list
        .iter()
        .all(|job| job.state == ScheduledJobState::Canceled));
    Ok(())
}
