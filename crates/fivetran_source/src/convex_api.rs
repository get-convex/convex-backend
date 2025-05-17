use std::{
    collections::BTreeMap,
    fmt::Display,
    sync::LazyLock,
};

use anyhow::Context;
use async_trait::async_trait;
use convex_fivetran_common::config::Config;
use convex_fivetran_source::api_types::{
    DocumentDeltasArgs,
    ListSnapshotArgs,
};
use derive_more::{
    Display,
    From,
    Into,
};
use headers::{
    HeaderName,
    HeaderValue,
};
use serde::{
    de::DeserializeOwned,
    Deserialize,
    Serialize,
};

use crate::api_types::{
    DocumentDeltasResponse,
    ListSnapshotResponse,
};

#[allow(clippy::declare_interior_mutable_const)]
const CONVEX_CLIENT_HEADER: HeaderName = HeaderName::from_static("convex-client");

static CONVEX_CLIENT_HEADER_VALUE: LazyLock<HeaderValue> = LazyLock::new(|| {
    let connector_version = env!("CARGO_PKG_VERSION");
    HeaderValue::from_str(&format!("fivetran-export-{connector_version}")).unwrap()
});

/// The APIs exposed by a Convex backend for streaming export.
#[async_trait]
pub trait Source: Display + Send {
    /// An endpoint that confirms the Convex backend is accessible with
    /// streaming export enabled
    async fn test_streaming_export_connection(&self) -> anyhow::Result<()>;

    /// See https://docs.convex.dev/http-api/#get-apilist_snapshot
    async fn list_snapshot(
        &self,
        snapshot: Option<i64>,
        cursor: Option<ListSnapshotCursor>,
        table_name: Option<String>,
    ) -> anyhow::Result<ListSnapshotResponse>;

    /// See https://docs.convex.dev/http-api/#get-apidocument_deltas
    async fn document_deltas(
        &self,
        cursor: DocumentDeltasCursor,
        table_name: Option<String>,
    ) -> anyhow::Result<DocumentDeltasResponse>;

    /// Get a list of columns for each table on the Convex backend.
    async fn get_tables_and_columns(&self) -> anyhow::Result<BTreeMap<TableName, Vec<FieldName>>>;
}

/// Implementation of [`Source`] accessing a real Convex deployment over HTTP.
pub struct ConvexApi {
    pub config: Config,
}

impl ConvexApi {
    /// Performs a GET HTTP request to a given endpoint of the Convex API.
    async fn get<T: DeserializeOwned>(&self, endpoint: &str) -> anyhow::Result<T> {
        let url = self
            .config
            .deploy_url
            .join("api/")
            .unwrap()
            .join(endpoint)
            .unwrap();

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
                if let Ok(text) = resp.text().await {
                    anyhow::bail!(
                        "Call to {endpoint} on {} returned an unsuccessful response: {text}",
                        self.config.deploy_url
                    )
                } else {
                    anyhow::bail!(
                        "Call to {endpoint} on {} returned no response",
                        self.config.deploy_url
                    )
                }
            },
            Err(e) => anyhow::bail!(e.to_string()),
        }
    }

    /// Performs a POST HTTP request to a given endpoint of the Convex API using
    /// the given parameters as a JSON body.
    async fn post<P: Serialize, T: DeserializeOwned>(
        &self,
        endpoint: &str,
        parameters: P,
    ) -> anyhow::Result<T> {
        let url = self
            .config
            .deploy_url
            .join("api/")
            .unwrap()
            .join(endpoint)
            .unwrap();

        match reqwest::Client::new()
            .post(url)
            .header(CONVEX_CLIENT_HEADER, &*CONVEX_CLIENT_HEADER_VALUE)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Convex {}", self.config.deploy_key),
            )
            .json(&parameters)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => Ok(resp
                .json::<T>()
                .await
                .context("Failed to deserialize query result")?),
            Ok(resp) => {
                if let Ok(text) = resp.text().await {
                    anyhow::bail!(
                        "Call to {endpoint} on {} returned an unsuccessful response: {text}",
                        self.config.deploy_url
                    )
                } else {
                    anyhow::bail!(
                        "Call to {endpoint} on {} returned no response",
                        self.config.deploy_url
                    )
                }
            },
            Err(e) => anyhow::bail!(e.to_string()),
        }
    }
}

#[async_trait]
impl Source for ConvexApi {
    async fn test_streaming_export_connection(&self) -> anyhow::Result<()> {
        self.get("test_streaming_export_connection").await
    }

    async fn list_snapshot(
        &self,
        snapshot: Option<i64>,
        cursor: Option<ListSnapshotCursor>,
        table_name: Option<String>,
    ) -> anyhow::Result<ListSnapshotResponse> {
        self.post(
            "list_snapshot",
            ListSnapshotArgs {
                snapshot,
                cursor: cursor.map(|c| c.into()),
                table_name,
                component: None,
                format: Some("convex_encoded_json".to_string()),
            },
        )
        .await
    }

    async fn document_deltas(
        &self,
        cursor: DocumentDeltasCursor,
        table_name: Option<String>,
    ) -> anyhow::Result<DocumentDeltasResponse> {
        self.post(
            "document_deltas",
            DocumentDeltasArgs {
                cursor: Some(cursor.into()),
                table_name,
                component: None,
                format: Some("convex_encoded_json".to_string()),
            },
        )
        .await
    }

    async fn get_tables_and_columns(&self) -> anyhow::Result<BTreeMap<TableName, Vec<FieldName>>> {
        let tables_to_columns: BTreeMap<TableName, Vec<String>> =
            self.get("get_tables_and_columns").await?;

        tables_to_columns
            .into_iter()
            .map(|(table_name, all_columns)| {
                let system_columns = ["_id", "_creationTime"].into_iter().map(String::from);
                let user_columns: Vec<_> = all_columns
                    .into_iter()
                    .filter(|key| !key.starts_with('_'))
                    .collect();

                let columns = system_columns.chain(user_columns).map(FieldName).collect();

                Ok((table_name, columns))
            })
            .try_collect()
    }
}

impl Display for ConvexApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.config.deploy_url.as_ref())
    }
}

#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, From, Into)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct ListSnapshotCursor(pub String);

#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, From, Into, Copy)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct DocumentDeltasCursor(pub i64);

#[derive(Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub struct TableName(pub String);

#[cfg(test)]
impl From<&str> for TableName {
    fn from(value: &str) -> Self {
        TableName(value.to_string())
    }
}

#[derive(Display)]
pub struct FieldName(pub String);

#[cfg(test)]
mod tests {
    use core::panic;

    use schemars::schema::Schema;
    use serde_json::json;

    use super::*;

    #[derive(Deserialize)]
    pub struct DatabaseSchema(pub BTreeMap<TableName, Schema>);

    #[test]
    fn can_deserialize_schema() {
        let json = json!({
            "emptyTable": false,
            "table": json!({
                "type": "object",
                "properties": json!({
                    "_creationTime": json!({ "type": "number" }),
                    "_id": json!({
                        "$description": "Id(messages)",
                        "type": "string"
                    }),
                    "author": json!({ "type": "string" }),
                    "body": json!({ "type": "string" }),
                    "_table": json!({ "type": "string" }),
                    "_ts": json!({ "type": "integer" }),
                    "_deleted": json!({ "type": "boolean" }),
                }),
                "additionalProperties": false,
                "required": vec!["_creationTime", "_id", "author", "body"],
                "$schema": "http://json-schema.org/draft-07/schema#",
            }),
        });

        let schema: DatabaseSchema = serde_json::from_value(json).unwrap();

        let Schema::Bool(_) = schema.0.get(&"emptyTable".into()).unwrap() else {
            panic!();
        };
        let Schema::Object(schema_object) = schema.0.get(&"table".into()).unwrap() else {
            panic!();
        };
        assert!(schema_object.object.is_some());
    }
}
