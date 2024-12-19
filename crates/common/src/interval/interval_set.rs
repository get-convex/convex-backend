use std::{
    collections::BTreeMap,
    iter,
    ops::Bound,
};

use itertools::Either;
use pb::common::{
    interval::End as EndProto,
    Interval as IntervalProto,
};
use value::heap_size::{
    HeapSize,
    WithHeapSize,
};

#[cfg(any(test, feature = "testing"))]
use super::BinaryKey;
use super::{
    bounds::{
        End,
        Start,
    },
    Interval,
};

/// A set of `Interval`s. Intersecting and adjacent intervals are merged.
#[derive(Clone, Debug)]
#[cfg_attr(any(test, feature = "testing"), derive(Eq))]
pub enum IntervalSet {
    /// Map from Interval.start to Interval.end. All intervals are
    /// non-intersecting, non-adjacent, and non-empty.
    Intervals(WithHeapSize<BTreeMap<Start, End>>),
    /// In-memory optimization to avoid allocating a [`BTreeMap`] to represent
    /// `{ Start::Included(BinaryKey::min()) => End::Unbounded }`
    All,
}

impl Default for IntervalSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(test, feature = "testing"))]
impl PartialEq for IntervalSet {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::All, Self::All) => true,
            (Self::All, Self::Intervals(intervals)) | (Self::Intervals(intervals), Self::All) => {
                let mut map: WithHeapSize<BTreeMap<_, _>> = WithHeapSize::default();
                map.insert(Start::Included(BinaryKey::min()), End::Unbounded);
                intervals == &map
            },
            (Self::Intervals(x), Self::Intervals(y)) => x == y,
        }
    }
}

const ALL_INTERVAL_PROTO: [IntervalProto; 1] = [IntervalProto {
    start_inclusive: vec![],
    end: Some(EndProto::AfterAll(())),
}];

impl From<IntervalSet> for Vec<IntervalProto> {
    fn from(set: IntervalSet) -> Self {
        match set {
            IntervalSet::All => ALL_INTERVAL_PROTO.to_vec(),
            IntervalSet::Intervals(intervals) => intervals
                .into_iter()
                .map(|(start, end)| {
                    let start = match start {
                        Start::Included(b) => b.into(),
                    };
                    let end = match end {
                        End::Unbounded => EndProto::AfterAll(()),
                        End::Excluded(e) => EndProto::Exclusive(e.into()),
                    };
                    IntervalProto {
                        start_inclusive: start,
                        end: Some(end),
                    }
                })
                .collect(),
        }
    }
}

impl TryFrom<Vec<IntervalProto>> for IntervalSet {
    type Error = anyhow::Error;

    fn try_from(intervals: Vec<IntervalProto>) -> anyhow::Result<Self> {
        let mut set = IntervalSet::new();
        if intervals == ALL_INTERVAL_PROTO {
            return Ok(IntervalSet::All);
        }
        for interval in intervals {
            let start = Start::Included(interval.start_inclusive.into());
            let end = match interval.end {
                None => return Err(anyhow::anyhow!("Interval missing end")),
                Some(end) => match end {
                    EndProto::AfterAll(()) => End::Unbounded,
                    EndProto::Exclusive(end) => End::Excluded(end.into()),
                },
            };
            set.add(Interval { start, end });
        }
        Ok(set)
    }
}

impl IntervalSet {
    /// Construct an empty set.
    pub fn new() -> Self {
        Self::Intervals(WithHeapSize::default())
    }

    /// True if this `IntervalSet` contains no keys.
    pub fn is_empty(&self) -> bool {
        match self {
            // self.intervals only contains non-empty intervals, so this is sufficient.
            Self::Intervals(intervals) => intervals.is_empty(),
            Self::All => false,
        }
    }

    /// How many intervals are in this set?
    pub fn len(&self) -> usize {
        match self {
            Self::Intervals(intervals) => intervals.len(),
            Self::All => 1,
        }
    }

    // Return all of the intervals in `self` that intersect with or are adjacent to
    // `interval`. This is O(log(n) + m), with `n` intervals in this IntervalSet and
    // `m` matches.
    fn intersecting_or_adjacent<'a>(
        intervals: &'a WithHeapSize<BTreeMap<Start, End>>,
        interval: &'a Interval,
    ) -> impl Iterator<Item = Interval> + 'a {
        iter::from_coroutine(
            #[coroutine]
            move || {
                // We *might* intersect with the preceeding interval.
                if let Some((other_start, other_end)) = intervals
                    .range((Bound::Unbounded, Bound::Excluded(interval.start.clone())))
                    .next_back()
                {
                    let other = Interval {
                        start: other_start.clone(),
                        end: other_end.clone(),
                    };
                    if !interval.is_disjoint(&other) || interval.is_adjacent(&other) {
                        yield other;
                    }
                }

                // We definitely intersect with any interval with a `start` inside `interval`.
                for (other_start, other_end) in intervals.range(&interval.start..) {
                    if interval.end.is_disjoint(other_start)
                        && !interval.end.is_adjacent(other_start)
                    {
                        break;
                    }
                    yield Interval {
                        start: other_start.clone(),
                        end: other_end.clone(),
                    };
                }
            },
        )
    }

    /// Add the given `Interval` to the set.
    pub fn add(&mut self, interval: Interval) {
        if interval.is_empty() {
            return;
        }
        if interval == Interval::all() {
            *self = IntervalSet::All;
        }
        match self {
            IntervalSet::All => {},
            IntervalSet::Intervals(ref mut intervals) => {
                let mut merged_start = interval.start.clone();
                let mut merged_end = interval.end.clone();
                // In order to merge adjacent and overlapping intervals, we need to find all of
                // the overlapping intervals and take the min of our new interval and
                // all of the overlapping to find the start of the merged interval
                // (merged_start) and likewise for the end. Then, we remove all
                // of the overlaps and insert the merged interval. This is linear in the
                // number of overlaps, but turns out to be amoritized constant time, because you
                // can 'charge' the eviction of a interval back to the insertion that put
                // it there.
                //
                // self.intervals            --- -----    ---       -----
                // interval                           ------------------
                // merged_start                  ^
                // merged_end                                           ^
                // -> self.intervals after   --- ------------------------
                //
                // self.intervals            ---          ---       -----
                // interval                           ------------------
                // merged start                       ^
                // merged_end                                           ^
                // -> self.intervals after   ---      -------------------
                //
                // self.intervals            ---          ---   ----   --
                // interval                           ---------------
                // merged start                       ^
                // merged_end                                       ^
                // -> self.intervals after   ---      ---------------  --
                let other_intervals: Vec<Interval> =
                    Self::intersecting_or_adjacent(intervals, &interval).collect();
                for other_interval in other_intervals {
                    if other_interval.start < merged_start {
                        merged_start = other_interval.start.clone();
                    }
                    if other_interval.end > merged_end {
                        merged_end = other_interval.end.clone();
                    }
                    intervals
                        .remove(&other_interval.start)
                        .expect("tried to remove existing interval");
                }
                intervals.insert(merged_start, merged_end);
            },
        };
    }

    fn interval_preceding(&self, k: &[u8]) -> Option<Interval> {
        match self {
            Self::All => Some(Interval::all()),
            Self::Intervals(intervals) => {
                let (start, end) = intervals
                    .range((
                        Bound::Unbounded,
                        Bound::Included(Start::Included(k.to_vec().into())),
                    ))
                    .next_back()?;
                Some(Interval {
                    start: start.clone(),
                    end: end.clone(),
                })
            },
        }
    }

    /// True if any of the intervals in the `IntervalSet` contain `k`.
    pub fn contains(&self, k: &[u8]) -> bool {
        // Since self.intervals are non-overlapping, the only interval that can contain
        // k is the first preceding k.
        let Some(interval) = self.interval_preceding(k) else {
            return false;
        };
        interval.contains(k)
    }

    pub fn contains_interval(&self, target: &Interval) -> bool {
        self.split_interval_components(target)
            .all(|(in_set, _)| in_set)
    }

    /// Return an iterator over all the intervals within the set.
    pub fn iter(&self) -> impl Iterator<Item = Interval> + '_ {
        match self {
            Self::All => Either::Left(std::iter::once(Interval::all())),
            Self::Intervals(intervals) => Either::Right(intervals.iter().map(|(a, b)| Interval {
                start: a.clone(),
                end: b.clone(),
            })),
        }
    }

    /// Computes the set-difference target - self.
    pub fn subtract_from_interval(&self, target: &Interval) -> Self {
        let mut difference: WithHeapSize<BTreeMap<_, _>> = WithHeapSize::default();
        for (in_set, interval) in self.split_interval_components(target) {
            // split_interval_components alternate between `in_set` and `!in_set`, and
            // returns intervals that are adjacent and nonempty. Therefore the intervals
            // with !in_set are not intersecting or adjacent.
            if !in_set {
                difference.insert(interval.start, interval.end);
            }
        }
        Self::Intervals(difference)
    }

    /// Splits a target interval into components by whether they are in self.
    /// Returns (in_set, interval) where in_set indicates whether interval is in
    /// self, and the union of intervals is target.
    pub fn split_interval_components<'a>(
        &'a self,
        target: &'a Interval,
    ) -> impl Iterator<Item = (bool, Interval)> + 'a {
        match self {
            Self::All => Either::Right(iter::once((true, target.clone()))),
            Self::Intervals(intervals) => {
                Either::Left(iter::from_coroutine(
                    #[coroutine]
                    || {
                        if target.is_empty() {
                            return;
                        }
                        let Start::Included(target_start) = target.start.clone();
                        let interval_before = self.interval_preceding(&target_start);
                        let mut component_start = match interval_before {
                            None => target_start,
                            Some(interval_before) => {
                                if target.end <= interval_before.end {
                                    yield (true, target.clone());
                                    return;
                                }
                                let interval_before_end = match &interval_before.end {
                                    End::Unbounded => unreachable!(),
                                    End::Excluded(interval_before_end) => {
                                        interval_before_end.clone()
                                    },
                                };
                                if interval_before_end > target_start {
                                    yield (
                                        true,
                                        Interval {
                                            start: target.start.clone(),
                                            end: interval_before.end,
                                        },
                                    );
                                    interval_before_end
                                } else {
                                    target_start
                                }
                            },
                        };
                        // `intersecting` is all intervals in `self` that intersect with `target`,
                        // excluding `interval_before`.
                        let intersecting = intervals.range((
                            Bound::Excluded(Start::Included(component_start.clone())),
                            match &target.end {
                                End::Excluded(target_end) => {
                                    Bound::Excluded(Start::Included(target_end.clone()))
                                },
                                End::Unbounded => Bound::Unbounded,
                            },
                        ));
                        for (interval_start, interval_end) in intersecting {
                            let Start::Included(interval_start_bytes) = interval_start;
                            yield (
                                false,
                                Interval {
                                    start: Start::Included(component_start),
                                    end: End::Excluded(interval_start_bytes.clone()),
                                },
                            );
                            if &target.end <= interval_end {
                                yield (
                                    true,
                                    Interval {
                                        start: interval_start.clone(),
                                        end: target.end.clone(),
                                    },
                                );
                                return;
                            }
                            yield (
                                true,
                                Interval {
                                    start: interval_start.clone(),
                                    end: interval_end.clone(),
                                },
                            );
                            component_start = match interval_end {
                                End::Unbounded => unreachable!(),
                                End::Excluded(interval_end) => interval_end.clone(),
                            };
                        }
                        yield (
                            false,
                            Interval {
                                start: Start::Included(component_start),
                                end: target.end.clone(),
                            },
                        );
                    },
                ))
            },
        }
    }
}

impl HeapSize for IntervalSet {
    fn heap_size(&self) -> usize {
        match self {
            Self::All => 0,
            Self::Intervals(intervals) => intervals.heap_size(),
        }
    }
}

#[cfg(any(test, feature = "testing"))]
mod proptest {
    use proptest::prelude::*;

    use super::IntervalSet;
    use crate::interval::Interval;

    impl Arbitrary for IntervalSet {
        type Parameters = ();

        type Strategy = impl Strategy<Value = IntervalSet>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            prop::collection::vec(any::<Interval>(), 0..4).prop_map(|intervals| {
                let mut set = IntervalSet::new();
                for interval in intervals {
                    set.add(interval);
                }
                set
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use cmd_util::env::env_config;
    use itertools::Itertools;
    use must_let::must_let;
    use proptest::prelude::*;
    use value::heap_size::WithHeapSize;

    use super::{
        super::{
            bounds::End,
            key::BinaryKey,
            test_helpers::*,
            Interval,
        },
        IntervalSet,
    };
    use crate::interval::Start;

    impl IntervalSet {
        fn intervals(&self) -> WithHeapSize<BTreeMap<Start, End>> {
            match self {
                Self::All => {
                    let mut map: WithHeapSize<BTreeMap<_, _>> = WithHeapSize::default();
                    map.insert(Start::Included(BinaryKey::min()), End::Unbounded);
                    map
                },
                Self::Intervals(intervals) => intervals.clone(),
            }
        }
    }

    #[test]
    fn test_add() {
        let mut r = IntervalSet::new();
        r.add(int_interval(5, 10));
        assert_eq!(r.intervals().len(), 1);
        assert_eq!(r.intervals().get(&int_start(5)), Some(&int_end(10)));

        // Merge with the first interval
        r.add(int_interval(3, 5));
        assert_eq!(r.intervals().len(), 1, "{:?}", r.intervals());
        assert_eq!(r.intervals().get(&int_start(3)), Some(&int_end(10)));

        // Extend interval below.
        r.add(int_interval(2, 4));
        assert_eq!(r.intervals().len(), 1);
        assert_eq!(r.intervals().get(&int_start(2)), Some(&int_end(10)));

        r.add(int_interval(0, 1));
        assert_eq!(r.intervals().len(), 2);
        assert_eq!(r.intervals().get(&int_start(0)), Some(&int_end(1)));
        assert_eq!(r.intervals().get(&int_start(2)), Some(&int_end(10)));

        // Merge intervals back together.
        r.add(int_interval(0, 12));
        assert_eq!(r.intervals().len(), 1);
        assert_eq!(r.intervals().get(&int_start(0)), Some(&int_end(12)));

        // Extend interval above.
        r.add(int_interval(10, 15));
        assert_eq!(r.intervals().len(), 1);
        assert_eq!(r.intervals().get(&int_start(0)), Some(&int_end(15)));

        // Disjoint interval above.
        r.add(int_interval(20, 25));
        assert_eq!(r.intervals().len(), 2);
        assert_eq!(r.intervals().get(&int_start(0)), Some(&int_end(15)));
        assert_eq!(r.intervals().get(&int_start(20)), Some(&int_end(25)));

        // Extend high interval to max.
        r.add(Interval {
            start: int_start(22),
            end: End::Unbounded,
        });
        assert_eq!(r.intervals().len(), 2);
        assert_eq!(r.intervals().get(&int_start(0)), Some(&int_end(15)));
        assert_eq!(r.intervals().get(&int_start(20)), Some(&End::Unbounded));

        // Empty intervals should no-op.
        r.add(Interval::empty());
        assert_eq!(r.intervals().len(), 2);
        assert_eq!(r.intervals().get(&int_start(0)), Some(&int_end(15)));
        assert_eq!(r.intervals().get(&int_start(20)), Some(&End::Unbounded));

        r.add(int_interval(25, 4));
        assert_eq!(r.intervals().len(), 2);
        assert_eq!(r.intervals().get(&int_start(0)), Some(&int_end(15)));
        assert_eq!(r.intervals().get(&int_start(20)), Some(&End::Unbounded));
    }

    #[test]
    fn test_merge_multi_overlap_above_and_below() {
        //                                   1         2
        //                         0123456789012345678901234567
        // self.intervals          --- -----    ----      -----
        // intervals                     ------------------
        // -> self.intervals after --- ------------------------
        let mut r = IntervalSet::new();

        r.add(int_interval(0, 3));
        r.add(int_interval(4, 9));
        r.add(int_interval(13, 17));
        r.add(int_interval(23, 28));
        assert_eq!(r.intervals().len(), 4);

        r.add(int_interval(6, 24));

        assert_eq!(
            r.intervals().clone().into_iter().collect::<Vec<_>>(),
            vec![(int_start(0), int_end(3)), (int_start(4), int_end(28)),]
        );
    }

    #[test]
    fn test_merge_multi_overlap_above() {
        // Partial overlap above.
        //
        //                                    1         2
        //                          0123456789012345678901234567
        // self.intervals           ---          ---       -----
        // interval                       ------------------
        // -> self.intervals after  ---   ----------------------
        let mut r = IntervalSet::new();

        r.add(int_interval(0, 3));
        r.add(int_interval(13, 16));
        r.add(int_interval(23, 28));
        assert_eq!(r.intervals().len(), 3);

        r.add(int_interval(6, 24));
        assert_eq!(
            r.intervals().clone().into_iter().collect::<Vec<_>>(),
            vec![(int_start(0), int_end(3)), (int_start(6), int_end(28)),]
        );
    }

    #[test]
    fn test_merge_multi_contained() {
        //                                    1         2
        //                          0123456789012345678901234567
        // self.intervals           ---          ---   ----   --
        // interval                       ------------------
        // -> self.intervals after  ---   ------------------  --
        let mut r = IntervalSet::new();

        r.add(int_interval(0, 3));
        r.add(int_interval(13, 16));
        r.add(int_interval(19, 23));
        r.add(int_interval(26, 28));
        assert_eq!(r.intervals().len(), 4);

        r.add(int_interval(6, 24));
        assert_eq!(
            r.intervals().clone().into_iter().collect::<Vec<_>>(),
            vec![
                (int_start(0), int_end(3)),
                (int_start(6), int_end(24)),
                (int_start(26), int_end(28)),
            ]
        );
    }

    #[test]
    fn test_contains() {
        let mut r = IntervalSet::new();
        r.add(int_interval(1, 2));
        r.add(int_interval(6, 11));
        r.add(Interval {
            start: int_start(15),
            end: End::Unbounded,
        });

        assert!(!r.contains(&[0]));
        assert!(r.contains(&[1]));
        assert!(!r.contains(&[5]));
        assert!(r.contains(&[6]));
        assert!(r.contains(&[10]));
        assert!(!r.contains(&[11]));
        assert!(r.contains(&[15]));
        assert!(r.contains(&[20]));
    }

    #[test]
    fn test_contains_interval() {
        let mut r = IntervalSet::new();
        r.add(int_interval(1, 2));
        r.add(int_interval(6, 11));
        r.add(Interval {
            start: int_start(15),
            end: End::Unbounded,
        });

        assert!(!r.contains_interval(&int_interval(0, 3)));
        assert!(r.contains_interval(&int_interval(1, 2)));
        assert!(!r.contains_interval(&int_interval(1, 3)));
        assert!(!r.contains_interval(&int_interval(3, 7)));
        assert!(r.contains_interval(&int_interval(6, 7)));
        assert!(!r.contains_interval(&int_interval(6, 13)));
        assert!(r.contains_interval(&int_interval(16, 17)));
        assert!(r.contains_interval(&Interval {
            start: int_start(16),
            end: End::Unbounded,
        }));
    }

    #[test]
    fn test_subtract_from_interval() {
        let mut s = IntervalSet::new();
        s.add(int_interval(1, 2));
        s.add(int_interval(6, 11));
        s.add(int_interval_unbounded(16));

        assert_eq!(
            s.subtract_from_interval(&int_interval(7, 10)),
            IntervalSet::new()
        );
        assert_eq!(
            s.subtract_from_interval(&int_interval(5, 10)),
            new_interval_set(vec![int_interval(5, 6)])
        );
        assert_eq!(
            s.subtract_from_interval(&int_interval(5, 11)),
            new_interval_set(vec![int_interval(5, 6)])
        );
        assert_eq!(
            s.subtract_from_interval(&int_interval(5, 12)),
            new_interval_set(vec![int_interval(5, 6), int_interval(11, 12)])
        );
        assert_eq!(
            s.subtract_from_interval(&int_interval_unbounded(0)),
            new_interval_set(vec![
                int_interval(0, 1),
                int_interval(2, 6),
                int_interval(11, 16)
            ]),
        );
        assert_eq!(
            new_interval_set(vec![int_interval(1, 2)])
                .subtract_from_interval(&int_interval_unbounded(0)),
            new_interval_set(vec![int_interval(0, 1), int_interval_unbounded(2)])
        );
        assert_eq!(
            new_interval_set(vec![int_interval_unbounded(0)])
                .subtract_from_interval(&Interval::empty()),
            IntervalSet::new()
        );
    }

    fn test_sequence(intervals: Vec<Interval>, points: Vec<BinaryKey>) {
        let mut r = IntervalSet::new();
        for interval in &intervals {
            r.add(interval.clone());
        }

        // Since r.intervals.iter() is in lower-bound order on each of the intervals, we
        // can just compare each neighboring pair of intervals (windows(2)) to
        // make sure that there is a gap between them.
        for window in r
            .intervals()
            .iter()
            .map(|(start, end)| Interval {
                start: start.clone(),
                end: end.clone(),
            })
            .collect::<Vec<_>>()
            .windows(2)
        {
            must_let!(let [r1, r2] = window);
            assert!(!r1.is_empty());
            assert!(!r2.is_empty());
            assert!(r1.start < r2.start, "intervals not kept in sorted order");
            assert!(
                r1.is_disjoint(r2),
                "{:?} and {:?} both appear but intersect",
                r1,
                r2
            );
            assert!(
                !r1.is_adjacent(r2),
                "{:?} and {:?} both appear but are adjacent",
                r1,
                r2
            );
        }

        // If any of the intervals contains a point, then certainly the IntervalSet that
        // is supposed to be the union of all of them does too.
        for point in points {
            assert!(
                intervals.iter().any(|i| i.contains(&point)) == r.contains(&point),
                "some interval contains {:?} but the IntervalSet does not",
                point,
            );
        }

        // The IntervalSet contains all of its component intervals.
        for interval in intervals.iter() {
            assert!(r.contains_interval(interval));
        }
    }

    #[test]
    fn test_empty_interval() {
        test_sequence(
            vec![Interval {
                start: Start::Included(BinaryKey::min()),
                end: End::Excluded(BinaryKey::min()),
            }],
            vec![BinaryKey::min()],
        );
    }

    #[test]
    fn test_interval_set_all_split_interval_components() {
        let mut set = IntervalSet::default();
        set.add(Interval {
            start: Start::Included(BinaryKey::min()),
            end: End::Unbounded,
        });
        let all_set = IntervalSet::All;
        let target = int_interval(1, 3);
        assert_eq!(
            set.split_interval_components(&target).collect_vec(),
            all_set.split_interval_components(&target).collect_vec()
        );
    }

    #[test]
    fn test_interval_set_add_all_makes_all() {
        let mut set1 = IntervalSet::default();
        set1.add(Interval::all());
        let mut set2 = IntervalSet::default();
        set2.add(Interval {
            start: Start::Included(BinaryKey::min()),
            end: End::Unbounded,
        });
        assert_eq!(set1, set2);
        assert_eq!(set1, IntervalSet::All);
    }

    #[test]
    fn test_interval_set_add_to_all_still_all() {
        let mut set = IntervalSet::All;
        set.add(int_interval(1, 2));
        assert_eq!(set, IntervalSet::All);
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 1024 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

        #[test]
        fn proptest_small_range_insert_contains(
            ranges in prop::collection::vec(small_interval(), 1..16),
            points in prop::collection::vec(small_key(), 1..16),
        ) {
            test_sequence(ranges, points);
        }

        #[test]
        fn proptest_contains_interval(
            ranges in prop::collection::vec(small_interval(), 1..16),
            points in prop::collection::vec(small_key(), 1..16),
            interval in small_interval(),
        ) {
            let mut r = IntervalSet::new();
            for range in ranges {
                r.add(range);
            }
            // I ⊆ R ⇒ p ∈ I ⇒ p ∈ R
            if r.contains_interval(&interval) {
                for point in &points {
                    if interval.contains(point) {
                        assert!(r.contains(point));
                    }
                }
            }
            let difference = r.subtract_from_interval(&interval);
            // p ∈ R ⇒ p ∉ I \ R
            for point in &points {
                if r.contains(point) {
                    assert!(!difference.contains(point));
                }
            }
        }

        #[test]
        fn proptest_interval_components(
            ranges in prop::collection::vec(small_interval(), 1..16),
            interval in small_interval(),
        ) {
            let mut r = IntervalSet::new();
            for range in ranges {
                r.add(range);
            }
            let components = r.split_interval_components(&interval).collect_vec();
            // Components alternate in_set between true and false.
            // And components are adjacent.
            for ((in_set1, interval1), (in_set2, interval2)) in components.iter().tuples() {
                assert!(in_set1 != in_set2);
                must_let!(let End::Excluded(interval1_end) = &interval1.end);
                must_let!(let Start::Included(interval2_start) = &interval2.start);
                assert_eq!(interval1_end, interval2_start);
            }
            let mut union_components = IntervalSet::new();
            for (in_set, interval) in components {
                assert_eq!(r.contains_interval(&interval), in_set);
                union_components.add(interval);
            }
            if interval.is_empty() {
                assert!(union_components.is_empty());
            } else {
                assert_eq!(union_components.iter().collect_vec(), vec![interval]);
            }
        }
    }
}
