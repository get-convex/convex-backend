use std::str::FromStr;

use common::{
    pause::PauseClient,
    runtime::Runtime,
    types::{
        AllowedVisibility,
        FunctionCaller,
    },
};
use database::{
    TableModel,
    Transaction,
};
use futures::FutureExt;
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
use request_context::RequestContext;
use runtime::prod::ProdRuntime;
use serde_json::Value as JsonValue;
use sync_types::UdfPath;
use value::{
    GenericDocumentId,
    TableIdAndTableNumber,
};

use crate::{
    test_helpers::{
        eventually_succeed,
        ApplicationTestExt,
        OBJECTS_TABLE,
    },
    Application,
};

fn udf_path() -> UdfPath {
    UdfPath::from_str("basic:insertObject").unwrap()
}

async fn create_scheduled_job<'a>(
    rt: &'a ProdRuntime,
    tx: &'a mut Transaction<ProdRuntime>,
) -> anyhow::Result<(
    GenericDocumentId<TableIdAndTableNumber>,
    SchedulerModel<'a, ProdRuntime>,
)> {
    let mut map = serde_json::Map::new();
    map.insert(
        "key".to_string(),
        serde_json::Value::String("value".to_string()),
    );
    let mut model = SchedulerModel::new(tx);
    let udf_path = udf_path();
    let job_id = model
        .schedule(
            udf_path.clone(),
            parse_udf_args(&udf_path, vec![JsonValue::Object(map)])?,
            rt.unix_timestamp(),
            RequestContext::new_for_test(),
        )
        .await?;
    let state = model.check_status(job_id).await?.unwrap();
    assert_eq!(state, ScheduledJobState::Pending);
    Ok((job_id, model))
}

#[convex_macro::prod_rt_test]
async fn test_scheduled_jobs_success(rt: ProdRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let mut tx = application.begin(Identity::system()).await?;
    let (job_id, _model) = create_scheduled_job(&rt, &mut tx).await?;
    assert!(
        TableModel::new(&mut tx)
            .table_is_empty(&OBJECTS_TABLE)
            .await?
    );

    application.commit_test(tx).await?;

    // Scheduled jobs executor within application will pick up the job and execute
    // it. Add some wait time to make this less racy.
    let fut = move || {
        let application = application.clone();
        async move {
            let mut tx = application.begin(Identity::system()).await?;
            let mut model = SchedulerModel::new(&mut tx);
            let state = model.check_status(job_id).await?.unwrap();
            anyhow::ensure!(state == ScheduledJobState::Success);
            anyhow::ensure!(
                !TableModel::new(&mut tx)
                    .table_is_empty(&OBJECTS_TABLE)
                    .await?
            );
            Ok(())
        }
        .boxed()
    };
    eventually_succeed(rt, fut).await?;
    Ok(())
}

#[convex_macro::prod_rt_test]
async fn test_scheduled_jobs_canceled(rt: ProdRuntime) -> anyhow::Result<()> {
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
    let udf_path = udf_path();
    model
        .cancel_all(Some(udf_path.canonicalize().to_string()), 1)
        .await?;
    let state = model.check_status(job_id).await?.unwrap();
    assert_eq!(state, ScheduledJobState::Canceled);
    application.commit_test(tx).await?;

    Ok(())
}

#[convex_macro::prod_rt_test]
async fn test_scheduled_jobs_race_condition(rt: ProdRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let mut tx = application.begin(Identity::system()).await?;

    let (_job_id, mut model) = create_scheduled_job(&rt, &mut tx).await?;
    let jobs = model.list().await?;
    assert_eq!(jobs.len(), 1);
    let (job_id, job) = jobs[0].clone().into_id_and_value();

    // Cancel the scheduled job
    let udf_path = udf_path();
    model
        .cancel_all(Some(udf_path.canonicalize().to_string()), 1)
        .await?;

    application.commit_test(tx).await?;

    // This simulates the race condition where the job executor picks up a job to
    // execute after the job was created but before it was canceled. We should
    // handle the race condition gracefully.
    application
        .test_one_off_scheduled_job_executor_run(job, job_id)
        .await?;
    Ok(())
}

#[convex_macro::prod_rt_test]
async fn test_scheduled_jobs_garbage_collection(rt: ProdRuntime) -> anyhow::Result<()> {
    std::env::set_var("SCHEDULED_JOB_RETENTION", "2");
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let mut tx = application.begin(Identity::system()).await?;

    let (job_id, _model) = create_scheduled_job(&rt, &mut tx).await?;
    assert!(
        TableModel::new(&mut tx)
            .table_is_empty(&OBJECTS_TABLE)
            .await?
    );

    application.commit_test(tx).await?;

    // Scheduled jobs executor within application will pick up the job and execute
    // it. Add some wait time to make this less racy.
    let application_clone = application.clone();
    let f = move || {
        let application = application_clone.clone();
        async move {
            let mut tx = application.begin(Identity::system()).await?;
            let mut model = SchedulerModel::new(&mut tx);
            let state = model.check_status(job_id).await?.unwrap();
            anyhow::ensure!(state == ScheduledJobState::Success);
            anyhow::ensure!(
                !TableModel::new(&mut tx)
                    .table_is_empty(&OBJECTS_TABLE)
                    .await?
            );
            Ok(())
        }
        .boxed()
    };
    eventually_succeed(rt.clone(), f).await?;

    // Wait for garbage collector to clean up the job
    let f = move || {
        let application = application.clone();
        async move {
            let mut tx = application.begin(Identity::system()).await?;
            let mut model = SchedulerModel::new(&mut tx);
            let state = model.check_status(job_id).await?;
            anyhow::ensure!(state.is_none());
            Ok(())
        }
        .boxed()
    };
    eventually_succeed(rt, f).await?;

    Ok(())
}

#[convex_macro::prod_rt_test]
async fn test_pause_scheduled_jobs(rt: ProdRuntime) -> anyhow::Result<()> {
    test_scheduled_jobs_helper(rt, BackendState::Paused).await?;

    Ok(())
}

#[convex_macro::prod_rt_test]
async fn test_disable_scheduled_jobs(rt: ProdRuntime) -> anyhow::Result<()> {
    test_scheduled_jobs_helper(rt, BackendState::Disabled).await?;

    Ok(())
}

async fn test_scheduled_jobs_helper(
    rt: ProdRuntime,
    backend_state: BackendState,
) -> anyhow::Result<()> {
    // Helper for testing the functionality of changing the backend state
    let application = Application::new_for_tests(&rt).await?;
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

    // Scheduled jobs executor within application would pick up the job and execute
    // it. Wait a second to avoid a race. Job should be pending since the backend is
    // paused.
    let application_clone = application.clone();
    let f = move || {
        let application = application_clone.clone();
        async move {
            let mut tx = application.begin(Identity::system()).await?;
            let mut model = SchedulerModel::new(&mut tx);
            let state = model.check_status(job_id).await?.unwrap();
            anyhow::ensure!(state == ScheduledJobState::Pending);
            anyhow::ensure!(
                TableModel::new(&mut tx)
                    .table_is_empty(&OBJECTS_TABLE)
                    .await?
            );
            Ok(())
        }
        .boxed()
    };
    eventually_succeed(rt.clone(), f).await?;

    // Resuming the backend should allow the job to be executed.
    let mut tx = application.begin(Identity::system()).await?;
    let mut model = BackendStateModel::new(&mut tx);
    model.toggle_backend_state(BackendState::Running).await?;
    application.commit_test(tx).await?;
    let f = move || {
        let application = application.clone();
        async move {
            let mut tx = application.begin(Identity::system()).await?;
            let mut model = SchedulerModel::new(&mut tx);
            let state = model.check_status(job_id).await?.unwrap();
            anyhow::ensure!(state == ScheduledJobState::Success);
            anyhow::ensure!(
                !TableModel::new(&mut tx)
                    .table_is_empty(&OBJECTS_TABLE)
                    .await?
            );
            Ok(())
        }
        .boxed()
    };
    eventually_succeed(rt.clone(), f).await?;

    Ok(())
}

#[convex_macro::prod_rt_test]
async fn test_cancel_recursively_scheduled_job(rt: ProdRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    // Schedule and cancel a job
    let mut tx = application.begin(Identity::system()).await?;
    let (job_id, mut model) = create_scheduled_job(&rt, &mut tx).await?;
    model.complete(job_id, ScheduledJobState::Canceled).await?;
    application.commit_test(tx).await?;

    // Run a mutation that has a canceled parent job and schedules
    let parent_scheduled_job = Some(job_id.into());
    let context = RequestContext::new(parent_scheduled_job);
    application
        .mutation_udf(
            UdfPath::from_str("scheduler:scheduleWithArbitraryJson")?,
            vec![],
            Identity::system(),
            None,
            AllowedVisibility::All,
            FunctionCaller::Action,
            PauseClient::new(),
            RequestContext::new(Some(job_id.into())),
        )
        .await??;

    // Run an action in v8 that has a canceled parent job and schedules
    application
        .action_udf(
            UdfPath::from_str("action:schedule")?,
            vec![],
            Identity::system(),
            AllowedVisibility::All,
            FunctionCaller::Action,
            context,
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
