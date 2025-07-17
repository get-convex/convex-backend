//! RangeSet and RangeMap for storing a set of intervals and querying whether a
//! point is within an interval.

use std::collections::BTreeMap;

use common::{
    index::IndexKeyBytes,
    interval::IntervalSet,
};

/// Maps an ID to a IntervalSet and allows querying all IDs that map to a
/// interval that contains a given point.
pub struct IntervalMap<ID: Clone + Ord> {
    sets: BTreeMap<ID, IntervalSet>,
}

impl<ID: Clone + Ord> IntervalMap<ID> {
    /// Construct a new [`IntervalMap`].
    pub fn new() -> Self {
        Self {
            sets: BTreeMap::new(),
        }
    }

    /// Is the IntervalMap empty?
    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    pub fn len(&self) -> usize {
        self.sets.len()
    }

    /// Insert the IntervalSet for the given ID.
    pub fn insert(&mut self, id: ID, set: IntervalSet) -> Option<IntervalSet> {
        self.sets.insert(id, set)
    }

    /// Remove the ID->IntervalSet mapping for the given ID.
    pub fn remove(&mut self, id: ID) -> Option<IntervalSet> {
        self.sets.remove(&id)
    }

    /// Returns all IDs for which the corresponding [`IntervalSet`] contains
    /// `point`.
    pub fn query<'a>(&'a self, point: &'a IndexKeyBytes) -> impl Iterator<Item = ID> + 'a {
        self.sets.iter().filter_map(move |(id, set)| {
            if set.contains(&point.0) {
                Some(id.clone())
            } else {
                None
            }
        })
    }
}
