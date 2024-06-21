use anyhow::Context;
use axum::{
    debug_handler,
    extract::State,
    response::IntoResponse,
};
use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentPath,
    },
    http::{
        extract::Json,
        HttpResponseError,
    },
};
use errors::ErrorMetadata;
use http::StatusCode;
use model::scheduled_jobs::{
    SchedulerModel,
    SCHEDULED_JOBS_TABLE,
};
use serde::{
    Deserialize,
    Serialize,
};
use value::TableNamespace;

use crate::{
    admin::bad_admin_key_error,
    authentication::ExtractIdentity,
    parse::parse_document_id,
    LocalAppState,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelAllJobsRequest {
    pub udf_path: Option<String>,
}

#[debug_handler]
pub async fn cancel_all_jobs(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(CancelAllJobsRequest { udf_path }): Json<CancelAllJobsRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    identity
        .member_id()
        .context(bad_admin_key_error(identity.instance_name()))?;

    let udf_path = udf_path
        .map(|p| p.parse())
        .transpose()
        .context(ErrorMetadata::bad_request(
            "InvaildUdfPath",
            "CancelAllJobs requires an optional canonicalized UdfPath",
        ))?;
    let path = udf_path.map(|udf_path| CanonicalizedComponentFunctionPath {
        component: ComponentPath::root(),
        udf_path,
    });
    st.application.cancel_all_jobs(path, identity).await?;

    Ok(StatusCode::OK)
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelJobRequest {
    pub id: String,
}

#[debug_handler]
pub async fn cancel_job(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(cancel_job_request): Json<CancelJobRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    identity
        .member_id()
        .context(bad_admin_key_error(identity.instance_name()))?;
    st.application
        .execute_with_audit_log_events_and_occ_retries(identity.clone(), "cancel_job", |tx| {
            async {
                let namespace = TableNamespace::by_component_TODO();
                let id = parse_document_id(
                    &cancel_job_request.id,
                    &tx.table_mapping().namespace(namespace),
                    &SCHEDULED_JOBS_TABLE,
                )?;

                let mut model = SchedulerModel::new(tx, namespace);
                model.cancel(id).await?;
                Ok(((), vec![]))
            }
            .into()
        })
        .await?;

    Ok(StatusCode::OK)
}
