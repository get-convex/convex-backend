macro_rules! tuple_impls {
    ($(
        $name: ident {
            $(($idx:tt) -> ($T:ident, $BorrowT:ident));+ $(;)?
        }
    )+) => {
        $(
            pub mod $name {
                use std::borrow::Borrow;
                use std::cmp::Ordering;
                use crate::comparators::AsComparator;

                pub trait TupleKey<$($T: ?Sized),+> {
                    fn key(&self) -> ($(&$T,)+);
                }

                impl<$($BorrowT: ?Sized),+, $($T: Borrow<$BorrowT>),+>
                    TupleKey<$($BorrowT),+> for ($($T),+) {
                    fn key(&self) -> ($(&$BorrowT,)+) {
                        ($(self.$idx.borrow()),+)
                    }
                }

                impl<'a, $($BorrowT: ?Sized),+, $($T: Borrow<$BorrowT> + 'a),+>
                    Borrow<dyn TupleKey<$($BorrowT),+> + 'a> for ($($T),+) {
                    fn borrow(&self) -> &(dyn TupleKey<$($BorrowT),+> + 'a) {
                        self
                    }
                }

                impl<'a, $($T: Ord + ?Sized),+> Ord for dyn TupleKey<$($T),+> + 'a {
                    fn cmp(&self, other: &Self) -> Ordering {
                        self.key().cmp(&other.key())
                    }
                }

                impl<'a, $($T: PartialOrd + ?Sized),+> PartialOrd for dyn TupleKey<$($T),+> + 'a {
                    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                        self.key().partial_cmp(&other.key())
                    }
                }

                impl<'a, $($T: Eq + ?Sized),+> Eq for dyn TupleKey<$($T),+> + 'a {
                }

                impl<'a, $($T: PartialEq + ?Sized),+> PartialEq for dyn TupleKey<$($T),+> + 'a {
                    fn eq(&self, other: &Self) -> bool {
                        self.key().eq(&other.key())
                    }
                }

                impl<'a, $($T: ?Sized),+> AsComparator for ($(&'a $T),+) {
                    type Comparator = dyn TupleKey<$($T),+> + 'a;

                    fn as_comparator(&self) -> &Self::Comparator {
                        self
                    }
                }
            }
        )+
    }
}

tuple_impls! {
    two {
        (0) -> (A, BorrowA);
        (1) -> (B, BorrowB);
    }
    three {
        (0) -> (A, BorrowA);
        (1) -> (B, BorrowB);
        (2) -> (C, BorrowC);
    }
    four {
        (0) -> (A, BorrowA);
        (1) -> (B, BorrowB);
        (2) -> (C, BorrowC);
        (3) -> (D, BorrowD);
    }
    five {
        (0) -> (A, BorrowA);
        (1) -> (B, BorrowB);
        (2) -> (C, BorrowC);
        (3) -> (D, BorrowD);
        (4) -> (E, BorrowE);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::two::TupleKey;
    use crate::comparators::AsComparator;

    #[test]
    fn test_two_int() {
        let mut s = BTreeMap::new();
        s.insert((1, 2), 3);
        let q = (&4, &5);
        assert_eq!(s.get(&q as &dyn TupleKey<_, _>), None);
        assert_eq!(s.get((&4, &5).as_comparator()), None);
    }

    #[test]
    fn test_two_str() {
        let mut s = BTreeMap::new();
        s.insert(("ironic".to_owned(), "he".to_owned()), 0);
        let q = ("could", "save");
        assert_eq!(s.get(&q as &dyn TupleKey<str, str>), None);
    }
}
