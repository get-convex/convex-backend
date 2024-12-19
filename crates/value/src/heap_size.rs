use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::{
        BTreeMap,
        BTreeSet,
        VecDeque,
    },
    fmt::{
        Debug,
        Display,
    },
    hash::{
        Hash,
        Hasher,
    },
    mem::{
        self,
        size_of,
    },
    ops::{
        Deref,
        RangeBounds,
    },
    ptr::NonNull,
};

use imbl::Vector;
#[cfg(any(test, feature = "testing"))]
use proptest::{
    prelude::Arbitrary,
    strategy::{
        BoxedStrategy,
        Strategy,
    },
};
use serde_json::Value as JsonValue;
use sync_types::{
    CanonicalizedUdfPath,
    ErrorPayload,
    FunctionName,
    LogLinesMessage,
    ServerMessage,
    SessionId,
    StateModification,
    StateVersion,
    Timestamp,
    UserIdentityAttributes,
};
use tokio::sync::oneshot;

pub trait HeapSize {
    fn heap_size(&self) -> usize;
}

/// Wraps a collection and implements HeapSize in constant time. WithHeapSize
/// provides Deref but not DerefMut. All methods that require mutable
/// reference, need to be manually implemented. Please implement as needed.
pub struct WithHeapSize<T> {
    inner: T,
    // The sum of the heap sizes of all elements in the collection.
    elements_heap_size: usize,
}

// We implement Deref but not DerefMut, since mutations can potentially affect
// the elements size.
impl<T> Deref for WithHeapSize<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Default> Default for WithHeapSize<T> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
            elements_heap_size: Default::default(),
        }
    }
}

impl<T: Clone> Clone for WithHeapSize<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            elements_heap_size: self.elements_heap_size,
        }
    }
}

impl<T: PartialEq> PartialEq for WithHeapSize<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<T: Eq> Eq for WithHeapSize<T> {}

impl<T: PartialOrd + Eq> PartialOrd for WithHeapSize<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<T: Ord> Ord for WithHeapSize<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl<T: Debug> Debug for WithHeapSize<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WithHeapSize")
            .field("inner", &self.inner)
            .field("elements_heap_size", &self.elements_heap_size)
            .finish()
    }
}

impl<T: Display> Display for WithHeapSize<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl<T: Hash> Hash for WithHeapSize<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state)
    }
}

impl<A, T> FromIterator<A> for WithHeapSize<T>
where
    T: FromIterator<A>,
    WithHeapSize<T>: From<T>,
{
    fn from_iter<I: IntoIterator<Item = A>>(iter: I) -> Self {
        WithHeapSize::from(iter.into_iter().collect::<T>())
    }
}

impl<T: IntoIterator> IntoIterator for WithHeapSize<T> {
    type IntoIter = T::IntoIter;
    type Item = T::Item;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a WithHeapSize<T>
where
    &'a T: IntoIterator,
{
    type IntoIter = <&'a T as IntoIterator>::IntoIter;
    type Item = <&'a T as IntoIterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

#[cfg(any(test, feature = "testing"))]
impl<T> Arbitrary for WithHeapSize<T>
where
    WithHeapSize<T>: From<T>,
    T: Arbitrary + 'static,
{
    type Parameters = T::Parameters;
    type Strategy = BoxedStrategy<WithHeapSize<T>>;

    fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
        T::arbitrary_with(args)
            .prop_map(|v| WithHeapSize::from(v))
            .boxed()
    }
}

#[cfg(any(test, feature = "testing"))]
pub fn of<V, T>(t: T) -> impl Strategy<Value = WithHeapSize<V>>
where
    V: Debug,
    WithHeapSize<V>: From<V>,
    T: Strategy<Value = V>,
{
    t.prop_map(|v| WithHeapSize::from(v))
}

// HeapSize for Vec<u8> can be implemented in constant time.
impl HeapSize for Vec<u8> {
    fn heap_size(&self) -> usize {
        self.capacity() * mem::size_of::<u8>()
    }
}

/// WithHeapSize for Vec implementation. Only implements subset of the mutation
/// methods. Please extend the implementation as needed.
impl<T: HeapSize> WithHeapSize<Vec<T>> {
    pub fn push(&mut self, value: T) {
        self.elements_heap_size += value.heap_size();
        self.inner.push(value);
    }

    pub fn pop(&mut self) -> Option<T> {
        let result = self.inner.pop();
        if let Some(value) = result.as_ref() {
            self.elements_heap_size -= value.heap_size();
        }
        result
    }

    pub fn drain<R>(&mut self, range: R) -> impl Iterator<Item = T> + '_
    where
        R: RangeBounds<usize>,
    {
        self.inner
            .drain(range)
            .inspect(|e| self.elements_heap_size -= e.heap_size())
    }
}

impl<T: HeapSize> Extend<T> for WithHeapSize<Vec<T>> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for value in iter.into_iter() {
            self.push(value)
        }
    }
}

impl<T: HeapSize> From<Vec<T>> for WithHeapSize<Vec<T>> {
    fn from(value: Vec<T>) -> Self {
        let elements_heap_size = value.iter().map(|e| e.heap_size()).sum();
        Self {
            inner: value,
            elements_heap_size,
        }
    }
}

impl<T: HeapSize> WithHeapSize<Vec<T>> {
    pub const fn new_vec() -> Self {
        Self {
            inner: Vec::new(),
            elements_heap_size: 0,
        }
    }
}

impl<T: HeapSize> From<WithHeapSize<Vec<T>>> for Vec<T> {
    fn from(value: WithHeapSize<Vec<T>>) -> Self {
        value.inner
    }
}

impl<T: HeapSize> HeapSize for WithHeapSize<Vec<T>> {
    fn heap_size(&self) -> usize {
        self.capacity() * mem::size_of::<T>() + self.elements_heap_size
    }
}

impl<T: HeapSize + Clone> WithHeapSize<imbl::Vector<T>> {
    pub fn push_back(&mut self, value: T) {
        self.elements_heap_size += value.heap_size();
        self.inner.push_back(value)
    }

    pub fn push_front(&mut self, value: T) {
        self.elements_heap_size += value.heap_size();
        self.inner.push_front(value)
    }

    fn remove_from_heap_size(&mut self, element: &Option<T>) {
        if let Some(value) = element {
            self.elements_heap_size -= value.heap_size();
        }
    }

    pub fn pop_front(&mut self) -> Option<T> {
        let result = self.inner.pop_front();
        self.remove_from_heap_size(&result);
        result
    }

    pub fn pop_back(&mut self) -> Option<T> {
        let result = self.inner.pop_back();
        self.remove_from_heap_size(&result);
        result
    }
}

impl<T: HeapSize + Clone> HeapSize for WithHeapSize<Vector<T>> {
    fn heap_size(&self) -> usize {
        self.elements_heap_size
    }
}

impl<T: HeapSize + Clone> From<Vector<T>> for WithHeapSize<Vector<T>> {
    fn from(value: Vector<T>) -> Self {
        let elements_heap_size = value.iter().map(|e| e.heap_size()).sum();
        Self {
            inner: value,
            elements_heap_size,
        }
    }
}

impl<T: HeapSize + Clone> From<WithHeapSize<Vector<T>>> for Vector<T> {
    fn from(value: WithHeapSize<Vector<T>>) -> Self {
        value.inner
    }
}

/// WithHeapSize for VecDeque implementation. Only implements subset of the
/// mutation methods. Please extend the implementation as needed.
impl<T: HeapSize> WithHeapSize<VecDeque<T>> {
    pub fn push_back(&mut self, value: T) {
        self.elements_heap_size += value.heap_size();
        self.inner.push_back(value)
    }

    pub fn push_front(&mut self, value: T) {
        self.elements_heap_size += value.heap_size();
        self.inner.push_front(value)
    }

    fn remove_from_heap_size(&mut self, element: &Option<T>) {
        if let Some(value) = element {
            self.elements_heap_size -= value.heap_size();
        }
    }

    pub fn pop_front(&mut self) -> Option<T> {
        let result = self.inner.pop_front();
        self.remove_from_heap_size(&result);
        result
    }

    pub fn pop_back(&mut self) -> Option<T> {
        let result = self.inner.pop_back();
        self.remove_from_heap_size(&result);
        result
    }

    pub fn swap_remove_back(&mut self, index: usize) -> Option<T> {
        let result = self.inner.swap_remove_back(index);
        self.remove_from_heap_size(&result);
        result
    }
}

impl<T: HeapSize> HeapSize for WithHeapSize<VecDeque<T>> {
    fn heap_size(&self) -> usize {
        self.inner.capacity() * mem::size_of::<T>() + self.elements_heap_size
    }
}

impl<T: HeapSize> From<VecDeque<T>> for WithHeapSize<VecDeque<T>> {
    fn from(value: VecDeque<T>) -> Self {
        let elements_heap_size = value.iter().map(|e| e.heap_size()).sum();
        Self {
            inner: value,
            elements_heap_size,
        }
    }
}

impl<T: HeapSize> From<WithHeapSize<VecDeque<T>>> for VecDeque<T> {
    fn from(value: WithHeapSize<VecDeque<T>>) -> Self {
        value.inner
    }
}

impl<T> Extend<T> for WithHeapSize<VecDeque<T>> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for value in iter.into_iter() {
            self.inner.push_back(value);
        }
    }
}

impl HeapSize for () {
    #[inline]
    fn heap_size(&self) -> usize {
        0
    }
}

impl<T: HeapSize, U: HeapSize> HeapSize for (T, U) {
    #[inline]
    fn heap_size(&self) -> usize {
        self.0.heap_size() + self.1.heap_size()
    }
}

impl<T: HeapSize, U: HeapSize, V: HeapSize> HeapSize for (T, U, V) {
    #[inline]
    fn heap_size(&self) -> usize {
        self.0.heap_size() + self.1.heap_size() + self.2.heap_size()
    }
}

impl<T: HeapSize> HeapSize for Option<T> {
    #[inline]
    fn heap_size(&self) -> usize {
        self.as_ref().map_or(0, |v| v.heap_size())
    }
}

impl<T: HeapSize, E: HeapSize> HeapSize for Result<T, E> {
    #[inline]
    fn heap_size(&self) -> usize {
        match self {
            Ok(t) => t.heap_size(),
            Err(e) => e.heap_size(),
        }
    }
}

impl HeapSize for anyhow::Error {
    fn heap_size(&self) -> usize {
        // This is incorrect but difficult to calculate efficiently.
        0
    }
}

impl<T: HeapSize> HeapSize for oneshot::Sender<T> {
    fn heap_size(&self) -> usize {
        mem::size_of::<T>()
    }
}

impl HeapSize for uuid::Uuid {
    fn heap_size(&self) -> usize {
        0
    }
}

impl HeapSize for Timestamp {
    #[inline]
    fn heap_size(&self) -> usize {
        0
    }
}

impl HeapSize for CanonicalizedUdfPath {
    fn heap_size(&self) -> usize {
        // We use the string length as an approximation since we don't have
        // the full String capacity.
        self.module().as_str().len() + self.function_name().len()
    }
}

impl HeapSize for UserIdentityAttributes {
    fn heap_size(&self) -> usize {
        self.token_identifier.0.heap_size()
            + self.issuer.heap_size()
            + self.subject.heap_size()
            + self.name.heap_size()
            + self.given_name.heap_size()
            + self.family_name.heap_size()
            + self.nickname.heap_size()
            + self.preferred_username.heap_size()
            + self.profile_url.heap_size()
            + self.picture_url.heap_size()
            + self.website_url.heap_size()
            + self.email.heap_size()
            + self.email_verified.heap_size()
            + self.gender.heap_size()
            + self.birthday.heap_size()
            + self.timezone.heap_size()
            + self.language.heap_size()
            + self.phone_number.heap_size()
            + self.phone_number_verified.heap_size()
            + self.address.heap_size()
            + self.updated_at.heap_size()
    }
}

impl HeapSize for &str {
    #[inline]
    fn heap_size(&self) -> usize {
        self.len()
    }
}

impl HeapSize for String {
    #[inline]
    fn heap_size(&self) -> usize {
        self.capacity()
    }
}

impl HeapSize for Cow<'_, str> {
    fn heap_size(&self) -> usize {
        match &self {
            Cow::Borrowed(_) => 0,
            Cow::Owned(s) => s.capacity(),
        }
    }
}

impl HeapSize for usize {
    #[inline]
    fn heap_size(&self) -> usize {
        0
    }
}

impl HeapSize for u8 {
    #[inline]
    fn heap_size(&self) -> usize {
        0
    }
}

impl HeapSize for bytes::Bytes {
    fn heap_size(&self) -> usize {
        self.len()
    }
}

impl HeapSize for u16 {
    #[inline]
    fn heap_size(&self) -> usize {
        0
    }
}

impl HeapSize for u32 {
    #[inline]
    fn heap_size(&self) -> usize {
        0
    }
}

impl HeapSize for u64 {
    #[inline]
    fn heap_size(&self) -> usize {
        0
    }
}

impl HeapSize for i64 {
    #[inline]
    fn heap_size(&self) -> usize {
        0
    }
}

impl HeapSize for f64 {
    #[inline]
    fn heap_size(&self) -> usize {
        0
    }
}

impl HeapSize for bool {
    #[inline]
    fn heap_size(&self) -> usize {
        0
    }
}

// These constants and structs are taken from `std::collections::BTreeMap`'s
// implementation.
const B: usize = 6;
const CAPACITY: usize = 2 * B - 1;

#[allow(unused)]
struct InternalNode<K, V> {
    data: LeafNode<K, V>,
    edges: [Option<NonNull<LeafNode<K, V>>>; 2 * B],
}

#[allow(unused)]
struct LeafNode<K, V> {
    parent: Option<NonNull<InternalNode<K, V>>>,
    parent_idx: u16,
    len: u16,
    keys: [K; CAPACITY],
    vals: [V; CAPACITY],
}

fn estimate_btree_heap_size<K, V>(n: usize) -> usize {
    // Return early if we don't have any values.
    if n == 0 {
        return 0;
    }

    // Each node has `CAPACITY = 11` values and up to `2B = 12` children.
    // So, a full tree of `k` levels has
    //
    //  11 * 12^0 + 11 * 12^1 + 11 * 12^2 + ... + 11 * 12^(k-1)
    //    = 11 * (1 - 12^k)/(1 - 12)
    //    = 12^k - 1
    //
    // values. Then, we can compute the number of levels for `n` as
    //
    //  n >= 12^k - 1
    //  k = floor(ln(n + 1) / ln(12)).
    //
    let two_b = (2 * B) as f64;
    let k = ((n as f64).ln_1p() / two_b.ln()).floor();
    let internal_values = two_b.powf(k) - 1.;

    assert_eq!(internal_values as usize % 11, 0);
    let internal_nodes = (internal_values as usize) / 11;

    let leaf_values = (n as f64) - internal_values;
    let leaf_nodes = (leaf_values / 11.).ceil() as usize;

    mem::size_of::<InternalNode<K, V>>() * internal_nodes
        + mem::size_of::<LeafNode<K, V>>() * leaf_nodes
}

fn estimate_index_map_heap_size<K, V>(n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    // IndexMap is a hash table with 7/8 load factor, plus a Vec<(hash, key, value)>
    let hash_table = size_of::<NonNull<K>>() * (n * 8 / 7);
    let entries = (size_of::<usize>() + size_of::<K>() + size_of::<V>()) * n;
    hash_table + entries
}

/// WithHeapSize for BTreeMap implementation. Only implements subset of the
/// mutation methods. Please extend the implementation as needed.
impl<K: HeapSize + Ord, V: HeapSize> WithHeapSize<BTreeMap<K, V>> {
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let key_heap_size = key.heap_size();
        self.elements_heap_size += value.heap_size();
        let old_value = self.inner.insert(key, value);
        match old_value.as_ref() {
            Some(value) => self.elements_heap_size -= value.heap_size(), // value changes.
            None => self.elements_heap_size += key_heap_size,            // newly added
        }
        old_value
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let old_value = self.inner.remove(key);
        match old_value.as_ref() {
            Some(value) => {
                self.elements_heap_size -= key.heap_size();
                self.elements_heap_size -= value.heap_size();
            },
            None => {},
        }
        old_value
    }

    pub fn pop_first(&mut self) -> Option<(K, V)> {
        let result = self.inner.pop_first();
        if let Some((k, v)) = &result {
            self.elements_heap_size -= k.heap_size();
            self.elements_heap_size -= v.heap_size();
        }
        result
    }

    /// Alternative to entry.or_insert_with() that requires passing a function
    /// to modify the entry. The more limiting API makes it easier for us
    /// easy to track the size after the mutation.
    pub fn mutate_entry_or_insert_with<D, F, T>(&mut self, key: K, default: D, mutation: F) -> T
    where
        D: FnOnce() -> V,
        F: FnOnce(&mut V) -> T,
    {
        let value = match self.inner.entry(key) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                self.elements_heap_size += entry.key().heap_size();
                entry.insert(default())
            },
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let value = entry.get_mut();
                self.elements_heap_size -= value.heap_size();
                entry.into_mut()
            },
        };
        let result = mutation(value);
        self.elements_heap_size += value.heap_size();
        result
    }

    /// Similar to mutate_entry_or_insert_with where the value to be inserted is
    /// derived from the Default trait.
    pub fn mutate_entry_or_default<F, T>(&mut self, key: K, mutation: F) -> T
    where
        V: Default,
        F: FnOnce(&mut V) -> T,
    {
        self.mutate_entry_or_insert_with(key, V::default, mutation)
    }

    /// Alternative to get_mut.
    pub fn mutate<F, T>(&mut self, key: &K, mutation: F) -> T
    where
        F: FnOnce(Option<&mut V>) -> T,
    {
        match self.inner.get_mut(key) {
            Some(value) => {
                self.elements_heap_size -= value.heap_size();
                let result = mutation(Some(value));
                self.elements_heap_size += value.heap_size();
                result
            },
            None => mutation(None),
        }
    }
}

impl<K: HeapSize, V: HeapSize> HeapSize for WithHeapSize<BTreeMap<K, V>> {
    fn heap_size(&self) -> usize {
        estimate_btree_heap_size::<K, V>(self.len()) + self.elements_heap_size
    }
}

impl<K: HeapSize, V: HeapSize> From<BTreeMap<K, V>> for WithHeapSize<BTreeMap<K, V>> {
    fn from(value: BTreeMap<K, V>) -> Self {
        let elements_heap_size = value
            .iter()
            .map(|(k, v)| k.heap_size() + v.heap_size())
            .sum();
        Self {
            inner: value,
            elements_heap_size,
        }
    }
}

impl<K: HeapSize, V: HeapSize> From<WithHeapSize<BTreeMap<K, V>>> for BTreeMap<K, V> {
    fn from(value: WithHeapSize<BTreeMap<K, V>>) -> Self {
        value.inner
    }
}

/// WithHeapSize for BTreeSet implementation. Only implements subset of the
/// mutation methods. Please extend the implementation as needed.
impl<T: HeapSize + Ord> WithHeapSize<BTreeSet<T>> {
    pub fn insert(&mut self, value: T) -> bool {
        let value_size = value.heap_size();
        let newly_inserted = self.inner.insert(value);
        if newly_inserted {
            self.elements_heap_size += value_size
        }
        newly_inserted
    }

    pub fn remove(&mut self, value: &T) -> bool {
        let value_size = value.heap_size();
        let was_present = self.inner.remove(value);
        if was_present {
            self.elements_heap_size -= value_size
        }
        was_present
    }
}

impl<T: HeapSize> HeapSize for WithHeapSize<BTreeSet<T>> {
    fn heap_size(&self) -> usize {
        estimate_btree_heap_size::<T, ()>(self.len()) + self.elements_heap_size
    }
}

impl<T: HeapSize> From<BTreeSet<T>> for WithHeapSize<BTreeSet<T>> {
    fn from(value: BTreeSet<T>) -> Self {
        let elements_heap_size = value.iter().map(|e| e.heap_size()).sum();
        Self {
            inner: value,
            elements_heap_size,
        }
    }
}

impl<T: HeapSize> From<WithHeapSize<BTreeSet<T>>> for BTreeSet<T> {
    fn from(value: WithHeapSize<BTreeSet<T>>) -> Self {
        value.inner
    }
}

impl HeapSize for serde_json::Map<String, JsonValue> {
    fn heap_size(&self) -> usize {
        // It's actually an IndexMap since we enable the preserve_order feature.
        let mut size = estimate_index_map_heap_size::<String, JsonValue>(self.len());
        for (k, v) in self {
            size += k.heap_size();
            size += v.heap_size();
        }
        size
    }
}

// estimate_vec_size estimates the vector heap size in O(len).
// TODO(presley): Ideally, we never have to use this and use
// WithHeapSize<Vec<T>> instead, that provides heap_size in O(1). However, we
// can't do that until we move HeapSize in sync_types. This will also allow us
// to move the impl HeapSize next to the respective struct.
fn estimate_vec_size<T: HeapSize>(vec: &Vec<T>) -> usize {
    let mut size = vec.capacity() * mem::size_of::<T>();
    size += vec.iter().map(|v| v.heap_size()).sum::<usize>();
    size
}

impl HeapSize for JsonValue {
    fn heap_size(&self) -> usize {
        match self {
            Self::Null | Self::Number(_) | Self::Bool(_) => 0,
            Self::String(s) => s.heap_size(),
            Self::Array(a) => estimate_vec_size(a),
            Self::Object(o) => o.heap_size(),
        }
    }
}

impl HeapSize for LogLinesMessage {
    fn heap_size(&self) -> usize {
        estimate_vec_size(&self.0)
    }
}

impl<V: HeapSize> HeapSize for ServerMessage<V> {
    fn heap_size(&self) -> usize {
        match self {
            ServerMessage::Transition {
                start_version,
                end_version,
                modifications,
            } => {
                start_version.heap_size()
                    + end_version.heap_size()
                    + estimate_vec_size(modifications)
            },
            ServerMessage::MutationResponse {
                request_id,
                result,
                ts,
                log_lines,
            } => {
                request_id.heap_size() + result.heap_size() + ts.heap_size() + log_lines.heap_size()
            },
            ServerMessage::ActionResponse {
                request_id,
                result,
                log_lines,
            } => request_id.heap_size() + result.heap_size() + log_lines.heap_size(),
            ServerMessage::AuthError {
                error_message,
                base_version,
            } => error_message.heap_size() + base_version.heap_size(),
            ServerMessage::FatalError { error_message } => error_message.heap_size(),
            ServerMessage::Ping => 0,
        }
    }
}

impl<V: HeapSize> HeapSize for StateModification<V> {
    fn heap_size(&self) -> usize {
        match self {
            StateModification::QueryUpdated {
                query_id: _,
                value,
                log_lines,
                journal,
            } => value.heap_size() + log_lines.heap_size() + journal.heap_size(),
            StateModification::QueryFailed {
                query_id: _,
                error_message,
                error_data,
                log_lines,
                journal,
            } => {
                error_message.heap_size()
                    + error_data.heap_size()
                    + log_lines.heap_size()
                    + journal.heap_size()
            },
            StateModification::QueryRemoved { query_id: _ } => 0,
        }
    }
}

impl HeapSize for StateVersion {
    fn heap_size(&self) -> usize {
        self.query_set.heap_size() + self.identity.heap_size() + self.ts.heap_size()
    }
}

impl<V: HeapSize> HeapSize for ErrorPayload<V> {
    fn heap_size(&self) -> usize {
        match self {
            ErrorPayload::Message(message) => message.heap_size(),
            ErrorPayload::ErrorData { message, data } => message.heap_size() + data.heap_size(),
        }
    }
}

impl HeapSize for SessionId {
    fn heap_size(&self) -> usize {
        // This wraps a UUID
        0
    }
}

impl HeapSize for FunctionName {
    fn heap_size(&self) -> usize {
        // This isn't strictly correct (we should be checking capacity) but is close
        // enough.
        self.len()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{
            BTreeMap,
            BTreeSet,
            VecDeque,
        },
        mem,
    };

    use super::{
        estimate_btree_heap_size,
        InternalNode,
        LeafNode,
        WithHeapSize,
    };

    #[test]
    fn test_btree_estimation() {
        let internal_size = mem::size_of::<InternalNode<u32, u64>>();
        let leaf_size = mem::size_of::<LeafNode<u32, u64>>();

        let test_cases = [
            // Zero levels
            (0, (0, 0)),
            // One level
            (1, (0, 1)),
            (7, (0, 1)),
            // Two levels
            (11, (1, 0)),
            (12, (1, 1)),
            (13, (1, 1)),
            (25, (1, 2)),
            (142, (1, 12)),
            // Three levels
            (143, (13, 0)),
            (144, (13, 1)),
            (300, (13, 15)),
            (600, (13, 42)),
            // Four levels
            (1727, (157, 0)),
        ];
        for (n, (internal_nodes, leaf_nodes)) in test_cases {
            assert_eq!(
                estimate_btree_heap_size::<u32, u64>(n),
                internal_size * internal_nodes + leaf_size * leaf_nodes,
                "estimate({}) = {}, not {} * {} + {} * {}",
                n,
                estimate_btree_heap_size::<u32, u64>(n),
                internal_size,
                internal_nodes,
                leaf_size,
                leaf_nodes,
            )
        }
    }

    #[test]
    fn test_vec_with_heap_size() {
        let mut vec: WithHeapSize<Vec<String>> = WithHeapSize::default();
        vec.push("John".to_owned());
        assert_eq!(vec.elements_heap_size, 4);
        vec.push("Doe".to_owned());
        assert_eq!(vec.elements_heap_size, 7);
        vec.push("Jimmy".to_owned());
        assert_eq!(vec.elements_heap_size, 12);
        assert_eq!(vec.drain(1..2).collect::<Vec<_>>(), vec!["Doe".to_owned()]);
        assert_eq!(vec.elements_heap_size, 9);
        assert_eq!(vec.pop(), Some("Jimmy".to_owned()));
        assert_eq!(vec.elements_heap_size, 4);
        assert_eq!(vec.pop(), Some("John".to_owned()));
        assert_eq!(vec.elements_heap_size, 0);
        assert_eq!(vec.pop(), None);
        assert_eq!(vec.elements_heap_size, 0);
    }

    #[test]
    fn test_vec_deque_with_heap_size() {
        let mut vec: WithHeapSize<VecDeque<String>> = WithHeapSize::default();
        vec.push_back("one".to_owned());
        assert_eq!(vec.elements_heap_size, 3);
        vec.push_back("two".to_owned());
        assert_eq!(vec.elements_heap_size, 6);
        vec.push_back("three".to_owned());
        assert_eq!(vec.elements_heap_size, 11);
        vec.push_front("zero".to_owned());
        assert_eq!(vec.elements_heap_size, 15);

        assert_eq!(vec.pop_back(), Some("three".to_owned()));
        assert_eq!(vec.elements_heap_size, 10);
        assert_eq!(vec.pop_front(), Some("zero".to_owned()));
        assert_eq!(vec.elements_heap_size, 6);
        assert_eq!(vec.pop_back(), Some("two".to_owned()));
        assert_eq!(vec.elements_heap_size, 3);
        assert_eq!(vec.pop_front(), Some("one".to_owned()));
        assert_eq!(vec.elements_heap_size, 0);
        assert_eq!(vec.pop_back(), None);
        assert_eq!(vec.elements_heap_size, 0);
    }

    #[test]
    fn test_btree_map_with_heap_size() {
        let mut map: WithHeapSize<BTreeMap<String, String>> = WithHeapSize::default();
        let old_value = map.insert("one".to_owned(), "one".to_owned());
        assert_eq!(old_value, None);
        assert_eq!(map.elements_heap_size, 6);
        let old_value = map.insert("one".to_owned(), "zero+one".to_owned());
        assert_eq!(old_value, Some("one".to_owned()));
        assert_eq!(map.elements_heap_size, 11);
        let old_value = map.insert("two".to_owned(), "two".to_owned());
        assert_eq!(old_value, None);
        assert_eq!(map.elements_heap_size, 17);
        let old_value = map.insert("three".to_owned(), "three".to_owned());
        assert_eq!(old_value, None);
        assert_eq!(map.elements_heap_size, 27);
        let removed_value = map.remove(&"four".to_owned());
        assert_eq!(removed_value, None);
        assert_eq!(map.elements_heap_size, 27);
        let removed_value = map.remove(&"two".to_owned());
        assert_eq!(removed_value, Some("two".to_owned()));
        assert_eq!(map.elements_heap_size, 21);
        let result = map.mutate_entry_or_default("three".to_owned(), |v| {
            let original_len = v.len();
            *v = "one+one+one".to_owned();
            original_len
        });
        assert_eq!(result, 5);
        assert_eq!(map.elements_heap_size, 27);
        let result = map.mutate_entry_or_default("two".to_owned(), |v| {
            let original_len = v.len();
            *v = "two".to_owned();
            original_len
        });
        assert_eq!(result, 0);
        assert_eq!(map.elements_heap_size, 33);
        let result = map.mutate(&"two".to_owned(), |v| {
            let original_len = v.as_ref().unwrap().len();
            *v.unwrap() = "one+one".to_owned();
            original_len
        });
        assert_eq!(result, 3);
        assert_eq!(map.elements_heap_size, 37);

        assert_eq!(
            map.pop_first(),
            Some(("one".to_owned(), "zero+one".to_owned()))
        );
        assert_eq!(map.elements_heap_size, 26);
        assert_eq!(
            map.pop_first(),
            Some(("three".to_owned(), "one+one+one".to_owned()))
        );
        assert_eq!(map.elements_heap_size, 10);
        assert_eq!(
            map.pop_first(),
            Some(("two".to_owned(), "one+one".to_owned()))
        );
        assert_eq!(map.elements_heap_size, 0);
        assert_eq!(map.pop_first(), None);
        assert_eq!(map.elements_heap_size, 0);
    }

    #[test]
    fn test_btree_set_with_heap_size() {
        let mut set: WithHeapSize<BTreeSet<String>> = WithHeapSize::default();
        let newly_inserted = set.insert("one".to_owned());
        assert!(newly_inserted);
        assert_eq!(set.elements_heap_size, 3);
        let newly_inserted = set.insert("four".to_owned());
        assert!(newly_inserted);
        assert_eq!(set.elements_heap_size, 7);
        let newly_inserted = set.insert("one".to_owned());
        assert!(!newly_inserted);
        assert_eq!(set.elements_heap_size, 7);
        let was_removed = set.remove(&"one".to_owned());
        assert!(was_removed);
        assert_eq!(set.elements_heap_size, 4);
        let was_removed = set.remove(&"one".to_owned());
        assert!(!was_removed);
        assert_eq!(set.elements_heap_size, 4);
    }
}
