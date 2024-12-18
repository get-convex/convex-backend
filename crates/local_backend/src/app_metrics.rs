use axum::{
    extract::State,
    response::IntoResponse,
};
use common::{
    components::{
        ComponentFunctionPath,
        ComponentPath,
    },
    http::{
        extract::{
            Json,
            Query,
        },
        HttpResponseError,
    },
    types::{
        UdfIdentifier,
        UdfType,
    },
};
use serde::Deserialize;
use sync_types::UdfPath;

use crate::{
    authentication::ExtractIdentity,
    LocalAppState,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UdfRateQueryArgs {
    component_path: Option<String>,
    #[serde(alias = "path")]
    udf_path: String,
    metric: String,
    window: String,
    udf_type: Option<String>,
}

pub(crate) async fn udf_rate(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Query(UdfRateQueryArgs {
        component_path,
        udf_path,
        metric,
        window,
        udf_type,
    }): Query<UdfRateQueryArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let window_json: serde_json::Value =
        serde_json::from_str(&window).map_err(anyhow::Error::new)?;
    let window = window_json.try_into()?;
    let udf_identifier = parse_udf_identifier(udf_type, component_path, udf_path)?;

    let timeseries = st
        .application
        .udf_rate(identity, udf_identifier, metric.parse()?, window)
        .await?;
    Ok(Json(timeseries))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CacheHitPercentageQueryArgs {
    component_path: Option<String>,
    #[serde(alias = "path")]
    udf_path: String,
    window: String,
    udf_type: Option<String>,
}
pub(crate) async fn cache_hit_percentage(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Query(query_args): Query<CacheHitPercentageQueryArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let window_json: serde_json::Value =
        serde_json::from_str(&query_args.window).map_err(anyhow::Error::new)?;
    let window = window_json.try_into()?;
    let udf_identifier = parse_udf_identifier(
        query_args.udf_type,
        query_args.component_path,
        query_args.udf_path,
    )?;
    let timeseries = st
        .application
        .cache_hit_percentage(identity, udf_identifier, window)
        .await?;
    Ok(Json(timeseries))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LatencyPercentilesQueryArgs {
    component_path: Option<String>,
    #[serde(alias = "path")]
    udf_path: String,
    percentiles: String,
    window: String,
    udf_type: Option<String>,
}
pub(crate) async fn latency_percentiles(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Query(query_args): Query<LatencyPercentilesQueryArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let udf_identifier = parse_udf_identifier(
        query_args.udf_type,
        query_args.component_path,
        query_args.udf_path,
    )?;
    let percentiles: Vec<usize> =
        serde_json::from_str(&query_args.percentiles).map_err(anyhow::Error::new)?;
    let window_json: serde_json::Value =
        serde_json::from_str(&query_args.window).map_err(anyhow::Error::new)?;
    let window = window_json.try_into()?;
    let timeseries: Vec<_> = st
        .application
        .latency_percentiles(identity, udf_identifier, percentiles, window)
        .await?
        .into_iter()
        .collect();
    Ok(Json(timeseries))
}

#[derive(Deserialize)]
pub(crate) struct TableRateQueryArgs {
    name: String,
    metric: String,
    window: String,
}
pub(crate) async fn table_rate(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Query(query_args): Query<TableRateQueryArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let name = query_args.name.parse()?;
    let metric = query_args.metric.parse()?;
    let window_json: serde_json::Value =
        serde_json::from_str(&query_args.window).map_err(anyhow::Error::new)?;
    let window = window_json.try_into()?;
    let timeseries = st
        .application
        .table_rate(identity, name, metric, window)
        .await?;
    Ok(Json(timeseries))
}

fn parse_udf_identifier(
    udf_type: Option<String>,
    component_path: Option<String>,
    identifier: String,
) -> anyhow::Result<UdfIdentifier> {
    let component = ComponentPath::deserialize(component_path.as_deref())?;
    let udf_identifier = match udf_type {
        Some(udf_type) => {
            let udf_type: UdfType = udf_type.parse()?;
            match udf_type {
                UdfType::HttpAction => UdfIdentifier::Http(identifier.parse()?),
                _ => {
                    let udf_path: UdfPath = identifier.parse()?;
                    let path = ComponentFunctionPath {
                        component,
                        udf_path,
                    };
                    UdfIdentifier::Function(path.canonicalize())
                },
            }
        },
        None => {
            let udf_path: UdfPath = identifier.parse()?;
            let path = ComponentFunctionPath {
                component,
                udf_path,
            };
            UdfIdentifier::Function(path.canonicalize())
        },
    };
    Ok(udf_identifier)
}

#[derive(Deserialize)]
pub(crate) struct ScheduledJobLagArgs {
    window: String,
}
pub(crate) async fn scheduled_job_lag(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Query(query_args): Query<ScheduledJobLagArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let window_json: serde_json::Value =
        serde_json::from_str(&query_args.window).map_err(anyhow::Error::new)?;
    let window = window_json.try_into()?;
    let timeseries = st.application.scheduled_job_lag(identity, window).await?;
    Ok(Json(timeseries))
}
