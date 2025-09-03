use std::collections::BTreeMap;

use convex_fivetran_common::fivetran_sdk::{
    Column,
    DataType,
    Schema,
    SchemaList,
    Table,
};

use crate::{
    api_types::selection::DEFAULT_FIVETRAN_SCHEMA_NAME,
    convex_api::{
        ComponentPath,
        FieldName,
        TableName,
    },
};

/// Generates the Fivetran schema (the list of tables by database) from the
/// Convex tables
pub fn generate_fivetran_schema(
    tables_by_component: BTreeMap<ComponentPath, BTreeMap<TableName, Vec<FieldName>>>,
) -> SchemaList {
    SchemaList {
        schemas: tables_by_component
            .into_iter()
            .map(|(component_path, tables)| Schema {
                name: fivetran_schema_name(component_path),
                tables: compute_fivetran_table_list(tables),
            })
            .collect(),
    }
}

fn fivetran_schema_name(component_path: ComponentPath) -> String {
    if component_path.0.is_empty() {
        DEFAULT_FIVETRAN_SCHEMA_NAME.to_string()
    } else {
        component_path.0
    }
}

fn compute_fivetran_table_list(tables: BTreeMap<TableName, Vec<FieldName>>) -> Vec<Table> {
    tables
        .into_iter()
        .map(|(table_name, column_names)| Table {
            name: table_name.to_string(),
            columns: column_names
                .into_iter()
                .map(|column_name| {
                    let column_name: String = column_name.to_string();
                    Column {
                        name: column_name.clone(),
                        r#type: match column_name.as_str() {
                            "_id" => DataType::String,
                            "_creationTime" => DataType::UtcDatetime,
                            // We map every non-system column to the “unspecified” data type
                            // and let Fivetran infer the correct column type from the data
                            // it receives.
                            _ => DataType::Unspecified,
                        } as i32,
                        primary_key: column_name == "_id",
                        params: None,
                    }
                })
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use maplit::btreemap;

    use super::*;

    #[test]
    fn test_fivetran_schema() -> anyhow::Result<()> {
        let tables_by_component = btreemap! {
            "".into() => btreemap! {
                "users".into() => vec![
                    "_id".into(),
                    "_creationTime".into(),
                    "otherField".into(),
                ],
            },
            "crons".into() => btreemap! {
                "jobs".into() => vec![
                    "_id".into(),
                    "_creationTime".into(),
                    "name".into(),
                ],
            },
        };

        assert_eq!(
            generate_fivetran_schema(tables_by_component),
            SchemaList {
                schemas: vec![
                    Schema {
                        name: "convex".to_string(),
                        tables: vec![Table {
                            name: "users".to_string(),
                            columns: vec![
                                Column {
                                    name: "_id".to_string(),
                                    r#type: DataType::String as i32,
                                    primary_key: true,
                                    params: None,
                                },
                                Column {
                                    name: "_creationTime".to_string(),
                                    r#type: DataType::UtcDatetime as i32,
                                    primary_key: false,
                                    params: None,
                                },
                                Column {
                                    name: "otherField".to_string(),
                                    r#type: DataType::Unspecified as i32,
                                    primary_key: false,
                                    params: None,
                                },
                            ],
                        }],
                    },
                    Schema {
                        name: "crons".to_string(),
                        tables: vec![Table {
                            name: "jobs".to_string(),
                            columns: vec![
                                Column {
                                    name: "_id".to_string(),
                                    r#type: DataType::String as i32,
                                    primary_key: true,
                                    params: None,
                                },
                                Column {
                                    name: "_creationTime".to_string(),
                                    r#type: DataType::UtcDatetime as i32,
                                    primary_key: false,
                                    params: None,
                                },
                                Column {
                                    name: "name".to_string(),
                                    r#type: DataType::Unspecified as i32,
                                    primary_key: false,
                                    params: None,
                                },
                            ],
                        }],
                    },
                ],
            },
        );
        Ok(())
    }

    #[test]
    fn test_fivetran_schema_name() {
        assert_eq!(fivetran_schema_name(ComponentPath::root()), "convex");
        assert_eq!(
            fivetran_schema_name(ComponentPath("myComponent".to_string())),
            "myComponent"
        );
    }
}
