use value::ConvexSet;

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
pub struct SetShape<C: ShapeConfig, S: ShapeCounter> {
    element: Shape<C, S>,
}

impl<C: ShapeConfig, S: ShapeCounter> SetShape<C, S> {
    pub fn new(element: Shape<C, S>) -> Self {
        Self { element }
    }

    pub fn element(&self) -> &Shape<C, S> {
        &self.element
    }
}

impl<C: ShapeConfig> SetShape<C, u64> {
    pub fn shape_of(set: &ConvexSet) -> CountedShapeEnum<C> {
        let mut builder = UnionBuilder::new();
        for value in set {
            builder = builder.push(Shape::shape_of(value));
        }
        ShapeEnum::Set(Self {
            element: builder.build(),
        })
    }
}
