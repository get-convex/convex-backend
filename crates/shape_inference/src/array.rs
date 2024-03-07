use value::ConvexArray;

use super::{
    config::ShapeConfig,
    union::UnionBuilder,
    Shape,
    ShapeEnum,
};
use crate::{
    CountedShape,
    CountedShapeEnum,
    ShapeCounter,
};

/// Shape of an array, parameterized by an element shape. Arrays are covariant
/// in their element shape, so `array<t> <= array<u>` if `t <= u`.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ArrayShape<C: ShapeConfig, S: ShapeCounter> {
    element: Shape<C, S>,
}

impl<C: ShapeConfig, S: ShapeCounter> ArrayShape<C, S> {
    pub fn new(element: Shape<C, S>) -> Self {
        Self { element }
    }

    pub fn element(&self) -> &Shape<C, S> {
        &self.element
    }
}

impl<C: ShapeConfig> ArrayShape<C, u64> {
    pub fn shape_of(array: &ConvexArray) -> CountedShapeEnum<C> {
        let mut builder = UnionBuilder::new();
        for value in array {
            builder = builder.push(CountedShape::shape_of(value));
        }
        ShapeEnum::Array(Self {
            element: builder.build(),
        })
    }
}
