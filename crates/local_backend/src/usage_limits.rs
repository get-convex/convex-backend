use std::collections::BTreeMap;

use axum::{
    extract::FromRef,
    response::IntoResponse,
};
use common::{
    http::{
        extract::{
            Json,
            MtState,
            Path,
        },
        ExtractRequestMetadata,
        HttpResponseError,
    },
    runtime::Runtime,
};
use errors::ErrorMetadata;
use http::StatusCode;
use model::{
    deployment_audit_log::types::DeploymentAuditLogEvent,
    usage_limits::{
        types::{
            MetricUnit,
            UsageLimitConfig,
            UsageLimitMetric,
            UsageLimitType,
            UsageLimitWindow,
        },
        UsageLimitsModel,
        USAGE_LIMITS_TABLE,
    },
};
use roles::RequireDeploymentOp;
use serde::{
    Deserialize,
    Serialize,
};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use value::{
    id_v6::DeveloperDocumentId,
    TableNamespace,
};

use crate::{
    authentication::ExtractIdentity,
    parse::parse_document_id,
    LocalAppState,
};

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UsageLimitConfigRequest {
    name: Option<String>,
    metric: String,
    window: String,
    limit_type: String,
    #[schema(minimum = 1)]
    limit: u64,
    enabled: bool,
}

impl UsageLimitConfigRequest {
    fn into_usage_limit_config(self) -> anyhow::Result<UsageLimitConfig> {
        let config = UsageLimitConfig {
            name: self.name,
            metric: parse_usage_limit_metric(self.metric)?,
            window: parse_usage_limit_window(self.window)?,
            limit_type: parse_usage_limit_type(self.limit_type)?,
            limit: self.limit,
            enabled: self.enabled,
        };
        config.validate()?;
        Ok(config)
    }
}

#[derive(Serialize, Deserialize, ToSchema, PartialEq, Eq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UsageLimitConfigResponse {
    id: String,
    name: Option<String>,
    metric: String,
    window: String,
    limit_type: String,
    #[schema(minimum = 1)]
    limit: u64,
    enabled: bool,
}

impl From<common::document::ParsedDocument<UsageLimitConfig>> for UsageLimitConfigResponse {
    fn from(doc: common::document::ParsedDocument<UsageLimitConfig>) -> Self {
        let id = String::from(DeveloperDocumentId::from(doc.id()));
        let config = doc.into_value();
        Self {
            id,
            name: config.name,
            metric: config.metric.to_string(),
            window: config.window.to_string(),
            limit_type: config.limit_type.to_string(),
            limit: config.limit,
            enabled: config.enabled,
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListUsageLimitsResponse {
    usage_limits: Vec<UsageLimitConfigResponse>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UsageLimitResponse {
    usage_limit: UsageLimitConfigResponse,
}

/// Current-window usage for a single metric.
#[derive(Serialize, Deserialize, ToSchema, PartialEq, Debug, Clone)]
pub struct MetricUsageResponse {
    /// The unit `usage` is expressed in, matching the unit this metric's
    /// configured limits use.
    unit: MetricUnit,
    usage: WindowUsageResponse,
}

/// Usage in each calendar-aligned window currently in progress.
#[derive(Serialize, Deserialize, ToSchema, PartialEq, Debug, Clone)]
pub struct WindowUsageResponse {
    current_day: f64,
    current_month: f64,
}

/// Current usage across every metric, keyed by the same metric name the usage
/// limit config API uses.
#[derive(Serialize, Deserialize, ToSchema, PartialEq, Debug, Clone)]
pub struct GetCurrentUsageResponse {
    metrics: BTreeMap<String, MetricUsageResponse>,
}

/// Get current usage
///
/// Get the current usage for each metric, in each in-progress window (the
/// current day and calendar month).
#[utoipa::path(
    get,
    path = "/get_current_usage",
    tag = "Usage Limits",
    tags = ["beta"],
    responses(
        (status = 200, body = GetCurrentUsageResponse)
    ),
    security(
        ("Deploy Key" = []),
        ("OAuth Team Token" = []),
        ("Team Token" = []),
        ("OAuth Project Token" = []),
    ),
)]
pub async fn get_current_usage(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    identity.require_operation(keybroker::DeploymentOp::ViewUsage)?;

    let meter = st.application.usage_meter();
    let now = st.application.runtime().system_time();
    let metrics = meter
        .usage_snapshot(now)?
        .into_iter()
        .map(|(metric, usage)| {
            (
                metric.to_string(),
                MetricUsageResponse {
                    unit: metric.unit(),
                    usage: WindowUsageResponse {
                        current_day: metric.usage_in_display_units(usage.current_day),
                        current_month: metric.usage_in_display_units(usage.current_month),
                    },
                },
            )
        })
        .collect();

    Ok(Json(GetCurrentUsageResponse { metrics }))
}

/// List usage limits
///
/// Get all usage limit configs for a deployment.
#[utoipa::path(
    get,
    path = "/list_usage_limits",
    tag = "Usage Limits",
    responses(
        (status = 200, body = ListUsageLimitsResponse)
    ),
    security(
        ("Deploy Key" = []),
        ("OAuth Team Token" = []),
        ("Team Token" = []),
        ("OAuth Project Token" = []),
    ),
)]
pub async fn list_usage_limits(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    identity.require_operation(keybroker::DeploymentOp::ViewUsageLimits)?;

    let mut tx = st.application.begin(identity).await?;
    let usage_limits = UsageLimitsModel::new(&mut tx)
        .list()
        .await?
        .into_iter()
        .map(UsageLimitConfigResponse::from)
        .collect();

    Ok(Json(ListUsageLimitsResponse { usage_limits }))
}

/// Create usage limit
///
/// Create a new usage limit config for a deployment.
#[utoipa::path(
    post,
    path = "/create_usage_limit",
    tag = "Usage Limits",
    request_body = UsageLimitConfigRequest,
    responses((status = 200, body = UsageLimitResponse)),
    security(
        ("Deploy Key" = []),
        ("OAuth Team Token" = []),
        ("Team Token" = []),
        ("OAuth Project Token" = []),
    ),
)]
pub async fn create_usage_limit(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    ExtractRequestMetadata(request_metadata): ExtractRequestMetadata,
    Json(req): Json<UsageLimitConfigRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    identity.require_operation(keybroker::DeploymentOp::WriteUsageLimits)?;

    let mut tx = st.application.begin(identity).await?;
    let config = req.into_usage_limit_config()?;
    let id = UsageLimitsModel::new(&mut tx).create(config).await?;
    let created = UsageLimitsModel::new(&mut tx)
        .get(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!(usage_limit_not_found()))?;
    let audit_events = vec![DeploymentAuditLogEvent::CreateUsageLimit {
        id: String::from(DeveloperDocumentId::from(id)),
        config: created.clone().into_value(),
    }];
    let usage_limit = created.into();

    st.application
        .commit_with_audit_log_events(tx, audit_events, request_metadata, "create_usage_limit")
        .await?;

    Ok(Json(UsageLimitResponse { usage_limit }))
}

/// Update usage limit
///
/// Replace an existing usage limit config for a deployment.
#[utoipa::path(
    post,
    path = "/update_usage_limit/{id}",
    tag = "Usage Limits",
    params(
        ("id" = String, Path, description = "id of the usage limit to update"),
    ),
    request_body = UsageLimitConfigRequest,
    responses((status = 200, body = UsageLimitResponse)),
    security(
        ("Deploy Key" = []),
        ("OAuth Team Token" = []),
        ("Team Token" = []),
        ("OAuth Project Token" = []),
    ),
)]
pub async fn update_usage_limit(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    ExtractRequestMetadata(request_metadata): ExtractRequestMetadata,
    Path(id): Path<String>,
    Json(req): Json<UsageLimitConfigRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    identity.require_operation(keybroker::DeploymentOp::WriteUsageLimits)?;

    let mut tx = st.application.begin(identity).await?;
    let id = parse_document_id(
        &id,
        &tx.table_mapping().namespace(TableNamespace::Global),
        &USAGE_LIMITS_TABLE,
    )?;
    let previous = UsageLimitsModel::new(&mut tx)
        .get(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!(usage_limit_not_found()))?
        .into_value();
    let config = req.into_usage_limit_config()?;
    UsageLimitsModel::new(&mut tx).replace(id, config).await?;
    let updated = UsageLimitsModel::new(&mut tx)
        .get(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!(usage_limit_not_found()))?;
    let audit_events: Vec<DeploymentAuditLogEvent> =
        vec![DeploymentAuditLogEvent::UpdateUsageLimit {
            id: String::from(DeveloperDocumentId::from(id)),
            previous,
            current: updated.clone().into_value(),
        }];
    let usage_limit = updated.into();

    st.application
        .commit_with_audit_log_events(tx, audit_events, request_metadata, "update_usage_limit")
        .await?;

    Ok(Json(UsageLimitResponse { usage_limit }))
}

/// Delete usage limit
///
/// Delete an existing usage limit config for a deployment.
#[utoipa::path(
    post,
    path = "/delete_usage_limit/{id}",
    tag = "Usage Limits",
    params(
        ("id" = String, Path, description = "id of the usage limit to delete"),
    ),
    responses((status = 200)),
    security(
        ("Deploy Key" = []),
        ("OAuth Team Token" = []),
        ("Team Token" = []),
        ("OAuth Project Token" = []),
    ),
)]
pub async fn delete_usage_limit(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    ExtractRequestMetadata(request_metadata): ExtractRequestMetadata,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, HttpResponseError> {
    identity.require_operation(keybroker::DeploymentOp::WriteUsageLimits)?;

    let mut tx = st.application.begin(identity).await?;
    let id = parse_document_id(
        &id,
        &tx.table_mapping().namespace(TableNamespace::Global),
        &USAGE_LIMITS_TABLE,
    )?;
    let Some(config) = UsageLimitsModel::new(&mut tx).delete(id).await? else {
        return Err(anyhow::anyhow!(usage_limit_not_found()).into());
    };
    let audit_events = vec![DeploymentAuditLogEvent::DeleteUsageLimit {
        id: String::from(DeveloperDocumentId::from(id)),
        config,
    }];

    st.application
        .commit_with_audit_log_events(tx, audit_events, request_metadata, "delete_usage_limit")
        .await?;

    Ok(StatusCode::OK)
}

fn usage_limit_not_found() -> ErrorMetadata {
    ErrorMetadata::not_found("UsageLimitNotFound", "The usage limit couldn't be found.")
}

fn parse_usage_limit_metric(metric: String) -> anyhow::Result<UsageLimitMetric> {
    metric.parse().map_err(|_| {
        ErrorMetadata::bad_request(
            "InvalidUsageLimitMetric",
            format!("Invalid usage limit metric: {metric}"),
        )
        .into()
    })
}

fn parse_usage_limit_window(window: String) -> anyhow::Result<UsageLimitWindow> {
    window.parse().map_err(|_| {
        ErrorMetadata::bad_request(
            "InvalidUsageLimitWindow",
            format!("Invalid usage limit window: {window}"),
        )
        .into()
    })
}

fn parse_usage_limit_type(limit_type: String) -> anyhow::Result<UsageLimitType> {
    limit_type.parse().map_err(|_| {
        ErrorMetadata::bad_request(
            "InvalidUsageLimitType",
            format!("Invalid usage limit type: {limit_type}"),
        )
        .into()
    })
}

pub fn platform_router<S>() -> OpenApiRouter<S>
where
    LocalAppState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    OpenApiRouter::new()
        .routes(utoipa_axum::routes!(get_current_usage))
        .routes(utoipa_axum::routes!(list_usage_limits))
        .routes(utoipa_axum::routes!(create_usage_limit))
        .routes(utoipa_axum::routes!(update_usage_limit))
        .routes(utoipa_axum::routes!(delete_usage_limit))
}
