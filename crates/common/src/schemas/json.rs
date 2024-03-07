use std::collections::{
    BTreeMap,
    BTreeSet,
    HashMap,
    HashSet,
};

use anyhow::Context;
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use itertools::Itertools;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use value::{
    ConvexValue,
    FieldPath,
    IdentifierFieldName,
    TableName,
};

use super::{
    validator::{
        FieldValidator,
        LiteralValidator,
        ObjectValidator,
        Validator,
    },
    DatabaseSchema,
    DocumentSchema,
    IndexSchema,
    VectorIndexSchema,
};
use crate::{
    bootstrap_model::index::{
        index_validation_error::{
            self,
            index_not_unique,
            search_field_not_unique,
            vector_field_not_unique,
        },
        vector_index::VectorDimensions,
    },
    json::invalid_json,
    schemas::{
        invalid_top_level_type_in_schema,
        SearchIndexSchema,
        TableDefinition,
        MAX_INDEXES_PER_TABLE,
        MAX_SEARCH_INDEXES_PER_TABLE,
        MAX_VECTOR_INDEXES_PER_TABLE,
    },
    types::{
        IndexDescriptor,
        IndexName,
    },
};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DatabaseSchemaJson {
    tables: Vec<JsonValue>,
    schema_validation: Option<bool>,
}

impl TryFrom<JsonValue> for DatabaseSchema {
    type Error = anyhow::Error;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        let j: DatabaseSchemaJson = serde_json::from_value(value).with_context(invalid_json)?;

        let tables = j
            .tables
            .into_iter()
            .map(|v| {
                let s = TableDefinition::try_from(v)?;
                Ok((s.table_name.clone(), s))
            })
            .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

        // Schemas written before schema validation was introduced don't include
        // this. Default to false.
        let schema_validation = j.schema_validation.unwrap_or(false);
        Ok(DatabaseSchema {
            tables,
            schema_validation,
        })
    }
}

impl TryFrom<DatabaseSchema> for JsonValue {
    type Error = anyhow::Error;

    fn try_from(
        DatabaseSchema {
            tables,
            schema_validation,
        }: DatabaseSchema,
    ) -> anyhow::Result<Self> {
        let database_schema_json = DatabaseSchemaJson {
            tables: tables
                .into_values()
                .map(JsonValue::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            schema_validation: Some(schema_validation),
        };
        Ok(serde_json::to_value(database_schema_json)?)
    }
}

impl TryFrom<ConvexValue> for DatabaseSchema {
    type Error = anyhow::Error;

    fn try_from(v: ConvexValue) -> anyhow::Result<Self> {
        JsonValue::try_from(v)?.try_into()
    }
}

impl TryFrom<DatabaseSchema> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(database_schema: DatabaseSchema) -> anyhow::Result<Self> {
        JsonValue::try_from(database_schema)?.try_into()
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TableDefinitionJson {
    table_name: String,
    indexes: Vec<JsonValue>,
    search_indexes: Option<Vec<JsonValue>>,
    vector_indexes: Option<Vec<JsonValue>>,
    document_type: Option<JsonValue>,
}

// Collect the index names separately from the deduplicating map so that we can
// complain complain about duplicate names
fn parse_names_and_indexes<T: TryFrom<JsonValue, Error = anyhow::Error>>(
    table_name: &TableName,
    indexes: Option<Vec<JsonValue>>,
    descriptor: impl Fn(&T) -> &IndexDescriptor,
) -> anyhow::Result<(Vec<IndexDescriptor>, BTreeMap<IndexDescriptor, T>)> {
    itertools::process_results(
        indexes
            .unwrap_or_default()
            .into_iter()
            .map(T::try_from)
            .map_ok(|idx| {
                let index_name = descriptor(&idx);
                (index_name.clone(), (index_name.clone(), idx))
            }),
        |iter| iter.unzip(),
    )
    .map_err(|e: anyhow::Error| e.wrap_error_message(|s| format!("In table \"{table_name}\": {s}")))
}

fn validate_unique_index_fields<T, Y: Clone + Eq + std::hash::Hash>(
    indexes: &BTreeMap<IndexDescriptor, T>,
    unique_index_field: impl Fn(&T) -> Y,
    non_unique_error: impl Fn(&IndexDescriptor, &IndexDescriptor) -> ErrorMetadata,
) -> anyhow::Result<()> {
    let index_fields: BTreeMap<_, _> = indexes
        .iter()
        .map(|(name, fields)| (name, unique_index_field(fields)))
        .collect();

    let mut seen: HashMap<_, &IndexDescriptor> = HashMap::new();
    for (name, fields) in index_fields.into_iter() {
        if let Some(other_name) = seen.insert(fields, name) {
            anyhow::bail!(non_unique_error(name, other_name));
        }
    }
    Ok(())
}

impl TryFrom<JsonValue> for TableDefinition {
    type Error = anyhow::Error;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        let j: TableDefinitionJson = serde_json::from_value(value).with_context(invalid_json)?;

        let document_type = j.document_type.map(|t| t.try_into()).transpose()?;

        let table_name: TableName = j
            .table_name
            .parse()
            .with_context(|| index_validation_error::invalid_table_name(&j.table_name))?;
        anyhow::ensure!(
            !table_name.is_system(),
            index_validation_error::table_name_reserved(&table_name)
        );

        if j.indexes.len() > MAX_INDEXES_PER_TABLE {
            anyhow::bail!(index_validation_error::too_many_indexes(
                &table_name,
                MAX_INDEXES_PER_TABLE
            ));
        }

        let (index_names, indexes) =
            parse_names_and_indexes(&table_name, Some(j.indexes), |idx: &IndexSchema| {
                &idx.index_descriptor
            })?;
        for schema in indexes.values() {
            if schema.fields.is_empty() {
                anyhow::bail!(index_validation_error::empty_index(&table_name, schema));
            }
        }
        validate_unique_index_fields(
            &indexes,
            |idx| Vec::<FieldPath>::from(idx.fields.clone()),
            |index1, index2| index_not_unique(&table_name, index1, index2),
        )?;

        let (search_index_names, search_indexes) =
            parse_names_and_indexes(&table_name, j.search_indexes, |idx: &SearchIndexSchema| {
                &idx.index_descriptor
            })?;
        validate_unique_index_fields(
            &search_indexes,
            |idx| idx.search_field.clone(),
            |index1, index2| search_field_not_unique(&table_name, index1, index2),
        )?;
        if search_indexes.len() > MAX_SEARCH_INDEXES_PER_TABLE {
            anyhow::bail!(index_validation_error::too_many_search_indexes(
                &table_name,
                MAX_SEARCH_INDEXES_PER_TABLE
            ));
        }

        let (vector_index_names, vector_indexes): (Vec<_>, BTreeMap<_, _>) =
            parse_names_and_indexes(&table_name, j.vector_indexes, |idx: &VectorIndexSchema| {
                &idx.index_descriptor
            })?;
        validate_unique_index_fields(
            &vector_indexes,
            |idx| (idx.vector_field.clone(), idx.dimension),
            |index1, index2| vector_field_not_unique(&table_name, index1, index2),
        )?;

        if vector_indexes.len() > MAX_VECTOR_INDEXES_PER_TABLE {
            anyhow::bail!(index_validation_error::too_many_vector_indexes(
                &table_name,
                MAX_VECTOR_INDEXES_PER_TABLE
            ));
        }

        let all_index_names: Vec<_> = index_names
            .into_iter()
            .chain(search_index_names)
            .chain(vector_index_names)
            .collect();

        let mut seen: HashSet<_> = HashSet::new();
        for index_name in all_index_names.into_iter() {
            // Validate the name
            IndexName::new(table_name.clone(), index_name.clone())?;

            if !seen.insert(index_name.clone()) {
                anyhow::bail!(index_validation_error::names_not_unique(
                    &table_name,
                    &index_name
                ));
            }
        }

        Ok(Self {
            table_name,
            indexes,
            search_indexes,
            vector_indexes,
            document_type,
        })
    }
}

impl TryFrom<TableDefinition> for JsonValue {
    type Error = anyhow::Error;

    fn try_from(
        TableDefinition {
            table_name,
            indexes,
            search_indexes,
            vector_indexes,
            document_type,
        }: TableDefinition,
    ) -> anyhow::Result<Self> {
        let table_name = String::from(table_name);
        let indexes = indexes
            .into_values()
            .map(JsonValue::try_from)
            .collect::<anyhow::Result<Vec<_>>>()?;
        let search_indexes = Some(
            search_indexes
                .into_values()
                .map(JsonValue::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
        );
        let document_type = document_type.map(JsonValue::try_from).transpose()?;
        let vector_indexes = Some(
            vector_indexes
                .into_values()
                .map(JsonValue::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
        );
        Ok(serde_json::to_value(TableDefinitionJson {
            table_name,
            indexes,
            search_indexes,
            vector_indexes,
            document_type,
        })?)
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexSchemaJson {
    index_descriptor: String,
    fields: Vec<String>,
}

impl TryFrom<JsonValue> for IndexSchema {
    type Error = anyhow::Error;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        let j: IndexSchemaJson = serde_json::from_value(value).with_context(invalid_json)?;
        let index_descriptor = j.index_descriptor.parse()?;
        let fields = j
            .fields
            .into_iter()
            .map(|p| {
                p.parse().with_context(|| {
                    index_validation_error::invalid_index_field(&index_descriptor, &p)
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?
            .try_into()
            .map_err(|e: anyhow::Error| {
                e.wrap_error_message(|s| format!("In index \"{index_descriptor}\": {s}"))
            })?;
        Ok(Self {
            index_descriptor,
            fields,
        })
    }
}

impl TryFrom<IndexSchema> for JsonValue {
    type Error = anyhow::Error;

    fn try_from(
        IndexSchema {
            index_descriptor,
            fields,
        }: IndexSchema,
    ) -> anyhow::Result<Self> {
        let index_schema_json = IndexSchemaJson {
            index_descriptor: String::from(index_descriptor),
            fields: Vec::<FieldPath>::from(fields)
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>(),
        };
        Ok(serde_json::to_value(index_schema_json)?)
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct VectorIndexSchemaJson {
    index_descriptor: String,
    vector_field: String,
    dimensions: Option<u32>,
    dimension: Option<u32>,
    filter_fields: Vec<String>,
}

impl TryFrom<JsonValue> for VectorIndexSchema {
    type Error = anyhow::Error;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        let j: VectorIndexSchemaJson = serde_json::from_value(value).with_context(invalid_json)?;
        let index_descriptor = j.index_descriptor.parse()?;
        let vector_field = j.vector_field.parse().with_context(|| {
            index_validation_error::invalid_index_field(&index_descriptor, &j.vector_field)
        })?;
        let filter_fields = j
            .filter_fields
            .into_iter()
            .map(|f| {
                f.parse().with_context(|| {
                    index_validation_error::invalid_index_field(&index_descriptor, &f)
                })
            })
            .collect::<anyhow::Result<BTreeSet<_>>>()?;
        let dimension: VectorDimensions = match j.dimensions {
            Some(d) => d.try_into()?,
            // Support legacy alpha users
            None => match j.dimension {
                Some(d) => d.try_into()?,
                None => anyhow::bail!("Missing dimensions field"),
            },
        };
        Self::new(index_descriptor, vector_field, dimension, filter_fields)
    }
}

impl TryFrom<VectorIndexSchema> for JsonValue {
    type Error = anyhow::Error;

    fn try_from(
        VectorIndexSchema {
            index_descriptor,
            vector_field,
            dimension,
            filter_fields,
            ..
        }: VectorIndexSchema,
    ) -> anyhow::Result<Self> {
        let vector_index_schema_json = VectorIndexSchemaJson {
            index_descriptor: String::from(index_descriptor),
            vector_field: String::from(vector_field),
            dimensions: Some(dimension.into()),
            dimension: None,
            filter_fields: filter_fields
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>(),
        };
        Ok(serde_json::to_value(vector_index_schema_json)?)
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchIndexSchemaJson {
    index_descriptor: String,
    search_field: String,
    filter_fields: BTreeSet<String>,
}

impl TryFrom<JsonValue> for SearchIndexSchema {
    type Error = anyhow::Error;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        let j: SearchIndexSchemaJson = serde_json::from_value(value).with_context(invalid_json)?;
        let index_descriptor = j.index_descriptor.parse()?;
        let search_field = j.search_field.parse().with_context(|| {
            index_validation_error::invalid_index_field(&index_descriptor, &j.search_field)
        })?;
        let filter_fields = j
            .filter_fields
            .into_iter()
            .map(|f| {
                f.parse().with_context(|| {
                    index_validation_error::invalid_index_field(&index_descriptor, &f)
                })
            })
            .collect::<anyhow::Result<BTreeSet<_>>>()?;

        Self::new(index_descriptor, search_field, filter_fields)
    }
}

impl TryFrom<SearchIndexSchema> for JsonValue {
    type Error = anyhow::Error;

    fn try_from(
        SearchIndexSchema {
            index_descriptor,
            search_field,
            filter_fields,
            ..
        }: SearchIndexSchema,
    ) -> anyhow::Result<Self> {
        let search_index_json = SearchIndexSchemaJson {
            index_descriptor: index_descriptor.to_string(),
            search_field: String::from(search_field),
            filter_fields: filter_fields
                .into_iter()
                .map(String::from)
                .collect::<BTreeSet<_>>(),
        };
        Ok(serde_json::to_value(search_index_json)?)
    }
}

impl TryFrom<JsonValue> for DocumentSchema {
    type Error = anyhow::Error;

    fn try_from(v: JsonValue) -> Result<Self, Self::Error> {
        let schema_type: Validator = v.try_into()?;
        match schema_type.clone() {
            Validator::Any => Ok(DocumentSchema::Any),
            Validator::Union(value) => {
                let schemas: Vec<_> = value
                    .into_iter()
                    .map(|s| {
                        if let Validator::Object(object_schema) = s {
                            // TODO(sarah) Change this to error on system fields at the top level
                            // once data has been migrated
                            Ok(object_schema.filter_system_fields())
                        } else {
                            Err(anyhow::anyhow!(invalid_top_level_type_in_schema(
                                &schema_type
                            )))
                        }
                    })
                    .collect::<anyhow::Result<_>>()?;
                Ok(DocumentSchema::Union(schemas))
            },
            Validator::Object(object_schema) => {
                // TODO(sarah) Change this to error on system fields at the top level
                // once data has been migrated
                let filtered_schema = object_schema.filter_system_fields();
                Ok(DocumentSchema::Union(vec![filtered_schema]))
            },
            _ => Err(anyhow::anyhow!(invalid_top_level_type_in_schema(
                &schema_type
            ))),
        }
    }
}

impl TryFrom<DocumentSchema> for JsonValue {
    type Error = anyhow::Error;

    fn try_from(d: DocumentSchema) -> anyhow::Result<JsonValue> {
        match d {
            DocumentSchema::Any => JsonValue::try_from(Validator::Any),
            DocumentSchema::Union(mut object_schemas) => {
                if object_schemas.len() == 1 {
                    let single_schema = object_schemas.pop().unwrap();
                    JsonValue::try_from(Validator::Object(single_schema))
                } else {
                    JsonValue::try_from(Validator::Union(
                        object_schemas.into_iter().map(Validator::Object).collect(),
                    ))
                }
            },
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct FieldTypeJson {
    field_type: JsonValue,
    optional: bool,
}

impl TryFrom<JsonValue> for FieldValidator {
    type Error = anyhow::Error;

    fn try_from(value: JsonValue) -> anyhow::Result<Self> {
        let field_type_json: FieldTypeJson =
            serde_json::from_value(value).context("Not a field validator")?;
        Ok(FieldValidator {
            validator: field_type_json.field_type.try_into()?,
            optional: field_type_json.optional,
        })
    }
}

impl TryFrom<FieldValidator> for JsonValue {
    type Error = anyhow::Error;

    fn try_from(f: FieldValidator) -> anyhow::Result<JsonValue> {
        let field_type_json = FieldTypeJson {
            field_type: JsonValue::try_from(f.validator)?,
            optional: f.optional,
        };
        Ok(serde_json::to_value(field_type_json)?)
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
enum ValidatorJson {
    Null,
    Number,
    Bigint,
    Boolean,
    String,
    Bytes,
    Any,
    Literal {
        value: JsonValue,
    },
    #[serde(rename_all = "camelCase")]
    Id {
        table_name: String,
    },
    Array {
        value: JsonValue,
    },
    Set {
        value: JsonValue,
    },
    Map {
        keys: JsonValue,
        values: JsonValue,
    },
    Record {
        keys: JsonValue,
        values: JsonValue,
    },
    Object {
        value: JsonValue,
    },
    Union {
        value: Vec<JsonValue>,
    },
}

impl TryFrom<ValidatorJson> for Validator {
    type Error = anyhow::Error;

    fn try_from(s: ValidatorJson) -> anyhow::Result<Self> {
        match s {
            ValidatorJson::Null => Ok(Validator::Null),
            ValidatorJson::Number => Ok(Validator::Float64),
            ValidatorJson::Bigint => Ok(Validator::Int64),
            ValidatorJson::Boolean => Ok(Validator::Boolean),
            ValidatorJson::String => Ok(Validator::String),
            ValidatorJson::Bytes => Ok(Validator::Bytes),
            ValidatorJson::Any => Ok(Validator::Any),
            ValidatorJson::Literal { value } => Ok(Validator::Literal(value.try_into()?)),
            ValidatorJson::Id { table_name } => Ok(Validator::Id(table_name.parse()?)),
            ValidatorJson::Array { value } => Ok(Validator::Array(Box::new(value.try_into()?))),
            ValidatorJson::Set { value } => Ok(Validator::Set(Box::new(value.try_into()?))),
            ValidatorJson::Map { keys, values } => Ok(Validator::Map(
                Box::new(keys.try_into()?),
                Box::new(values.try_into()?),
            )),
            ValidatorJson::Record { keys, values } => {
                let keys_validator = Validator::try_from(keys)?;
                if !keys_validator.is_subset(&Validator::String) {
                    anyhow::bail!(ErrorMetadata::bad_request(
                        "InvalidRecordType",
                        format!(
                            "Records can only have string keys. Your validator contains a record \
                             with key typed as `{keys_validator}`, which is not a subtype of \
                             `v.string()`"
                        )
                    ))
                }
                let values_validator = FieldValidator::try_from(values)?;
                if keys_validator.is_string_subtype_with_string_literal()
                    && !values_validator.optional
                {
                    let optional_values_validator = FieldValidator {
                        optional: true,
                        validator: values_validator.validator.clone(),
                    };
                    anyhow::bail!(ErrorMetadata::bad_request(
                        "InvalidRecordType",
                        format!(
                            "Records with string literal keys must have an optional value. Your \
                             validator has value type {values_validator} instead of \
                             {optional_values_validator}"
                        )
                    ));
                }
                Ok(Validator::Record(
                    Box::new(keys_validator),
                    Box::new(values_validator.validator),
                ))
            },
            ValidatorJson::Object { value } => Ok(Validator::Object(value.try_into()?)),
            ValidatorJson::Union { value } => {
                let schemas = value
                    .into_iter()
                    .map(Validator::try_from)
                    .collect::<anyhow::Result<_>>()?;
                Ok(Validator::Union(schemas))
            },
        }
    }
}

impl TryFrom<JsonValue> for Validator {
    type Error = anyhow::Error;

    fn try_from(value: JsonValue) -> anyhow::Result<Self> {
        let schema_type_json: ValidatorJson = serde_json::from_value(value)?;
        schema_type_json.try_into()
    }
}

impl TryFrom<Validator> for JsonValue {
    type Error = anyhow::Error;

    fn try_from(s: Validator) -> anyhow::Result<JsonValue> {
        let schema_type = match s {
            Validator::Id(table_name) => ValidatorJson::Id {
                table_name: table_name.to_string(),
            },
            Validator::Null => ValidatorJson::Null,
            Validator::Float64 => ValidatorJson::Number,
            Validator::Int64 => ValidatorJson::Bigint,
            Validator::Boolean => ValidatorJson::Boolean,
            Validator::String => ValidatorJson::String,
            Validator::Bytes => ValidatorJson::Bytes,
            Validator::Literal(literal) => ValidatorJson::Literal {
                value: literal.try_into()?,
            },
            Validator::Array(t) => ValidatorJson::Array {
                value: JsonValue::try_from(*t)?,
            },
            Validator::Set(t) => ValidatorJson::Set {
                value: JsonValue::try_from(*t)?,
            },
            Validator::Map(k, v) => ValidatorJson::Map {
                keys: JsonValue::try_from(*k)?,
                values: JsonValue::try_from(*v)?,
            },
            Validator::Record(k, v) => {
                let optional_value = k.is_string_subtype_with_string_literal();
                ValidatorJson::Record {
                    keys: JsonValue::try_from(*k)?,
                    values: JsonValue::try_from(FieldValidator {
                        optional: optional_value,
                        validator: *v,
                    })?,
                }
            },
            Validator::Object(o) => ValidatorJson::Object {
                value: o.try_into()?,
            },
            Validator::Union(v) => ValidatorJson::Union {
                value: v
                    .into_iter()
                    .map(JsonValue::try_from)
                    .collect::<anyhow::Result<Vec<_>>>()?,
            },
            Validator::Any => ValidatorJson::Any,
        };
        Ok(serde_json::to_value(schema_type)?)
    }
}

impl TryFrom<JsonValue> for LiteralValidator {
    type Error = anyhow::Error;

    fn try_from(v: JsonValue) -> anyhow::Result<Self> {
        let value: ConvexValue = v.try_into()?;
        value.try_into()
    }
}

impl TryFrom<LiteralValidator> for JsonValue {
    type Error = anyhow::Error;

    fn try_from(s: LiteralValidator) -> anyhow::Result<JsonValue> {
        let v = match s {
            LiteralValidator::Float64(f) => {
                let f: f64 = f.into();
                let n = serde_json::Number::from_f64(f)
                    .ok_or_else(|| anyhow::anyhow!("Number failed to serialize from f64: {f}"))?;
                JsonValue::Number(n)
            },
            LiteralValidator::Int64(i) => JsonValue::from(ConvexValue::Int64(i)),
            LiteralValidator::Boolean(b) => JsonValue::Bool(b),
            LiteralValidator::String(s) => JsonValue::String(s.to_string()),
        };
        Ok(v)
    }
}

impl TryFrom<JsonValue> for ObjectValidator {
    type Error = anyhow::Error;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        let value = value
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Object must be an object"))?;
        let schema = ObjectValidator(
            value
                .into_iter()
                .map(|(k, v)| {
                    let field_name = k.parse::<IdentifierFieldName>()?;
                    let field_value = FieldValidator::try_from(v.clone()).map_err(|e| {
                        e.wrap_error_message(|msg| {
                            format!("Invalid validator for key `{field_name}`: {msg}")
                        })
                    })?;
                    Ok((field_name, field_value))
                })
                .collect::<anyhow::Result<_>>()?,
        );
        Ok(schema)
    }
}

impl TryFrom<ObjectValidator> for JsonValue {
    type Error = anyhow::Error;

    fn try_from(o: ObjectValidator) -> anyhow::Result<JsonValue> {
        let mut map = serde_json::Map::new();
        for (field, field_type) in o.0 {
            map.insert(field.to_string(), JsonValue::try_from(field_type)?);
        }
        Ok(JsonValue::Object(map))
    }
}
