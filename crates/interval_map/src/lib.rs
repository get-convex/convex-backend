#![feature(impl_trait_in_assoc_type)]
#![allow(clippy::manual_flatten)]
#![allow(clippy::collapsible_else_if)]

use std::{
    collections::HashMap,
    num::NonZeroU32,
    ops::{
        Index,
        IndexMut,
    },
};

use common::{
    interval::{
        End,
        Interval,
        StartIncluded,
    },
    types::SubscriberId,
};
use fastrand::Rng;
use slab::Slab;

#[cfg(test)]
mod tests;

/// A data structure storing a set of (possibly overlapping) [Interval]s, that
/// can efficiently query which intervals overlap a given point.
///
/// This is implemented as a treap ordered by `interval.start`, and with an
/// annotation on each subtree recording the maximum `interval.end` in that
/// subtree.
pub struct IntervalMap {
    nodes: Slab<Node>,
    root: Option<NodeKey>,
    subscribers: HashMap<SubscriberId, Option<NodeKey>>,
    rng: Rng,
}

// TODO: the node layout could be optimized
struct Node {
    // These fields are "immutable"
    weight: u32,        // treap property: a node's weight is minimal within its subtree
    key: StartIncluded, // BST key, also the lower bound of the interval
    upper_bound: End,
    subscriber: SubscriberId,

    // These form the binary tree structure
    parent: Option<NodeKey>,
    child: [Option<NodeKey>; 2],

    // Points to the `Node` with the greatest `upper_bound` in the subtree
    // rooted at this node
    max_upper_bound: NodeKey,

    // This forms a linked list of nodes with the same `subscriber`
    next: Option<NodeKey>,
}
impl Node {
    /// Panics if `from` is not a child of `self`
    fn replace_child(&mut self, from: NodeKey, to: Option<NodeKey>) {
        if self.child[0] == Some(from) {
            self.child[0] = to;
        } else {
            assert_eq!(self.child[1], Some(from));
            self.child[1] = to;
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct NodeKey(NonZeroU32);

impl NodeKey {
    fn new(index: usize) -> Result<Self, TooFull> {
        if let Some(inner) = u32::try_from(index + 1).ok().and_then(NonZeroU32::new) {
            Ok(Self(inner))
        } else {
            Err(TooFull)
        }
    }

    fn key(&self) -> usize {
        self.0.get() as usize - 1
    }
}

impl Index<NodeKey> for Slab<Node> {
    type Output = Node;

    fn index(&self, index: NodeKey) -> &Self::Output {
        &self[index.key()]
    }
}

impl IndexMut<NodeKey> for Slab<Node> {
    fn index_mut(&mut self, index: NodeKey) -> &mut Self::Output {
        &mut self[index.0.get() as usize - 1]
    }
}

#[derive(Debug)]
pub struct TooFull;

impl IntervalMap {
    #[inline]
    pub fn new() -> Self {
        Self {
            nodes: Slab::new(),
            root: None,
            subscribers: HashMap::new(),
            rng: Rng::new(),
        }
    }

    /// Returns the number of subscribers (_not_ intervals) registered in the
    /// map
    #[inline]
    pub fn subscriber_len(&self) -> usize {
        self.subscribers.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.subscribers.is_empty()
    }

    /// Adds the given intervals under the provided `subscriber`. Panics if
    /// `subscriber` is already registered.
    pub fn insert(
        &mut self,
        subscriber: SubscriberId,
        intervals: impl IntoIterator<Item = Interval>,
    ) -> Result<(), TooFull> {
        assert!(
            self.subscribers.insert(subscriber, None).is_none(),
            "double-inserted subscriber {subscriber}"
        );
        for interval in intervals {
            let weight = self.rng.u32(..);
            if let Err(TooFull) = self.insert_interval(subscriber, interval, weight) {
                self.remove(subscriber);
                return Err(TooFull);
            }
        }
        Ok(())
    }

    fn insert_interval(
        &mut self,
        subscriber: SubscriberId,
        interval: Interval,
        weight: u32,
    ) -> Result<(), TooFull> {
        let entry = self.nodes.vacant_entry();
        let key = NodeKey::new(entry.key())?;
        let sub = self
            .subscribers
            .get_mut(&subscriber)
            .expect("unknown subscriber");
        entry.insert(Node {
            weight,
            // awkward, but we insert the key later so that it can be borrowed in `insert_node`
            key: StartIncluded(Vec::new().into()),
            upper_bound: interval.end,
            parent: None,
            child: [None; 2],
            max_upper_bound: key,
            next: sub.take(),
            subscriber,
        });
        *sub = Some(key);
        match self.root {
            None => self.root = Some(key),
            Some(root) => self.root = Some(self.insert_node(root, &interval.start, key)),
        }
        self.nodes[key].key = interval.start;
        Ok(())
    }

    fn insert_node(&mut self, parent: NodeKey, key: &StartIncluded, node: NodeKey) -> NodeKey {
        if self.nodes[node].weight < self.nodes[parent].weight {
            // to maintain heap ordering, `node` replaces `parent` in the tree
            let (l, r) = self.split(parent, key);
            self.nodes[node].child = [l, r];
            if let Some(l) = l {
                self.nodes[l].parent = Some(node);
            }
            if let Some(r) = r {
                self.nodes[r].parent = Some(node);
            }
            self.recalculate_annotation(node);
            return node;
        }
        let child = if *key < self.nodes[parent].key { 0 } else { 1 };
        let inserted = if let Some(child_node) = self.nodes[parent].child[child] {
            self.insert_node(child_node, key, node)
        } else {
            node
        };
        self.nodes[parent].child[child] = Some(inserted);
        self.nodes[inserted].parent = Some(parent);
        self.recalculate_annotation(parent);
        parent
    }

    /// Splits the subtree at `node` into two: one where all keys are `<= key`
    /// and one `> key`
    fn split(&mut self, node: NodeKey, key: &StartIncluded) -> (Option<NodeKey>, Option<NodeKey>) {
        if self.nodes[node].key <= *key {
            if let Some(child) = self.nodes[node].child[1] {
                let (r0, r1) = self.split(child, key);
                self.nodes[node].child[1] = r0;
                if let Some(r0) = r0 {
                    self.nodes[r0].parent = Some(node);
                }
                self.recalculate_annotation(node);
                (Some(node), r1)
            } else {
                (Some(node), None)
            }
        } else {
            if let Some(child) = self.nodes[node].child[0] {
                let (l0, l1) = self.split(child, key);
                self.nodes[node].child[0] = l1;
                if let Some(l1) = l1 {
                    self.nodes[l1].parent = Some(node);
                }
                self.recalculate_annotation(node);
                (l0, Some(node))
            } else {
                (None, Some(node))
            }
        }
    }

    /// Removes all intervals belonging to the given `subscriber` and frees that
    /// key. Panics if `subscriber` was not previously inserted.
    pub fn remove(&mut self, subscriber: SubscriberId) {
        let mut node = self
            .subscribers
            .remove(&subscriber)
            .expect("removed unknown subscriber");
        while let Some(n) = node {
            node = self.nodes[n].next.take();
            self.remove_in_place(n);
        }
    }

    /// Removes `n` from the binary tree structure and frees it from the slab
    fn remove_in_place(&mut self, n: NodeKey) {
        let new_child = self.merge(self.nodes[n].child[0], self.nodes[n].child[1]);
        let parent = self.nodes[n].parent;
        if let Some(c) = new_child {
            self.nodes[c].parent = parent;
        }
        if let Some(p) = parent {
            self.nodes[p].replace_child(n, new_child);
            self.recalculate_annotations_to_root(p);
        } else {
            self.root = new_child;
        }
        self.nodes.remove(n.key());
    }

    /// Merges the two subtrees into a single tree. This is order-preserving, so
    /// keys under `n` must be less than or equal to keys under `m`.
    fn merge(&mut self, n: Option<NodeKey>, m: Option<NodeKey>) -> Option<NodeKey> {
        let Some(n) = n else {
            return m;
        };
        let Some(m) = m else {
            return Some(n);
        };
        debug_assert!(self.nodes[n].key <= self.nodes[m].key);
        if self.nodes[n].weight <= self.nodes[m].weight {
            // n becomes the root
            let right = self.merge(self.nodes[n].child[1], Some(m));
            self.nodes[n].child[1] = right;
            if let Some(r) = right {
                self.nodes[r].parent = Some(n);
            }
            self.recalculate_annotation(n);
            Some(n)
        } else {
            // m becomes the root
            let left = self.merge(Some(n), self.nodes[m].child[0]);
            self.nodes[m].child[0] = left;
            if let Some(l) = left {
                self.nodes[l].parent = Some(m);
            }
            self.recalculate_annotation(m);
            Some(m)
        }
    }

    /// Recalculates `self.nodes[node].max_upper_bound`
    fn recalculate_annotation(&mut self, node: NodeKey) {
        let mut ix = node;
        for child in self.nodes[node].child {
            if let Some(c) = child {
                let m = self.nodes[c].max_upper_bound;
                if self.nodes[m].upper_bound > self.nodes[ix].upper_bound {
                    ix = m;
                }
            }
        }
        self.nodes[node].max_upper_bound = ix;
    }

    fn recalculate_annotations_to_root(&mut self, node: NodeKey) {
        let mut node = Some(node);
        while let Some(n) = node {
            self.recalculate_annotation(n);
            node = self.nodes[n].parent;
        }
    }

    /// Calls `cb` for each interval in the map that overlaps `point`.
    ///
    /// Time complexity is on average `O((k + 1) log n)` where `k` is the number
    /// of returned intervals and `n` is the total number of intervals stored.
    pub fn query(&self, point: &[u8], mut cb: impl FnMut(SubscriberId)) {
        self.query_subtree(point, self.root, &mut cb);
    }

    fn query_subtree(
        &self,
        point: &[u8],
        node: Option<NodeKey>,
        cb: &mut impl FnMut(SubscriberId),
    ) {
        let Some(node) = node else {
            return;
        };
        if self.nodes[self.nodes[node].max_upper_bound]
            .upper_bound
            .greater_than(point)
        {
            if self.nodes[node].key.as_ref() <= point {
                self.query_subtree(point, self.nodes[node].child[0], cb);
                if self.nodes[node].upper_bound.greater_than(point) {
                    cb(self.nodes[node].subscriber);
                }
                self.query_subtree(point, self.nodes[node].child[1], cb);
            } else {
                self.query_subtree(point, self.nodes[node].child[0], cb);
            }
        }
    }

    #[cfg(test)]
    fn check_invariants(&self) {
        let intervals = if let Some(root) = self.root {
            assert_eq!(self.nodes[root].parent, None);
            self.check_invariants_at(root, ..).1
        } else {
            0
        };
        assert_eq!(intervals, self.nodes.len());
    }

    /// Checks:
    /// - the subtree is in nondescending `key` order
    /// - that all keys lie in `range`
    /// - parent pointers are correct
    /// - that `max_upper_bound` annotations are correct
    /// - that the subscriber linked-list makes sense
    ///
    /// Returns the number of nodes under the subtree.
    #[cfg(test)]
    fn check_invariants_at(
        &self,
        n: NodeKey,
        key_range: impl std::ops::RangeBounds<StartIncluded>,
    ) -> (NodeKey, usize) {
        use std::ops::Bound;

        let mut max_ub = n;
        let mut total_size = 1;
        for (c, subrange) in [
            (
                self.nodes[n].child[0],
                (key_range.start_bound(), Bound::Included(&self.nodes[n].key)),
            ),
            (
                self.nodes[n].child[1],
                (Bound::Included(&self.nodes[n].key), key_range.end_bound()),
            ),
        ] {
            if let Some(c) = c {
                assert_eq!(self.nodes[c].parent, Some(n));
                let (next, size) = self.check_invariants_at(c, subrange);
                total_size += size;
                if self.nodes[next].upper_bound > self.nodes[max_ub].upper_bound {
                    max_ub = next;
                }
            }
        }
        assert_eq!(
            self.nodes[self.nodes[n].max_upper_bound].upper_bound,
            self.nodes[max_ub].upper_bound
        );
        assert!(
            key_range.contains(&self.nodes[n].key),
            "nodes out of order: key {:?} not in range {:?}",
            self.nodes[n].key,
            (key_range.start_bound(), key_range.end_bound())
        );
        if let Some(next) = self.nodes[n].next {
            assert_eq!(self.nodes[n].subscriber, self.nodes[next].subscriber);
        }
        (max_ub, total_size)
    }
}

impl Default for IntervalMap {
    fn default() -> Self {
        Self::new()
    }
}
