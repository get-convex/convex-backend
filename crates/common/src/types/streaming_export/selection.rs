use std::collections::BTreeMap;

use maplit::btreemap;
use serde::{
    Deserialize,
    Serialize,
};

use super::SelectionArg;

/// Defines the components, tables, and columns to export in a deployment.
///
/// This is the serializable version of `StreamingExportSelection` in the
/// database crate.
#[derive(Serialize, Deserialize, Clone)]
pub struct Selection {
    #[serde(flatten)]
    pub components: BTreeMap<
        String, // The component name ("" for the default component)
        ComponentSelection,
    >,
    #[serde(rename = "_other")]
    pub other_components: InclusionDefault,
}

/// Serializable version of `StreamingExportInclusionDefault`
#[derive(Clone, Copy, Eq, PartialEq, Debug, Serialize, Deserialize)]
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

/// Serializable version of `StreamingExportComponentSelection`.
#[derive(Serialize, Deserialize, Clone)]
pub enum ComponentSelection {
    #[serde(rename = "excl")]
    Excluded,
    #[serde(untagged)]
    Included {
        #[serde(flatten)]
        tables: BTreeMap<String, TableSelection>,
        #[serde(rename = "_other")]
        other_tables: InclusionDefault,
    },
}

/// Serializable version of
/// `StreamingExportTableSelection` + `StreamingExportColumnSelection`
#[derive(Serialize, Deserialize, Clone)]
pub enum TableSelection {
    #[serde(rename = "excl")]
    Excluded,
    #[serde(untagged)]
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

/// Serializable version of `StreamingExportColumnInclusion`.
#[derive(Clone, Copy, Eq, PartialEq, Debug, Serialize, Deserialize)]
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
