use value::ConvexMap;

use super::{
    config::ShapeConfig,
    union::UnionBuilder,
    Shape,
    ShapeEnum,
};
use crate::{
    CountedShapeEnum,
    ShapeCounter,
};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MapShape<C: ShapeConfig, S: ShapeCounter> {
    key: Shape<C, S>,
    value: Shape<C, S>,
}

impl<C: ShapeConfig, S: ShapeCounter> MapShape<C, S> {
    pub fn new(key: Shape<C, S>, value: Shape<C, S>) -> Self {
        assert_eq!(key.num_values, value.num_values);
        Self { key, value }
    }

    pub fn key(&self) -> &Shape<C, S> {
        &self.key
    }

    pub fn value(&self) -> &Shape<C, S> {
        &self.value
    }
}

impl<C: ShapeConfig> MapShape<C, u64> {
    pub fn shape_of(map: &ConvexMap) -> CountedShapeEnum<C> {
        let mut key_builder = UnionBuilder::new();
        let mut value_builder = UnionBuilder::new();
        for (key, value) in map {
            key_builder = key_builder.push(Shape::shape_of(key));
            value_builder = value_builder.push(Shape::shape_of(value));
        }
        ShapeEnum::Map(Self {
            key: key_builder.build(),
            value: value_builder.build(),
        })
    }
}
