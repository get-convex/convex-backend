//! Tracks subscribers to document read-sets and includes functionality to
//! notify subscribers on any changes to these documents.

use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    future::Future,
    sync::{
        atomic::{
            AtomicI64,
            Ordering,
        },
        Arc,
    },
    time::Duration,
};

use ::metrics::Timer;
use common::{
    bootstrap_model::index::database_index::IndexedFields,
    document_index_keys::{
        DocumentIndexKeyValue,
        DocumentIndexKeys,
    },
    errors::report_error,
    knobs::{
        SUBSCRIPTIONS_WORKER_QUEUE_SIZE,
        SUBSCRIPTION_ADVANCE_LOG_TRACING_THRESHOLD,
        SUBSCRIPTION_INVALIDATION_DELAY_MULTIPLIER,
        SUBSCRIPTION_INVALIDATION_DELAY_THRESHOLD,
        SUBSCRIPTION_PROCESS_LOG_ENTRY_TRACING_THRESHOLD,
    },
    runtime::{
        block_in_place,
        Runtime,
        SpawnHandle,
    },
    types::{
        GenericIndexName,
        SubscriberId,
        TabletIndexName,
        Timestamp,
    },
};
use fastrace::future::FutureExt as _;
use futures::{
    future::BoxFuture,
    stream::FuturesUnordered,
    FutureExt as _,
    StreamExt as _,
};
use interval_map::IntervalMap;
use parking_lot::Mutex;
use prometheus::VMHistogram;
use search::query::TextSearchSubscriptions;
use slab::Slab;
use tokio::sync::{
    mpsc::{
        self,
        error::TrySendError,
    },
    watch,
};
use value::ResolvedDocumentId;

use crate::{
    metrics::{
        self,
        log_subscriptions_invalidated,
    },
    reads::ReadSet,
    write_log::{
        LogOwner,
        LogReader,
    },
    Token,
};

type Sequence = usize;

#[derive(Clone, Copy, Debug)]
struct SubscriptionKey {
    id: SubscriberId,
    seq: Sequence,
}

#[derive(Clone)]
pub struct SubscriptionsClient {
    handle: Arc<Mutex<Box<dyn SpawnHandle>>>,
    log: LogReader,
    sender: mpsc::Sender<SubscriptionRequest>,
}

impl SubscriptionsClient {
    pub fn subscribe(&self, token: Token) -> anyhow::Result<Subscription> {
        let token = match self.log.refresh_reads_until_max_ts(token)? {
            Ok(t) => t,
            Err(invalid_ts) => return Ok(Subscription::invalid(invalid_ts)),
        };
        let (subscription, sender) = Subscription::new(&token);
        let request = SubscriptionRequest::Subscribe { token, sender };
        self.sender.try_send(request).map_err(|e| match e {
            TrySendError::Full(..) => metrics::subscriptions_worker_full_error().into(),
            TrySendError::Closed(..) => metrics::shutdown_error(),
        })?;
        metrics::log_subscription_queue_length_delta(1);
        Ok(subscription)
    }

    pub fn shutdown(&self) {
        self.handle.lock().shutdown();
    }
}

/// The other half of a `Subscription`, owned by the subscription worker.
/// On drop, this will invalidate the subscription.
pub struct SubscriptionSender {
    validity: Arc<Validity>,
    valid_tx: watch::Sender<SubscriptionState>,
}

impl Drop for SubscriptionSender {
    fn drop(&mut self) {
        self.validity.valid_ts.store(-1, Ordering::SeqCst);
        _ = self.valid_tx.send(SubscriptionState::Invalid);
    }
}

impl SubscriptionSender {
    fn drop_with_delay(self, delay: Option<Duration>, invalid_ts: Option<Timestamp>) {
        if let Some(invalid_ts) = invalid_ts {
            self.validity.set_invalid_ts(invalid_ts);
        }
        self.validity.valid_ts.store(-1, Ordering::SeqCst);
        if let Some(delay) = delay {
            // Wait to invalidate the subscription by moving it into a new task
            tokio::spawn(async move {
                tokio::select! {
                    _ = self.valid_tx.closed() => (),
                    _ = tokio::time::sleep(delay) => (),
                }
                drop(self);
            });
        } else {
            drop(self);
        }
    }
}

enum SubscriptionRequest {
    Subscribe {
        token: Token,
        sender: SubscriptionSender,
    },
}

pub enum SubscriptionsWorker {}

impl SubscriptionsWorker {
    pub(crate) fn start<RT: Runtime>(log: LogOwner, runtime: RT) -> SubscriptionsClient {
        let (tx, rx) = mpsc::channel(*SUBSCRIPTIONS_WORKER_QUEUE_SIZE);
        let rx = CountingReceiver(rx);

        let log_reader = log.reader();
        let mut manager = SubscriptionManager::new(log);
        let handle = runtime.spawn("subscription_worker", async move {
            manager.run_worker(rx).await
        });
        SubscriptionsClient {
            handle: Arc::new(Mutex::new(handle)),
            log: log_reader,
            sender: tx,
        }
    }
}

struct CountingReceiver(mpsc::Receiver<SubscriptionRequest>);
impl Drop for CountingReceiver {
    fn drop(&mut self) {
        self.0.close();
        metrics::log_subscription_queue_length_delta(-(self.0.len() as i64));
    }
}
impl CountingReceiver {
    async fn recv(&mut self) -> Option<SubscriptionRequest> {
        let r = self.0.recv().await;
        if r.is_some() {
            metrics::log_subscription_queue_length_delta(-1);
        }
        r
    }
}

impl SubscriptionManager {
    async fn run_worker(&mut self, mut rx: CountingReceiver) {
        tracing::info!("Starting subscriptions worker");
        loop {
            futures::select_biased! {
                // N.B.: `futures` select macro (not `tokio`) needed for `select_next_some`
                key = self.closed_subscriptions.select_next_some() => {
                    self.remove(key);
                },
                request = rx.recv().fuse() => {
                    match request {
                        Some(SubscriptionRequest::Subscribe { token, sender, }) => {
                            match self.subscribe(token, sender) {
                                Ok(_) => (),
                                Err(mut e) => report_error(&mut e).await,
                            }
                        },
                        None => {
                            tracing::info!("All clients have gone away, shutting down subscriptions worker...");
                            break;
                        },
                    }
                },
                next_ts = self.log.wait_for_higher_ts(self.processed_ts).fuse() => {
                    if let Err(mut e) = self.advance_log(next_ts) {
                        report_error(&mut e).await;
                    }
                },
            }
        }
    }
}

/// Tracks all subscribers to queries and the read-set they're watching for
/// updates on.
pub struct SubscriptionManager {
    subscribers: Slab<Subscriber>,
    subscriptions: SubscriptionMap,
    next_seq: Sequence,

    closed_subscriptions: FuturesUnordered<BoxFuture<'static, SubscriptionKey>>,

    log: LogOwner,

    // The timestamp until which the worker has processed the log, which may be lagging behind
    // `conflict_checker.max_ts()`.
    //
    // Invariant: All `ReadSet` in `subscribers` have a timestamp greater than or equal to
    // `processed_ts`.
    processed_ts: Timestamp,
}

struct Subscriber {
    reads: Arc<ReadSet>,
    sender: SubscriptionSender,
    seq: Sequence,
}

impl SubscriptionManager {
    #[allow(unused)]
    #[cfg(any(test, feature = "testing"))]
    pub fn new_for_testing() -> Self {
        use crate::write_log::new_write_log;

        let (log_owner, ..) = new_write_log(Timestamp::MIN);
        Self::new(log_owner)
    }

    fn new(log: LogOwner) -> Self {
        let processed_ts = log.max_ts();
        Self {
            subscribers: Slab::new(),
            subscriptions: SubscriptionMap::new(),
            next_seq: 0,
            closed_subscriptions: FuturesUnordered::new(),
            log,
            processed_ts,
        }
    }

    pub fn subscribe(
        &mut self,
        mut token: Token,
        sender: SubscriptionSender,
    ) -> anyhow::Result<SubscriberId> {
        metrics::log_subscription_queue_lag(self.log.max_ts().secs_since_f64(token.ts()));
        // The client may not have fully refreshed their token past our
        // processed timestamp, so finish the job for them if needed.
        //
        // Note that we allow tokens to go past the end of `self.processed_ts` if the
        // subscription worker is lagging far behind the client's
        // `refresh_reads` call. This is okay since we'll only duplicate
        // processing some log entries from `(self.processed_ts, token.ts()]`.
        if token.ts() < self.processed_ts {
            token = match self.log.refresh_token(token, self.processed_ts)? {
                Ok(t) => t,
                Err(invalid_ts) => {
                    if let Some(invalid_ts) = invalid_ts {
                        sender.validity.set_invalid_ts(invalid_ts);
                    }
                    // N.B.: we only use the returned value for tests which
                    // don't encounter this case
                    return Ok(usize::MAX);
                },
            };
        }
        assert!(token.ts() >= self.processed_ts);

        let entry = self.subscribers.vacant_entry();
        let subscriber_id = entry.key();

        self.subscriptions.insert(subscriber_id, token.reads());

        let seq: usize = self.next_seq;
        let key = SubscriptionKey {
            id: subscriber_id,
            seq,
        };
        self.next_seq += 1;
        let valid_tx = sender.valid_tx.clone();
        entry.insert(Subscriber {
            reads: token.reads_owned(),
            sender,
            seq,
        });
        self.closed_subscriptions.push(
            async move {
                valid_tx.closed().await;
                key
            }
            .boxed(),
        );
        Ok(subscriber_id)
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn subscribe_for_testing(
        &mut self,
        token: Token,
    ) -> anyhow::Result<(Subscription, SubscriberId)> {
        let (subscription, sender) = Subscription::new(&token);
        let id = self.subscribe(token, sender)?;
        Ok((subscription, id))
    }

    pub fn advance_log(&mut self, next_ts: Timestamp) -> anyhow::Result<()> {
        let _timer = metrics::subscriptions_update_timer();
        block_in_place(|| {
            let from_ts = self.processed_ts.succ()?;

            let mut to_notify = BTreeMap::new();
            {
                let _timer = metrics::subscriptions_log_iterate_timer();
                let mut log_len = 0;
                let mut num_writes = 0;
                self.log.for_each(from_ts, next_ts, |write_ts, writes| {
                    let process_log_timer = metrics::subscription_process_write_log_entry_timer();
                    log_len += 1;
                    num_writes += writes.len();
                    let mut tablet_ids = BTreeSet::new();
                    let mut notify = |subscriber_id| {
                        // Always take the earliest matching write_ts
                        to_notify.entry(subscriber_id).or_insert(write_ts);
                    };
                    for (resolved_id, document_change) in writes {
                        tablet_ids.insert(resolved_id.tablet_id);
                        // We're applying a mutation to the document so if it already exists
                        // we need to remove it before writing the new version.
                        if let Some(ref old_document_keys) = document_change.old_document_keys {
                            self.overlapping(resolved_id, old_document_keys, &mut notify);
                        }
                        // If we're doing anything other than deleting the document then
                        // we'll also need to insert a new value.
                        if let Some(ref new_document_keys) = document_change.new_document_keys {
                            self.overlapping(resolved_id, new_document_keys, &mut notify);
                        }
                    }

                    if process_log_timer.elapsed()
                        > Duration::from_secs(*SUBSCRIPTION_PROCESS_LOG_ENTRY_TRACING_THRESHOLD)
                    {
                        tracing::info!(
                            "[{next_ts}: advance_log] simple commit took {:?}, affected tables: \
                             {tablet_ids:?}",
                            process_log_timer.elapsed()
                        );
                    }
                })?;
                metrics::log_subscriptions_log_processed_commits(log_len);
                metrics::log_subscriptions_log_processed_writes(num_writes);
                if _timer.elapsed()
                    > Duration::from_secs(*SUBSCRIPTION_ADVANCE_LOG_TRACING_THRESHOLD)
                {
                    let subscribers_by_index: BTreeMap<&GenericIndexName<_>, usize> = self
                        .subscriptions
                        .indexed
                        .iter()
                        .map(|(key, (_fields, range_map))| (key, range_map.subscriber_len()))
                        .collect();
                    let total_subscribers: usize = subscribers_by_index.values().sum();
                    let search_len = self.subscriptions.search.filter_len();
                    let fuzzy_len = self.subscriptions.search.fuzzy_len();
                    tracing::info!(
                        "[{next_ts} advance_log] Duration {}ms, indexes: {}, search filters: {}, \
                         fuzzy search: {}",
                        _timer.elapsed().as_millis(),
                        self.subscriptions.indexed.len(),
                        search_len,
                        fuzzy_len
                    );
                    tracing::info!(
                        "`[{next_ts} advance_log] Subscription map size: {total_subscribers}"
                    );
                    tracing::info!(
                        "[{next_ts} advance_log] Subscribers by index {subscribers_by_index:?}"
                    );
                }
            }

            {
                let _timer = metrics::subscriptions_invalidate_timer();
                // First, do a pass where we advance all of the valid subscriptions.
                for (subscriber_id, subscriber) in &mut self.subscribers {
                    if !to_notify.contains_key(&subscriber_id) {
                        subscriber.sender.validity.set_valid_ts(next_ts)
                    }
                }
                // Then, invalidate all the remaining subscriptions.
                let num_subscriptions_invalidated = to_notify.len();
                let should_splay_invalidations =
                    num_subscriptions_invalidated > *SUBSCRIPTION_INVALIDATION_DELAY_THRESHOLD;
                if should_splay_invalidations {
                    tracing::info!(
                        "Splaying subscription invalidations since there are {} subscriptions to \
                         invalidate. The threshold is {}",
                        num_subscriptions_invalidated,
                        *SUBSCRIPTION_INVALIDATION_DELAY_THRESHOLD
                    );
                }
                for (subscriber_id, invalid_ts) in to_notify {
                    let delay = should_splay_invalidations.then(|| {
                        Duration::from_millis(rand::random_range(
                            0..=num_subscriptions_invalidated as u64
                                * *SUBSCRIPTION_INVALIDATION_DELAY_MULTIPLIER,
                        ))
                    });
                    self._remove(subscriber_id, delay, Some(invalid_ts));
                }
                log_subscriptions_invalidated(num_subscriptions_invalidated);

                assert!(self.processed_ts <= next_ts);
                self.processed_ts = next_ts;
            }

            // Enforce retention after we have processed the subscriptions.
            {
                let _timer = metrics::subscriptions_log_enforce_retention_timer();
                self.log.enforce_retention_policy(next_ts);
            }

            Ok(())
        })
    }

    pub fn overlapping(
        &self,
        document_id: &ResolvedDocumentId,
        document_index_keys: &DocumentIndexKeys,
        notify: &mut impl FnMut(SubscriberId),
    ) {
        for (index, (_, range_map)) in &self.subscriptions.indexed {
            if *index.table() == document_id.tablet_id {
                let Some(DocumentIndexKeyValue::Standard(index_key)) =
                    document_index_keys.get(index)
                else {
                    metrics::log_missing_index_key_subscriptions();
                    continue;
                };
                range_map.query(index_key, &mut *notify);
            }
        }

        self.subscriptions
            .search
            .add_matches(document_id, document_index_keys, notify);
    }

    fn get_subscriber(&self, key: SubscriptionKey) -> Option<&Subscriber> {
        let entry = self.subscribers.get(key.id)?;
        if entry.seq > key.seq {
            return None;
        }
        assert_eq!(entry.seq, key.seq);
        Some(entry)
    }

    /// Remove the given subscription if it exists.
    fn remove(&mut self, key: SubscriptionKey) {
        // Don't remove anything if `key` is no longer valid.
        if self.get_subscriber(key).is_none() {
            return;
        }
        self._remove(key.id, None, None);
    }

    fn _remove(
        &mut self,
        id: SubscriberId,
        delay: Option<Duration>,
        invalid_ts: Option<Timestamp>,
    ) {
        let entry = self.subscribers.remove(id);
        self.subscriptions.remove(id, &entry.reads);
        // dropping `entry.sender` will invalidate the subscription
        entry.sender.drop_with_delay(delay, invalid_ts);
    }
}

#[derive(Copy, Clone)]
enum SubscriptionState {
    Valid,
    Invalid,
}

struct Validity {
    /// -1 means invalid, in which case `invalid_ts` may be populated
    valid_ts: AtomicI64,
    /// -1 means unknown
    invalid_ts: AtomicI64,
}

impl Validity {
    fn valid(ts: Timestamp) -> Self {
        Self {
            valid_ts: AtomicI64::new(ts.into()),
            invalid_ts: AtomicI64::new(-1),
        }
    }

    fn invalid(invalid_ts: Option<Timestamp>) -> Validity {
        Self {
            valid_ts: AtomicI64::new(-1),
            invalid_ts: AtomicI64::new(invalid_ts.map_or(-1, i64::from)),
        }
    }

    fn valid_ts(&self) -> Option<Timestamp> {
        match self.valid_ts.load(Ordering::SeqCst) {
            -1 => None,
            ts => Some(
                ts.try_into()
                    .expect("only legal timestamp values can be written to valid_ts"),
            ),
        }
    }

    fn set_valid_ts(&self, ts: Timestamp) {
        self.valid_ts.store(ts.into(), Ordering::SeqCst);
    }

    fn invalid_ts(&self) -> Option<Timestamp> {
        match self.invalid_ts.load(Ordering::SeqCst) {
            -1 => None,
            ts => Some(
                ts.try_into()
                    .expect("only legal timestamp values can be written to invalid_ts"),
            ),
        }
    }

    fn set_invalid_ts(&self, ts: Timestamp) {
        self.invalid_ts.store(ts.into(), Ordering::SeqCst);
    }
}

/// A subscription on a set of read keys from a prior read-only transaction.
#[must_use]
pub struct Subscription {
    validity: Arc<Validity>,
    // May lag behind `validity` in case of subscription splaying
    valid: watch::Receiver<SubscriptionState>,
    _timer: Timer<VMHistogram>,
}

impl Subscription {
    fn new(token: &Token) -> (Self, SubscriptionSender) {
        let validity = Arc::new(Validity::valid(token.ts()));
        let (valid_tx, valid_rx) = watch::channel(SubscriptionState::Valid);
        let subscription = Subscription {
            validity: validity.clone(),
            valid: valid_rx,
            _timer: metrics::subscription_timer(),
        };
        (subscription, SubscriptionSender { validity, valid_tx })
    }

    fn invalid(invalid_ts: Option<Timestamp>) -> Self {
        let (_, receiver) = watch::channel(SubscriptionState::Invalid);
        Subscription {
            validity: Arc::new(Validity::invalid(invalid_ts)),
            valid: receiver,
            _timer: metrics::subscription_timer(),
        }
    }

    pub fn current_ts(&self) -> Option<Timestamp> {
        self.validity.valid_ts()
    }

    pub fn invalid_ts(&self) -> Option<Timestamp> {
        self.validity.invalid_ts()
    }

    pub fn wait_for_invalidation(&self) -> impl Future<Output = Option<Timestamp>> {
        let mut valid = self.valid.clone();
        let validity = self.validity.clone();
        let span = fastrace::Span::enter_with_local_parent("wait_for_invalidation");
        async move {
            let _: Result<_, _> = valid
                .wait_for(|state| matches!(state, SubscriptionState::Invalid))
                .await;
            validity.invalid_ts()
        }
        .in_span(span)
    }
}

/// Tracks every subscriber for a given read-set.
struct SubscriptionMap {
    // TODO: remove nesting, merge all IntervalMaps into one big data structure
    indexed: BTreeMap<TabletIndexName, (IndexedFields, IntervalMap)>,
    search: TextSearchSubscriptions,
}

impl SubscriptionMap {
    fn new() -> Self {
        Self {
            indexed: BTreeMap::new(),
            search: TextSearchSubscriptions::new(),
        }
    }

    fn insert(&mut self, id: SubscriberId, reads: &ReadSet) {
        for (index, index_reads) in reads.iter_indexed() {
            let (_, interval_map) = self
                .indexed
                .entry(index.clone())
                .or_insert_with(|| (index_reads.fields.clone(), IntervalMap::new()));
            interval_map
                .insert(id, index_reads.intervals.iter())
                .expect("stored more than u32::MAX intervals?");
        }
        for (index, reads) in reads.iter_search() {
            self.search.insert(id, index, reads);
        }
    }

    fn remove(&mut self, id: SubscriberId, reads: &ReadSet) {
        for (index, _) in reads.iter_indexed() {
            let (_, range_map) = self
                .indexed
                .get_mut(index)
                .unwrap_or_else(|| panic!("Missing index entry for {index}"));
            range_map.remove(id);
            if range_map.is_empty() {
                self.indexed.remove(index);
            }
        }
        for (index, reads) in reads.iter_search() {
            self.search.remove(id, index, reads);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{
            BTreeMap,
            BTreeSet,
        },
        ops::Range,
        str::FromStr,
        time::Duration,
    };

    use cmd_util::env::env_config;
    use common::{
        document::{
            CreationTime,
            PackedDocument,
            ResolvedDocument,
        },
        document_index_keys::DocumentIndexKeys,
        runtime::testing::TestDriver,
        testing::TestIdGenerator,
        types::{
            GenericIndexName,
            IndexDescriptor,
            SubscriberId,
            TabletIndexName,
        },
    };
    use convex_macro::test_runtime;
    use itertools::Itertools;
    use maplit::btreeset;
    use proptest::{
        collection::VecStrategy,
        prelude::*,
        sample::SizeRange,
        string::{
            string_regex,
            RegexGeneratorStrategy,
        },
    };
    use proptest_derive::Arbitrary;
    use runtime::testing::TestRuntime;
    use search::{
        query::{
            tokenize,
            FuzzyDistance,
            TextQueryTerm,
        },
        EXACT_SEARCH_MAX_WORD_LENGTH,
        SINGLE_TYPO_SEARCH_MAX_WORD_LENGTH,
    };
    use sync_types::Timestamp;
    use tokio::sync::mpsc;
    use value::{
        ConvexObject,
        ConvexString,
        ConvexValue,
        DeveloperDocumentId,
        FieldName,
        FieldPath,
        ResolvedDocumentId,
        TableNumber,
        TabletId,
        TabletIdAndTableNumber,
    };

    use crate::{
        subscription::{
            CountingReceiver,
            SubscriptionManager,
        },
        ReadSet,
        Token,
    };

    fn tokens_only(
        num_tokens: impl Into<SizeRange>,
        num_chars: Range<usize>,
    ) -> VecStrategy<RegexGeneratorStrategy<String>> {
        prop::collection::vec(
            string_regex(&format!("[a-z]{{{}, {}}}", num_chars.start, num_chars.end)).unwrap(),
            num_tokens,
        )
    }

    prop_compose! {
        fn search_token(num_tokens: impl Into<SizeRange>, num_chars: Range<usize>) (
            tokens in tokens_only(num_tokens, num_chars),
            tablet_id in any::<TabletId>(),
            field_path in any::<FieldPath>(),
            max_distance in any::<FuzzyDistance>(),
            prefix in any::<bool>(),
        ) -> Token {
            let index_name = create_index_name(tablet_id);
            Token::text_search_token(
                index_name,
                field_path,
                tokens.into_iter().map(|token| {
                    TextQueryTerm::Fuzzy { token, max_distance, prefix }
                }).collect_vec())
        }
    }

    prop_compose! {
        // NOTE: Token can refer either to the Token in subscriptions (like this
        // method) or to an individual word in a search query (like the
        // search_token() method we call below). The overlap is unfortunate :/
        fn search_tokens(size: impl Into<SizeRange>) (
            tokens in prop::collection::vec(search_token(0..15, 1..31), size)
        ) -> Vec<Token> {
            tokens
        }
    }

    #[derive(Debug, Arbitrary, PartialEq, Eq)]
    enum MismatchType {
        Prefix,
        Typo,
    }

    fn max_distance(token: &str) -> FuzzyDistance {
        let num_chars = token.chars().count();
        if num_chars > SINGLE_TYPO_SEARCH_MAX_WORD_LENGTH {
            FuzzyDistance::Two
        } else if num_chars > EXACT_SEARCH_MAX_WORD_LENGTH {
            FuzzyDistance::One
        } else {
            FuzzyDistance::Zero
        }
    }

    prop_compose! {
        fn token_and_mismatch(num_chars: Range<usize>) (
            tokens in tokens_only(1, num_chars),
            tablet_id in any::<TabletId>(),
            field_path in any::<FieldPath>(),
            prefix in any::<bool>(),
        ) (
            token in Just(tokens[0].clone()),
            tablet_id in Just(tablet_id),
            field_path in Just(field_path),
            prefix in Just(prefix),
            mismatch_token in mismatch(tokens[0].clone())
        ) -> (Token, Token) {
            let max_distance = max_distance(&token);
            let index_name = create_index_name(tablet_id);
            (
                Token::text_search_token(
                    index_name.clone(),
                    field_path.clone(),
                    vec![TextQueryTerm::Fuzzy { token, max_distance, prefix }]),
                Token::text_search_token(
                    index_name,
                    field_path,
                    vec![TextQueryTerm::Fuzzy { token: mismatch_token, max_distance, prefix }]),
            )
        }
    }

    prop_compose! {
        fn mismatch(token: String) (
            mismatch_type in any::<MismatchType>()
        ) -> String {
            let max_distance = max_distance(&token);
            match mismatch_type {
                MismatchType::Prefix =>
                    add_prefix(add_typos(token.clone(), *max_distance), max_distance),
                MismatchType::Typo => add_typos(token.clone(), *max_distance + 1),
            }
        }
    }

    fn add_prefix(token: String, max_distance: FuzzyDistance) -> String {
        let prefix = (0..=*max_distance).map(|_| "ü").join("");
        format!("{prefix}{token}")
    }

    fn add_typos(token: String, distance: u8) -> String {
        let mut result = String::from("");
        let distance: usize = distance.into();

        for (i, char) in token.chars().enumerate() {
            // Use a constant character that cannot be present in the token based on our
            // regex for simplicity. If we use a valid character, then we might
            // accidentally introduce a transposition instead of a prefix.
            result.push(if i < distance { 'ü' } else { char });
        }
        result.to_string()
    }

    fn create_matching_documents(
        read_set: &ReadSet,
        id_generator: &mut TestIdGenerator,
    ) -> Vec<(PackedDocument, FieldPath)> {
        let mut result = Vec::new();
        for (index_name, reads) in read_set.iter_search() {
            for query in &reads.text_queries {
                // All we need is the table id of the index to match the table id of the doc.
                let internal_id = id_generator.generate_internal();
                let id = ResolvedDocumentId::new(
                    *index_name.table(),
                    DeveloperDocumentId::new(TableNumber::try_from(1).unwrap(), internal_id),
                );
                assert_eq!(*index_name.table(), id.tablet_id);

                let document = pack(&create_document(
                    query.field_path.clone(),
                    match &query.term {
                        TextQueryTerm::Exact(term) => term.clone(),
                        TextQueryTerm::Fuzzy { token, .. } => token.clone(),
                    },
                    id,
                ));
                assert_eq!(*index_name.table(), document.id().tablet_id);
                result.push((document, query.field_path.clone()));
            }
        }
        result
    }

    fn pack(doc: &ResolvedDocument) -> PackedDocument {
        PackedDocument::pack(doc)
    }

    fn create_document(
        field_path: FieldPath,
        field_value: String,
        id: ResolvedDocumentId,
    ) -> ResolvedDocument {
        let object = create_object(field_path, field_value);
        let time =
            CreationTime::try_from(Timestamp::MIN.add(Duration::from_secs(1)).unwrap()).unwrap();
        ResolvedDocument::new(id, time, object).unwrap()
    }

    fn create_object(field_path: FieldPath, field_value: String) -> ConvexObject {
        let mut map: BTreeMap<FieldName, ConvexValue> = BTreeMap::new();
        let name = field_path.fields().last().unwrap();
        map.insert(
            FieldName::from(name.clone()),
            ConvexValue::String(ConvexString::try_from(field_value).unwrap()),
        );
        let mut object = ConvexObject::try_from(map).unwrap();

        for field in field_path.fields().iter().rev().skip(1) {
            let mut new_map = BTreeMap::new();
            new_map.insert(FieldName::from(field.clone()), ConvexValue::Object(object));
            object = ConvexObject::try_from(new_map).unwrap();
        }
        object
    }

    fn create_index_name(tablet_id: TabletId) -> TabletIndexName {
        GenericIndexName::new(tablet_id, IndexDescriptor::new("index").unwrap()).unwrap()
    }

    fn create_search_token(
        table_id: TabletIdAndTableNumber,
        terms: Vec<TextQueryTerm>,
    ) -> anyhow::Result<Token> {
        let field_path = FieldPath::from_str("path")?;

        Ok(Token::text_search_token(
            create_index_name(table_id.tablet_id),
            field_path,
            terms,
        ))
    }

    #[test_runtime]
    async fn add_remove_two_identical_search_subscriptions_different_subscribers(
        _rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let mut id_generator = TestIdGenerator::new();
        let table_id = id_generator.user_table_id(&id_generator.generate_table_name());
        let token = "token".to_string();
        let first = create_search_token(
            table_id,
            vec![TextQueryTerm::Fuzzy {
                token: token.clone(),
                prefix: false,
                max_distance: FuzzyDistance::One,
            }],
        )?;
        let second = create_search_token(
            table_id,
            vec![TextQueryTerm::Fuzzy {
                token,
                prefix: false,
                max_distance: FuzzyDistance::One,
            }],
        )?;

        let mut subscription_manager = SubscriptionManager::new_for_testing();
        let mut subscriptions = vec![];
        let tokens = vec![first, second];
        for token in &tokens {
            let (subscriber, id) = subscription_manager
                .subscribe_for_testing(token.clone())
                .unwrap();
            subscriptions.push(subscriber);
            subscription_manager._remove(id, None, None);
        }

        assert!(
            notify_subscribed_tokens(&mut id_generator, &mut subscription_manager, tokens)
                .is_empty()
        );

        Ok(())
    }

    #[test_runtime]
    async fn add_remove_two_identical_search_subscriptions_same_subscriber(
        _rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let mut id_generator = TestIdGenerator::new();
        let table_id = id_generator.user_table_id(&id_generator.generate_table_name());
        let token = "token".to_string();
        let token = create_search_token(
            table_id,
            vec![
                TextQueryTerm::Fuzzy {
                    token: token.clone(),
                    prefix: false,
                    max_distance: FuzzyDistance::One,
                },
                TextQueryTerm::Fuzzy {
                    token,
                    prefix: false,
                    max_distance: FuzzyDistance::One,
                },
            ],
        )?;

        let mut subscription_manager = SubscriptionManager::new_for_testing();
        let (_subscription, id) = subscription_manager
            .subscribe_for_testing(token.clone())
            .unwrap();
        subscription_manager._remove(id, None, None);

        assert!(notify_subscribed_tokens(
            &mut id_generator,
            &mut subscription_manager,
            vec![token]
        )
        .is_empty());

        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

        #[test]
        fn search_subscriptions_are_notified(tokens in search_tokens(0..10)) {
            let test = async move {
                let mut id_generator = TestIdGenerator::new();
                let mut subscription_manager = SubscriptionManager::new_for_testing();
                let mut subscriptions = vec![];
                for token in &tokens {
                    subscriptions.push(
                        subscription_manager.subscribe_for_testing(token.clone()).unwrap(),
                    );
                }
                for (token, (_, id)) in tokens.into_iter().zip(subscriptions.into_iter()) {
                    let notifications = notify_subscribed_tokens(
                        &mut id_generator,
                        &mut subscription_manager,
                        vec![token.clone()],
                    );

                    if !contains_text_query(token) {
                        assert!(notifications.is_empty());
                    } else {
                        assert_eq!(notifications, btreeset! { id });
                    }
                }
                anyhow::Ok(())
            };
            TestDriver::new().run_until(test).unwrap();
        }

        #[test]
        fn mismatched_subscriptions_are_not_notified(
            (token, mismatch) in token_and_mismatch(1..31)
        ) {
            let test = async move {
                let mut id_generator = TestIdGenerator::new();
                let mut subscription_manager = SubscriptionManager::new_for_testing();
                _ = subscription_manager.subscribe_for_testing(token.clone()).unwrap();
                let notifications =
                    notify_subscribed_tokens(
                        &mut id_generator, &mut subscription_manager, vec![mismatch]
                    );
                assert!(notifications.is_empty());
                anyhow::Ok(())
            };
            TestDriver::new().run_until(test).unwrap();
        }

        #[test]
        fn removed_search_subscriptions_are_not_notified(tokens in search_tokens(0..10)) {
            let test = async move {
                let mut id_generator = TestIdGenerator::new();
                let mut subscription_manager = SubscriptionManager::new_for_testing();
                let mut subscriptions = vec![];
                for token in &tokens {
                    subscriptions.push(
                        subscription_manager.subscribe_for_testing(token.clone()).unwrap(),
                    );
                }
                for (_, id) in &subscriptions {
                    subscription_manager._remove(*id, None, None);
                }
                let notifications = notify_subscribed_tokens(
                    &mut id_generator,
                    &mut subscription_manager,
                    tokens,
                );
                assert!(notifications.is_empty());
                anyhow::Ok(())
            };
            TestDriver::new().run_until(test).unwrap();
        }

        // A more constrained version of the above test that's more likely to generate edge cases
        // like duplicate tokens
        #[test]
        fn constrained_removed_search_subscriptions_are_not_notified(
            tokens in prop::collection::vec(search_token(10..=10, 3..4), 20)
        ) {
            let test = async move {
                let mut id_generator = TestIdGenerator::new();
                let mut subscription_manager = SubscriptionManager::new_for_testing();
                for token in &tokens {
                    let (_subscription, id) = subscription_manager
                        .subscribe_for_testing(token.clone()).unwrap();
                    subscription_manager._remove(id, None, None);
                }
                let notifications = notify_subscribed_tokens(
                    &mut id_generator,
                    &mut subscription_manager,
                    tokens
                );
                assert!(notifications.is_empty());
                anyhow::Ok(())
            };
            TestDriver::new().run_until(test).unwrap();
        }
    }

    fn contains_text_query(token: Token) -> bool {
        token
            .reads()
            .iter_search()
            .any(|(_, reads)| !reads.text_queries.is_empty())
    }

    fn notify_subscribed_tokens(
        id_generator: &mut TestIdGenerator,
        subscription_manager: &mut SubscriptionManager,
        tokens: Vec<Token>,
    ) -> BTreeSet<SubscriberId> {
        let mut to_notify = BTreeSet::new();
        for token in tokens {
            let documents = create_matching_documents(token.reads(), id_generator);

            for (doc, search_field) in documents {
                let search_field_value = match doc.value().get_path(&search_field) {
                    Some(ConvexValue::String(s)) => s.clone(),
                    _ => panic!("Expected string value in {:?}", doc.value()),
                };

                subscription_manager.overlapping(
                    &doc.id(),
                    &DocumentIndexKeys::with_search_index_for_test(
                        create_index_name(doc.id().tablet_id),
                        search_field,
                        tokenize(search_field_value),
                    ),
                    &mut |id| {
                        to_notify.insert(id);
                    },
                );
            }
        }
        to_notify
    }

    fn disconnected_rx() -> CountingReceiver {
        CountingReceiver(mpsc::channel(1).1)
    }

    #[tokio::test]
    async fn test_cleans_up_dropped_subscriptions() {
        let mut subscription_manager = SubscriptionManager::new_for_testing();
        let (subscription, id) = subscription_manager
            .subscribe_for_testing(Token::empty(Timestamp::MIN))
            .unwrap();
        subscription_manager.run_worker(disconnected_rx()).await;
        assert!(subscription_manager.subscribers.get(id).is_some());
        // The worker should notice that the `Subscription` dropped and clean up its
        // state.
        drop(subscription);
        // HAX: this is relying on the fact that `run_worker` internally uses
        // `select_biased!` and polls for closed subscriptions before reading
        // from `rx`
        subscription_manager.run_worker(disconnected_rx()).await;
        assert!(subscription_manager.subscribers.get(id).is_none());
        assert!(subscription_manager.subscribers.is_empty());
    }
}
