use std::str::FromStr;

use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentPath,
    },
    execution_context::ExecutionContext,
    runtime::Runtime,
};
use database::{
    BootstrapComponentsModel,
    Transaction,
};
use runtime::testing::TestRuntime;
use sync_types::CanonicalizedUdfPath;
use value::{
    assert_obj,
    ConvexValue,
    ResolvedDocumentId,
};

use crate::scheduled_jobs::{
    types::ScheduledJobState,
    SchedulerModel,
};

pub fn insert_object_path() -> CanonicalizedComponentFunctionPath {
    CanonicalizedComponentFunctionPath {
        component: ComponentPath::test_user(),
        udf_path: CanonicalizedUdfPath::from_str("basic:insertObject").unwrap(),
    }
}
pub async fn create_scheduled_job<'a>(
    rt: &'a TestRuntime,
    tx: &'a mut Transaction<TestRuntime>,
    path: CanonicalizedComponentFunctionPath,
) -> anyhow::Result<(ResolvedDocumentId, SchedulerModel<'a, TestRuntime>)> {
    create_scheduled_job_with_args(
        rt,
        tx,
        path,
        vec![ConvexValue::Object(assert_obj!("key" => "value"))],
    )
    .await
}

pub async fn create_scheduled_job_with_args<'a>(
    rt: &'a TestRuntime,
    tx: &'a mut Transaction<TestRuntime>,
    path: CanonicalizedComponentFunctionPath,
    args: Vec<ConvexValue>,
) -> anyhow::Result<(ResolvedDocumentId, SchedulerModel<'a, TestRuntime>)> {
    let (_, component) =
        BootstrapComponentsModel::new(tx).must_component_path_to_ids(&path.component)?;
    let mut model = SchedulerModel::new(tx, component.into());
    let job_id = model
        .schedule(
            path.clone(),
            args.try_into()?,
            rt.unix_timestamp(),
            ExecutionContext::new_for_test(),
        )
        .await?;
    let state = model.check_status(job_id).await?.unwrap();
    assert_eq!(state, ScheduledJobState::Pending);
    Ok((job_id, model))
}
