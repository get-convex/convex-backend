//! In-transaction memoization of read-only subquery (`ctx.runQuery`) results.
//!
//! When a Convex query calls another query via `ctx.runQuery`, the nested query
//! executes against the same [`Transaction`](crate::Transaction) snapshot as
//! its parent. A query tree performs no writes, so the snapshot is immutable
//! for the lifetime of the transaction: calling the same subquery (identified
//! by component, path, and args) more than once is guaranteed to read the same
//! data and produce the same result. This cache memoizes those results so
//! repeated calls within a
//! single top-level query execution are served without re-executing the
//! subquery.
//!
//! ## Why this is safe
//!
//! The reads performed by the first execution of a subquery are already
//! recorded in the parent transaction's read set (reads are *never* rolled back
//! by subtransactions — see [`Transaction::rollback_subtransaction`]). Skipping
//! a later identical call therefore loses no read dependency, so reactivity is
//! preserved automatically: the parent query is still invalidated whenever any
//! data the subquery read changes.
//!
//! ## Invariants enforced by the caller (`run_udf` in `crates/isolate`)
//!
//! * Only consulted/populated when the parent UDF is a `Query`. Mutations may
//!   write between two subquery calls, which would change the snapshot, so
//!   memoization is unsound there.
//! * System UDFs are excluded: they thread a pagination journal through nested
//!   query calls that a cache hit would skip.
//! * Only `Ok` results are stored; a subquery that returns an error re-executes
//!   on the next call.
//! * Only entries whose execution did **not** observe randomness are stored:
//!   each `ctx.runQuery` advances the parent RNG and seeds the child
//!   independently, so a randomness-observing subquery may legitimately differ
//!   between calls.
//! * Subqueries that emitted audit log lines are not stored, because a cache
//!   hit does not re-emit them.
//! * Time and identity are constant for the duration of a single execution, so
//!   `observed_time` / `observed_identity` entries are safe to memoize. The
//!   caller re-propagates those observation flags on a cache hit so the parent
//!   outcome (and the top-level query cache key) stay correct.
//!
//! ## Behavior intentionally NOT replayed on a cache hit
//!
//! A cache hit returns the stored result and replays only the observation flags
//! and the single RNG draw. It does **not** re-emit the subquery's
//! `console.log` lines, re-merge its syscall trace, or re-count its database
//! egress. So a subquery called N times with identical args logs / meters its
//! reads once, not N times. This is an accepted, developer-visible behavior
//! change (a strict reduction — the reads only physically happened once
//! anyway).
//!
//! ## Scope
//!
//! This cache lives for exactly one transaction and is dropped with it. A
//! future cross-request subquery cache — reusing results across separate
//! top-level queries and reactive recomputations — would need independent
//! per-subquery read-set tokens and would live in the application layer
//! alongside the `CacheManager` (`crates/application/src/cache`). This type
//! intentionally stays simple to keep that future work decoupled.

use common::{
    components::ComponentId,
    knobs::MAX_SUBQUERY_CACHE_ENTRIES,
};
use sync_types::{
    types::SerializedArgs,
    CanonicalizedUdfPath,
};
use value::ConvexValue;

/// Identifies a subquery call within a transaction snapshot. Two calls with the
/// same key against the same (immutable) snapshot are guaranteed to produce the
/// same result.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SubqueryCacheKey {
    pub component: ComponentId,
    pub udf_path: CanonicalizedUdfPath,
    pub args: SerializedArgs,
}

/// A memoized subquery result plus the observation flags that must be replayed
/// onto the parent execution when the result is served from cache.
#[derive(Clone, Debug)]
pub struct SubqueryCacheValue {
    pub result: ConvexValue,
    /// Whether the cached execution read `ctx.auth`. Re-applied to the parent
    /// on a hit so the top-level query cache keys on identity correctly.
    pub observed_identity: bool,
    /// Whether the cached execution read the current time. Re-applied to the
    /// parent on a hit. Time is constant within one execution, so reuse is
    /// safe.
    pub observed_time: bool,
}

/// Per-transaction memoization table for `ctx.runQuery` results. See the module
/// docs for the correctness argument.
#[derive(Default)]
pub struct SubqueryCache {
    entries: std::collections::HashMap<SubqueryCacheKey, SubqueryCacheValue>,
}

impl SubqueryCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the memoized result for `key`, if present.
    pub fn get(&self, key: &SubqueryCacheKey) -> Option<SubqueryCacheValue> {
        self.entries.get(key).cloned()
    }

    /// Memoizes `value` under `key`. Returns `true` if the entry was stored.
    ///
    /// Once the per-transaction entry cap ([`MAX_SUBQUERY_CACHE_ENTRIES`]) is
    /// reached, new distinct keys are not stored (they will simply re-execute
    /// on the next call). Existing entries continue to serve. This bounds
    /// the memory the cache can hold for a single (potentially
    /// pathological) query.
    pub fn insert(&mut self, key: SubqueryCacheKey, value: SubqueryCacheValue) -> bool {
        if self.entries.len() >= *MAX_SUBQUERY_CACHE_ENTRIES && !self.entries.contains_key(&key) {
            return false;
        }
        self.entries.insert(key, value);
        true
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use common::components::ComponentId;
    use serde_json::json;
    use sync_types::{
        types::SerializedArgs,
        CanonicalizedUdfPath,
    };
    use value::ConvexValue;

    use super::{
        SubqueryCache,
        SubqueryCacheKey,
        SubqueryCacheValue,
    };

    fn key(path: &str, x: i64) -> SubqueryCacheKey {
        SubqueryCacheKey {
            component: ComponentId::Root,
            udf_path: path.parse::<CanonicalizedUdfPath>().unwrap(),
            args: SerializedArgs::from_args(vec![json!({ "x": x })]).unwrap(),
        }
    }

    fn val(n: i64, observed_identity: bool, observed_time: bool) -> SubqueryCacheValue {
        SubqueryCacheValue {
            result: ConvexValue::Int64(n),
            observed_identity,
            observed_time,
        }
    }

    #[test]
    fn get_returns_inserted_value() {
        let mut cache = SubqueryCache::new();
        assert!(cache.get(&key("a:b", 0)).is_none());
        cache.insert(key("a:b", 0), val(7, true, false));
        let got = cache.get(&key("a:b", 0)).expect("should be cached");
        assert_eq!(got.result, ConvexValue::Int64(7));
        assert!(got.observed_identity);
        assert!(!got.observed_time);
    }

    #[test]
    fn distinct_args_and_paths_are_distinct_keys() {
        let mut cache = SubqueryCache::new();
        cache.insert(key("a:b", 1), val(1, false, false));
        cache.insert(key("a:b", 2), val(2, false, false));
        cache.insert(key("c:d", 1), val(3, false, false));
        assert_eq!(
            cache.get(&key("a:b", 1)).unwrap().result,
            ConvexValue::Int64(1)
        );
        assert_eq!(
            cache.get(&key("a:b", 2)).unwrap().result,
            ConvexValue::Int64(2)
        );
        assert_eq!(
            cache.get(&key("c:d", 1)).unwrap().result,
            ConvexValue::Int64(3)
        );
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn entry_cap_stops_storing_new_keys_but_serves_existing() {
        let cap = *common::knobs::MAX_SUBQUERY_CACHE_ENTRIES;
        let mut cache = SubqueryCache::new();
        for i in 0..cap as i64 {
            assert!(cache.insert(key("a:b", i), val(i, false, false)));
        }
        assert_eq!(cache.len(), cap);
        // A brand new key is rejected once full.
        assert!(!cache.insert(key("a:b", -1), val(-1, false, false)));
        assert!(cache.get(&key("a:b", -1)).is_none());
        // Existing keys still serve, and overwriting an existing key is allowed.
        assert!(cache.get(&key("a:b", 0)).is_some());
        assert!(cache.insert(key("a:b", 0), val(999, false, false)));
        assert_eq!(
            cache.get(&key("a:b", 0)).unwrap().result,
            ConvexValue::Int64(999)
        );
    }

    #[test]
    fn observed_flags_round_trip() {
        let mut cache = SubqueryCache::new();
        cache.insert(key("a:b", 1), val(1, false, true));
        cache.insert(key("a:b", 2), val(2, true, true));
        cache.insert(key("a:b", 3), val(3, true, false));
        let v1 = cache.get(&key("a:b", 1)).unwrap();
        assert!(!v1.observed_identity && v1.observed_time);
        let v2 = cache.get(&key("a:b", 2)).unwrap();
        assert!(v2.observed_identity && v2.observed_time);
        let v3 = cache.get(&key("a:b", 3)).unwrap();
        assert!(v3.observed_identity && !v3.observed_time);
    }

    #[test]
    fn equal_keys_match_and_hash_equal() {
        use std::{
            collections::hash_map::DefaultHasher,
            hash::{
                Hash,
                Hasher,
            },
        };
        let k1 = key("a:b", 5);
        let k2 = key("a:b", 5);
        assert_eq!(k1, k2);
        let hash = |k: &SubqueryCacheKey| {
            let mut h = DefaultHasher::new();
            k.hash(&mut h);
            h.finish()
        };
        assert_eq!(hash(&k1), hash(&k2));
        // A differing path or args yields a different key.
        assert_ne!(key("a:b", 5), key("a:c", 5));
        assert_ne!(key("a:b", 5), key("a:b", 6));
        // Re-inserting an equal key overwrites in place rather than growing.
        let mut cache = SubqueryCache::new();
        cache.insert(k1, val(1, false, false));
        assert!(cache.insert(k2, val(2, false, false)));
        assert_eq!(cache.len(), 1);
        assert_eq!(
            cache.get(&key("a:b", 5)).unwrap().result,
            ConvexValue::Int64(2)
        );
    }
}
