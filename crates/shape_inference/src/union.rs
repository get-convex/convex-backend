use std::{
    collections::BTreeSet,
    ops::Deref,
};

use super::{
    config::ShapeConfig,
    Shape,
    ShapeEnum,
};
use crate::{
    pretty::format_shapes,
    supertype::supertype_candidates,
    CountedShape,
    ShapeCounter,
    StructuralShape,
};

/// Union of shapes.
///
/// Since we have to infer shapes without any user intent, shape inference
/// requires some restrictions on unions. For a union shape `u_1 | ... | u_n`,
/// we have the following invariants:
///
/// 1. To keep space bounded, `2 <= n <= ShapeConfig::MAX_UNION_VARIANTS`.
/// 2. There are no nested unions.
/// 3. For any distinct `u_i` and `u_j`, `u_i` and `u_j` are disjoint sets.
/// Then, when we're inserting or removing a value into the union, we can know
/// precisely which variant it belongs in.
///
/// To maintain these invariants, we alternate between [`UnionShape`] and a
/// builder shape [`UnionBuilder`] that's used for inserting new shapes into the
/// union.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct UnionShape<C: ShapeConfig, S: ShapeCounter> {
    variants: BTreeSet<Shape<C, S>>,
}

impl<C: ShapeConfig> UnionShape<C, u64> {
    /// "Open up" the union for modification via [`UnionBuilder`]. This is more
    /// efficient than rebuilding the union from scratch.
    pub fn into_builder(self) -> UnionBuilder<C> {
        UnionBuilder {
            variants: self.variants,
        }
    }
}

impl<C: ShapeConfig> UnionShape<C, ()> {
    pub fn from_parts(variants: BTreeSet<StructuralShape<C>>) -> Self {
        Self { variants }
    }
}

pub struct UnionBuilder<C: ShapeConfig> {
    // All of the union invariants hold for this set of shapes, but we'll check them again anyways
    // when transitioning back to [`UnionShape`].
    variants: BTreeSet<CountedShape<C>>,
}

impl<C: ShapeConfig> UnionBuilder<C> {
    /// Create an empty union.
    pub fn new() -> Self {
        Self {
            variants: BTreeSet::new(),
        }
    }

    /// Add a new shape to the union.
    pub fn push(mut self, new_shape: CountedShape<C>) -> Self {
        // Step 1: Check the base cases.
        // Step 1a: If the new shape is empty, there's nothing new to add.
        if new_shape.is_empty() {
            return self;
        }
        // Step 1b: If the new shape is a union, flatten it by adding each of its
        // variants.
        if let ShapeEnum::Union(new_shapes) = &*new_shape.variant {
            for new_shape in new_shapes.iter() {
                self = self.push(new_shape.clone());
            }
            return self;
        }
        // Step 1c: If we're empty, just add the new shape.
        if self.variants.is_empty() {
            self.variants.insert(new_shape);
            return self;
        }

        // Step 2: If `new_shape <= u_i` for some `u_i` already in the union, just merge
        // it into that shape and return.
        let mut no_subtypes = BTreeSet::new();
        let mut found_subtype = false;
        for existing_shape in self.variants {
            if found_subtype {
                assert!(!new_shape.variant.is_subtype(&existing_shape.variant));
                assert!(no_subtypes.insert(existing_shape));
                continue;
            }
            if let Some(merged_shape) = new_shape.merge_if_subtype(&existing_shape) {
                found_subtype = true;
                assert!(no_subtypes.insert(merged_shape));
                continue;
            }
            assert!(no_subtypes.insert(existing_shape));
        }
        if found_subtype {
            return Self {
                variants: no_subtypes,
            };
        }

        // Step 3: Ensure that all of the shapes in `no_subtypes | {new_shape}` are
        // pairwise disjoint.
        let disjoint = Self::add_new_shape(no_subtypes, new_shape);

        // Step 4: To ensure our `MAX_UNION_LENGTH` invariant, contract our shapes if
        // needed.
        let variants = Self::contract(disjoint, C::MAX_UNION_LENGTH);

        Self { variants }
    }

    // Add `new_shape` to the disjoint set `shapes`, merging shapes as needed to
    // preserve disjointness.
    //
    // Precondition: `new_shape` is not a subtype of any shape in `shapes`.
    fn add_new_shape(
        mut shapes: BTreeSet<CountedShape<C>>,
        mut new_shape: CountedShape<C>,
    ) -> BTreeSet<CountedShape<C>> {
        assert!(
            !shapes
                .iter()
                .any(|t| new_shape.variant.is_subtype(&*t.variant)),
            "{new_shape} is a subtype of a shape in {}",
            format_shapes(shapes.iter()),
        );
        loop {
            // First, do the simpler subtyping check to see if there's any `u_i <=
            // new_shape`, which will eliminate any duplicates from `shapes | { new_shape
            // }`.
            let mut no_subtypes = BTreeSet::new();
            for existing_shape in shapes {
                if let Some(merged_shape) = existing_shape.merge_if_subtype(&new_shape) {
                    new_shape = merged_shape;
                } else {
                    assert!(no_subtypes.insert(existing_shape));
                }
            }
            // Next, do the lossier overlapping check, collecting all shapes that overlap
            // with `new_shape`.
            let mut nonoverlapping = BTreeSet::new();
            let mut overlapping = BTreeSet::new();
            for existing_shape in no_subtypes {
                if existing_shape.variant.may_overlap(&new_shape.variant) {
                    assert!(overlapping.insert(existing_shape));
                } else {
                    assert!(nonoverlapping.insert(existing_shape));
                }
            }
            // If we didn't find any overlapping shapes, we're done!
            if overlapping.is_empty() {
                assert!(nonoverlapping.insert(new_shape));
                return nonoverlapping;
            }
            // Otherwise, contract all of the overlapping shapes into a single shape and
            // restart.
            assert!(overlapping.insert(new_shape));
            new_shape = Self::contract(overlapping, 1)
                .into_iter()
                .next()
                .expect("Contracted to an empty shape?");
            shapes = nonoverlapping;
            continue;
        }
    }

    fn contract(shapes: BTreeSet<CountedShape<C>>, goal: usize) -> BTreeSet<CountedShape<C>> {
        assert!(goal >= 1);
        // Repeatedly apply the loop body until we reach our goal. We're guaranteed
        // we'll always decrease the number of shapes since every supertype chain
        // contains `unknown`, which is a supertype of every shape.
        let mut shapes: Vec<_> = shapes.into_iter().collect();
        while shapes.len() > goal {
            let initial_size = shapes.len();
            for (new_shape, merged_indexes) in supertype_candidates(&shapes.clone()) {
                let contracted = Self::add_new_shape(
                    shapes
                        .iter()
                        .enumerate()
                        .filter_map(|(i, s)| {
                            if merged_indexes.contains(&i) {
                                None
                            } else {
                                Some(s.clone())
                            }
                        })
                        .collect(),
                    new_shape,
                );
                if contracted.len() < initial_size {
                    shapes = contracted.into_iter().collect();
                    break;
                }
            }
            assert!(shapes.len() < initial_size);
        }
        shapes.into_iter().collect()
    }

    /// Fully contract a union into a single shape, finding some shape `t` such
    /// that all `u_i <= t`.
    pub fn fully_contract(mut self) -> CountedShape<C> {
        self.variants = Self::contract(self.variants, 1);
        assert!(self.variants.len() <= 1);
        self.variants.pop_first().unwrap_or_else(Shape::empty)
    }

    /// Build the union shape, checking union invariants again.
    pub fn build(mut self) -> CountedShape<C> {
        if self.variants.is_empty() {
            return Shape::empty();
        }
        if self.variants.len() == 1 {
            return self.variants.pop_first().unwrap();
        }
        let shapes_vec: Vec<_> = self.variants.iter().collect();
        for (i, t) in shapes_vec.iter().enumerate() {
            for u in &shapes_vec[i + 1..] {
                assert!(
                    !t.variant.may_overlap(&u.variant),
                    "{t} and {u} overlap in {}",
                    format_shapes(self.variants.iter()),
                );
            }
            if let ShapeEnum::Union(..) = &*t.variant {
                panic!("Found nested Union in new Union shape");
            }
            if let ShapeEnum::Never = &*t.variant {
                panic!("Found Never in new Union shape");
            }
            if let ShapeEnum::Unknown = &*t.variant {
                panic!("Found Unknown in new Union shape");
            }
        }
        assert!(self.variants.len() <= C::MAX_UNION_LENGTH);
        let num_values = self.variants.iter().map(|s| s.num_values).sum();
        let union = UnionShape {
            variants: self.variants,
        };
        CountedShape::new(ShapeEnum::Union(union), num_values)
    }
}

impl<C: ShapeConfig, S: ShapeCounter> Deref for UnionShape<C, S> {
    type Target = BTreeSet<Shape<C, S>>;

    fn deref(&self) -> &Self::Target {
        &self.variants
    }
}
