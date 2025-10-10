/// ConvexValue::export creates human-readable JSON from a ConvexValue,
/// which might lose some information like make String and Bytes ambiguous.
/// This module allows ConvexValue to round trip.
///
/// Take a document D which exports as JSON J, in a table with Shape T.
/// there is an export context ExportContext::of(D, T)
/// such that D can be recovered from J, T, and ExportContext::of(D, T).
/// Which means that a document can round-trip through the export JSON,
/// as long as the Shape of its table, and possibly some document-specific
/// context are passed along.
use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    io::Cursor,
    iter,
};

use anyhow::Context;
use itertools::Itertools;
use maplit::btreeset;
use serde_json::{
    json,
    Value as JsonValue,
};
use value::{
    id_v6::DeveloperDocumentId,
    ConvexObject,
    ConvexValue,
    FieldName,
    IdentifierFieldName,
};

use crate::{
    Shape,
    ShapeConfig,
    ShapeCounter,
    ShapeEnum,
    StructuralShape,
};

/// Shape hint associated with a Convex value. This allows us to uniquely
/// convert the exported value back to the original Convex value.
///
/// For example, the document {foo: int64(5)} is exported as {"foo":"5"}, while
/// a hypothetical document {foo: string("5")} would have the same export.
/// There are two possibilites when encoding {foo: int64(5)}:
/// 1. the table's shape is {foo: int64} (or Record<string, int64>, etc). In
///    this case we know "5" should be decoded as int64, so we don't need any
///    special ExportContext. This is represented by ExportContext::Infer and
///    encoded as the document having no overrides.
/// 2. the table's shape is {foo: int64 | string} (or Unknown, etc). In this
///    case we don't know that "5" should be decoded as int64, so we use
///    ExportContext::Object({foo: ExportContext::Int64}) to allow decoding to
///    round-trip.
///
/// Now consider what happens when decoding the exported {"foo":"5"}.
/// 1. If the shape is {foo: int64}, we decode "5" as int64(5)
/// 2. If the ExportContext is Object({foo: Int64}), decode "5" as int64(5)
/// 3. If the shape is ambiguous *and* ExportContext is Infer, decode "5" as
///    string("5").
#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Clone)]
pub enum ExportContext {
    /// As described above, Infer can mean
    /// 1. the shape gives enough information to decode
    /// 2. if the shape is ambiguous, decode with the canonical format, i.e.
    ///    string.
    Infer,
    Int64,
    Float64NaN {
        // Store the f64 value in the export context when it is NaN, because the export format
        // assumes a single NaN value. This ensures that we can fully roundtrip values.
        nan_le_bytes: [u8; 8],
    },
    Float64Inf,
    Bytes,
    Array(Vec<ExportContext>),
    Object(BTreeMap<FieldName, ExportContext>),
}

impl<C: ShapeConfig, S: ShapeCounter> Shape<C, S> {
    fn union_options(&self) -> impl Iterator<Item = &Self> {
        // There are no nested unions, so this does not need to be recursive.
        iter::from_coroutine(
            #[coroutine]
            || {
                if let ShapeEnum::Union(options) = self.variant() {
                    for option in options.iter() {
                        yield option;
                    }
                } else {
                    yield self;
                }
            },
        )
    }

    fn array_element(&self) -> BTreeSet<&Self> {
        match self.variant() {
            ShapeEnum::Array(array) => btreeset!(array.element()),
            ShapeEnum::Union(options) => options
                .iter()
                .flat_map(|option| option.array_element())
                .collect(),
            ShapeEnum::Unknown => btreeset!(self),
            _ => btreeset!(),
        }
    }

    fn object_field(&self, field: &FieldName) -> BTreeSet<&Self> {
        match self.variant() {
            ShapeEnum::Object(object) => match IdentifierFieldName::try_from(field.clone()) {
                Ok(field) => object
                    .get(&field)
                    .into_iter()
                    .map(|object_field| &object_field.value_shape)
                    .collect(),
                Err(_) => btreeset!(),
            },
            ShapeEnum::Union(options) => options
                .iter()
                .flat_map(|option| option.object_field(field))
                .collect(),
            ShapeEnum::Record(record) => btreeset!(record.value()),
            ShapeEnum::Unknown => btreeset!(self),
            _ => btreeset!(),
        }
    }

    fn specified_fields(&self) -> BTreeSet<&IdentifierFieldName> {
        match self.variant() {
            ShapeEnum::Union(options) => options
                .iter()
                .flat_map(|option| option.specified_fields())
                .collect(),
            ShapeEnum::Object(object) => object.keys().collect(),
            _ => btreeset!(),
        }
    }

    /// What is the shape of an object's value at a key that does not appear in
    /// the shape.
    fn object_unspecified_field(&self) -> BTreeSet<&Self> {
        match self.variant() {
            ShapeEnum::Union(options) => options
                .iter()
                .flat_map(|option| option.object_unspecified_field())
                .collect(),
            ShapeEnum::Record(record) => btreeset!(record.value()),
            ShapeEnum::Unknown => btreeset!(self),
            _ => btreeset!(),
        }
    }
}

impl ExportContext {
    pub fn is_infer(&self) -> bool {
        matches!(self, Self::Infer)
    }

    pub fn of<C: ShapeConfig, S: ShapeCounter>(
        value: &ConvexValue,
        shape: &Shape<C, S>,
    ) -> ExportContext {
        let shape_options = btreeset!(shape);
        Self::of_inner(value, &shape_options)
    }

    pub fn of_object<C: ShapeConfig, S: ShapeCounter>(
        object: &ConvexObject,
        shape: &Shape<C, S>,
    ) -> ExportContext {
        let shape_options = btreeset!(shape);
        Self::of_object_inner(object, &shape_options)
    }

    fn of_object_inner<C: ShapeConfig, S: ShapeCounter>(
        fields: &ConvexObject,
        shape: &BTreeSet<&Shape<C, S>>,
    ) -> ExportContext {
        if !Self::is_ambiguous_inner(shape) {
            return ExportContext::Infer;
        }
        let element_contexts: BTreeMap<_, _> = fields
            .iter()
            .filter_map(|(key, value)| {
                let inner_shape = shape
                    .iter()
                    .flat_map(|shape| shape.object_field(key))
                    .collect();
                let value_context = ExportContext::of_inner(value, &inner_shape);
                if value_context.is_infer() {
                    None
                } else {
                    Some((key.clone(), value_context))
                }
            })
            .collect();
        if element_contexts.is_empty() {
            ExportContext::Infer
        } else {
            ExportContext::Object(element_contexts)
        }
    }

    pub fn of_inner<C: ShapeConfig, S: ShapeCounter>(
        value: &ConvexValue,
        shape: &BTreeSet<&Shape<C, S>>,
    ) -> ExportContext {
        if !Self::is_ambiguous_inner(shape) {
            return ExportContext::Infer;
        }
        match value {
            ConvexValue::Null => ExportContext::Infer,
            ConvexValue::Int64(_) => {
                if Self::inferred_context_for_string(shape).is_some() {
                    ExportContext::Infer
                } else {
                    ExportContext::Int64
                }
            },
            ConvexValue::Float64(f) => {
                if f.is_nan() {
                    ExportContext::Float64NaN {
                        nan_le_bytes: f.to_le_bytes(),
                    }
                } else if f.is_infinite() {
                    if Self::inferred_context_for_string(shape).is_some() {
                        ExportContext::Infer
                    } else {
                        ExportContext::Float64Inf
                    }
                } else {
                    ExportContext::Infer
                }
            },
            ConvexValue::Boolean(_) => ExportContext::Infer,
            ConvexValue::String(_) => ExportContext::Infer,
            ConvexValue::Bytes(_) => {
                if Self::inferred_context_for_string(shape).is_some() {
                    ExportContext::Infer
                } else {
                    ExportContext::Bytes
                }
            },
            ConvexValue::Array(elements) => {
                let inner_shape = shape
                    .iter()
                    .flat_map(|shape| shape.array_element())
                    .collect();
                let element_contexts: Vec<_> = elements
                    .iter()
                    .map(|element| ExportContext::of_inner(element, &inner_shape))
                    .collect();
                if element_contexts
                    .iter()
                    .all(|context| matches!(context, ExportContext::Infer))
                {
                    ExportContext::Infer
                } else {
                    ExportContext::Array(element_contexts)
                }
            },
            ConvexValue::Object(fields) => Self::of_object_inner(fields, shape),
        }
    }

    /// Returns true if all values with the given shape can use
    /// ExportContext::Infer.
    pub fn is_ambiguous<C: ShapeConfig, S: ShapeCounter>(shape: &Shape<C, S>) -> bool {
        Self::is_ambiguous_inner(&btreeset! {shape})
    }

    fn is_ambiguous_inner<C: ShapeConfig, S: ShapeCounter>(
        shape_options: &BTreeSet<&Shape<C, S>>,
    ) -> bool {
        if shape_options.is_empty() {
            return false;
        }
        // Needs ExportContext when exported value is a string?
        if Self::possible_contexts_for_string(shape_options)
            .take(2)
            .count()
            == 2
        {
            return true;
        }
        // Needs ExportContext when exported value is an array?
        let array_element_shape = shape_options
            .iter()
            .flat_map(|shape| shape.array_element())
            .collect();
        if Self::is_ambiguous_inner(&array_element_shape) {
            return true;
        }
        // Needs ExportContext when exported value is an object?
        // Part 1: the object shape itself requires ExportContext.
        if shape_options
            .iter()
            .flat_map(|shape| shape.union_options())
            .any(|shape| matches!(shape.variant(), ShapeEnum::Unknown))
        {
            return true;
        }
        // Part 2: object[key] where key doesn't appear in the shape may be ambiguous.
        // e.g. record<k, string|int64>, or record<k1, string>|record<k2, int64>
        let unspecified_field_shape = shape_options
            .iter()
            .flat_map(|shape| shape.object_unspecified_field())
            .collect();
        if Self::is_ambiguous_inner(&unspecified_field_shape) {
            return true;
        }
        // Part 3: object[key] where key does appear in the shape may be ambiguous.
        // e.g. {k: string|int64}, or {k1: string}|record<k2, int64>
        let specified_fields: BTreeSet<_> = shape_options
            .iter()
            .flat_map(|shape| shape.specified_fields())
            .map(|field| FieldName::from(field.clone()))
            .collect();
        for field in specified_fields.iter() {
            let field_shape = shape_options
                .iter()
                .flat_map(|shape| shape.object_field(field))
                .collect();
            if Self::is_ambiguous_inner(&field_shape) {
                return true;
            }
        }

        false
    }

    /// If the given shapes can export a String in exactly one way, return that
    /// way. If it's ambiguous, returns None.
    fn inferred_context_for_string<C: ShapeConfig, S: ShapeCounter>(
        shape_options: &BTreeSet<&Shape<C, S>>,
    ) -> Option<Self> {
        let mut possibilities = Self::possible_contexts_for_string(shape_options);
        let first = possibilities.next()?;
        match possibilities.next() {
            None => Some(first),
            Some(_) => None,
        }
    }

    /// Given a set of possible shapes, knowing that the exported value is a
    /// string, what are the possible ExportContexts?
    /// e.g. if the shapes are Union(Array(null), Int64, String), the possible
    /// export contexts are Int64 and Infer.
    fn possible_contexts_for_string<'a, C: ShapeConfig, S: ShapeCounter>(
        shape_options: &'a BTreeSet<&'a Shape<C, S>>,
    ) -> impl Iterator<Item = Self> + 'a {
        iter::from_coroutine(
            #[coroutine]
            move || {
                for shape in shape_options
                    .iter()
                    .flat_map(|option| option.union_options())
                {
                    match shape.variant() {
                        ShapeEnum::Int64 => yield ExportContext::Int64,
                        ShapeEnum::NegativeInf | ShapeEnum::PositiveInf => {
                            yield ExportContext::Float64Inf
                        },
                        ShapeEnum::NaN | ShapeEnum::Float64 => {
                            // We care about whether this function yields multiple distinct results,
                            // to see if we can use Infer. There are
                            // multiple values of NaN, and multiple
                            // kinds of floats, so we mark NaN as not
                            // inferrable by returning multiple distinct results.
                            yield ExportContext::Float64NaN {
                                nan_le_bytes: f64::NAN.to_le_bytes(),
                            };
                            yield ExportContext::Float64Inf;
                        },
                        ShapeEnum::StringLiteral(_)
                        | ShapeEnum::Id(_)
                        | ShapeEnum::FieldName
                        | ShapeEnum::String => yield ExportContext::Infer,
                        ShapeEnum::Bytes => yield ExportContext::Bytes,
                        // Unknown could have any ExportContext that can be a string.
                        ShapeEnum::Unknown => {
                            yield ExportContext::Infer;
                            yield ExportContext::Float64Inf;
                            yield ExportContext::Float64NaN {
                                nan_le_bytes: f64::NAN.to_le_bytes(),
                            };
                            yield ExportContext::Int64;
                            yield ExportContext::Bytes;
                        },
                        // coroutine cannot be recursive, so unions are already handled by
                        // union_options() above.
                        ShapeEnum::Union(_) => unreachable!(),
                        // Never exported as strings.
                        ShapeEnum::Never
                        | ShapeEnum::Null
                        | ShapeEnum::NegativeZero
                        | ShapeEnum::NormalFloat64
                        | ShapeEnum::Boolean
                        | ShapeEnum::Array(_)
                        | ShapeEnum::Object(_)
                        | ShapeEnum::Record(_) => {},
                    }
                }
            },
        )
        .dedup()
    }

    pub fn apply<C: ShapeConfig, S: ShapeCounter>(
        self,
        exported_value: JsonValue,
        shape: &Shape<C, S>,
    ) -> anyhow::Result<ConvexValue> {
        let shape_options = btreeset!(shape);
        self.apply_inner(exported_value, &shape_options)
    }

    fn apply_inner<C: ShapeConfig, S: ShapeCounter>(
        self,
        exported_value: JsonValue,
        shape: &BTreeSet<&Shape<C, S>>,
    ) -> anyhow::Result<ConvexValue> {
        match exported_value {
            JsonValue::Null => Ok(ConvexValue::Null),
            JsonValue::Bool(value) => Ok(value.into()),
            JsonValue::Number(n) => n
                .as_f64()
                .map(ConvexValue::from)
                .context("Unexpected number for i64"),
            JsonValue::String(value) => {
                let inferred_context = if self.is_infer() {
                    if let Some(inferred) = Self::inferred_context_for_string(shape) {
                        inferred
                    } else {
                        self
                    }
                } else {
                    self
                };
                match inferred_context {
                    Self::Infer => value.try_into(),
                    Self::Int64 => value
                        .parse::<i64>()
                        .map(ConvexValue::from)
                        .context("Unexpected string for i64"),
                    Self::Float64NaN { nan_le_bytes } => {
                        let nan_value = f64::from_le_bytes(nan_le_bytes);
                        if !nan_value.is_nan() {
                            anyhow::bail!("Unexpected non-NaN value in the export context");
                        }

                        if &value != "NaN" {
                            anyhow::bail!("Unexpected serialization of a NaN value");
                        }

                        Ok(nan_value.into())
                    },
                    Self::Float64Inf => match value.as_ref() {
                        "Infinity" => Ok(f64::INFINITY.into()),
                        "-Infinity" => Ok(f64::NEG_INFINITY.into()),
                        _ => anyhow::bail!("Unexpected string for f64"),
                    },
                    Self::Bytes => ConvexValue::try_from(base64::decode(value)?),
                    Self::Array(_) | Self::Object(_) => {
                        anyhow::bail!("unexpected shape hint for string")
                    },
                }
            },
            JsonValue::Array(exported_values) => match self {
                Self::Infer => {
                    let mut values = vec![];
                    let element_shape = shape
                        .iter()
                        .flat_map(|shape| shape.array_element())
                        .collect();
                    for exported_value in exported_values {
                        values.push(Self::Infer.apply_inner(exported_value, &element_shape)?);
                    }
                    ConvexValue::try_from(values)
                },
                Self::Array(shape_hints) => {
                    if exported_values.len() != shape_hints.len() {
                        anyhow::bail!("Array lengths do not match");
                    }

                    let mut values = vec![];
                    let element_shape = shape
                        .iter()
                        .flat_map(|shape| shape.array_element())
                        .collect();
                    for (exported_value, shape_hint) in exported_values.into_iter().zip(shape_hints)
                    {
                        values.push(shape_hint.apply_inner(exported_value, &element_shape)?);
                    }
                    ConvexValue::try_from(values)
                },
                Self::Bytes
                | Self::Float64NaN { .. }
                | Self::Float64Inf
                | Self::Int64
                | Self::Object(_) => anyhow::bail!("unsupported shape hint for array value"),
            },
            JsonValue::Object(exported_values) => match self {
                Self::Infer => {
                    let entries: BTreeMap<FieldName, ConvexValue> = exported_values
                        .into_iter()
                        .map(|(key, value)| {
                            let field: FieldName = key.parse()?;
                            let field_shape = shape
                                .iter()
                                .flat_map(|shape| shape.object_field(&field))
                                .collect();
                            anyhow::Ok((field, Self::Infer.apply_inner(value, &field_shape)?))
                        })
                        .try_collect()?;

                    Ok(ConvexValue::Object(entries.try_into()?))
                },
                Self::Object(mut shape_hints) => {
                    let entries: BTreeMap<FieldName, ConvexValue> = exported_values
                        .into_iter()
                        .map(|(key, value)| {
                            let field: FieldName = key.parse()?;
                            let field_shape = shape
                                .iter()
                                .flat_map(|shape| shape.object_field(&field))
                                .collect();
                            let shape_hint =
                                shape_hints.remove(&field).unwrap_or(ExportContext::Infer);
                            anyhow::Ok((field, shape_hint.apply_inner(value, &field_shape)?))
                        })
                        .try_collect()?;

                    Ok(ConvexValue::Object(entries.try_into()?))
                },
                Self::Int64
                | Self::Float64NaN { .. }
                | Self::Float64Inf
                | Self::Bytes
                | Self::Array(_) => anyhow::bail!("unsupported shape hint for object value"),
            },
        }
    }
}

impl From<ExportContext> for JsonValue {
    fn from(value: ExportContext) -> Self {
        match value {
            ExportContext::Infer => json!("infer"),
            ExportContext::Int64 => json!("int64"),
            ExportContext::Float64Inf => json!("float64inf"),
            ExportContext::Bytes => json!("bytes"),
            ExportContext::Float64NaN { nan_le_bytes } => {
                json!({"$float64NaN": base64::encode(nan_le_bytes) })
            },
            ExportContext::Array(array) => {
                json!(array.into_iter().map(JsonValue::from).collect_vec())
            },
            ExportContext::Object(object) => json!(object
                .into_iter()
                .map(|(k, v)| (k.to_string(), JsonValue::from(v)))
                .collect::<BTreeMap<_, _>>()),
        }
    }
}

impl TryFrom<JsonValue> for ExportContext {
    type Error = anyhow::Error;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        let export_context = match value {
            JsonValue::String(s) => match &*s {
                "infer" => Self::Infer,
                "int64" => Self::Int64,
                "float64inf" => Self::Float64Inf,
                "bytes" => Self::Bytes,
                _ => anyhow::bail!("invalid export context {s}"),
            },
            JsonValue::Array(array) => ExportContext::Array(
                array
                    .into_iter()
                    .map(ExportContext::try_from)
                    .try_collect()?,
            ),
            JsonValue::Object(mut object) => {
                if let Some(nan_value) = object.remove("$float64NaN")
                    && let JsonValue::String(nan_value_str) = nan_value
                {
                    let nan_le_bytes: [u8; 8] = base64::decode(nan_value_str.as_bytes())?
                        .try_into()
                        .map_err(|_| anyhow::anyhow!("Float64 must be exactly eight bytes"))?;
                    ExportContext::Float64NaN { nan_le_bytes }
                } else {
                    ExportContext::Object(
                        object
                            .into_iter()
                            .map(|(k, v)| anyhow::Ok((k.parse()?, ExportContext::try_from(v)?)))
                            .try_collect()?,
                    )
                }
            },
            _ => anyhow::bail!("invalid export context {value}"),
        };
        Ok(export_context)
    }
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use maplit::btreemap;
    use proptest::prelude::*;
    use serde_json::Value as JsonValue;
    use sync_types::testing::assert_roundtrips;
    use value::{
        assert_obj,
        assert_val,
        export::ValueFormat,
        ConvexValue,
    };

    use crate::{
        export_context::ExportContext,
        testing::SmallTestConfig,
        CountedShape,
        Shape,
        ShapeEnum,
        StructuralShape,
    };

    fn test_export_context_with_shape(
        value: ConvexValue,
        expected_context: ExportContext,
        shape: StructuralShape<SmallTestConfig>,
        inferred_yet_ambiguous: bool,
    ) {
        let is_ambiguous = ExportContext::is_ambiguous(&shape);
        if expected_context.is_infer() {
            if !inferred_yet_ambiguous {
                assert!(!is_ambiguous, "shape should not be ambiguous: {shape}");
            }
        } else {
            assert!(is_ambiguous, "shape should be ambiguous: {shape}");
        }
        let shape_hint = ExportContext::of(&value, &shape);
        assert_eq!(shape_hint, expected_context);
        let exported = value.clone().export(ValueFormat::ConvexCleanJSON);
        let recreated_value = shape_hint.apply(exported, &shape).unwrap();
        assert_eq!(value, recreated_value);
    }

    fn test_export_context(value: ConvexValue, expected_context: ExportContext) {
        let shape = Shape::structural_shape_of(&value);
        test_export_context_with_shape(value, expected_context, shape, false)
    }

    fn test_inferred(value: ConvexValue) {
        test_export_context(value, ExportContext::Infer);
    }

    #[test]
    fn test_array_of_ints() {
        test_inferred(assert_val!([1, 2, 3, 4]));
    }

    #[test]
    fn test_array_of_strings() {
        // literals
        test_inferred(assert_val!(["a", "b"]));
        // strings
        test_inferred(assert_val!(["a", "b", "c", "d", "e", "f", "g", "h"]));
        // ids
        test_inferred(assert_val!(["3yvf0j22p6ez1rqyy5m379kz9kh4sar"]));
    }

    #[test]
    fn test_heterogenous_array_where_strings_are_strings() {
        // array<string | null>
        test_inferred(assert_val!(["1", null]));
    }

    #[test]
    fn test_heterogenous_array_where_strings_are_ints() {
        // array<int64 | null>
        test_inferred(assert_val!([1, null]));
    }

    #[test]
    fn test_unknown_array_where_strings_are_strings() {
        // array<unknown>
        let value = assert_val!(["1", null, 1.0, true]);
        let shape = Shape::structural_shape_of(&value);
        test_export_context_with_shape(value, ExportContext::Infer, shape, true);
    }

    #[test]
    fn test_unknown_array_where_strings_are_ints() {
        // array<unknown>
        test_export_context(
            assert_val!([1, null, 1.0, true]),
            ExportContext::Array(vec![
                ExportContext::Int64,
                ExportContext::Infer,
                ExportContext::Infer,
                ExportContext::Infer,
            ]),
        );
    }

    #[test]
    fn test_array_of_int_and_string() {
        test_export_context(
            assert_val!([1, "1"]),
            ExportContext::Array(vec![ExportContext::Int64, ExportContext::Infer]),
        );
    }

    #[test]
    fn test_array_of_bytes() {
        let bytes1 = ConvexValue::Bytes(vec![0, 1].try_into().unwrap());
        let bytes2 = ConvexValue::Bytes(vec![0, 2].try_into().unwrap());
        test_inferred(assert_val!([bytes1, bytes2]));
    }

    #[test]
    fn test_array_of_bytes_and_string() {
        let bytes = ConvexValue::Bytes(vec![0, 2].try_into().unwrap());
        test_export_context(
            assert_val!([bytes, "1"]),
            ExportContext::Array(vec![ExportContext::Bytes, ExportContext::Infer]),
        );
    }

    #[test]
    fn test_primitives() {
        test_export_context(
            assert_val!(f64::NAN),
            ExportContext::Float64NaN {
                nan_le_bytes: f64::NAN.to_le_bytes(),
            },
        );
        test_inferred(assert_val!(f64::INFINITY));
        test_inferred(assert_val!(f64::NEG_INFINITY));
        test_inferred(assert_val!(-0.0));
        test_inferred(assert_val!(1.0));
        test_inferred(assert_val!(null));
        test_inferred(assert_val!(true));
        test_inferred(assert_val!(false));
    }

    #[test]
    fn test_array_of_floats() {
        test_inferred(assert_val!([1.0, 2.0, 3.0]));
        test_inferred(assert_val!([f64::NEG_INFINITY, 1.0]));
        test_inferred(assert_val!([f64::INFINITY, 1.0]));
        test_inferred(assert_val!([f64::INFINITY, f64::NEG_INFINITY]));
        // Why are the above arrays inferred but this next one not?
        // Because -inf, 0, inf are distinct Shapes, and
        // SmallTestConfig::MAX_UNION_LENGTH is 2, so the Shape collapses to
        // Array<Float64>. And Float64 contains NaN, so we don't know if the
        // array contains NaN. There are multi[ple values of NaN so it can't be
        // inferred at all.
        // We could potentially optimize this case by checking for the existence
        // of NaN in the array, but MAX_UNION_LENGTH>2 in prod and this
        // case is unlikely to begin with.
        test_export_context(
            assert_val!([f64::NEG_INFINITY, 1.0, f64::INFINITY]),
            ExportContext::Array(vec![
                ExportContext::Float64Inf,
                ExportContext::Infer,
                ExportContext::Float64Inf,
            ]),
        );
        test_export_context(
            assert_val!([f64::NEG_INFINITY, 1.0, f64::NAN]),
            ExportContext::Array(vec![
                ExportContext::Float64Inf,
                ExportContext::Infer,
                ExportContext::Float64NaN {
                    nan_le_bytes: f64::NAN.to_le_bytes(),
                },
            ]),
        );
    }

    #[test]
    fn test_objects_different_keys() {
        test_inferred(assert_val!([{"a" => 1}, {"b" => "1"}]));
    }

    #[test]
    fn test_record_same_value() {
        test_inferred(assert_val!([{"a" => 1}, {"b" => 2}, {"c" => 3}, {"d" => 4}]));
    }

    #[test]
    fn test_record_different_values() {
        test_export_context(
            assert_val!([{"a" => "a"}, {"b" => 2}, {"c" => 3}, {"d" => 4}]),
            ExportContext::Array(vec![
                ExportContext::Infer,
                ExportContext::Object(btreemap! {"b".parse().unwrap() => ExportContext::Int64}),
                ExportContext::Object(btreemap! {"c".parse().unwrap() => ExportContext::Int64}),
                ExportContext::Object(btreemap! {"d".parse().unwrap() => ExportContext::Int64}),
            ]),
        );
    }

    #[test]
    fn test_objects_same_key() {
        test_export_context(
            assert_val!([{"a" => 1}, {"a" => "1"}]),
            ExportContext::Array(vec![
                ExportContext::Object(btreemap! {"a".parse().unwrap() => ExportContext::Int64}),
                ExportContext::Infer,
            ]),
        );
    }

    #[test]
    fn test_objects_multiple_same_keys() {
        // Key "b" can be inferred, so it's omitted entirely.
        test_export_context(
            assert_val!([{"a" => 1, "b" => 2}, {"a" => "1", "b" => 2}]),
            ExportContext::Array(vec![
                ExportContext::Object(btreemap! {"a".parse().unwrap() => ExportContext::Int64}),
                ExportContext::Infer,
            ]),
        );
    }

    #[test]
    fn test_discriminated_union() {
        // Objects with discriminator fields so the Shape is a nested union.
        let obj1 = assert_obj!("d" => 1, "f" => {"d" => 1, "v" => 0});
        let obj2 = assert_obj!("d" => 1, "f" => {"e" => 1, "v" => "$_[]+=+"});
        let obj3 = assert_obj!("e" => 1, "f" => {"d" => 1, "v" => 0});
        let obj4 = assert_obj!("e" => 1, "f" => {"e" => 1, "v" => 0});
        let shape = StructuralShape::from(
            &CountedShape::<SmallTestConfig>::empty()
                .insert(&obj1)
                .insert(&obj2)
                .insert(&obj3)
                .insert(&obj4),
        );
        // Double check that the shape didn't get collapsed.
        assert_eq!(
            shape.to_string(),
            r#"{"d": int64, "f": {"d": int64, "v": int64} | {"e": int64, "v": string}} | {"e": int64, "f": {"d": int64, "v": int64} | {"e": int64, "v": int64}}"#
        );
        // Since f.v is int64 | string, when it's int64 we need to attach ExportContext,
        // even though the string only appears in one part of the discriminated union.
        let export_context = ExportContext::Object(
            btreemap! {"f".parse().unwrap() => ExportContext::Object(btreemap! {"v".parse().unwrap() => ExportContext::Int64})},
        );
        test_export_context_with_shape(
            assert_val!(obj1),
            export_context.clone(),
            shape.clone(),
            false,
        );
        test_export_context_with_shape(
            assert_val!(obj2),
            ExportContext::Infer,
            shape.clone(),
            true,
        );
        test_export_context_with_shape(
            assert_val!(obj3),
            export_context.clone(),
            shape.clone(),
            false,
        );
        test_export_context_with_shape(assert_val!(obj4), export_context.clone(), shape, false);
    }

    #[test]
    fn test_is_ambiguous() {
        let object_mismatch_different_fields = CountedShape::<SmallTestConfig>::empty()
            .insert(&assert_obj!("a" => [1, 2]))
            .insert(&assert_obj!("b" => ["1", "2"]));
        assert!(!ExportContext::is_ambiguous(
            &object_mismatch_different_fields
        ));
        let object_mismatch_same_field = CountedShape::<SmallTestConfig>::empty()
            .insert(&assert_obj!("a" => [1, 2]))
            .insert(&assert_obj!("a" => ["1", "2"]));
        assert!(ExportContext::is_ambiguous(&object_mismatch_same_field));
        let record_same_shape = CountedShape::<SmallTestConfig>::empty()
            .insert(&assert_obj!("a]" => [1, 2]))
            .insert(&assert_obj!("a" => [2, 3]));
        assert!(!ExportContext::is_ambiguous(&record_same_shape));
        let record_mismatch_object = CountedShape::<SmallTestConfig>::empty()
            .insert(&assert_obj!("a]" => [1, 2]))
            .insert(&assert_obj!("a" => ["1"]));
        assert!(ExportContext::is_ambiguous(&record_mismatch_object));
        let record_mismatch_record = CountedShape::<SmallTestConfig>::empty()
            .insert(&assert_obj!("a]" => [1, 2]))
            .insert(&assert_obj!("b]" => ["1"]));
        assert!(ExportContext::is_ambiguous(&record_mismatch_record));
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            failure_persistence: None, cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), ..ProptestConfig::default()
        })]
        #[test]
        fn export_context_json_roundtrip(export_context in any::<ExportContext>()) {
            assert_roundtrips::<ExportContext, JsonValue>(export_context);
        }

        #[test]
        fn export_roundtrips_with_arbitrary_shape_hint(
            value in any::<ConvexValue>(),
            shape in any::<CountedShape<SmallTestConfig>>(),
        ) {
            let exported_value = value.clone().export(ValueFormat::ConvexCleanJSON);
            let shape = shape.insert_value(&value);
            let shape = StructuralShape::from(&shape);
            let shape_hint = ExportContext::of(&value, &shape);

            prop_assert_eq!(
                value,
                shape_hint.apply(exported_value, &shape).unwrap()
            );
        }

        /// Slimmed down version of the test above to make sure ExportContext
        /// contains enough information to round-trip when the shape is the
        /// most accurate, so the shape hint is often Infer.
        #[test]
        fn export_roundtrips_with_minimal_shape_hint(
            value in any::<ConvexValue>(),
        ) {
            let exported_value = value.clone().export(ValueFormat::ConvexCleanJSON);
            let shape = StructuralShape::<SmallTestConfig>::structural_shape_of(&value);
            let shape_hint = ExportContext::of(&value, &shape);

            prop_assert_eq!(
                value,
                shape_hint.apply(exported_value, &shape).unwrap()
            );
        }

        /// Slimmed down version of the test above to make sure ExportContext
        /// contains enough information to round-trip even when the shape is
        /// useless.
        #[test]
        fn export_roundtrips_with_maximal_shape_hint(
            value in any::<ConvexValue>(),
        ) {
            let exported_value = value.clone().export(ValueFormat::ConvexCleanJSON);
            let shape = StructuralShape::<SmallTestConfig>::new(ShapeEnum::Unknown);
            let shape_hint = ExportContext::of(&value, &shape);

            prop_assert_eq!(
                value,
                shape_hint.apply(exported_value, &shape).unwrap()
            );
        }

        #[test]
        fn export_roundtrips_with_unambiguous_shape(
            value in any::<ConvexValue>(),
            shape in any::<CountedShape<SmallTestConfig>>(),
        ) {
            let shape = shape.insert_value(&value);
            let shape = StructuralShape::from(&shape);
            if !ExportContext::is_ambiguous(&shape) {
                let shape_hint = ExportContext::Infer;
                let exported_value = value.clone().export(ValueFormat::ConvexCleanJSON);
                prop_assert_eq!(
                    value.clone(),
                    shape_hint.apply(exported_value, &shape).unwrap(),
                    "{} should not be ambiguous", shape
                );
            }
        }
    }
}

/// GeneratedSchema stores sidecar data necessary to round-trip an entire table.
#[derive(Debug, Clone)]
pub struct GeneratedSchema<T: ShapeConfig> {
    pub inferred_shape: StructuralShape<T>,
    pub overrides: BTreeMap<DeveloperDocumentId, ExportContext>,
}

impl<T: ShapeConfig> GeneratedSchema<T> {
    pub fn new(inferred_shape: StructuralShape<T>) -> Self {
        Self {
            inferred_shape,
            overrides: BTreeMap::default(),
        }
    }

    pub fn insert(&mut self, object: &ConvexObject, id: DeveloperDocumentId) {
        let export_context = ExportContext::of_object(object, &self.inferred_shape);
        if !export_context.is_infer() {
            self.overrides.insert(id, export_context);
        }
    }

    pub fn apply(
        schema: &mut Option<&mut Self>,
        exported_value: JsonValue,
    ) -> anyhow::Result<ConvexValue> {
        let Some(exported_object) = exported_value.as_object() else {
            let mut buf = [0u8; 100];
            let mut writer = Cursor::new(&mut buf[..]);
            let truncated = serde_json::to_writer(&mut writer, &exported_value).is_err();
            let len = writer.position() as usize;
            anyhow::bail!(
                "expected object, received {}{}",
                String::from_utf8_lossy(&buf[..len]),
                if truncated { "..." } else { "" }
            );
        };
        let export_context = if let Some(schema) = schema
            && let Some(JsonValue::String(id_str)) = exported_object.get("_id")
        {
            let id = DeveloperDocumentId::decode(id_str)?;
            schema.overrides.remove(&id).unwrap_or(ExportContext::Infer)
        } else {
            ExportContext::Infer
        };
        let unknown = StructuralShape::new(ShapeEnum::Unknown);
        let value = export_context.apply(
            exported_value,
            if let Some(schema) = schema {
                &schema.inferred_shape
            } else {
                &unknown
            },
        )?;
        Ok(value)
    }
}

#[cfg(test)]
mod test_generated_schema {
    use std::collections::BTreeMap;

    use cmd_util::env::env_config;
    use proptest::prelude::*;
    use value::{
        assert_val,
        export::ValueFormat,
        id_v6::DeveloperDocumentId,
        ConvexObject,
    };

    use crate::{
        export_context::GeneratedSchema,
        testing::SmallTestConfig,
        CountedShape,
    };

    proptest! {
        #![proptest_config(ProptestConfig {
            failure_persistence: None, cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), ..ProptestConfig::default()
        })]
        #[test]
        fn generated_schema_object_roundtrip(
            objects in prop::collection::vec(any::<(ConvexObject, DeveloperDocumentId)>(), 1..3),
        ) {
            let mut inferred_shape = CountedShape::<SmallTestConfig>::empty();
            let mut id_to_object = BTreeMap::new();
            for (mut object, id) in objects {
                let mut fields: BTreeMap<_, _> = object.into();
                fields.insert("_id".parse().unwrap(), assert_val!(id.encode()));
                object = fields.try_into().unwrap();
                inferred_shape = inferred_shape.insert(&object);
                id_to_object.insert(id, object);
            }
            let mut generated_schema = GeneratedSchema::new((&inferred_shape).into());
            for (id, object) in id_to_object.iter() {
                generated_schema.insert(object, *id);
            }
            let generated_schema = &mut Some(&mut generated_schema);
            // Now generated_schema contains all the info.
            // See if we can extract it.
            for (_, object) in id_to_object.into_iter() {
                let exported_value = object.clone().export(ValueFormat::ConvexCleanJSON);
                let extracted = GeneratedSchema::apply(generated_schema, exported_value).unwrap();
                assert_eq!(extracted, assert_val!(object));
            }
        }
    }
}
