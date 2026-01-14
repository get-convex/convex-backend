use std::collections::BTreeMap;

use anyhow::Context;
use application::{
    log_streaming::LogSinkWithId,
    Application,
};
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
        HttpResponseError,
    },
    knobs::AXIOM_MAX_ATTRIBUTES,
    log_streaming::LogEventFormatVersion,
};
use errors::ErrorMetadata;
use http::StatusCode;
use model::log_sinks::types::{
    axiom::{
        AxiomAttribute,
        AxiomConfig,
        VALID_AXIOM_INGEST_URLS,
    },
    datadog::{
        DatadogConfig,
        DatadogSiteLocation,
    },
    sentry::{
        ExceptionFormatVersion,
        SentryConfig,
        SerializedSentryConfig,
    },
    webhook::{
        generate_webhook_hmac_secret,
        WebhookConfig,
        WebhookFormat,
    },
    SinkConfig,
    SinkType,
};
use runtime::prod::ProdRuntime;
use sentry::types::Dsn;
use serde::{
    Deserialize,
    Serialize,
};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use value::{
    FieldName,
    ResolvedDocumentId,
};

use crate::{
    admin::{
        must_be_admin,
        must_be_admin_with_write_access,
    },
    authentication::ExtractIdentity,
    LocalAppState,
};

fn validate_axiom_ingest_url(ingest_url: Option<&String>) -> anyhow::Result<()> {
    if let Some(url) = ingest_url
        && !VALID_AXIOM_INGEST_URLS.contains(&url.as_str())
    {
        anyhow::bail!(ErrorMetadata::bad_request(
            "InvalidAxiomIngestUrl",
            format!(
                "Invalid Axiom ingest URL: {url}. Must be one of: {}",
                VALID_AXIOM_INGEST_URLS.join(", ")
            ),
        ));
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatadogSinkPostArgs {
    site_location: DatadogSiteLocation,
    dd_api_key: String,
    dd_tags: Vec<String>,
    version: Option<String>,
    service: Option<String>,
}

impl TryFrom<DatadogSinkPostArgs> for DatadogConfig {
    type Error = anyhow::Error;

    fn try_from(value: DatadogSinkPostArgs) -> Result<Self, Self::Error> {
        Ok(Self {
            site_location: value.site_location,
            dd_api_key: value.dd_api_key.into(),
            dd_tags: value.dd_tags,
            version: match value.version {
                Some(v) => v.parse().context(ErrorMetadata::bad_request(
                    "InvalidLogStreamVersion",
                    format!("Invalid log stream version {v}"),
                ))?,
                None => LogEventFormatVersion::V1,
            },
            service: value.service,
        })
    }
}

pub async fn add_datadog_sink(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(args): Json<DatadogSinkPostArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    tracing::info!("add_datadog_sink called (deprecated, use create_log_stream instead)");
    must_be_admin_with_write_access(&identity)?;
    st.application
        .ensure_log_streaming_allowed(identity)
        .await?;

    let config: DatadogConfig = args.try_into()?;
    st.application
        .add_log_sink(SinkConfig::Datadog(config))
        .await?;
    Ok(StatusCode::OK)
}

pub async fn regenerate_webhook_secret(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    tracing::info!(
        "regenerate_webhook_secret called (deprecated, use rotate_webhook_secret instead)"
    );
    must_be_admin_with_write_access(&identity)?;
    st.application
        .ensure_log_streaming_allowed(identity)
        .await?;

    let Some(SinkConfig::Webhook(existing_webhook_sink)) =
        st.application.get_log_sink(&SinkType::Webhook).await?
    else {
        return Err(anyhow::anyhow!(ErrorMetadata::bad_request(
            "NoWebhookLogStream",
            "No webhook log stream exists for this deployment"
        ))
        .into());
    };

    let hmac_secret = generate_webhook_hmac_secret(st.application.runtime());

    let config = WebhookConfig {
        url: existing_webhook_sink.url,
        format: existing_webhook_sink.format,
        hmac_secret,
    };
    st.application
        .add_log_sink(SinkConfig::Webhook(config))
        .await?;

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookSinkPostArgs {
    url: String,
    format: WebhookFormat,
}

pub async fn add_webhook_sink(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(args): Json<WebhookSinkPostArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    tracing::info!("add_webhook_sink called (deprecated, use create_log_stream instead)");
    must_be_admin_with_write_access(&identity)?;
    st.application
        .ensure_log_streaming_allowed(identity)
        .await?;

    add_webhook_sink_inner(st.application, args).await?;

    Ok(StatusCode::OK)
}

async fn add_webhook_sink_inner(
    application: Application<ProdRuntime>,
    args: WebhookSinkPostArgs,
) -> Result<(ResolvedDocumentId, String), HttpResponseError> {
    let existing_webhook_sink = application.get_log_sink(&SinkType::Webhook).await?;

    let hmac_secret = match existing_webhook_sink {
        Some(SinkConfig::Webhook(WebhookConfig {
            hmac_secret: existing_secret,
            ..
        })) => existing_secret,
        _ => generate_webhook_hmac_secret(application.runtime()),
    };

    let url = args.url.parse().map_err(|_| {
        anyhow::anyhow!(ErrorMetadata::bad_request(
            "InvalidWebhookUrl",
            "The URL passed was invalid"
        ))
    })?;

    let config = WebhookConfig {
        url,
        format: args.format,
        hmac_secret: hmac_secret.clone(),
    };
    let id = application
        .add_log_sink(SinkConfig::Webhook(config))
        .await?;

    Ok((id, hmac_secret))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AxiomSinkPostArgs {
    api_key: String,
    dataset_name: String,
    attributes: Vec<AxiomAttribute>,
    version: Option<String>,
    ingest_url: Option<String>,
}

impl TryFrom<AxiomSinkPostArgs> for AxiomConfig {
    type Error = anyhow::Error;

    fn try_from(value: AxiomSinkPostArgs) -> Result<Self, Self::Error> {
        validate_axiom_ingest_url(value.ingest_url.as_ref())?;

        Ok(Self {
            api_key: value.api_key.into(),
            dataset_name: value.dataset_name,
            attributes: value.attributes,
            version: match value.version {
                Some(v) => v.parse().context(ErrorMetadata::bad_request(
                    "InvalidLogStreamVersion",
                    format!("Invalid log stream version {v}"),
                ))?,
                None => LogEventFormatVersion::V1,
            },
            ingest_url: value.ingest_url,
        })
    }
}

pub async fn add_axiom_sink(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(args): Json<AxiomSinkPostArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    tracing::info!("add_axiom_sink called (deprecated, use create_log_stream instead)");
    must_be_admin_with_write_access(&identity)?;
    st.application
        .ensure_log_streaming_allowed(identity)
        .await?;

    add_axiom_sink_inner(st.application, args).await?;

    Ok(StatusCode::OK)
}

async fn add_axiom_sink_inner(
    application: Application<ProdRuntime>,
    args: AxiomSinkPostArgs,
) -> Result<ResolvedDocumentId, HttpResponseError> {
    if args.attributes.len() > *AXIOM_MAX_ATTRIBUTES {
        return Err(anyhow::anyhow!(
            "Exceeded max number of Axiom attributes. Contact support@convex.dev to request a \
             limit increase."
        )
        .into());
    }

    let config: AxiomConfig = args.try_into()?;
    let id = application.add_log_sink(SinkConfig::Axiom(config)).await?;

    Ok(id)
}

pub async fn add_sentry_sink(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(args): Json<SerializedSentryConfig>,
) -> Result<impl IntoResponse, HttpResponseError> {
    tracing::info!("add_sentry_sink called (deprecated, use create_log_stream instead)");
    must_be_admin_with_write_access(&identity)?;
    st.application
        .ensure_log_streaming_allowed(identity)
        .await?;
    st.application
        .add_log_sink(SinkConfig::Sentry(args.try_into()?))
        .await?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogSinkDeleteArgs {
    sink_type: SinkType,
}

pub async fn delete_log_sink(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(LogSinkDeleteArgs { sink_type }): Json<LogSinkDeleteArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    tracing::info!("delete_log_sink called (deprecated, use delete_log_stream instead)");
    must_be_admin_with_write_access(&identity)?;
    st.application
        .ensure_log_streaming_allowed(identity)
        .await?;

    st.application.remove_log_sink(sink_type).await?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
enum LogStreamType {
    Datadog,
    Webhook,
    Axiom,
    Sentry,
}

impl From<LogStreamType> for SinkType {
    fn from(log_stream_type: LogStreamType) -> Self {
        match log_stream_type {
            LogStreamType::Datadog => SinkType::Datadog,
            LogStreamType::Webhook => SinkType::Webhook,
            LogStreamType::Axiom => SinkType::Axiom,
            LogStreamType::Sentry => SinkType::Sentry,
        }
    }
}

/// Delete log stream
///
/// Delete the deployment's log stream with the given id.
#[utoipa::path(
    post,
    path = "/delete_log_stream/{id}",
    responses((status = 200)),
    params(
        ("id" = String, Path, description = "id of the log stream to delete"),
    ),
    security(
        ("Deploy Key" = []),
        ("OAuth Team Token" = []),
        ("Team Token" = []),
        ("OAuth Project Token" = []),
    ),
)]
pub async fn delete_log_stream(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    st.application
        .ensure_log_streaming_allowed(identity)
        .await?;

    st.application.remove_log_sink_by_id(id).await?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDatadogLogStreamArgs {
    /// Location of your Datadog deployment.
    site_location: DatadogSiteLocation,
    /// Datadog API key for authentication.
    dd_api_key: String,
    /// Optional comma-separated list of tags. These are sent to Datadog in each
    /// log event via the `ddtags` field.
    dd_tags: Vec<String>,
    /// Service name used as a special tag in Datadog.
    service: Option<String>,
}

impl TryFrom<CreateDatadogLogStreamArgs> for DatadogConfig {
    type Error = anyhow::Error;

    fn try_from(value: CreateDatadogLogStreamArgs) -> Result<Self, Self::Error> {
        Ok(Self {
            site_location: value.site_location,
            dd_api_key: value.dd_api_key.into(),
            dd_tags: value.dd_tags,
            version: LogEventFormatVersion::V2,
            service: value.service,
        })
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateWebhookLogStreamArgs {
    /// URL to send logs to.
    url: String,
    /// Format for the webhook payload. JSONL sends one object per line of
    /// request, JSON sends one array per request.
    format: WebhookFormat,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAxiomLogStreamArgs {
    /// Axiom API key for authentication.
    api_key: String,
    /// Name of the dataset in Axiom. This is where the logs will be sent.
    dataset_name: String,
    /// Optional list of attributes. These are extra fields and values sent to
    /// Axiom in each log event.
    attributes: Vec<AxiomAttribute>,
    /// Optional ingest endpoint for Axiom
    ingest_url: Option<String>,
}

impl TryFrom<CreateAxiomLogStreamArgs> for AxiomConfig {
    type Error = anyhow::Error;

    fn try_from(value: CreateAxiomLogStreamArgs) -> Result<Self, Self::Error> {
        validate_axiom_ingest_url(value.ingest_url.as_ref())?;

        Ok(Self {
            api_key: value.api_key.into(),
            dataset_name: value.dataset_name,
            attributes: value.attributes,
            version: LogEventFormatVersion::V2,
            ingest_url: value.ingest_url,
        })
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSentryLogStreamArgs {
    /// Sentry Data Source Name (DSN) to route exceptions to.
    dsn: String,
    /// Tags to add to all events routed to Sentry.
    #[schema(value_type = Option<BTreeMap<String, String>>)]
    tags: Option<BTreeMap<FieldName, String>>,
}

impl TryFrom<CreateSentryLogStreamArgs> for SentryConfig {
    type Error = anyhow::Error;

    fn try_from(value: CreateSentryLogStreamArgs) -> Result<Self, Self::Error> {
        Ok(Self {
            dsn: value
                .dsn
                .parse::<Dsn>()
                .context(ErrorMetadata::bad_request(
                    "InvalidSentryDsn",
                    "The Sentry DSN passed was invalid",
                ))?
                .into(),
            tags: value.tags,
            version: ExceptionFormatVersion::V2,
        })
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", tag = "logStreamType")]
pub enum CreateLogStreamArgs {
    #[schema(title = "Datadog")]
    Datadog(CreateDatadogLogStreamArgs),
    #[schema(title = "Webhook")]
    Webhook(CreateWebhookLogStreamArgs),
    #[schema(title = "Axiom")]
    Axiom(CreateAxiomLogStreamArgs),
    #[schema(title = "Sentry")]
    Sentry(CreateSentryLogStreamArgs),
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateWebhookLogStreamResponse {
    id: String,
    /// Use this secret to verify webhook signatures.
    hmac_secret: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase", tag = "logStreamType")]
pub enum CreateLogStreamResponse {
    #[schema(title = "Webhook")]
    Webhook(CreateWebhookLogStreamResponse),
    #[schema(title = "Datadog")]
    Datadog { id: String },
    #[schema(title = "Axiom")]
    Axiom { id: String },
    #[schema(title = "Sentry")]
    Sentry { id: String },
}

async fn ensure_log_sink_does_not_exist(
    application: &Application<ProdRuntime>,
    sink_type: &SinkType,
) -> Result<(), HttpResponseError> {
    if application.get_log_sink(sink_type).await?.is_some() {
        return Err(anyhow::anyhow!(ErrorMetadata::conflict(
            "LogStreamAlreadyExists",
            format!("{sink_type:?} log stream already exists for this deployment",)
        ))
        .into());
    }
    Ok(())
}

/// Create log stream
///
/// Create a new log stream for the deployment. Errors if a log stream of the
/// given type already exists.
#[utoipa::path(
    post,
    path = "/create_log_stream",
    responses((status = 200, body = CreateLogStreamResponse)),
    security(
        ("Deploy Key" = []),
        ("OAuth Team Token" = []),
        ("Team Token" = []),
        ("OAuth Project Token" = []),
    ),
)]
pub async fn create_log_stream(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(args): Json<CreateLogStreamArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    st.application
        .ensure_log_streaming_allowed(identity)
        .await?;

    match args {
        CreateLogStreamArgs::Datadog(datadog_sink_post_args) => {
            ensure_log_sink_does_not_exist(&st.application, &SinkType::Datadog).await?;

            let config: DatadogConfig = datadog_sink_post_args.try_into()?;
            let id = st
                .application
                .add_log_sink(SinkConfig::Datadog(config))
                .await?;
            Ok(Json(CreateLogStreamResponse::Datadog {
                id: id.to_string(),
            }))
        },
        CreateLogStreamArgs::Webhook(webhook_sink_post_args) => {
            ensure_log_sink_does_not_exist(&st.application, &SinkType::Webhook).await?;

            let hmac_secret = generate_webhook_hmac_secret(st.application.runtime());

            let url = webhook_sink_post_args.url.parse().map_err(|_| {
                anyhow::anyhow!(ErrorMetadata::bad_request(
                    "InvalidWebhookUrl",
                    "The URL passed was invalid"
                ))
            })?;

            let config = WebhookConfig {
                url,
                format: webhook_sink_post_args.format,
                hmac_secret: hmac_secret.clone(),
            };
            let id = st
                .application
                .add_log_sink(SinkConfig::Webhook(config))
                .await?;

            Ok(Json(CreateLogStreamResponse::Webhook(
                CreateWebhookLogStreamResponse {
                    hmac_secret,
                    id: id.to_string(),
                },
            )))
        },
        CreateLogStreamArgs::Axiom(axiom_sink_post_args) => {
            ensure_log_sink_does_not_exist(&st.application, &SinkType::Axiom).await?;

            if axiom_sink_post_args.attributes.len() > *AXIOM_MAX_ATTRIBUTES {
                return Err(anyhow::anyhow!(
                    "Exceeded max number of Axiom attributes. Contact support@convex.dev to \
                     request a limit increase."
                )
                .into());
            }

            let config: AxiomConfig = axiom_sink_post_args.try_into()?;
            let id = st
                .application
                .add_log_sink(SinkConfig::Axiom(config))
                .await?;

            Ok(Json(CreateLogStreamResponse::Axiom { id: id.to_string() }))
        },
        CreateLogStreamArgs::Sentry(sentry_config_args) => {
            ensure_log_sink_does_not_exist(&st.application, &SinkType::Sentry).await?;

            let config = sentry_config_args.try_into()?;
            let id = st
                .application
                .add_log_sink(SinkConfig::Sentry(config))
                .await?;
            Ok(Json(CreateLogStreamResponse::Sentry { id: id.to_string() }))
        },
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", tag = "logStreamType")]
pub enum RotateLogStreamSecretArgs {
    Webhook,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase", tag = "logStreamType")]
pub enum RotateLogStreamSecretResponse {
    #[serde(rename_all = "camelCase")]
    #[schema(title = "Webhook")]
    Webhook { hmac_secret: String },
}

/// Rotate webhook log stream secret
///
/// Rotate the secret for the webhook log stream.
#[utoipa::path(
    post,
    path = "/rotate_webhook_secret/{id}",
    responses((status = 200, body = RotateLogStreamSecretResponse)),
    params(
        ("id" = String, Path, description = "id of the webhook log stream for which to rotate the secret"),
    ),
    security(
        ("Deploy Key" = []),
        ("OAuth Team Token" = []),
        ("Team Token" = []),
        ("OAuth Project Token" = []),
    ),
)]
pub async fn rotate_webhook_secret(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    st.application
        .ensure_log_streaming_allowed(identity)
        .await?;

    let Some(LogSinkWithId {
        config: sink_config,
        ..
    }) = st.application.get_log_sink_by_id(&id).await?
    else {
        return Err(anyhow::anyhow!(ErrorMetadata::bad_request(
            "LogStreamDoesntExist",
            "No log stream with the given id exists for this deployment",
        ))
        .into());
    };

    match sink_config {
        SinkConfig::Webhook(existing_webhook_sink) => {
            let hmac_secret = generate_webhook_hmac_secret(st.application.runtime());

            let config = WebhookConfig {
                url: existing_webhook_sink.url,
                format: existing_webhook_sink.format,
                hmac_secret: hmac_secret.clone(),
            };
            st.application
                .patch_log_sink_config(&id, SinkConfig::Webhook(config))
                .await?;

            Ok(Json(RotateLogStreamSecretResponse::Webhook { hmac_secret }))
        },
        _ => Err(anyhow::anyhow!(ErrorMetadata::bad_request(
            "NoSecretToRotate",
            "This log stream does not have a secret to rotate."
        ))
        .into()),
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", tag = "logStreamType")]
enum LogStreamConfig {
    #[schema(title = "Datadog")]
    Datadog(DatadogLogStreamConfig),
    #[schema(title = "Webhook")]
    Webhook(WebhookLogStreamConfig),
    #[schema(title = "Axiom")]
    Axiom(AxiomLogStreamConfig),
    #[schema(title = "Sentry")]
    Sentry(SentryLogStreamConfig),
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(title = "DatadogConfig")]
pub struct DatadogLogStreamConfig {
    pub id: String,
    /// Location of your Datadog deployment.
    pub site_location: DatadogSiteLocation,
    /// Optional comma-separated list of tags. These are sent to Datadog in each
    /// log event via the `ddtags` field.
    pub dd_tags: Vec<String>,
    /// Service name used as a special tag in Datadog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(title = "WebhookConfig")]
pub struct WebhookLogStreamConfig {
    pub id: String,
    /// URL to send logs to.
    pub url: String,
    /// Format for the webhook payload. JSONL sends one object per line of
    /// request, JSON sends one array per request.
    pub format: WebhookFormat,
    /// Use this secret to verify webhook signatures.
    pub hmac_secret: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(title = "AxiomConfig")]
pub struct AxiomLogStreamConfig {
    pub id: String,
    /// Name of the dataset in Axiom. This is where the logs will be sent.
    pub dataset_name: String,
    /// Optional list of attributes. These are extra fields and values sent to
    /// Axiom in each log event.
    pub attributes: Vec<AxiomAttribute>,
    /// Optional ingest endpoint for Axiom
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ingest_url: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(title = "SentryConfig")]
pub struct SentryLogStreamConfig {
    pub id: String,
    /// Tags to add to all events routed to Sentry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<BTreeMap<String, String>>,
}

fn log_sink_to_log_stream_config(sink: LogSinkWithId) -> Option<LogStreamConfig> {
    match sink.config {
        SinkConfig::Datadog(config) => Some(LogStreamConfig::Datadog(DatadogLogStreamConfig {
            id: sink.id.to_string(),
            site_location: config.site_location,
            dd_tags: config.dd_tags,
            service: config.service,
        })),
        SinkConfig::Webhook(config) => Some(LogStreamConfig::Webhook(WebhookLogStreamConfig {
            id: sink.id.to_string(),
            url: config.url.to_string(),
            format: config.format,
            hmac_secret: config.hmac_secret,
        })),
        SinkConfig::Axiom(config) => Some(LogStreamConfig::Axiom(AxiomLogStreamConfig {
            id: sink.id.to_string(),
            dataset_name: config.dataset_name,
            attributes: config.attributes,
            ingest_url: config.ingest_url,
        })),
        SinkConfig::Sentry(config) => Some(LogStreamConfig::Sentry(SentryLogStreamConfig {
            id: sink.id.to_string(),
            tags: config
                .tags
                .map(|tags| tags.into_iter().map(|(k, v)| (k.into(), v)).collect()),
        })),
        _ => None,
    }
}

/// List log streams
///
/// List configs for all existing log streams in a deployment.
#[utoipa::path(
    get,
    path = "/list_log_streams",
    responses((status = 200, body = Vec<LogStreamConfig>)),
    security(
        ("Deploy Key" = []),
        ("OAuth Team Token" = []),
        ("Team Token" = []),
        ("OAuth Project Token" = []),
    ),
)]
pub async fn list_log_streams(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin(&identity)?;

    Ok(Json(
        st.application
            .list_log_sinks()
            .await?
            .into_iter()
            .filter_map(log_sink_to_log_stream_config)
            .collect::<Vec<LogStreamConfig>>(),
    ))
}

/// Get log stream
///
/// Get the config for a specific log stream by id.
#[utoipa::path(
    get,
    path = "/get_log_stream/{id}",
    responses((status = 200, body = LogStreamConfig)),
    params(
        ("id" = String, Path, description = "id of the log stream to retrieve"),
    ),
    security(
        ("Deploy Key" = []),
        ("OAuth Team Token" = []),
        ("Team Token" = []),
        ("OAuth Project Token" = []),
    ),
)]
pub async fn get_log_stream(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin(&identity)?;

    let Some(log_sink_with_id) = st.application.get_log_sink_by_id(&id).await? else {
        return Err(anyhow::anyhow!(ErrorMetadata::bad_request(
            "LogStreamDoesntExist",
            "No log stream with the given id exists for this deployment",
        ))
        .into());
    };

    let config = log_sink_to_log_stream_config(log_sink_with_id).ok_or_else(|| {
        anyhow::anyhow!(ErrorMetadata::bad_request(
            "UnsupportedLogStreamType",
            "This log stream type is not supported",
        ))
    })?;

    Ok(Json(config))
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDatadogSinkArgs {
    /// Location of your Datadog deployment.
    #[serde(default)]
    site_location: Option<DatadogSiteLocation>,
    /// Datadog API key for authentication.
    #[serde(default)]
    dd_api_key: Option<String>,
    /// Optional comma-separated list of tags. These are sent to Datadog in each
    /// log event via the `ddtags` field.
    #[serde(default)]
    dd_tags: Option<Vec<String>>,
    /// Service name used as a special tag in Datadog.
    #[serde(default, with = "::serde_with::rust::double_option")]
    service: Option<Option<String>>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWebhookSinkArgs {
    /// URL to send logs to.
    #[serde(default)]
    url: Option<String>,
    /// Format for the webhook payload. JSONL sends one object per line of
    /// request, JSON sends one array per request.
    #[serde(default)]
    format: Option<WebhookFormat>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAxiomSinkArgs {
    /// Axiom API key for authentication.
    #[serde(default)]
    api_key: Option<String>,
    /// Name of the dataset in Axiom. This is where the logs will be sent.
    #[serde(default)]
    dataset_name: Option<String>,
    /// Optional list of attributes. These are extra fields and values sent to
    /// Axiom in each log event.
    #[serde(default)]
    attributes: Option<Vec<AxiomAttribute>>,
    /// Optional ingest endpoint for Axiom
    #[serde(default, with = "::serde_with::rust::double_option")]
    ingest_url: Option<Option<String>>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSentrySinkArgs {
    /// Sentry Data Source Name (DSN) to route exceptions to.
    #[serde(default)]
    dsn: Option<String>,
    /// Tags to add to all events routed to Sentry.
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[schema(value_type = Option<Option<BTreeMap<String, String>>>)]
    tags: Option<Option<BTreeMap<FieldName, String>>>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", tag = "logStreamType")]
pub enum UpdateLogStreamArgs {
    #[schema(title = "Datadog")]
    Datadog(UpdateDatadogSinkArgs),
    #[schema(title = "Webhook")]
    Webhook(UpdateWebhookSinkArgs),
    #[schema(title = "Axiom")]
    Axiom(UpdateAxiomSinkArgs),
    #[schema(title = "Sentry")]
    Sentry(UpdateSentrySinkArgs),
}

/// Update log stream
///
/// Update an existing log stream for the deployment. Omit a field to keep the
/// existing value, and use `null` to unset a field.
#[utoipa::path(
    post,
    path = "/update_log_stream/{id}",
    responses((status = 200)),
    params(
        ("id" = String, Path, description = "id of the log stream to update"),
    ),
    security(
        ("Deploy Key" = []),
        ("OAuth Team Token" = []),
        ("Team Token" = []),
        ("OAuth Project Token" = []),
    ),
)]
pub async fn update_log_stream(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Path(id): Path<String>,
    Json(args): Json<UpdateLogStreamArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    st.application
        .ensure_log_streaming_allowed(identity)
        .await?;

    let Some(LogSinkWithId {
        config: sink_config,
        ..
    }) = st.application.get_log_sink_by_id(&id).await?
    else {
        return Err(anyhow::anyhow!(ErrorMetadata::bad_request(
            "LogStreamDoesntExist",
            "No log stream with the given id exists for this deployment",
        ))
        .into());
    };

    match sink_config {
        SinkConfig::Datadog(existing_config) => {
            let UpdateLogStreamArgs::Datadog(update_args) = args else {
                return Err(anyhow::anyhow!(ErrorMetadata::bad_request(
                    "LogStreamTypeMismatch",
                    "Cannot update a Datadog log stream with arguments for a different log stream \
                     type",
                ))
                .into());
            };

            let config = DatadogConfig {
                site_location: update_args
                    .site_location
                    .unwrap_or(existing_config.site_location),
                dd_api_key: update_args
                    .dd_api_key
                    .map(|k| k.into())
                    .unwrap_or(existing_config.dd_api_key),
                dd_tags: update_args.dd_tags.unwrap_or(existing_config.dd_tags),
                version: existing_config.version,
                service: update_args.service.unwrap_or(existing_config.service),
            };

            st.application
                .patch_log_sink_config(&id, SinkConfig::Datadog(config))
                .await?;
        },
        SinkConfig::Webhook(existing_config) => {
            let UpdateLogStreamArgs::Webhook(update_args) = args else {
                return Err(anyhow::anyhow!(ErrorMetadata::bad_request(
                    "LogStreamTypeMismatch",
                    "Cannot update a Webhook log stream with arguments for a different log stream \
                     type",
                ))
                .into());
            };

            let url = if let Some(url_str) = update_args.url {
                url_str.parse().map_err(|_| {
                    anyhow::anyhow!(ErrorMetadata::bad_request(
                        "InvalidWebhookUrl",
                        "The URL passed was invalid"
                    ))
                })?
            } else {
                existing_config.url
            };

            let config = WebhookConfig {
                url,
                format: update_args.format.unwrap_or(existing_config.format),
                hmac_secret: existing_config.hmac_secret,
            };

            st.application
                .patch_log_sink_config(&id, SinkConfig::Webhook(config))
                .await?;
        },
        SinkConfig::Axiom(existing_config) => {
            let UpdateLogStreamArgs::Axiom(update_args) = args else {
                return Err(anyhow::anyhow!(ErrorMetadata::bad_request(
                    "LogStreamTypeMismatch",
                    "Cannot update an Axiom log stream with arguments for a different log stream \
                     type",
                ))
                .into());
            };

            let attributes = update_args.attributes.unwrap_or(existing_config.attributes);
            if attributes.len() > *AXIOM_MAX_ATTRIBUTES {
                return Err(anyhow::anyhow!(
                    "Exceeded max number of Axiom attributes. Contact support@convex.dev to \
                     request a limit increase."
                )
                .into());
            }

            let ingest_url = update_args.ingest_url.unwrap_or(existing_config.ingest_url);
            if ingest_url.is_some() {
                validate_axiom_ingest_url(ingest_url.as_ref())?
            }

            let config = AxiomConfig {
                api_key: update_args
                    .api_key
                    .map(|k| k.into())
                    .unwrap_or(existing_config.api_key),
                dataset_name: update_args
                    .dataset_name
                    .unwrap_or(existing_config.dataset_name),
                attributes,
                version: existing_config.version,
                ingest_url,
            };

            st.application
                .patch_log_sink_config(&id, SinkConfig::Axiom(config))
                .await?;
        },
        SinkConfig::Sentry(existing_config) => {
            let UpdateLogStreamArgs::Sentry(update_args) = args else {
                return Err(anyhow::anyhow!(ErrorMetadata::bad_request(
                    "LogStreamTypeMismatch",
                    "Cannot update a Sentry log stream with arguments for a different log stream \
                     type",
                ))
                .into());
            };

            let dsn = if let Some(dsn_str) = update_args.dsn {
                dsn_str
                    .parse::<Dsn>()
                    .context(ErrorMetadata::bad_request(
                        "InvalidSentryDsn",
                        "The Sentry DSN passed was invalid",
                    ))?
                    .into()
            } else {
                existing_config.dsn
            };

            let config = SentryConfig {
                dsn,
                tags: update_args.tags.unwrap_or(existing_config.tags),
                version: existing_config.version,
            };

            st.application
                .patch_log_sink_config(&id, SinkConfig::Sentry(config))
                .await?;
        },
        _ => {
            return Err(anyhow::anyhow!(ErrorMetadata::bad_request(
                "UnsupportedLogStreamType",
                "This log stream type does not support updates",
            ))
            .into())
        },
    }

    // Reset the log sink status to Pending so that it retries verification
    st.application.reset_log_sink_to_pending(&id).await?;

    Ok(StatusCode::OK)
}

pub fn platform_router<S>() -> OpenApiRouter<S>
where
    LocalAppState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    OpenApiRouter::new()
        .routes(utoipa_axum::routes!(list_log_streams))
        .routes(utoipa_axum::routes!(get_log_stream))
        .routes(utoipa_axum::routes!(delete_log_stream))
        .routes(utoipa_axum::routes!(create_log_stream))
        .routes(utoipa_axum::routes!(update_log_stream))
        .routes(utoipa_axum::routes!(rotate_webhook_secret))
}

#[cfg(test)]
mod tests {
    use model::log_sinks::types::{
        axiom::AxiomConfig,
        datadog::{
            DatadogConfig,
            DatadogSiteLocation,
        },
        LogSinksRow,
    };
    use serde_json::json;

    use crate::log_sinks::{
        AxiomSinkPostArgs,
        DatadogSinkPostArgs,
    };

    #[test]
    fn datadog_config_deserialize() -> anyhow::Result<()> {
        // Basic deserialize
        let json = json!({
            "siteLocation": "US1",
            "ddApiKey": "test_key",
            "ddTags": vec!["tag:abc","abc"],
        });
        let post_args: DatadogSinkPostArgs = serde_json::from_value(json)?;
        let config = DatadogConfig::try_from(post_args)?;
        assert_eq!(config.site_location, DatadogSiteLocation::US1);
        assert_eq!(config.dd_api_key, "test_key".to_string().into());
        assert_eq!(
            config.dd_tags,
            vec!["tag:abc".to_string(), "abc".to_string()]
        );

        // No tags
        let json = json!({
            "siteLocation": "US1",
            "ddApiKey": "test_key",
            "ddTags": Vec::<String>::new()
        });
        let post_args: DatadogSinkPostArgs = serde_json::from_value(json)?;
        let config = DatadogConfig::try_from(post_args)?;
        assert_eq!(config.site_location, DatadogSiteLocation::US1);
        assert_eq!(config.dd_api_key, "test_key".to_string().into());
        assert!(config.dd_tags.is_empty());

        // US1_FED -- ensure we handle the SCREAMING_SNAKE_CASE
        let json = json!({
            "siteLocation": "US1_FED",
            "ddApiKey": "test_key",
            "ddTags": Vec::<String>::new()
        });
        let post_args: DatadogSinkPostArgs = serde_json::from_value(json)?;
        let config = DatadogConfig::try_from(post_args)?;
        assert_eq!(config.site_location, DatadogSiteLocation::US1_FED);
        assert_eq!(config.dd_api_key, "test_key".to_string().into());
        assert!(config.dd_tags.is_empty());
        Ok(())
    }

    #[test]
    fn axiom_config_valid_ingest_urls() -> anyhow::Result<()> {
        // Test with no ingest_url (default)
        let json = json!({
            "apiKey": "test_key",
            "datasetName": "test_dataset",
            "attributes": [],
        });
        let post_args: AxiomSinkPostArgs = serde_json::from_value(json)?;
        let config = AxiomConfig::try_from(post_args)?;
        assert!(config.ingest_url.is_none());

        // Test with default URL
        let json = json!({
            "apiKey": "test_key",
            "datasetName": "test_dataset",
            "attributes": [],
            "ingestUrl": "https://api.axiom.co",
        });
        let post_args: AxiomSinkPostArgs = serde_json::from_value(json)?;
        let config = AxiomConfig::try_from(post_args)?;
        assert_eq!(config.ingest_url, Some("https://api.axiom.co".to_string()));

        // Test with US East 1
        let json = json!({
            "apiKey": "test_key",
            "datasetName": "test_dataset",
            "attributes": [],
            "ingestUrl": "https://us-east-1.aws.edge.axiom.co",
        });
        let post_args: AxiomSinkPostArgs = serde_json::from_value(json)?;
        let config = AxiomConfig::try_from(post_args)?;
        assert_eq!(
            config.ingest_url,
            Some("https://us-east-1.aws.edge.axiom.co".to_string())
        );

        // Test with EU Central 1
        let json = json!({
            "apiKey": "test_key",
            "datasetName": "test_dataset",
            "attributes": [],
            "ingestUrl": "https://eu-central-1.aws.edge.axiom.co",
        });
        let post_args: AxiomSinkPostArgs = serde_json::from_value(json)?;
        let config = AxiomConfig::try_from(post_args)?;
        assert_eq!(
            config.ingest_url,
            Some("https://eu-central-1.aws.edge.axiom.co".to_string())
        );

        Ok(())
    }

    #[test]
    fn axiom_config_invalid_ingest_url() {
        // Test with invalid URL
        let json = json!({
            "apiKey": "test_key",
            "datasetName": "test_dataset",
            "attributes": [],
            "ingestUrl": "https://invalid.axiom.co",
        });
        let post_args: AxiomSinkPostArgs = serde_json::from_value(json).unwrap();
        let result = AxiomConfig::try_from(post_args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid Axiom ingest URL"));
        assert!(err.to_string().contains("https://invalid.axiom.co"));

        // Test with completely wrong URL
        let json = json!({
            "apiKey": "test_key",
            "datasetName": "test_dataset",
            "attributes": [],
            "ingestUrl": "https://example.com",
        });
        let post_args: AxiomSinkPostArgs = serde_json::from_value(json).unwrap();
        let result = AxiomConfig::try_from(post_args);
        assert!(result.is_err());
    }

    // Endpoint tests
    use axum::body::Body;
    use axum_extra::headers::authorization::Credentials;
    use common::{
        document::{
            ParseDocument,
            ParsedDocument,
        },
        log_streaming::LogEventFormatVersion,
    };
    use http::{
        Request,
        StatusCode,
    };
    use keybroker::Identity;
    use model::log_sinks::types::{
        sentry::ExceptionFormatVersion,
        SinkConfig,
    };
    use runtime::prod::ProdRuntime;
    use serde_json::Value as JsonValue;

    use crate::test_helpers::{
        setup_backend_for_test,
        TestLocalBackend,
    };

    async fn create_log_stream(
        backend: &TestLocalBackend,
        body: JsonValue,
    ) -> anyhow::Result<JsonValue> {
        let req = Request::builder()
            .uri("/api/v1/create_log_stream")
            .method("POST")
            .header("Content-Type", "application/json")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(Body::from(serde_json::to_vec(&body)?))?;
        backend.expect_success(req).await
    }

    async fn list_log_streams(backend: &TestLocalBackend) -> anyhow::Result<Vec<JsonValue>> {
        let req = Request::builder()
            .uri("/api/v1/list_log_streams")
            .method("GET")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(Body::empty())?;
        backend.expect_success(req).await
    }

    async fn get_log_stream(backend: &TestLocalBackend, id: &str) -> anyhow::Result<JsonValue> {
        let req = Request::builder()
            .uri(format!("/api/v1/get_log_stream/{id}"))
            .method("GET")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(Body::empty())?;
        backend.expect_success(req).await
    }

    async fn update_log_stream(
        backend: &TestLocalBackend,
        id: &str,
        body: JsonValue,
    ) -> anyhow::Result<()> {
        let req = Request::builder()
            .uri(format!("/api/v1/update_log_stream/{id}"))
            .method("POST")
            .header("Content-Type", "application/json")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(Body::from(serde_json::to_vec(&body)?))?;
        backend.expect_success(req).await
    }

    async fn delete_log_stream(backend: &TestLocalBackend, id: &str) -> anyhow::Result<()> {
        let req = Request::builder()
            .uri(format!("/api/v1/delete_log_stream/{id}"))
            .method("POST")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(Body::empty())?;
        backend.expect_success(req).await
    }

    async fn rotate_webhook_secret(
        backend: &TestLocalBackend,
        id: &str,
    ) -> anyhow::Result<JsonValue> {
        let req = Request::builder()
            .uri(format!("/api/v1/rotate_webhook_secret/{id}"))
            .method("POST")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(Body::empty())?;
        backend.expect_success(req).await
    }

    async fn get_log_sink_from_db(
        backend: &TestLocalBackend,
        id: &str,
    ) -> anyhow::Result<Option<SinkConfig>> {
        let mut tx = backend.st.application.begin(Identity::system()).await?;
        let resolved_id = tx.resolve_developer_id(&id.parse()?, value::TableNamespace::Global)?;
        let doc = tx.get(resolved_id).await?;
        if let Some(doc) = doc {
            let row: ParsedDocument<LogSinksRow> = doc.parse()?;
            Ok(Some(row.config.clone()))
        } else {
            Ok(None)
        }
    }

    #[convex_macro::prod_rt_test]
    async fn test_list_log_streams_empty(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let streams = list_log_streams(&backend).await?;
        assert_eq!(streams.len(), 0);
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_list_log_streams(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        // Create a webhook log stream
        create_log_stream(
            &backend,
            json!({
                "logStreamType": "webhook",
                "url": "https://example.com/webhook",
                "format": "jsonl",
            }),
        )
        .await?;

        // Create a datadog log stream
        create_log_stream(
            &backend,
            json!({
                "logStreamType": "datadog",
                "siteLocation": "US1",
                "ddApiKey": "test_key",
                "ddTags": ["tag1", "tag2"],
                "service": "my-service",
            }),
        )
        .await?;

        let streams = list_log_streams(&backend).await?;
        assert_eq!(streams.len(), 2);

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_get_log_stream(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let create_response = create_log_stream(
            &backend,
            json!({
                "logStreamType": "webhook",
                "url": "https://example.com/webhook",
                "format": "jsonl",
            }),
        )
        .await?;

        let id = create_response["id"].as_str().unwrap();
        let stream = get_log_stream(&backend, id).await?;

        assert_eq!(stream["logStreamType"], "webhook");
        assert_eq!(stream["url"], "https://example.com/webhook");
        assert_eq!(stream["format"], "jsonl");

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_get_log_stream_not_found(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let req = Request::builder()
            .uri("/api/v1/get_log_stream/invalid_id")
            .method("GET")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(Body::empty())?;

        backend
            .expect_error(req, StatusCode::BAD_REQUEST, "InvalidLogStreamId")
            .await?;

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_create_log_stream_webhook(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let response = create_log_stream(
            &backend,
            json!({
                "logStreamType": "webhook",
                "url": "https://example.com/webhook",
                "format": "jsonl",
            }),
        )
        .await?;

        assert!(response["id"].is_string());
        assert!(response["hmacSecret"].is_string());
        assert_eq!(response["logStreamType"], "webhook");

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_create_log_stream_datadog_defaults_to_v2(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let response = create_log_stream(
            &backend,
            json!({
                "logStreamType": "datadog",
                "siteLocation": "US1",
                "ddApiKey": "test_key",
                "ddTags": ["tag1"],
                "service": "my-service",
            }),
        )
        .await?;

        let id = response["id"].as_str().unwrap();

        // Check the version in the database
        let config = get_log_sink_from_db(&backend, id).await?;
        assert!(config.is_some());
        match config.unwrap() {
            SinkConfig::Datadog(config) => {
                assert_eq!(config.version, LogEventFormatVersion::V2);
            },
            _ => panic!("Expected Datadog config"),
        }

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_create_log_stream_axiom_defaults_to_v2(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let response = create_log_stream(
            &backend,
            json!({
                "logStreamType": "axiom",
                "apiKey": "test_key",
                "datasetName": "my-dataset",
                "attributes": [],
            }),
        )
        .await?;

        let id = response["id"].as_str().unwrap();

        // Check the version in the database
        let config = get_log_sink_from_db(&backend, id).await?;
        assert!(config.is_some());
        match config.unwrap() {
            SinkConfig::Axiom(config) => {
                assert_eq!(config.version, LogEventFormatVersion::V2);
            },
            _ => panic!("Expected Axiom config"),
        }

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_create_log_stream_sentry_defaults_to_v2(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let response = create_log_stream(
            &backend,
            json!({
                "logStreamType": "sentry",
                "dsn": "https://ef1d32d342354c87869ab2db8b490b2c@o1192621.ingest.sentry.io/6333191",
                "tags": null,
            }),
        )
        .await?;

        let id = response["id"].as_str().unwrap();

        // Check the version in the database
        let config = get_log_sink_from_db(&backend, id).await?;
        assert!(config.is_some());
        match config.unwrap() {
            SinkConfig::Sentry(config) => {
                assert_eq!(config.version, ExceptionFormatVersion::V2);
            },
            _ => panic!("Expected Sentry config"),
        }

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_create_log_stream_already_exists(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        create_log_stream(
            &backend,
            json!({
                "logStreamType": "webhook",
                "url": "https://example.com/webhook",
                "format": "jsonl",
            }),
        )
        .await?;

        let req = Request::builder()
            .uri("/api/v1/create_log_stream")
            .method("POST")
            .header("Content-Type", "application/json")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(Body::from(serde_json::to_vec(&json!({
                "logStreamType": "webhook",
                "url": "https://example.com/webhook2",
                "format": "json",
            }))?))?;

        backend
            .expect_error(req, StatusCode::CONFLICT, "LogStreamAlreadyExists")
            .await?;

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_create_log_stream_invalid_webhook_url(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let req = Request::builder()
            .uri("/api/v1/create_log_stream")
            .method("POST")
            .header("Content-Type", "application/json")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(Body::from(serde_json::to_vec(&json!({
                "logStreamType": "webhook",
                "url": "not-a-valid-url",
                "format": "jsonl",
            }))?))?;

        backend
            .expect_error(req, StatusCode::BAD_REQUEST, "InvalidWebhookUrl")
            .await?;

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_create_log_stream_invalid_sentry_dsn(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let req = Request::builder()
            .uri("/api/v1/create_log_stream")
            .method("POST")
            .header("Content-Type", "application/json")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(Body::from(serde_json::to_vec(&json!({
                "logStreamType": "sentry",
                "dsn": "not-a-valid-dsn",
                "tags": null,
            }))?))?;

        backend
            .expect_error(req, StatusCode::BAD_REQUEST, "InvalidSentryDsn")
            .await?;

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_update_log_stream_omit_field(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let create_response = create_log_stream(
            &backend,
            json!({
                "logStreamType": "datadog",
                "siteLocation": "US1",
                "ddApiKey": "original_key",
                "ddTags": ["tag1"],
                "service": "original-service",
            }),
        )
        .await?;

        let id = create_response["id"].as_str().unwrap();

        // Update only the service field
        update_log_stream(
            &backend,
            id,
            json!({
                "logStreamType": "datadog",
                "service": "updated-service",
            }),
        )
        .await?;

        let stream = get_log_stream(&backend, id).await?;
        assert_eq!(stream["siteLocation"], "US1");
        assert_eq!(stream["ddTags"], json!(["tag1"]));
        assert_eq!(stream["service"], "updated-service");

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_update_log_stream_unset_optional_field(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let create_response = create_log_stream(
            &backend,
            json!({
                "logStreamType": "datadog",
                "siteLocation": "US1",
                "ddApiKey": "test_key",
                "ddTags": [],
                "service": "my-service",
            }),
        )
        .await?;

        let id = create_response["id"].as_str().unwrap();

        // Unset the service field
        update_log_stream(
            &backend,
            id,
            json!({
                "logStreamType": "datadog",
                "service": null,
            }),
        )
        .await?;

        let stream = get_log_stream(&backend, id).await?;
        assert!(stream.get("service").is_none());

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_update_log_stream_not_found(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let req = Request::builder()
            .uri("/api/v1/update_log_stream/invalid_id")
            .method("POST")
            .header("Content-Type", "application/json")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(Body::from(serde_json::to_vec(&json!({
                "logStreamType": "webhook",
                "url": "https://example.com/new",
            }))?))?;

        backend
            .expect_error(req, StatusCode::BAD_REQUEST, "InvalidLogStreamId")
            .await?;

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_update_log_stream_type_mismatch(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let create_response = create_log_stream(
            &backend,
            json!({
                "logStreamType": "webhook",
                "url": "https://example.com/webhook",
                "format": "jsonl",
            }),
        )
        .await?;

        let id = create_response["id"].as_str().unwrap();

        let req = Request::builder()
            .uri(format!("/api/v1/update_log_stream/{id}"))
            .method("POST")
            .header("Content-Type", "application/json")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(Body::from(serde_json::to_vec(&json!({
                "logStreamType": "datadog",
                "ddApiKey": "new_key",
            }))?))?;

        backend
            .expect_error(req, StatusCode::BAD_REQUEST, "LogStreamTypeMismatch")
            .await?;

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_update_log_stream_preserves_version(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let create_response = create_log_stream(
            &backend,
            json!({
                "logStreamType": "datadog",
                "siteLocation": "US1",
                "ddApiKey": "test_key",
                "ddTags": [],
                "service": null,
            }),
        )
        .await?;

        let id = create_response["id"].as_str().unwrap();

        // Update the API key
        update_log_stream(
            &backend,
            id,
            json!({
                "logStreamType": "datadog",
                "ddApiKey": "new_key",
            }),
        )
        .await?;

        // Check the version is still V2
        let config = get_log_sink_from_db(&backend, id).await?;
        match config.unwrap() {
            SinkConfig::Datadog(config) => {
                assert_eq!(config.version, LogEventFormatVersion::V2);
            },
            _ => panic!("Expected Datadog config"),
        }

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_update_log_stream_webhook_preserves_hmac_secret(
        rt: ProdRuntime,
    ) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let create_response = create_log_stream(
            &backend,
            json!({
                "logStreamType": "webhook",
                "url": "https://example.com/webhook",
                "format": "jsonl",
            }),
        )
        .await?;

        let id = create_response["id"].as_str().unwrap();
        let original_secret = create_response["hmacSecret"].as_str().unwrap();

        // Update the URL
        update_log_stream(
            &backend,
            id,
            json!({
                "logStreamType": "webhook",
                "url": "https://example.com/new-webhook",
            }),
        )
        .await?;

        let stream = get_log_stream(&backend, id).await?;
        assert_eq!(stream["url"], "https://example.com/new-webhook");
        assert_eq!(stream["hmacSecret"], original_secret);

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_delete_log_stream(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let create_response = create_log_stream(
            &backend,
            json!({
                "logStreamType": "webhook",
                "url": "https://example.com/webhook",
                "format": "jsonl",
            }),
        )
        .await?;

        let id = create_response["id"].as_str().unwrap();

        delete_log_stream(&backend, id).await?;

        // Verify it's deleted
        let req = Request::builder()
            .uri(format!("/api/v1/get_log_stream/{id}"))
            .method("GET")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(Body::empty())?;

        backend
            .expect_error(req, StatusCode::BAD_REQUEST, "LogStreamDoesntExist")
            .await?;

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_delete_log_stream_not_found(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let req = Request::builder()
            .uri("/api/v1/delete_log_stream/invalid_id")
            .method("POST")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(Body::empty())?;

        backend
            .expect_error(req, StatusCode::BAD_REQUEST, "InvalidLogStreamId")
            .await?;

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_delete_and_recreate_same_type(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let create_response = create_log_stream(
            &backend,
            json!({
                "logStreamType": "webhook",
                "url": "https://example.com/webhook",
                "format": "jsonl",
            }),
        )
        .await?;

        let id = create_response["id"].as_str().unwrap();
        delete_log_stream(&backend, id).await?;

        // Create a new webhook log stream
        let new_response = create_log_stream(
            &backend,
            json!({
                "logStreamType": "webhook",
                "url": "https://example.com/new-webhook",
                "format": "json",
            }),
        )
        .await?;

        assert!(new_response["id"].is_string());
        assert_ne!(new_response["id"], id);

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_rotate_webhook_secret(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let create_response = create_log_stream(
            &backend,
            json!({
                "logStreamType": "webhook",
                "url": "https://example.com/webhook",
                "format": "jsonl",
            }),
        )
        .await?;

        let id = create_response["id"].as_str().unwrap();
        let original_secret = create_response["hmacSecret"].as_str().unwrap();

        let rotate_response = rotate_webhook_secret(&backend, id).await?;
        let new_secret = rotate_response["hmacSecret"].as_str().unwrap();

        assert_ne!(new_secret, original_secret);

        // Verify the stream still has the same URL and format
        let stream = get_log_stream(&backend, id).await?;
        assert_eq!(stream["url"], "https://example.com/webhook");
        assert_eq!(stream["format"], "jsonl");
        assert_eq!(stream["hmacSecret"], new_secret);

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_rotate_secret_non_webhook_errors(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let create_response = create_log_stream(
            &backend,
            json!({
                "logStreamType": "datadog",
                "siteLocation": "US1",
                "ddApiKey": "test_key",
                "ddTags": [],
                "service": null,
            }),
        )
        .await?;

        let id = create_response["id"].as_str().unwrap();

        let req = Request::builder()
            .uri(format!("/api/v1/rotate_webhook_secret/{id}"))
            .method("POST")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(Body::empty())?;

        backend
            .expect_error(req, StatusCode::BAD_REQUEST, "NoSecretToRotate")
            .await?;

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_rotate_secret_not_found(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let req = Request::builder()
            .uri("/api/v1/rotate_webhook_secret/invalid_id")
            .method("POST")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(Body::empty())?;

        backend
            .expect_error(req, StatusCode::BAD_REQUEST, "InvalidLogStreamId")
            .await?;

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_multiple_log_streams_different_types(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        create_log_stream(
            &backend,
            json!({
                "logStreamType": "webhook",
                "url": "https://example.com/webhook",
                "format": "jsonl",
            }),
        )
        .await?;

        create_log_stream(
            &backend,
            json!({
                "logStreamType": "datadog",
                "siteLocation": "US1",
                "ddApiKey": "test_key",
                "ddTags": [],
                "service": null,
            }),
        )
        .await?;

        create_log_stream(
            &backend,
            json!({
                "logStreamType": "axiom",
                "apiKey": "test_key",
                "datasetName": "my-dataset",
                "attributes": [],
            }),
        )
        .await?;

        let streams = list_log_streams(&backend).await?;
        assert_eq!(streams.len(), 3);

        Ok(())
    }
}
