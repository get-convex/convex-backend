use std::{
    collections::BTreeMap,
    str::FromStr,
    time::Duration,
};

use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentId,
        ComponentPath,
    },
    query::{
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
    },
    runtime::Runtime,
};
use database::{
    query::TableFilter,
    DeveloperQuery,
    TableModel,
    Transaction,
};
use keybroker::Identity;
use model::{
    backend_state::{
        types::BackendState,
        BackendStateModel,
    },
    cron_jobs::{
        types::{
            CronIdentifier,
            CronJob,
            CronSchedule,
            CronSpec,
        },
        CronModel,
        CRON_JOB_LOGS_INDEX_BY_NAME_TS,
        CRON_JOB_LOGS_NAME_FIELD,
    },
};
use runtime::testing::TestRuntime;
use serde_json::Value as JsonValue;
use udf::helpers::parse_udf_args;

use crate::{
    test_helpers::{
        ApplicationTestExt,
        OBJECTS_TABLE,
        OBJECTS_TABLE_COMPONENT,
    },
    Application,
};

fn test_cron_identifier() -> CronIdentifier {
    CronIdentifier::from_str("test").unwrap()
}

async fn create_cron_job(
    tx: &mut Transaction<TestRuntime>,
) -> anyhow::Result<(BTreeMap<CronIdentifier, CronJob>, CronModel<TestRuntime>)> {
    let mut cron_model = CronModel::new(tx, ComponentId::test_user());
    let mut map = serde_json::Map::new();
    map.insert(
        "key".to_string(),
        serde_json::Value::String("value".to_string()),
    );
    let path = CanonicalizedComponentFunctionPath {
        component: ComponentPath::test_user(),
        udf_path: "basic:insertObject".parse()?,
    };
    let cron_spec = CronSpec {
        udf_path: path.udf_path.clone(),
        udf_args: parse_udf_args(&path.udf_path, vec![JsonValue::Object(map)])?,
        cron_schedule: CronSchedule::Interval { seconds: 60 },
    };
    let original_jobs = cron_model.list().await?;
    let name = test_cron_identifier();
    cron_model.create(name, cron_spec).await?;
    Ok((original_jobs, cron_model))
}

fn cron_log_query<RT: Runtime>(
    tx: &mut Transaction<RT>,
    component: ComponentId,
) -> anyhow::Result<DeveloperQuery<RT>> {
    DeveloperQuery::new(
        tx,
        component.into(),
        Query::index_range(IndexRange {
            index_name: CRON_JOB_LOGS_INDEX_BY_NAME_TS.name(),
            range: vec![IndexRangeExpression::Eq(
                CRON_JOB_LOGS_NAME_FIELD.clone(),
                common::types::MaybeValue(Some(test_cron_identifier().to_string().try_into()?)),
            )],
            order: Order::Asc,
        }),
        TableFilter::IncludePrivateSystemTables,
    )
}

#[convex_macro::test_runtime]
pub(crate) async fn test_cron_jobs_success(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;
    // udf-tests include crons, so we let them execute so that we can then add
    // a new cron without hitting an OCC.
    rt.wait(Duration::from_secs(100)).await;

    let mut tx = application.begin(Identity::system()).await?;

    let (original_jobs, mut cron_model) = create_cron_job(&mut tx).await?;

    let jobs = cron_model.list().await?;
    assert_eq!(jobs.len(), original_jobs.len() + 1);

    let mut table_model = TableModel::new(&mut tx);
    assert!(
        table_model
            .table_is_empty(OBJECTS_TABLE_COMPONENT.into(), &OBJECTS_TABLE)
            .await?
    );

    application.commit_test(tx).await?;

    // Cron jobs executor within application will pick up the job and
    // execute it. Add some wait time to make this less racy.
    rt.wait(Duration::from_secs(100)).await;
    let mut tx = application.begin(Identity::system()).await?;
    let mut table_model = TableModel::new(&mut tx);
    assert!(
        !table_model
            .table_is_empty(OBJECTS_TABLE_COMPONENT.into(), &OBJECTS_TABLE)
            .await?
    );
    let mut logs_query = cron_log_query(&mut tx, OBJECTS_TABLE_COMPONENT)?;
    assert!(logs_query.next(&mut tx, None).await?.is_some());
    Ok(())
}

#[convex_macro::test_runtime]
pub(crate) async fn test_cron_jobs_race_condition(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;
    // udf-tests include crons, so we let them execute so that we can then add
    // a new cron without hitting an OCC.
    rt.wait(Duration::from_secs(100)).await;

    let mut tx = application.begin(Identity::system()).await?;
    let (original_jobs, mut model) = create_cron_job(&mut tx).await?;

    let jobs = model.list().await?;
    assert_eq!(jobs.len(), original_jobs.len() + 1);
    let job = jobs.get(&test_cron_identifier()).unwrap();

    // Delete the cron job
    let job_metadata = model
        .list_metadata()
        .await?
        .remove(&test_cron_identifier())
        .unwrap();
    model.delete(job_metadata).await?;
    let jobs = model.list().await?;
    assert_eq!(jobs.len(), original_jobs.len());

    application.commit_test(tx).await?;

    // This simulates the race condition where the job executor picks up a cron
    // to execute after the cron was created but before it was deleted. We should
    // handle the race condition gracefully instead of trying to run the stale cron.
    application
        .test_one_off_cron_job_executor_run(job.clone())
        .await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_paused_cron_jobs(rt: TestRuntime) -> anyhow::Result<()> {
    test_cron_jobs_helper(rt, BackendState::Paused).await?;

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_disable_cron_jobs(rt: TestRuntime) -> anyhow::Result<()> {
    test_cron_jobs_helper(rt, BackendState::Disabled).await?;

    Ok(())
}

async fn test_cron_jobs_helper(rt: TestRuntime, backend_state: BackendState) -> anyhow::Result<()> {
    // Helper for testing behavior for pausing or disabling backends
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    // Change backend state
    let mut tx = application.begin(Identity::system()).await?;
    let mut backend_state_model = BackendStateModel::new(&mut tx);
    backend_state_model
        .toggle_backend_state(backend_state)
        .await?;
    application.commit_test(tx).await?;

    let mut tx = application.begin(Identity::system()).await?;
    let (original_jobs, mut cron_model) = create_cron_job(&mut tx).await?;
    let jobs = cron_model.list().await?;
    assert_eq!(jobs.len(), original_jobs.len() + 1);
    let mut table_model = TableModel::new(&mut tx);
    assert!(
        table_model
            .table_is_empty(OBJECTS_TABLE_COMPONENT.into(), &OBJECTS_TABLE)
            .await?
    );
    application.commit_test(tx).await?;

    // Cron jobs executor within application will pick up the job and
    // execute it. Add some wait time to make this less racy. Job should not execute
    // because the backend is paused.
    rt.wait(Duration::from_secs(100)).await;
    let mut tx = application.begin(Identity::system()).await?;
    let mut table_model = TableModel::new(&mut tx);
    assert!(
        table_model
            .table_is_empty(OBJECTS_TABLE_COMPONENT.into(), &OBJECTS_TABLE)
            .await?
    );
    let mut logs_query = cron_log_query(&mut tx, ComponentId::test_user())?;
    assert!(logs_query.next(&mut tx, Some(1)).await?.is_none());

    // Resuming the backend should make the jobs execute.
    let mut model = BackendStateModel::new(&mut tx);
    model.toggle_backend_state(BackendState::Running).await?;
    application.commit_test(tx).await?;
    rt.wait(Duration::from_secs(100)).await;
    let mut tx = application.begin(Identity::system()).await?;
    let mut table_model = TableModel::new(&mut tx);
    assert!(
        !table_model
            .table_is_empty(OBJECTS_TABLE_COMPONENT.into(), &OBJECTS_TABLE)
            .await?
    );
    let mut logs_query = cron_log_query(&mut tx, ComponentId::Root)?;
    assert!(logs_query.next(&mut tx, None).await?.is_some());

    Ok(())
}
