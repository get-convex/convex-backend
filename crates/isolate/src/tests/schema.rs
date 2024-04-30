use common::{
    object_validator,
    runtime::Runtime,
    schemas::{
        validator::{
            FieldValidator,
            LiteralValidator,
            Validator,
        },
        DatabaseSchema,
        DocumentSchema,
        IndexSchema,
        SearchIndexSchema,
        TableDefinition,
    },
    types::{
        IndexDescriptor,
        TableName,
    },
};
use maplit::{
    btreemap,
    btreeset,
};
use rand::Rng;
use runtime::testing::TestRuntime;

use crate::test_helpers::UdfTest;

#[convex_macro::test_runtime]
async fn test_eval_schema(rt: TestRuntime) -> anyhow::Result<()> {
    // Document type is copied and pasted from
    // `npm-packages/convex/src/schema/index.test.ts`
    let source = r#"
    class SchemaDefinition {
        tables;

        constructor() {
          this.tables = {
            schemaValidation: true,
            tables: [
              {
                tableName: "noIndexes",
                indexes: [],
                "documentType": {
                  "type": "object",
                  "value": {
                    "ref": {
                      "fieldType": {
                        "type": "id",
                        "tableName": "twoIndexTable"
                      },
                      "optional": false
                    },
                    "nullField": {
                      "fieldType": {
                        "type": "null"
                      },
                      "optional": false
                    },
                    "numberField": {
                      "fieldType": {
                        "type": "number"
                      },
                      "optional": false
                    },
                    "bigintField": {
                      "fieldType": {
                        "type": "bigint"
                      },
                      "optional": false
                    },
                    "booleanField": {
                      "fieldType": {
                        "type": "boolean"
                      },
                      "optional": false
                    },
                    "stringField": {
                      "fieldType": {
                        "type": "string"
                      },
                      "optional": false
                    },
                    "bytesField": {
                      "fieldType": {
                        "type": "bytes"
                      },
                      "optional": false
                    },
                    "arrayField": {
                      "fieldType": {
                        "type": "array",
                        "value": {
                          "type": "boolean"
                        }
                      },
                      "optional": false
                    },
                    "setField": {
                      "fieldType": {
                        "type": "set",
                        "value": {
                          "type": "number"
                        }
                      },
                      "optional": false
                    },
                    "mapField": {
                      "fieldType": {
                        "type": "map",
                        "keys": {
                          "type": "string"
                        },
                        "values": {
                          "type": "number"
                        }
                      },
                      "optional": false
                    },
                    "anyField": {
                      "fieldType": {
                        "type": "any"
                      },
                      "optional": false
                    },
                    "literalBigint": {
                      "fieldType": {
                        "type": "literal",
                        "value": {
                          "$integer": "AQAAAAAAAAA="
                        }
                      },
                      "optional": false
                    },
                    "literalNumber": {
                      "fieldType": {
                        "type": "literal",
                        "value": 0
                      },
                      "optional": false
                    },
                    "literalString": {
                      "fieldType": {
                        "type": "literal",
                        "value": "hello world"
                      },
                      "optional": false
                    },
                    "literalBoolean": {
                      "fieldType": {
                        "type": "literal",
                        "value": true
                      },
                      "optional": false
                    },
                    "union": {
                      "fieldType": {
                        "type": "union",
                        "value": [
                          {
                            "type": "string"
                          },
                          {
                            "type": "number"
                          }
                        ]
                      },
                      "optional": false
                    },
                    "object": {
                      "fieldType": {
                        "type": "object",
                        "value": {
                          "a": {
                            "fieldType": {
                              "type": "any"
                            },
                            "optional": true
                          }
                        }
                      },
                      "optional": false
                    }
                  }
                }
              },
              {
                tableName: "twoIndexTable",
                indexes: [
                  {
                    indexDescriptor: "by_email",
                    fields: ["email"],
                  },
                  {
                    indexDescriptor: "by_creation_deleted",
                    fields: ["creation", "deleted"],
                  },
                ],
              },
              {
                tableName: "searchIndexTable",
                indexes: [],
                searchIndexes: [
                  {
                    indexDescriptor: "search_index",
                    searchField: "title",
                    filterFields: ["is_deleted", "workspace_id"],
                  },
                ],
              },
            ],
          };
        }

        export() {
          return JSON.stringify(this.tables);
        }
      }

      export default new SchemaDefinition();
"#;

    let rng_seed = rt.with_rng(|rng| rng.gen());
    let unix_timestamp = rt.unix_timestamp();
    let t = UdfTest::default_with_modules(vec![], rt).await??;

    let schema = t
        .isolate
        .evaluate_schema(source.to_string(), None, rng_seed, unix_timestamp)
        .await?;

    let name1: TableName = "noIndexes".parse()?;
    let name2: TableName = "twoIndexTable".parse()?;
    let name3: TableName = "searchIndexTable".parse()?;
    let by_email: IndexDescriptor = "by_email".parse()?;
    let by_creation_deleted: IndexDescriptor = "by_creation_deleted".parse()?;
    let search_index: IndexDescriptor = "search_index".parse()?;
    let expected = DatabaseSchema {
        tables: btreemap!(
            name1.clone() => TableDefinition {
                table_name: name1,
                indexes: btreemap!(),
                search_indexes: btreemap!(),
                vector_indexes: btreemap!(),
                document_type: Some(DocumentSchema::Union(vec![
                  object_validator!(
                    "ref" => FieldValidator::required_field_type(Validator::Id("twoIndexTable".parse()?)),
                    "nullField" => FieldValidator::required_field_type(Validator::Null),
                    "numberField" => FieldValidator::required_field_type(Validator::Float64),
                    "bigintField" => FieldValidator::required_field_type(Validator::Int64),
                    "booleanField" => FieldValidator::required_field_type(Validator::Boolean),
                    "stringField" => FieldValidator::required_field_type(Validator::String),
                    "bytesField" => FieldValidator::required_field_type(Validator::Bytes),
                    "arrayField" => FieldValidator::required_field_type(Validator::Array(Box::new(Validator::Boolean))),
                    "setField" => FieldValidator::required_field_type(Validator::Set(Box::new(Validator::Float64))),
                    "mapField" => FieldValidator::required_field_type(Validator::Map(Box::new(Validator::String), Box::new(Validator::Float64))),
                    "anyField" => FieldValidator::required_field_type(Validator::Any),
                    "literalBigint" => FieldValidator::required_field_type(Validator::Literal(LiteralValidator::Int64(1))),
                    "literalNumber" => FieldValidator::required_field_type(Validator::Literal(LiteralValidator::Float64((0.).into()))),
                    "literalString" => FieldValidator::required_field_type(Validator::Literal(LiteralValidator::String("hello world".to_string().try_into()?))),
                    "literalBoolean" => FieldValidator::required_field_type(Validator::Literal(LiteralValidator::Boolean(true))),
                    "union" => FieldValidator::required_field_type(Validator::Union(vec![Validator::String, Validator::Float64])),
                    "object" => FieldValidator::required_field_type(Validator::Object(object_validator!("a" => FieldValidator::optional_field_type(Validator::Any))))
                  )
                ]))
            },
            name2.clone() => TableDefinition {
                table_name: name2,
                indexes: btreemap!(
                    by_email.clone() => IndexSchema {
                        index_descriptor: by_email,
                        fields: vec!["email".parse()?].try_into()?,
                    },
                    by_creation_deleted.clone() => IndexSchema {
                        index_descriptor: by_creation_deleted,
                        fields: vec!["creation".parse()?, "deleted".parse()?].try_into()?,
                    },
                ),
                search_indexes: btreemap!(),
                vector_indexes: btreemap!(),
                document_type: None,
            },
            name3.clone() => TableDefinition {
              table_name: name3,
              indexes: btreemap!(),
              search_indexes: btreemap! {
                search_index.clone() => SearchIndexSchema::new(
                  search_index,
                  "title".parse()?,
                  btreeset!{"is_deleted".parse()?, "workspace_id".parse()?}
                )?
               },
               vector_indexes: btreemap!(),
               document_type: None,
          }
        ),
        schema_validation: true,
    };
    assert_eq!(schema, expected);
    Ok(())
}
