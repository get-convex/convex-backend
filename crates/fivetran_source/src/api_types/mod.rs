// API types for the HTTP APIs used by the Fivetran and Airbyte source
// connectors

pub mod selection;

use std::collections::BTreeMap;

use selection::Selection;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDeltasArgs {
    /// Exclusive timestamp. Initially pass `ListSnapshotResponse.snapshot` for
    /// the first page. Then pass DocumentDeltasResponse.cursor for
    /// subsequent pages.
    pub cursor: Option<i64>,

    /// The components, tables, and columns to export.
    #[serde(flatten)]
    pub selection: SelectionArg,

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

    /// The components, tables, and columns to export.
    #[serde(flatten)]
    pub selection: SelectionArg,

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

/// Arguments to the data sync (streaming export) API (`/api/v1/data/sync`).
// Constructed in the backend (`local_backend`), so unused within this crate.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataSyncArgs {
    /// Opaque cursor returned by a previous call. Omit to start from scratch.
    pub cursor: Option<String>,

    /// The components, tables, and columns to export. When omitted, everything
    /// is exported. Supports the shorthand forms `{"tableName": "...",
    /// "component": "..."}` and `{"component": "..."}`, or the exact form
    /// `{"selection": {...}}` (a map of component -> table -> column
    /// inclusion).
    #[serde(flatten)]
    #[schema(value_type = Object)]
    pub selection: SelectionArg,
}

/// One page returned by the data sync API.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataSyncResponse {
    /// Tables truncated by this page: the consumer should drop everything it
    /// previously synced for each, then apply `values` (which re-sync them from
    /// scratch). Logically applies before `values`.
    pub truncates: Vec<DataSyncTruncate>,
    /// Documents and tombstones produced by this page.
    pub values: Vec<DataSyncValue>,
    /// Opaque cursor to pass back in as `cursor` on the next call.
    pub cursor: String,
    /// The consistency state of the sync after this page.
    pub status: DataSyncStatus,
}

/// A table whose contents were replaced wholesale (e.g. by `npx convex
/// import`). Reported separately from `values` since it carries none of the
/// per-document fields.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DataSyncTruncate {
    /// The path of the component the table is in.
    #[serde(rename = "_component")]
    pub component: String,

    /// The name of the truncated table.
    #[serde(rename = "_table")]
    pub table: String,
}

/// A single document-level entry emitted by the data sync API: a Convex
/// document (or a tombstone, for a deletion) with some special fields added.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DataSyncValue {
    /// The path of the component this entry is from.
    #[serde(rename = "_component")]
    pub component: String,

    /// The name of the table this entry is from.
    #[serde(rename = "_table")]
    pub table: String,

    /// The timestamp at which this revision was written.
    #[serde(rename = "_ts")]
    pub ts: i64,

    /// Whether the document was deleted (a tombstone).
    #[serde(rename = "_deleted")]
    pub deleted: bool,

    /// The fields of the document. Connectors must ignore fields prefixed by
    /// `_` (except `_id` and `_creationTime`) since they could be used by
    /// future versions of the API for new fields. For tombstones, only `_id`
    /// is present.
    #[serde(flatten)]
    #[schema(value_type = Object)]
    pub fields: BTreeMap<String, JsonValue>,
}

/// The consistency state reported alongside a data sync page.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DataSyncStatus {
    /// The entries emitted so far represent a consistent snapshot at
    /// `snapshot`. The cursor can be persisted and used to continue the sync
    /// later (within the document retention window).
    Synced {
        snapshot: i64,
        /// Whether `snapshot` is behind the latest timestamp — i.e. it's a
        /// consistent snapshot but not fully caught up to the most recent
        /// commit. Callers use this to decide whether to keep calling the API
        /// or pause until later.
        // Renamed explicitly rather than via the enum's `rename_all` (which
        // only renames variant tags, not struct-variant fields). We can't use
        // `rename_all_fields`: serde honors it but utoipa doesn't, which would
        // make the generated OpenAPI spec disagree with the wire format.
        #[serde(rename = "hasMore")]
        has_more: bool,
    },
    /// More pages are required before the view is consistent.
    InProgress {
        #[serde(flatten)]
        progress: DataSyncProgress,
    },
}

/// Progress indicator returned while a data sync is `InProgress`.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataSyncProgress {
    pub num_tables_synced: u64,
    pub total_tables: u64,
    pub current_component: Option<String>,
    pub current_table: Option<String>,
    pub num_documents_in_current_table: u64,
}

/// Since [ListSnapshotArgs] and [DocumentDeltasArgs] need to support the older
/// selection formats, this wraps the newer selection format ([Selection]) while
/// providing a way to deserialize the older formats.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum SelectionArg {
    /// Newer selection format, allows to select specific tables, components,
    /// and columns.
    Exact { selection: Selection },

    /// If only the table name is provided, assumes it’s in the root component.
    SingleTable {
        #[serde(alias = "tableName")]
        table_name: String,

        /// The component path of the table. If not provided, the table is
        /// assumed to be in the root component.
        component: Option<String>,
    },

    /// The user can also provide a component name to export all tables in that
    /// component.
    SingleComponent { component: String },

    /// If no selection parameter is provided, return all components, tables and
    /// columns.
    Everything {},
}

impl Default for SelectionArg {
    fn default() -> Self {
        SelectionArg::Everything {}
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct GetTableColumnNamesResponse {
    pub by_component: BTreeMap<String, Vec<GetTableColumnNameTable>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTableColumnNameTable {
    pub name: String,
    pub columns: Vec<String>,
}
