use std::{
    collections::BTreeMap,
    mem,
    time::SystemTime,
};

use application::{
    api::SubscriptionTrait,
    redaction::{
        RedactedJsError,
        RedactedLogLines,
    },
};
use common::{
    sha256::{
        Sha256,
        Sha256Digest,
    },
    types::SessionId,
    value::ConvexValue,
};
use errors::ErrorMetadata;
use futures::{
    future::{
        self,
        AbortHandle,
        Aborted,
        BoxFuture,
    },
    stream::FuturesUnordered,
    FutureExt,
    StreamExt,
};
use keybroker::Identity;
use sync_types::{
    IdentityVersion,
    Query,
    QueryId,
    QuerySetModification,
    QuerySetVersion,
    SerializedQueryJournal,
    StateModification,
    StateVersion,
};

use crate::metrics;

type ValueDigest = Sha256Digest;
type ErrorDigest = Sha256Digest;

pub struct SyncedQuery {
    query: Query,

    /// What is the active subscription for the given query?
    ///
    /// - Starts `None`: Query is newly inserted.
    /// - `None -> Some(subscription)`: `SyncState::complete_fetch`.
    /// - `Some(..) -> None`: `SyncState::prune_invalidated_queries`.
    subscription: Option<Box<dyn SubscriptionTrait>>,

    /// What was the hash of the last successful return value? This allows us to
    /// deduplicate transitions for queries whose results haven't actually
    /// changed.
    ///
    /// - Starts `None`: Query is newly inserted.
    /// - `None -> Some(r)`: `SyncState::complete_fetch`, the first time the
    ///   query is executed.
    /// - `Some(..) -> Some(..)`: `SyncState::complete_fetch`, after the first
    ///   time.
    result_hash: Option<Result<ValueDigest, ErrorDigest>>,

    /// Handle to the query's current invalidation future. This future completes
    /// when `self.subscription` is no longer valid and the query should be
    /// rerun.
    invalidation_future: Option<AbortHandle>,
}

/// The client issues modifications to sync state predicated on a client
/// version, and this represents the latest received version from the client.
#[derive(Clone, Copy)]
pub struct ClientVersion {
    query_set: QuerySetVersion,
    identity: IdentityVersion,
}

impl ClientVersion {
    fn initial() -> Self {
        Self {
            query_set: 0,
            identity: 0,
        }
    }
}

/// Current state for the sync protocol's worker.
///
/// Fundamentally, the state is determined by the current `StateVersion`, which
/// specifies a query set version and a timestamp.
///
/// The query set version implies a set of `Query`s, stored under the `query`
/// field of `SyncedQuery`.
///
/// The timestamp, then, implies a `Subscription` for each query, which is
/// managed by the subscription worker. The subscription worker processes the
/// commit log, updates subscriptions' timestamps forward when they continue to
/// be valid, and completes the invalidation future when there's an overlap. A
/// handle to this invalidation future is stored within `invalidation_future`.
///
/// In steady state, both `subscription` and `invalidation_future` must be
/// `Some` for each query, and `SyncState::validate` checks this invariant. This
/// invariant can be temporarily broken when a query's subscription completes,
/// its invalidation future spuriously completes, or a new query is added.
///
/// We fix up queries that don't have a subscription by first finding them with
/// `SyncState::need_fetch`, running their UDF, and then initializing the
/// subscription with `SyncState::complete_fetch`. We fix up queries that don't
/// have an invalidation future with `SyncState::fill_subscriptions`.
///
/// The general idea here is that the state "self describes" what work needs to
/// be done next.
pub struct SyncState {
    // ID for the current session. Will be None for old clients that connect
    // without specifying a session ID.
    session_id: Option<SessionId>,
    current_version: StateVersion,
    invalidation_futures:
        FuturesUnordered<BoxFuture<'static, Result<anyhow::Result<QueryId>, Aborted>>>,
    queries: BTreeMap<QueryId, SyncedQuery>,
    /// Queries being computed for the next transition.
    in_progress_queries: BTreeMap<QueryId, Query>,
    identity: Identity,

    // If this is true, it means we have invalidated but have not yet refilled
    // some query subscription. `next_invalidated_query` blocks forever until
    // `fill_invalidation_futures` is called to recreate the subscriptions.
    refill_needed: bool,

    /// Updates to the query set and identity requested by the
    /// client since the last transition began computing.
    /// These are emptied before computing a new transition.
    pending_query_updates: Vec<QuerySetModification>,
    pending_identity: Option<Identity>,
    /// These are the query set version and identity according to the client.
    received_client_version: ClientVersion,
}

impl SyncState {
    pub fn new() -> Self {
        Self {
            session_id: None,
            current_version: StateVersion::initial(),
            invalidation_futures: FuturesUnordered::new(),
            queries: BTreeMap::new(),
            in_progress_queries: BTreeMap::new(),
            identity: Identity::Unknown,

            refill_needed: false,

            pending_query_updates: vec![],
            pending_identity: None,
            received_client_version: ClientVersion::initial(),
        }
    }

    pub fn set_session_id(&mut self, session_id: SessionId) {
        self.session_id = Some(session_id);
    }

    pub fn session_id(&self) -> Option<SessionId> {
        self.session_id
    }

    /// What is the current state version?
    pub fn current_version(&self) -> StateVersion {
        self.current_version
    }

    pub fn advance_version(&mut self, new_version: StateVersion) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.current_version <= new_version,
            "Version went backwards: {:?} > {:?}",
            self.current_version,
            new_version
        );
        if self.current_version == new_version {
            metrics::log_empty_transition();
        }
        self.current_version = new_version;
        Ok(())
    }

    /// Check that all queries have a subscription and token.
    pub fn validate(&self) -> anyhow::Result<()> {
        for query in self.queries.values() {
            anyhow::ensure!(query.result_hash.is_some());
            anyhow::ensure!(self.refill_needed || query.subscription.is_some());
            anyhow::ensure!(self.refill_needed || query.invalidation_future.is_some());
        }
        Ok(())
    }

    pub fn modify_query_set(
        &mut self,
        base_version: QuerySetVersion,
        new_version: QuerySetVersion,
        modifications: Vec<QuerySetModification>,
    ) -> anyhow::Result<()> {
        let current_version = self.received_client_version.query_set;
        if current_version != base_version {
            anyhow::bail!(ErrorMetadata::bad_request(
                "BaseVersionMismatch",
                format!(
                    "Base version {base_version} passed up doesn't match the current version \
                     {current_version}"
                )
            ));
        }
        anyhow::ensure!(base_version < new_version);
        self.pending_query_updates.extend(modifications);
        self.received_client_version.query_set = new_version;
        Ok(())
    }

    pub fn take_modifications(
        &mut self,
    ) -> (
        Vec<QuerySetModification>,
        QuerySetVersion,
        Option<Identity>,
        IdentityVersion,
    ) {
        (
            mem::take(&mut self.pending_query_updates),
            self.received_client_version.query_set,
            self.pending_identity.take(),
            self.received_client_version.identity,
        )
    }

    /// Set the pending identity for the current sync session, bumping the
    /// pending identity version.
    pub fn modify_identity(
        &mut self,
        new_identity: Identity,
        base_version: IdentityVersion,
    ) -> anyhow::Result<()> {
        let current_version = self.received_client_version.identity;
        anyhow::ensure!(current_version == base_version);
        self.pending_identity = Some(new_identity);
        self.received_client_version.identity = current_version + 1;
        Ok(())
    }

    /// Immediately set the current identity.
    pub fn insert_identity(&mut self, identity: Identity) {
        self.identity = identity;
    }

    // Returns the current session identity. If the identity is a user ID
    // token, also validates using the current SystemTime that it hasn't expired.
    // If there is a pending update to the identity, use that instead.
    pub fn identity(&self, current_time: SystemTime) -> anyhow::Result<Identity> {
        let identity = self
            .pending_identity
            .clone()
            .unwrap_or_else(|| self.identity.clone());
        if let Identity::User(user) = &identity {
            anyhow::ensure!(
                !user.is_expired(current_time),
                ErrorMetadata::unauthenticated("TokenExpired", "Convex token identity expired")
            );
        }
        Ok(identity)
    }

    /// Wait on the next invalidated query future to break.
    pub async fn next_invalidated_query(&mut self) -> anyhow::Result<QueryId> {
        loop {
            // `FuturesUnordered` is ready immediately if it's empty, so if it's empty, just
            // never return. The layer above will select on this future and
            // receiving a new command from the client, and it'll drop this
            // future when it gets a new command.
            if self.refill_needed || self.invalidation_futures.is_empty() {
                future::pending().await
            }
            match self.invalidation_futures.next().await {
                Some(Ok(query_id)) => {
                    let query_id = query_id?;
                    if let Some(query) = self.queries.get_mut(&query_id) {
                        // Leave the query's subscription intact since we'll look at it in
                        // `prune_invalidated_queries` below. Take the abort handle so we'll
                        // resubscribe in case this was a spurious wakeup.
                        query.invalidation_future.take();
                    }
                    self.refill_needed = true;
                    return Ok(query_id);
                },
                Some(Err(Aborted)) | None => continue,
            };
        }
    }

    /// Insert a new in-progress query. The query won't have a subscription
    /// or token, so you'll need to subsequently call
    /// `SyncState::complete_fetch` and `SyncState::fill_subscriptions` to
    /// fill out these fields.
    pub fn insert(&mut self, query: Query) -> anyhow::Result<()> {
        let query_id = query.query_id;
        if self.in_progress_queries.insert(query_id, query).is_some() {
            anyhow::bail!("Duplicate query ID: {}", query_id);
        }
        self.refill_needed = true;
        Ok(())
    }

    /// Remove a query from the query set.
    pub fn remove(&mut self, query_id: QueryId) -> anyhow::Result<()> {
        if let Some(mut query) = self.queries.remove(&query_id) {
            if let Some(handle) = query.invalidation_future.take() {
                handle.abort();
            }
        } else if self.in_progress_queries.remove(&query_id).is_some() {
            // Removed in-progress query.
        } else {
            anyhow::bail!("Nonexistent query id: {}", query_id);
        }
        Ok(())
    }

    pub fn take_subscriptions(&mut self) -> BTreeMap<QueryId, Box<dyn SubscriptionTrait>> {
        let mut newly_invalidated = BTreeMap::new();

        for (query_id, query) in self.queries.iter_mut() {
            let subscription = query.subscription.take();
            if let Some(subscription) = subscription {
                newly_invalidated.insert(*query_id, subscription);
            }
            self.refill_needed = true;
            if let Some(handle) = query.invalidation_future.take() {
                handle.abort();
            }
        }
        newly_invalidated
    }

    /// Which queries do not have a token?
    pub fn need_fetch(&self) -> impl Iterator<Item = Query> + '_ {
        self.queries
            .values()
            .filter(|sq| sq.subscription.is_none())
            .map(|sq| sq.query.clone())
            .chain(self.in_progress_queries.values().cloned())
    }

    pub fn refill_subscription(
        &mut self,
        query_id: QueryId,
        subscription: Box<dyn SubscriptionTrait>,
    ) -> anyhow::Result<()> {
        // Per the state machine, we should only be refilling subscriptions if we
        // had a valid subscription before, which means the query is non-pending
        // and has a prior result hash.
        let query = self
            .queries
            .get_mut(&query_id)
            .ok_or_else(|| anyhow::anyhow!("Nonexistent query ID: {}", query_id))?;
        anyhow::ensure!(
            query.result_hash.is_some(),
            "Refilling subscription for query with no result"
        );
        query.subscription = Some(subscription);
        Ok(())
    }

    /// Set the token for a query after successfully executing its UDF.
    pub fn complete_fetch(
        &mut self,
        query_id: QueryId,
        result: Result<ConvexValue, RedactedJsError>,
        log_lines: RedactedLogLines,
        journal: SerializedQueryJournal,
        subscription: Box<dyn SubscriptionTrait>,
    ) -> anyhow::Result<Option<StateModification<ConvexValue>>> {
        if let Some(query) = self.in_progress_queries.remove(&query_id) {
            let sq = SyncedQuery {
                query,
                subscription: None,
                result_hash: None,
                invalidation_future: None,
            };
            if self.queries.insert(query_id, sq).is_some() {
                anyhow::bail!("Duplicate query ID: {}", query_id);
            }
        }
        let query = self
            .queries
            .get_mut(&query_id)
            .ok_or_else(|| anyhow::anyhow!("Nonexistent query ID: {}", query_id))?;
        if query.subscription.is_some() {
            anyhow::bail!(
                "Completing future for query that was already up-to-date: {}",
                query_id
            );
        }

        // Save the new query journal so any recomputations will be done with it
        // present.
        query.query.journal = Some(journal.clone());

        // Cancel the query's (now out-of-date) subscription so we resubscribe in the
        // next call to `fill_subscriptions`.
        if let Some(handle) = query.invalidation_future.take() {
            handle.abort();
        }

        let new_hash = hash_result(&result, &log_lines);
        let same_result = query.result_hash.as_ref() == Some(&new_hash);
        metrics::log_query_result_dedup(same_result);

        query.result_hash = Some(new_hash);
        query.subscription = Some(subscription);

        let result = if same_result {
            None
        } else {
            let modification = match result {
                Ok(value) => StateModification::QueryUpdated {
                    query_id,
                    value,
                    log_lines: log_lines.into(),
                    journal,
                },
                Err(error) => {
                    metrics::log_query_failed();
                    StateModification::QueryFailed {
                        query_id,
                        error_message: error.to_string(),
                        log_lines: log_lines.into(),
                        journal,
                        error_data: error.custom_data_if_any(),
                    }
                },
            };
            Some(modification)
        };
        Ok(result)
    }

    /// Resubscribe queries that don't have an active invalidation future.
    pub fn fill_invalidation_futures(&mut self) -> anyhow::Result<()> {
        for (&query_id, sq) in &mut self.queries {
            if sq.invalidation_future.is_some() {
                continue;
            }
            let future = sq
                .subscription
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing subscription for {}", query_id))?
                .wait_for_invalidation()
                .map(move |r| r.map(move |()| query_id));
            let (future, handle) = future::abortable(future);
            sq.invalidation_future = Some(handle);
            self.invalidation_futures.push(future.boxed());
        }
        self.refill_needed = false;
        Ok(())
    }

    pub fn num_queries(&self) -> usize {
        self.queries.len() + self.in_progress_queries.len()
    }
}

fn hash_result(
    r: &Result<ConvexValue, RedactedJsError>,
    log_lines: &RedactedLogLines,
) -> Result<ValueDigest, ErrorDigest> {
    r.as_ref()
        .map(|v| udf_result_sha256(v, log_lines))
        .map_err(|e| {
            let mut hasher = Sha256::new();
            e.deduplication_hash(&mut hasher);
            hash_log_lines(&mut hasher, log_lines);
            hasher.finalize()
        })
}

fn udf_result_sha256(return_value: &ConvexValue, log_lines: &RedactedLogLines) -> ValueDigest {
    let mut hasher = Sha256::new();
    return_value
        .encode_for_hash(&mut hasher)
        .expect("Failed to create SHA256 digest");
    hash_log_lines(&mut hasher, log_lines);

    hasher.finalize()
}

fn hash_log_lines(hasher: &mut Sha256, log_lines: &RedactedLogLines) {
    for line in log_lines.iter() {
        // Write the line's length before its contents so we
        // don't collide with another string that shares a
        // prefix but has a different length.
        hasher.update(&line.len().to_le_bytes());
        hasher.update(line.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use application::redaction::RedactedLogLines;
    use cmd_util::env::env_config;
    use common::{
        log_lines::{
            LogLevel,
            LogLine,
            LogLines,
        },
        runtime::UnixTimestamp,
        value::ConvexValue,
    };
    use proptest::prelude::*;

    use crate::state::udf_result_sha256;

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_sha256_deterministic(v in any::<ConvexValue>(), logs in any::<LogLines>()) {
            let logs = RedactedLogLines::from_log_lines(logs, false);
            let digest = udf_result_sha256(&v, &logs);
            assert_eq!(udf_result_sha256(&v, &logs), digest);
        }

        #[test]
        fn test_sha256_collisions(
            v1 in any::<ConvexValue>(),
            v1_logs in any::<LogLines>(),
            v2 in any::<ConvexValue>(),
            v2_logs in any::<LogLines>()
        ) {
            if v1 != v2 {
                let v1_logs = RedactedLogLines::from_log_lines(v1_logs, false);
                let v2_logs = RedactedLogLines::from_log_lines(v2_logs, false);
                assert_ne!(udf_result_sha256(&v1, &v1_logs), udf_result_sha256(&v2, &v2_logs));
            }
        }
    }

    #[test]
    fn test_sha256_does_not_collide_with_similar_logs() {
        let v = ConvexValue::from(42);
        let ts = UnixTimestamp::from_millis(1715980547440);
        let v_logs = RedactedLogLines::from_log_lines(
            vec![LogLine::new_developer_log_line(
                LogLevel::Log,
                vec!["foobar".to_string()],
                ts,
            )]
            .into(),
            false,
        );
        let v2_logs = RedactedLogLines::from_log_lines(
            vec![
                LogLine::new_developer_log_line(LogLevel::Log, vec!["foo".to_string()], ts),
                LogLine::new_developer_log_line(LogLevel::Log, vec!["bar".to_string()], ts),
            ]
            .into(),
            false,
        );
        assert_ne!(
            udf_result_sha256(&v, &v_logs),
            udf_result_sha256(&v, &v2_logs)
        );
    }
}
