#[cfg(any(test, feature = "testing"))]
use std::collections::BTreeSet;
use std::{
    borrow::Borrow,
    collections::BTreeMap,
    fmt::{
        self,
        Display,
    },
    iter,
};

use errors::ErrorMetadata;
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use serde_json::{
    Number,
    Value as JsonValue,
};
use shape_inference::{
    Shape,
    ShapeConfig,
    ShapeCounter,
    ShapeEnum,
};
use value::{
    export::ValueFormat,
    id_v6::DeveloperDocumentId,
    sorting::TotalOrdF64,
    utils::{
        all_tables_number_to_name,
        display_map,
        display_sequence,
    },
    ConvexObject,
    ConvexValue,
    FieldName,
    FieldPath,
    IdentifierFieldName,
    Namespace,
    NamespacedTableMapping,
    TableName,
    TableNumber,
    VirtualTableMapping,
};

use super::DocumentSchema;
use crate::{
    document::{
        CREATION_TIME_FIELD,
        ID_FIELD,
    },
    json_schemas,
};

/// Validates that a Convex value has the given type.
///
/// These are used by both schema enforcement and argument validation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Validator {
    Id(TableName),
    Null,
    Float64,
    Int64,
    Boolean,
    String,
    Bytes,
    Literal(LiteralValidator),
    Array(Box<Validator>),
    Set(Box<Validator>),
    Record(Box<Validator>, Box<Validator>),
    Map(Box<Validator>, Box<Validator>),
    Object(ObjectValidator),
    Union(Vec<Validator>),
    Any,
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for Validator {
    type Parameters = BTreeSet<TableName>;

    type Strategy = impl proptest::strategy::Strategy<Value = Validator>;

    fn arbitrary_with(table_names: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        let id_validator = if table_names.is_empty() {
            any::<TableName>().prop_map(Validator::Id).boxed()
        } else {
            let table_names: Vec<_> = table_names.into_iter().collect();
            proptest::sample::select(table_names)
                .prop_map(Validator::Id)
                .boxed()
        };
        let leaf = prop_oneof![
            Just(Validator::Null),
            id_validator,
            Just(Validator::Float64),
            Just(Validator::Int64),
            Just(Validator::Boolean),
            Just(Validator::String),
            Just(Validator::Bytes),
            any::<LiteralValidator>().prop_map(Validator::Literal),
            Just(Validator::Any),
        ];
        leaf.prop_recursive(3, 8, 8, move |inner| {
            prop_oneof![
                inner.clone().prop_map(Box::new).prop_map(Validator::Array),
                inner.clone().prop_map(Box::new).prop_map(Validator::Set),
                (inner.clone(), inner.clone())
                    .prop_map(|(s1, s2)| Validator::Map(Box::new(s1), Box::new(s2))),
                prop::collection::btree_map(
                    any::<IdentifierFieldName>(),
                    (inner.clone(), proptest::bool::ANY).prop_map(|(validator, optional)| {
                        FieldValidator {
                            validator,
                            optional,
                        }
                    }),
                    0..8
                )
                .prop_map(ObjectValidator)
                .prop_map(Validator::Object),
                prop::collection::vec(inner, 1..8).prop_map(Validator::Union),
            ]
        })
    }
}

impl Display for Validator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Validator::Id(table_name) => write!(f, "v.id(\"{table_name}\")"),
            Validator::Null => write!(f, "v.null()"),
            Validator::Float64 => write!(f, "v.float64()"),
            Validator::Int64 => write!(f, "v.int64()"),
            Validator::Boolean => write!(f, "v.boolean()"),
            Validator::String => write!(f, "v.string()"),
            Validator::Bytes => write!(f, "v.bytes()"),
            Validator::Literal(literal) => write!(f, "v.literal({literal})"),
            Validator::Array(validator) => write!(f, "v.array({validator})"),
            Validator::Set(validator) => write!(f, "v.set({validator})"),
            Validator::Map(keys, values) => write!(f, "v.map({keys}, {values})"),
            Validator::Record(keys, values) => write!(f, "v.record({keys}, {values})"),
            Validator::Object(object_validator) => write!(f, "{}", object_validator),
            Validator::Union(validators) => {
                display_sequence(f, ["v.union(", ")"], validators.iter())
            },
            Validator::Any => write!(f, "v.any()"),
        }
    }
}

impl Validator {
    pub fn check_value(
        &self,
        value: &ConvexValue,
        table_mapping: &NamespacedTableMapping,
        virtual_table_mapping: &VirtualTableMapping,
    ) -> Result<(), ValidationError> {
        let all_tables_number_to_name =
            all_tables_number_to_name(table_mapping, virtual_table_mapping);
        self.check_value_internal(value, &all_tables_number_to_name, ValidationContext::new())
    }

    fn check_value_internal(
        &self,
        value: &ConvexValue,
        all_tables_number_to_name: &impl Fn(TableNumber) -> anyhow::Result<TableName>,
        context: ValidationContext,
    ) -> Result<(), ValidationError> {
        match (self, value) {
            (Validator::Id(validator_table), ConvexValue::String(s)) => {
                if let Ok(id) = DeveloperDocumentId::decode(s)
                    && let Ok(table_name) = all_tables_number_to_name(id.table())
                {
                    if &table_name != validator_table {
                        if table_name.is_system() {
                            let err = ValidationError::SystemTableReference {
                                id,
                                validator_table: validator_table.clone(),
                                context,
                            };
                            return Err(err);
                        } else {
                            let err = ValidationError::TableNamesDoNotMatch {
                                id,
                                found_table_name: table_name,
                                validator_table: validator_table.clone(),
                                context,
                            };
                            return Err(err);
                        }
                    }
                } else {
                    let err = ValidationError::NoMatch {
                        value: value.clone(),
                        validator: self.clone(),
                        context,
                    };
                    return Err(err);
                }
            },
            (Validator::Null, ConvexValue::Null)
            | (Validator::Float64, ConvexValue::Float64(_))
            | (Validator::Int64, ConvexValue::Int64(_))
            | (Validator::Boolean, ConvexValue::Boolean(_))
            | (Validator::String, ConvexValue::String(_))
            | (Validator::Bytes, ConvexValue::Bytes(_)) => return Ok(()),
            (Validator::Literal(literal), value) => {
                let literal_as_value: ConvexValue = literal.clone().into();
                if value != &literal_as_value {
                    return Err(ValidationError::LiteralValuesDoNotMatch {
                        value: value.clone(),
                        literal_validator: literal.clone(),
                        context,
                    });
                }
            },
            (Validator::Array(t), ConvexValue::Array(v)) => {
                for (i, elt) in v.into_iter().enumerate() {
                    t.check_value_internal(
                        elt,
                        all_tables_number_to_name,
                        context.with(format!("[{i}]")),
                    )?;
                }
            },
            (Validator::Set(t), ConvexValue::Set(v)) => {
                for (i, elt) in v.into_iter().enumerate() {
                    t.check_value_internal(
                        elt,
                        all_tables_number_to_name,
                        context.with(format!(".keys()[{i}]")),
                    )?;
                }
            },
            (Validator::Map(key_type, value_type), ConvexValue::Map(map)) => {
                for (i, (key, value)) in map.into_iter().enumerate() {
                    key_type.check_value_internal(
                        key,
                        all_tables_number_to_name,
                        context.with(format!("keys()[{i}]")),
                    )?;
                    value_type.check_value_internal(
                        value,
                        all_tables_number_to_name,
                        context.with(format!(".values()[{i}]")),
                    )?;
                }
            },
            (Validator::Record(key_type, value_type), ConvexValue::Object(object)) => {
                for (key, value) in object.iter() {
                    key_type.check_value_internal(
                        &ConvexValue::from(key.clone()),
                        all_tables_number_to_name,
                        context.with(format!(".keys()")),
                    )?;
                    value_type.check_value_internal(
                        value,
                        all_tables_number_to_name,
                        context.with(format!(".values()")),
                    )?;
                }
            },
            (Validator::Object(object_validator), ConvexValue::Object(object)) => {
                for (field_name, field_type) in &object_validator.0 {
                    let maybe_value = object.get::<str>(field_name.borrow());
                    if let Some(value) = maybe_value {
                        field_type.validator.check_value_internal(
                            value,
                            all_tables_number_to_name,
                            context.with(format!(".{field_name}")),
                        )?
                    } else if !field_type.optional {
                        return Err(ValidationError::MissingRequiredField {
                            object: object.clone(),
                            field_name: field_name.clone(),
                            object_validator: object_validator.clone(),
                            context,
                        });
                    }
                }
                for field in object.keys() {
                    if !object_validator.0.contains_key::<str>(field.borrow()) {
                        return Err(ValidationError::ExtraField {
                            object: object.clone(),
                            field_name: field.clone(),
                            object_validator: object_validator.clone(),
                            context,
                        });
                    }
                }
            },
            (Validator::Union(validators), value) => {
                if validators.len() == 1 {
                    return validators[0].check_value_internal(
                        value,
                        all_tables_number_to_name,
                        context,
                    );
                }

                // TODO: This is dropping the error messages from the individual
                // validators. Maybe we should combine them if this fails?
                for t in validators {
                    if t.check_value_internal(value, all_tables_number_to_name, context.clone())
                        .is_ok()
                    {
                        return Ok(());
                    }
                }
                return Err(ValidationError::NoMatch {
                    value: value.clone(),
                    validator: self.clone(),
                    context,
                });
            },
            (Validator::Any, _) => return Ok(()),
            (..) => {
                return Err(ValidationError::NoMatch {
                    value: value.clone(),
                    validator: self.clone(),
                    context,
                })
            },
        };
        Ok(())
    }

    pub fn from_shape<C: ShapeConfig, S: ShapeCounter>(
        t: &Shape<C, S>,
        table_mapping: &NamespacedTableMapping,
        virtual_table_mapping: &VirtualTableMapping,
    ) -> Self {
        match t.variant() {
            ShapeEnum::Never => Self::Union(vec![]),
            ShapeEnum::Null => Self::Null,
            ShapeEnum::Int64 => Self::Int64,
            ShapeEnum::Float64 => Self::Float64,
            ShapeEnum::NegativeInf => Self::Float64,
            ShapeEnum::PositiveInf => Self::Float64,
            ShapeEnum::NegativeZero => Self::Float64,
            ShapeEnum::NaN => Self::Float64,
            ShapeEnum::NormalFloat64 => Self::Float64,
            ShapeEnum::Boolean => Self::Boolean,
            ShapeEnum::StringLiteral(s) => {
                Self::Literal(LiteralValidator::String(s.literal.clone()))
            },
            ShapeEnum::Id(table_number) => {
                match all_tables_number_to_name(table_mapping, virtual_table_mapping)(*table_number)
                {
                    Ok(table_name) => Self::Id(table_name),
                    Err(_) => Self::String,
                }
            },
            ShapeEnum::FieldName => Self::String,
            ShapeEnum::String => Self::String,
            ShapeEnum::Bytes => Self::Bytes,
            ShapeEnum::Array(array_type) => Self::Array(Box::new(Self::from_shape(
                array_type.element(),
                table_mapping,
                virtual_table_mapping,
            ))),
            ShapeEnum::Set(set_type) => Self::Set(Box::new(Self::from_shape(
                set_type.element(),
                table_mapping,
                virtual_table_mapping,
            ))),
            ShapeEnum::Map(map_type) => Self::Map(
                Box::new(Self::from_shape(
                    map_type.key(),
                    table_mapping,
                    virtual_table_mapping,
                )),
                Box::new(Self::from_shape(
                    map_type.value(),
                    table_mapping,
                    virtual_table_mapping,
                )),
            ),
            ShapeEnum::Object(object_type) => {
                let object_fields = object_type
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            FieldValidator {
                                validator: Self::from_shape(
                                    &v.value_shape,
                                    table_mapping,
                                    virtual_table_mapping,
                                ),
                                optional: v.optional,
                            },
                        )
                    })
                    .collect();
                Self::Object(ObjectValidator(object_fields))
            },
            ShapeEnum::Record(record_type) => Self::Record(
                Box::new(Self::from_shape(
                    record_type.field(),
                    table_mapping,
                    virtual_table_mapping,
                )),
                Box::new(Self::from_shape(
                    record_type.value(),
                    table_mapping,
                    virtual_table_mapping,
                )),
            ),
            ShapeEnum::Union(union_type) => Self::Union(
                union_type
                    .iter()
                    .map(|t| Self::from_shape(t, table_mapping, virtual_table_mapping))
                    .collect(),
            ),
            ShapeEnum::Unknown => Self::Any,
        }
    }

    /// A validator A is a subset of the validator B iff for every value that
    /// conforms to A, the value also conforms to B.
    ///
    /// This verification is used to know if a full table scan can be skipped
    /// when updating the schema. Hence, false negatives are permissible but
    /// false positives are not.
    pub fn is_subset(&self, superset: &Validator) -> bool {
        match (&self, &superset) {
            // Generic types
            (Validator::Array(left_contents), Validator::Array(right_contents))
            | (Validator::Set(left_contents), Validator::Set(right_contents)) => {
                left_contents.is_subset(right_contents)
            },
            (Validator::Map(left_keys, left_values), Validator::Map(right_keys, right_values)) => {
                left_keys.is_subset(right_keys) && left_values.is_subset(right_values)
            },
            (
                Validator::Object(ObjectValidator(left_fields)),
                Validator::Object(ObjectValidator(right_fields)),
            ) => {
                // No field disappears
                left_fields
                    .keys()
                    .all(|left_field_name| right_fields.contains_key(left_field_name))
                    && right_fields.iter().all(|(field, right_validator)| -> bool {
                        match left_fields.get(field) {
                            // Either a non-breaking change…
                            Some(left_validator) => {
                                (!left_validator.optional || right_validator.optional) // no mandatory → optional change
                                    && left_validator
                                        .validator
                                        .is_subset(&right_validator.validator)
                            },
                            // …or a new optional field
                            None => right_validator.optional,
                        }
                    })
            },

            // Identical types
            (v1, v2) if v1 == v2 => true,

            // Types that are subsets of other ones
            (_, Validator::Any)
            | (Validator::Literal(LiteralValidator::String(_)), Validator::String)
            | (Validator::Literal(LiteralValidator::Int64(_)), Validator::Int64)
            | (Validator::Literal(LiteralValidator::Float64(_)), Validator::Float64)
            | (Validator::Literal(LiteralValidator::Boolean(_)), Validator::Boolean)
            | (Validator::Id(_), Validator::String) => true,

            // Unions
            (Validator::Union(left_cases), _) => left_cases
                .iter()
                .all(|left_case| left_case.is_subset(superset)),
            (_, Validator::Union(cases)) => {
                if cases.iter().any(|case| self.is_subset(case)) {
                    true
                } else if let Validator::Boolean = self {
                    // Allow boolean ⊆ true | false
                    Validator::Literal(LiteralValidator::Boolean(true)).is_subset(superset)
                        && Validator::Literal(LiteralValidator::Boolean(false)).is_subset(superset)
                } else {
                    false
                }
            },

            _ => false,
        }
    }

    /// Is this something like `v.union(v.literal("foo"), v.literal("bar"))`
    /// These need to be treated differently if they are the key type for
    /// Validator::Record
    pub(crate) fn is_string_subtype_with_string_literal(&self) -> bool {
        match self {
            Validator::Id(_)
            | Validator::Null
            | Validator::Float64
            | Validator::Int64
            | Validator::Boolean
            | Validator::String
            | Validator::Bytes
            | Validator::Array(_)
            | Validator::Set(_)
            | Validator::Record(..)
            | Validator::Map(..)
            | Validator::Object(_)
            | Validator::Any => false,
            Validator::Literal(l) => match l {
                LiteralValidator::Float64(_)
                | LiteralValidator::Int64(_)
                | LiteralValidator::Boolean(_) => false,
                LiteralValidator::String(_) => true,
            },
            Validator::Union(unions) => unions
                .iter()
                .any(|v| v.is_string_subtype_with_string_literal()),
        }
    }

    /// Returns `true` when it is sometimes possible to have a field with the
    /// given path on the document if this table definition is enforced, or
    /// `false` when it is never possible.
    pub fn can_contain_field(&self, field_path: &FieldPath) -> bool {
        self._can_contain_field(field_path.fields())
    }

    fn _can_contain_field(&self, field_path_parts: &[IdentifierFieldName]) -> bool {
        let Some(first_part) = field_path_parts.first() else {
            return true;
        };

        match &self {
            Validator::Any => true,
            Validator::Union(cases) => cases
                .iter()
                .any(|case| case._can_contain_field(field_path_parts)),
            Validator::Object(ObjectValidator(fields)) => fields
                .get(first_part)
                .map(|field_validator| {
                    field_validator
                        .validator
                        ._can_contain_field(&field_path_parts[1..])
                })
                .unwrap_or(false),
            _ => false,
        }
    }

    /// Returns true if field_path points to a field where at least one allowed
    /// value for that field is could be Array<Float64>.
    ///
    /// Some weird cases - if any path in field_path is Any, we return true. If
    /// any path is a union and at least one of the unions has a path that
    /// matches our field_path that matches, we return true. If the field path
    /// points to an Array<Any> we also return true.
    pub fn overlaps_with_array_float64(&self, field_path: &FieldPath) -> bool {
        self._overlaps_with_array_float64(field_path.fields())
    }

    fn is_valid_vector_validator(validator: &Validator) -> bool {
        return match validator {
            Validator::Array(validator) => {
                matches!(**validator, Validator::Float64 | Validator::Any)
            },
            Validator::Any => true,
            Validator::Union(validators) => validators.iter().any(Self::is_valid_vector_validator),
            _ => false,
        };
    }

    fn _overlaps_with_array_float64(&self, field_path_parts: &[IdentifierFieldName]) -> bool {
        let Some(first_part) = field_path_parts.first() else {
            return Self::is_valid_vector_validator(self);
        };

        match &self {
            Validator::Any => true,
            Validator::Union(cases) => cases
                .iter()
                .any(|case| case._overlaps_with_array_float64(field_path_parts)),
            Validator::Object(ObjectValidator(fields)) => fields
                .get(first_part)
                .map(|field_validator| {
                    field_validator
                        .validator
                        ._overlaps_with_array_float64(&field_path_parts[1..])
                })
                .unwrap_or(true),
            _ => false,
        }
    }

    pub fn ensure_supported_for_streaming_export(&self) -> anyhow::Result<()> {
        match self {
            // Leaf values
            Validator::Id(_)
            | Validator::Null
            | Validator::Float64
            | Validator::Int64
            | Validator::Boolean
            | Validator::String
            | Validator::Bytes
            | Validator::Literal(_)
            // Values that map to `any`
            | Validator::Record(_, _)
            | Validator::Any => Ok(()),
            Validator::Array(element_validator) => {
                element_validator.ensure_supported_for_streaming_export()
            },
            Validator::Set(element_validator) => {
                element_validator.ensure_supported_for_streaming_export()
            },
            Validator::Map(key_validator, value_validator) => {
                key_validator.ensure_supported_for_streaming_export()?;
                value_validator.ensure_supported_for_streaming_export()
            },
            Validator::Object(object_validator) => {
                let fields = &object_validator.0;
                for field_validator in fields.values() {
                    field_validator.validator.ensure_supported_for_streaming_export()?
                }
                Ok(())
            },
            Validator::Union(validators) => {
                let mut num_objects = 0;
                for validator in validators {
                    if matches!(validator, Validator::Object(_)) {
                        num_objects += 1;
                    };
                    validator.ensure_supported_for_streaming_export()?
                };
                if num_objects > 1 {
                    Err(anyhow::anyhow!(ErrorMetadata::bad_request(
                        "UnsupportedSchemaForExport",
                        "Schema contains a union of objects, which is not supported for export"
                    )))
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn to_json_schema(&self, value_format: ValueFormat) -> JsonValue {
        let json_schema = match self {
            Validator::Id(table_name) => json_schemas::id(table_name),
            Validator::Null => json_schemas::null(),
            Validator::Float64 => json_schemas::float64(true, value_format),
            Validator::Int64 => json_schemas::int64(value_format),
            Validator::Boolean => json_schemas::boolean(),
            Validator::String => json_schemas::string(),
            Validator::Bytes => json_schemas::bytes(value_format),
            Validator::Literal(literal_validator) => match literal_validator {
                LiteralValidator::Float64(_) => json_schemas::float64(true, value_format),
                LiteralValidator::Int64(_) => json_schemas::int64(value_format),
                LiteralValidator::Boolean(_) => json_schemas::boolean(),
                LiteralValidator::String(_) => json_schemas::string(),
            },
            Validator::Array(element_validator) => {
                json_schemas::array(element_validator.to_json_schema(value_format))
            },
            Validator::Set(element_validator) => {
                json_schemas::set(element_validator.to_json_schema(value_format))
            },
            Validator::Record(..) => json_schemas::any(),
            Validator::Map(key_validator, value_validator) => json_schemas::map(
                key_validator.to_json_schema(value_format),
                value_validator.to_json_schema(value_format),
            ),
            Validator::Object(object_validator) => {
                object_validator.to_json_schema(AddTopLevelFields::False, value_format)
            },
            Validator::Union(validators) => {
                let options = validators
                    .iter()
                    .map(|v| v.to_json_schema(value_format))
                    .collect();
                json_schemas::union(options)
            },
            Validator::Any => json_schemas::any(),
        };
        json_schema
    }

    pub fn foreign_keys<'a>(&'a self) -> Box<dyn Iterator<Item = &'a TableName> + 'a> {
        Box::new(iter::from_coroutine(
            #[coroutine]
            move || match self {
                Self::Id(table_name) => yield table_name,
                Self::Object(object) => {
                    for table_name in object.foreign_keys() {
                        yield table_name;
                    }
                },
                Self::Array(item) | Self::Set(item) => {
                    for table_name in item.foreign_keys() {
                        yield table_name;
                    }
                },
                Self::Union(options) => {
                    for table_name in options.iter().flat_map(|option| option.foreign_keys()) {
                        yield table_name;
                    }
                },
                Self::Record(key, value) | Self::Map(key, value) => {
                    for table_name in key.foreign_keys() {
                        yield table_name;
                    }
                    for table_name in value.foreign_keys() {
                        yield table_name;
                    }
                },
                Self::Any
                | Self::Boolean
                | Self::Bytes
                | Self::String
                | Self::Literal(_)
                | Self::Null
                | Self::Float64
                | Self::Int64 => {},
            },
        ))
    }

    pub fn has_map_or_set(&self) -> bool {
        match self {
            Self::Id(_)
            | Self::Null
            | Self::Float64
            | Self::Int64
            | Self::Boolean
            | Self::String
            | Self::Bytes
            | Self::Literal(_)
            | Self::Any => false,
            Self::Set(_) | Self::Map(..) => true,
            Self::Array(a) => a.has_map_or_set(),
            Self::Record(k, v) => k.has_map_or_set() || v.has_map_or_set(),
            Self::Object(o) => o.has_map_or_set(),
            Self::Union(u) => u.iter().any(|o| o.has_map_or_set()),
        }
    }

    // Filter out `_id` and `_creationTime` at the top level
    pub fn filter_top_level_system_fields(self) -> Self {
        match self {
            Validator::Id(_)
            | Validator::Null
            | Validator::Float64
            | Validator::Int64
            | Validator::Boolean
            | Validator::String
            | Validator::Bytes
            | Validator::Literal(_)
            | Validator::Array(_)
            | Validator::Set(_)
            | Validator::Record(..)
            | Validator::Map(..)
            | Validator::Any => self,
            Validator::Object(o) => Validator::Object(o.filter_system_fields()),
            Validator::Union(validators) => Validator::Union(
                validators
                    .into_iter()
                    .map(|v| v.filter_top_level_system_fields())
                    .collect(),
            ),
        }
    }
}

impl From<DocumentSchema> for Validator {
    fn from(document_schema: DocumentSchema) -> Self {
        match document_schema {
            DocumentSchema::Any => Validator::Any,
            DocumentSchema::Union(validators) => {
                Validator::Union(validators.into_iter().map(Validator::Object).collect())
            },
        }
    }
}

impl From<Option<DocumentSchema>> for Validator {
    fn from(option: Option<DocumentSchema>) -> Self {
        match option {
            None => Validator::Any,
            Some(document_schema) => document_schema.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidationContext(Option<String>);

impl ValidationContext {
    pub fn new() -> Self {
        ValidationContext(None)
    }

    pub fn with(&self, new_context: String) -> Self {
        match &self.0 {
            Some(context) => Self(Some(format!("{context}{new_context}"))),
            None => Self(Some(new_context)),
        }
    }
}

impl Display for ValidationContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(context) = &self.0 {
            write!(f, "Path: {}", context)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialOrd, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum LiteralValidator {
    Float64(TotalOrdF64),
    Int64(i64),
    Boolean(bool),
    String(value::ConvexString),
}
impl Display for LiteralValidator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Attempt to display this with JSON. For the values that can't be
        // printed with JSON, fall back to the general type.
        let string = match self {
            LiteralValidator::Float64(float) => {
                if let Some(json_number) = Number::from_f64(f64::from(float.clone())) {
                    serde_json::to_string(&JsonValue::Number(json_number))
                } else {
                    Ok("<number>".to_string())
                }
            },
            LiteralValidator::Int64(_) => Ok("<bigint>".to_string()),
            LiteralValidator::Boolean(bool) => serde_json::to_string(&JsonValue::Bool(*bool)),
            LiteralValidator::String(string) => {
                serde_json::to_string(&JsonValue::String(string.clone().into()))
            },
        }
        .map_err(|_| fmt::Error)?;
        write!(f, "{}", string)
    }
}

impl From<LiteralValidator> for ConvexValue {
    fn from(literal: LiteralValidator) -> Self {
        match literal {
            LiteralValidator::Float64(float) => ConvexValue::Float64(float.into()),
            LiteralValidator::Int64(int) => ConvexValue::Int64(int),
            LiteralValidator::Boolean(bool) => ConvexValue::Boolean(bool),
            LiteralValidator::String(string) => ConvexValue::String(string),
        }
    }
}

impl TryFrom<ConvexValue> for LiteralValidator {
    type Error = anyhow::Error;

    fn try_from(v: ConvexValue) -> anyhow::Result<Self> {
        match v {
            ConvexValue::Float64(f) => Ok(LiteralValidator::Float64(f.into())),
            ConvexValue::Int64(i) => Ok(LiteralValidator::Int64(i)),
            ConvexValue::Boolean(b) => Ok(LiteralValidator::Boolean(b)),
            ConvexValue::String(s) => Ok(LiteralValidator::String(s.to_string().try_into()?)),
            _ => Err(anyhow::anyhow!("Value {v} is not a valid literal.")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(
    any(test, feature = "testing"),
    proptest(params = "BTreeSet<TableName>")
)]

pub struct ObjectValidator(
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(
            strategy = "prop::collection::btree_map(any::<IdentifierFieldName>(), \
                        any_with::<FieldValidator>(params), 0..8)"
        )
    )]
    pub BTreeMap<IdentifierFieldName, FieldValidator>,
);

#[macro_export]
macro_rules! object_validator {
    ($($field_name:expr => $field_type:expr),* $(,)?) => {
        {
            use $crate::schemas::validator::ObjectValidator;
            use std::collections::BTreeMap;
            #[allow(unused_mut)]
            let mut fields = BTreeMap::new();
            {
                $(fields.insert($field_name.to_string().parse()?, $field_type);)*
            }
            ObjectValidator(fields)
        }
    };
}

impl Display for ObjectValidator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        display_map(f, ["v.object({", "})"], self.0.iter())
    }
}

pub enum AddTopLevelFields {
    True(TableName),
    False,
}

impl ObjectValidator {
    pub fn has_validator_for_system_field(&self) -> bool {
        let fields = &self.0;
        fields.keys().any(|f| f.is_system())
    }

    pub fn filter_system_fields(self) -> Self {
        if !self.has_validator_for_system_field() {
            return self;
        }
        let fields = self.0;
        let filtered_fields = fields.into_iter().filter(|(f, _)| !f.is_system()).collect();
        Self(filtered_fields)
    }

    pub fn has_map_or_set(&self) -> bool {
        let fields = &self.0;
        fields.values().any(|f| f.has_map_or_set())
    }

    pub fn to_json_schema(
        &self,
        add_top_level_fields: AddTopLevelFields,
        value_format: ValueFormat,
    ) -> JsonValue {
        let fields = &self.0;
        let mut field_infos: BTreeMap<String, json_schemas::FieldInfo> = fields
            .iter()
            .map(|(field_name, field_validator)| {
                (
                    field_name.to_string(),
                    json_schemas::FieldInfo {
                        schema: field_validator.validator.to_json_schema(value_format),
                        optional: field_validator.optional,
                    },
                )
            })
            .collect();
        if let AddTopLevelFields::True(table_name) = add_top_level_fields {
            field_infos.insert(
                ID_FIELD.to_string(),
                json_schemas::FieldInfo {
                    schema: json_schemas::id(&table_name),
                    optional: false,
                },
            );
            field_infos.insert(
                CREATION_TIME_FIELD.to_string(),
                json_schemas::FieldInfo {
                    schema: json_schemas::float64(false, value_format),
                    optional: false,
                },
            );
        };
        json_schemas::object(field_infos)
    }

    pub fn foreign_keys(&self) -> impl Iterator<Item = &TableName> {
        self.0
            .values()
            .flat_map(|field| field.validator.foreign_keys())
    }
}

/// Object fields can be optional.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(
    any(test, feature = "testing"),
    proptest(params = "BTreeSet<TableName>")
)]
pub struct FieldValidator {
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "any_with::<Validator>(params)")
    )]
    pub(crate) validator: Validator,
    pub(crate) optional: bool,
}

impl FieldValidator {
    pub fn validator(&self) -> &Validator {
        &self.validator
    }

    pub fn required_field_type(validator: Validator) -> Self {
        Self {
            validator,
            optional: false,
        }
    }

    pub fn optional_field_type(validator: Validator) -> Self {
        Self {
            validator,
            optional: true,
        }
    }

    pub fn has_map_or_set(&self) -> bool {
        self.validator.has_map_or_set()
    }
}

impl Display for FieldValidator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.optional {
            write!(f, "v.optional({})", self.validator)
        } else {
            write!(f, "{}", self.validator)
        }
    }
}

#[derive(derive_more::Display, Debug, Clone, PartialEq)]
pub enum ValidationError {
    #[display(
        fmt = "Found ID \"{id}\" from table `{found_table_name}`, which does not match the table \
               name in validator `v.id(\"{validator_table}\")`.{context}"
    )]
    TableNamesDoNotMatch {
        id: DeveloperDocumentId,
        found_table_name: TableName,
        validator_table: TableName,
        context: ValidationContext,
    },
    #[display(
        fmt = "Found ID \"{id}\" from a system table, which does not match the table name in \
               validator `v.id(\"{validator_table}\")`.{context}"
    )]
    SystemTableReference {
        id: DeveloperDocumentId,
        validator_table: TableName,
        context: ValidationContext,
    },
    #[display(fmt = "`{value}` does not match literal validator \
                     `v.literal({literal_validator})`.{context}")]
    LiteralValuesDoNotMatch {
        value: ConvexValue,
        literal_validator: LiteralValidator,
        context: ValidationContext,
    },
    #[display(
        fmt = "Object is missing the required field `{field_name}`. Consider wrapping the field \
               validator in `v.optional(...)` if this is expected.
{context}
Object: {object}
Validator: {object_validator}"
    )]
    MissingRequiredField {
        object: ConvexObject,
        field_name: IdentifierFieldName,
        object_validator: ObjectValidator,
        context: ValidationContext,
    },
    #[display(
        fmt = "Object contains extra field `{field_name}` that is not in the validator.
{context}
Object: {object}
Validator: {object_validator}"
    )]
    ExtraField {
        object: ConvexObject,
        field_name: FieldName,
        object_validator: ObjectValidator,
        context: ValidationContext,
    },
    #[display(fmt = "Value does not match validator.
{context}
Value: {value}
Validator: {validator}")]
    NoMatch {
        value: ConvexValue,
        validator: Validator,
        context: ValidationContext,
    },
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        str::FromStr,
    };

    use cmd_util::env::env_config;
    use errors::ErrorMetadataAnyhowExt;
    use maplit::{
        btreemap,
        btreeset,
    };
    use proptest::prelude::*;
    use serde_json::json;
    use shape_inference::{
        testing::TestConfig,
        CountedShape,
    };
    use value::{
        array,
        assert_obj,
        assert_val,
        export::ValueFormat,
        id_v6::DeveloperDocumentId,
        ConvexObject,
        ConvexValue,
        ExcludeSetsAndMaps,
        FieldName,
        FieldType,
        InternalId,
        NamespacedTableMapping,
        TableMapping,
        TableName,
        TableNamespace,
        VirtualTableMapping,
    };

    use super::Validator;
    use crate::{
        schemas::{
            validator::{
                FieldValidator,
                LiteralValidator,
                ObjectValidator,
                ValidationContext,
                ValidationError,
            },
            DocumentSchema,
        },
        testing::TestIdGenerator,
    };

    fn empty_table_mapping() -> NamespacedTableMapping {
        TableMapping::new().namespace(TableNamespace::test_user())
    }

    // Arbitrary `TryFrom` implementation for testing `check_value`.
    fn value_from_validator(
        validator: Validator,
        id_generator: &TestIdGenerator,
    ) -> anyhow::Result<ConvexValue> {
        let value = match validator {
            Validator::Id(table_name) => {
                let id = InternalId::MIN;
                let namespaced_table_mapping = id_generator.namespace(TableNamespace::test_user());
                let table_number = match namespaced_table_mapping.name_to_id()(table_name.clone()) {
                    Err(_) => id_generator.virtual_table_mapping.number(&table_name)?,
                    Ok(id) => id.table_number,
                };
                let doc_idv6 = DeveloperDocumentId::new(table_number, id);
                ConvexValue::String(doc_idv6.encode().try_into()?)
            },
            Validator::Null => assert_val!(null),
            Validator::Float64 => assert_val!(0.),
            Validator::Int64 => assert_val!(0),
            Validator::Boolean => assert_val!(false),
            Validator::String => assert_val!(""),
            Validator::Bytes => ConvexValue::Bytes(vec![1, 2, 3].try_into()?),
            Validator::Literal(literal) => literal.into(),
            Validator::Array(v) => {
                assert_val!([value_from_validator(*v, id_generator)?])
            },
            Validator::Set(v) => {
                ConvexValue::Set(btreeset! { value_from_validator(*v, id_generator)? }.try_into()?)
            },
            Validator::Map(k, v) => {
                let key = value_from_validator(*k, id_generator)?;
                let map: BTreeMap<ConvexValue, ConvexValue> = btreemap! {
                    key => value_from_validator(*v, id_generator)?
                };
                ConvexValue::Map(map.try_into()?)
            },
            Validator::Record(k, v) => {
                let key = value_from_validator(*k, id_generator)?;
                let field_name = match key {
                    ConvexValue::String(s) => FieldName::from_str(&s)?,
                    _ => anyhow::bail!("Record key was not a string"),
                };
                assert_val!({field_name => value_from_validator(*v, id_generator)?})
            },
            Validator::Object(object) => {
                let map: BTreeMap<_, _> = object
                    .0
                    .into_iter()
                    .map(|(field_name, field_type)| {
                        let value = value_from_validator(field_type.validator, id_generator)?;
                        anyhow::Ok::<(FieldName, ConvexValue)>((field_name.into(), value))
                    })
                    .try_collect()?;
                ConvexValue::Object(map.try_into()?)
            },
            Validator::Union(validators) => {
                let validator = validators.into_iter().next().ok_or_else(|| {
                    anyhow::anyhow!("Union validator must have at least one validator")
                })?;
                value_from_validator(validator, id_generator)?
            },
            Validator::Any => assert_val!(null),
        };
        Ok(value)
    }

    // Arbitrary implementation for testing `check_object`.
    fn object_from_schema(
        schema: DocumentSchema,
        id_generator: &TestIdGenerator,
    ) -> anyhow::Result<ConvexObject> {
        match schema {
            DocumentSchema::Any => Ok(btreemap! {}.try_into()?),
            DocumentSchema::Union(objects) => {
                let object = objects.into_iter().next().ok_or_else(|| {
                    anyhow::anyhow!("Union validator must have at least one validator")
                })?;
                value_from_validator(Validator::Object(object), id_generator)?.try_into()
            },
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 64 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]
        #[test]
        fn test_check_value(v in any_with::<DocumentSchema>(btreeset! { "table_name".parse::<TableName>().unwrap(), "table_name_2".parse::<TableName>().unwrap()}, )) {
            let mut id_generator = TestIdGenerator::new();
            id_generator.user_generate(&"table_name".parse().unwrap());
            id_generator.generate_virtual(&"table_name_2".parse().unwrap());
            // Test that `check_value` succeeds for objects created from arbitrary document schemas.
            let object = object_from_schema(v.clone(), &id_generator).unwrap();
            v.check_value(
                &object,
                &id_generator.namespace(TableNamespace::test_user()),
                &id_generator.virtual_table_mapping
            ).unwrap();
        }

    }

    #[test]
    fn test_record_must_have_string_keys() -> anyhow::Result<()> {
        let validator_json = json!({
            "type": "object",
            "value": {
                "myArg": {
                    "fieldType": {
                        "type": "record",
                        "keys": {
                            "type": "number",
                        },
                        "values": {
                            "type": "number"
                        }
                    },
                    "optional": false
                },
            }
        });
        must_let::must_let!(let Err(e) = Validator::try_from(validator_json));
        assert_eq!(e.short_msg(), "InvalidRecordType");
        Ok(())
    }

    #[test]
    fn test_record_with_string_literal_keys_must_have_optional_value() -> anyhow::Result<()> {
        let validator_json = json!({
            "type": "object",
            "value": {
                "myArg": {
                    "fieldType": {
                        "type": "record",
                        "keys": {
                            "type": "number",
                        },
                        "values": {
                            "fieldType": { "type": "number" },
                            "optional": false,
                        }
                    },
                    "optional": false
                },
            }
        });
        must_let::must_let!(let Err(e) = Validator::try_from(validator_json));
        assert_eq!(e.short_msg(), "InvalidRecordType");
        Ok(())
    }

    #[test]
    fn test_record_can_have_string_subset_as_key() -> anyhow::Result<()> {
        let validator_json = json!({
            "type": "object",
            "value": {
                "myArg": {
                    "fieldType": {
                        "type": "record",
                        "keys": {
                            "type": "id",
                            "tableName": "users",
                        },
                        "values": {
                            "fieldType": { "type": "number" },
                            "optional": true,
                        }
                    },
                    "optional": false
                },
            }
        });
        assert!(Validator::try_from(validator_json).is_ok());
        Ok(())
    }

    #[test]
    fn test_record_key_any() -> anyhow::Result<()> {
        let validator_json = json!({
            "type": "object",
            "value": {
                "myArg": {
                    "fieldType": {
                        "type": "record",
                        "keys": {
                            "type": "any",
                        },
                        "values": {
                            "fieldType": { "type": "number" },
                            "optional": false,

                        }
                    },
                    "optional": false
                },
            }
        });
        must_let::must_let!(let Err(e) =Validator::try_from(validator_json));
        assert_eq!(e.short_msg(), "InvalidRecordType");
        Ok(())
    }

    #[test]
    fn test_record_check_value() -> anyhow::Result<()> {
        let mut id_generator = TestIdGenerator::new();
        let table_name: TableName = "users".parse()?;
        let key_validator = Validator::Id(table_name.clone());
        let value_validator = Validator::Float64;
        let validator = Validator::Record(
            Box::new(key_validator.clone()),
            Box::new(value_validator.clone()),
        );

        let value_wrong_type = ConvexValue::String("hello".try_into()?);
        let err = validator
            .check_value(
                &value_wrong_type,
                &id_generator.namespace(TableNamespace::test_user()),
                &VirtualTableMapping::new(),
            )
            .unwrap_err();
        assert_eq!(
            err,
            ValidationError::NoMatch {
                value: value_wrong_type,
                context: ValidationContext::new(),
                validator: validator.clone()
            }
        );

        let user_id1 = id_generator.user_generate(&table_name);
        let user_id2 = id_generator.user_generate(&table_name);

        let value_wrong_key: ConvexValue = assert_obj!(
            user_id1.to_string() => ConvexValue::Float64(0.0),
            user_id2.to_string() => ConvexValue::Float64(0.0),
            "hello" => ConvexValue::Float64(0.0),
        )
        .into();
        let err: ValidationError = validator
            .check_value(
                &value_wrong_key,
                &id_generator.namespace(TableNamespace::test_user()),
                &VirtualTableMapping::new(),
            )
            .unwrap_err();
        assert_eq!(
            err,
            ValidationError::NoMatch {
                value: ConvexValue::String("hello".try_into()?),
                context: ValidationContext::new().with(".keys()".to_string()),
                validator: key_validator
            }
        );

        let value_wrong_value: ConvexValue = assert_obj!(
            user_id1.to_string() => ConvexValue::Boolean(true),
            user_id2.to_string() => ConvexValue::Float64(0.0),
        )
        .into();
        let err: ValidationError = validator
            .check_value(
                &value_wrong_value,
                &id_generator.namespace(TableNamespace::test_user()),
                &VirtualTableMapping::new(),
            )
            .unwrap_err();
        assert_eq!(
            err,
            ValidationError::NoMatch {
                value: ConvexValue::Boolean(true),
                context: ValidationContext::new().with(".values()".to_string()),
                validator: value_validator
            }
        );

        Ok(())
    }

    #[test]
    fn test_record_check_value_with_virtual_ids() -> anyhow::Result<()> {
        let mut id_generator = TestIdGenerator::new();
        let table_name: TableName = "users".parse()?;
        let key_validator = Validator::Id(table_name.clone());
        let value_validator = Validator::Float64;
        let validator = Validator::Record(
            Box::new(key_validator.clone()),
            Box::new(value_validator.clone()),
        );

        let value_wrong_type = ConvexValue::String("hello".try_into()?);
        let err = validator
            .check_value(
                &value_wrong_type,
                &id_generator.namespace(TableNamespace::test_user()),
                &id_generator.virtual_table_mapping,
            )
            .unwrap_err();
        assert_eq!(
            err,
            ValidationError::NoMatch {
                value: value_wrong_type,
                context: ValidationContext::new(),
                validator: validator.clone()
            }
        );

        let user_id1 = id_generator.generate_virtual(&table_name);
        let user_id2 = id_generator.generate_virtual(&table_name);

        let value_wrong_key: ConvexValue = assert_obj!(
            user_id1.to_string() => ConvexValue::Float64(0.0),
            user_id2.to_string() => ConvexValue::Float64(0.0),
            "hello" => ConvexValue::Float64(0.0),
        )
        .into();
        let err: ValidationError = validator
            .check_value(
                &value_wrong_key,
                &id_generator.namespace(TableNamespace::test_user()),
                &id_generator.virtual_table_mapping,
            )
            .unwrap_err();
        assert_eq!(
            err,
            ValidationError::NoMatch {
                value: ConvexValue::String("hello".try_into()?),
                context: ValidationContext::new().with(".keys()".to_string()),
                validator: key_validator
            }
        );

        let value_wrong_value: ConvexValue = assert_obj!(
            user_id1.to_string() => ConvexValue::Boolean(true),
            user_id2.to_string() => ConvexValue::Float64(0.0),
        )
        .into();
        let err: ValidationError = validator
            .check_value(
                &value_wrong_value,
                &id_generator.namespace(TableNamespace::test_user()),
                &id_generator.virtual_table_mapping,
            )
            .unwrap_err();
        assert_eq!(
            err,
            ValidationError::NoMatch {
                value: ConvexValue::Boolean(true),
                context: ValidationContext::new().with(".values()".to_string()),
                validator: value_validator
            }
        );

        Ok(())
    }

    #[test]
    fn test_display() -> anyhow::Result<()> {
        // Test the display of our complex validator types.

        let id_validator = Validator::Id("tableName".parse()?);
        assert_eq!(id_validator.to_string(), "v.id(\"tableName\")");

        let float_literal = Validator::Literal(LiteralValidator::Float64(123f64.into()));
        assert_eq!(float_literal.to_string(), "v.literal(123.0)");

        let int_literal = Validator::Literal(LiteralValidator::Int64(123));
        assert_eq!(int_literal.to_string(), "v.literal(<bigint>)");

        let string_literal =
            Validator::Literal(LiteralValidator::String("abc".to_string().try_into()?));
        assert_eq!(string_literal.to_string(), "v.literal(\"abc\")");

        let boolean_literal = Validator::Literal(LiteralValidator::Boolean(true));
        assert_eq!(boolean_literal.to_string(), "v.literal(true)");

        let array_validator = Validator::Array(Box::new(Validator::String));
        assert_eq!(array_validator.to_string(), "v.array(v.string())");

        let set_validator = Validator::Set(Box::new(Validator::Float64));
        assert_eq!(set_validator.to_string(), "v.set(v.float64())");

        let map_validator =
            Validator::Map(Box::new(Validator::Int64), Box::new(Validator::Boolean));
        assert_eq!(map_validator.to_string(), "v.map(v.int64(), v.boolean())");

        let object_validator = Validator::Object(
            object_validator!("required" => FieldValidator::required_field_type(Validator::String), "optional" => FieldValidator::optional_field_type(Validator::Float64)),
        );
        assert_eq!(
            object_validator.to_string(),
            "v.object({optional: v.optional(v.float64()), required: v.string()})"
        );

        let union_validator = Validator::Union(vec![Validator::String, Validator::Float64]);
        assert_eq!(
            union_validator.to_string(),
            "v.union(v.string(), v.float64())"
        );

        Ok(())
    }

    #[test]
    fn test_id_match() -> anyhow::Result<()> {
        let table1: TableName = "table1".parse()?;
        let table2: TableName = "table2".parse()?;
        let id_validator = Validator::Id(table1.clone());
        let mut id_generator = TestIdGenerator::new();

        // generate an ID so it's in the table mapping
        id_generator.user_generate(&table1);
        let document_id = id_generator.user_generate(&table2);
        let id_v6 = DeveloperDocumentId::from(document_id);
        let value: ConvexValue = id_v6.into();

        let err = id_validator
            .check_value(
                &value,
                &id_generator.namespace(TableNamespace::test_user()),
                &id_generator.virtual_table_mapping,
            )
            .unwrap_err();
        assert_eq!(
            err,
            ValidationError::TableNamesDoNotMatch {
                validator_table: table1,
                context: ValidationContext::new(),
                id: id_v6,
                found_table_name: table2
            }
        );
        Ok(())
    }

    #[test]
    fn test_id_match_with_virtual_ids() -> anyhow::Result<()> {
        let table1: TableName = "table1".parse()?;
        let table2: TableName = "table2".parse()?;
        let id_validator = Validator::Id(table1.clone());
        let mut id_generator = TestIdGenerator::new();

        // generate an ID so it's in the table mapping
        id_generator.user_generate(&table1);
        let id_v6 = id_generator.generate_virtual(&table2);
        let value: ConvexValue = id_v6.into();

        let err = id_validator
            .check_value(
                &value,
                &id_generator.namespace(TableNamespace::test_user()),
                &id_generator.virtual_table_mapping,
            )
            .unwrap_err();
        assert_eq!(
            err,
            ValidationError::TableNamesDoNotMatch {
                validator_table: table1,
                context: ValidationContext::new(),
                id: id_v6,
                found_table_name: table2
            }
        );
        Ok(())
    }

    #[test]
    fn test_schema_literal_match() -> anyhow::Result<()> {
        let schema_literal = LiteralValidator::String("hello".to_string().try_into()?);
        let string_literal = Validator::Literal(schema_literal.clone());
        string_literal
            .check_value(
                &ConvexValue::String("hello".try_into()?),
                &empty_table_mapping(),
                &VirtualTableMapping::new(),
            )
            .unwrap();

        let value = ConvexValue::Int64(0);
        let err = string_literal
            .check_value(&value, &empty_table_mapping(), &VirtualTableMapping::new())
            .unwrap_err();
        assert_eq!(
            err,
            ValidationError::LiteralValuesDoNotMatch {
                value,
                literal_validator: schema_literal,
                context: ValidationContext::new()
            }
        );
        Ok(())
    }

    #[test]
    fn test_error_messages_include_context() -> anyhow::Result<()> {
        // The validator expects `property` to be an array of strings,
        // but it actually contains an int.
        let validator = Validator::Object(ObjectValidator(btreemap! {
            "property".parse()? => FieldValidator::required_field_type(
                Validator::Array(Box::new(Validator::String))
            )
        }));
        let object =
            ConvexValue::Object(assert_obj!("property" => ConvexValue::Array(array!(123.into())?)));

        // Check that the error message includes the path to the
        assert!(validator
            .check_value(&object, &empty_table_mapping(), &VirtualTableMapping::new())
            .unwrap_err()
            .to_string()
            .contains(".property[0]"));

        Ok(())
    }

    #[test]
    fn test_ensure_supported_for_streaming_export() -> anyhow::Result<()> {
        let simple_object_validator = Validator::Object(ObjectValidator(btreemap! {
            "property".parse()? => FieldValidator::required_field_type(Validator::String)
        }));
        assert!(simple_object_validator
            .ensure_supported_for_streaming_export()
            .is_ok());
        let any_validator = Validator::Any;
        assert!(any_validator
            .ensure_supported_for_streaming_export()
            .is_ok());

        let union_object_validator = Validator::Union(vec![
            Validator::Object(ObjectValidator(btreemap! {
                "propertyA".parse()? => FieldValidator::required_field_type(Validator::String)
            })),
            Validator::Object(ObjectValidator(btreemap! {
                "propertyB".parse()? => FieldValidator::required_field_type(Validator::String)
            })),
        ]);
        must_let::must_let!(
            let Err(e) = union_object_validator.ensure_supported_for_streaming_export()
        );
        assert_eq!(e.short_msg(), "UnsupportedSchemaForExport");
        let nested_union_object_validator = Validator::Array(Box::new(Validator::Union(vec![
            Validator::Object(ObjectValidator(btreemap! {
                "propertyA".parse()? => FieldValidator::required_field_type(Validator::String)
            })),
            Validator::Object(ObjectValidator(btreemap! {
                "propertyB".parse()? => FieldValidator::required_field_type(Validator::String)
            })),
        ])));
        must_let::must_let!(
            let Err(e) = nested_union_object_validator.ensure_supported_for_streaming_export()
        );
        assert_eq!(e.short_msg(), "UnsupportedSchemaForExport");

        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 64 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

        #[test]
        fn test_validators_are_subsets_of_themselves(validator in any_with::<Validator>(btreeset! { "table_name".parse::<TableName>().unwrap()})) {
            prop_assert!(validator.is_subset(&validator));
        }

        #[test]
        fn test_all_types_are_a_subset_of_any(validator in any_with::<Validator>(btreeset! { "table_name".parse::<TableName>().unwrap()})) {
            prop_assert!(validator.is_subset(&Validator::Any));
        }

        #[test]
        fn test_no_type_is_a_subset_of_never(validator in any_with::<Validator>(btreeset! { "table_name".parse::<TableName>().unwrap()})) {
            let never = Validator::Union(vec![]);
            prop_assert!(!validator.is_subset(&never));
        }

        #[test]
        fn test_union_of_one_element_is_equivalent_to_this_element(validator in any_with::<Validator>(btreeset! { "table_name".parse::<TableName>().unwrap()})) {
            let union_of_one = Validator::Union(vec![validator.clone()]);
            prop_assert!(validator.is_subset(&union_of_one));
            prop_assert!(union_of_one.is_subset(&validator));
        }

        #[test]
        fn test_to_json_schema(
            v in any_with::<Validator>(btreeset! { "table_name".parse::<TableName>().unwrap()}),
            value_format in any::<ValueFormat>(),
        ) {
            jsonschema::JSONSchema::compile(&v.to_json_schema(value_format)).unwrap();
        }

        #[test]
        fn test_validator_from_a_shape_validates_it(
            resolved_value in any_with::<ConvexValue>(
                (FieldType::User, ExcludeSetsAndMaps(false))
            )
        ) {
            let table_mapping = empty_table_mapping();
            let virtual_table_mapping = VirtualTableMapping::new();
            let shape = CountedShape::<TestConfig>::empty().insert_value(&resolved_value);
            let validator = Validator::from_shape(&shape, &table_mapping, &virtual_table_mapping);
            prop_assert!(validator.check_value(
                &resolved_value,
                &table_mapping,
                &virtual_table_mapping
            ).is_ok());
        }
    }
}
