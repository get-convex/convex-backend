use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    ops::Deref,
    str::FromStr,
};

use anyhow::bail;
use common::{
    bootstrap_model::index::database_index::IndexedFields,
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
};
use convex_fivetran_destination::{
    api_types::{
        FivetranFieldName,
        FivetranTableName,
    },
    constants::{
        FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR,
        FIVETRAN_SYNCED_INDEX_DESCRIPTOR,
        FIVETRAN_SYNC_INDEX_WITHOUT_SOFT_DELETE_FIELDS,
        FIVETRAN_SYNC_INDEX_WITH_SOFT_DELETE_FIELDS,
        ID_CONVEX_FIELD_NAME,
        ID_FIVETRAN_FIELD_NAME,
        METADATA_CONVEX_FIELD_NAME,
        SOFT_DELETE_CONVEX_FIELD_NAME,
        SOFT_DELETE_FIELD_PATH,
        SOFT_DELETE_FIVETRAN_FIELD_NAME,
        SYNCED_CONVEX_FIELD_NAME,
        SYNCED_FIVETRAN_FIELD_NAME,
        UNDERSCORED_COLUMNS_CONVEX_FIELD_NAME,
    },
};

use crate::{
    error::{
        DestinationError,
        MetadataFieldError,
        SuggestedIndex,
        SuggestedTable,
        TableSchemaError,
    },
    log,
};

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
        let table_name: FivetranTableName = table
            .name
            .parse()
            .map_err(|err| DestinationError::InvalidTableName(table.name, err))?;

        let columns = table
            .columns
            .into_iter()
            .map(|column| -> Result<_, _> {
                let data_type = column.r#type();
                Ok((
                    column.name.parse().map_err(|err| {
                        DestinationError::InvalidColumnName(column.name, table_name.clone(), err)
                    })?,
                    FivetranTableColumn {
                        data_type,
                        in_primary_key: column.primary_key,
                    },
                ))
            })
            .try_collect()?;
        Ok(FivetranTableSchema {
            name: table_name,
            columns,
        })
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

pub fn suggested_convex_table(
    table: fivetran_sdk::Table,
) -> Result<TableDefinition, DestinationError> {
    let schema = FivetranTableSchema::try_from(table)?;
    schema.suggested_convex_table()
}

impl FivetranTableSchema {
    fn suggested_convex_table(&self) -> anyhow::Result<TableDefinition, DestinationError> {
        let mut field_validators: BTreeMap<IdentifierFieldName, FieldValidator> = self
            .columns
            .iter()
            .filter(|(field_name, _)| {
                !field_name.is_fivetran_system_field() && !field_name.is_underscored_field()
            })
            .map(|(field_name, column)| -> anyhow::Result<_, _> {
                let field_name = field_name.parse().map_err(|err| {
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
            self.suggested_metadata_validator(),
        );

        let document_type = Some(DocumentSchema::Union(vec![ObjectValidator(
            field_validators,
        )]));

        let table_name: TableName = self
            .name
            .parse()
            .map_err(|err| DestinationError::UnsupportedTableName(self.name.to_string(), err))?;

        let indexes = self.suggested_indexes().map_err(|err| {
            DestinationError::IncorrectSchemaForTableWithoutSuggestion(table_name.clone(), err)
        })?;

        Ok(TableDefinition {
            table_name,
            document_type,
            indexes,
            staged_db_indexes: Default::default(),
            text_indexes: Default::default(),
            staged_text_indexes: Default::default(),
            vector_indexes: Default::default(),
            staged_vector_indexes: Default::default(),
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

        let fields = IndexedFields::try_from(primary_key_index_fields)
            .map_err(TableSchemaError::UnsupportedPrimaryKey)?;

        Ok(IndexSchema {
            index_descriptor: FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.clone(),
            fields,
        })
    }

    fn sync_index(&self) -> IndexSchema {
        IndexSchema {
            index_descriptor: FIVETRAN_SYNCED_INDEX_DESCRIPTOR.clone(),
            fields: if self.is_using_soft_deletes() {
                FIVETRAN_SYNC_INDEX_WITH_SOFT_DELETE_FIELDS.clone()
            } else {
                FIVETRAN_SYNC_INDEX_WITHOUT_SOFT_DELETE_FIELDS.clone()
            },
        }
    }

    /// Generates the recommended validator for the `fivetran` column of this
    /// table.
    ///
    /// The validator looks like:
    ///
    /// ```no_run
    /// fivetran: v.object({
    ///   synced: v.number(),
    ///   id: v.string(), // only if the table has no natural primary key
    ///   deleted: v.boolean(), // only if the table is using soft deletes
    ///   columns: v.object({ // only if the (for instance `_field`)
    ///     field: v.union(v.string(), v.null()), // (for instance)
    ///   }),
    /// }),
    /// ```
    ///
    /// See: https://github.com/fivetran/fivetran_sdk/blob/main/development-guide.md#system-columns
    fn suggested_metadata_validator(&self) -> FieldValidator {
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

        let underscored_fields: BTreeMap<_, _> = self
            .columns
            .iter()
            .filter(|(name, _)| name.is_underscored_field())
            .flat_map(|(name, column)| {
                name[1..].parse().ok().map(|name| {
                    (
                        name,
                        FieldValidator::required_field_type(suggested_validator(
                            column.data_type,
                            Nullability::Nullable,
                        )),
                    )
                })
            })
            .collect();
        if !underscored_fields.is_empty() {
            fields.insert(
                UNDERSCORED_COLUMNS_CONVEX_FIELD_NAME.clone(),
                FieldValidator::required_field_type(Validator::Object(ObjectValidator(
                    underscored_fields,
                ))),
            );
        }

        FieldValidator::required_field_type(Validator::Object(ObjectValidator(fields)))
    }

    fn validate_metadata_validator(
        &self,
        metadata_validator: &FieldValidator,
    ) -> Result<(), MetadataFieldError> {
        let Validator::Object(metadata_validator) = metadata_validator.validator() else {
            return Err(MetadataFieldError::InvalidMetadataFieldType);
        };

        // Synced
        let expected_synced_validator =
            Some(FieldValidator::required_field_type(Validator::Float64));
        if metadata_validator.0.get(SYNCED_CONVEX_FIELD_NAME.deref())
            != expected_synced_validator.as_ref()
        {
            return Err(MetadataFieldError::InvalidSyncedField);
        }

        // Fivetran ID
        let expected_id_validator = self.columns.get(&ID_FIVETRAN_FIELD_NAME).map(|column| {
            FieldValidator::required_field_type(suggested_validator(
                column.data_type,
                Nullability::NonNullable,
            ))
        });
        if metadata_validator.0.get(ID_CONVEX_FIELD_NAME.deref()) != expected_id_validator.as_ref()
        {
            return Err(MetadataFieldError::InvalidIdField);
        }

        // Soft delete
        let expected_soft_delete_validator = self
            .columns
            .contains_key(&SOFT_DELETE_FIVETRAN_FIELD_NAME)
            .then_some(FieldValidator::required_field_type(Validator::Boolean));
        if metadata_validator
            .0
            .get(SOFT_DELETE_CONVEX_FIELD_NAME.deref())
            != expected_soft_delete_validator.as_ref()
        {
            return Err(MetadataFieldError::InvalidDeletedField);
        }

        // `fivetran.columns` in the Convex schema only contains existing columns
        for metadata_column_name in column_names_in_metadata(metadata_validator)? {
            if !self.columns.contains_key(&metadata_column_name) {
                return Err(MetadataFieldError::ColumnInMetadataNotInDataSource(
                    metadata_column_name,
                ));
            }
        }

        // All non-system columns starting by _ in the Fivetran table exist in the
        // Convex schema with a type matching their original type
        let underscored_columns = self
            .columns
            .iter()
            .filter(|(field_name, _)| field_name.is_underscored_field());
        for (field_name, column) in underscored_columns {
            let Some(columns_validator) = metadata_validator
                .0
                .get(UNDERSCORED_COLUMNS_CONVEX_FIELD_NAME.deref())
            else {
                return Err(MetadataFieldError::MissingColumnsField(field_name.clone()));
            };

            let Validator::Object(columns_validator) = columns_validator.validator() else {
                return Err(MetadataFieldError::InvalidColumnsFieldType);
            };

            let actual_validator = columns_validator
                .0
                .get(&field_name[1..])
                .ok_or_else(|| MetadataFieldError::MissingFieldInColumns(field_name.clone()))?
                .validator();
            if !is_field_validator_valid(actual_validator, column.data_type) {
                return Err(MetadataFieldError::IncorrectColumnSpecification {
                    field_name: field_name.clone(),
                    actual_validator: actual_validator.clone(),
                    expected_validator: suggested_validator(
                        column.data_type,
                        Nullability::NonNullable,
                    ),
                });
            }
        }

        Ok(())
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
        for (fivetran_field_name, fivetran_column) in
            self.columns.iter().filter(|(field_name, _)| {
                !field_name.is_fivetran_system_field() && !field_name.is_underscored_field()
            })
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

            if !is_field_validator_valid(actual_validator, fivetran_column.data_type) {
                return Err(TableSchemaError::NonmatchingFieldValidator {
                    field_name: fivetran_field_name.clone(),
                    actual_validator: actual_validator.clone(),
                    expected_validator: suggested_validator(
                        fivetran_column.data_type,
                        Nullability::Nullable,
                    ),
                    fivetran_type: fivetran_column.data_type,
                });
            }
        }

        // Validate the metadata column
        let Some(metadata_validator) = object_validator.0.get(&METADATA_CONVEX_FIELD_NAME.clone())
        else {
            return Err(TableSchemaError::MissingMetadataColumn {
                suggested: self.suggested_metadata_validator(),
            });
        };

        self.validate_metadata_validator(metadata_validator)
            .map_err(|error| TableSchemaError::IncorrectMetadataColumn {
                error,
                actual: metadata_validator.clone(),
                suggested: self.suggested_metadata_validator(),
            })?;

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
        let Some(primary_key_index_fields) =
            indexes_targets.get(&FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR)
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
            .skip(if self.is_using_soft_deletes() { 1 } else { 0 })
            .cloned()
            .collect();
        Ok(fields_to_compare == primary_key_columns)
    }

    pub fn to_convex_table(&self) -> anyhow::Result<TableDefinition> {
        let table_name: TableName = self.name.parse()?;
        let mut object_schema = ObjectValidator(BTreeMap::new());
        let mut metadata_object_schema = ObjectValidator(BTreeMap::new());
        let mut underscored_columns_object_schema = ObjectValidator(BTreeMap::new());
        for (field_name, column) in self.columns.iter() {
            // Handle system columns
            // Soft delete
            if field_name == &*SOFT_DELETE_FIVETRAN_FIELD_NAME {
                metadata_object_schema.0.insert(
                    SOFT_DELETE_CONVEX_FIELD_NAME.clone(),
                    FieldValidator::optional_field_type(Validator::Boolean),
                );
            }
            // Fivetran pseudo-ID
            else if field_name == &*ID_FIVETRAN_FIELD_NAME {
                metadata_object_schema.0.insert(
                    ID_CONVEX_FIELD_NAME.clone(),
                    FieldValidator::optional_field_type(Validator::String),
                );
            }
            // Synchronization timestamp
            else if field_name == &*SYNCED_FIVETRAN_FIELD_NAME {
                metadata_object_schema.0.insert(
                    SYNCED_CONVEX_FIELD_NAME.clone(),
                    FieldValidator::optional_field_type(Validator::Float64),
                );
            }
            // Columns having a Fivetran name starting by _
            else if let Some(field_name) = field_name.strip_prefix('_') {
                let field_name = field_name.parse()?;
                let column_type = column.data_type;
                let field_validator =
                    FieldValidator::optional_field_type(recognize_convex_type(&column_type)?);
                underscored_columns_object_schema
                    .0
                    .insert(field_name, field_validator);
            }
            // User columns
            else {
                let field_name = field_name.parse()?;
                let column_type = column.data_type;
                let field_validator =
                    FieldValidator::optional_field_type(recognize_convex_type(&column_type)?);
                object_schema.0.insert(field_name, field_validator);
            }
        }

        metadata_object_schema.0.insert(
            UNDERSCORED_COLUMNS_CONVEX_FIELD_NAME.clone(),
            FieldValidator::required_field_type(Validator::Object(
                underscored_columns_object_schema,
            )),
        );
        object_schema.0.insert(
            METADATA_CONVEX_FIELD_NAME.clone(),
            FieldValidator::required_field_type(Validator::Object(metadata_object_schema)),
        );

        let indexes = self.suggested_indexes()?;
        let document_schema = DocumentSchema::Union(vec![object_schema]);

        Ok(TableDefinition {
            table_name,
            indexes,
            staged_db_indexes: BTreeMap::new(),
            text_indexes: BTreeMap::new(),
            staged_text_indexes: BTreeMap::new(),
            vector_indexes: BTreeMap::new(),
            staged_vector_indexes: BTreeMap::new(),
            document_type: Some(document_schema),
        })
    }
}

fn column_names_in_metadata(
    metadata_validator: &ObjectValidator,
) -> Result<Vec<FivetranFieldName>, MetadataFieldError> {
    let Some(columns_validator) = metadata_validator
        .0
        .get(UNDERSCORED_COLUMNS_CONVEX_FIELD_NAME.deref())
    else {
        return Ok(Vec::new());
    };

    let Validator::Object(columns_validator) = columns_validator.validator() else {
        return Err(MetadataFieldError::InvalidColumnsFieldType);
    };

    let column_names: Vec<_> = columns_validator
        .0
        .keys()
        .map(|convex_field_name| {
            format!("_{convex_field_name}").parse().map_err(|err| {
                MetadataFieldError::UnsupportedColumnName(convex_field_name.clone(), err)
            })
        })
        .try_collect()?;

    Ok(column_names)
}

/// Validates that the table in the Convex schema is compatible with the source
/// Fivetran table.
///
/// For the same Fivetran table, there can be multiple valid Convex schemas. For
/// instance, the fields in the primary key index can be in an arbitrary order.
/// Also, fields in Convex can either be nullable (e.g. `v.union(v.string(),
/// v.null())`) or not (e.g. `v.string()`).
pub fn validate_destination_schema_table(
    fivetran_table: fivetran_sdk::Table,
    convex_table: &TableDefinition,
) -> Result<(), DestinationError> {
    let fivetran_table_name = FivetranTableName::from_str(&fivetran_table.name)
        .map_err(|err| DestinationError::InvalidTableName(fivetran_table.name.clone(), err))?;
    let table_name = TableName::from_str(&fivetran_table.name).map_err(|err| {
        DestinationError::UnsupportedTableName(fivetran_table_name.to_string(), err)
    })?;

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

pub fn is_field_validator_valid(actual_validator: &Validator, data_type: FivetranDataType) -> bool {
    let expected_validator = suggested_validator(data_type, Nullability::NonNullable);

    actual_validator == &expected_validator
        || actual_validator == &Validator::Union(vec![Validator::Null, expected_validator.clone()])
        || actual_validator == &Validator::Union(vec![expected_validator, Validator::Null])
}

/// Converts the given Convex schema table to a Fivetran table. This is used in
/// the implementation of the `AlterTable` endpoint so that Fivetran can be
/// aware of the current state of the Convex destination.
pub fn to_fivetran_table(
    convex_table: &TableDefinition,
) -> anyhow::Result<fivetran_sdk::Table, DestinationError> {
    let fivetran_columns = to_fivetran_columns(convex_table)?;

    Ok(fivetran_sdk::Table {
        name: convex_table.table_name.to_string(),
        columns: fivetran_columns,
    })
}

/// Returns the validator for the `fivetran` field of the given Convex table
/// definition.
///
/// Returns `None` if the `fivetran` field isn’t specified or is incorrectly
/// specified.
fn metadata_field_validator(validator: &ObjectValidator) -> Option<&ObjectValidator> {
    // System columns
    let field_validator = validator.0.get(&METADATA_CONVEX_FIELD_NAME.clone())?;
    let Validator::Object(metadata_object_validator) = field_validator.validator() else {
        return None;
    };

    Some(metadata_object_validator)
}

fn user_columns(table_def: &TableDefinition, validator: &ObjectValidator) -> Vec<Column> {
    let primary_key_index = table_def
        .indexes
        .get(&FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR);
    if primary_key_index.is_none() {
        log(&format!(
            "The table {} in your Convex schema is missing a `by_primary_key` index, so Fivetran \
             will not able to identify the columns of its primary key.",
            table_def.table_name
        ));
    }

    validator
        .0
        .iter()
        .filter(|(field_name, _)| **field_name != *METADATA_CONVEX_FIELD_NAME)
        .flat_map(|(field_name, field_validator)| {
            let fivetran_data_type = recognize_fivetran_type(field_validator.validator()).ok();
            if fivetran_data_type.is_none() {
                log(&format!(
                    "The type of the field `field_name` in the table `{}` isn’t supported by \
                     Fivetran.",
                    table_def.table_name
                ))
            }

            Some(fivetran_sdk::Column {
                name: field_name.to_string(),
                r#type: fivetran_data_type.unwrap_or(FivetranDataType::Unspecified) as i32,
                primary_key: primary_key_index.is_some_and(|primary_key_index| {
                    primary_key_index
                        .fields
                        .contains(&FieldPath::for_root_field(field_name.clone()))
                }),
                params: None,
            })
        })
        .collect()
}

fn to_fivetran_columns(
    table_def: &TableDefinition,
) -> Result<Vec<fivetran_sdk::Column>, DestinationError> {
    let Some(DocumentSchema::Union(validators)) = &table_def.document_type else {
        return Err(DestinationError::DestinationHasAnySchema(
            table_def.table_name.clone(),
        ));
    };
    let [validator] = &validators[..] else {
        return Err(DestinationError::DestinationHasMultipleSchemas(
            table_def.table_name.clone(),
        ));
    };

    let mut columns: Vec<fivetran_sdk::Column> = Vec::new();

    // System columns
    let metadata_validator = metadata_field_validator(validator);
    if let Some(metadata_validator) = metadata_validator {
        // Soft delete
        if metadata_validator
            .0
            .contains_key(&SOFT_DELETE_CONVEX_FIELD_NAME.clone())
        {
            columns.push(fivetran_sdk::Column {
                name: SOFT_DELETE_FIVETRAN_FIELD_NAME.to_string(),
                r#type: FivetranDataType::Boolean as i32,
                primary_key: false,
                params: None,
            });
        }

        // Fivetran pseudo-ID
        if let Some(field_validator) = metadata_validator.0.get(&ID_CONVEX_FIELD_NAME.clone()) {
            let id_field_type = recognize_fivetran_type(field_validator.validator()).ok();
            if id_field_type.is_none() {
                log(&format!(
                    "The type of the field `convex.id` in the table `{}` isn’t supported by \
                     Fivetran.",
                    table_def.table_name
                ))
            }

            columns.push(fivetran_sdk::Column {
                name: ID_FIVETRAN_FIELD_NAME.to_string(),
                r#type: id_field_type.unwrap_or(FivetranDataType::Unspecified) as i32,
                primary_key: true,
                params: None,
            });
        }

        // Synchronization timestamp
        columns.push(fivetran_sdk::Column {
            name: SYNCED_FIVETRAN_FIELD_NAME.to_string(),
            r#type: FivetranDataType::UtcDatetime as i32,
            primary_key: false,
            params: None,
        });

        // Columns having a Fivetran name starting by _
        if let Some(columns_validator) = metadata_validator
            .0
            .get(&UNDERSCORED_COLUMNS_CONVEX_FIELD_NAME.clone())
        {
            if let Validator::Object(columns_validator) = columns_validator.validator() {
                let primary_key_index = table_def
                    .indexes
                    .get(&FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR);

                for (column_name, column_validator) in columns_validator.0.iter() {
                    let field_path = FieldPath::new(vec![
                        METADATA_CONVEX_FIELD_NAME.clone(),
                        UNDERSCORED_COLUMNS_CONVEX_FIELD_NAME.clone(),
                        column_name.clone(),
                    ])
                    .expect("A three-column field path is always valid");

                    columns.push(fivetran_sdk::Column {
                        name: format!("_{column_name}"),
                        r#type: recognize_fivetran_type(column_validator.validator())
                            .unwrap_or(FivetranDataType::Unspecified)
                            as i32,
                        primary_key: primary_key_index.is_some_and(|primary_key_index| {
                            primary_key_index.fields.contains(&field_path)
                        }),
                        params: None,
                    });
                }
            };
        }
    }

    // User columns
    columns.append(&mut user_columns(table_def, validator));

    Ok(columns)
}

fn recognize_fivetran_type(validator: &Validator) -> anyhow::Result<FivetranDataType> {
    match validator {
        Validator::Float64 => Ok(FivetranDataType::Double),
        Validator::Int64 => Ok(FivetranDataType::Long),
        Validator::Boolean => Ok(FivetranDataType::Boolean),
        Validator::String => Ok(FivetranDataType::String),
        Validator::Bytes => Ok(FivetranDataType::Binary),
        Validator::Object(_) | Validator::Array(_) => Ok(FivetranDataType::Json),

        // Allow nullable types
        Validator::Union(validators) => match &validators[..] {
            [v] | [Validator::Null, v] | [v, Validator::Null] => recognize_fivetran_type(v),
            _ => bail!("Unsupported union"),
        },

        Validator::Null
        | Validator::Literal(_)
        | Validator::Id(_)
        | Validator::Record(..)
        | Validator::Any => bail!("The type of this Convex column isn’t supported by Fivetran."),
    }
}

fn recognize_convex_type(data_type: &FivetranDataType) -> anyhow::Result<Validator> {
    let validator = match data_type {
        FivetranDataType::Double => Validator::Float64,
        FivetranDataType::Long => Validator::Int64,
        FivetranDataType::Boolean => Validator::Boolean,
        FivetranDataType::String => Validator::String,
        FivetranDataType::Binary => Validator::Bytes,
        FivetranDataType::Json => Validator::Object(ObjectValidator(BTreeMap::new())),
        _ => anyhow::bail!("The type of this Convex column isn’t supported by Fivetran."),
    };
    Ok(Validator::Union(vec![validator, Validator::Null]))
}

#[cfg(test)]
mod tests {
    use std::collections::{
        BTreeMap,
        BTreeSet,
        HashSet,
    };

    use cmd_util::env::env_config;
    use common::{
        bootstrap_model::index::database_index::IndexedFields,
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
        value::FieldPath,
    };
    use convex_fivetran_common::fivetran_sdk::{
        self,
        Column,
        DataType as FivetranDataType,
        Table,
    };
    use convex_fivetran_destination::constants::{
        FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR,
        FIVETRAN_SYNCED_INDEX_DESCRIPTOR,
    };
    use maplit::{
        btreemap,
        btreeset,
        hashset,
    };
    use must_let::must_let;
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;

    use super::{
        validate_destination_schema_table,
        FivetranTableColumn,
        FivetranTableSchema,
    };
    use crate::{
        error::DestinationError,
        schema::to_fivetran_table,
        testing::fivetran_table_strategy,
    };

    fn fivetran_table(
        columns: BTreeMap<&str, FivetranDataType>,
        primary_key_columns: HashSet<&str>,
    ) -> fivetran_sdk::Table {
        for col_name in &primary_key_columns {
            if !columns.contains_key(col_name) {
                panic!("Unknown column `{col_name}` in the primary key");
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
                    params: None,
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
            table_name: "table_name".parse().unwrap(),
            staged_db_indexes: Default::default(),
            text_indexes: Default::default(),
            staged_text_indexes: Default::default(),
            vector_indexes: Default::default(),
            staged_vector_indexes: Default::default(),
            document_type: Some(DocumentSchema::Union(vec![ObjectValidator(
                fields
                    .into_iter()
                    .map(|(field_name, field_validator)| {
                        (field_name.parse().unwrap(), field_validator)
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
                let index_descriptor = IndexDescriptor::new(index_name.to_string()).unwrap();
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
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.as_str() => vec![
                        FieldPath::new(vec!["id".parse()?])?,
                    ],
                    FIVETRAN_SYNCED_INDEX_DESCRIPTOR.as_str() => vec![
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "synced".parse()?,
                        ])?,
                        FieldPath::new(vec!["_creationTime".parse()?])?,
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
                        FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.as_str() => vec![
                            FieldPath::new(vec!["id".parse()?])?,
                        ],
                        FIVETRAN_SYNCED_INDEX_DESCRIPTOR.as_str() => vec![
                            FieldPath::new(vec![
                                "fivetran".parse()?,
                                "synced".parse()?,
                            ])?,
                        FieldPath::new(vec!["_creationTime".parse()?])?,
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
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.as_str() => vec![
                        FieldPath::new(vec![
                            "id".parse()?,
                        ])?,
                    ],
                    FIVETRAN_SYNCED_INDEX_DESCRIPTOR.as_str() => vec![
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "synced".parse()?,
                        ])?,
                        FieldPath::new(vec!["_creationTime".parse()?])?,
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
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.as_str() => vec![
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "deleted".parse()?,
                        ])?,
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "id".parse()?,
                        ])?,
                    ],
                    FIVETRAN_SYNCED_INDEX_DESCRIPTOR.as_str() => vec![
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "deleted".parse()?,
                        ])?,
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "synced".parse()?,
                        ])?,
                        FieldPath::new(vec!["_creationTime".parse()?])?,
                    ],
                },
            ),
        )
        .unwrap();
        Ok(())
    }

    #[test]
    fn it_allows_tables_with_fivetran_columns_starting_by_underscore() -> anyhow::Result<()> {
        validate_destination_schema_table(
            fivetran_table(
                btreemap! {
                    "_key" => FivetranDataType::String,
                    "_nullable_field" => FivetranDataType::String,
                    "_non_nullable_field" => FivetranDataType::String,
                    "_fivetran_synced" => FivetranDataType::UtcDatetime,
                },
                hashset! {"_key"},
            ),
            &convex_table(
                btreemap! {
                    "fivetran" => FieldValidator::required_field_type(Validator::Object(
                        object_validator!(
                            "synced" => FieldValidator::required_field_type(Validator::Float64),
                            "columns" => FieldValidator::required_field_type(Validator::Object(object_validator!(
                                "key" => FieldValidator::required_field_type(Validator::String),
                                "nullable_field" => FieldValidator::required_field_type(Validator::String),
                                "non_nullable_field" => FieldValidator::required_field_type(Validator::Union(vec![
                                    Validator::String,
                                    Validator::Null,
                                ])),
                            ))),
                        ),
                    )),
                },
                btreemap! {
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.as_str() => vec![
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "columns".parse()?,
                            "key".parse()?,
                        ])?,
                    ],
                    FIVETRAN_SYNCED_INDEX_DESCRIPTOR.as_str() => vec![
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "synced".parse()?,
                        ])?,
                        FieldPath::new(vec!["_creationTime".parse()?])?,
                    ],
                },
            ),
        )
        .unwrap();
        Ok(())
    }

    #[test]
    fn it_refuses_tables_where_a_fivetran_field_with_underscore_is_missing_in_convex(
    ) -> anyhow::Result<()> {
        validate_destination_schema_table(
            fivetran_table(
                btreemap! {
                    "_field" => FivetranDataType::String,
                    "_fivetran_synced" => FivetranDataType::UtcDatetime,
                },
                hashset! {"_field"},
            ),
            &convex_table(
                btreemap! {
                    "fivetran" => FieldValidator::required_field_type(Validator::Object(
                        object_validator!(
                            "synced" => FieldValidator::required_field_type(Validator::Float64),
                        ),
                    )),
                },
                btreemap! {
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.as_str() => vec![
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "columns".parse()?,
                            "field".parse()?,
                        ])?,
                    ],
                    FIVETRAN_SYNCED_INDEX_DESCRIPTOR.as_str() => vec![
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "synced".parse()?,
                        ])?,
                        FieldPath::new(vec!["_creationTime".parse()?])?,
                    ],
                },
            ),
        )
        .unwrap_err();
        Ok(())
    }

    #[test]
    fn it_refuses_tables_with_extraneous_columns_in_the_metadata_field() -> anyhow::Result<()> {
        validate_destination_schema_table(
            fivetran_table(
                btreemap! {
                    "name" => FivetranDataType::String,
                    "_fivetran_synced" => FivetranDataType::UtcDatetime,
                },
                hashset! {"name"},
            ),
            &convex_table(
                btreemap! {
                    "name" => FieldValidator::required_field_type(Validator::String),
                    "fivetran" => FieldValidator::required_field_type(Validator::Object(
                        object_validator!(
                            "synced" => FieldValidator::required_field_type(Validator::Float64),
                            "columns" => FieldValidator::required_field_type(Validator::Object(object_validator!(
                                "extraneous" => FieldValidator::required_field_type(Validator::String),
                            ))),
                        ),
                    )),
                },
                btreemap! {
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.as_str() => vec![
                        FieldPath::new(vec![
                            "name".parse()?,
                        ])?,
                    ],
                    FIVETRAN_SYNCED_INDEX_DESCRIPTOR.as_str() => vec![
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "synced".parse()?,
                        ])?,
                        FieldPath::new(vec!["_creationTime".parse()?])?,
                    ],
                },
            ),
        )
        .unwrap_err();
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
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.as_str() => vec![
                        // _fivetran_deleted must be the first field in the index
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "deleted".parse()?,
                        ])?,

                        // The other fields can be in an arbitrary order
                        FieldPath::new(vec!["b".parse()?])?,
                        FieldPath::new(vec!["a".parse()?])?,
                        FieldPath::new(vec!["c".parse()?])?,

                    ],
                    "sync_index_named_arbitrarily" => vec![
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "deleted".parse()?,
                        ])?,
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "synced".parse()?,
                        ])?,
                        FieldPath::new(vec!["_creationTime".parse()?])?,
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
            FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.as_str() => vec![
                FieldPath::new(vec!["id".parse()?])?,
            ],
            "my_sync_index" => vec![
                FieldPath::new(vec![
                    "fivetran".parse()?,
                    "synced".parse()?,
                ])?,
                FieldPath::new(vec!["_creationTime".parse()?])?,
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

        let primary_key_index = vec![FieldPath::new(vec!["id".parse()?])?];
        let sync_index = vec![FieldPath::new(vec![
            "fivetran".parse()?,
            "synced".parse()?,
        ])?];

        assert!(table_schema
            .validate_destination_indexes(&convex_indexes(btreemap! {}))
            .is_err());
        assert!(table_schema
            .validate_destination_indexes(&convex_indexes(btreemap! {
                FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.as_str() => primary_key_index,
            }))
            .is_err());
        assert!(table_schema
            .validate_destination_indexes(&convex_indexes(btreemap! {
                FIVETRAN_SYNCED_INDEX_DESCRIPTOR.as_str() => sync_index,
            }))
            .is_err());
        Ok(())
    }

    #[test]
    fn required_indexes_include_the_soft_delete_field_if_it_exists() -> anyhow::Result<()> {
        fivetran_table_schema(
            btreemap! {
                "id" => FivetranDataType::Long,
                "_fivetran_synced" => FivetranDataType::UtcDatetime,
                "_fivetran_deleted" => FivetranDataType::Boolean,
            },
            btreeset! {"id"},
        )
        .validate_destination_indexes(&convex_indexes(btreemap! {
            FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.as_str() => vec![
                FieldPath::new(vec![
                    "fivetran".parse()?,
                    "deleted".parse()?,
                ])?,
                FieldPath::new(vec!["id".parse()?])?,
            ],
            "my_sync_index" => vec![
                FieldPath::new(vec![
                    "fivetran".parse()?,
                    "deleted".parse()?,
                ])?,
                FieldPath::new(vec![
                    "fivetran".parse()?,
                    "synced".parse()?,
                ])?,
                FieldPath::new(vec!["_creationTime".parse()?])?,
            ],
        }))
        .expect("Failed to validate indexes");

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
            FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.as_str() => vec![
                FieldPath::new(vec![
                    "fivetran".parse()?,
                    "deleted".parse()?,
                ])?,
                FieldPath::new(vec!["id".parse()?])?,
            ],
            "my_sync_index" => vec![
                // Wrong
                FieldPath::new(vec![
                    "fivetran".parse()?,
                    "synced".parse()?,
                ])?,
                FieldPath::new(vec![
                    "fivetran".parse()?,
                    "deleted".parse()?,
                ])?,
                FieldPath::new(vec!["_creationTime".parse()?])?,
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
            FieldPath::new(vec!["fivetran".parse()?, "deleted".parse()?])?,
            FieldPath::new(vec!["fivetran".parse()?, "synced".parse()?])?,
            FieldPath::new(vec!["_creationTime".parse()?])?,
        ];

        assert!(fivetran_table_schema
            .validate_destination_indexes(&convex_indexes(btreemap! {
                FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.as_str() => vec![
                    FieldPath::new(vec![
                        "fivetran".parse()?,
                        "deleted".parse()?,
                    ])?,
                    FieldPath::new(vec!["b".parse()?])?,
                    FieldPath::new(vec!["a".parse()?])?,
                    FieldPath::new(vec!["c".parse()?])?,
                ],
                FIVETRAN_SYNCED_INDEX_DESCRIPTOR.as_str() => sync_index.clone(),
            }))
            .is_ok());

        assert!(fivetran_table_schema
            .validate_destination_indexes(&convex_indexes(btreemap! {
                FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.as_str() => vec![
                    FieldPath::new(vec![
                        "fivetran".parse()?,
                        "deleted".parse()?,
                    ])?,
                    FieldPath::new(vec!["c".parse()?])?,
                    FieldPath::new(vec!["b".parse()?])?,
                    FieldPath::new(vec!["a".parse()?])?,
                ],
                FIVETRAN_SYNCED_INDEX_DESCRIPTOR.as_str() => sync_index.clone(),
            }))
            .is_ok());

        // The _fivetran_deleted field must be first
        assert!(fivetran_table_schema
            .validate_destination_indexes(&convex_indexes(btreemap! {
                FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.as_str() => vec![
                    FieldPath::new(vec!["c".parse()?])?,
                    FieldPath::new(vec!["b".parse()?])?,
                    FieldPath::new(vec!["a".parse()?])?,
                    // Error
                    FieldPath::new(vec![
                        "fivetran".parse()?,
                        "deleted".parse()?,
                    ])?,
                ],
                FIVETRAN_SYNCED_INDEX_DESCRIPTOR.as_str() => sync_index,
            }))
            .is_err());

        Ok(())
    }

    #[test]
    fn it_converts_convex_tables_to_fivetran_tables() -> anyhow::Result<()> {
        assert_eq!(
            to_fivetran_table(&convex_table(
                btreemap! {
                    "id" => FieldValidator::required_field_type(Validator::Int64),
                    "name" => FieldValidator::required_field_type(Validator::Union(vec![
                        Validator::Null,
                        Validator::String,
                    ])),
                    "fivetran" => FieldValidator::required_field_type(Validator::Object(
                        object_validator!(
                            "synced" => FieldValidator::required_field_type(Validator::Float64),
                        ),
                    )),
                },
                btreemap! {
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.as_str() => vec![
                        FieldPath::new(vec!["id".parse()?])?,
                    ],
                    FIVETRAN_SYNCED_INDEX_DESCRIPTOR.as_str() => vec![
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "synced".parse()?,
                        ])?,
                    ],
                },
            ))?,
            Table {
                name: "table_name".into(),
                columns: vec![
                    Column {
                        name: "_fivetran_synced".to_string(),
                        r#type: FivetranDataType::UtcDatetime as i32,
                        primary_key: false,
                        params: None,
                    },
                    Column {
                        name: "id".to_string(),
                        r#type: FivetranDataType::Long as i32,
                        primary_key: true,
                        params: None,
                    },
                    Column {
                        name: "name".to_string(),
                        r#type: FivetranDataType::String as i32,
                        primary_key: false,
                        params: None,
                    },
                ],
            }
        );

        Ok(())
    }

    #[test]
    fn it_converts_convex_tables_to_fivetran_tables_with_soft_deletes_and_fivetran_id(
    ) -> anyhow::Result<()> {
        assert_eq!(
            to_fivetran_table(&convex_table(
                btreemap! {
                    "data" => FieldValidator::required_field_type(Validator::Bytes),
                    "fivetran" => FieldValidator::required_field_type(Validator::Object(
                        object_validator!(
                            "synced" => FieldValidator::required_field_type(Validator::Float64),
                            "id" => FieldValidator::required_field_type(Validator::String),
                            "deleted" => FieldValidator::required_field_type(Validator::Boolean),
                        ),
                    )),
                },
                btreemap! {
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.as_str() => vec![
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "deleted".parse()?,
                        ])?,
                        FieldPath::new(vec!["id".parse()?])?,
                    ],
                    FIVETRAN_SYNCED_INDEX_DESCRIPTOR.as_str() => vec![
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "deleted".parse()?,
                        ])?,
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "synced".parse()?,
                        ])?,
                    ],
                },
            ))?,
            Table {
                name: "table_name".into(),
                columns: vec![
                    Column {
                        name: "_fivetran_deleted".to_string(),
                        r#type: FivetranDataType::Boolean as i32,
                        primary_key: false,
                        params: None,
                    },
                    Column {
                        name: "_fivetran_id".to_string(),
                        r#type: FivetranDataType::String as i32,
                        primary_key: true,
                        params: None,
                    },
                    Column {
                        name: "_fivetran_synced".to_string(),
                        r#type: FivetranDataType::UtcDatetime as i32,
                        primary_key: false,
                        params: None,
                    },
                    Column {
                        name: "data".to_string(),
                        r#type: FivetranDataType::Binary as i32,
                        primary_key: false,
                        params: None,
                    },
                ],
            }
        );

        Ok(())
    }

    #[test]
    fn it_converts_convex_tables_to_fivetran_tables_containing_columns_with_underscore(
    ) -> anyhow::Result<()> {
        assert_eq!(
            to_fivetran_table(&convex_table(
                btreemap! {
                    "fivetran" => FieldValidator::required_field_type(Validator::Object(
                        object_validator!(
                            "synced" => FieldValidator::required_field_type(Validator::Float64),
                            "columns" => FieldValidator::required_field_type(Validator::Object(
                                object_validator!(
                                    "key" => FieldValidator::required_field_type(Validator::String),
                                    "nullable_field" => FieldValidator::required_field_type(Validator::String),
                                    "non_nullable_field" => FieldValidator::required_field_type(Validator::Union(vec![
                                        Validator::String,
                                        Validator::Null,
                                    ])),
                                )
                            )),
                        ),
                    )),
                },
                btreemap! {
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.as_str() => vec![
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "columns".parse()?,
                            "key".parse()?,
                        ])?,
                    ],
                    FIVETRAN_SYNCED_INDEX_DESCRIPTOR.as_str() => vec![
                        FieldPath::new(vec![
                            "fivetran".parse()?,
                            "synced".parse()?,
                        ])?,
                    ],
                },
            ))?,
            Table {
                name: "table_name".into(),
                columns: vec![
                    Column {
                        name: "_fivetran_synced".to_string(),
                        r#type: FivetranDataType::UtcDatetime as i32,
                        primary_key: false,
                        params: None,
                    },
                    Column {
                        name: "_key".to_string(),
                        r#type: FivetranDataType::String as i32,
                        primary_key: true,
                        params: None,
                    },
                    Column {
                        name: "_non_nullable_field".to_string(),
                        r#type: FivetranDataType::String as i32,
                        primary_key: false,
                        params: None,
                    },
                    Column {
                        name: "_nullable_field".to_string(),
                        r#type: FivetranDataType::String as i32,
                        primary_key: false,
                        params: None,
                    },
                ],
            }
        );

        Ok(())
    }

    #[test]
    fn it_suggests_convex_tables() -> anyhow::Result<()> {
        let fivetran_table = fivetran_table_schema(
            btreemap! {
                "name" => FivetranDataType::String,
                "slug" => FivetranDataType::String,
                "_key" => FivetranDataType::String,
                "_fivetran_synced" => FivetranDataType::UtcDatetime,
                "_fivetran_deleted" => FivetranDataType::Boolean,
                "_fivetran_id" => FivetranDataType::String,
            },
            btreeset! {"slug", "_fivetran_id", "_key"},
        );

        assert_eq!(
            fivetran_table.suggested_convex_table()?,
            TableDefinition {
                table_name: "my_table".parse()?,
                indexes: btreemap! {
                    FIVETRAN_SYNCED_INDEX_DESCRIPTOR.clone() => IndexSchema {
                        index_descriptor: FIVETRAN_SYNCED_INDEX_DESCRIPTOR.clone(),
                        fields: vec![
                            "fivetran.deleted".parse()?,
                            "fivetran.synced".parse()?,
                            "_creationTime".parse()?,
                        ].try_into()?
                    },
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.clone() => IndexSchema {
                        index_descriptor: FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.clone(),
                        fields: vec![
                            "fivetran.deleted".parse()?,
                            "fivetran.id".parse()?,
                            "fivetran.columns.key".parse()?,
                            "slug".parse()?,
                        ].try_into()?
                    }
                },
                staged_db_indexes: btreemap! {},
                staged_text_indexes: btreemap! {},
                staged_vector_indexes: btreemap! {},
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
                        "id" => FieldValidator::required_field_type(Validator::String),
                        "columns" => FieldValidator::required_field_type(Validator::Object(object_validator!(
                            "key" => FieldValidator::required_field_type(
                                Validator::Union(vec![
                                    Validator::String,
                                    Validator::Null
                                ])
                            )
                        ))),
                    ))),
                )])),
                text_indexes: Default::default(),
                vector_indexes: Default::default(),
            },
        );
        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1),
            failure_persistence: None, ..ProptestConfig::default()
        })]
        #[test]
        fn suggested_convex_schemas_are_always_valid(fivetran_table in fivetran_table_strategy()) {
            let schema: FivetranTableSchema = fivetran_table.clone().try_into()?;
            let suggested_convex_table = schema.suggested_convex_table()?;
            prop_assert!(
                validate_destination_schema_table(fivetran_table, &suggested_convex_table).is_ok()
            );
        }
    }
}
