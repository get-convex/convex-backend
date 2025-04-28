use std::{
    collections::BTreeMap,
    time::Duration,
};

use application::deploy_config::{
    FinishPushDiff,
    SchemaStatus,
    SchemaStatusJson,
    StartPushRequest,
    StartPushResponse,
};
use axum::{
    debug_handler,
    extract::State,
    response::IntoResponse,
};
use common::{
    auth::{
        AuthInfo,
        SerializedAuthInfo,
    },
    bootstrap_model::components::definition::SerializedComponentDefinitionMetadata,
    http::{
        extract::Json,
        HttpResponseError,
    },
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use fastrace::{
    collector::EventRecord,
    prelude::{
        SpanId,
        SpanRecord,
        TraceId,
    },
};
use model::{
    auth::types::AuthDiff,
    components::{
        config::{
            SerializedComponentDefinitionDiff,
            SerializedComponentDiff,
            SerializedSchemaChange,
        },
        type_checking::SerializedCheckedComponent,
        types::SerializedEvaluatedComponentDefinition,
    },
    external_packages::types::ExternalDepsPackageId,
    modules::module_versions::SerializedAnalyzedModule,
    source_packages::types::SourcePackage,
};
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use value::{
    base64,
    ConvexObject,
    DeveloperDocumentId,
};

use crate::{
    admin::{
        must_be_admin_from_key,
        must_be_admin_from_key_with_write_access,
    },
    LocalAppState,
};

impl TryFrom<StartPushResponse> for SerializedStartPushResponse {
    type Error = anyhow::Error;

    fn try_from(value: StartPushResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            environment_variables: value
                .environment_variables
                .into_iter()
                .map(|(k, v)| Ok((String::from(k), String::from(v))))
                .collect::<anyhow::Result<_>>()?,
            external_deps_id: value
                .external_deps_id
                .map(|id| String::from(DeveloperDocumentId::from(id))),
            component_definition_packages: value
                .component_definition_packages
                .into_iter()
                .map(|(k, v)| Ok((String::from(k), JsonValue::from(ConvexObject::try_from(v)?))))
                .collect::<anyhow::Result<_>>()?,
            app_auth: value
                .app_auth
                .into_iter()
                .map(SerializedAuthInfo::try_from)
                .collect::<anyhow::Result<_>>()?,
            analysis: value
                .analysis
                .into_iter()
                .map(|(k, v)| Ok((String::from(k), v.try_into()?)))
                .collect::<anyhow::Result<_>>()?,
            app: value.app.try_into()?,
            schema_change: value.schema_change.try_into()?,
        })
    }
}

impl TryFrom<SerializedStartPushResponse> for StartPushResponse {
    type Error = anyhow::Error;

    fn try_from(value: SerializedStartPushResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            environment_variables: value
                .environment_variables
                .into_iter()
                .map(|(k, v)| Ok((k.parse()?, v.parse()?)))
                .collect::<anyhow::Result<_>>()?,
            external_deps_id: value
                .external_deps_id
                .map(|id| {
                    anyhow::Ok(ExternalDepsPackageId::from(
                        id.parse::<DeveloperDocumentId>()?,
                    ))
                })
                .transpose()?,
            component_definition_packages: value
                .component_definition_packages
                .into_iter()
                .map(|(k, v)| {
                    Ok((
                        k.parse()?,
                        SourcePackage::try_from(ConvexObject::try_from(v)?)?,
                    ))
                })
                .collect::<anyhow::Result<_>>()?,
            app_auth: value
                .app_auth
                .into_iter()
                .map(AuthInfo::try_from)
                .collect::<anyhow::Result<_>>()?,
            analysis: value
                .analysis
                .into_iter()
                .map(|(k, v)| Ok((k.parse()?, v.try_into()?)))
                .collect::<anyhow::Result<_>>()?,
            app: value.app.try_into()?,
            schema_change: value.schema_change.try_into()?,
        })
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedStartPushResponse {
    environment_variables: BTreeMap<String, String>,

    // Pointers to uploaded code.
    external_deps_id: Option<String>,
    component_definition_packages: BTreeMap<String, JsonValue>,

    // Analysis results.
    app_auth: Vec<SerializedAuthInfo>,
    analysis: BTreeMap<String, SerializedEvaluatedComponentDefinition>,

    // Typechecking results.
    app: SerializedCheckedComponent,

    // Schema changes.
    schema_change: SerializedSchemaChange,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SerializedIndexDiff {
    added: Vec<String>,
    removed: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzedComponent {
    definition: SerializedComponentDefinitionMetadata,
    schema: Option<JsonValue>,
    modules: BTreeMap<String, SerializedAnalyzedModule>,
}

#[debug_handler]
pub async fn start_push(
    State(st): State<LocalAppState>,
    Json(req): Json<StartPushRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let _identity = must_be_admin_from_key_with_write_access(
        st.application.app_auth(),
        st.instance_name.clone(),
        req.admin_key.clone(),
    )
    .await?;
    let dry_run = req.dry_run;
    let config = req.into_project_config().map_err(|e| {
        anyhow::Error::new(ErrorMetadata::bad_request("InvalidConfig", e.to_string()))
    })?;
    let resp = st
        .application
        .start_push(&config, dry_run)
        .await
        .map_err(|e| e.wrap_error_message(|msg| format!("Hit an error while pushing:\n{msg}")))?;
    Ok(Json(SerializedStartPushResponse::try_from(resp)?))
}

const DEFAULT_SCHEMA_TIMEOUT_MS: u32 = 10_000;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitForSchemaRequest {
    admin_key: String,
    schema_change: SerializedSchemaChange,
    dry_run: bool,
    timeout_ms: Option<u32>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum WaitForSchemaResponse {
    InProgress {
        status: SchemaStatusJson,
    },
    Failed {
        error: String,
        table_name: Option<String>,
    },
    RaceDetected,
    Complete,
}

#[debug_handler]
pub async fn wait_for_schema(
    State(st): State<LocalAppState>,
    Json(req): Json<WaitForSchemaRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let identity = must_be_admin_from_key(
        st.application.app_auth(),
        st.instance_name.clone(),
        req.admin_key,
    )
    .await?;
    let timeout = Duration::from_millis(req.timeout_ms.unwrap_or(DEFAULT_SCHEMA_TIMEOUT_MS) as u64);
    let schema_change = req.schema_change.try_into()?;

    // We can't query schema in a dry run, since we didn't commit the schema changes
    // in a `start_push` dry run. Just return immediately.
    if req.dry_run {
        tracing::info!("Skipping wait_for_schema in dry run");
        return Ok(Json(SchemaStatusJson::from(SchemaStatus::Complete)));
    }

    let resp = st
        .application
        .wait_for_schema(identity, schema_change, timeout)
        .await?;
    Ok(Json(SchemaStatusJson::from(resp)))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishPushRequest {
    admin_key: String,
    start_push: SerializedStartPushResponse,
    dry_run: bool,
}

#[debug_handler]
pub async fn finish_push(
    State(st): State<LocalAppState>,
    Json(req): Json<FinishPushRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let identity = must_be_admin_from_key_with_write_access(
        st.application.app_auth(),
        st.instance_name.clone(),
        req.admin_key.clone(),
    )
    .await?;

    let start_push = StartPushResponse::try_from(req.start_push)?;

    // We can't actually run `finish_push` in a dry run, since we rolled back all of
    // our changes during start push.
    if req.dry_run {
        tracing::info!("Skipping finish_push in dry run");
        let empty_diff = FinishPushDiff::default();
        return Ok(Json(SerializedFinishPushDiff::try_from(empty_diff)?));
    }

    let resp = st
        .application
        .finish_push(identity, start_push)
        .await
        .map_err(|e| e.wrap_error_message(|msg| format!("Hit an error while pushing:\n{msg}")))?;
    Ok(Json(SerializedFinishPushDiff::try_from(resp)?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportPushCompletedRequest {
    admin_key: String,
    spans: Vec<SerializedCompletedSpan>,
}

pub async fn report_push_completed(
    st: LocalAppState,
    req: ReportPushCompletedRequest,
) -> anyhow::Result<Vec<SpanRecord>> {
    let _identity = must_be_admin_from_key_with_write_access(
        st.application.app_auth(),
        st.instance_name.clone(),
        req.admin_key.clone(),
    )
    .await?;
    let spans = req
        .spans
        .into_iter()
        .map(|s| s.try_into())
        .collect::<anyhow::Result<Vec<SpanRecord>>>()?;
    Ok(spans)
}

#[debug_handler]
pub async fn report_push_completed_handler(
    State(st): State<LocalAppState>,
    Json(req): Json<ReportPushCompletedRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let spans = report_push_completed(st, req).await?;
    tracing::debug!("Received spans: {:?}", spans);
    Ok(Json(()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SerializedFinishPushDiff {
    auth_diff: AuthDiff,
    definition_diffs: BTreeMap<String, SerializedComponentDefinitionDiff>,
    component_diffs: BTreeMap<String, SerializedComponentDiff>,
}

impl TryFrom<FinishPushDiff> for SerializedFinishPushDiff {
    type Error = anyhow::Error;

    fn try_from(value: FinishPushDiff) -> Result<Self, Self::Error> {
        Ok(Self {
            auth_diff: value.auth_diff,
            definition_diffs: value
                .definition_diffs
                .into_iter()
                .map(|(k, v)| Ok((String::from(k), v.try_into()?)))
                .collect::<anyhow::Result<_>>()?,
            component_diffs: value
                .component_diffs
                .into_iter()
                .map(|(k, v)| Ok((String::from(k), v.try_into()?)))
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SerializedCompletedSpan {
    trace_id: String,
    parent_id: String,
    span_id: String,
    begin_time_unix_ns: String,
    duration_ns: String,
    name: String,
    properties: BTreeMap<String, String>,
    events: Vec<SerializedEventRecord>,
}

impl TryFrom<SerializedCompletedSpan> for SpanRecord {
    type Error = anyhow::Error;

    fn try_from(value: SerializedCompletedSpan) -> Result<Self, Self::Error> {
        let trace_id_buf = base64::decode_urlsafe(&value.trace_id)?;
        let trace_id = u128::from_le_bytes(trace_id_buf[..].try_into()?);

        let parent_id_buf = base64::decode_urlsafe(&value.parent_id)?;
        let parent_id = u64::from_le_bytes(parent_id_buf[..].try_into()?);

        let span_id_buf = base64::decode_urlsafe(&value.span_id)?;
        let span_id = u64::from_le_bytes(span_id_buf[..].try_into()?);

        let begin_time_unix_ns_buf = base64::decode_urlsafe(&value.begin_time_unix_ns)?;
        let begin_time_unix_ns = u64::from_le_bytes(begin_time_unix_ns_buf[..].try_into()?);

        let duration_ns_buf = base64::decode_urlsafe(&value.duration_ns)?;
        let duration_ns = u64::from_le_bytes(duration_ns_buf[..].try_into()?);

        let properties = value
            .properties
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect::<Vec<_>>();

        let events = value
            .events
            .into_iter()
            .map(|e| e.try_into())
            .collect::<anyhow::Result<_>>()?;

        Ok(Self {
            trace_id: TraceId(trace_id),
            parent_id: SpanId(parent_id),
            span_id: SpanId(span_id),
            begin_time_unix_ns,
            duration_ns,
            name: value.name.into(),
            properties,
            events,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SerializedEventRecord {
    name: String,
    timestamp_unix_ns: String,
    properties: BTreeMap<String, String>,
}

impl TryFrom<SerializedEventRecord> for EventRecord {
    type Error = anyhow::Error;

    fn try_from(value: SerializedEventRecord) -> Result<Self, Self::Error> {
        let timestamp_unix_ns_buf = base64::decode_urlsafe(&value.timestamp_unix_ns)?;
        let timestamp_unix_ns = u64::from_le_bytes(timestamp_unix_ns_buf[..].try_into()?);
        let properties = value
            .properties
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect::<Vec<_>>();
        Ok(Self {
            name: value.name.into(),
            timestamp_unix_ns,
            properties,
        })
    }
}
