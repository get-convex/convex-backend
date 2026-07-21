use std::collections::BTreeMap;

use maplit::btreemap;
use serde::{
    Deserialize,
    Serialize,
};
use utoipa::ToSchema;

use super::SelectionArg;

#[derive(Serialize, Deserialize, Clone, ToSchema)]
#[schema(example = json!({
    "_other": "excluded",
    "": {
        "_other": "excluded",
        "posts": {"_other": "included"},
        "users": {"_other": "included", "ssn": "excluded"},
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

/// Whether items not explicitly listed are exported
#[derive(Clone, Copy, Eq, PartialEq, Debug, Serialize, Deserialize, ToSchema)]
pub enum InclusionDefault {
    #[serde(alias = "excl", rename = "excluded")]
    Excluded,
    #[serde(alias = "incl", rename = "included")]
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

/// The literal string `"excluded"`, excluding the item entirely.
#[derive(Clone, Copy, Eq, PartialEq, Debug, Serialize, Deserialize, ToSchema)]
pub enum ExcludedTag {
    #[serde(alias = "excl", rename = "excluded")]
    Excluded,
}

/// Set of components to include/exclude in sync.
///
/// Mapping from the component path to the inclusion/exclusion.
/// Use the empty string to represent the root component.
#[derive(Serialize, Deserialize, Clone, ToSchema)]
#[serde(untagged)]
pub enum ComponentSelection {
    /// Export some of this component's tables.
    Included {
        #[serde(flatten)]
        tables: BTreeMap<String, TableSelection>,
        #[serde(rename = "_other")]
        other_tables: InclusionDefault,
    },
    /// Exclude this component entirely.
    Excluded(ExcludedTag),
}

/// What to export from one table: either the literal string `"excluded"` to
/// exclude the table entirely, or an object selecting some of its columns —
/// each key is a column (field) name mapped to whether it is exported, and
/// the required `_other` key sets the default for columns not listed. `_id`
/// cannot be excluded.
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
    #[serde(alias = "excl", rename = "excluded")]
    Excluded,
    #[serde(alias = "incl", rename = "included")]
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
