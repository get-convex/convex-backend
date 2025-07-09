use common::{
    bootstrap_model::index::database_index::IndexedFields,
    types::{
        IndexDescriptor,
        ModuleEnvironment,
    },
};
use errors::ErrorMetadataAnyhowExt;
use model::config::types::ModuleConfig;
use runtime::testing::TestRuntime;

use crate::{
    test_helpers::ApplicationTestExt,
    Application,
};

#[convex_macro::test_runtime]
async fn test_evaluate_schema_append_creation_time(rt: TestRuntime) -> anyhow::Result<()> {
    let source = r#"
class SchemaDefinition {
    tables;

    constructor() {
        this.tables = {
            schemaValidation: true,
            tables: [
                {
                    tableName: "messages",
                    indexes: [
                        {
                            indexDescriptor: "by_channel",
                            fields: ["channel"],
                        },
                        {
                            indexDescriptor: "by_author",
                            fields: ["author"],
                        },
                    ],
                },
            ],
        };
    }

    export() {
        return JSON.stringify(this.tables)
    }
}

export default new SchemaDefinition()
"#;
    let application = Application::new_for_tests(&rt).await?;
    let schema = application
        .evaluate_schema(ModuleConfig {
            path: "schema".parse()?,
            source: source.into(),
            source_map: None,
            environment: ModuleEnvironment::Isolate,
        })
        .await?;

    let table_schema = schema
        .tables
        .get(&"messages".parse()?)
        .expect("Missing messages table");
    assert_eq!(
        table_schema.indexes[&IndexDescriptor::new("by_channel")?].fields,
        IndexedFields::try_from(vec!["channel".parse()?, "_creationTime".parse()?])?
    );
    assert_eq!(
        table_schema.indexes[&IndexDescriptor::new("by_author")?].fields,
        IndexedFields::try_from(vec!["author".parse()?, "_creationTime".parse()?])?
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_evaluate_schema_contains_id(rt: TestRuntime) -> anyhow::Result<()> {
    // Test that we get the proper error message when we add `_id`.
    let source = r#"
class SchemaDefinition {
    tables;

    constructor() {
        this.tables = {
            schemaValidation: true,
            tables: [
                {
                    tableName: "messages",
                    indexes: [
                        {
                            indexDescriptor: "by_my_id",
                            fields: ["_id"],
                        },
                    ],
                },
            ],
        };
    }

    export() {
        return JSON.stringify(this.tables)
    }
}

export default new SchemaDefinition()
"#;
    let application = Application::new_for_tests(&rt).await?;
    let err = application
        .evaluate_schema(ModuleConfig {
            path: "schema".parse()?,
            source: source.into(),
            source_map: None,
            environment: ModuleEnvironment::Isolate,
        })
        .await
        .expect_err("Successfully created invalid schema");
    assert_eq!(err.short_msg(), "IndexFieldsContainId");

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_evaluate_schema_contains_creation_time(rt: TestRuntime) -> anyhow::Result<()> {
    // Test that we get the proper error message when we add `_creationTime`.
    let source = r#"
class SchemaDefinition {
    tables;

    constructor() {
        this.tables = {
            schemaValidation: true,
            tables: [
                {
                    tableName: "messages",
                    indexes: [
                        {
                            indexDescriptor: "by_my_creation_time",
                            fields: ["_creationTime"],
                        },
                    ],
                },
            ],
        };
    }

    export() {
        return JSON.stringify(this.tables)
    }
}

export default new SchemaDefinition()
"#;
    let application = Application::new_for_tests(&rt).await?;
    let err = application
        .evaluate_schema(ModuleConfig {
            path: "schema".parse()?,
            source: source.into(),
            source_map: None,
            environment: ModuleEnvironment::Isolate,
        })
        .await
        .expect_err("Successfully created invalid schema");
    assert_eq!(err.short_msg(), "IndexFieldsContainCreationTime");

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_evaluate_schema_contains_system_field(rt: TestRuntime) -> anyhow::Result<()> {
    let source = r#"
class SchemaDefinition {
    tables;

    constructor() {
        this.tables = {
            schemaValidation: true,
            tables: [
                {
                    tableName: "messages",
                    indexes: [
                        {
                            indexDescriptor: "by_system_field",
                            fields: ["_reserved_for_future"],
                        },
                    ],
                },
            ],
        };
    }

    export() {
        return JSON.stringify(this.tables)
    }
}

export default new SchemaDefinition()
"#;
    let application = Application::new_for_tests(&rt).await?;

    let err = application
        .evaluate_schema(ModuleConfig {
            path: "schema".parse()?,
            source: source.into(),
            source_map: None,
            environment: ModuleEnvironment::Isolate,
        })
        .await
        .expect_err("Successfully created invalid schema");
    assert_eq!(err.short_msg(), "IndexFieldNameReserved");

    Ok(())
}
