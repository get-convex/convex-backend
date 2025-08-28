use cmd_util::env::env_config;
use proptest::prelude::*;
use serde_json::json;
use value::{
    assert_obj,
    ConvexObject,
    FieldName,
    NamespacedTableMapping,
    TableMapping,
    TableNamespace,
};

use crate::{
    db_schema_with_indexes,
    json::JsonSerializable,
    object_validator,
    schemas::{
        json::DatabaseSchemaJson,
        validator::{
            FieldValidator,
            ValidationContext,
            ValidationError,
        },
        DatabaseSchema,
        DocumentSchema,
        Validator,
    },
    testing::assert_roundtrips,
    virtual_system_mapping::VirtualSystemMapping,
};

proptest! {
    #![proptest_config(ProptestConfig { cases: 64 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]
    #[test]
    fn test_database_schema_roundtrips(v in any::<DatabaseSchema>()) {
        assert_roundtrips::<DatabaseSchema, DatabaseSchemaJson>(v);
    }

    #[test]
    fn test_any_matches_all(v in any::<ConvexObject>()) {
        let document_schema = DocumentSchema::Any;
        document_schema.check_value(
            &v,
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
        ).unwrap();
    }
}

#[test]
fn test_document_schema_no_match() -> anyhow::Result<()> {
    let object_validator = object_validator!("name" => FieldValidator::required_field_type(Validator::String), "age" => FieldValidator::required_field_type(Validator::Int64));
    let document_schema = DocumentSchema::Union(vec![object_validator]);
    let object = assert_obj!("name" => "emma", "age" => "24");
    let err = document_schema
        .check_value(
            &object,
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
        )
        .unwrap_err();
    assert!(matches!(
        err,
        ValidationError::NoMatch {
            value: _,
            validator: _,
            context: _
        }
    ));
    let value = assert_obj!("name" => "emma", "age" => 24);
    document_schema
        .check_value(
            &value,
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
        )
        .unwrap();
    Ok(())
}

#[test]
fn test_document_schema_missing_required_field() -> anyhow::Result<()> {
    let object_validator = object_validator!("name" => FieldValidator::required_field_type(Validator::String), "age" => FieldValidator::required_field_type(Validator::Int64));
    let document_schema = DocumentSchema::Union(vec![object_validator.clone()]);
    let object = assert_obj!("name" => "emma");
    let err = document_schema
        .check_value(
            &object,
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
        )
        .unwrap_err();
    assert_eq!(
        err,
        ValidationError::MissingRequiredField {
            object: object.clone(),
            field_name: "age".parse()?,
            object_validator,
            context: ValidationContext::new()
        }
    );

    let object_validator = object_validator!("name" => FieldValidator::required_field_type(Validator::String), "age" => FieldValidator::optional_field_type(Validator::Int64));
    let document_schema = DocumentSchema::Union(vec![object_validator]);
    document_schema
        .check_value(
            &object,
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
        )
        .unwrap();

    Ok(())
}

#[test]
fn test_document_schema_extra_field() -> anyhow::Result<()> {
    let object_validator =
        object_validator!("name" => FieldValidator::required_field_type(Validator::String));
    let document_schema = DocumentSchema::Union(vec![object_validator.clone()]);
    let non_existent_field: FieldName = "field".parse()?;
    let object = assert_obj!("name" => "emma", "field" => "extra stuff");
    let err = document_schema
        .check_value(
            &object,
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
        )
        .unwrap_err();
    assert_eq!(
        err,
        ValidationError::ExtraField {
            object,
            field_name: non_existent_field,
            object_validator,
            context: ValidationContext::new()
        }
    );
    Ok(())
}

#[test]
fn test_nonexistent_table_name_reference() {
    // This schema has an ID that references "otherTable" but never defines
    // "otherTable".
    let schema_json = json!({
        "tables": [
            {
                "tableName": "testTable",
                "documentType": {
                    "type": "object",
                    "value": {
                        "property": {
                            "fieldType": {
                                "type": "id",
                                "tableName": "otherTable"
                            },
                            "optional": false
                        },
                    }
                },
                "indexes": [],
                "searchIndexes": []
            },
        ],
        "schemaValidation": true
    });
    DatabaseSchema::json_deserialize_value(schema_json).unwrap();
}

#[test]
fn test_nested_optional_float64_vector_index_field_succeeds() -> anyhow::Result<()> {
    let document_schema = DocumentSchema::Union(vec![object_validator!(
        "myNestedObject" => FieldValidator::optional_field_type(
            Validator::Object(
                object_validator!(
                    "nestedField" => FieldValidator::optional_field_type(
                        Validator::Array(Box::new(Validator::Float64))
                    )
                )
            )
        )
    )]);
    let db_schema = db_schema_with_indexes!(
        "table" => {
            vector_indexes: ("myVectorIndex", "myNestedObject.nestedField"),
            document_schema: document_schema
        }
    );
    db_schema.check_index_references()?;
    Ok(())
}

#[test]
fn test_nested_union_vector_eligible_types_succeeds() -> anyhow::Result<()> {
    let document_schema = DocumentSchema::Union(vec![object_validator!(
        "myNestedObject" => FieldValidator::optional_field_type(
            Validator::Object(
                object_validator!(
                    "nestedField" => FieldValidator::optional_field_type(
                        Validator::Union(vec![
                            Validator::Array(Box::new(Validator::Float64)),
                            Validator::Any,
                            Validator::Null,
                        ])
                    )
                )
            )
        )
    )]);
    let db_schema = db_schema_with_indexes!(
        "table" => {
            vector_indexes: ("myVectorIndex", "myNestedObject.nestedField"),
            document_schema: document_schema
        }
    );
    db_schema.check_index_references()?;
    Ok(())
}

#[test]
fn test_nested_union_vector_and_int_index_field_succeeds() -> anyhow::Result<()> {
    let document_schema = DocumentSchema::Union(vec![object_validator!(
        "myNestedObject" => FieldValidator::optional_field_type(
            Validator::Object(
                object_validator!(
                    "nestedField" => FieldValidator::optional_field_type(
                        Validator::Union(vec![
                            Validator::Array(Box::new(Validator::Float64)),
                            Validator::Int64,
                        ])
                    )
                )
            )
        )
    )]);
    let db_schema = db_schema_with_indexes!(
        "table" => {
            vector_indexes: ("myVectorIndex", "myNestedObject.nestedField"),
            document_schema: document_schema
        }
    );
    db_schema.check_index_references()?;
    Ok(())
}

#[test]
fn test_union_of_objects_only_one_has_null_field_succeeds() -> anyhow::Result<()> {
    let document_schema = DocumentSchema::Union(vec![
        object_validator!(
            "vector" => FieldValidator::optional_field_type(Validator::Null)
        ),
        object_validator!(
            "other" => FieldValidator::optional_field_type(
                Validator::String,
            )
        ),
    ]);
    let db_schema = db_schema_with_indexes!(
        "table" => {
            vector_indexes: ("myVectorIndex", "vector"),
            document_schema: document_schema
        }
    );
    db_schema.check_index_references()?;
    Ok(())
}

#[test]
fn test_union_of_objects_only_one_has_vector_field_succeeds() -> anyhow::Result<()> {
    let document_schema = DocumentSchema::Union(vec![
        object_validator!(
            "vector" => FieldValidator::optional_field_type(
                Validator::Array(Box::new(Validator::Float64))
            )
        ),
        object_validator!(
            "other" => FieldValidator::optional_field_type(
                Validator::String,
            )
        ),
    ]);
    let db_schema = db_schema_with_indexes!(
        "table" => {
            vector_indexes: ("myVectorIndex", "vector"),
            document_schema: document_schema
        }
    );
    db_schema.check_index_references()?;
    Ok(())
}

#[test]
fn test_nested_union_of_objects_only_one_has_vector_field_succeeds() -> anyhow::Result<()> {
    let document_schema = DocumentSchema::Union(vec![object_validator!(
        "object" => FieldValidator::optional_field_type(
            Validator::Union(vec![
                Validator::Object(
                    object_validator!(
                        "vector" => FieldValidator::optional_field_type(
                            Validator::Array(Box::new(Validator::Float64))
                        )
                    )
                ),
                Validator::Object(
                    object_validator!(
                        "other" => FieldValidator::optional_field_type(
                            Validator::String,
                        )
                    )
                )
            ])
        )
    )]);
    let db_schema = db_schema_with_indexes!(
        "table" => {
            vector_indexes: ("myVectorIndex", "object.vector"),
            document_schema: document_schema
        }
    );
    db_schema.check_index_references()?;
    Ok(())
}

#[test]
fn test_nested_union_of_objects_both_have_field_only_one_is_vector_succeeds() -> anyhow::Result<()>
{
    let document_schema = DocumentSchema::Union(vec![object_validator!(
        "object" => FieldValidator::optional_field_type(
            Validator::Union(vec![
                Validator::Object(
                    object_validator!(
                        "vector" => FieldValidator::optional_field_type(
                            Validator::Array(Box::new(Validator::Float64))
                        )
                    )
                ),
                Validator::Object(
                    object_validator!(
                        "vector" => FieldValidator::optional_field_type(
                            Validator::String,
                        )
                    )
                )
            ])
        )
    )]);
    let db_schema = db_schema_with_indexes!(
        "table" => {
            vector_indexes: ("myVectorIndex", "object.vector"),
            document_schema: document_schema
        }
    );
    db_schema.check_index_references()?;
    Ok(())
}

#[test]
fn test_nested_union_of_objects_both_have_field_neither_is_vector_fails() -> anyhow::Result<()> {
    let document_schema = DocumentSchema::Union(vec![object_validator!(
        "object" => FieldValidator::optional_field_type(
            Validator::Union(vec![
                Validator::Object(
                    object_validator!(
                        "vector" => FieldValidator::optional_field_type(
                            Validator::Int64,
                        )
                    )
                ),
                Validator::Object(
                    object_validator!(
                        "vector" => FieldValidator::optional_field_type(
                            Validator::String,
                        )
                    )
                )
            ])
        )
    )]);
    let db_schema = db_schema_with_indexes!(
        "table" => {
            vector_indexes: ("myVectorIndex", "object.vector"),
            document_schema: document_schema
        }
    );
    expect_invalid_vector_error(db_schema);
    Ok(())
}

#[test]
fn test_optional_vector_index_field_succeeds() -> anyhow::Result<()> {
    let document_schema = DocumentSchema::Union(vec![object_validator!(
        "myField" =>
            FieldValidator::optional_field_type(
                Validator::Array(Box::new(Validator::Float64))
            )
    )]);
    let db_schema = db_schema_with_indexes!(
        "table" => {
            vector_indexes: ("myVectorIndex", "myField"),
            document_schema: document_schema
        }
    );
    db_schema.check_index_references()?;
    Ok(())
}

#[test]
fn test_float64_vector_index_field_succeeds() -> anyhow::Result<()> {
    let document_schema = DocumentSchema::Union(vec![object_validator!(
        "myField" =>
            FieldValidator::required_field_type(
                Validator::Array(Box::new(Validator::Float64))
            )
    )]);
    let db_schema = db_schema_with_indexes!(
        "table" => {
            vector_indexes: ("myVectorIndex", "myField"),
            document_schema: document_schema
        }
    );
    db_schema.check_index_references()?;
    Ok(())
}

#[test]
fn test_any_vector_index_field_succeeds() -> anyhow::Result<()> {
    let document_schema = DocumentSchema::Union(vec![object_validator!(
        "myField" =>
            FieldValidator::required_field_type(Validator::Any)
    )]);
    let db_schema = db_schema_with_indexes!(
        "table" => {
            vector_indexes: ("myVectorIndex", "myField"),
            document_schema: document_schema
        }
    );
    db_schema.check_index_references()?;
    Ok(())
}

#[test]
fn test_null_vector_index_field_fails() -> anyhow::Result<()> {
    let document_schema = DocumentSchema::Union(vec![object_validator!(
        "myField" =>
            FieldValidator::required_field_type(Validator::Null)
    )]);
    let db_schema = db_schema_with_indexes!(
        "table" => {
            vector_indexes: ("myVectorIndex", "myField"),
            document_schema: document_schema
        }
    );
    expect_invalid_vector_error(db_schema);
    Ok(())
}

#[test]
fn test_string_vector_index_field_fails() -> anyhow::Result<()> {
    let document_schema = DocumentSchema::Union(vec![object_validator!(
        "myField" =>
            FieldValidator::required_field_type(Validator::String)
    )]);
    let db_schema = db_schema_with_indexes!(
        "table" => {
            vector_indexes: ("myVectorIndex", "myField"),
            document_schema: document_schema
        }
    );
    expect_invalid_vector_error(db_schema);
    Ok(())
}

#[test]
fn test_union_same_field_name_both_not_vector_fields_fails() -> anyhow::Result<()> {
    let document_schema = DocumentSchema::Union(vec![
        object_validator!(
            "myField" =>
                FieldValidator::required_field_type(Validator::String)
        ),
        object_validator!(
            "myField" =>
                FieldValidator::required_field_type(Validator::Float64)
        ),
    ]);
    let db_schema = db_schema_with_indexes!(
        "table" => {
            vector_indexes: ("myVectorIndex", "myField"),
            document_schema: document_schema
        }
    );
    expect_invalid_vector_error(db_schema);
    Ok(())
}

fn expect_invalid_vector_error(db_schema: DatabaseSchema) {
    let error = db_schema
        .check_index_references()
        .expect_err("Validated invalid index!");
    assert!(error
        .to_string()
        .contains("that is neither an array of float64 or optional array of float64"));
}

#[test]
fn test_union_same_field_name_both_vector_fields_succeeds() -> anyhow::Result<()> {
    let document_schema = DocumentSchema::Union(vec![
        object_validator!(
            "myField" =>
                FieldValidator::optional_field_type(
                    Validator::Array(Box::new(Validator::Float64))
                )
        ),
        object_validator!(
            "myField" =>
                FieldValidator::required_field_type(
                    Validator::Array(Box::new(Validator::Float64))
                )
        ),
    ]);
    let db_schema = db_schema_with_indexes!(
        "table" => {
            vector_indexes: ("myVectorIndex", "myField"),
            document_schema: document_schema
        }
    );
    db_schema.check_index_references()?;
    Ok(())
}

#[test]
fn test_union_same_field_any_and_vector_index_field_succeeds() -> anyhow::Result<()> {
    let document_schema = DocumentSchema::Union(vec![
        object_validator!(
            "myField" =>
                FieldValidator::optional_field_type(Validator::Any)
        ),
        object_validator!(
            "myField" =>
                FieldValidator::required_field_type(
                    Validator::Array(Box::new(Validator::Float64))
                )
        ),
    ]);
    let db_schema = db_schema_with_indexes!(
        "table" => {
            vector_indexes: ("myVectorIndex", "myField"),
            document_schema: document_schema
        }
    );
    db_schema.check_index_references()?;
    Ok(())
}

#[test]
fn test_union_same_field_one_vector_field_succeeds() -> anyhow::Result<()> {
    let document_schema = DocumentSchema::Union(vec![
        object_validator!(
            "myField" =>
                FieldValidator::optional_field_type(
                    Validator::Array(Box::new(Validator::Float64))
                )
        ),
        object_validator!(
            "myField" =>
                FieldValidator::required_field_type(
                    Validator::Float64
                )
        ),
    ]);
    let db_schema = db_schema_with_indexes!(
        "table" => {
            vector_indexes: ("myVectorIndex", "myField"),
            document_schema: document_schema
        }
    );
    db_schema.check_index_references()?;
    Ok(())
}

#[test]
fn test_union_same_field_one_vector_one_null_field_succeeds() -> anyhow::Result<()> {
    let document_schema = DocumentSchema::Union(vec![
        object_validator!(
            "myField" =>
                FieldValidator::optional_field_type(
                    Validator::Array(Box::new(Validator::Float64))
                )
        ),
        object_validator!(
            "myField" =>
                FieldValidator::required_field_type(
                    Validator::Null
                )
        ),
    ]);
    let db_schema = db_schema_with_indexes!(
        "table" => {
            vector_indexes: ("myVectorIndex", "myField"),
            document_schema: document_schema
        }
    );
    db_schema.check_index_references()?;
    Ok(())
}

#[test]
fn test_invalid_field_name() {
    // This schema has a field with an invalid name ("123 myField")
    let schema_json = json!({
        "tables": [
            {
                "tableName": "testTable",
                "documentType": {
                    "type": "object",
                    "value": {
                        "123 myField": {
                            "fieldType": {
                                "type": "string",
                            },
                            "optional": false
                        },
                    }
                },
                "indexes": [],
                "searchIndexes": []
            },
        ],
        "schemaValidation": true
    });
    let error = DatabaseSchema::json_deserialize_value(schema_json)
        .expect_err("Successfully created invalid schema");
    assert!(error.to_string().contains("Identifiers must start with"));

    // This schema has a nested field with an invalid name ("123 nested")
    let schema_json = json!({
        "tables": [
            {
                "tableName": "testTable",
                "documentType": {
                    "type": "object",
                    "value": {
                        "myField": {
                            "fieldType": {
                                "type": "object",
                                "value": {
                                    "123 nested": {
                                        "fieldType": {
                                            "type": "string",
                                        },
                                        "optional": false
                                    }
                                }
                            },
                            "optional": false
                        },
                    }
                },
                "indexes": [],
                "searchIndexes": []
            },
        ],
        "schemaValidation": true
    });
    let error = DatabaseSchema::json_deserialize_value(schema_json)
        .expect_err("Successfully created invalid schema");
    assert!(error.to_string().contains("Identifiers must start with"));
}

#[test]
fn test_json_backwards_compatibility() -> anyhow::Result<()> {
    // JSON from the npm package <= 0.13.0 didn't include the `schemaValidation`
    // or `documentType` keys. Test that this still works.
    let schema_json = json!({
        "tables": [
            {
                "tableName": "testTable",
                "indexes": [],
                "searchIndexes": []
            },
        ],
    });
    let schema = DatabaseSchema::json_deserialize_value(schema_json)?;
    assert!(!schema.schema_validation);
    Ok(())
}

fn empty_table_mapping() -> NamespacedTableMapping {
    TableMapping::new().namespace(TableNamespace::test_user())
}

mod tables_to_revalidate {
    use std::str::FromStr;

    use maplit::btreeset;
    use shape_inference::{
        testing::TestConfig,
        CountedShape,
    };
    use value::{
        assert_obj,
        assert_val,
        id_v6::DeveloperDocumentId,
        ConvexValue,
        ResolvedDocumentId,
        TableName,
        TableNamespace,
    };

    use crate::{
        db_schema,
        db_schema_not_validated,
        object_validator,
        schemas::{
            tests::empty_table_mapping,
            validator::{
                FieldValidator,
                LiteralValidator,
                Validator,
            },
            DatabaseSchema,
            DocumentSchema,
        },
        testing::TestIdGenerator,
        virtual_system_mapping::VirtualSystemMapping,
    };

    #[test]
    fn test_should_always_return_empty_if_schema_validation_is_disabled() -> anyhow::Result<()> {
        let document_with_int = ConvexValue::Object(assert_obj!("field" => 42));
        let shape = CountedShape::<TestConfig>::empty().insert_value(&document_with_int);

        // No schema → schema with validation disabled
        let schema_validation_disabled = db_schema_not_validated!(
            "table1" => DocumentSchema::Union(vec![object_validator!("field" => FieldValidator::required_field_type(Validator::Int64))]));
        let tables_to_validate = DatabaseSchema::tables_to_validate(
            &schema_validation_disabled,
            None,
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
            &|_name| Some(shape.clone()),
        )?;
        assert!(tables_to_validate.is_empty());

        // Validation enabled → different schema with validation disabled
        let schema_validation_enabled = db_schema!(
        "table2" => DocumentSchema::Union(vec![object_validator!("field" => FieldValidator::required_field_type(Validator::String))]));
        let tables_to_validate = DatabaseSchema::tables_to_validate(
            &schema_validation_disabled,
            Some(&schema_validation_enabled),
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
            &|_name| Some(shape.clone()),
        )?;
        assert!(tables_to_validate.is_empty());
        Ok(())
    }

    fn document_with_literal() -> anyhow::Result<DocumentSchema> {
        Ok(DocumentSchema::Union(vec![
            object_validator!("field" => FieldValidator::required_field_type(Validator::Literal(LiteralValidator::String("a".try_into()?)))),
        ]))
    }

    #[test]
    fn test_should_not_validate_any_table_that_has_a_schema_of_type_any() -> anyhow::Result<()> {
        let document_with_string = ConvexValue::Object(assert_obj!("field" => "value"));
        let shape = CountedShape::<TestConfig>::empty().insert_value(&document_with_string);

        let db_schema = db_schema!(
        "table1" => DocumentSchema::Any,
        "table2" => document_with_literal()?);
        let tables_to_validate = DatabaseSchema::tables_to_validate(
            &db_schema,
            None,
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
            &|_name| Some(shape.clone()),
        )?;
        let table_2 = "table2".parse()?;
        assert_eq!(tables_to_validate, btreeset! {&table_2});
        Ok(())
    }

    #[test]
    fn test_should_not_validate_tables_that_have_the_same_schema_as_the_active_schema(
    ) -> anyhow::Result<()> {
        let document_with_string = ConvexValue::Object(assert_obj!("field" => "value"));
        let shape = CountedShape::<TestConfig>::empty().insert_value(&document_with_string);

        let db_schema = db_schema!(
        "table1" => DocumentSchema::Any,
        "table2" => document_with_literal()?);
        let superset_db_schema = db_schema!(
        "table1" => DocumentSchema::Any,
        "table2" => document_with_literal()?,
        "table3" => document_with_literal()?);
        let tables_to_validate = DatabaseSchema::tables_to_validate(
            &superset_db_schema,
            Some(&db_schema),
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
            &|_name| Some(shape.clone()),
        )?;
        let table_3 = "table3".parse()?;
        assert_eq!(tables_to_validate, btreeset! {&table_3});
        Ok(())
    }

    #[test]
    fn test_table_must_be_validated_when_schema_starts_to_be_enforced() -> anyhow::Result<()> {
        let document_with_string = ConvexValue::Object(assert_obj!("field" => "value"));
        let shape = CountedShape::<TestConfig>::empty().insert_value(&document_with_string);

        let db_schema_unenforced = db_schema_not_validated!("table" => document_with_literal()?);
        let db_schema_enforced = db_schema!("table" => document_with_literal()?);

        let tables_to_validate = DatabaseSchema::tables_to_validate(
            &db_schema_enforced,
            Some(&db_schema_unenforced),
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
            &|_name| Some(shape.clone()),
        )?;
        let table_name = TableName::from_str("table")?;
        assert_eq!(tables_to_validate, btreeset! {&table_name});

        Ok(())
    }

    fn one_field_schema(validator: Validator) -> anyhow::Result<DatabaseSchema> {
        Ok(db_schema!(
            "table" => DocumentSchema::Union(vec![object_validator!(
                "field" => FieldValidator::required_field_type(
                    validator,
                )
            )])
        ))
    }

    fn literals_validator(values: Vec<&str>) -> anyhow::Result<Validator> {
        let strings: Vec<value::ConvexString> =
            values.into_iter().map(|str| str.try_into()).try_collect()?;
        Ok(Validator::Union(
            strings
                .into_iter()
                .map(|str| Validator::Literal(LiteralValidator::String(str)))
                .collect(),
        ))
    }

    #[test]
    fn test_skips_validation_if_schema_is_subset() -> anyhow::Result<()> {
        let document_with_string = ConvexValue::Object(assert_obj!("field" => "value"));
        let shape = CountedShape::<TestConfig>::empty().insert_value(&document_with_string);

        let old_schema = one_field_schema(literals_validator(vec!["a", "b", "c"])?)?;
        let new_schema = one_field_schema(literals_validator(vec!["a", "b", "c", "d"])?)?;

        // Skips validation in the situation described above
        let tables_to_validate = DatabaseSchema::tables_to_validate(
            &new_schema,
            Some(&old_schema),
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
            &|_name| Some(shape.clone()),
        )?;
        assert!(tables_to_validate.is_empty());

        // Does not skip validation if the diff necessitates it
        let new_schema = one_field_schema(literals_validator(vec!["a", "b", "d"])?)?;
        let tables_to_validate = DatabaseSchema::tables_to_validate(
            &new_schema,
            Some(&old_schema),
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
            &|_name| Some(shape.clone()),
        )?;
        let table_name = TableName::from_str("table")?;
        assert!(tables_to_validate == btreeset! { &table_name });

        // Does not skip validation if there is no previous schema
        let tables_to_validate = DatabaseSchema::tables_to_validate(
            &new_schema,
            None,
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
            &|_name| Some(shape.clone()),
        )?;
        assert!(tables_to_validate == btreeset! { &table_name });

        Ok(())
    }

    #[test]
    fn test_does_not_skip_validation_if_schema_is_not_subset_and_shape_is_not_narrower(
    ) -> anyhow::Result<()> {
        let document_with_string = assert_val!({"field" => "value"});
        let shape = CountedShape::<TestConfig>::empty().insert_value(&document_with_string);

        let old_schema = one_field_schema(literals_validator(vec!["a", "b", "c"])?)?;
        let new_schema = one_field_schema(literals_validator(vec!["a", "b", "d"])?)?;

        let tables_to_validate = DatabaseSchema::tables_to_validate(
            &new_schema,
            Some(&old_schema),
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
            &|_name| Some(shape.clone()),
        )?;
        let table_name = TableName::from_str("table")?;
        assert!(tables_to_validate == btreeset! { &table_name });

        // Does not skip validation if there is no previous schema
        let tables_to_validate = DatabaseSchema::tables_to_validate(
            &new_schema,
            None,
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
            &|_name| Some(shape.clone()),
        )?;
        assert!(tables_to_validate == btreeset! { &table_name });

        Ok(())
    }

    #[test]
    fn test_does_not_skip_validation_there_is_no_previous_schema_and_the_shape_is_not_narrower(
    ) -> anyhow::Result<()> {
        let document_with_string = assert_val!({"field" => "value"});
        let shape = CountedShape::<TestConfig>::empty().insert_value(&document_with_string);

        let new_schema = one_field_schema(literals_validator(vec!["a", "b", "c", "d"])?)?;

        let tables_to_validate = DatabaseSchema::tables_to_validate(
            &new_schema,
            None,
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
            &|_name| Some(shape.clone()),
        )?;
        let table_name = TableName::from_str("table")?;
        assert!(tables_to_validate == btreeset! { &table_name });

        Ok(())
    }

    #[test]
    fn test_type_narrower_than_schema() -> anyhow::Result<()> {
        let shape = CountedShape::<TestConfig>::empty().insert_value(&assert_val!("value"));
        assert!(!Validator::from_shape(
            &shape,
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
        )
        .is_subset(&literals_validator(vec!["a", "b", "c", "d"])?));
        Ok(())
    }

    #[test]
    fn test_skips_validation_if_shape_is_narrower_than_new_schema() -> anyhow::Result<()> {
        let document_with_string = assert_val!({"field" => "value"});
        let shape = CountedShape::<TestConfig>::empty().insert_value(&document_with_string);

        // The schema itself does not allow validation to be skipped
        let old_schema = one_field_schema(Validator::Union(vec![
            Validator::String,
            Validator::Float64,
            Validator::Int64,
        ]))?;
        let new_schema = one_field_schema(Validator::Union(vec![
            Validator::String,
            Validator::Float64,
        ]))?;

        let tables_to_validate = DatabaseSchema::tables_to_validate(
            &new_schema,
            Some(&old_schema),
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
            &|_name| Some(shape.clone()),
        )?;
        assert!(tables_to_validate.is_empty());

        Ok(())
    }

    #[test]
    fn test_schema_narrower_than_type() -> anyhow::Result<()> {
        let shape = CountedShape::<TestConfig>::empty().insert_value(&assert_val!("value"));
        assert!(Validator::from_shape(
            &shape,
            &empty_table_mapping(),
            &VirtualSystemMapping::default(),
        )
        .is_subset(&Validator::Union(vec![
            Validator::String,
            Validator::Float64
        ])));
        Ok(())
    }

    #[test]
    fn test_skips_validation_from_ids_in_shapes() -> anyhow::Result<()> {
        let mut id_generator = TestIdGenerator::new();
        let dog_id: ResolvedDocumentId = id_generator.user_generate(&"dogs".parse()?);
        let document_with_id = assert_val!({"field" => dog_id});
        let shape = CountedShape::<TestConfig>::empty().insert_value(&document_with_id);

        let old_schema = one_field_schema(Validator::Union(vec![
            Validator::Id(TableName::from_str("cats")?),
            Validator::Id(TableName::from_str("dogs")?),
        ]))?;
        let new_schema = one_field_schema(Validator::Id(TableName::from_str("dogs")?))?;

        let table_name = TableName::from_str("table")?;
        let tables_to_validate = DatabaseSchema::tables_to_validate(
            &new_schema,
            Some(&old_schema),
            &id_generator.namespace(TableNamespace::test_user()),
            &VirtualSystemMapping::default(),
            &|name| {
                assert_eq!(&table_name, name);
                Some(shape.clone())
            },
        )?;
        assert!(tables_to_validate.is_empty());

        Ok(())
    }

    #[test]
    fn test_skips_validation_from_virtual_ids_in_shapes() -> anyhow::Result<()> {
        let mut id_generator = TestIdGenerator::new();
        let dog_id: DeveloperDocumentId = id_generator.generate_virtual(&"dogs".parse()?);
        let document_with_id = assert_val!({"field" => dog_id});
        let shape = CountedShape::<TestConfig>::empty().insert_value(&document_with_id);

        let old_schema = one_field_schema(Validator::Union(vec![
            Validator::Id(TableName::from_str("cats")?),
            Validator::Id(TableName::from_str("dogs")?),
        ]))?;
        let new_schema = one_field_schema(Validator::Id(TableName::from_str("dogs")?))?;

        let table_name = TableName::from_str("table")?;
        let tables_to_validate = DatabaseSchema::tables_to_validate(
            &new_schema,
            Some(&old_schema),
            &id_generator.namespace(TableNamespace::test_user()),
            &id_generator.virtual_system_mapping,
            &|name| {
                assert_eq!(&table_name, name);
                Some(shape.clone())
            },
        )?;
        assert!(tables_to_validate.is_empty());

        Ok(())
    }

    #[test]
    fn test_skips_validation_from_ids_in_type() -> anyhow::Result<()> {
        let mut id_generator = TestIdGenerator::new();
        let dog_id: ResolvedDocumentId = id_generator.user_generate(&"dogs".parse()?);
        let shape = CountedShape::<TestConfig>::empty().insert_value(&dog_id.into());
        assert!(Validator::from_shape(
            &shape,
            &id_generator.namespace(TableNamespace::test_user()),
            &id_generator.virtual_system_mapping,
        )
        .is_subset(&Validator::Id(TableName::from_str("dogs")?)));
        Ok(())
    }

    #[test]
    fn test_skips_validation_from_virtual_ids_in_type() -> anyhow::Result<()> {
        let mut id_generator = TestIdGenerator::new();
        let dog_id: DeveloperDocumentId = id_generator.generate_virtual(&"dogs".parse()?);
        let shape = CountedShape::<TestConfig>::empty().insert_value(&dog_id.into());
        assert!(Validator::from_shape(
            &shape,
            &id_generator.namespace(TableNamespace::test_user()),
            &id_generator.virtual_system_mapping,
        )
        .is_subset(&Validator::Id(TableName::from_str("dogs")?)));
        Ok(())
    }
}

mod validator_subset {
    use std::str::FromStr;

    use value::TableName;

    use crate::{
        object_validator,
        schemas::validator::{
            FieldValidator,
            LiteralValidator,
            Validator,
        },
    };

    fn assert_is_strict_subset(subset: Validator, superset: Validator) -> anyhow::Result<()> {
        assert!(
            subset.is_subset(&superset),
            "{subset} must be a subset of {superset}"
        );
        assert!(
            !superset.is_subset(&subset),
            "{subset} must be a strict subset of {superset}, but both are equivalent"
        );
        Ok(())
    }

    fn assert_is_equivalent(left: Validator, right: Validator) -> anyhow::Result<()> {
        assert!(left.is_subset(&right), "{left} must be a subset of {right}");
        assert!(
            right.is_subset(&left),
            "{left} must be a equivalent to {right} but is only a subset"
        );
        Ok(())
    }

    fn assert_is_unrelated(left: Validator, right: Validator) -> anyhow::Result<()> {
        assert!(
            !left.is_subset(&right),
            "{left} must not be a subset of {right}"
        );
        assert!(
            !right.is_subset(&left),
            "{right} must not be a subset of {left}"
        );
        Ok(())
    }

    #[test]
    fn test_different_types_are_unrelated() -> anyhow::Result<()> {
        assert_is_unrelated(Validator::Int64, Validator::String)
    }

    #[test]
    fn test_validators_with_different_parameters_are_unrelated() -> anyhow::Result<()> {
        assert_is_unrelated(
            Validator::Literal(LiteralValidator::Boolean(false)),
            Validator::Literal(LiteralValidator::Boolean(true)),
        )
    }

    #[test]
    fn test_generic_validators_with_same_type_are_equivalent() -> anyhow::Result<()> {
        assert_is_equivalent(
            Validator::Array(Box::new(Validator::Int64)),
            Validator::Array(Box::new(Validator::Int64)),
        )
    }

    #[test]
    fn test_generic_validators_with_different_types_are_unrelated() -> anyhow::Result<()> {
        assert_is_unrelated(
            Validator::Array(Box::new(Validator::Int64)),
            Validator::Array(Box::new(Validator::Float64)),
        )
    }

    #[test]
    fn test_union_containing_an_validator_is_a_superset_of_the_validator() -> anyhow::Result<()> {
        assert_is_strict_subset(
            Validator::Int64,
            Validator::Union(vec![Validator::Float64, Validator::Int64]),
        )
    }

    #[test]
    fn test_adding_an_element_to_an_union() -> anyhow::Result<()> {
        assert_is_strict_subset(
            Validator::Union(vec![Validator::Float64, Validator::Int64]),
            Validator::Union(vec![
                Validator::Float64,
                Validator::Int64,
                Validator::String,
            ]),
        )
    }

    #[test]
    fn test_nested_unions() -> anyhow::Result<()> {
        assert_is_equivalent(
            Validator::Union(vec![Validator::Union(vec![Validator::Bytes])]),
            Validator::Union(vec![Validator::Union(vec![Validator::Union(vec![
                Validator::Bytes,
            ])])]),
        )
    }

    #[test]
    fn test_nested_unions_with_multiple_elements() -> anyhow::Result<()> {
        assert_is_equivalent(
            Validator::Union(vec![
                Validator::Union(vec![Validator::Float64, Validator::Int64]),
                Validator::Union(vec![Validator::Boolean, Validator::String]),
            ]),
            Validator::Union(vec![Validator::Union(vec![
                Validator::Int64,
                Validator::Float64,
                Validator::Boolean,
                Validator::String,
            ])]),
        )
    }

    #[test]
    fn test_union_containing_any() -> anyhow::Result<()> {
        assert_is_equivalent(
            Validator::Union(vec![Validator::Float64, Validator::Int64, Validator::Any]),
            Validator::Any,
        )
    }

    #[test]
    fn test_null_is_a_value() -> anyhow::Result<()> {
        assert_is_strict_subset(
            Validator::String,
            Validator::Union(vec![Validator::String, Validator::Null]),
        )
    }

    #[test]
    fn test_arrays_are_covariant() -> anyhow::Result<()> {
        assert_is_strict_subset(
            Validator::Array(Box::new(Validator::String)),
            Validator::Array(Box::new(Validator::Any)),
        )
    }

    #[test]
    fn test_sets_are_covariant() -> anyhow::Result<()> {
        assert_is_strict_subset(
            Validator::Set(Box::new(Validator::String)),
            Validator::Set(Box::new(Validator::Any)),
        )
    }

    #[test]
    fn test_arrays_and_sets_dont_mix() -> anyhow::Result<()> {
        assert_is_unrelated(
            Validator::Array(Box::new(Validator::String)),
            Validator::Set(Box::new(Validator::String)),
        )
    }

    #[test]
    fn test_maps_are_covariant_over_keys() -> anyhow::Result<()> {
        assert_is_strict_subset(
            Validator::Map(Box::new(Validator::String), Box::new(Validator::Any)),
            Validator::Map(Box::new(Validator::Any), Box::new(Validator::Any)),
        )
    }

    #[test]
    fn test_maps_are_covariant_over_values() -> anyhow::Result<()> {
        assert_is_strict_subset(
            Validator::Map(Box::new(Validator::Any), Box::new(Validator::String)),
            Validator::Map(Box::new(Validator::Any), Box::new(Validator::Any)),
        )
    }

    #[test]
    fn test_maps_not_related_when_key_not_related() -> anyhow::Result<()> {
        assert_is_unrelated(
            Validator::Map(Box::new(Validator::String), Box::new(Validator::Any)),
            Validator::Map(Box::new(Validator::Int64), Box::new(Validator::Any)),
        )
    }

    #[test]
    fn test_maps_not_related_when_value_not_related() -> anyhow::Result<()> {
        assert_is_unrelated(
            Validator::Map(Box::new(Validator::Any), Box::new(Validator::String)),
            Validator::Map(Box::new(Validator::Any), Box::new(Validator::Int64)),
        )
    }

    #[test]
    fn test_changing_union_order() -> anyhow::Result<()> {
        assert_is_equivalent(
            Validator::Union(vec![Validator::Int64, Validator::Float64]),
            Validator::Union(vec![Validator::Float64, Validator::Int64]),
        )
    }

    #[test]
    fn test_string_literal() -> anyhow::Result<()> {
        assert_is_strict_subset(
            Validator::Literal(LiteralValidator::String("abc".try_into()?)),
            Validator::String,
        )
    }

    #[test]
    fn test_different_string_literals_dont_mix() -> anyhow::Result<()> {
        assert_is_unrelated(
            Validator::Literal(LiteralValidator::String("abc".try_into()?)),
            Validator::Literal(LiteralValidator::String("xyz".try_into()?)),
        )
    }

    #[test]
    fn test_int_literal_is_a_int() -> anyhow::Result<()> {
        assert_is_strict_subset(
            Validator::Literal(LiteralValidator::Int64(42)),
            Validator::Int64,
        )
    }

    #[test]
    fn test_union_of_int_literals_is_subset_of_int() -> anyhow::Result<()> {
        assert_is_strict_subset(
            Validator::Union(vec![
                Validator::Literal(LiteralValidator::Int64(1)),
                Validator::Literal(LiteralValidator::Int64(2)),
                Validator::Literal(LiteralValidator::Int64(3)),
            ]),
            Validator::Int64,
        )
    }

    #[test]
    fn test_float_literal_is_a_float() -> anyhow::Result<()> {
        assert_is_strict_subset(
            Validator::Literal(LiteralValidator::Float64((12.34).into())),
            Validator::Float64,
        )
    }

    #[test]
    fn test_boolean_literal_is_a_boolean() -> anyhow::Result<()> {
        assert_is_strict_subset(
            Validator::Literal(LiteralValidator::Boolean(false)),
            Validator::Boolean,
        )
    }

    #[test]
    fn test_union_of_boolean_literals() -> anyhow::Result<()> {
        assert_is_equivalent(
            Validator::Boolean,
            Validator::Union(vec![
                Validator::Literal(LiteralValidator::Boolean(true)),
                Validator::Literal(LiteralValidator::Boolean(false)),
            ]),
        )
    }

    #[test]
    fn test_ids_from_different_tables_dont_mix_up() -> anyhow::Result<()> {
        assert_is_unrelated(
            Validator::Id(TableName::from_str("users")?),
            Validator::Id(TableName::from_str("messages")?),
        )
    }

    #[test]
    fn test_ids_are_strings() -> anyhow::Result<()> {
        assert_is_strict_subset(
            Validator::Id(TableName::from_str("messages")?),
            Validator::String,
        )
    }

    #[test]
    fn test_union_of_arrays_is_subset_of_array_of_union() -> anyhow::Result<()> {
        assert_is_strict_subset(
            Validator::Union(vec![
                Validator::Array(Box::new(Validator::String)),
                Validator::Array(Box::new(Validator::Int64)),
            ]),
            Validator::Array(Box::new(Validator::Union(vec![
                Validator::String,
                Validator::Int64,
            ]))),
        )
    }

    #[test]
    fn test_identical_objects() -> anyhow::Result<()> {
        let object_validator = Validator::Object(object_validator!(
            "a" => FieldValidator::required_field_type(Validator::String),
            "b" => FieldValidator::optional_field_type(Validator::Int64),
        ));
        assert_is_equivalent(object_validator.clone(), object_validator)
    }

    #[test]
    fn test_objects_with_different_field_names() -> anyhow::Result<()> {
        assert_is_unrelated(
            Validator::Object(object_validator!(
                "a" => FieldValidator::required_field_type(Validator::Any),
            )),
            Validator::Object(object_validator!(
                "b" => FieldValidator::required_field_type(Validator::Any),
            )),
        )
    }

    #[test]
    fn test_objects_whose_fields_are_subsets() -> anyhow::Result<()> {
        assert_is_strict_subset(
            Validator::Object(object_validator!(
                "a" => FieldValidator::required_field_type(Validator::String),
            )),
            Validator::Object(object_validator!(
                "a" => FieldValidator::required_field_type(Validator::Any),
            )),
        )
    }

    #[test]
    fn test_adding_an_optional_field_to_an_objet() -> anyhow::Result<()> {
        assert_is_strict_subset(
            Validator::Object(object_validator!()),
            Validator::Object(object_validator!(
                "a" => FieldValidator::optional_field_type(Validator::Any),
            )),
        )
    }

    #[test]
    fn test_making_a_field_optional_without_changing_its_type() -> anyhow::Result<()> {
        assert_is_strict_subset(
            Validator::Object(object_validator!(
                "a" => FieldValidator::required_field_type(Validator::Any),
            )),
            Validator::Object(object_validator!(
                "a" => FieldValidator::optional_field_type(Validator::Any),
            )),
        )
    }

    #[test]
    fn test_making_a_field_optional_while_making_breaking_changes() -> anyhow::Result<()> {
        assert_is_unrelated(
            Validator::Object(object_validator!(
                "a" => FieldValidator::required_field_type(Validator::Any),
            )),
            Validator::Object(object_validator!(
                "a" => FieldValidator::optional_field_type(Validator::String),
            )),
        )
    }

    #[test]
    fn test_making_a_field_optional_while_making_non_breaking_changes() -> anyhow::Result<()> {
        assert_is_strict_subset(
            Validator::Object(object_validator!(
                "a" => FieldValidator::required_field_type(Validator::String),
            )),
            Validator::Object(object_validator!(
                "a" => FieldValidator::optional_field_type(Validator::Any),
            )),
        )
    }

    #[test]
    fn test_duplicates_in_unions_can_be_simplified() -> anyhow::Result<()> {
        assert_is_equivalent(
            Validator::Union(vec![
                Validator::String,
                Validator::Int64,
                Validator::Int64,
                Validator::String,
                Validator::Int64,
            ]),
            Validator::Union(vec![Validator::Int64, Validator::String]),
        )
    }
}

mod can_contain_field {
    use std::str::FromStr;

    use value::FieldPath;

    use crate::{
        object_validator,
        schemas::{
            validator::{
                FieldValidator,
                LiteralValidator,
                Validator,
            },
            DocumentSchema,
        },
    };

    #[test]
    fn returns_true_for_existing_fields_in_a_simple_schema() -> anyhow::Result<()> {
        assert!(DocumentSchema::Union(vec![
            object_validator!("field" => FieldValidator::required_field_type(Validator::String))
        ])
        .can_contain_field(&FieldPath::from_str("field")?));
        Ok(())
    }

    #[test]
    fn returns_true_for_existing_optional_fields() -> anyhow::Result<()> {
        assert!(DocumentSchema::Union(vec![
            object_validator!("field" => FieldValidator::optional_field_type(Validator::String))
        ])
        .can_contain_field(&FieldPath::from_str("field")?));
        Ok(())
    }

    #[test]
    fn returns_false_for_missing_fields_in_a_simple_schema() -> anyhow::Result<()> {
        assert!(!DocumentSchema::Union(vec![
            object_validator!("field" => FieldValidator::required_field_type(Validator::String))
        ])
        .can_contain_field(&FieldPath::from_str("other_field")?));
        Ok(())
    }

    #[test]
    fn returns_true_for_fields_that_are_sometimes_there_in_a_union_schema() -> anyhow::Result<()> {
        assert!(DocumentSchema::Union(vec![
            object_validator!("field" => FieldValidator::required_field_type(Validator::String)),
            object_validator!("other_field" => FieldValidator::required_field_type(Validator::String)),
        ])
        .can_contain_field(&FieldPath::from_str("other_field")?));
        Ok(())
    }

    #[test]
    fn returns_true_for_fields_that_are_always_in_a_schema_containing_an_union(
    ) -> anyhow::Result<()> {
        assert!(DocumentSchema::Union(vec![
            object_validator!("token" => FieldValidator::required_field_type(Validator::Union(vec![
                Validator::Object(object_validator!(
                    "type" => FieldValidator::required_field_type(Validator::Literal(LiteralValidator::String("discord".try_into()?))),
                    "username" => FieldValidator::required_field_type(Validator::String),
                )),
                Validator::Object(object_validator!(
                    "type" => FieldValidator::required_field_type(Validator::Literal(LiteralValidator::String("google".try_into()?))),
                    "email" => FieldValidator::required_field_type(Validator::String),
                )),
            ]))),
        ])
        .can_contain_field(&FieldPath::from_str("token.type")?));
        Ok(())
    }

    #[test]
    fn returns_true_for_fields_that_are_sometimes_in_a_schema_containing_an_union(
    ) -> anyhow::Result<()> {
        assert!(DocumentSchema::Union(vec![
            object_validator!("token" => FieldValidator::required_field_type(Validator::Union(vec![
                Validator::Object(object_validator!(
                    "type" => FieldValidator::required_field_type(Validator::Literal(LiteralValidator::String("discord".try_into()?))),
                    "username" => FieldValidator::required_field_type(Validator::String),
                )),
                Validator::Object(object_validator!(
                    "type" => FieldValidator::required_field_type(Validator::Literal(LiteralValidator::String("google".try_into()?))),
                    "email" => FieldValidator::required_field_type(Validator::String),
                )),
                Validator::String,
            ]))),
        ])
        .can_contain_field(&FieldPath::from_str("token.email")?));
        Ok(())
    }

    #[test]
    fn returns_true_fields_in_schemas_using_any_at_root() -> anyhow::Result<()> {
        assert!(DocumentSchema::Any.can_contain_field(&FieldPath::from_str("field")?));
        Ok(())
    }

    #[test]
    fn returns_true_for_nested_fields() -> anyhow::Result<()> {
        assert!(DocumentSchema::Union(vec![
            object_validator!("field" => FieldValidator::required_field_type(Validator::Object(
                object_validator!("subfield" => FieldValidator::required_field_type(Validator::String))
            )))
        ])
        .can_contain_field(&FieldPath::from_str("field.subfield")?));
        Ok(())
    }

    #[test]
    fn returns_false_for_nested_fields_that_are_nested_too_far() -> anyhow::Result<()> {
        assert!(!DocumentSchema::Union(vec![
            object_validator!("field" => FieldValidator::required_field_type(Validator::Object(
                object_validator!("subfield" => FieldValidator::required_field_type(Validator::String))
            )))
        ])
        .can_contain_field(&FieldPath::from_str("field.subfield.does_not_exist")?));
        Ok(())
    }

    #[test]
    fn returns_true_for_fields_in_schemas_using_any_at_sublevel() -> anyhow::Result<()> {
        assert!(DocumentSchema::Union(vec![
            object_validator!("field" => FieldValidator::required_field_type(Validator::Any))
        ])
        .can_contain_field(&FieldPath::from_str("field.subfield")?));
        Ok(())
    }

    #[test]
    fn returns_true_for_the_id_field() -> anyhow::Result<()> {
        assert!(DocumentSchema::Union(vec![
            object_validator!("field" => FieldValidator::required_field_type(Validator::Any))
        ])
        .can_contain_field(&FieldPath::from_str("_id")?));
        Ok(())
    }

    #[test]
    fn returns_true_for_the_creation_time_field() -> anyhow::Result<()> {
        assert!(DocumentSchema::Union(vec![
            object_validator!("field" => FieldValidator::required_field_type(Validator::Any))
        ])
        .can_contain_field(&FieldPath::from_str("_creationTime")?));
        Ok(())
    }
}
