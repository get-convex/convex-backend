use std::{
    fmt::Display,
    ops::Deref,
};

use common::{
    document::CREATION_TIME_FIELD_PATH,
    schemas::{
        validator::{
            FieldValidator,
            Validator,
        },
        DocumentSchema,
        IndexSchema,
        TableDefinition,
    },
    value::{
        IdentifierFieldName,
        TableName,
    },
};
use convex_fivetran_common::fivetran_sdk::DataType as FivetranDataType;
use convex_fivetran_destination::api_types::{
    FivetranFieldName,
    FivetranTableName,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DestinationError {
    #[error("The name of table `{0}` is invalid: {1:#}")]
    InvalidTableName(String, anyhow::Error),

    #[error("The name of table `{0}` isn’t supported by Convex: {1:#}")]
    UnsupportedTableName(String, anyhow::Error),

    #[error("The name of column `{0}` in table `{1}` is invalid: {1:#}")]
    InvalidColumnName(String, FivetranTableName, anyhow::Error),

    #[error("The name of column `{0}` in table `{1}` isn’t supported by Convex: {2:#}")]
    UnsupportedColumnName(FivetranFieldName, FivetranTableName, anyhow::Error),

    #[error(
        "Your Convex destination is not using a schema. Please add a `schema.ts` file to add the \
         `{0}` table. You can use the following table definition: {1}"
    )]
    DestinationHasNoSchema(TableName, SuggestedTable),

    #[error(
        "Your Convex destination is not using a schema. We are not able to suggest a schema \
         because the following error happened: {0}"
    )]
    DestinationHasNoSchemaWithoutSuggestion(Box<DestinationError>),

    #[error(
        "The table `{0}` from your data source is missing in the schema of your Convex \
         destination. Please edit your `schema.ts` file to add the table. You can use the \
         following table definition: {1}"
    )]
    MissingTable(TableName, SuggestedTable),

    #[error(
        "The table `{0}` from your data source is missing in the schema of your Convex \
         destination. We are not able to suggest a schema because the following error happened: \
         {1}"
    )]
    MissingTableWithoutSuggestion(TableName, Box<DestinationError>),

    #[error(
        "The table `{0}` from your data source is incorrect in the schema of your Convex \
         destination. {1} HINT: you can use the following table definition in your `schema.ts` \
         file: {2}"
    )]
    IncorrectSchemaForTable(TableName, TableSchemaError, SuggestedTable),

    #[error(
        "The table `{0}` from your data source is incorrect in the schema of your Convex \
         destination. {1}"
    )]
    IncorrectSchemaForTableWithoutSuggestion(TableName, TableSchemaError),

    #[error(
        "The key provided by Fivetran to decrypt the source data is invalid. Please contact \
         support."
    )]
    InvalidKey,

    #[error(
        "The table `{0}` in the Convex destination stores arbitrary documents, which is not \
         supported by Fivetran. Please edit the schema of the table in `schema.ts` so that the \
         table isn’t defined as `v.any()`."
    )]
    DestinationHasAnySchema(TableName),

    #[error(
        "The table `{0}` in the Convex destination stores multiple different types of documents, \
         which is not supported by Fivetran. Please edit the schema of the table in `schema.ts` \
         so that the table isn’t defiend as `v.union()`."
    )]
    DestinationHasMultipleSchemas(TableName),

    #[error("An error occurred on the Convex deployment: {0:#}")]
    DeploymentError(anyhow::Error),

    #[error("A row from your data source is invalid: {0:#}")]
    InvalidRow(anyhow::Error),

    #[error("Can’t read the file {0}: {1:#}. Please contact support.")]
    FileReadError(String, anyhow::Error),
}

#[derive(Debug, Error)]
pub enum TableSchemaError {
    #[error(
        "The `fivetran` column is missing from the table in Convex. Please edit the schema of the \
         table in `schema.ts` and add the following attribute: `fivetran: {suggested}`."
    )]
    MissingMetadataColumn { suggested: FieldValidator },

    #[error(
        "{error}. Please fix the `fivetran` column in your Convex schema (currently defined as \
         `fivetran: {actual}`. You can fix this by editing the schema of the table in `schema.ts` \
         and defining the `fivetran` field as such: `fivetran: {suggested}`."
    )]
    IncorrectMetadataColumn {
        error: MetadataFieldError,
        actual: FieldValidator,
        suggested: FieldValidator,
    },

    #[error(
        "The table in the Convex destination stores arbitrary documents, which is not supported \
         by Fivetran. Please edit the schema of the table in `schema.ts` so that the table isn’t \
         defined as `v.any()`."
    )]
    DestinationHasAnySchema,

    #[error(
        "The table in the Convex destination stores multiple different types of documents, which \
         is not supported by Fivetran. Please edit the schema of the table in `schema.ts` so that \
         the table isn’t defiend as `v.union()`."
    )]
    DestinationHasMultipleSchemas,

    #[error(
        "The name of field `{0}` isn’t supported by Convex: {1:#}. Please modify the name of the \
         field in your data source."
    )]
    UnsupportedFieldName(FivetranFieldName, anyhow::Error),

    #[error(
        "The primary key of the table isn’t supported by Convex: {0:#}. Please contact \
         support@convex.dev if you need help."
    )]
    UnsupportedPrimaryKey(anyhow::Error),

    #[error(
        "The field `{field_name}` is missing from your Convex schema. Please add `{field_name}: \
         {suggested_validator}` to the definition of the table in `schema.ts`."
    )]
    MissingField {
        field_name: FivetranFieldName,
        suggested_validator: Validator,
    },

    #[error(
        "The field `{field_name}` has a type in Convex ({actual_validator}) that doesn’t match \
         the type in the source table ({fivetran_type:?}, which would be {expected_validator} in \
         Convex). Please modify the definition of the field in `schema.ts`."
    )]
    NonmatchingFieldValidator {
        field_name: FivetranFieldName,
        actual_validator: Validator,
        expected_validator: Validator,
        fivetran_type: FivetranDataType,
    },

    #[error(
        "The table in your data source contains a field named `fivetran`. This name isn’t \
         supported in Convex, as it is used to store the Fivetran synchronization metadata. \
         Please modify the name of the column in your data source."
    )]
    SourceTableHasFivetranField,

    #[error(
        "The table in Convex has a `{0}` field that is missing in your data source. Please modify \
         your Convex schema in `schema.ts` to remove the field."
    )]
    FieldMissingInSource(IdentifierFieldName),

    #[error(
        "The table in Convex needs an index on `fivetran.synced`. Please add the following index \
         to the table in your `schema.ts` file: `.index(\"sync_index\", [\"fivetran.synced\"])`"
    )]
    MissingSyncIndex,

    #[error(
        "The table in Convex needs an index on (`fivetran.deleted`, `fivetran.synced`). Please \
         add the following index to the table in your `schema.ts` file: `.index(\"sync_index\", \
         [\"fivetran.deleted\", \"fivetran.synced\"])`"
    )]
    MissingSyncIndexWithSoftDeletes,

    #[error(
        "The table in Convex is missing a `by_primary_key` index. Please modify the table \
         definition in `schema.ts` to add an index for the primary key of the table: (`{0}`)."
    )]
    MissingPrimaryKeyIndex(SuggestedIndex),

    #[error(
        "The `by_primary_key` index on the Convex table doesn’t match the fields of the primary \
         key. Please modify the index in `schema.ts` to match the primary key of the table: \
         (`{0}`)."
    )]
    WrongPrimaryKeyIndex(SuggestedIndex),
}

#[derive(Debug, Error)]
pub enum MetadataFieldError {
    #[error("The type of the `fivetran` field must be v.object()")]
    InvalidMetadataFieldType,

    #[error("Invalid validator for _fivetran_synced")]
    InvalidSyncedField,

    #[error("Invalid validator for _fivetran_id")]
    InvalidIdField,

    #[error("Invalid validator for _fivetran_deleted")]
    InvalidDeletedField,

    #[error("Invalid type for `fivetran.columns`, which must be an object validator")]
    InvalidColumnsFieldType,

    #[error("The name of column `{0}` is not supported by Fivetran: {1:#}")]
    UnsupportedColumnName(IdentifierFieldName, anyhow::Error),

    #[error("The data source does not contain a column named `{0}`")]
    MissingColumnsField(FivetranFieldName),

    #[error("Missing field {0} in `fivetran.columns`")]
    MissingFieldInColumns(FivetranFieldName),

    #[error(
        "The column `{field_name}` is incorrectly specified in `fivetran.columns` \
         (`{actual_validator}` instead of `{expected_validator}`)"
    )]
    IncorrectColumnSpecification {
        field_name: FivetranFieldName,
        actual_validator: Validator,
        expected_validator: Validator,
    },

    #[error(
        "Missing a `fivetran.columns` field, which is expected since your data source contains a \
         field name starting with `_` (`{0}`)"
    )]
    ColumnInMetadataNotInDataSource(FivetranFieldName),
}

/// Wrapper around `TableDefinition` that formats it in the same format as
/// `schema.ts`.
#[derive(Debug)]
pub struct SuggestedTable(pub TableDefinition);

impl Display for SuggestedTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let table_name = &self.0.table_name;

        let fields = display_fields(&self.0.document_type).unwrap_or_else(|| "/* … */".to_string());

        let indexes: Vec<String> = self
            .0
            .indexes
            .values()
            .map(|index| SuggestedIndex(index.clone()).to_string())
            .collect();
        let indexes = indexes.join("");

        write!(f, "`{table_name}: defineTable({{ {fields} }}){indexes}`",)
    }
}

fn display_fields(schema: &Option<DocumentSchema>) -> Option<String> {
    // We only support here simple schemas. Complex schemas aren’t supported by
    // Fivetran, so we’re never suggesting them.
    let Some(schema) = schema else {
        return None;
    };
    let DocumentSchema::Union(validators) = schema else {
        return None;
    };
    let [validator] = &validators[..] else {
        return None;
    };

    let fields: Vec<_> = validator
        .0
        .iter()
        .map(|(field_name, validator)| format!("{field_name}: {validator}"))
        .collect();
    Some(fields.join(", "))
}

/// Wrapper around `IndexSchema` that formats it in the same format as
/// `schema.ts`.
#[derive(Debug)]
pub struct SuggestedIndex(pub IndexSchema);

impl Display for SuggestedIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fields: Vec<_> = self
            .0
            .fields
            .iter()
            .filter(|f| *f != CREATION_TIME_FIELD_PATH.deref())
            .map(|field| field.to_string())
            .collect();
        write!(
            f,
            ".index(\"{}\", [{}])",
            self.0.index_descriptor,
            fields.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use common::{
        object_validator,
        schemas::{
            validator::{
                FieldValidator,
                Validator,
            },
            DocumentSchema,
            IndexSchema,
            TableDefinition,
        },
        types::IndexDescriptor,
    };
    use maplit::btreemap;

    use super::SuggestedIndex;
    use crate::error::SuggestedTable;

    #[test]
    fn it_formats_suggested_indexes() {
        let schema = IndexSchema {
            index_descriptor: IndexDescriptor::new("by_field_and_subfield").unwrap(),
            fields: vec![
                "field".parse().unwrap(),
                "field.subfield".parse().unwrap(),
                "_creationTime".parse().unwrap(),
            ]
            .try_into()
            .unwrap(),
        };

        assert_eq!(
            SuggestedIndex(schema).to_string(),
            ".index(\"by_field_and_subfield\", [\"field\", \"field.subfield\"])".to_string(),
        );
    }

    #[test]
    fn it_formats_table_definitions() -> anyhow::Result<()> {
        let table = TableDefinition {
            table_name: "my_table".parse().unwrap(),
            indexes: btreemap! {
                IndexDescriptor::new("by_name").unwrap() => IndexSchema {
                    index_descriptor: IndexDescriptor::new("by_name").unwrap(),
                    fields: vec![
                        "name".parse().unwrap()
                    ].try_into().unwrap()
                },
                IndexDescriptor::new("by_email").unwrap() => IndexSchema {
                    index_descriptor: IndexDescriptor::new("by_email").unwrap(),
                    fields: vec![
                        "email".parse().unwrap()
                    ].try_into().unwrap()
                }
            },
            document_type: Some(DocumentSchema::Union(vec![object_validator!(
                "name" => FieldValidator::required_field_type(Validator::String),
                "email" => FieldValidator::required_field_type(Validator::String),
            )])),
            search_indexes: Default::default(),
            vector_indexes: Default::default(),
        };

        assert_eq!(
            SuggestedTable(table).to_string(),
            "`my_table: defineTable({ email: v.string(), name: v.string() }).index(\"by_email\", \
             [\"email\"]).index(\"by_name\", [\"name\"])`"
                .to_string(),
        );

        Ok(())
    }
}
