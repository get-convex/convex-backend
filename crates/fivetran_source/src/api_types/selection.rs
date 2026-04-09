use std::collections::BTreeMap;

use fivetran_common::fivetran_sdk::{
    selection::Selection as FivetranSelection,
    SchemaSelection as FivetranSchemaSelection,
    Selection as FivetranRootSelection,
    TableSelection as FivetranTableSelection,
    TablesWithSchema as FivetranSelectionWithSchema,
};
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

/// The name of the Fivetran schema that we use for the Convex tables in the
/// root component (i.e. the “database name” users see for the Convex root
/// component)
pub const DEFAULT_FIVETRAN_SCHEMA_NAME: &str = "app";

impl TryFrom<Option<FivetranRootSelection>> for Selection {
    type Error = anyhow::Error;

    fn try_from(value: Option<FivetranRootSelection>) -> Result<Self, Self::Error> {
        Selection::try_from(value.and_then(|val| val.selection))
    }
}

impl TryFrom<Option<FivetranSelection>> for Selection {
    type Error = anyhow::Error;

    fn try_from(value: Option<FivetranSelection>) -> Result<Self, Self::Error> {
        match value {
            None => Ok(Selection::default()),
            Some(FivetranSelection::WithSchema(with_schema)) => Ok(with_schema.into()),
            Some(FivetranSelection::WithoutSchema(_)) => {
                anyhow::bail!("Fivetran unexpectedly sent a selection setting without a schema.")
            },
        }
    }
}

impl From<FivetranSelectionWithSchema> for Selection {
    fn from(value: FivetranSelectionWithSchema) -> Self {
        Self {
            components: value
                .schemas
                .into_iter()
                .map(|schema| {
                    (
                        if schema.schema_name == DEFAULT_FIVETRAN_SCHEMA_NAME {
                            String::from("")
                        } else {
                            schema.schema_name.clone()
                        },
                        schema.into(),
                    )
                })
                .collect(),
            other_components: if value.include_new_schemas {
                InclusionDefault::Included
            } else {
                InclusionDefault::Excluded
            },
        }
    }
}

impl From<FivetranSchemaSelection> for ComponentSelection {
    fn from(value: FivetranSchemaSelection) -> Self {
        if !value.included {
            Self::Excluded
        } else {
            Self::Included {
                tables: value
                    .tables
                    .into_iter()
                    .map(|table| (table.table_name.clone(), table.into()))
                    .collect(),
                other_tables: if value.include_new_tables {
                    InclusionDefault::Included
                } else {
                    InclusionDefault::Excluded
                },
            }
        }
    }
}

impl From<FivetranTableSelection> for TableSelection {
    fn from(value: FivetranTableSelection) -> Self {
        if !value.included {
            Self::Excluded
        } else {
            Self::Included {
                columns: value
                    .columns
                    .into_iter()
                    .map(|(name, included)| {
                        (
                            name,
                            if included {
                                ColumnInclusion::Included
                            } else {
                                ColumnInclusion::Excluded
                            },
                        )
                    })
                    .collect(),
                other_columns: if value.include_new_columns {
                    InclusionDefault::Included
                } else {
                    InclusionDefault::Excluded
                },
            }
        }
    }
}
