use std::collections::BTreeMap;

use convex_fivetran_common::fivetran_sdk::{
    selection::Selection as FivetranSelection,
    SchemaSelection as FivetranSchemaSelection,
    TableSelection as FivetranTableSelection,
    TablesWithSchema as FivetranSelectionWithSchema,
};
use maplit::btreemap;
#[cfg(test)]
use proptest::prelude::*;
use serde::{
    Deserialize,
    Serialize,
};

use super::SelectionArg;

/// Defines the components, tables, and columns to export in a deployment.
///
/// This is the serializable version of `StreamingExportSelection` in the
/// database crate.
#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Eq, PartialEq, Debug, Clone, proptest_derive::Arbitrary))]
pub struct Selection {
    #[serde(flatten)]
    #[cfg_attr(
        test,
        proptest(strategy = "prop::collection::btree_map(any::<String>(), \
                             any::<ComponentSelection>(), 0..3)")
    )]
    pub components: BTreeMap<
        String, // The component name ("" for the default component)
        ComponentSelection,
    >,
    #[serde(rename = "_other")]
    pub other_components: InclusionDefault,
}

/// Serializable version of `StreamingExportInclusionDefault`
#[derive(Clone, Copy, Eq, PartialEq, Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
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
#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Clone, Eq, PartialEq, Debug, proptest_derive::Arbitrary))]
pub enum ComponentSelection {
    #[serde(rename = "excl")]
    Excluded,
    #[serde(untagged)]
    Included {
        #[serde(flatten)]
        #[cfg_attr(
            test,
            proptest(strategy = "prop::collection::btree_map(any::<String>(), \
                                 any::<TableSelection>(), 0..3)")
        )]
        tables: BTreeMap<String, TableSelection>,
        #[serde(rename = "_other")]
        other_tables: InclusionDefault,
    },
}

/// Serializable version of
/// `StreamingExportTableSelection` + `StreamingExportColumnSelection`
#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Clone, Eq, PartialEq, Debug, proptest_derive::Arbitrary))]
pub enum TableSelection {
    #[serde(rename = "excl")]
    Excluded,
    #[serde(untagged)]
    Included {
        #[serde(flatten)]
        #[cfg_attr(
            test,
            proptest(strategy = "prop::collection::btree_map(any::<String>(), \
                                 any::<ColumnInclusion>(), 0..3)")
        )]
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
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
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
pub const DEFAULT_FIVETRAN_SCHEMA_NAME: &str = "convex";

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

#[cfg(test)]
mod tests_selection_serde {
    use maplit::btreemap;
    use serde::{
        de::DeserializeOwned,
        Serialize,
    };
    use serde_json::{
        json,
        Value as JsonValue,
    };

    use super::*;

    #[test]
    pub fn test_selection_serde() {
        assert_serde(
            json!({
                "": json!({
                    "users": json!({
                        "name": json!("incl"),
                        "_creationTime": json!("incl"),
                        "_deleted": json!("incl"),
                        "password": json!("excl"),
                        "_other": json!("excl"),
                    }),
                    "_other": json!("excl"),
                }),
                "waitlist": json!("excl"),
                "_other": json!("incl"),
            }),
            Selection {
                components: btreemap! {
                    "".to_string() => ComponentSelection::Included {
                        tables: btreemap! {
                            "users".to_string() => TableSelection::Included {
                                columns: btreemap! {
                                    "name".to_string() => ColumnInclusion::Included,
                                    "_creationTime".to_string() => ColumnInclusion::Included,
                                    "_deleted".to_string() => ColumnInclusion::Included,
                                    "password".to_string() => ColumnInclusion::Excluded,
                                },
                                other_columns: InclusionDefault::Excluded,
                            },
                        },
                        other_tables: InclusionDefault::Excluded,
                    },
                    "waitlist".to_string() => ComponentSelection::Excluded,
                },
                other_components: InclusionDefault::Included,
            },
        );
    }

    #[test]
    fn test_serde_inclusion() {
        assert_serde(json!("incl"), InclusionDefault::Included);
        assert_serde(json!("excl"), InclusionDefault::Excluded);
    }

    #[test]
    fn test_serde_table() {
        assert_serde(json!("excl"), TableSelection::Excluded);
        assert_serde(
            json!({ "_other": "incl" }),
            TableSelection::Included {
                columns: BTreeMap::new(),
                other_columns: InclusionDefault::Included,
            },
        );
    }

    #[test]
    fn test_serde_component() {
        assert_serde(json!("excl"), ComponentSelection::Excluded);
        assert_serde(
            json!({ "users": json!({ "_other": "incl" }), "_other": "excl" }),
            ComponentSelection::Included {
                tables: btreemap! {
                    "users".to_string() => TableSelection::Included {
                        columns: BTreeMap::new(),
                        other_columns: InclusionDefault::Included,
                    },
                },
                other_tables: InclusionDefault::Excluded,
            },
        );
    }

    fn assert_serde<T: DeserializeOwned + Serialize + Eq + std::fmt::Debug + Clone>(
        json: JsonValue,
        value: T,
    ) {
        let serialized = serde_json::to_value(&value).expect("can’t serialize to JSON");
        assert_eq!(
            json, serialized,
            "Incorrect serialization\n➡️ Input: {value:?}\n✅ Expected: {json:?}\n❌ Got:      \
             {serialized:?}"
        );

        let deserialized: T = serde_json::from_value(json.clone()).expect("can’t deserialize to T");
        assert_eq!(
            value, deserialized,
            "Incorrect deserialization\n➡️ Input: {json:?}\n✅ Expected: {value:?}\n❌ Got:      \
             {deserialized:?}"
        );
    }
}

#[cfg(test)]
impl From<Selection> for FivetranSelectionWithSchema {
    fn from(value: Selection) -> Self {
        Self {
            schemas: value
                .components
                .into_iter()
                .map(|(component_name, component_selection)| {
                    let schema_name = if component_name.is_empty() {
                        DEFAULT_FIVETRAN_SCHEMA_NAME.to_string()
                    } else {
                        component_name
                    };
                    FivetranSchemaSelection {
                        schema_name,
                        included: match component_selection {
                            ComponentSelection::Excluded => false,
                            ComponentSelection::Included { .. } => true,
                        },
                        tables: match component_selection {
                            ComponentSelection::Excluded => vec![],
                            ComponentSelection::Included {
                                ref tables,
                                other_tables: _,
                            } => tables
                                .iter()
                                .map(|(table_name, table_selection)| FivetranTableSelection {
                                    table_name: table_name.clone(),
                                    included: match table_selection {
                                        TableSelection::Excluded => false,
                                        TableSelection::Included { .. } => true,
                                    },
                                    columns: match &table_selection {
                                        TableSelection::Excluded => BTreeMap::new(),
                                        TableSelection::Included { columns, .. } => columns
                                            .iter()
                                            .map(|(name, inclusion)| {
                                                (
                                                    name.clone(),
                                                    matches!(inclusion, ColumnInclusion::Included),
                                                )
                                            })
                                            .collect(),
                                    },
                                    include_new_columns: match table_selection {
                                        TableSelection::Excluded => false,
                                        TableSelection::Included { other_columns, .. } => {
                                            matches!(other_columns, InclusionDefault::Included)
                                        },
                                    },
                                })
                                .collect(),
                        },
                        include_new_tables: match &component_selection {
                            ComponentSelection::Excluded => false,
                            ComponentSelection::Included { other_tables, .. } => {
                                matches!(other_tables, InclusionDefault::Included)
                            },
                        },
                    }
                })
                .collect(),
            include_new_schemas: matches!(value.other_components, InclusionDefault::Included),
        }
    }
}

#[cfg(test)]
mod tests_selection_fivetran_conversion {
    use convex_fivetran_common::fivetran_sdk::TablesWithNoSchema as FivetranSelectionWithNoSchema;
    use maplit::btreemap;

    use super::*;

    #[test]
    fn test_schema_equals_none_converts_to_everything_included() {
        let result: Result<Selection, _> = Selection::try_from(None);
        assert_eq!(
            result.unwrap(),
            Selection {
                components: BTreeMap::new(),
                other_components: InclusionDefault::Included,
            }
        );
    }

    #[test]
    fn test_can_convert_from_fivetran_selection_with_schema() {
        let fivetran_selection = FivetranSelection::WithSchema(FivetranSelectionWithSchema {
            schemas: vec![FivetranSchemaSelection {
                schema_name: "convex".to_string(),
                included: true,
                tables: vec![FivetranTableSelection {
                    table_name: "users".to_string(),
                    included: true,
                    columns: btreemap! {
                        "name".to_string() => true,
                        "email".to_string() => false,
                    },
                    include_new_columns: false,
                }],
                include_new_tables: true,
            }],
            include_new_schemas: false,
        });

        let result: Result<Selection, _> = Selection::try_from(Some(fivetran_selection));
        assert!(result.is_ok());

        let expected = Selection {
            components: btreemap! {
                "".to_string() => ComponentSelection::Included {
                    tables: btreemap! {
                        "users".to_string() => TableSelection::Included {
                            columns: btreemap! {
                                "name".to_string() => ColumnInclusion::Included,
                                "email".to_string() => ColumnInclusion::Excluded,
                            },
                            other_columns: InclusionDefault::Excluded,
                        },
                    },
                    other_tables: InclusionDefault::Included,
                },
            },
            other_components: InclusionDefault::Excluded,
        };

        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_without_schema_variant_returns_error() {
        let without_schema_selection = FivetranSelectionWithNoSchema {
            tables: vec![FivetranTableSelection {
                included: true,
                table_name: "table1".to_string(),
                columns: BTreeMap::new(),
                include_new_columns: false,
            }],
            include_new_tables: false,
        };
        let fivetran_selection = FivetranSelection::WithoutSchema(without_schema_selection);

        let result: Result<Selection, _> = Selection::try_from(Some(fivetran_selection));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Fivetran unexpectedly sent a selection setting without a schema"));
    }

    #[test]
    fn test_convex_component_in_fivetran_maps_to_empty_string() {
        // Fivetran doesn’t support empty schema names, so the root component ("" in
        // Convex) is called "convex" in Fivetran

        let fivetran_selection = FivetranSelectionWithSchema {
            schemas: vec![FivetranSchemaSelection {
                schema_name: "convex".to_string(),
                included: true,
                tables: vec![],
                include_new_tables: true,
            }],
            include_new_schemas: false,
        };

        let selection: Selection = fivetran_selection.into();

        assert_eq!(
            selection,
            Selection {
                components: btreemap! {
                    "".to_string() => ComponentSelection::Included {
                        tables: BTreeMap::new(),
                        other_tables: InclusionDefault::Included,
                    },
                },
                other_components: InclusionDefault::Excluded,
            }
        );
    }

    #[cfg(test)]
    mod tests_selection_roundtrip {
        use cmd_util::env::env_config;
        use proptest::prelude::*;

        use super::*;

        proptest! {
            #![proptest_config(ProptestConfig {
                cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1),
                failure_persistence: None, ..ProptestConfig::default()
            })]
            #[test]
            fn test_selection_to_fivetran_roundtrips(selection in any::<Selection>()) {
                let fivetran_selection: FivetranSelectionWithSchema = selection.clone().into();
                let roundtripped_selection: Selection = fivetran_selection.into();
                prop_assert_eq!(selection, roundtripped_selection);
            }
        }
    }
}
