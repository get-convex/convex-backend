use std::collections::BTreeMap;

use proptest::prelude::*;
use value::{
    id_v6::DeveloperDocumentId,
    proptest::float64_strategy,
    ConvexArray,
    ConvexMap,
    ConvexObject,
    ConvexSet,
    ConvexValue,
    FieldName,
    InternalId,
};

use super::arbitrary_shape::nonempty_shape_strategy;
use crate::{
    testing::SmallTestConfig,
    CountedShape,
    ShapeConfig,
    ShapeEnum,
};

const BRANCHING: usize = 4;

pub fn shape_member_strategy<C: ShapeConfig>(t: &CountedShape<C>) -> BoxedStrategy<ConvexValue> {
    match &*t.variant {
        ShapeEnum::Never => panic!("Proptest doesn't permit empty strategies"),
        ShapeEnum::Null => Just(ConvexValue::Null).boxed(),
        ShapeEnum::Int64 => prop::num::i64::ANY.prop_map(ConvexValue::Int64).boxed(),
        ShapeEnum::Float64 => float64_strategy().prop_map(ConvexValue::Float64).boxed(),
        ShapeEnum::NormalFloat64 => (prop::num::f64::NORMAL | prop::num::f64::SUBNORMAL)
            .prop_map(ConvexValue::Float64)
            .boxed(),
        ShapeEnum::NegativeInf => Just(ConvexValue::Float64(f64::NEG_INFINITY)).boxed(),
        ShapeEnum::PositiveInf => Just(ConvexValue::Float64(f64::INFINITY)).boxed(),
        ShapeEnum::NegativeZero => Just(ConvexValue::Float64(-0.0)).boxed(),
        ShapeEnum::NaN => Just(ConvexValue::Float64(f64::NAN)).boxed(),
        ShapeEnum::Boolean => any::<bool>().prop_map(ConvexValue::Boolean).boxed(),
        ShapeEnum::StringLiteral(ref s) => {
            Just(ConvexValue::String(s[..].try_into().unwrap())).boxed()
        },
        ShapeEnum::Id(table) => {
            let table = *table;
            any::<InternalId>()
                .prop_map(move |id| {
                    let id = DeveloperDocumentId::new(table, id);
                    ConvexValue::String(String::from(id).try_into().unwrap())
                })
                .boxed()
        },
        ShapeEnum::FieldName => any::<FieldName>()
            .prop_map(|s| ConvexValue::String(s[..].try_into().unwrap()))
            .boxed(),
        ShapeEnum::String => any::<value::ConvexString>()
            .prop_map(ConvexValue::String)
            .boxed(),
        ShapeEnum::Bytes => any::<value::ConvexBytes>()
            .prop_map(ConvexValue::Bytes)
            .boxed(),
        ShapeEnum::Array(ref array) => {
            prop::collection::vec(shape_member_strategy(array.element()), 0..BRANCHING)
                .prop_map(|values| ConvexValue::Array(ConvexArray::try_from(values).unwrap()))
                .boxed()
        },
        ShapeEnum::Set(ref set) => {
            prop::collection::btree_set(shape_member_strategy(set.element()), 0..BRANCHING)
                .prop_map(|values| ConvexValue::Set(ConvexSet::try_from(values).unwrap()))
                .boxed()
        },
        ShapeEnum::Map(ref map) => prop::collection::btree_map(
            shape_member_strategy(map.key()),
            shape_member_strategy(map.value()),
            0..BRANCHING,
        )
        .prop_map(|values| ConvexValue::Map(ConvexMap::try_from(values).unwrap()))
        .boxed(),
        ShapeEnum::Object(ref object) => {
            let mut strategy = Just(BTreeMap::new()).boxed();
            for (field_name, field) in object.iter() {
                let k = field_name.clone();
                let value_strategy = shape_member_strategy(&field.value_shape);
                let field_value_strategy = if field.optional {
                    prop::option::of(value_strategy).boxed()
                } else {
                    value_strategy.prop_map(Some).boxed()
                };
                strategy = (strategy, field_value_strategy)
                    .prop_map(move |(mut object, value)| {
                        if let Some(value) = value {
                            object.insert(FieldName::from(k.clone()), value);
                        }
                        object
                    })
                    .boxed();
            }
            strategy
                .prop_map(|fields| ConvexValue::Object(fields.try_into().unwrap()))
                .boxed()
        },
        ShapeEnum::Record(ref record) => {
            let field_strategy = shape_member_strategy(record.field()).prop_map(|v| {
                let ConvexValue::String(ref s) = v else {
                    panic!("Generated non-string for record field?");
                };
                s.parse::<FieldName>()
                    .expect("Generated invalid field name for record")
            });
            prop::collection::btree_map(
                field_strategy,
                shape_member_strategy(record.value()),
                0..BRANCHING,
            )
            .prop_map(|value| ConvexValue::Object(ConvexObject::try_from(value).unwrap()))
            .boxed()
        },
        ShapeEnum::Union(ref union) => {
            prop::strategy::Union::new(union.iter().map(|t| shape_member_strategy(t))).boxed()
        },
        ShapeEnum::Unknown => any::<ConvexValue>().boxed(),
    }
}

pub fn shape_and_values_strategy(
    n: usize,
) -> impl Strategy<Value = (CountedShape<SmallTestConfig>, Vec<ConvexValue>)> {
    nonempty_shape_strategy::<SmallTestConfig>().prop_flat_map(move |t| {
        prop::collection::vec(shape_member_strategy(&t), 1..n).prop_map(move |vs| (t.clone(), vs))
    })
}
