use value::{
    id_v6::DocumentIdV6,
    ConvexValue,
    FieldName,
};

use crate::{
    CountedShape,
    Float64Shape,
    ShapeConfig,
    ShapeEnum,
};

impl<C: ShapeConfig> CountedShape<C> {
    /// Directly check if a value is contained in the set described by a shape
    /// without going through an intermediate `Shape::shape_of`.
    pub fn contains(&self, value: &ConvexValue) -> bool {
        match (value, &*self.variant) {
            (_, ShapeEnum::Never) => false,
            (ConvexValue::Null, ShapeEnum::Null) => true,
            (ConvexValue::Int64(..), ShapeEnum::Int64) => true,
            (ConvexValue::Float64(..), ShapeEnum::Float64) => true,
            (
                ConvexValue::Float64(f),
                ShapeEnum::NegativeInf
                | ShapeEnum::PositiveInf
                | ShapeEnum::NegativeZero
                | ShapeEnum::NaN
                | ShapeEnum::NormalFloat64,
            ) => Float64Shape::<C>::shape_of(*f) == *self.variant,
            (ConvexValue::Boolean(..), ShapeEnum::Boolean) => true,
            (ConvexValue::String(ref s), ShapeEnum::StringLiteral(ref literal)) => {
                s[..] == literal[..]
            },
            (ConvexValue::String(ref s), ShapeEnum::Id(ref table)) => {
                if let Ok(ref id) = DocumentIdV6::decode(s) {
                    id.table() == table
                } else {
                    false
                }
            },
            (ConvexValue::String(ref s), ShapeEnum::FieldName) => s.parse::<FieldName>().is_ok(),
            (ConvexValue::String(..), ShapeEnum::String) => true,
            (ConvexValue::Bytes(..), ShapeEnum::Bytes) => true,
            (ConvexValue::Array(ref array), ShapeEnum::Array(ref array_shape)) => array
                .iter()
                .all(|value| array_shape.element().contains(value)),
            (ConvexValue::Set(ref set), ShapeEnum::Set(ref set_shape)) => {
                set.iter().all(|value| set_shape.element().contains(value))
            },
            (ConvexValue::Map(ref map), ShapeEnum::Map(ref map_shape)) => {
                map.iter().all(|(key, value)| {
                    map_shape.key().contains(key) && map_shape.value().contains(value)
                })
            },
            (ConvexValue::Object(ref object), ShapeEnum::Object(ref object_shape)) => {
                for (field_name, value) in object.iter() {
                    let Some(field) = object_shape.get(&field_name[..]) else {
                        return false;
                    };
                    if !field.value_shape.contains(value) {
                        return false;
                    }
                }
                for (field_name, field) in object_shape.iter() {
                    if !field.optional && !object.contains_key(&field_name[..]) {
                        return false;
                    }
                }
                true
            },
            (ConvexValue::Object(ref object), ShapeEnum::Record(ref record_shape)) => {
                object.iter().all(|(key, value)| {
                    let key = ConvexValue::String(key.to_string().try_into().unwrap());
                    record_shape.field().contains(&key) && record_shape.value().contains(value)
                })
            },
            (value, ShapeEnum::Union(ref union)) => union.iter().any(|ty| ty.contains(value)),

            (_, ShapeEnum::Unknown) => true,

            _ => false,
        }
    }
}
