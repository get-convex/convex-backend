use std::{
    collections::BTreeMap,
    fmt::Display,
    sync::LazyLock,
};

use anyhow::Context;
use async_trait::async_trait;
use convex_fivetran_common::config::Config;
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
    selection::{
        Selection,
        DEFAULT_FIVETRAN_SCHEMA_NAME,
    },
    DocumentDeltasArgs,
    DocumentDeltasResponse,
    DocumentDeltasValue,
    ListSnapshotArgs,
    ListSnapshotResponse,
    ListSnapshotValue,
    SelectionArg,
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
        selection: Selection,
    ) -> anyhow::Result<ListSnapshotResponse>;

    /// See https://docs.convex.dev/http-api/#get-apidocument_deltas
    async fn document_deltas(
        &self,
        cursor: DocumentDeltasCursor,
        selection: Selection,
    ) -> anyhow::Result<DocumentDeltasResponse>;

    /// Get a list of columns for each table and component on the Convex
    /// backend.
    async fn get_table_column_names(
        &self,
    ) -> anyhow::Result<BTreeMap<ComponentPath, BTreeMap<TableName, Vec<FieldName>>>>;
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
        selection: Selection,
    ) -> anyhow::Result<ListSnapshotResponse> {
        self.post(
            "list_snapshot",
            ListSnapshotArgs {
                snapshot,
                cursor: cursor.map(|c| c.into()),
                selection: SelectionArg::Exact { selection },
                format: Some("convex_encoded_json".to_string()),
            },
        )
        .await
    }

    async fn document_deltas(
        &self,
        cursor: DocumentDeltasCursor,
        selection: Selection,
    ) -> anyhow::Result<DocumentDeltasResponse> {
        self.post(
            "document_deltas",
            DocumentDeltasArgs {
                cursor: Some(cursor.into()),
                selection: SelectionArg::Exact { selection },
                format: Some("convex_encoded_json".to_string()),
            },
        )
        .await
    }

    async fn get_table_column_names(
        &self,
    ) -> anyhow::Result<BTreeMap<ComponentPath, BTreeMap<TableName, Vec<FieldName>>>> {
        let response: GetTableColumnNamesResponse = self.get("get_table_column_names").await?;

        let by_component = response
            .by_component
            .into_iter()
            .map(|(component_path, tables)| {
                (
                    ComponentPath(component_path),
                    tables
                        .into_iter()
                        .map(|table| {
                            let columns: Vec<_> =
                                table.columns.into_iter().map(FieldName).collect();
                            (TableName(table.name), columns)
                        })
                        .collect(),
                )
            })
            .collect();

        Ok(by_component)
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

#[derive(Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Display, Debug)]
pub struct TableName(pub String);

#[cfg(test)]
impl From<&str> for TableName {
    fn from(value: &str) -> Self {
        TableName(value.to_string())
    }
}

#[derive(Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Display, Clone, Debug)]
pub struct ComponentPath(pub String);

#[cfg(test)]
impl From<&str> for ComponentPath {
    fn from(value: &str) -> Self {
        ComponentPath(value.to_string())
    }
}

#[cfg(test)]
impl ComponentPath {
    pub fn root() -> Self {
        ComponentPath("".to_string())
    }

    pub fn test_component() -> Self {
        ComponentPath("waitlist".to_string())
    }
}

#[derive(Display, Debug)]
pub struct FieldName(pub String);

#[cfg(test)]
impl From<&str> for FieldName {
    fn from(value: &str) -> Self {
        FieldName(value.to_string())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTableColumnNamesResponse {
    pub by_component: BTreeMap<String, Vec<GetTableColumnNameTable>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTableColumnNameTable {
    pub name: String,
    pub columns: Vec<String>,
}

pub trait SnapshotValue {
    fn table(&self) -> &String;
    fn component(&self) -> &String;

    /// The full path of the table, including the component name,
    /// in the format used for `tables_seen` in [`State`].
    fn table_path_for_state(&self) -> String {
        match self.component().as_str() {
            "" => self.table().clone(),
            _ => format!("{}/{}", self.component(), self.table()),
        }
    }

    fn fivetran_schema_name(&self) -> String {
        match self.component().as_str() {
            "" => DEFAULT_FIVETRAN_SCHEMA_NAME.to_string(),
            _ => self.component().clone(),
        }
    }
}

impl SnapshotValue for ListSnapshotValue {
    fn table(&self) -> &String {
        &self.table
    }

    fn component(&self) -> &String {
        &self.component
    }
}

impl SnapshotValue for DocumentDeltasValue {
    fn table(&self) -> &String {
        &self.table
    }

    fn component(&self) -> &String {
        &self.component
    }
}

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

    #[test]
    fn test_table_path_for_state_root_component() {
        let snapshot_value = ListSnapshotValue {
            table: "messages".to_string(),
            component: "".to_string(),
            fields: BTreeMap::new(),
            ts: 0,
        };
        assert_eq!(snapshot_value.table_path_for_state(), "messages");
    }

    #[test]
    fn test_table_path_for_state_other_component() {
        let snapshot_value = ListSnapshotValue {
            table: "messages".to_string(),
            component: "waitlist".to_string(),
            fields: BTreeMap::new(),
            ts: 0,
        };
        assert_eq!(snapshot_value.table_path_for_state(), "waitlist/messages");
    }

    #[test]
    fn test_fivetran_schema_name_root_component() {
        let snapshot_value = ListSnapshotValue {
            table: "messages".to_string(),
            component: "".to_string(),
            fields: BTreeMap::new(),
            ts: 0,
        };
        assert_eq!(snapshot_value.fivetran_schema_name(), "convex");
    }

    #[test]
    fn test_fivetran_schema_name_other_component() {
        let snapshot_value = ListSnapshotValue {
            table: "messages".to_string(),
            component: "waitlist".to_string(),
            fields: BTreeMap::new(),
            ts: 0,
        };
        assert_eq!(snapshot_value.fivetran_schema_name(), "waitlist");
    }
}
