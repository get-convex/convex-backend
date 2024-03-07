//! Module for zero-copy composite keys for maps and sets.
//!
//! It's often difficult to work with maps and sets with tuple keys without
//! copying. For example,
//! ```compile_fail
//! use std::collections::BTreeMap;
//!
//! let mut x = BTreeMap::new();
//! x.insert(("hello".to_owned(), "there".to_owned()), 0);
//! x.get(&("hello", "there"));
//! ```
//! does not compile, since we need to provide a reference to a tuple of owned
//! values, not a tuple of references. An easy solution to this problem is to
//! just copy the query key.
//! ```
//! # use std::collections::BTreeMap;
//! #
//! # let mut x = BTreeMap::new();
//! # x.insert(("hello".to_owned(), "there".to_owned()), 0);
//! x.get(&("hello".to_owned(), "there".to_owned()));
//! ```
//! How can we query this data structure without these copies?
//!
//! Our solution, inspired by this [Stack Overflow
//! post](https://stackoverflow.com/questions/45786717/how-to-implement-hashmap-with-two-keys/45795699#45795699),
//! uses trait objects to provide comparator types. In most cases, you can use
//! the [`AsComparator`] trait to turn your composite borrowed keys into types
//! that can be compared against their owned versions.
//! ```
//! use std::collections::BTreeMap;
//! use common::comparators::AsComparator;
//!
//! let mut x = BTreeMap::new();
//! x.insert(("hello".to_owned(), "there".to_owned()), 0);
//! x.get(("hello", "there").as_comparator());
//! ```
//! Under the hood, the comparator type returned by
//! [`AsComparator::as_comparator`] is a trait object that implements a `key`
//! method to return a borrowed version of the original owned type. For example,
//! `TupleKey` for 2-tuples implements `key` to take `&(T, U)` to `(&T, &U)`,
//! connecting these two types. You can manipulate these trait objects directly
//! instead of using the [`AsComparator`] trait.
//! ```
//! use std::collections::BTreeMap;
//! use common::comparators::tuple::two::TupleKey;
//!
//! let mut x = BTreeMap::new();
//! x.insert(("hello".to_owned(), "there".to_owned()), 0);
//! let q = ("hello", "there");
//! x.get(&q as &dyn TupleKey<str, str>);
//! ```
//! You can even nest these connections. For example, here's a
//! [`std::collections::BTreeMap`] that has a `LowerBound<(String, usize)>` key.
//! ```
//! use std::collections::BTreeMap;
//! use std::ops::Bound;
//! use common::bounds::LowerBound;
//! use common::comparators::lower_bound::LowerBoundKey;
//! use common::comparators::tuple::two::TupleKey as TwoTupleKey;
//!
//! let mut x = BTreeMap::new();
//! x.insert(LowerBound(Bound::Included(("apple".to_owned(), 0usize))), 1);
//! let q = ("grape", &0usize);
//! let r = LowerBound(Bound::Included(&q as &dyn TwoTupleKey<str, usize>));
//! x.get(&r as &dyn LowerBoundKey<dyn TwoTupleKey<_, _>>);
//! ```
//! If you find yourself using a particular conversion with explicit object type
//! casts a lot, consider implementing the `AsComparator` trait to permit the
//! easier `.as_comparator()` pattern.

pub mod lower_bound;
pub mod tuple;

pub trait AsComparator {
    type Comparator: ?Sized;

    fn as_comparator(&self) -> &Self::Comparator;
}
