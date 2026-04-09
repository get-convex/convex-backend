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

use super::{
    bounds::{
        End,
        EndRef,
        StartIncluded,
    },
    BinaryKey,
    Interval,
    IntervalRef,
};

/// A set of `Interval`s. Intersecting and adjacent intervals are merged.
#[derive(Clone, Debug)]
pub enum IntervalSet {
    /// Map from Interval.start to Interval.end. All intervals are
    /// non-intersecting, non-adjacent, and non-empty.
    Intervals(WithHeapSize<BTreeMap<StartIncluded, End>>),
    /// In-memory optimization to avoid allocating a [`BTreeMap`] to represent
    /// `{ Start(BinaryKey::min()) => End::Unbounded }`
    All,
}

impl Default for IntervalSet {
    fn default() -> Self {
        Self::new()
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
                        StartIncluded(b) => b.into(),
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
            let start = StartIncluded(interval.start_inclusive.into());
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
        intervals: &'a WithHeapSize<BTreeMap<StartIncluded, End>>,
        interval: &'a Interval,
    ) -> impl Iterator<Item = Interval> + 'a {
        iter::from_coroutine(
            #[coroutine]
            move || {
                // We *might* intersect with the preceeding interval.
                if let Some((other_start, other_end)) = intervals
                    .range::<StartIncluded, _>(..&interval.start)
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
                for (other_start, other_end) in
                    intervals.range::<StartIncluded, _>(&interval.start..)
                {
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
            IntervalSet::Intervals(intervals) => {
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

    fn interval_preceding(&self, k: &[u8]) -> Option<IntervalRef<'_>> {
        match self {
            Self::All => Some(IntervalRef::all()),
            Self::Intervals(intervals) => {
                let (start, end) = intervals
                    .range::<[u8], _>((Bound::Unbounded, Bound::Included(k)))
                    .next_back()?;
                Some(IntervalRef {
                    start: start.as_ref(),
                    end: end.as_ref(),
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

    pub fn contains_interval(&self, target: IntervalRef<'_>) -> bool {
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
        for (in_set, interval) in self.split_interval_components(target.as_ref()) {
            // split_interval_components alternate between `in_set` and `!in_set`, and
            // returns intervals that are adjacent and nonempty. Therefore the intervals
            // with !in_set are not intersecting or adjacent.
            if !in_set {
                difference.insert(
                    StartIncluded(BinaryKey::from(interval.start.to_owned())),
                    interval.end.to_owned(),
                );
            }
        }
        Self::Intervals(difference)
    }

    /// Splits a target interval into components by whether they are in self.
    /// Returns (in_set, interval) where in_set indicates whether interval is in
    /// self, and the union of intervals is target.
    pub fn split_interval_components<'a>(
        &'a self,
        target: IntervalRef<'a>,
    ) -> impl Iterator<Item = (bool, IntervalRef<'a>)> + 'a {
        match self {
            Self::All => Either::Right(iter::once((true, target))),
            Self::Intervals(intervals) => {
                Either::Left(iter::from_coroutine(
                    #[coroutine]
                    move || {
                        if target.is_empty() {
                            return;
                        }
                        let target_start = target.start;
                        let interval_before = self.interval_preceding(target_start);
                        let mut component_start = match interval_before {
                            None => target_start,
                            Some(interval_before) => {
                                if target.end <= interval_before.end {
                                    yield (true, target);
                                    return;
                                }
                                let interval_before_end = match interval_before.end {
                                    EndRef::Unbounded => unreachable!(),
                                    EndRef::Excluded(interval_before_end) => interval_before_end,
                                };
                                if interval_before_end > target_start {
                                    yield (
                                        true,
                                        IntervalRef {
                                            start: target.start,
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
                        let intersecting = intervals.range(IntervalRef {
                            start: component_start,
                            end: target.end,
                        });
                        for (interval_start, interval_end) in intersecting {
                            yield (
                                false,
                                IntervalRef {
                                    start: component_start,
                                    end: EndRef::Excluded(interval_start.as_ref()),
                                },
                            );
                            if target.end <= interval_end.as_ref() {
                                yield (
                                    true,
                                    IntervalRef {
                                        start: interval_start.as_ref(),
                                        end: target.end,
                                    },
                                );
                                return;
                            }
                            yield (
                                true,
                                IntervalRef {
                                    start: interval_start.as_ref(),
                                    end: interval_end.as_ref(),
                                },
                            );
                            component_start = match interval_end {
                                End::Unbounded => unreachable!(),
                                End::Excluded(interval_end) => interval_end.as_ref(),
                            };
                        }
                        yield (
                            false,
                            IntervalRef {
                                start: component_start,
                                end: target.end,
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
