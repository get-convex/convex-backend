#[macro_export]
// Turns a mapping of tableName => (index_name, vec![index_fields]) into a
// DatabaseSchema struct.
macro_rules! db_schema_with_indexes {
    ($($table:expr => [$(($index_name:expr, $fields:expr)),*]),* $(,)?) => {
        {
            #[allow(unused)]
            let mut tables = std::collections::BTreeMap::new();
            {
                $(
                    let table_name: common::types::TableName = str::parse($table)?;
                    #[allow(unused)]
                    let mut indexes = std::collections::BTreeMap::new();
                    $(
                        let index_name = database::test_helpers::index_utils::new_index_name(
                            $table,
                            $index_name,
                        )?;
                        let field_paths: Vec<common::paths::FieldPath> = $fields
                            .iter()
                            .map(|s| str::parse(s).unwrap())
                            .collect();
                        indexes.insert(
                            index_name.descriptor().clone(),
                            common::schemas::IndexSchema {
                                index_descriptor: index_name.descriptor().clone(),
                                fields: field_paths.try_into()?,
                            },
                        );
                    )*
                    let table_def = common::schemas::TableDefinition {
                        table_name: table_name.clone(),
                        indexes,
                        staged_db_indexes: Default::default(),
                        text_indexes: Default::default(),
                        staged_text_indexes: Default::default(),
                        vector_indexes: Default::default(),
                        staged_vector_indexes: Default::default(),
                        document_type: None,
                    };
                    tables.insert(table_name, table_def);
                )*
            }
            common::schemas::DatabaseSchema {
                tables,
                schema_validation: true,
            }
        }
    };
}

#[macro_export]
macro_rules! db_schema_with_search_indexes {
    ($($table:expr => [$(($index_name:expr, $field:expr)),*]),* $(,)?) => {
        {
            use std::collections::{
                BTreeMap,
                BTreeSet,
            };
            use common::types::TableName;
            use common::paths::FieldPath;
            use common::schemas::{
                TableDefinition,
                TextIndexSchema,
            };
            use database::test_helpers::index_utils::new_index_name;

            #[allow(unused)]
            let mut tables = BTreeMap::new();
            {
                $(
                    let table_name: TableName = str::parse($table)?;
                    #[allow(unused)]
                    let mut text_indexes = BTreeMap::new();
                    $(
                        let index_name = new_index_name($table, $index_name)?;
                        let field_path: FieldPath = str::parse($field).unwrap();
                        text_indexes.insert(
                            index_name.descriptor().clone(),
                            TextIndexSchema::new(
                                index_name.descriptor().clone(),
                                field_path.try_into()?,
                                BTreeSet::new(),
                            )?,
                        );
                    )*
                    let table_def = TableDefinition {
                        table_name: table_name.clone(),
                        indexes: BTreeMap::new(),
                        staged_db_indexes: Default::default(),
                        text_indexes,
                        staged_text_indexes: Default::default(),
                        vector_indexes: Default::default(),
                        staged_vector_indexes: Default::default(),
                        document_type: None,
                    };
                    tables.insert(table_name, table_def);
                )*
            }
            DatabaseSchema {
                tables,
                schema_validation: true,
            }
        }
    };
}

#[macro_export]
// Turns a mapping of tableName => (index_name, vector_field) into a
// DatabaseSchema struct.
macro_rules! db_schema_with_vector_indexes {
    ($($table:expr => {
        $document_schema:expr, [$(($index_name:expr, $vector_field:expr)),*]
    }),* $(,)?) => {
        {
            #[allow(unused)]
            use std::str::FromStr;
            #[allow(unused)]
            let mut tables = std::collections::BTreeMap::new();
            {
                $(
                    let table_name: $crate::types::TableName =
                        str::parse($table)?;
                    #[allow(unused)]
                    let mut vector_indexes = std::collections::BTreeMap::new();
                    $(
                        let index_name = $crate::types::IndexName::new(
                            str::parse($table)?,
                            $crate::types::IndexDescriptor::new($index_name)?
                        )?;
                        vector_indexes.insert(
                            index_name.descriptor().clone(),
                            $crate::schemas::VectorIndexSchema::new(
                                index_name.descriptor().clone(),
                                value::FieldPath::from_str($vector_field)?,
                                1536u32.try_into()?,
                                Default::default(),
                            )?,
                        );
                    )*
                    let table_def = $crate::schemas::TableDefinition {
                        table_name: table_name.clone(),
                        indexes: Default::default(),
                        staged_db_indexes: Default::default(),
                        text_indexes: Default::default(),
                        staged_text_indexes: Default::default(),
                        vector_indexes,
                        staged_vector_indexes: Default::default(),
                        document_type: Some($document_schema),
                    };
                    tables.insert(table_name, table_def);
                )*
            }
            $crate::schemas::DatabaseSchema {
                tables,
                schema_validation: true,
            }
        }
    };
}
