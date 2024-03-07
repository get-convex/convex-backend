use std::{
    collections::BTreeMap,
    ops::Deref,
};

use value::{
    ConvexObject,
    IdentifierFieldName,
};

use super::{
    config::ShapeConfig,
    string::StringLiteralShape,
    union::UnionBuilder,
    Shape,
    ShapeEnum,
};
use crate::{
    CountedShape,
    CountedShapeEnum,
    ShapeCounter,
};

/// Object shape with a fixed set of fields. Two object shapes with the same
/// fields are covariant in their value shapes.
///
/// An object shape with fields `f_1: v_1, ..., f_n: v_n` is a subtype of a
/// record shape if its fields (interpreted as `Value::String`s) and values are
/// all subtypes of the record's field shape and value shape.
///
/// Object shapes are limited by configuration to have
/// [`ShapeConfig::MAX_OBJECT_FIELDS`], and all fields must pass
/// [`ShapeConfig::is_valid_object_field`].
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ObjectShape<C: ShapeConfig, S: ShapeCounter> {
    fields: BTreeMap<IdentifierFieldName, ObjectField<C, S>>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ObjectField<C: ShapeConfig, S: ShapeCounter> {
    pub value_shape: Shape<C, S>,
    pub optional: bool,
}

impl<C: ShapeConfig> ObjectShape<C, ()> {
    pub fn new(fields: BTreeMap<IdentifierFieldName, ObjectField<C, ()>>) -> Self {
        assert!(fields.len() <= C::MAX_OBJECT_FIELDS);

        Self { fields }
    }
}

impl<C: ShapeConfig> ObjectShape<C, u64> {
    pub fn new(fields: BTreeMap<IdentifierFieldName, ObjectField<C, u64>>) -> Self {
        assert!(fields.len() <= C::MAX_OBJECT_FIELDS);

        // Check that we don't have any empty shapes in fields.
        assert!(fields.values().all(|f| !f.value_shape.is_empty()));

        Self { fields }
    }

    pub fn validate_value_counts(&self, num_values: u64) {
        let fields = self.fields();
        // Check that all required fields have `num_values`.
        assert!(fields
            .values()
            .filter(|f| !f.optional)
            .all(|f| f.value_shape.num_values == num_values));
        // Check that all optional fields have `num_values` or less.
        assert!(fields
            .values()
            .filter(|f| f.optional)
            .all(|f| f.value_shape.num_values < num_values));
    }

    pub fn shape_of(object: &ConvexObject) -> CountedShapeEnum<C> {
        if object.len() <= C::MAX_OBJECT_FIELDS {
            if let Ok(fields) = object
                .iter()
                .map(|(f, v)| Ok((IdentifierFieldName::try_from(f.clone())?, v)))
                .collect::<anyhow::Result<BTreeMap<_, _>>>()
            {
                let mut field_shapes = BTreeMap::new();
                for (field_name, value) in fields {
                    let field = ObjectField {
                        value_shape: Shape::shape_of(value),
                        optional: false,
                    };
                    field_shapes.insert(field_name.clone(), field);
                }
                return ShapeEnum::Object(Self::new(field_shapes));
            }
        }
        let mut fields = UnionBuilder::new();
        let mut values = UnionBuilder::new();

        for (field, value) in object.iter() {
            let field_shape = CountedShape::new(StringLiteralShape::shape_of(&field[..]), 1);
            fields = fields.push(field_shape);

            let value_shape = Shape::shape_of(value);
            values = values.push(value_shape);
        }
        ShapeEnum::Record(RecordShape::new(fields.build(), values.build()))
    }
}

impl<C: ShapeConfig, S: ShapeCounter> ObjectShape<C, S> {
    pub fn fields(&self) -> &BTreeMap<IdentifierFieldName, ObjectField<C, S>> {
        &self.fields
    }
}

impl<C: ShapeConfig, S: ShapeCounter> Deref for ObjectShape<C, S> {
    type Target = BTreeMap<IdentifierFieldName, ObjectField<C, S>>;

    fn deref(&self) -> &Self::Target {
        &self.fields
    }
}

/// Record shape with dynamic fields, parameterized over a single field shape
/// and value shape. Even though record shapes are more flexible in their field
/// shapes than object shapes, the field shape must be a subtype of string,
/// since `Value::Object` only permits these shapes.
///
/// Records are covariant in their field and value shapes, so a record shape
/// `record<f1, v1>` is a subtype of `record<f2, v2>` if `f1 <= f2` and `v1 <=
/// v2`.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RecordShape<C: ShapeConfig, S: ShapeCounter> {
    field: Shape<C, S>,
    value: Shape<C, S>,
}

impl<C: ShapeConfig, S: ShapeCounter> RecordShape<C, S> {
    pub fn new(field: Shape<C, S>, value: Shape<C, S>) -> Self {
        assert!(
            field.variant.is_subtype(&ShapeEnum::FieldName),
            "{} not a subtype of field_name",
            field.variant
        );
        assert!(
            field.num_values == value.num_values,
            "{field}:{:?} vs. {value}:{:?}",
            field.num_values,
            value.num_values
        );
        Self { field, value }
    }

    pub fn field(&self) -> &Shape<C, S> {
        &self.field
    }

    pub fn value(&self) -> &Shape<C, S> {
        &self.value
    }
}
