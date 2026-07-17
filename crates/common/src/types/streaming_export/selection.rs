use std::collections::BTreeMap;

use maplit::btreemap;
use serde::{
    Deserialize,
    Serialize,
};
use utoipa::ToSchema;

use super::SelectionArg;

/// Selects the components, tables, and columns to export. Each key is a
/// component path (`""` for the root component, which holds your tables
/// unless you use components), mapped to the selection for that component;
/// the required `_other` key sets the default for components not listed.
#[derive(Serialize, Deserialize, Clone, ToSchema)]
#[schema(example = json!({
    "_other": "excl",
    "": {
        "_other": "excl",
        "posts": {"_other": "incl"},
        "users": {"_other": "incl", "ssn": "excl"},
    },
}))]
pub struct Selection {
    #[serde(flatten)]
    pub components: BTreeMap<
        String, // The component name ("" for the default component)
        ComponentSelection,
    >,
    #[serde(rename = "_other")]
    pub other_components: InclusionDefault,
}

/// Whether items not explicitly listed are exported (`"incl"`) or not
/// (`"excl"`).
#[derive(Clone, Copy, Eq, PartialEq, Debug, Serialize, Deserialize, ToSchema)]
pub enum InclusionDefault {
    #[serde(rename = "excl")]
    Excluded,
    #[serde(rename = "incl")]
    Included,
}

impl Default for Selection {
    fn default() -> Self {
        // By default, include everything.
        Self {
            components: BTreeMap::new(),
            other_components: InclusionDefault::Included,
        }
    }
}

/// The literal string `"excl"`, excluding the item entirely.
// A dedicated newtype-wrapped tag (rather than a `#[serde(rename)]`d unit
// variant with `#[serde(untagged)]` on the *other* variant) so that the
// enums below can use container-level `#[serde(untagged)]`: the wire format
// is identical, but utoipa's `ToSchema` derive only understands `untagged`
// at the container level and silently ignores it on variants.
#[derive(Clone, Copy, Eq, PartialEq, Debug, Serialize, Deserialize, ToSchema)]
pub enum ExcludedTag {
    #[serde(rename = "excl")]
    Excluded,
}

/// What to export from one component: either the literal string `"excl"` to
/// exclude the component entirely, or an object selecting some of its
/// tables — each key is a table name, and the required `_other` key sets the
/// default for tables not listed.
// Serializable version of `StreamingExportComponentSelection`.
#[derive(Serialize, Deserialize, Clone, ToSchema)]
#[serde(untagged)]
pub enum ComponentSelection {
    /// Exclude this component entirely.
    Excluded(ExcludedTag),
    /// Export some of this component's tables.
    Included {
        #[serde(flatten)]
        tables: BTreeMap<String, TableSelection>,
        #[serde(rename = "_other")]
        other_tables: InclusionDefault,
    },
}

/// What to export from one table: either the literal string `"excl"` to
/// exclude the table entirely, or an object selecting some of its columns —
/// each key is a column (field) name mapped to whether it is exported, and
/// the required `_other` key sets the default for columns not listed. `_id`
/// cannot be excluded.
// Serializable version of
// `StreamingExportTableSelection` + `StreamingExportColumnSelection`.
#[derive(Serialize, Deserialize, Clone, ToSchema)]
#[serde(untagged)]
pub enum TableSelection {
    /// Exclude this table entirely.
    Excluded(ExcludedTag),
    /// Export some of this table's columns.
    Included {
        #[serde(flatten)]
        columns: BTreeMap<String, ColumnInclusion>,
        #[serde(rename = "_other")]
        other_columns: InclusionDefault,
    },
}

impl TableSelection {
    pub fn included_with_all_columns() -> Self {
        Self::Included {
            columns: BTreeMap::new(),
            other_columns: InclusionDefault::Included,
        }
    }
}

/// Whether the column is exported.
// Serializable version of `StreamingExportColumnInclusion`.
#[derive(Clone, Copy, Eq, PartialEq, Debug, Serialize, Deserialize, ToSchema)]
pub enum ColumnInclusion {
    #[serde(rename = "excl")]
    Excluded,
    #[serde(rename = "incl")]
    Included,
}

impl From<SelectionArg> for Selection {
    fn from(arg: SelectionArg) -> Self {
        match arg {
            SelectionArg::Exact { selection } => selection,
            SelectionArg::SingleTable {
                table_name,
                component,
            } => Self {
                components: btreemap! {
                    component.unwrap_or(String::from("")) => ComponentSelection::Included {
                        tables: btreemap! {
                            table_name => TableSelection::included_with_all_columns(),
                        },
                        other_tables: InclusionDefault::Excluded,
                    },
                },
                other_components: InclusionDefault::Excluded,
            },
            SelectionArg::SingleComponent { component } => Self {
                components: btreemap! {
                    component => ComponentSelection::Included {
                        tables: BTreeMap::new(),
                        other_tables: InclusionDefault::Included,
                    },
                },
                other_components: InclusionDefault::Excluded,
            },
            SelectionArg::Everything {} => Self {
                components: BTreeMap::new(),
                other_components: InclusionDefault::Included,
            },
        }
    }
}
