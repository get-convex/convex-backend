use std::{
    str::FromStr,
    time::Duration,
};

use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentId,
        ComponentPath,
        PublicFunctionPath,
    },
    execution_context::ExecutionContext,
    pause::{
        HoldGuard,
        PauseController,
    },
    runtime::Runtime,
    types::FunctionCaller,
    RequestId,
};
use database::{
    BootstrapComponentsModel,
    TableModel,
    Transaction,
};
use errors::ErrorMetadata;
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
use sync_types::CanonicalizedUdfPath;
use udf::helpers::parse_udf_args;
use value::{
    ResolvedDocumentId,
    TableNamespace,
};

use crate::{
    scheduled_jobs::{
        SCHEDULED_JOB_COMMITTING,
        SCHEDULED_JOB_EXECUTED,
    },
    test_helpers::{
        ApplicationTestExt,
        OBJECTS_TABLE,
        OBJECTS_TABLE_COMPONENT,
    },
    Application,
};

fn insert_object_path() -> CanonicalizedComponentFunctionPath {
    CanonicalizedComponentFunctionPath {
        component: ComponentPath::test_user(),
        udf_path: CanonicalizedUdfPath::from_str("basic:insertObject").unwrap(),
    }
}

async fn create_scheduled_job<'a>(
    rt: &'a TestRuntime,
    tx: &'a mut Transaction<TestRuntime>,
    path: CanonicalizedComponentFunctionPath,
) -> anyhow::Result<(ResolvedDocumentId, SchedulerModel<'a, TestRuntime>)> {
    let mut map = serde_json::Map::new();
    map.insert(
        "key".to_string(),
        serde_json::Value::String("value".to_string()),
    );
    let (_, component) =
        BootstrapComponentsModel::new(tx).must_component_path_to_ids(&path.component)?;
    let mut model = SchedulerModel::new(tx, component.into());
    let job_id = model
        .schedule(
            path.clone(),
            parse_udf_args(&path.udf_path, vec![JsonValue::Object(map)])?,
            rt.unix_timestamp(),
            ExecutionContext::new_for_test(),
        )
        .await?;
    let state = model.check_status(job_id).await?.unwrap();
    assert_eq!(state, ScheduledJobState::Pending);
    Ok((job_id, model))
}

/// Waits for scheduled job to execute and unpauses the scheduled job executor.
async fn wait_for_scheduled_job_execution(hold_guard: HoldGuard) {
    if let Some(pause_guard) = hold_guard.wait_for_blocked().await {
        pause_guard.unpause();
    }
}

#[convex_macro::test_runtime]
async fn test_scheduled_jobs_success(
    rt: TestRuntime,
    pause_controller: PauseController,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let hold_guard = pause_controller.hold(SCHEDULED_JOB_EXECUTED);

    let mut tx = application.begin(Identity::system()).await?;
    let (job_id, _model) = create_scheduled_job(&rt, &mut tx, insert_object_path()).await?;
    assert!(
        TableModel::new(&mut tx)
            .table_is_empty(OBJECTS_TABLE_COMPONENT.into(), &OBJECTS_TABLE)
            .await?
    );

    application.commit_test(tx).await?;

    wait_for_scheduled_job_execution(hold_guard).await;
    tx = application.begin(Identity::system()).await?;
    let mut model = SchedulerModel::new(&mut tx, TableNamespace::test_user());
    let state = model.check_status(job_id).await?.unwrap();
    assert_eq!(state, ScheduledJobState::Success);
    assert!(
        !TableModel::new(&mut tx)
            .table_is_empty(OBJECTS_TABLE_COMPONENT.into(), &OBJECTS_TABLE)
            .await?
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_scheduled_jobs_canceled(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let mut tx = application.begin(Identity::system()).await?;

    let path = insert_object_path();
    let (_job_id, mut model) = create_scheduled_job(&rt, &mut tx, path.clone()).await?;
    let jobs = model.list().await?;
    assert_eq!(jobs.len(), 1);
    let (job_id, job) = jobs[0].clone().into_id_and_value();
    assert_eq!(job.state, ScheduledJobState::Pending);
    assert!(job.next_ts.is_some());

    // Cancel the scheduled job
    model.cancel_all(Some(path), 1, None, None).await?;
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

    let path = insert_object_path();
    let (_job_id, mut model) = create_scheduled_job(&rt, &mut tx, path.clone()).await?;
    let jobs = model.list().await?;
    assert_eq!(jobs.len(), 1);
    let (job_id, job) = jobs[0].clone().into_id_and_value();

    // Cancel the scheduled job
    model.cancel_all(Some(path), 1, None, None).await?;

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
async fn test_scheduled_jobs_garbage_collection(
    rt: TestRuntime,
    pause_controller: PauseController,
) -> anyhow::Result<()> {
    // TODO: this is sketchy and could interfere with other tests in this process
    unsafe { std::env::set_var("SCHEDULED_JOB_RETENTION", "30") };
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let hold_guard = pause_controller.hold(SCHEDULED_JOB_EXECUTED);

    let mut tx = application.begin(Identity::system()).await?;

    let (job_id, _model) = create_scheduled_job(&rt, &mut tx, insert_object_path()).await?;
    assert!(
        TableModel::new(&mut tx)
            .table_is_empty(OBJECTS_TABLE_COMPONENT.into(), &OBJECTS_TABLE)
            .await?
    );

    application.commit_test(tx).await?;

    wait_for_scheduled_job_execution(hold_guard).await;
    tx = application.begin(Identity::system()).await?;
    let mut model = SchedulerModel::new(&mut tx, TableNamespace::test_user());
    let state = model.check_status(job_id).await?.unwrap();
    assert_eq!(state, ScheduledJobState::Success);
    assert!(
        !TableModel::new(&mut tx)
            .table_is_empty(OBJECTS_TABLE_COMPONENT.into(), &OBJECTS_TABLE)
            .await?
    );

    // Wait for garbage collector to clean up the job
    rt.wait(Duration::from_secs(60)).await;
    tx = application.begin(Identity::system()).await?;
    let state = SchedulerModel::new(&mut tx, TableNamespace::test_user())
        .check_status(job_id)
        .await?;
    assert!(state.is_none());

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_pause_scheduled_jobs(
    rt: TestRuntime,
    pause_controller: PauseController,
) -> anyhow::Result<()> {
    test_scheduled_jobs_helper(rt, BackendState::Paused, pause_controller).await?;

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_disable_scheduled_jobs(
    rt: TestRuntime,
    pause_controller: PauseController,
) -> anyhow::Result<()> {
    test_scheduled_jobs_helper(rt, BackendState::Disabled, pause_controller).await?;

    Ok(())
}

/// Helper for testing the functionality of changing the backend state
async fn test_scheduled_jobs_helper(
    rt: TestRuntime,
    backend_state: BackendState,
    pause_controller: PauseController,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let scheduled_job_executed_hold_guard = pause_controller.hold(SCHEDULED_JOB_EXECUTED);

    let mut tx = application.begin(Identity::system()).await?;
    let mut backend_state_model = BackendStateModel::new(&mut tx);
    backend_state_model
        .toggle_backend_state(backend_state)
        .await?;
    let (job_id, _model) = create_scheduled_job(&rt, &mut tx, insert_object_path()).await?;
    assert!(
        TableModel::new(&mut tx)
            .table_is_empty(OBJECTS_TABLE_COMPONENT.into(), &OBJECTS_TABLE)
            .await?
    );

    application.commit_test(tx).await?;

    tx = application.begin(Identity::system()).await?;
    let mut model = SchedulerModel::new(&mut tx, TableNamespace::test_user());
    let state = model.check_status(job_id).await?.unwrap();
    assert_eq!(state, ScheduledJobState::Pending);
    assert!(
        TableModel::new(&mut tx)
            .table_is_empty(OBJECTS_TABLE_COMPONENT.into(), &OBJECTS_TABLE)
            .await?
    );

    // Resuming the backend should allow the job to be executed.
    let mut model = BackendStateModel::new(&mut tx);
    model.toggle_backend_state(BackendState::Running).await?;
    application.commit_test(tx).await?;
    wait_for_scheduled_job_execution(scheduled_job_executed_hold_guard).await;
    tx = application.begin(Identity::system()).await?;
    let mut model = SchedulerModel::new(&mut tx, TableNamespace::test_user());
    let state = model.check_status(job_id).await?.unwrap();
    assert_eq!(state, ScheduledJobState::Success);
    assert!(
        !TableModel::new(&mut tx)
            .table_is_empty(OBJECTS_TABLE_COMPONENT.into(), &OBJECTS_TABLE)
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
    let (job_id, mut model) = create_scheduled_job(&rt, &mut tx, insert_object_path()).await?;
    model.complete(job_id, ScheduledJobState::Canceled).await?;
    application.commit_test(tx).await?;

    // Run a mutation that has a canceled parent job and schedules
    let parent_scheduled_job = Some((ComponentId::test_user(), job_id.into()));
    application
        .mutation_udf(
            RequestId::new(),
            PublicFunctionPath::Component(CanonicalizedComponentFunctionPath {
                component: ComponentPath::test_user(),
                udf_path: CanonicalizedUdfPath::from_str("scheduler:scheduleWithArbitraryJson")?,
            }),
            vec![],
            Identity::system(),
            None,
            FunctionCaller::Action {
                parent_scheduled_job,
                parent_execution_id: None,
            },
            None,
        )
        .await??;

    // Run an action in v8 that has a canceled parent job and schedules
    application
        .action_udf(
            RequestId::new(),
            PublicFunctionPath::Component(CanonicalizedComponentFunctionPath {
                component: ComponentPath::test_user(),
                udf_path: CanonicalizedUdfPath::from_str("action:schedule")?,
            }),
            vec![],
            Identity::system(),
            FunctionCaller::Action {
                parent_scheduled_job,
                parent_execution_id: None,
            },
        )
        .await??;

    let mut tx = application.begin(Identity::system()).await?;
    let mut model = SchedulerModel::new(&mut tx, TableNamespace::test_user());
    let list = model.list().await?;
    assert_eq!(list.len(), 3);
    assert!(list
        .iter()
        .all(|job| job.state == ScheduledJobState::Canceled));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_scheduled_job_retry(
    rt: TestRuntime,
    pause_controller: PauseController,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let attempt_commit = pause_controller.hold(SCHEDULED_JOB_COMMITTING);
    let attempt_execute = pause_controller.hold(SCHEDULED_JOB_EXECUTED);

    let mut tx = application.begin(Identity::system()).await?;
    let (job_id, _model) = create_scheduled_job(&rt, &mut tx, insert_object_path()).await?;
    application.commit_test(tx).await?;

    // Simulate a failure in the scheduled job
    let mut pause_guard = attempt_commit.wait_for_blocked().await.unwrap();
    pause_guard.inject_error(anyhow::anyhow!(ErrorMetadata::user_occ(
        None, None, None, None
    )));
    // Pause the next attempt as well.
    let second_attempt_commit = pause_controller.hold(SCHEDULED_JOB_COMMITTING);
    pause_guard.unpause();

    // Wait for the first attempt, which will fail.
    // Hitting this label means the scheduler thread is freed up temporarily,
    // so more jobs can execute while this one is backing off.
    let pause_guard = attempt_execute.wait_for_blocked().await.unwrap();
    let second_attempt_execute = pause_controller.hold(SCHEDULED_JOB_EXECUTED);
    pause_guard.unpause();
    // The second attempt throws no error.
    let pause_guard = second_attempt_commit.wait_for_blocked().await.unwrap();
    pause_guard.unpause();
    // Wait for the second attempt, which will succeed.
    let pause_guard = second_attempt_execute.wait_for_blocked().await.unwrap();
    pause_guard.unpause();

    let mut tx = application.begin(Identity::system()).await?;
    let mut model = SchedulerModel::new(&mut tx, TableNamespace::test_user());
    let state = model.check_status(job_id).await?.unwrap();
    assert_eq!(state, ScheduledJobState::Success);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_delete_scheduled_jobs_table(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let mut tx = application.begin(Identity::system()).await?;
    create_scheduled_job(&rt, &mut tx, insert_object_path()).await?;
    application.commit_test(tx).await?;

    let mut tx = application.begin(Identity::system()).await?;
    let mut model = SchedulerModel::new(&mut tx, TableNamespace::test_user());
    let scheduled_jobs = model.list().await?;
    assert_eq!(scheduled_jobs.len(), 1);

    application
        .delete_scheduled_jobs_table(Identity::system(), ComponentId::Root)
        .await?;
    let mut tx = application.begin(Identity::system()).await?;
    let mut model = SchedulerModel::new(&mut tx, TableNamespace::test_user());
    let scheduled_jobs = model.list().await?;
    assert!(scheduled_jobs.is_empty());

    Ok(())
}
