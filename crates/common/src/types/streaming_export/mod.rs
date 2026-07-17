// API types for the streaming export HTTP APIs. These are consumed by the
// Fivetran and Airbyte source connectors, as well as any other client of the
// streaming export API.

pub mod selection;

use std::collections::BTreeMap;

use selection::Selection;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;

use crate::http::PaginationMetadata;

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
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataSyncArgs {
    /// Opaque cursor returned by a previous call. Omit to start from scratch.
    pub cursor: Option<String>,

    /// The components, tables, and columns to export. When omitted, everything
    /// is exported. The selection may change between calls of the same sync:
    /// newly selected tables are synced from scratch, and deselected tables
    /// stop being exported (documents already exported from them are not
    /// tombstoned).
    #[serde(default)]
    pub selection: Selection,
}

/// One page returned by the data sync API.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataSyncResponse {
    /// Tables truncated by this page: the consumer should drop everything it
    /// previously synced for each, then apply `values` (which re-sync them from
    /// scratch). Logically applies before `values`.
    pub truncates: Vec<DataSyncTruncate>,
    /// Documents and tombstones produced by this page.
    pub values: Vec<DataSyncValue>,
    /// Unique id of the sync, assigned on the first page and stable across
    /// the sync's lifetime. Identifies this sync in `/data/list_active_syncs`.
    pub sync_id: String,
    /// The consistency state of the sync after this page.
    pub status: DataSyncStatus,
    /// Pagination information. The data sync endpoint is an infinite streaming
    /// endpoint, so `nextCursor` is always present. `hasMore` is `true` while
    /// data can be fetched immediately. When `hasMore` is `false`, the cursor
    /// has caught up; in that case, it is recommended to back off significantly
    /// to wait for more writes before making another call.
    pub pagination: PaginationMetadata,
}

/// A table whose contents were replaced wholesale (e.g. by `npx convex
/// import`). Reported separately from `values` since it carries none of the
/// per-document fields.
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DataSyncTruncate {
    /// The path of the component the table is in.
    pub component: String,

    /// The name of the truncated table.
    pub table: String,
}

/// A single document-level entry emitted by the data sync API: a Convex
/// document (or a tombstone, for a deletion) nested under `value`, with
/// metadata fields alongside it.
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DataSyncValue {
    /// The path of the component this entry is from.
    pub component: String,

    /// The name of the table this entry is from.
    pub table: String,

    /// The timestamp at which this revision was written.
    pub ts: i64,

    /// Whether the document was deleted (a tombstone).
    pub deleted: bool,

    /// The fields of the document, including the built-in `_id` and
    /// `_creationTime`. For tombstones, only `_id` is present.
    #[schema(value_type = Object)]
    pub value: BTreeMap<String, JsonValue>,
}

/// The literal string `synced`, discriminating "synced" status objects.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, ToSchema)]
pub enum SyncedTag {
    #[serde(rename = "synced")]
    Synced,
}

/// The literal string `inProgress`, discriminating "in progress" status
/// objects.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, ToSchema)]
pub enum InProgressTag {
    #[serde(rename = "inProgress")]
    InProgress,
}

/// The consistency state reported alongside a data sync page, discriminated
/// by `type`.
// Modeled as a serde-untagged enum over structs that each carry a
// single-value `type` tag (rather than `#[serde(tag = "type")]`): the wire
// format is identical, but this shape lets utoipa emit an OpenAPI
// `discriminator`, which the docs render as tabs labeled by `type`.
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
#[serde(untagged)]
#[schema(discriminator(
    property_name = "type",
    mapping(
        ("synced" = "#/components/schemas/DataSyncSynced"),
        ("inProgress" = "#/components/schemas/DataSyncInProgress"),
    )
))]
pub enum DataSyncStatus {
    Synced(DataSyncSynced),
    InProgress(DataSyncInProgress),
}

/// The entries emitted so far represent a consistent snapshot at `syncedTs`.
/// The cursor can be persisted and used to continue the sync later (within
/// the document retention window).
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataSyncSynced {
    /// Always `synced`.
    #[serde(rename = "type")]
    #[schema(inline)]
    pub status_type: SyncedTag,
    /// The database timestamp at which the synced data is consistent.
    pub synced_ts: i64,
}

/// More pages are required before the view is consistent. The sync's progress
/// can be monitored via `/data/list_active_syncs`, keyed by the response's
/// `syncId`.
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataSyncInProgress {
    /// Always `inProgress`.
    #[serde(rename = "type")]
    #[schema(inline)]
    pub status_type: InProgressTag,
}

/// Response of the active-syncs listing API
/// (`/api/v1/data/list_active_syncs`).
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListActiveSyncsResponse {
    /// This page of active data syncs, most recently updated first. A sync is
    /// active if it fetched a page from `/api/v1/data/sync` within the past 3
    /// days.
    pub syncs: Vec<ActiveDataSync>,
    pub pagination: PaginationMetadata,
}

/// The status of one active data sync, as of its most recent page.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActiveDataSync {
    /// Unique id of the sync, assigned when it started (i.e. when
    /// `/api/v1/data/sync` was called without a cursor) and stable across its
    /// pages.
    pub sync_id: String,
    /// Wall-clock time of the last `/data/sync` call made by this sync, as a
    /// unix timestamp in milliseconds.
    pub last_updated: i64,
    /// The sync's progress as of its most recently recorded page.
    pub status: ActiveDataSyncStatus,
}

/// The progress of an active data sync, discriminated by `type`.
// See `DataSyncStatus` for why this is untagged with per-variant structs.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
#[serde(untagged)]
#[schema(discriminator(
    property_name = "type",
    mapping(
        ("inProgress" = "#/components/schemas/ActiveDataSyncInProgress"),
        ("synced" = "#/components/schemas/ActiveDataSyncSynced"),
    )
))]
pub enum ActiveDataSyncStatus {
    InProgress(ActiveDataSyncInProgress),
    Synced(ActiveDataSyncSynced),
}

/// The sync is still traversing its selected tables; the data returned so
/// far is not yet a consistent snapshot.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActiveDataSyncInProgress {
    /// Always `inProgress`.
    #[serde(rename = "type")]
    #[schema(inline)]
    pub status_type: InProgressTag,
    /// Tables whose initial traversal has completed.
    pub num_tables_synced: u64,
    /// Total tables selected for the sync.
    pub total_tables: u64,
    /// The component of the table currently being traversed (the empty
    /// string for the root component).
    pub current_component: String,
    /// The table currently being traversed.
    pub current_table: String,
    /// Documents synced so far from the current table.
    pub num_documents_in_current_table: u64,
    /// Total documents in the current table, as of a recent snapshot.
    pub total_documents_in_current_table: u64,
    /// Documents synced over the sync's lifetime, including deletions and
    /// re-synced revisions of documents that changed mid-sync — so this can
    /// slightly exceed `totalDocuments`.
    pub num_documents_synced: u64,
    /// Total documents across all selected tables, as of a recent snapshot.
    pub total_documents: u64,
}

/// The sync reached a consistent snapshot and is streaming later changes.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActiveDataSyncSynced {
    /// Always `synced`.
    #[serde(rename = "type")]
    #[schema(inline)]
    pub status_type: SyncedTag,
    /// Total tables selected for the sync.
    pub total_tables: u64,
    /// Documents synced over the sync's lifetime, including deletions and
    /// re-synced revisions.
    pub num_documents_synced: u64,
    /// The database timestamp at which the synced data is consistent.
    pub synced_ts: i64,
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
pub struct GetTableColumnNamesResponse {
    pub by_component: BTreeMap<String, Vec<GetTableColumnNameTable>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTableColumnNameTable {
    pub name: String,
    pub columns: Vec<String>,
}
