use std::cmp;

use proptest::prelude::*;
use value::{
    FieldName,
    TableNumber,
};

use crate::{
    array::ArrayShape,
    map::MapShape,
    object::{
        ObjectField,
        ObjectShape,
        RecordShape,
    },
    set::SetShape,
    string::StringLiteralShape,
    union::UnionBuilder,
    CountedShape,
    Shape,
    ShapeConfig,
    ShapeEnum,
    StructuralShape,
};

impl<C: ShapeConfig> Arbitrary for StringLiteralShape<C> {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        C::string_literal_strategy()
            .prop_map(|s| match StringLiteralShape::shape_of::<()>(&s) {
                ShapeEnum::StringLiteral(s) => s,
                t => panic!("String literal strategy generated {t}"),
            })
            .boxed()
    }
}

const MAX_NUM_VALUES: u64 = (u32::MAX as u64) + 1024;

impl<C: ShapeConfig> CountedShape<C> {
    fn adjust_num_values(&self, num_values: u64) -> Self {
        assert!(num_values > 0);
        let new_variant = match &*self.variant {
            ShapeEnum::Never => panic!("Adjusting num_values in `never` shape"),
            ShapeEnum::Object(object) => {
                let fields = object
                    .iter()
                    .map(|(field_name, field)| {
                        let new_field = if field.optional {
                            if num_values == 1 {
                                // Impossible to have an optional field
                                ObjectField {
                                    value_shape: field.value_shape.adjust_num_values(num_values),
                                    optional: false,
                                }
                            } else {
                                // Shave off one value from the optional value shape for better
                                // coverage,
                                let new_num_values =
                                    cmp::min(num_values - 1, field.value_shape.num_values);
                                ObjectField {
                                    value_shape: field
                                        .value_shape
                                        .adjust_num_values(new_num_values),
                                    optional: field.optional,
                                }
                            }
                        } else {
                            ObjectField {
                                value_shape: field.value_shape.adjust_num_values(num_values),
                                optional: field.optional,
                            }
                        };
                        (field_name.clone(), new_field)
                    })
                    .collect();
                ShapeEnum::Object(ObjectShape::<C, u64>::new(fields))
            },
            ShapeEnum::Union(ref union) => {
                let mut new_counts: Vec<_> = union
                    .iter()
                    .map(|t| {
                        (t.num_values as f64 * num_values as f64 / self.num_values as f64) as u64
                    })
                    .collect();
                let remaining = num_values - new_counts.iter().sum::<u64>();
                new_counts[0] += remaining;

                let mut builder = UnionBuilder::new();
                for (union_shape, &new_count) in union.iter().zip(new_counts.iter()) {
                    if new_count == 0 {
                        continue;
                    }
                    builder = builder.push(union_shape.adjust_num_values(new_count));
                }
                let new_shape = builder.build();
                assert_eq!(new_shape.num_values, num_values);
                return new_shape;
            },
            v => v.clone(),
        };
        CountedShape::new(new_variant, num_values)
    }
}

fn field_name_subtype_strategy<C: ShapeConfig>() -> impl Strategy<Value = CountedShape<C>> {
    let branching = 4;
    let leaf = prop_oneof![
        (1..MAX_NUM_VALUES, any::<FieldName>()).prop_map(|(num_values, s)| CountedShape::new(
            StringLiteralShape::shape_of(&s[..]),
            num_values
        )),
        (1..MAX_NUM_VALUES, any::<TableNumber>())
            .prop_map(|(num_values, t)| CountedShape::new(ShapeEnum::Id(t), num_values)),
        (1..MAX_NUM_VALUES)
            .prop_map(|num_values| CountedShape::new(ShapeEnum::FieldName, num_values)),
    ];
    leaf.prop_recursive(2, 8, branching, move |inner| {
        let union_bound = cmp::min(branching as usize, C::MAX_UNION_LENGTH);
        prop::collection::vec(inner, 2..=union_bound).prop_map(|union_shapes| {
            let mut builder = UnionBuilder::new();
            for union_shape in union_shapes {
                builder = builder.push(union_shape);
            }
            builder.build()
        })
    })
}

pub fn nonempty_shape_strategy<C: ShapeConfig>() -> impl Strategy<Value = CountedShape<C>> {
    let branching = 4;
    let nonempty_leaf = prop_oneof![
        (1..MAX_NUM_VALUES).prop_map(|num_values| CountedShape::new(ShapeEnum::Null, num_values)),
        (1..MAX_NUM_VALUES).prop_map(|num_values| CountedShape::new(ShapeEnum::Int64, num_values)),
        (1..MAX_NUM_VALUES)
            .prop_map(|num_values| CountedShape::new(ShapeEnum::Float64, num_values)),
        (1..MAX_NUM_VALUES)
            .prop_map(|num_values| CountedShape::new(ShapeEnum::NormalFloat64, num_values)),
        (1..MAX_NUM_VALUES)
            .prop_map(|num_values| CountedShape::new(ShapeEnum::NegativeInf, num_values)),
        (1..MAX_NUM_VALUES)
            .prop_map(|num_values| CountedShape::new(ShapeEnum::PositiveInf, num_values)),
        (1..MAX_NUM_VALUES)
            .prop_map(|num_values| CountedShape::new(ShapeEnum::NegativeZero, num_values)),
        (1..MAX_NUM_VALUES).prop_map(|num_values| CountedShape::new(ShapeEnum::NaN, num_values)),
        (1..MAX_NUM_VALUES)
            .prop_map(|num_values| CountedShape::new(ShapeEnum::Boolean, num_values)),
        (1..MAX_NUM_VALUES, any::<StringLiteralShape<C>>())
            .prop_map(|(num_values, s)| CountedShape::new(ShapeEnum::StringLiteral(s), num_values)),
        (1..MAX_NUM_VALUES, any::<TableNumber>())
            .prop_map(|(num_values, t)| CountedShape::new(ShapeEnum::Id(t), num_values)),
        (1..MAX_NUM_VALUES)
            .prop_map(|num_values| CountedShape::new(ShapeEnum::FieldName, num_values)),
        (1..MAX_NUM_VALUES).prop_map(|num_values| CountedShape::new(ShapeEnum::String, num_values)),
        (1..MAX_NUM_VALUES).prop_map(|num_values| CountedShape::new(ShapeEnum::Bytes, num_values)),
    ];
    nonempty_leaf.prop_recursive(2, 16, branching, move |inner| {
        // When generating non-leaf shapes, we need to be sure to adjust the number of
        // values for variants like maps, objects, and unions. For example, the number
        // of values in the key and value shapes of a map shape must be equal.
        let array_strategy =
            (1..MAX_NUM_VALUES, inner.clone()).prop_map(|(num_values, element_shape)| {
                CountedShape::new(ShapeEnum::Array(ArrayShape::new(element_shape)), num_values)
            });
        let set_strategy =
            (1..MAX_NUM_VALUES, inner.clone()).prop_map(|(num_values, element_shape)| {
                CountedShape::new(ShapeEnum::Set(SetShape::new(element_shape)), num_values)
            });
        let map_strategy = (1..MAX_NUM_VALUES, inner.clone(), inner.clone()).prop_map(
            |(num_values, key_shape, value_shape)| {
                let adjusted_value_shape = value_shape.adjust_num_values(key_shape.num_values);
                CountedShape::new(
                    ShapeEnum::Map(MapShape::new(key_shape, adjusted_value_shape)),
                    num_values,
                )
            },
        );
        let fields_strategy = prop::collection::btree_map(
            C::object_field_strategy(),
            (inner.clone(), any::<bool>()),
            0..C::MAX_OBJECT_FIELDS,
        );
        let object_strategy =
            (1..MAX_NUM_VALUES, fields_strategy).prop_map(|(num_values, fields)| {
                let adjusted_fields = fields
                    .into_iter()
                    .map(|(field_name, (value_shape, optional))| {
                        let is_field_optional = optional && num_values > 1;
                        let field_num_values = if is_field_optional {
                            cmp::min(num_values - 1, value_shape.num_values)
                        } else {
                            num_values
                        };
                        let field = ObjectField {
                            value_shape: value_shape.adjust_num_values(field_num_values),
                            optional: is_field_optional,
                        };
                        (field_name, field)
                    })
                    .collect();
                CountedShape::new(
                    ShapeEnum::Object(ObjectShape::<C, u64>::new(adjusted_fields)),
                    num_values,
                )
            });
        let record_strategy = (
            1..MAX_NUM_VALUES,
            field_name_subtype_strategy(),
            inner.clone(),
        )
            .prop_map(|(num_values, field_shape, value_shape)| {
                let adjusted_value_shape = value_shape.adjust_num_values(field_shape.num_values);
                CountedShape::new(
                    ShapeEnum::Record(RecordShape::new(field_shape, adjusted_value_shape)),
                    num_values,
                )
            });
        let union_bound = cmp::min(branching as usize, C::MAX_UNION_LENGTH);
        let union_strategy =
            prop::collection::vec(inner, 2..=union_bound).prop_map(|union_shapes| {
                let mut builder = UnionBuilder::new();
                for union_shape in union_shapes {
                    builder = builder.push(union_shape);
                }
                builder.build()
            });
        prop_oneof![
            array_strategy,
            set_strategy,
            map_strategy,
            object_strategy,
            record_strategy,
            union_strategy
        ]
    })
}

impl<C: ShapeConfig> Arbitrary for CountedShape<C> {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        prop_oneof![
            1 => Just(Shape::empty()),
            14 => nonempty_shape_strategy(),
        ]
        .boxed()
    }
}

impl<C: ShapeConfig> Arbitrary for StructuralShape<C> {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        any::<CountedShape<C>>()
            .prop_map(|sized| Self::from(&sized))
            .boxed()
    }
}
