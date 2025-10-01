use cmd_util::env::env_config;
use json_trait::JsonForm;
use proptest::prelude::*;
use serde_json::Value as JsonValue;
use value::ConvexValue;

use super::Shape;
use crate::{
    supertype::supertype_candidates,
    testing::{
        arbitrary_shape::nonempty_shape_strategy,
        arbitrary_value::shape_and_values_strategy,
        SmallTestConfig,
        TestConfig,
    },
    union::UnionBuilder,
    CountedShape,
};

proptest! {
    #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]

    #[test]
    fn proptest_contains_one(value in any::<ConvexValue>()) {
        let start_shape = CountedShape::<SmallTestConfig>::empty();
        assert!(!start_shape.contains(&value));
        let inserted = start_shape.insert_value(&value);
        assert!(inserted.contains(&value));
        let removed = inserted.remove_value(&value).unwrap();
        assert!(!removed.contains(&value));
    }

    #[test]
    fn proptest_insert_remove_inverse(
        start_value in any::<ConvexValue>(),
        value in any::<ConvexValue>(),
    ) {
        let start_shape = CountedShape::<SmallTestConfig>::shape_of(&start_value);
        let inserted = start_shape.insert_value(&value);
        let removed = inserted.remove_value(&value).unwrap();
        assert!(start_shape.variant.is_subtype(&removed.variant));
    }

    #[test]
    fn proptest_shape_insert_remove_inverse(
        start in any::<CountedShape<SmallTestConfig>>(),
        value in any::<ConvexValue>(),
    ) {
        let inserted = start.insert_value(&value);
        let removed = inserted.remove_value(&value).unwrap();
        assert!(start.variant.is_subtype(&removed.variant));
    }

    #[test]
    fn proptest_insert_remove_many(
        start in prop::collection::vec(any::<ConvexValue>(), 1..4),
        (insert, remove) in prop::collection::vec(any::<ConvexValue>(), 1..4)
            .prop_flat_map(|v| (Just(v.clone()), Just(v).prop_shuffle()))
    ) {
        let start_num_values = start.len() as u64;
        let insert_num_values = insert.len() as u64;
        assert_eq!(insert.len(), remove.len());

        let mut start_shape = CountedShape::<SmallTestConfig>::empty();
        for (i, value) in start.into_iter().enumerate() {
            start_shape = start_shape.insert_value(&value);
            assert_eq!(start_shape.num_values, i as u64 + 1);
        }
        let mut current = start_shape.clone();
        for (i, value) in insert.into_iter().enumerate() {
            current = current.insert_value(&value);
            assert_eq!(current.num_values, start_num_values + i as u64 + 1);
        }
        for (i, value) in remove.into_iter().enumerate() {
            current = current.remove_value(&value).unwrap();
            assert_eq!(current.num_values, start_num_values + insert_num_values - i as u64 - 1);
        }
        assert!(start_shape.variant.is_subtype(&current.variant));
    }

    #[test]
    fn proptest_shape_insert_remove_many(
        start in any::<CountedShape<SmallTestConfig>>(),
        (insert, remove) in prop::collection::vec(any::<ConvexValue>(), 1..4)
            .prop_flat_map(|v| (Just(v.clone()), Just(v).prop_shuffle()))
    ) {
        let start_num_values = start.num_values;
        let insert_num_values = insert.len() as u64;
        assert_eq!(insert.len(), remove.len());

        let mut current = start.clone();
        for (i, value) in insert.into_iter().enumerate() {
            current = current.insert_value(&value);
            assert_eq!(current.num_values, start_num_values + i as u64 + 1);
        }
        for (i, value) in remove.into_iter().enumerate() {
            current = current.remove_value(&value).unwrap();
            assert_eq!(current.num_values, start_num_values + insert_num_values - i as u64 - 1);
        }
        assert!(start.variant.is_subtype(&current.variant));
    }

    #[test]
    fn proptest_insert_commutative(
        first in any::<ConvexValue>(),
        second in any::<ConvexValue>(),
    ) {
        let start = CountedShape::<TestConfig>::empty();
        assert_eq!(start.insert_value(&first).insert_value(&second),
        start.insert_value(&second).insert_value(&first));
    }

    #[test]
    fn proptest_insert_associative(
        first in any::<ConvexValue>(),
        second in any::<ConvexValue>(),
        third in any::<ConvexValue>(),
    ) {
        let start = CountedShape::<TestConfig>::empty();
        let left_to_right = start.insert_value(&first).insert_value(&second).insert_value(&third);
        let right_to_left = start.insert_value(&third).insert_value(&second).insert_value(&first);
        assert!(left_to_right == right_to_left, "{left_to_right} != {right_to_left}");
    }

    #[test]
    fn proptest_subtype_reflexive(t in any::<CountedShape<SmallTestConfig>>()) {
        assert!(t.variant.is_subtype(&t.variant));
    }

    #[test]
    fn proptest_subtype_and_merge_compatible(
        t in any::<CountedShape<SmallTestConfig>>(),
        u in any::<CountedShape<SmallTestConfig>>(),
    ) {
        assert_eq!(t.merge_if_subtype(&u).is_some(), t.variant.is_subtype(&u.variant));
        assert_eq!(u.merge_if_subtype(&t).is_some(), u.variant.is_subtype(&t.variant));
    }

    #[test]
    fn proptest_subtype_transitive(
        t in any::<CountedShape<SmallTestConfig>>(),
        u in any::<CountedShape<SmallTestConfig>>(),
        v in any::<CountedShape<SmallTestConfig>>(),
    ) {
        if t.variant.is_subtype(&*u.variant) && u.variant.is_subtype(&*v.variant) {
            assert!(t.variant.is_subtype(&*v.variant));
        }
        let one = t.clone();
        let two = UnionBuilder::new().push(t.clone()).push(u.clone()).build();
        let three = UnionBuilder::new().push(t).push(u).push(v).build();
        assert!(one.variant.is_subtype(&two.variant));
        assert!(two.variant.is_subtype(&three.variant));
        assert!(one.variant.is_subtype(&three.variant));
    }

    #[test]
    fn proptest_union_supertype(
        shapes in prop::collection::vec(any::<CountedShape<SmallTestConfig>>(), 1..8),
    ) {
        let mut builder = UnionBuilder::new();
        for t in &shapes {
            builder = builder.push(t.clone());
        }
        let union_shape = builder.build();
        for t in shapes {
            assert!(t.variant.is_subtype(&union_shape.variant));
        }
    }

    #[test]
    fn proptest_overlaps_reflexive(t in any::<CountedShape<SmallTestConfig>>()) {
        assert!(t.variant.may_overlap(&t.variant));
    }

    #[test]
    fn proptest_overlaps_symmetric(
        t in any::<CountedShape<SmallTestConfig>>(),
        u in any::<CountedShape<SmallTestConfig>>(),
    ) {
        assert_eq!(t.variant.may_overlap(&u.variant), u.variant.may_overlap(&t.variant));
    }

    #[test]
    fn proptest_supertype_candidates(
        ts in prop::collection::vec(nonempty_shape_strategy::<SmallTestConfig>(), 2..8),
    ) {
        for (supertype, indexes) in supertype_candidates(&ts) {
            assert!(indexes.len() >= 2);
            for i in indexes {
                assert!(ts[i].variant.is_subtype(&supertype.variant));
            }
        }
    }

    #[test]
    fn proptest_semantic_shape_of((t, vs) in shape_and_values_strategy(8)) {
        for v in vs {
            assert!(t.contains(&v));
            let value_shape = Shape::shape_of(&v);
            assert!(value_shape.variant.is_subtype(&t.variant));
        }
    }

    #[test]
    fn proptest_semantic_overlap(
        (t1, vs1) in shape_and_values_strategy(8),
        (t2, vs2) in shape_and_values_strategy(8),
    ) {
        assert!(vs1.iter().all(|v| t1.contains(v)));
        assert!(vs2.iter().all(|v| t2.contains(v)));

        let t1_subtype_t2 = t1.variant.is_subtype(&t2.variant);
        let t2_subtype_t1 = t2.variant.is_subtype(&t1.variant);

        if t1_subtype_t2 {
            assert!(vs1.iter().all(|v| t2.contains(v)));
        }
        if t2_subtype_t1 {
            assert!(vs2.iter().all(|v| t1.contains(v)));
        }
        if !t1_subtype_t2 && !t2_subtype_t1 {
            let vs1_in_t2 = vs1.iter().any(|v| t2.contains(v));
            let vs2_in_t1 = vs2.iter().any(|v| t1.contains(v));
            if vs1_in_t2 || vs2_in_t1 {
                assert!(t1.variant.may_overlap(&t2.variant));
                assert!(t2.variant.may_overlap(&t1.variant));
            }
        }
    }

    #[test]
    fn proptest_semantic_supertype_candidates(
        shapes_and_values in prop::collection::vec(shape_and_values_strategy(4), 4),
    ) {
        let shapes: Vec<_> = shapes_and_values.iter().map(|(t, _)| t.clone()).collect();
        for (supertype, indexes) in supertype_candidates(&shapes) {
            for i in indexes {
                let (_, values) = &shapes_and_values[i];
                for v in values {
                    assert!(supertype.contains(v));
                }
            }
        }
    }

    #[test]
    fn proptest_json_roundtrips(
        left in any::<CountedShape<TestConfig>>()
    ) {
        let right =
            CountedShape::<TestConfig>::json_deserialize_value(JsonValue::from(&left))
                .unwrap();
        assert_eq!(left, right);
    }
}
