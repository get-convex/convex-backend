use std::{
    collections::BTreeSet,
    iter,
};

use proptest::prelude::*;

use super::{
    bounds::{
        End,
        Start,
    },
    key::BinaryKey,
    Interval,
    IntervalSet,
};

pub fn key(s: &'static [u8]) -> BinaryKey {
    s.to_vec().into()
}

pub fn start(s: &'static [u8]) -> Start {
    Start::Included(key(s))
}

pub fn int_start(s: u8) -> Start {
    Start::Included(vec![s].into())
}

pub fn end(s: &'static [u8]) -> End {
    End::Excluded(key(s))
}

pub fn int_end(s: u8) -> End {
    End::Excluded(vec![s].into())
}

pub fn int_interval(s: u8, e: u8) -> Interval {
    Interval {
        start: int_start(s),
        end: int_end(e),
    }
}

pub fn int_interval_unbounded(s: u8) -> Interval {
    Interval {
        start: int_start(s),
        end: End::Unbounded,
    }
}

pub fn new_interval_set(intervals: Vec<Interval>) -> IntervalSet {
    let mut interval_set = IntervalSet::new();
    for interval in intervals {
        interval_set.add(interval);
    }
    interval_set
}

pub fn small_key() -> impl Strategy<Value = BinaryKey> {
    prop::collection::vec(any::<u8>(), 0..4).prop_map(|v| v.into())
}

pub fn u8_interval() -> impl Strategy<Value = (BTreeSet<BinaryKey>, Interval)> {
    any::<(Option<u8>, Option<u8>)>().prop_map(|(s, t)| {
        let mut reference = BTreeSet::new();
        if s.is_none() {
            reference.insert(vec![].into());
        }
        let start = s.unwrap_or(0) as usize;
        let end = t.map(|e| e as usize).unwrap_or(256);
        for i in start..end {
            reference.insert(vec![i as u8].into());
        }
        let start = Start::Included(s.map(|s| vec![s]).unwrap_or_else(Vec::new).into());
        let end = t
            .map(|t| End::Excluded(vec![t].into()))
            .unwrap_or(End::Unbounded);
        (reference, Interval { start, end })
    })
}

prop_compose! {
    pub fn u16_interval()(
        start in prop::collection::vec(any::<u8>(), 0..2),
        end in prop::option::of(prop::collection::vec(any::<u8>(), 0..2)),
    ) -> (BTreeSet<BinaryKey>, Interval) {
        let mut reference = BTreeSet::new();

        let length_zero = iter::once(vec![]);
        let length_one = (0..256).map(|i| vec![i as u8]);
        let length_two = (0..65536).map(|i: u32| i.to_be_bytes().to_vec());
        let keys = length_zero.chain(length_one).chain(length_two);

        for key in keys {
            let in_interval = match end {
                Some(ref end) => start <= key && &key < end,
                None => start <= key,
            };
            if in_interval {
                reference.insert(key.into());
            }
        }

        let start = Start::Included(start.into());
        let end = end.map(|e| End::Excluded(e.into())).unwrap_or(End::Unbounded);

        (reference, Interval { start, end })
    }
}

prop_compose! {
    pub fn small_interval()(
        start in small_key(),
        end in prop::option::of(small_key()),
    ) -> Interval {
        let start = Start::Included(start);
        let end = end.map(End::Excluded).unwrap_or(End::Unbounded);
        Interval { start, end }
    }
}
