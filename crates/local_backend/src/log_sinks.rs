use anyhow::Context;
use axum::response::IntoResponse;
use common::{
    http::{
        extract::{
            Json,
            MtState,
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
    },
    datadog::{
        DatadogConfig,
        DatadogSiteLocation,
    },
    sentry::SerializedSentryConfig,
    webhook::{
        generate_webhook_hmac_secret,
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

pub async fn add_datadog_sink(
    MtState(st): MtState<LocalAppState>,
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

pub async fn regenerate_webhook_secret(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    st.application
        .ensure_log_streaming_allowed(identity)
        .await?;

    let Some(SinkConfig::Webhook(existing_webhook_sink)) =
        st.application.get_log_sink(SinkType::Webhook).await?
    else {
        return Err(anyhow::anyhow!("No webhook log stream exists for this deployment").into());
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
    must_be_admin_with_write_access(&identity)?;
    st.application
        .ensure_log_streaming_allowed(identity)
        .await?;

    let existing_webhook_sink = st.application.get_log_sink(SinkType::Webhook).await?;

    let hmac_secret = match existing_webhook_sink {
        Some(SinkConfig::Webhook(WebhookConfig {
            hmac_secret: existing_secret,
            ..
        })) => existing_secret,
        _ => generate_webhook_hmac_secret(st.application.runtime()),
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
        hmac_secret,
    };
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

pub async fn add_axiom_sink(
    MtState(st): MtState<LocalAppState>,
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

pub async fn add_sentry_sink(
    MtState(st): MtState<LocalAppState>,
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

pub async fn delete_log_sink(
    MtState(st): MtState<LocalAppState>,
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
