use std::collections::{
    BTreeMap,
    HashSet,
};

use cmd_util::env::env_config;
use common::{
    index::IndexKeyBytes,
    interval::{
        End,
        Interval,
        IntervalSet,
        StartIncluded,
    },
    types::SubscriberId,
};
use proptest::prelude::*;

use crate::IntervalMap;

#[derive(Default)]
pub struct Model {
    sets: BTreeMap<SubscriberId, IntervalSet>,
}

impl Model {
    pub fn insert(&mut self, id: SubscriberId, set: IntervalSet) {
        assert!(self.sets.insert(id, set).is_none());
    }

    pub fn remove(&mut self, id: SubscriberId) {
        self.sets.remove(&id).unwrap();
    }

    /// Returns all IDs for which the corresponding [`IntervalSet`] contains
    /// `point`.
    pub fn query<'a>(&'a self, point: &'a [u8]) -> impl Iterator<Item = SubscriberId> + 'a {
        self.sets.iter().filter_map(
            move |(id, set)| {
                if set.contains(point) {
                    Some(*id)
                } else {
                    None
                }
            },
        )
    }
}

// The weights are normally chosen at random, which allows the tree to be
// approximately balanced, but for testing we let proptest pick the weights
#[derive(Debug)]
struct WeightedIntervalSet(IntervalSet, Vec<u32>);
impl Arbitrary for WeightedIntervalSet {
    type Parameters = ();

    type Strategy = impl Strategy<Value = WeightedIntervalSet>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        IntervalSet::arbitrary().prop_flat_map(|set| {
            prop::collection::vec(any::<u32>(), set.len())
                .prop_map(move |weights| WeightedIntervalSet(set.clone(), weights))
        })
    }
}

#[derive(proptest_derive::Arbitrary, Debug)]
enum Action {
    Insert(WeightedIntervalSet),
    Query(IndexKeyBytes),
    Remove(usize),
}

#[derive(Default)]
struct Test {
    map: IntervalMap,
    model: Model,
}

impl Test {
    fn execute(&mut self, action: Action) {
        match action {
            Action::Insert(set) => {
                let id = (0..).find(|id| !self.model.sets.contains_key(id)).unwrap();
                assert!(self.map.subscribers.insert(id, None).is_none());
                for (interval, weight) in set.0.iter().zip(set.1) {
                    self.map.insert_interval(id, interval, weight).unwrap();
                    self.map.check_invariants();
                }
                self.model.insert(id, set.0);
            },
            Action::Query(point) => {
                let mut map_answer = HashSet::new();
                self.map.query(&point.0, |id| {
                    map_answer.insert(id);
                });
                let model_answer: HashSet<_> = self.model.query(&point.0).collect();
                assert_eq!(map_answer, model_answer);
            },
            Action::Remove(i) => {
                if self.model.sets.is_empty() {
                    return;
                }
                let i = i % self.model.sets.keys().len();
                let id = *self.model.sets.keys().nth(i).unwrap();
                self.map.remove(id);
                self.model.remove(id);
                self.map.check_invariants();
            },
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 32 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1),
        failure_persistence: None,
        .. ProptestConfig::default()
    })]
    #[test]
    fn proptest(
        actions in prop::collection::vec(any::<Action>(), 1..32),
    ) {
        let mut test = Test::default();
        for action in actions {
            test.execute(action);
        }
    }
}

#[test]
fn test_basic_query() {
    let mut map = IntervalMap::new();
    map.insert(0, [Interval::all()]).unwrap();
    let mut ans = vec![];
    map.query(b"hi", |id| ans.push(id));
    assert_eq!(ans, vec![0]);
}

#[test]
fn test_insert_rebalance() {
    let mut map = IntervalMap::new();
    map.subscribers.insert(0, None);
    map.insert_interval(
        0,
        Interval {
            start: StartIncluded(vec![1].into()),
            end: End::Unbounded,
        },
        2,
    )
    .unwrap();
    map.insert_interval(
        0,
        Interval {
            start: StartIncluded(vec![0].into()),
            end: End::Unbounded,
        },
        1,
    )
    .unwrap();
    map.insert_interval(
        0,
        Interval {
            start: StartIncluded(vec![2].into()),
            end: End::Unbounded,
        },
        0,
    )
    .unwrap();
    map.check_invariants();
}

#[test]
fn test_multiple_intervals_for_same_subscriber() {
    let mut map = IntervalMap::new();
    let subscriber_id: SubscriberId = 1;

    // Insert two non-overlapping intervals: [0, 10) and [20, 30) using public API
    map.insert(
        subscriber_id,
        [
            Interval {
                start: StartIncluded(vec![0].into()),
                end: End::Excluded(vec![10].into()),
            },
            Interval {
                start: StartIncluded(vec![20].into()),
                end: End::Excluded(vec![30].into()),
            },
        ],
    )
    .unwrap();
    map.check_invariants();

    // Query a value between the intervals, e.g., [15]
    let mut result = vec![];
    map.query(&[15], |id| result.push(id));
    assert!(result.is_empty());

    // Verify that queries within the intervals do find the subscriber
    let mut result_in_first = vec![];
    map.query(&[5], |id| result_in_first.push(id));
    assert_eq!(result_in_first, vec![subscriber_id]);

    let mut result_in_second = vec![];
    map.query(&[25], |id| result_in_second.push(id));
    assert_eq!(result_in_second, vec![subscriber_id]);

    // Remove the subscriber
    map.remove(subscriber_id);
    map.check_invariants();

    // Verify that no query results in a subscriber being found
    let mut result = vec![];
    map.query(&[15], |id| result.push(id));
    assert!(result.is_empty());

    let mut result = vec![];
    map.query(&[5], |id| result.push(id));
    assert!(result.is_empty());

    let mut result = vec![];
    map.query(&[25], |id| result.push(id));
    assert!(result.is_empty());
}
