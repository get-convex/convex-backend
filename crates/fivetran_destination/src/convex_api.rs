use std::{
    collections::HashMap,
    fmt::Display,
    sync::LazyLock,
};

use anyhow::Context;
use async_trait::async_trait;
use chrono::{
    DateTime,
    Utc,
};
use common::{
    schemas::DatabaseSchema,
    value::TableName,
};
use convex_fivetran_common::config::Config;
use convex_fivetran_destination::api_types::{
    BatchWriteRow,
    DeleteType,
    TruncateTableArgs,
};
use serde::{
    de::DeserializeOwned,
    Serialize,
};
use serde_json::Value as JsonValue;
use tonic::codegen::http::{
    HeaderName,
    HeaderValue,
};

#[allow(clippy::declare_interior_mutable_const)]
const CONVEX_CLIENT_HEADER: HeaderName = HeaderName::from_static("convex-client");

static CONVEX_CLIENT_HEADER_VALUE: LazyLock<HeaderValue> = LazyLock::new(|| {
    let destination_version = env!("CARGO_PKG_VERSION");
    HeaderValue::from_str(&format!("fivetran-import-{destination_version}")).unwrap()
});

/// The APIs exposed by a Convex backend for streaming export.
#[async_trait]
pub trait Destination: Display + Send {
    /// An endpoint that confirms the Convex backend is accessible with
    /// streaming import enabled
    async fn test_streaming_import_connection(&self) -> anyhow::Result<()>;

    async fn get_schema(&self) -> anyhow::Result<Option<DatabaseSchema>>;

    async fn truncate_table(
        &self,
        table_name: TableName,
        delete_type: DeleteType,
        delete_before: Option<DateTime<Utc>>,
    ) -> anyhow::Result<()>;
    async fn batch_write(&self, rows: Vec<BatchWriteRow>) -> anyhow::Result<()>;
}

/// Implementation of [`Destination`] accessing a real Convex deployment over
/// HTTP.
pub struct ConvexApi {
    pub config: Config,
}

impl ConvexApi {
    /// Performs a GET HTTP request to a given endpoint of the Convex API using
    /// the given query parameters.
    async fn get<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        parameters: HashMap<&str, Option<String>>,
    ) -> anyhow::Result<T> {
        let non_null_parameters: HashMap<&str, String> = parameters
            .into_iter()
            .filter_map(|(key, value)| value.map(|value| (key, value)))
            .collect();

        let mut url = self.config.deploy_url.join(endpoint).unwrap();

        url.query_pairs_mut().extend_pairs(non_null_parameters);

        match reqwest::Client::new()
            .get(url)
            .header(CONVEX_CLIENT_HEADER, &*CONVEX_CLIENT_HEADER_VALUE)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Convex {}", self.config.deploy_key),
            )
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => Ok(resp
                .json::<T>()
                .await
                .context("Failed to deserialize query result")?),
            Ok(resp) => {
                let status = resp.status().as_str().to_string();
                if let Ok(text) = resp.text().await {
                    anyhow::bail!(
                        "Call to {endpoint} on {} returned an unsuccessful response ({status}): \
                         {text}",
                        self.config.deploy_url
                    )
                } else {
                    anyhow::bail!(
                        "Call to {endpoint} on {} returned an unsuccessful response with no \
                         content ({status})",
                        self.config.deploy_url
                    )
                }
            },
            Err(e) => anyhow::bail!(e.to_string()),
        }
    }

    /// Performs a POST  HTTP request to a given endpoint of the Convex API
    /// using the given query parameters.
    async fn post<T: Serialize>(&self, endpoint: &str, args: T) -> anyhow::Result<()> {
        let url = self
            .config
            .deploy_url
            .join("api/")
            .unwrap()
            .join(endpoint)
            .unwrap();

        match reqwest::Client::new()
            .post(url)
            .json(&args)
            .header(CONVEX_CLIENT_HEADER, &*CONVEX_CLIENT_HEADER_VALUE)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Convex {}", self.config.deploy_key),
            )
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => Ok(()),
            Ok(resp) => {
                let status = resp.status().as_str().to_string();
                if let Ok(text) = resp.text().await {
                    anyhow::bail!(
                        "Call to {endpoint} on {} returned an unsuccessful response ({status}): \
                         {text}",
                        self.config.deploy_url
                    )
                } else {
                    anyhow::bail!(
                        "Call to {endpoint} on {} returned an unsuccessful response with no \
                         content ({status})",
                        self.config.deploy_url
                    )
                }
            },
            Err(e) => anyhow::bail!(e.to_string()),
        }
    }
}

#[async_trait]
impl Destination for ConvexApi {
    async fn test_streaming_import_connection(&self) -> anyhow::Result<()> {
        self.get_schema().await?;
        Ok(())
    }

    async fn get_schema(&self) -> anyhow::Result<Option<DatabaseSchema>> {
        let value: JsonValue = self
            .get("/api/streaming_import/get_schema", HashMap::default())
            .await?;

        if value == JsonValue::Null {
            return Ok(None);
        };

        let schema =
            DatabaseSchema::try_from(value).context("Can’t deserialize the retrived schema")?;
        Ok(Some(schema))
    }

    async fn truncate_table(
        &self,
        table_name: TableName,
        delete_type: DeleteType,
        delete_before: Option<DateTime<Utc>>,
    ) -> anyhow::Result<()> {
        self.post(
            "/api/streaming_import/fivetran_truncate_table",
            TruncateTableArgs {
                table_name: table_name.to_string(),
                delete_type,
                delete_before,
            },
        )
        .await?;
        Ok(())
    }

    async fn batch_write(&self, rows: Vec<BatchWriteRow>) -> anyhow::Result<()> {
        self.post("/api/streaming_import/apply_fivetran_operations", rows)
            .await?;
        Ok(())
    }
}

impl Display for ConvexApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.config.deploy_url.as_ref())
    }
}
