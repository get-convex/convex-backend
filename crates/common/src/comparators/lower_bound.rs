use std::{
    borrow::Borrow,
    cmp::Ordering,
    ops::Bound,
};

use crate::{
    bounds::LowerBound,
    comparators::AsComparator,
};

pub trait LowerBoundKey<T: ?Sized> {
    fn key(&self) -> LowerBound<&T>;
}

impl<Q: ?Sized, T: Borrow<Q>> LowerBoundKey<Q> for LowerBound<T> {
    fn key(&self) -> LowerBound<&Q> {
        match self.0 {
            Bound::Included(ref t) => LowerBound(Bound::Included(t.borrow())),
            Bound::Excluded(ref t) => LowerBound(Bound::Excluded(t.borrow())),
            Bound::Unbounded => LowerBound(Bound::Unbounded),
        }
    }
}

impl<'a, Q: ?Sized, T: Borrow<Q> + 'a> Borrow<dyn LowerBoundKey<Q> + 'a> for LowerBound<T> {
    fn borrow(&self) -> &(dyn LowerBoundKey<Q> + 'a) {
        self
    }
}

impl<'a, T: Ord + ?Sized> Ord for dyn LowerBoundKey<T> + 'a {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key().cmp(&other.key())
    }
}

impl<'a, T: Ord + ?Sized> PartialOrd for dyn LowerBoundKey<T> + 'a {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a, T: Eq + ?Sized> Eq for dyn LowerBoundKey<T> + 'a {}

impl<'a, T: PartialEq + ?Sized> PartialEq for dyn LowerBoundKey<T> + 'a {
    fn eq(&self, other: &Self) -> bool {
        self.key().eq(&other.key())
    }
}

impl<'a, T: ?Sized> AsComparator for LowerBound<&'a T> {
    type Comparator = dyn LowerBoundKey<T> + 'a;

    fn as_comparator(&self) -> &Self::Comparator {
        self
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        ops::Bound,
    };

    use super::LowerBoundKey;
    use crate::{
        bounds::LowerBound,
        comparators::{
            tuple::two::TupleKey as TwoTupleKey,
            AsComparator,
        },
    };

    #[test]
    fn test_lower_bound_int() {
        let mut s = BTreeMap::new();
        s.insert(LowerBound(Bound::Included(0)), 0);

        let q = LowerBound(Bound::Included(&1));
        assert_eq!(s.get(&q as &dyn LowerBoundKey<i32>), None);

        assert_eq!(s.get(LowerBound(Bound::Included(&1)).as_comparator()), None);
    }

    #[test]
    fn test_lower_bound_str() {
        let mut s = BTreeMap::new();
        s.insert(LowerBound(Bound::Included("hello".to_owned())), 0usize);

        let q = LowerBound(Bound::Included("there"));
        assert_eq!(s.get(&q as &dyn LowerBoundKey<str>), None);
    }

    #[test]
    fn test_lower_bound_tuple() {
        let mut s = BTreeMap::new();
        s.insert(LowerBound(Bound::Included((0, 1))), 2);
        let q = (&3, &4);
        let r: LowerBound<&dyn TwoTupleKey<_, _>> =
            LowerBound(Bound::Included(&q as &dyn TwoTupleKey<_, _>));
        assert_eq!(s.get(&r as &dyn LowerBoundKey<dyn TwoTupleKey<_, _>>), None);
    }

    #[test]
    fn test_lower_bound_tuple_str() {
        let mut s = BTreeMap::new();
        s.insert(LowerBound(Bound::Included(("impressive".to_owned(), 1))), 2);
        let q = ("most impressive", &4);
        let r: LowerBound<&dyn TwoTupleKey<str, _>> =
            LowerBound(Bound::Included(&q as &dyn TwoTupleKey<_, _>));
        assert_eq!(s.get(&r as &dyn LowerBoundKey<dyn TwoTupleKey<_, _>>), None);
    }
}
