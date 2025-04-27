use std::sync::LazyLock;

use errors::ErrorMetadata;
use itertools::Itertools;
use serde_json::{
    json,
    Value as JsonValue,
};

use crate::{
    bootstrap_model::index::{
        MAX_INDEX_FIELDS_SIZE,
        MAX_TEXT_INDEX_FILTER_FIELDS_SIZE,
        MAX_VECTOR_INDEX_FILTER_FIELDS_SIZE,
    },
    json::JsonSerializable,
    schemas::{
        DatabaseSchema,
        MAX_INDEXES_PER_TABLE,
    },
};

static TOO_MANY_INDEXES: LazyLock<Vec<JsonValue>> = LazyLock::new(|| {
    (0..MAX_INDEXES_PER_TABLE + 1)
        .map(|i| {
            json!({
                "indexDescriptor": format!("index{i}"),
                "fields": ["x"],
            })
        })
        .collect()
});

static TOO_MANY_INDEX_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    (0..MAX_INDEX_FIELDS_SIZE + 1)
        .map(|i| format!("field_{}", i))
        .collect()
});

fn index_validation_test(schema_value: JsonValue) -> ErrorMetadata {
    let e = DatabaseSchema::json_deserialize_value(schema_value)
        .expect_err("Successfully created invalid schema");
    e.downcast::<ErrorMetadata>()
        .unwrap_or_else(|e| panic!("Error <{e}> is not an ErrorMetadata"))
}

#[test]
fn test_empty_index() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [
                    {
                        "indexDescriptor": "by_email",
                        "fields": [],
                    }
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "EmptyIndex",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_index_fields_not_unique() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [
                    {
                        "indexDescriptor": "by_email",
                        "fields": ["email", "email"],
                    },
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "FieldsNotUniqueWithinIndex",
        "<{err}> does not match expected error type"
    );
    // Test the full string since there's some complex interpolation involved
    assert_eq!(
        err.msg,
        "In table \"test\": In index \"by_email\": Duplicate field \"email\". Index fields must \
         be unique within an index.",
        "<{err}> does not match expected error string"
    );
}

#[test]
fn test_search_index_fields_not_unique() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "searchIndexes": [
                    {
                        "indexDescriptor": "firstIndex",
                        "searchField": "text",
                        "filterFields": []
                    },
                    {
                        "indexDescriptor": "secondIndex",
                        "searchField": "text",
                        "filterFields": []
                    }
                ],

            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "SearchIndexFieldNotUnique",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_search_index_fields_not_unique_but_filter_fields_are_unique_fails() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "searchIndexes": [
                    {
                        "indexDescriptor": "firstIndex",
                        "searchField": "text",
                        "filterFields": ["first"]
                    },
                    {
                        "indexDescriptor": "secondIndex",
                        "searchField": "text",
                        "filterFields": ["second"]
                    }
                ],

            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "SearchIndexFieldNotUnique",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_vector_indexes_not_unique() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "vectorIndexes": [
                    {
                        "indexDescriptor": "firstIndex",
                        "vectorField": "fieldName",
                        "filterFields": [],
                        "dimensions": 1536,
                    },
                    {
                        "indexDescriptor": "secondIndex",
                        "vectorField": "fieldName",
                        "filterFields": [],
                        "dimensions": 1536,
                    }
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "VectorIndexFieldNotUnique",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_vector_fields_and_dimensions_not_unique_but_filter_fields_are_unique_fails() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "vectorIndexes": [
                    {
                        "indexDescriptor": "firstIndex",
                        "vectorField": "fieldName",
                        "filterFields": ["first"],
                        "dimensions": 1536,
                    },
                    {
                        "indexDescriptor": "secondIndex",
                        "vectorField": "fieldName",
                        "filterFields": ["second"],
                        "dimensions": 1536,
                    }
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "VectorIndexFieldNotUnique",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_vector_indexes_same_fields_different_dimensions_are_valid() -> anyhow::Result<()> {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "vectorIndexes": [
                    {
                        "indexDescriptor": "firstIndex",
                        "vectorField": "fieldName",
                        "filterFields": [],
                        "dimensions": 1536,
                    },
                    {
                        "indexDescriptor": "secondIndex",
                        "vectorField": "fieldName",
                        "filterFields": [],
                        "dimensions": 3,
                    }
                ],
            },
        ],
        "schemaValidation": true,
    });
    DatabaseSchema::json_deserialize_value(value)?;
    Ok(())
}

#[test]
fn test_index_fields_contain_id() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [
                    {
                        "indexDescriptor": "my_index",
                        "fields": ["_id"],
                    },
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "IndexFieldsContainId",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_index_paths_not_unique() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [
                    {
                        "indexDescriptor": "by_email",
                        "fields": ["email"],
                    },
                    {
                        "indexDescriptor": "by_email2",
                        "fields": ["email"],
                    }
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "IndexNotUnique",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_index_names_reserved() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [
                    {
                        "indexDescriptor": "by_id",
                        "fields": ["id"],
                    }
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "IndexNameReserved",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_search_index_name_reserved() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "searchIndexes": [
                    {
                        "indexDescriptor": "by_id",
                        "searchField": "text",
                        "filterFields": []
                    }
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "IndexNameReserved",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_index_names_not_unique() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [
                    {
                        "indexDescriptor": "by_email",
                        "fields": ["email"],
                    },
                    {
                        "indexDescriptor": "by_email",
                        "fields": ["email2"],
                    }
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "IndexNamesNotUnique",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_search_index_names_not_unique() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "searchIndexes": [
                    {
                        "indexDescriptor": "index_name",
                        "searchField": "field1",
                        "filterFields": []
                    },
                    {
                        "indexDescriptor": "index_name",
                        "searchField": "field2",
                        "filterFields": []
                    }
                ],

            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "IndexNamesNotUnique",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_vector_index_names_not_unique() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "vectorIndexes": [
                    {
                        "indexDescriptor": "index_name",
                        "vectorField": "fieldName",
                        "filterFields": [],
                        "dimensions": 1536,
                    },
                    {
                        "indexDescriptor": "index_name",
                        "vectorField": "fieldName2",
                        "filterFields": [],
                        "dimensions": 1536,
                    },
                ],

            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "IndexNamesNotUnique",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_reused_index_name_between_database_and_search_indexes() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [
                    {
                        "indexDescriptor": "index_name",
                        "fields": ["field1"],
                    }
                ],
                "searchIndexes": [
                    {
                        "indexDescriptor": "index_name",
                        "searchField": "field1",
                        "filterFields": []
                    },
                ],

            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "IndexNamesNotUnique",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_reused_index_name_between_database_and_vector_indexes() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [
                    {
                        "indexDescriptor": "index_name",
                        "fields": ["field1"],
                    }
                ],
                "vectorIndexes": [
                    {
                        "indexDescriptor": "index_name",
                        "vectorField": "fieldName",
                        "filterFields": [],
                        "dimensions": 1536,
                    }
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "IndexNamesNotUnique",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_reused_index_name_between_search_and_vector_indexes() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "searchIndexes": [
                    {
                        "indexDescriptor": "index_name",
                        "searchField": "field1",
                        "filterFields": []
                    },
                ],
                "vectorIndexes": [
                    {
                        "indexDescriptor": "index_name",
                        "vectorField": "fieldName",
                        "filterFields": [],
                        "dimensions": 1536,
                    }
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "IndexNamesNotUnique",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_invalid_index_descriptor() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [
                    {
                        "indexDescriptor": "_",
                        "fields": ["email"],
                    }
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "InvalidIndexName",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_invalid_search_index_descriptor() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "searchIndexes": [
                    {
                        "indexDescriptor": "_",
                        "searchField": "field1",
                        "filterFields": []
                    },
                ],

            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "InvalidIndexName",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_invalid_vector_index_descriptor() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "vectorIndexes": [
                    {
                        "indexDescriptor": "_",
                        "vectorField": "fieldName",
                        "filterFields": [],
                        "dimensions": 1536,
                    }
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "InvalidIndexName",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_invalid_field_name() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [
                    {
                        "indexDescriptor": "by_invalid",
                        "fields": ["_"],
                    }
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "InvalidIndexField",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_invalid_search_field_name() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "searchIndexes": [
                    {
                        "indexDescriptor": "index_name",
                        "searchField": "_",
                        "filterFields": []
                    },
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "InvalidIndexField",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_invalid_search_filter_field_name() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "searchIndexes": [
                    {
                        "indexDescriptor": "index_name",
                        "searchField": "field",
                        "filterFields": ["_"]
                    },
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "InvalidIndexField",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_invalid_vector_field_name() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "vectorIndexes": [
                    {
                        "indexDescriptor": "index_name",
                        "vectorField": "_",
                        "filterFields": [],
                        "dimensions": 1536,
                    }
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "InvalidIndexField",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_invalid_vector_filter_field_name() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "vectorIndexes": [
                    {
                        "indexDescriptor": "index_name",
                        "vectorField": "fieldName",
                        "filterFields": ["_"],
                        "dimensions": 1536,
                    }
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "InvalidIndexField",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_invalid_json() {
    let value = json!({
        "tables": "blah"
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "InvalidJson",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_invalid_table_name() {
    let value = json!({
        "tables": [
            {
                "tableName": "_",
                "indexes": [],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "InvalidTableName",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_too_many_fields() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [
                    {
                        "indexDescriptor": "by_toomany",
                        "fields": *TOO_MANY_INDEX_FIELDS,
                    },
                ],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "IndexTooManyFields",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_too_many_search_filter_fields() {
    let fields: Vec<_> = (0..MAX_TEXT_INDEX_FILTER_FIELDS_SIZE + 1)
        .map(|i| format!("field_{}", i))
        .collect();
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "searchIndexes": [{
                    "indexDescriptor": "search_index",
                    "searchField": "fieldName",
                    "filterFields": fields
                }]
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "IndexTooManyFilterFields",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_too_many_indexes() {
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": *TOO_MANY_INDEXES,
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "TooManyIndexes",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_too_many_indexes_across_types() -> anyhow::Result<()> {
    let num_indexes_per_type = (MAX_INDEXES_PER_TABLE / 3) + 1;
    let indexes: Vec<JsonValue> = (0..num_indexes_per_type)
        .map(|i| {
            json!({
                "indexDescriptor": format!("index{i}"),
                "fields": [format!("field{i}")]
            })
        })
        .collect_vec();
    let text_indexes = (0..num_indexes_per_type)
        .map(|i| {
            json!({
                "indexDescriptor": format!("search_index{}", i),
                "searchField": format!("fieldName{}", i),
                "filterFields": [],
            })
        })
        .collect_vec();
    let vector_indexes = (0..num_indexes_per_type)
        .map(|i| {
            json!({
                "indexDescriptor": format!("vector_index{}", i),
                "vectorField": format!("fieldName{}", i),
                "filterFields": [],
                "dimensions": 1536,
            })
        })
        .collect_vec();
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": indexes,
                "searchIndexes": text_indexes,
                "vectorIndexes": vector_indexes,
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "TooManyIndexes",
        "<{err}> does not match expected error type"
    );
    Ok(())
}

#[test]
fn test_many_search_indexes() -> anyhow::Result<()> {
    let indexes: Vec<_> = (0..7)
        .map(|i| {
            json!({
                "indexDescriptor": format!("index{}", i),
                "searchField": format!("fieldName{}", i),
                "filterFields": [],
            })
        })
        .collect();
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "searchIndexes": indexes,
            },
        ],
        "schemaValidation": true,
    });
    DatabaseSchema::json_deserialize_value(value)?;
    Ok(())
}

#[test]
fn test_many_vector_indexes() -> anyhow::Result<()> {
    let indexes: Vec<_> = (0..5)
        .map(|i| {
            json!({
                "indexDescriptor": format!("index{}", i),
                "vectorField": format!("fieldName{}", i),
                "filterFields": [],
                "dimensions": 1536,
            })
        })
        .collect();
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "vectorIndexes": indexes,
            },
        ],
        "schemaValidation": true,
    });
    DatabaseSchema::json_deserialize_value(value)?;
    Ok(())
}

#[test]
fn test_too_many_vector_filter_fields() {
    let fields: Vec<_> = (0..MAX_VECTOR_INDEX_FILTER_FIELDS_SIZE + 1)
        .map(|i| format!("field_{}", i))
        .collect();
    let value = json!({
        "tables": [
            {
                "tableName": "test",
                "indexes": [],
                "vectorIndexes": [{
                    "indexDescriptor": "indexName",
                    "vectorField": "fieldName",
                    "filterFields": fields,
                    "dimensions": 1536,
                }]
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "IndexTooManyFilterFields",
        "<{err}> does not match expected error type"
    );
}

#[test]
fn test_table_name_reserved() {
    let value = json!({
        "tables": [
            {
                "tableName": "_reserved",
                "indexes": [],
            },
        ],
        "schemaValidation": true,
    });
    let err = index_validation_test(value);
    assert_eq!(
        err.short_msg, "TableNameReserved",
        "<{err}> does not match expected error type"
    );
}
