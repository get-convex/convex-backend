use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    ops::Deref,
    str::FromStr,
    sync::LazyLock,
};

use common::{
    bootstrap_model::index::database_index::IndexedFields,
    document::CREATION_TIME_FIELD_PATH,
    schemas::{
        validator::{
            FieldValidator,
            ObjectValidator,
            Validator,
        },
        DocumentSchema,
        IndexSchema,
        TableDefinition,
    },
    types::IndexDescriptor,
    value::{
        FieldPath,
        IdentifierFieldName,
        TableName,
    },
};
use convex_fivetran_common::fivetran_sdk::{
    self,
    DataType as FivetranDataType,
};

use crate::{
    api_types::{
        FivetranFieldName,
        FivetranTableName,
    },
    constants::{
        FIVETRAN_SYNC_INDEX_WITHOUT_SOFT_DELETE_FIELDS,
        FIVETRAN_SYNC_INDEX_WITH_SOFT_DELETE_FIELDS,
        ID_CONVEX_FIELD_NAME,
        ID_FIVETRAN_FIELD_NAME,
        METADATA_CONVEX_FIELD_NAME,
        PRIMARY_KEY_INDEX_DESCRIPTOR,
        SOFT_DELETE_CONVEX_FIELD_NAME,
        SOFT_DELETE_FIELD_PATH,
        SOFT_DELETE_FIVETRAN_FIELD_NAME,
        SYNCED_CONVEX_FIELD_NAME,
        SYNCED_FIVETRAN_FIELD_NAME,
    },
    error::{
        DestinationError,
        SuggestedIndex,
        SuggestedTable,
        TableSchemaError,
    },
};

/// The default name of the sync index suggested to the user in error messages.
/// The user doesn’t have to name their sync index like this, it’s only a
/// suggestion.
pub static DEFAULT_FIVETRAN_SYNCED_INDEX_DESCRIPTOR: LazyLock<IndexDescriptor> =
    LazyLock::new(|| "by_fivetran_synced".parse().unwrap());

#[derive(Clone, Debug)]
pub struct FivetranTableColumn {
    pub data_type: FivetranDataType,
    pub in_primary_key: bool,
}

#[derive(Debug, derive_more::From, Clone)]
pub struct FivetranTableSchema {
    pub name: FivetranTableName,
    pub columns: BTreeMap<FivetranFieldName, FivetranTableColumn>,
}

impl TryFrom<fivetran_sdk::Table> for FivetranTableSchema {
    type Error = DestinationError;

    fn try_from(table: fivetran_sdk::Table) -> Result<Self, Self::Error> {
        let name: FivetranTableName = FivetranTableName::from_str(&table.name)
            .map_err(|err| DestinationError::InvalidTableName(table.name, err))?;

        let columns = table
            .columns
            .into_iter()
            .map(|column| -> Result<_, _> {
                let data_type = column.r#type();
                Ok((
                    column
                        .name
                        .parse()
                        .map_err(|err| DestinationError::InvalidTableName(column.name, err))?,
                    FivetranTableColumn {
                        data_type,
                        in_primary_key: column.primary_key,
                    },
                ))
            })
            .try_collect()?;
        Ok(FivetranTableSchema { name, columns })
    }
}

#[derive(PartialEq, Eq)]
enum Nullability {
    NonNullable,
    Nullable,
}

/// Generates a Convex validator matching the column in the Fivetran schema.
///
/// This does not return the only possible validator for this column, but it
/// will be the one that will be suggested if the user doesn’t have a
/// matching Convex schema.
fn suggested_validator(data_type: FivetranDataType, nullability: Nullability) -> Validator {
    // https://www.notion.so/convex-dev/Fivetran-Destination-Connector-Implementation-bc917ad7f68b483a93212d93dbbf7b0d?pvs=4#d9e675c1fe8b4c5bb54beb26b9f2b721
    let non_nullable_validator = match data_type {
        FivetranDataType::Unspecified => Validator::Any,
        FivetranDataType::Boolean => Validator::Boolean,
        FivetranDataType::Short => Validator::Float64,
        FivetranDataType::Int => Validator::Float64,
        FivetranDataType::Long => Validator::Int64,
        FivetranDataType::Decimal => Validator::String,
        FivetranDataType::Float => Validator::Float64,
        FivetranDataType::Double => Validator::Float64,
        FivetranDataType::NaiveDate => Validator::String,
        FivetranDataType::NaiveTime => Validator::String,
        FivetranDataType::NaiveDatetime => Validator::String,
        FivetranDataType::UtcDatetime => Validator::Float64,
        FivetranDataType::Binary => Validator::Bytes,
        FivetranDataType::Xml => Validator::String,
        FivetranDataType::String => Validator::String,
        FivetranDataType::Json => Validator::Object(ObjectValidator(BTreeMap::new())),
    };

    if nullability == Nullability::Nullable
        && data_type != FivetranDataType::Unspecified
        && data_type != FivetranDataType::Json
    {
        Validator::Union(vec![non_nullable_validator, Validator::Null])
    } else {
        non_nullable_validator
    }
}

impl FivetranTableSchema {
    fn suggested_convex_table(&self) -> anyhow::Result<TableDefinition, DestinationError> {
        let mut field_validators: BTreeMap<IdentifierFieldName, FieldValidator> = self
            .columns
            .iter()
            .filter(|(field_name, _)| !field_name.starts_with('_'))
            .map(|(field_name, column)| -> anyhow::Result<_, _> {
                let field_name = IdentifierFieldName::from_str(field_name).map_err(|err| {
                    DestinationError::UnsupportedColumnName(
                        field_name.clone(),
                        self.name.clone(),
                        err,
                    )
                })?;

                Ok((
                    field_name,
                    FieldValidator::required_field_type(suggested_validator(
                        column.data_type,
                        Nullability::Nullable,
                    )),
                ))
            })
            .try_collect()?;
        field_validators.insert(
            METADATA_CONVEX_FIELD_NAME.clone(),
            self.metadata_validator(),
        );
        let document_type = Some(DocumentSchema::Union(vec![ObjectValidator(
            field_validators,
        )]));

        let table_name = TableName::from_str(&self.name)
            .map_err(|err| DestinationError::UnsupportedTableName(self.name.to_string(), err))?;

        let indexes = self.suggested_indexes().map_err(|err| {
            DestinationError::IncorrectSchemaForTableWithoutSuggestion(table_name.clone(), err)
        })?;

        Ok(TableDefinition {
            table_name,
            document_type,
            indexes,
            search_indexes: Default::default(),
            vector_indexes: Default::default(),
        })
    }

    fn suggested_indexes(
        &self,
    ) -> anyhow::Result<BTreeMap<IndexDescriptor, IndexSchema>, TableSchemaError> {
        let indexes: Vec<IndexSchema> =
            vec![self.suggested_primary_key_index()?, self.sync_index()];

        Ok(indexes
            .into_iter()
            .map(|index| (index.index_descriptor.clone(), index))
            .collect())
    }

    fn suggested_primary_key_index(&self) -> anyhow::Result<IndexSchema, TableSchemaError> {
        let mut primary_key_index_fields: Vec<FieldPath> = vec![];
        if self.is_using_soft_deletes() {
            primary_key_index_fields.push(SOFT_DELETE_FIELD_PATH.clone());
        }

        // We are here suggesting to index the columns in lexicographic order. This is
        // not the only possible primary key index, as the columns in the primary key
        // can be placed in an arbitrary order.
        for (name, column) in self.columns.iter() {
            if column.in_primary_key {
                let field_path: FieldPath = name
                    .clone()
                    .try_into()
                    .map_err(|err| TableSchemaError::UnsupportedFieldName(name.clone(), err))?;
                primary_key_index_fields.push(field_path);
            }
        }

        primary_key_index_fields.push(CREATION_TIME_FIELD_PATH.clone());

        let fields = IndexedFields::try_from(primary_key_index_fields)
            .map_err(TableSchemaError::UnsupportedPrimaryKey)?;

        Ok(IndexSchema {
            index_descriptor: PRIMARY_KEY_INDEX_DESCRIPTOR.clone(),
            fields,
        })
    }

    fn sync_index(&self) -> IndexSchema {
        IndexSchema {
            index_descriptor: DEFAULT_FIVETRAN_SYNCED_INDEX_DESCRIPTOR.clone(),
            fields: if self.is_using_soft_deletes() {
                FIVETRAN_SYNC_INDEX_WITH_SOFT_DELETE_FIELDS.clone()
            } else {
                FIVETRAN_SYNC_INDEX_WITHOUT_SOFT_DELETE_FIELDS.clone()
            },
        }
    }

    /// Generates the only possible validator for the `fivetran` column of this
    /// table.
    ///
    /// The validator looks like:
    ///
    /// ```no_run
    /// fivetran: v.object({
    ///   synced: v.number(),
    ///   id: v.string(), // only if the table has no natural primary key
    ///   deleted: v.boolean(), // only if the table is using soft deletes
    /// }),
    /// ```
    ///
    /// See: https://github.com/fivetran/fivetran_sdk/blob/main/development-guide.md#system-columns
    fn metadata_validator(&self) -> FieldValidator {
        let mut fields = BTreeMap::new();

        fields.insert(
            SYNCED_CONVEX_FIELD_NAME.clone(),
            FieldValidator::required_field_type(Validator::Float64),
        );

        if let Some(column) = self.columns.get(&ID_FIVETRAN_FIELD_NAME) {
            fields.insert(
                ID_CONVEX_FIELD_NAME.clone(),
                FieldValidator::required_field_type(suggested_validator(
                    column.data_type,
                    Nullability::NonNullable,
                )),
            );
        }

        if self.columns.contains_key(&SOFT_DELETE_FIVETRAN_FIELD_NAME) {
            fields.insert(
                SOFT_DELETE_CONVEX_FIELD_NAME.clone(),
                FieldValidator::required_field_type(Validator::Boolean),
            );
        }

        FieldValidator::required_field_type(Validator::Object(ObjectValidator(fields)))
    }

    /// Validates that the columns in the Convex destination match the Fivetran
    /// schema.
    pub fn validate_destination_schema(
        &self,
        convex_table_schema: &DocumentSchema,
    ) -> Result<(), TableSchemaError> {
        // Ensure that there are no columns with forbidden names
        if self.columns.contains_key(
            &FivetranFieldName::from_str(&METADATA_CONVEX_FIELD_NAME)
                .expect("Expecting the name of the metadata field to also be valid in Fivetran"),
        ) {
            return Err(TableSchemaError::SourceTableHasFivetranField);
        }

        if let Some(forbidden_field_name) = self.columns.keys().find(|key| {
            (key.starts_with('_')).to_owned()
                && **key != *SOFT_DELETE_FIVETRAN_FIELD_NAME
                && **key != *ID_FIVETRAN_FIELD_NAME
                && **key != *SYNCED_FIVETRAN_FIELD_NAME
        }) {
            return Err(TableSchemaError::SourceContainsSystemFields(
                forbidden_field_name.to_owned(),
            ));
        }

        // Ensure that every destination column is in the source
        let DocumentSchema::Union(object_validator) = convex_table_schema else {
            return Err(TableSchemaError::DestinationHasAnySchema);
        };
        let [object_validator] = &object_validator[..] else {
            return Err(TableSchemaError::DestinationHasMultipleSchemas);
        };
        if let Some(missing_field) = object_validator.0.keys().find(|field_name| {
            let Ok(fivetran_field_name) = FivetranFieldName::from_str(&field_name.to_string())
            else {
                return false;
            };
            **field_name != *METADATA_CONVEX_FIELD_NAME
                && !self.columns.contains_key(&fivetran_field_name)
        }) {
            return Err(TableSchemaError::FieldMissingInSource(
                missing_field.clone(),
            ));
        }

        // Validate user columns
        for (fivetran_field_name, fivetran_column) in self
            .columns
            .iter()
            .filter(|(field_name, _)| !&field_name.starts_with('_'))
        {
            let convex_field_name: IdentifierFieldName =
                IdentifierFieldName::from_str(fivetran_field_name).map_err(|err| {
                    TableSchemaError::UnsupportedFieldName(fivetran_field_name.clone(), err)
                })?;
            let actual_validator = object_validator
                .0
                .get(&convex_field_name)
                .ok_or_else(|| TableSchemaError::MissingField {
                    field_name: fivetran_field_name.clone(),
                    suggested_validator: suggested_validator(
                        fivetran_column.data_type,
                        Nullability::Nullable,
                    ),
                })?
                .validator();

            let expected_validator =
                suggested_validator(fivetran_column.data_type, Nullability::NonNullable);
            let is_validator_valid = actual_validator == &expected_validator
                || actual_validator
                    == &Validator::Union(vec![Validator::Null, expected_validator.clone()])
                || actual_validator
                    == &Validator::Union(vec![expected_validator.clone(), Validator::Null]);
            if !is_validator_valid {
                return Err(TableSchemaError::NonmatchingFieldValidator {
                    field_name: fivetran_field_name.clone(),
                    actual_validator: actual_validator.clone(),
                    expected_validator,
                    fivetran_type: fivetran_column.data_type,
                });
            }
        }

        // Validate the metadata column
        let Some(actual_metadata_validator) =
            object_validator.0.get(&METADATA_CONVEX_FIELD_NAME.clone())
        else {
            return Err(TableSchemaError::MissingMetadataColumn {
                expected: self.metadata_validator(),
            });
        };

        let expected_metadata_validator = self.metadata_validator();
        if actual_metadata_validator != &expected_metadata_validator {
            return Err(TableSchemaError::IncorrectMetadataColumn {
                actual: actual_metadata_validator.clone(),
                expected: expected_metadata_validator,
            });
        }

        Ok(())
    }

    fn is_using_soft_deletes(&self) -> bool {
        self.columns.contains_key(&SOFT_DELETE_FIVETRAN_FIELD_NAME)
    }

    pub fn validate_destination_indexes(
        &self,
        indexes: &BTreeMap<IndexDescriptor, IndexSchema>,
    ) -> Result<(), TableSchemaError> {
        let indexes_targets: BTreeMap<IndexDescriptor, IndexedFields> = indexes
            .clone()
            .values()
            .map(|index| (index.index_descriptor.clone(), index.fields.clone()))
            .collect();

        // _fivetran_synced index
        let expected_index = if self.is_using_soft_deletes() {
            FIVETRAN_SYNC_INDEX_WITH_SOFT_DELETE_FIELDS.deref()
        } else {
            FIVETRAN_SYNC_INDEX_WITHOUT_SOFT_DELETE_FIELDS.deref()
        };

        if !indexes_targets
            .values()
            .any(|fields| fields == expected_index)
        {
            return Err(if self.is_using_soft_deletes() {
                TableSchemaError::MissingSyncIndexWithSoftDeletes
            } else {
                TableSchemaError::MissingSyncIndex
            });
        }

        // Primary key index
        let Some(primary_key_index_fields) = indexes_targets.get(&PRIMARY_KEY_INDEX_DESCRIPTOR)
        else {
            return Err(TableSchemaError::MissingPrimaryKeyIndex(SuggestedIndex(
                self.suggested_primary_key_index()?,
            )));
        };
        if !self.is_primary_key_index(primary_key_index_fields)? {
            return Err(TableSchemaError::WrongPrimaryKeyIndex(SuggestedIndex(
                self.suggested_primary_key_index()?,
            )));
        }

        Ok(())
    }

    /// Validates that a given index is a valid index for the Fivetran primary
    /// key.
    fn is_primary_key_index(
        &self,
        indexed_fields: &IndexedFields,
    ) -> anyhow::Result<bool, TableSchemaError> {
        let primary_key_columns: BTreeSet<FieldPath> = self
            .columns
            .iter()
            .filter(|(_, col)| col.in_primary_key)
            .map(|(name, _)| -> anyhow::Result<_, _> {
                let field_path: FieldPath = name
                    .clone()
                    .try_into()
                    .map_err(|err| TableSchemaError::UnsupportedFieldName(name.clone(), err))?;
                Ok(field_path)
            })
            .try_collect()?;

        let fields = indexed_fields.deref();
        if self.is_using_soft_deletes() {
            // The index must start with _fivetran_deleted
            let Some(first_field) = fields.first() else {
                return Ok(false);
            };
            if first_field != SOFT_DELETE_FIELD_PATH.deref() {
                return Ok(false);
            }
        }

        let fields_to_compare: BTreeSet<FieldPath> = fields
            .iter()
            .filter(|path| *path != &*CREATION_TIME_FIELD_PATH)
            .skip(if self.is_using_soft_deletes() { 1 } else { 0 })
            .cloned()
            .collect();
        Ok(fields_to_compare == primary_key_columns)
    }
}

/// Validates that the table in the Convex schema is compatible with the source
/// Fivetran table.
///
/// For the same Fivetran table, there can be multiple valid Convex schemas. For
/// instance, the fields in the primary key index can be in an arbitrary order.
/// Also, fields in Convex can either be nullable (e.g. `v.union(v.string(),
/// v.null())`) or not (e.g. `v.string()`).
#[allow(dead_code)]
pub fn validate_destination_schema_table(
    fivetran_table: fivetran_sdk::Table,
    convex_table: &TableDefinition,
) -> Result<(), DestinationError> {
    let fivetran_table_name = fivetran_table.name.clone();
    let table_name = TableName::from_str(&fivetran_table_name)
        .map_err(|err| DestinationError::UnsupportedTableName(fivetran_table_name, err))?;

    let fivetran_table_schema = FivetranTableSchema::try_from(fivetran_table)?;

    let Some(convex_table_schema) = &convex_table.document_type else {
        return Err(DestinationError::MissingTable(
            table_name,
            SuggestedTable(fivetran_table_schema.suggested_convex_table()?),
        ));
    };

    fivetran_table_schema
        .validate_destination_schema(convex_table_schema)
        .map_err(|err| {
            fivetran_table_schema
                .suggested_convex_table()
                .map(|suggested_table| {
                    DestinationError::IncorrectSchemaForTable(
                        table_name.clone(),
                        err,
                        SuggestedTable(suggested_table),
                    )
                })
                .unwrap_or_else(|e| e)
        })?;

    fivetran_table_schema
        .validate_destination_indexes(&convex_table.indexes)
        .map_err(|err| {
            fivetran_table_schema
                .suggested_convex_table()
                .map(|suggested_table| {
                    DestinationError::IncorrectSchemaForTable(
                        table_name.clone(),
                        err,
                        SuggestedTable(suggested_table),
                    )
                })
                .unwrap_or_else(|e| e)
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{
            BTreeMap,
            BTreeSet,
            HashSet,
        },
        str::FromStr,
    };

    use common::{
        bootstrap_model::index::database_index::IndexedFields,
        document::CREATION_TIME_FIELD_PATH,
        object_validator,
        schemas::{
            validator::{
                FieldValidator,
                ObjectValidator,
                Validator,
            },
            DocumentSchema,
            IndexSchema,
            TableDefinition,
        },
        types::IndexDescriptor,
        value::{
            FieldPath,
            IdentifierFieldName,
            TableName,
        },
    };
    use convex_fivetran_common::fivetran_sdk::{
        self,
        Column,
        DataType as FivetranDataType,
        Table,
    };
    use maplit::{
        btreemap,
        btreeset,
        hashset,
    };
    use must_let::must_let;
    use proptest::prelude::*;

    use super::{
        validate_destination_schema_table,
        FivetranTableColumn,
        FivetranTableSchema,
    };
    use crate::{
        error::DestinationError,
        testing::fivetran_table_strategy,
    };

    fn fivetran_table(
        columns: BTreeMap<&str, FivetranDataType>,
        primary_key_columns: HashSet<&str>,
    ) -> fivetran_sdk::Table {
        for col_name in &primary_key_columns {
            if !columns.contains_key(col_name) {
                panic!("Unknown column `{}` in the primary key", col_name);
            }
        }

        Table {
            name: "my_table".into(),
            columns: columns
                .into_iter()
                .map(|(col_name, col_type)| Column {
                    name: col_name.into(),
                    r#type: col_type as i32,
                    primary_key: primary_key_columns.contains(col_name),
                    decimal: None,
                })
                .collect(),
        }
    }

    fn fivetran_table_schema(
        columns: BTreeMap<&str, FivetranDataType>,
        primary_key_columns: BTreeSet<&str>,
    ) -> FivetranTableSchema {
        FivetranTableSchema {
            name: "my_table".parse().unwrap(),
            columns: columns
                .into_iter()
                .map(|(name, data_type)| {
                    (
                        name.parse().unwrap(),
                        FivetranTableColumn {
                            data_type,
                            in_primary_key: primary_key_columns.contains(name),
                        },
                    )
                })
                .collect(),
        }
    }

    fn convex_table(
        fields: BTreeMap<&str, FieldValidator>,
        indexes: BTreeMap<&str, Vec<FieldPath>>,
    ) -> TableDefinition {
        TableDefinition {
            table_name: TableName::from_str("table_name").unwrap(),
            search_indexes: Default::default(),
            vector_indexes: Default::default(),
            document_type: Some(DocumentSchema::Union(vec![ObjectValidator(
                fields
                    .into_iter()
                    .map(|(field_name, field_validator)| {
                        (
                            IdentifierFieldName::from_str(field_name).unwrap(),
                            field_validator,
                        )
                    })
                    .collect(),
            )])),
            indexes: convex_indexes(indexes),
        }
    }

    fn convex_indexes(
        indexes: BTreeMap<&str, Vec<FieldPath>>,
    ) -> BTreeMap<IndexDescriptor, IndexSchema> {
        indexes
            .into_iter()
            .map(|(index_name, index_fields)| {
                let index_descriptor = IndexDescriptor::from_str(index_name).unwrap();
                (
                    index_descriptor.clone(),
                    IndexSchema {
                        index_descriptor,
                        fields: IndexedFields::try_from(index_fields).unwrap(),
                    },
                )
            })
            .collect()
    }

    #[test]
    fn it_allows_correct_convex_tables() -> anyhow::Result<()> {
        validate_destination_schema_table(
            fivetran_table(
                btreemap! {
                    "id" => FivetranDataType::Long,
                    "_fivetran_synced" => FivetranDataType::UtcDatetime,
                },
                hashset! {"id"},
            ),
            &convex_table(
                btreemap! {
                    "id" => FieldValidator::required_field_type(Validator::Union(vec![
                        Validator::Null,
                        Validator::Int64,
                    ])),
                    "fivetran" => FieldValidator::required_field_type(Validator::Object(
                        object_validator!(
                            "synced" => FieldValidator::required_field_type(Validator::Float64),
                        ),
                    )),
                },
                btreemap! {
                    "by_primary_key" => vec![
                        FieldPath::new(vec![
                            IdentifierFieldName::from_str("id")?,
                        ])?,
                        CREATION_TIME_FIELD_PATH.clone(),
                    ],
                    "sync_index" => vec![
                        FieldPath::new(vec![
                            IdentifierFieldName::from_str("fivetran")?,
                            IdentifierFieldName::from_str("synced")?,
                        ])?,
                        CREATION_TIME_FIELD_PATH.clone(),
                    ],
                },
            ),
        )
        .unwrap();
        Ok(())
    }

    #[test]
    fn it_errors_when_a_field_has_an_incorrect_type() -> anyhow::Result<()> {
        must_let!(
            let Err(
                DestinationError::IncorrectSchemaForTable(_, _, _)
            ) = validate_destination_schema_table(
                fivetran_table(
                    btreemap! {
                        "id" => FivetranDataType::Long,
                        "_fivetran_synced" => FivetranDataType::UtcDatetime,
                    },
                    hashset! {"id"},
                ),
                &convex_table(
                    btreemap! {
                        "id" => FieldValidator::required_field_type(Validator::Union(vec![
                            Validator::Null,
                            Validator::Float64, // incorrect
                        ])),
                        "fivetran" => FieldValidator::required_field_type(Validator::Object(
                            object_validator!(
                                "synced" => FieldValidator::required_field_type(Validator::Float64),
                            ),
                        )),
                    },
                    btreemap! {
                        "by_primary_key" => vec![
                            FieldPath::new(vec![
                                IdentifierFieldName::from_str("id")?,
                            ])?,
                            CREATION_TIME_FIELD_PATH.clone(),
                        ],
                        "sync_index" => vec![
                            FieldPath::new(vec![
                                IdentifierFieldName::from_str("fivetran")?,
                                IdentifierFieldName::from_str("synced")?,
                            ])?,
                            CREATION_TIME_FIELD_PATH.clone(),
                        ],
                    },
                ),
            )
        );
        Ok(())
    }

    #[test]
    fn it_allows_convex_tables_when_a_field_isnt_nullable_in_convex() -> anyhow::Result<()> {
        validate_destination_schema_table(
            fivetran_table(
                btreemap! {
                    "id" => FivetranDataType::Long,
                    "_fivetran_synced" => FivetranDataType::UtcDatetime,
                },
                hashset! {"id"},
            ),
            &convex_table(
                btreemap! {
                    "id" => FieldValidator::required_field_type(Validator::Int64),
                    "fivetran" => FieldValidator::required_field_type(Validator::Object(
                        object_validator!(
                            "synced" => FieldValidator::required_field_type(Validator::Float64),
                        ),
                    )),
                },
                btreemap! {
                    "by_primary_key" => vec![
                        FieldPath::new(vec![
                            IdentifierFieldName::from_str("id")?,
                        ])?,
                        CREATION_TIME_FIELD_PATH.clone(),
                    ],
                    "sync_index" => vec![
                        FieldPath::new(vec![
                            IdentifierFieldName::from_str("fivetran")?,
                            IdentifierFieldName::from_str("synced")?,
                        ])?,
                        CREATION_TIME_FIELD_PATH.clone(),
                    ],
                },
            ),
        )
        .unwrap();
        Ok(())
    }

    #[test]
    fn it_allows_convex_tables_with_optional_fivetran_system_columns() -> anyhow::Result<()> {
        validate_destination_schema_table(
            fivetran_table(
                btreemap! {
                    "name" => FivetranDataType::String,
                    "_fivetran_synced" => FivetranDataType::UtcDatetime,
                    "_fivetran_id" => FivetranDataType::String,
                    "_fivetran_deleted" => FivetranDataType::Boolean,
                },
                hashset! {"_fivetran_id"},
            ),
            &convex_table(
                btreemap! {
                    "name" => FieldValidator::required_field_type(Validator::String),
                    "fivetran" => FieldValidator::required_field_type(Validator::Object(
                        object_validator!(
                            "synced" => FieldValidator::required_field_type(Validator::Float64),
                            "id" => FieldValidator::required_field_type(Validator::String),
                            "deleted" => FieldValidator::required_field_type(Validator::Boolean),
                        ),
                    )),
                },
                btreemap! {
                    "by_primary_key" => vec![
                        FieldPath::new(vec![
                            IdentifierFieldName::from_str("fivetran")?,
                            IdentifierFieldName::from_str("deleted")?,
                        ])?,
                        FieldPath::new(vec![
                            IdentifierFieldName::from_str("fivetran")?,
                            IdentifierFieldName::from_str("id")?,
                        ])?,
                        CREATION_TIME_FIELD_PATH.clone(),
                    ],
                    "sync_index" => vec![
                        FieldPath::new(vec![
                            IdentifierFieldName::from_str("fivetran")?,
                            IdentifierFieldName::from_str("deleted")?,
                        ])?,
                        FieldPath::new(vec![
                            IdentifierFieldName::from_str("fivetran")?,
                            IdentifierFieldName::from_str("synced")?,
                        ])?,
                        CREATION_TIME_FIELD_PATH.clone(),
                    ],
                },
            ),
        )
        .unwrap();
        Ok(())
    }

    #[test]
    fn it_allows_convex_tables_with_multiple_columns_in_the_primary_key() -> anyhow::Result<()> {
        validate_destination_schema_table(
            fivetran_table(
                btreemap! {
                    "a" => FivetranDataType::String,
                    "b" => FivetranDataType::String,
                    "c" => FivetranDataType::String,
                    "_fivetran_deleted" => FivetranDataType::Boolean,
                    "_fivetran_synced" => FivetranDataType::UtcDatetime,
                },
                hashset! {"a", "b", "c"},
            ),
            &convex_table(
                btreemap! {
                    "a" => FieldValidator::required_field_type(Validator::String),
                    "b" => FieldValidator::required_field_type(Validator::String),
                    "c" => FieldValidator::required_field_type(Validator::String),
                    "fivetran" => FieldValidator::required_field_type(Validator::Object(
                        object_validator!(
                            "synced" => FieldValidator::required_field_type(Validator::Float64),
                            "deleted" => FieldValidator::required_field_type(Validator::Boolean),
                        ),
                    )),
                },
                btreemap! {
                    "by_primary_key" => vec![
                        // _fivetran_deleted must be the first field in the index
                        FieldPath::new(vec![
                            IdentifierFieldName::from_str("fivetran")?,
                            IdentifierFieldName::from_str("deleted")?,
                        ])?,

                        // The other fields can be in an arbitrary order
                        FieldPath::new(vec![IdentifierFieldName::from_str("b")?])?,
                        FieldPath::new(vec![IdentifierFieldName::from_str("a")?])?,
                        FieldPath::new(vec![IdentifierFieldName::from_str("c")?])?,

                        CREATION_TIME_FIELD_PATH.clone(),
                    ],
                    "sync_index_named_arbitrarily" => vec![
                        FieldPath::new(vec![
                            IdentifierFieldName::from_str("fivetran")?,
                            IdentifierFieldName::from_str("deleted")?,
                        ])?,
                        FieldPath::new(vec![
                            IdentifierFieldName::from_str("fivetran")?,
                            IdentifierFieldName::from_str("synced")?,
                        ])?,
                        CREATION_TIME_FIELD_PATH.clone(),
                    ],
                },
            ),
        )
        .unwrap();
        Ok(())
    }

    #[test]
    fn it_requires_two_system_indexes() -> anyhow::Result<()> {
        assert!(fivetran_table_schema(
            btreemap! {
                "id" => FivetranDataType::Long,
                "_fivetran_synced" => FivetranDataType::UtcDatetime,
            },
            btreeset! {"id"},
        )
        .validate_destination_indexes(&convex_indexes(btreemap! {
            "by_primary_key" => vec![
                FieldPath::new(vec![IdentifierFieldName::from_str("id")?])?,
                CREATION_TIME_FIELD_PATH.clone(),
            ],
            "my_sync_index" => vec![
                FieldPath::new(vec![
                    IdentifierFieldName::from_str("fivetran")?,
                    IdentifierFieldName::from_str("synced")?,
                ])?,
                CREATION_TIME_FIELD_PATH.clone(),
            ],
        }))
        .is_ok());
        Ok(())
    }

    #[test]
    fn it_fails_if_a_required_index_is_missing() -> anyhow::Result<()> {
        let table_schema = fivetran_table_schema(
            btreemap! {
                "id" => FivetranDataType::Long,
                "_fivetran_synced" => FivetranDataType::UtcDatetime,
            },
            btreeset! {"id"},
        );

        let primary_key_index = vec![
            FieldPath::new(vec![IdentifierFieldName::from_str("id")?])?,
            CREATION_TIME_FIELD_PATH.clone(),
        ];
        let sync_index = vec![
            FieldPath::new(vec![
                IdentifierFieldName::from_str("fivetran")?,
                IdentifierFieldName::from_str("synced")?,
            ])?,
            CREATION_TIME_FIELD_PATH.clone(),
        ];

        assert!(table_schema
            .validate_destination_indexes(&convex_indexes(btreemap! {}))
            .is_err());
        assert!(table_schema
            .validate_destination_indexes(&convex_indexes(btreemap! {
                "by_primary_key" => primary_key_index,
            }))
            .is_err());
        assert!(table_schema
            .validate_destination_indexes(&convex_indexes(btreemap! {
                "my_sync_index" => sync_index,
            }))
            .is_err());
        Ok(())
    }

    #[test]
    fn required_indexes_include_the_soft_delete_field_if_it_exists() -> anyhow::Result<()> {
        assert!(fivetran_table_schema(
            btreemap! {
                "id" => FivetranDataType::Long,
                "_fivetran_synced" => FivetranDataType::UtcDatetime,
                "_fivetran_deleted" => FivetranDataType::Boolean,
            },
            btreeset! {"id"}
        )
        .validate_destination_indexes(&convex_indexes(btreemap! {
            "by_primary_key" => vec![
                FieldPath::new(vec![
                    IdentifierFieldName::from_str("fivetran")?,
                    IdentifierFieldName::from_str("deleted")?,
                ])?,
                FieldPath::new(vec![IdentifierFieldName::from_str("id")?])?,
                CREATION_TIME_FIELD_PATH.clone(),
            ],
            "my_sync_index" => vec![
                FieldPath::new(vec![
                    IdentifierFieldName::from_str("fivetran")?,
                    IdentifierFieldName::from_str("deleted")?,
                ])?,
                FieldPath::new(vec![
                    IdentifierFieldName::from_str("fivetran")?,
                    IdentifierFieldName::from_str("synced")?,
                ])?,
                CREATION_TIME_FIELD_PATH.clone(),
            ],
        }))
        .is_ok());

        // The soft delete field must come before the other fields
        assert!(fivetran_table_schema(
            btreemap! {
                "id" => FivetranDataType::Long,
                "_fivetran_synced" => FivetranDataType::UtcDatetime,
                "_fivetran_deleted" => FivetranDataType::Boolean,
            },
            btreeset! {"id"}
        )
        .validate_destination_indexes(&convex_indexes(btreemap! {
            "by_primary_key" => vec![
                FieldPath::new(vec![
                    IdentifierFieldName::from_str("fivetran")?,
                    IdentifierFieldName::from_str("deleted")?,
                ])?,
                FieldPath::new(vec![IdentifierFieldName::from_str("id")?])?,
                CREATION_TIME_FIELD_PATH.clone(),
            ],
            "my_sync_index" => vec![
                // Wrong
                FieldPath::new(vec![
                    IdentifierFieldName::from_str("fivetran")?,
                    IdentifierFieldName::from_str("synced")?,
                ])?,
                FieldPath::new(vec![
                    IdentifierFieldName::from_str("fivetran")?,
                    IdentifierFieldName::from_str("deleted")?,
                ])?,
                CREATION_TIME_FIELD_PATH.clone(),
            ],
        }))
        .is_err());

        Ok(())
    }

    #[test]
    fn primary_key_columns_can_be_in_an_arbitrary_order_in_the_index() -> anyhow::Result<()> {
        let fivetran_table_schema = fivetran_table_schema(
            btreemap! {
                "a" => FivetranDataType::Long,
                "b" => FivetranDataType::Long,
                "c" => FivetranDataType::Long,
                "_fivetran_synced" => FivetranDataType::UtcDatetime,
                "_fivetran_deleted" => FivetranDataType::Boolean,
            },
            btreeset! {"a", "b", "c"},
        );

        let sync_index = vec![
            FieldPath::new(vec![
                IdentifierFieldName::from_str("fivetran")?,
                IdentifierFieldName::from_str("deleted")?,
            ])?,
            FieldPath::new(vec![
                IdentifierFieldName::from_str("fivetran")?,
                IdentifierFieldName::from_str("synced")?,
            ])?,
            CREATION_TIME_FIELD_PATH.clone(),
        ];

        assert!(fivetran_table_schema
            .validate_destination_indexes(&convex_indexes(btreemap! {
                "by_primary_key" => vec![
                    FieldPath::new(vec![
                        IdentifierFieldName::from_str("fivetran")?,
                        IdentifierFieldName::from_str("deleted")?,
                    ])?,
                    FieldPath::new(vec![IdentifierFieldName::from_str("b")?])?,
                    FieldPath::new(vec![IdentifierFieldName::from_str("a")?])?,
                    FieldPath::new(vec![IdentifierFieldName::from_str("c")?])?,
                    CREATION_TIME_FIELD_PATH.clone(),
                ],
                "my_sync_index" => sync_index.clone(),
            }))
            .is_ok());

        assert!(fivetran_table_schema
            .validate_destination_indexes(&convex_indexes(btreemap! {
                "by_primary_key" => vec![
                    FieldPath::new(vec![
                        IdentifierFieldName::from_str("fivetran")?,
                        IdentifierFieldName::from_str("deleted")?,
                    ])?,
                    FieldPath::new(vec![IdentifierFieldName::from_str("c")?])?,
                    FieldPath::new(vec![IdentifierFieldName::from_str("b")?])?,
                    FieldPath::new(vec![IdentifierFieldName::from_str("a")?])?,
                ],
                "my_sync_index" => sync_index.clone(),
            }))
            .is_ok());

        // The _fivetran_deleted field must be first
        assert!(fivetran_table_schema
            .validate_destination_indexes(&convex_indexes(btreemap! {
                "by_primary_key" => vec![
                    FieldPath::new(vec![IdentifierFieldName::from_str("c")?])?,
                    FieldPath::new(vec![IdentifierFieldName::from_str("b")?])?,
                    FieldPath::new(vec![IdentifierFieldName::from_str("a")?])?,
                    // Error
                    FieldPath::new(vec![
                        IdentifierFieldName::from_str("fivetran")?,
                        IdentifierFieldName::from_str("deleted")?,
                    ])?,
                    CREATION_TIME_FIELD_PATH.clone(),
                ],
                "my_sync_index" => sync_index.clone(),
            }))
            .is_err());

        Ok(())
    }

    #[test]
    fn it_suggests_convex_tables() -> anyhow::Result<()> {
        let fivetran_table = fivetran_table_schema(
            btreemap! {
                "name" => FivetranDataType::String,
                "slug" => FivetranDataType::String,
                "_fivetran_synced" => FivetranDataType::UtcDatetime,
                "_fivetran_deleted" => FivetranDataType::Boolean,
            },
            btreeset! {"slug"},
        );

        assert_eq!(
            fivetran_table.suggested_convex_table()?,
            TableDefinition {
                table_name: "my_table".parse()?,
                indexes: btreemap! {
                    "by_fivetran_synced".parse()? => IndexSchema {
                        index_descriptor: "by_fivetran_synced".parse()?,
                        fields: vec![
                            "fivetran.deleted".parse()?,
                            "fivetran.synced".parse()?,
                            "_creationTime".parse()?,
                        ].try_into()?
                    },
                    "by_primary_key".parse()? => IndexSchema {
                        index_descriptor: "by_primary_key".parse()?,
                        fields: vec![
                            "fivetran.deleted".parse()?,
                            "slug".parse()?,
                            "_creationTime".parse()?,
                        ].try_into()?
                    }
                },
                document_type: Some(DocumentSchema::Union(vec![object_validator!(
                    "name" => FieldValidator::required_field_type(Validator::Union(vec![
                        Validator::String,
                        Validator::Null,
                    ])),
                    "slug" => FieldValidator::required_field_type(Validator::Union(vec![
                        Validator::String,
                        Validator::Null,
                    ])),
                    "fivetran" => FieldValidator::required_field_type(Validator::Object(object_validator!(
                        "synced" => FieldValidator::required_field_type(Validator::Float64),
                        "deleted" => FieldValidator::required_field_type(Validator::Boolean),
                    ))),
                )])),
                search_indexes: Default::default(),
                vector_indexes: Default::default(),
            },
        );
        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            failure_persistence: None, ..ProptestConfig::default()
        })]
        #[test]
        fn suggested_convex_schemas_are_always_valid(fivetran_table in fivetran_table_strategy()) {
            let Ok(schema) = TryInto::<FivetranTableSchema>::try_into(fivetran_table.clone()) else {
                return Err(TestCaseError::Fail("Invalid Fivetran schema".into()));
            };

            let Ok(suggested_convex_table) = schema.suggested_convex_table() else {
                return Err(TestCaseError::Reject("Unsupported Fivetran schema".into()));
            };

            prop_assert!(
                validate_destination_schema_table(fivetran_table, &suggested_convex_table).is_ok()
            );
        }
    }
}
