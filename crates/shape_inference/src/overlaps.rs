use super::{
    config::ShapeConfig,
    ShapeEnum,
};
use crate::ShapeCounter;

impl<C: ShapeConfig, S: ShapeCounter> ShapeEnum<C, S> {
    // Define a "may overlaps" relation on shapes, where any two shapes that overlap
    // (as sets of values) must have "may overlaps" return true. Note that we're
    // allowed to be a bit imprecise here: To make our lives easy, we can return
    // true for shapes that may not have any overlapping values.
    //
    // This relation is useful for ensuring union disjointness, where we don't
    // permit any of the union's variants to potentially overlap with each other.
    // For example, we define all array shapes to overlap, no matter their element
    // shapes. This then implies that a union may only have a single array shape.
    //
    // Specifically, if we have two shapes `A` and `B` such that neither is a
    // subtype of the other but they have nonempty intersection as sets, we must
    // define them as overlapping.
    //
    // This relation is reflexive and symmetric but not transitive.
    pub fn may_overlap(&self, other: &Self) -> bool {
        if self == other {
            return true;
        }
        match (self, other) {
            // All arrays, sets, and map, and objects overlap with each other, so unions may have at
            // most one of each of these shapes.
            (ShapeEnum::Array(..), ShapeEnum::Array(..)) => true,
            (ShapeEnum::Set(..), ShapeEnum::Set(..)) => true,
            (ShapeEnum::Map(..), ShapeEnum::Map(..)) => true,
            (ShapeEnum::Record(..), ShapeEnum::Record(..)) => true,

            // Two object shapes overlap if there is some value that satisfies both shapes.
            // Two object shapes definitely do not overlap if there is a required field in one that
            // is not present in the other.
            (ShapeEnum::Object(ref object), ShapeEnum::Object(ref other_object)) => {
                // Does `object` have a required field that's not present in `other_object`.
                let left_disjoint = object.iter().any(|(field_name, field)| {
                    !field.optional && !other_object.contains_key(field_name)
                });
                // Does `other_object` have a required field that's not present in `object`.
                let right_disjoint = other_object
                    .iter()
                    .any(|(field_name, field)| !field.optional && !object.contains_key(field_name));
                !(left_disjoint || right_disjoint)
            },
            // All objects and record shapes overlap. Computing this more precisely would be tricky,
            // since object and record shapes can have non-trivial intersections: Consider `{a:
            // int64 | string}` and `record<"a" | "b", int64>`.
            (ShapeEnum::Object(..), ShapeEnum::Record(..))
            | (ShapeEnum::Record(..), ShapeEnum::Object(..)) => true,

            // This isn't that relevant for the union algorithm (since we don't allow nested
            // unions and unknown at this stage of union processing), but define all shapes to
            // overlap with union shapes and unknown to make the semantic definition of overlapping
            // work out.
            (ShapeEnum::Union(..), _) | (_, ShapeEnum::Union(..)) => true,
            (ShapeEnum::Unknown, _) | (_, ShapeEnum::Unknown) => true,

            _ => false,
        }
    }
}
