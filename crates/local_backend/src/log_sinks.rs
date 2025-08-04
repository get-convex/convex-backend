use anyhow::Context;
use axum::{
    debug_handler,
    extract::State,
    response::IntoResponse,
};
use common::{
    http::{
        extract::Json,
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
    },
    datadog::{
        DatadogConfig,
        DatadogSiteLocation,
    },
    sentry::SerializedSentryConfig,
    webhook::{
        WebhookConfig,
        WebhookFormat,
    },
    SinkConfig,
    SinkType,
};
use serde::Deserialize;

use crate::{
    admin::must_be_admin_with_write_access,
    authentication::ExtractIdentity,
    LocalAppState,
};

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

#[debug_handler]
pub async fn add_datadog_sink(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(args): Json<DatadogSinkPostArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookSinkPostArgs {
    url: String,
    format: WebhookFormat,
}

impl TryFrom<WebhookSinkPostArgs> for WebhookConfig {
    type Error = anyhow::Error;

    fn try_from(value: WebhookSinkPostArgs) -> Result<Self, Self::Error> {
        let url = value.url.parse().map_err(|_| {
            anyhow::anyhow!(ErrorMetadata::bad_request(
                "InvalidWebhookUrl",
                "The URL passed was invalid"
            ))
        })?;
        Ok(WebhookConfig {
            url,
            format: value.format,
        })
    }
}

#[debug_handler]
pub async fn add_webhook_sink(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(args): Json<WebhookSinkPostArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    st.application
        .ensure_log_streaming_allowed(identity)
        .await?;

    let config: WebhookConfig = args.try_into()?;
    st.application
        .add_log_sink(SinkConfig::Webhook(config))
        .await?;

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AxiomSinkPostArgs {
    api_key: String,
    dataset_name: String,
    attributes: Vec<AxiomAttribute>,
    version: Option<String>,
}

impl TryFrom<AxiomSinkPostArgs> for AxiomConfig {
    type Error = anyhow::Error;

    fn try_from(value: AxiomSinkPostArgs) -> Result<Self, Self::Error> {
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
        })
    }
}

#[debug_handler]
pub async fn add_axiom_sink(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(args): Json<AxiomSinkPostArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    st.application
        .ensure_log_streaming_allowed(identity)
        .await?;
    if args.attributes.len() > *AXIOM_MAX_ATTRIBUTES {
        return Err(anyhow::anyhow!(
            "Exceeded max number of Axiom attributes. Contact support@convex.dev to request a \
             limit increase."
        )
        .into());
    }

    let config: AxiomConfig = args.try_into()?;
    st.application
        .add_log_sink(SinkConfig::Axiom(config))
        .await?;
    Ok(StatusCode::OK)
}

#[debug_handler]
pub async fn add_sentry_sink(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(args): Json<SerializedSentryConfig>,
) -> Result<impl IntoResponse, HttpResponseError> {
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

#[debug_handler]
pub async fn delete_log_sink(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(LogSinkDeleteArgs { sink_type }): Json<LogSinkDeleteArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    st.application
        .ensure_log_streaming_allowed(identity)
        .await?;

    st.application.remove_log_sink(sink_type).await?;
    Ok(StatusCode::OK)
}

#[cfg(test)]
mod tests {
    use model::log_sinks::types::datadog::{
        DatadogConfig,
        DatadogSiteLocation,
    };
    use serde_json::json;

    use crate::log_sinks::DatadogSinkPostArgs;

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
}
