#[macro_export]
// Turns a mapping of tableName => (index_name, vec![index_fields]) into a
// DatabaseSchema struct.
macro_rules! db_schema_with_indexes {
    ($($table:expr => {
        $( indexes: $(($index_name:expr, $fields:expr)),*$(,)? )?
        $( staged_db_indexes: $((
            $staged_db_index_name:expr,
            $staged_db_index_fields:expr
        )),*$(,)? )?
        $( text_indexes: $(($text_index_name:expr, $text_index_field:expr)),*$(,)? )?
        $( staged_text_indexes: $((
            $staged_text_index_name:expr,
            $staged_text_index_field:expr
        )),*$(,)? )?
        $( vector_indexes: $(($vector_index_name:expr, $vector_index_field:expr)),*$(,)? )?
        $( staged_vector_indexes: $((
            $staged_vector_index_name:expr,
            $staged_vector_index_field:expr
        )),*$(,)? )?
        $( document_schema: $document_schema:expr$(,)? )?
    }),* $(,)?) => {
        {
            #[allow(unused)]
            let mut tables = std::collections::BTreeMap::new();
            {
                $(
                    let table_name: $crate::types::TableName = $table.parse()?;
                    #[allow(unused)]
                    let mut indexes = std::collections::BTreeMap::new();
                    $($(
                        let index_name = $crate::index::test_helpers::new_index_name(
                            $table,
                            $index_name,
                        )?;
                        let field_paths = $fields
                            .iter()
                            .map(|s| s.parse())
                            .collect::<anyhow::Result<Vec<_>>>()?;
                        indexes.insert(
                            index_name.descriptor().clone(),
                            $crate::schemas::IndexSchema {
                                index_descriptor: index_name.descriptor().clone(),
                                fields: field_paths.try_into()?,
                            },
                        );
                    )*)?
                    #[allow(unused)]
                    let mut staged_db_indexes = std::collections::BTreeMap::new();
                    $($(
                        let index_name = $crate::index::test_helpers::new_index_name(
                            $table,
                            $staged_db_index_name,
                        )?;
                        let field_paths = $staged_db_index_fields
                            .iter()
                            .map(|s| s.parse())
                            .collect::<anyhow::Result<Vec<_>>>()?;
                        staged_db_indexes.insert(
                            index_name.descriptor().clone(),
                            $crate::schemas::IndexSchema {
                                index_descriptor: index_name.descriptor().clone(),
                                fields: field_paths.try_into()?,
                            },
                        );
                    )*)?
                    #[allow(unused)]
                    let mut text_indexes = std::collections::BTreeMap::new();
                    $($(
                        let index_name = $crate::index::test_helpers::new_index_name(
                            $table,
                            $text_index_name,
                        )?;
                        let field_path = $text_index_field.parse()?;
                        text_indexes.insert(
                            index_name.descriptor().clone(),
                            $crate::schemas::TextIndexSchema::new(
                                index_name.descriptor().clone(),
                                field_path,
                                std::collections::BTreeSet::new(),
                            )?,
                        );
                    )*)?
                    #[allow(unused)]
                    let mut staged_text_indexes = std::collections::BTreeMap::new();
                    $($(
                        let index_name = $crate::index::test_helpers::new_index_name(
                            $table,
                            $staged_text_index_name,
                        )?;
                        let field_path = $staged_text_index_field.parse()?;
                        staged_text_indexes.insert(
                            index_name.descriptor().clone(),
                            $crate::schemas::TextIndexSchema::new(
                                index_name.descriptor().clone(),
                                field_path,
                                std::collections::BTreeSet::new(),
                            )?,
                        );
                    )*)?
                    #[allow(unused)]
                    let mut vector_indexes = std::collections::BTreeMap::new();
                    $($(
                        let index_name = $crate::index::test_helpers::new_index_name(
                            $table,
                            $vector_index_name,
                        )?;
                        vector_indexes.insert(
                            index_name.descriptor().clone(),
                            $crate::schemas::VectorIndexSchema::new(
                                index_name.descriptor().clone(),
                                $vector_index_field.parse()?,
                                1536u32.try_into()?,
                                Default::default(),
                            )?,
                        );
                    )*)?
                    #[allow(unused)]
                    let mut staged_vector_indexes = std::collections::BTreeMap::new();
                    $($(
                        let index_name = $crate::index::test_helpers::new_index_name(
                            $table,
                            $staged_vector_index_name,
                        )?;
                        staged_vector_indexes.insert(
                            index_name.descriptor().clone(),
                            $crate::schemas::VectorIndexSchema::new(
                                index_name.descriptor().clone(),
                                $staged_vector_index_field.parse()?,
                                1536u32.try_into()?,
                                Default::default(),
                            )?,
                        );
                    )*)?
                    #[allow(unused)]
                    let mut document_type = None;
                    $(
                        document_type = Some($document_schema);
                    )?
                    let table_def = $crate::schemas::TableDefinition {
                        table_name: table_name.clone(),
                        indexes,
                        staged_db_indexes,
                        text_indexes,
                        staged_text_indexes,
                        vector_indexes,
                        staged_vector_indexes,
                        document_type,
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
    ($($table:expr => [$(($index_name:expr, $fields:expr)),*]),* $(,)?) => {
         db_schema_with_indexes!($($table => {
            indexes: $(($index_name, $fields)),*
        }),*)
    };
}

#[test]
fn test_db_schema_with_indexes() -> anyhow::Result<()> {
    // Test with only indexes (array syntax)
    db_schema_with_indexes!("table" => [("indexname", vec!["a"])]);

    // Test with only indexes (new syntax)
    db_schema_with_indexes!("table" => {
        indexes: ("indexname", vec!["a", "b", "c"])
    });

    // Test with only text_indexes
    db_schema_with_indexes!("table" => {
        text_indexes: ("text_indexname", "a")
    });

    // Test with both indexes and text_indexes
    db_schema_with_indexes!("table" => {
        indexes: ("indexname", vec!["a"]),
        text_indexes: ("text_indexname", "a")
    });

    // Test with all index types
    db_schema_with_indexes!("table" => {
        indexes: ("indexname", vec!["a"]),
        staged_db_indexes: ("staged_db_indexname", vec!["a"]),
        text_indexes: ("text_indexname", "a"),
        staged_text_indexes: ("staged_text_indexname", "a"),
        vector_indexes: ("vector_indexname", "a"),
        staged_vector_indexes: ("staged_vector_indexname", "a"),
    });

    // Test with no indexes (empty table)
    db_schema_with_indexes!("table" => {});

    Ok(())
}

#[macro_export]
macro_rules! db_schema_with_search_indexes {
    ($($table:expr => [$(($index_name:expr, $field:expr)),*]),* $(,)?) => {
         $crate::db_schema_with_indexes!($($table => {
            text_indexes: $(($index_name, $field)),*
        }),*)
    };
}

#[test]
fn test_db_schema_with_text_indexes() -> anyhow::Result<()> {
    db_schema_with_search_indexes!("table" => [("indexname", "a")]);
    Ok(())
}

#[macro_export]
// Turns a mapping of tableName => (index_name, vector_field) into a
// DatabaseSchema struct.
macro_rules! db_schema_with_vector_indexes {
    ($($table:expr => {
        $document_schema:expr, [$(($index_name:expr, $vector_field:expr)),*]
    }),* $(,)?) => {
         $crate::db_schema_with_indexes!($($table => {
            vector_indexes: $(($index_name, $vector_field)),*,
            document_schema: $document_schema,
        }),*)
    };
}

#[test]
fn test_db_schema_with_vector_indexes() -> anyhow::Result<()> {
    let document_schema = crate::schemas::DocumentSchema::Any;
    db_schema_with_vector_indexes!(
        "table" => {document_schema, [("myVectorIndex", "myField")]}
    );
    Ok(())
}
