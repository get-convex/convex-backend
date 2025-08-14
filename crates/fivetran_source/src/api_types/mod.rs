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

/// Since [ListSnapshotArgs] and [DocumentDeltasArgs] need to support the older
/// selection formats, this wraps the newer selection format ([Selection]) while
/// providing a way to deserialize the older formats.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
#[cfg_attr(test, derive(Eq, PartialEq, Debug))]
pub enum SelectionArg {
    /// Newer selection format, allows to select specific tables, components,
    /// and columns.
    Exact { selection: Selection },

    /// If only the table name is provided, assumes it’s in the root component.
    SingleTable {
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

#[cfg(test)]
mod tests {
    use super::{
        Selection,
        SelectionArg,
    };
    use crate::api_types::ListSnapshotArgs;

    #[test]
    fn test_no_args() {
        assert_selection_arg_deserialization("{}", SelectionArg::Everything {});
    }

    #[test]
    fn test_table_name_only() {
        assert_selection_arg_deserialization(
            r#"{"table_name": "users"}"#,
            SelectionArg::SingleTable {
                table_name: "users".to_string(),
                component: None,
            },
        );
    }

    #[test]
    fn test_table_name_and_component() {
        assert_selection_arg_deserialization(
            r#"{"table_name": "jobs", "component": "cron"}"#,
            SelectionArg::SingleTable {
                table_name: "jobs".to_string(),
                component: Some("cron".to_string()),
            },
        );
    }

    #[test]
    fn test_component_only() {
        assert_selection_arg_deserialization(
            r#"{"component": "waitlist"}"#,
            SelectionArg::SingleComponent {
                component: "waitlist".to_string(),
            },
        );
    }

    #[test]
    fn test_exact_selection() {
        assert_selection_arg_deserialization(
            r#"{"selection": { "_other": "incl" }}"#,
            SelectionArg::Exact {
                selection: Selection::default(),
            },
        );
    }

    fn assert_selection_arg_deserialization(json: &str, expected_selection: SelectionArg) {
        let args: ListSnapshotArgs =
            serde_json::from_str(json).expect("can’t deserialize to ListSnapshotArgs");
        assert_eq!(args.cursor, None);
        assert_eq!(args.snapshot, None);
        assert_eq!(args.format, None);
        assert_eq!(args.selection, expected_selection);
    }
}
