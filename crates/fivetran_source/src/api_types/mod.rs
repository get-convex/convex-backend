// API types for the HTTP APIs used by the Fivetran and Airbyte source
// connectors

pub mod selection;

use std::collections::BTreeMap;

use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDeltasArgs {
    /// Exclusive timestamp. Initially pass `ListSnapshotResponse.snapshot` for
    /// the first page. Then pass DocumentDeltasResponse.cursor for
    /// subsequent pages.
    pub cursor: Option<i64>,
    /// Leave as None to get all tables.
    pub table_name: Option<String>,
    /// Component path. Leave as None to get all components.
    pub component: Option<String>,
    /// Export format
    pub format: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDeltasResponse {
    /// Document deltas, in timestamp order.
    pub values: Vec<DocumentDeltasValue>,
    /// Exclusive timestamp for passing in as `cursor` to subsequent API calls.
    pub cursor: i64,
    /// Continue calling the API while has_more is true.
    pub has_more: bool,
}

/// Identical to `ListSnapshotValue`, but with a `deleted` field
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DocumentDeltasValue {
    /// The path of the component this document is from.
    #[serde(rename = "_component")]
    pub component: String,

    /// The name of the table this document is from.
    #[serde(rename = "_table")]
    pub table: String,

    /// _ts is the field used for ordering documents with the same
    /// _id, and determining which version is latest.
    #[serde(rename = "_ts")]
    pub ts: i64,

    /// Indicates whether the document was deleted. Will always be `false` in
    /// the list snapshot API
    #[serde(rename = "_deleted")]
    pub deleted: bool,

    /// The fields of the document. Connectors must ignore fields prefixed by
    /// `_` (except `_id` and `_creationTime`) since they could be used by
    /// future versions of the API for new fields.
    #[serde(flatten)]
    pub fields: BTreeMap<String, JsonValue>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSnapshotArgs {
    /// Timestamp snapshot. Initially pass None, then pass
    /// ListSnapshotResponse.snapshot for subsequent pages.
    pub snapshot: Option<i64>,
    /// Exclusive internal identifier. Initially pass None, then pass
    /// ListSnapshotResponse.cursor for subsequent pages.
    pub cursor: Option<String>,
    /// Leave as None to get all tables.
    pub table_name: Option<String>,
    /// Component path. Leave as None to get all components.
    pub component: Option<String>,
    /// Export format
    pub format: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSnapshotResponse {
    /// Documents, in (id, ts) order.
    pub values: Vec<ListSnapshotValue>,
    /// Timestamp snapshot. Pass this in as `snapshot` to subsequent API calls.
    pub snapshot: i64,
    /// Exclusive document id for passing in as `cursor` to subsequent API
    /// calls.
    pub cursor: Option<String>,
    /// Continue calling the API while has_more is true.
    /// When this becomes false, the `ListSnapshotResponse.snapshot` can be used
    /// as `DocumentDeltasArgs.cursor` to get deltas after the snapshot.
    pub has_more: bool,
}

/// A value returned by the list snapshot API.
/// This corresponds to a Convex document with some special fields added.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListSnapshotValue {
    /// The path of the component this document is from.
    #[serde(rename = "_component")]
    pub component: String,

    /// The name of the table this document is from.
    #[serde(rename = "_table")]
    pub table: String,

    /// _ts is the field used for ordering documents with the same
    /// _id, and determining which version is latest.
    #[serde(rename = "_ts")]
    pub ts: i64,

    /// The fields of the document. Connectors must ignore fields prefixed by
    /// `_` (except `_id` and `_creationTime`) since they could be used by
    /// future versions of the API for new fields.
    #[serde(flatten)]
    pub fields: BTreeMap<String, JsonValue>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTableColumnNamesResponse {
    pub by_component: BTreeMap<String, Vec<GetTableColumnNameTable>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTableColumnNameTable {
    pub name: String,
    pub columns: Vec<String>,
}
