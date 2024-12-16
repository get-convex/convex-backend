//! Tracks subscribers to document read-sets and includes functionality to
//! notify subscribers on any changes to these documents.

use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    future::Future,
    sync::Arc,
};

use ::metrics::Timer;
use common::{
    bootstrap_model::index::database_index::IndexedFields,
    document::PackedDocument,
    errors::report_error,
    runtime::{
        block_in_place,
        Runtime,
        SpawnHandle,
    },
    types::{
        PersistenceVersion,
        SubscriberId,
        TabletIndexName,
        Timestamp,
    },
};
use indexing::interval::IntervalMap;
use minitrace::future::FutureExt as MinitraceFutureExt;
use parking_lot::Mutex;
use prometheus::VMHistogram;
use search::query::TextSearchSubscriptions;
use slab::Slab;
use tokio::sync::{
    mpsc::{
        self,
        error::TrySendError,
    },
    oneshot,
    watch,
};

use crate::{
    metrics,
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

// Provide large enough limit so we never hit this in practice.
const SUBSCRIPTIONS_BUFFER: usize = 10000;

#[derive(Clone)]
pub struct SubscriptionsClient {
    handle: Arc<Mutex<Box<dyn SpawnHandle>>>,
    log: LogReader,
    sender: mpsc::Sender<SubscriptionRequest>,
}

impl SubscriptionsClient {
    pub async fn subscribe(&self, token: Token) -> anyhow::Result<Subscription> {
        let token = match self.log.refresh_reads_until_max_ts(token)? {
            Some(t) => t,
            None => return Ok(Subscription::invalid(self.sender.clone())),
        };
        let (tx, rx) = oneshot::channel();
        let request = SubscriptionRequest::Subscribe { token, result: tx };
        self.sender.clone().try_send(request).map_err(|e| match e {
            TrySendError::Full(..) => metrics::subscriptions_worker_full_error().into(),
            TrySendError::Closed(..) => metrics::shutdown_error(),
        })?;
        // The only reason we might fail here if the subscriptions worker is shutting
        // down.
        rx.await.map_err(|_| metrics::shutdown_error())
    }

    pub fn shutdown(&self) {
        self.handle.lock().shutdown();
    }
}

enum SubscriptionRequest {
    Subscribe {
        token: Token,

        // Returns a subscriptions and the timestamp it was created.
        // The client is responsible to check the log until the timestamp.
        result: oneshot::Sender<Subscription>,
    },
    Cancel(SubscriptionKey),
}

pub struct SubscriptionsWorker {
    subscriptions: SubscriptionManager,
}

impl SubscriptionsWorker {
    pub(crate) fn start<RT: Runtime>(
        log: LogOwner,
        runtime: RT,
        persistence_version: PersistenceVersion,
    ) -> SubscriptionsClient {
        let (tx, rx) = mpsc::channel(SUBSCRIPTIONS_BUFFER);

        let log_reader = log.reader();
        let worker = Self {
            subscriptions: SubscriptionManager::new(tx.clone(), log, persistence_version),
        };

        let handle = runtime.spawn("subscription_worker", worker.go(rx));
        SubscriptionsClient {
            handle: Arc::new(Mutex::new(handle)),
            log: log_reader,
            sender: tx,
        }
    }

    async fn go(mut self, mut rx: mpsc::Receiver<SubscriptionRequest>) {
        tracing::info!("Starting subscriptions worker");
        loop {
            tokio::select! {
                request = rx.recv() => {
                    match request {
                        Some(SubscriptionRequest::Subscribe { token, result }) => {
                            match self.subscriptions.subscribe(token) {
                                Ok(s) => {
                                    let _: Result<_, _> = result.send(s);
                                },
                                Err(mut e) => report_error(&mut e).await,
                            }
                        },
                        Some(SubscriptionRequest::Cancel(key)) => {
                            self.subscriptions.remove(key);
                        },
                        None => {
                            tracing::info!("All clients have gone away, shutting down subscriptions worker...");
                            break;
                        },
                    }
                },
                next_ts = self.subscriptions.wait_for_next_ts() => {
                    if let Err(mut e) = self.subscriptions.advance_log(next_ts) {
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

    log: LogOwner,

    // The timestamp until which the worker has processed the log, which may be lagging behind
    // `conflict_checker.max_ts()`.
    //
    // Invariant: All `ReadSet` in `subscribers` have a timestamp greater than or equal to
    // `processed_ts`.
    processed_ts: Timestamp,

    sender: mpsc::Sender<SubscriptionRequest>,
    persistence_version: PersistenceVersion,
}

struct Subscriber {
    reads: Arc<ReadSet>,
    valid_ts: Arc<Mutex<Option<Timestamp>>>,
    valid: watch::Sender<SubscriptionState>,
    seq: Sequence,
}

impl SubscriptionManager {
    #[allow(unused)]
    #[cfg(any(test, feature = "testing"))]
    pub fn new_for_testing() -> Self {
        use crate::write_log::new_write_log;

        let (log_owner, ..) = new_write_log(Timestamp::MIN, PersistenceVersion::V5);
        let (tx, _) = mpsc::channel(10);
        Self::new(tx, log_owner, PersistenceVersion::V5)
    }

    fn new(
        tx: mpsc::Sender<SubscriptionRequest>,
        log: LogOwner,
        persistence_version: PersistenceVersion,
    ) -> Self {
        let processed_ts = log.max_ts();
        Self {
            subscribers: Slab::new(),
            subscriptions: SubscriptionMap::new(),
            next_seq: 0,
            log,
            processed_ts,
            sender: tx,
            persistence_version,
        }
    }

    pub fn subscribe(&mut self, mut token: Token) -> anyhow::Result<Subscription> {
        // The client may not have fully refreshed their token past our
        // processed timestamp, so finish the job for them if needed.
        //
        // Note that we allow tokens to go past the end of `self.processed_ts` if the
        // subscription worker is lagging far behind the client's
        // `refresh_reads` call. This is okay since we'll only duplicate
        // processing some log entries from `(self.processed_ts, token.ts()]`.
        if token.ts() < self.processed_ts {
            token = match self.log.refresh_token(token, self.processed_ts)? {
                Some(t) => t,
                None => return Ok(Subscription::invalid(self.sender.clone())),
            };
        }
        assert!(token.ts() >= self.processed_ts);

        let entry = self.subscribers.vacant_entry();
        let subscriber_id = entry.key();

        self.subscriptions.insert(subscriber_id, token.reads());

        let valid_ts = Arc::new(Mutex::new(Some(token.ts())));
        let (valid_tx, valid_rx) = watch::channel(SubscriptionState::Valid);
        let seq: usize = self.next_seq;
        let key = SubscriptionKey {
            id: subscriber_id,
            seq,
        };
        self.next_seq += 1;
        entry.insert(Subscriber {
            reads: token.reads_owned(),
            valid_ts: valid_ts.clone(),
            valid: valid_tx,
            seq,
        });
        let subscription = Subscription {
            valid_ts,
            valid: valid_rx,
            key: Some(key),
            sender: self.sender.clone(),
            _timer: metrics::subscription_timer(),
        };
        Ok(subscription)
    }

    pub fn advance_log(&mut self, next_ts: Timestamp) -> anyhow::Result<()> {
        let _timer = metrics::subscriptions_update_timer();
        block_in_place(|| {
            let from_ts = self.processed_ts.succ()?;

            let mut to_notify = BTreeSet::new();
            self.log.for_each(from_ts, next_ts, |_, writes| {
                for (_, document_change) in writes {
                    // We're applying a mutation to the document so if it already exists
                    // we need to remove it before writing the new version.
                    if let Some(ref old_document) = document_change.old_document {
                        self.overlapping(old_document, &mut to_notify, self.persistence_version);
                    }
                    // If we're doing anything other than deleting the document then
                    // we'll also need to insert a new value.
                    if let Some(ref new_document) = document_change.new_document {
                        self.overlapping(new_document, &mut to_notify, self.persistence_version);
                    }
                }
            })?;

            // First, do a pass where we advance all of the valid subscriptions.
            for (subscriber_id, subscriber) in &mut self.subscribers {
                if !to_notify.contains(&subscriber_id) {
                    *subscriber.valid_ts.lock() = Some(next_ts);
                }
            }
            // Then, invalidate all the remaining subscriptions.
            for subscriber_id in to_notify {
                self._remove(subscriber_id);
            }

            assert!(self.processed_ts <= next_ts);
            self.processed_ts = next_ts;

            // Enforce retention after we have processed the subscriptions.
            self.log.enforce_retention_policy(next_ts);

            Ok(())
        })
    }

    pub async fn wait_for_next_ts(&mut self) -> Timestamp {
        self.log.wait_for_higher_ts(self.processed_ts).await
    }

    #[allow(unused)]
    #[cfg(any(test, feature = "testing"))]
    pub fn overlapping_for_testing(
        &self,
        document: &PackedDocument,
        to_notify: &mut BTreeSet<SubscriberId>,
        persistence_version: PersistenceVersion,
    ) {
        self.overlapping(document, to_notify, persistence_version);
    }

    fn overlapping(
        &self,
        document: &PackedDocument,
        to_notify: &mut BTreeSet<SubscriberId>,
        persistence_version: PersistenceVersion,
    ) {
        for (index, (fields, range_map)) in &self.subscriptions.indexed {
            if *index.table() == document.id().tablet_id {
                let index_key = document.index_key(fields, persistence_version);
                for subscriber_id in range_map.query(index_key.into_bytes()) {
                    to_notify.insert(subscriber_id);
                }
            }
        }
        self.subscriptions.search.add_matches(document, to_notify);
    }

    fn get_subscriber(&self, key: SubscriptionKey) -> Option<&Subscriber> {
        let entry = match self.subscribers.get(key.id) {
            None => return None,
            Some(e) => e,
        };
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
        self._remove(key.id);
    }

    fn _remove(&mut self, id: SubscriberId) {
        let entry = self.subscribers.remove(id);
        *entry.valid_ts.lock() = None;
        let _ = entry.valid.send(SubscriptionState::Invalid);
        self.subscriptions.remove(id, &entry.reads);
    }
}

#[derive(Copy, Clone)]
enum SubscriptionState {
    Valid,
    Invalid,
}

/// A subscription on a set of read keys from a prior read-only transaction.
pub struct Subscription {
    valid_ts: Arc<Mutex<Option<Timestamp>>>,
    valid: watch::Receiver<SubscriptionState>,
    key: Option<SubscriptionKey>,
    sender: mpsc::Sender<SubscriptionRequest>,
    _timer: Timer<VMHistogram>,
}

impl Subscription {
    #[allow(unused)]
    #[cfg(any(test, feature = "testing"))]
    fn id(&self) -> &SubscriberId {
        &self.key.as_ref().unwrap().id
    }

    fn invalid(sender: mpsc::Sender<SubscriptionRequest>) -> Self {
        let (_, receiver) = watch::channel(SubscriptionState::Invalid);
        Subscription {
            valid_ts: Arc::new(Mutex::new(None)),
            valid: receiver,
            key: None,
            sender,
            _timer: metrics::subscription_timer(),
        }
    }

    pub fn current_ts(&self) -> Option<Timestamp> {
        *self.valid_ts.lock()
    }

    pub fn wait_for_invalidation(&self) -> impl Future<Output = ()> {
        let mut valid = self.valid.clone();
        let span = minitrace::Span::enter_with_local_parent("wait_for_invalidation");
        async move {
            let _: Result<_, _> = valid
                .wait_for(|state| matches!(state, SubscriptionState::Invalid))
                .await;
        }
        .in_span(span)
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(key) = self.key.take() {
            let _: Result<_, _> = self.sender.try_send(SubscriptionRequest::Cancel(key));
        }
    }
}

/// Tracks every subscriber for a given read-set.
struct SubscriptionMap {
    indexed: BTreeMap<TabletIndexName, (IndexedFields, IntervalMap<SubscriberId>)>,
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
            interval_map.insert(id, index_reads.intervals.clone());
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
                .unwrap_or_else(|| panic!("Missing index entry for {}", index));
            assert!(range_map.remove(id).is_some());
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
        runtime::testing::TestDriver,
        testing::TestIdGenerator,
        types::{
            GenericIndexName,
            IndexDescriptor,
            PersistenceVersion,
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
            FuzzyDistance,
            TextQueryTerm,
        },
        EXACT_SEARCH_MAX_WORD_LENGTH,
        SINGLE_TYPO_SEARCH_MAX_WORD_LENGTH,
    };
    use sync_types::Timestamp;
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
        subscription::SubscriptionManager,
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
            index_name in any::<TabletIndexName>(),
            field_path in any::<FieldPath>(),
            max_distance in any::<FuzzyDistance>(),
            prefix in any::<bool>(),
        ) -> Token {
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
            index_name in any::<TabletIndexName>(),
            field_path in any::<FieldPath>(),
            prefix in any::<bool>(),
        ) (
            token in Just(tokens.clone().remove(0)),
            index_name in Just(index_name),
            field_path in Just(field_path),
            prefix in Just(prefix),
            mismatch_token in mismatch(tokens.clone().remove(0))
        ) -> (Token, Token) {
            let max_distance = max_distance(&token);
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
        format!("{}{}", prefix, token)
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
    ) -> Vec<PackedDocument> {
        let mut result: Vec<PackedDocument> = vec![];
        for (index_name, reads) in read_set.iter_search() {
            for query in &reads.text_queries {
                // All we need is the table id of the index to match the table id of the doc.
                let internal_id = id_generator.generate_internal();
                let id = ResolvedDocumentId::new(
                    *index_name.table(),
                    DeveloperDocumentId::new(TableNumber::try_from(1).unwrap(), internal_id),
                );
                assert_eq!(*index_name.table(), id.tablet_id);

                let document = pack(create_document(
                    query.field_path.clone(),
                    match &query.term {
                        TextQueryTerm::Exact(term) => term.clone(),
                        TextQueryTerm::Fuzzy { token, .. } => token.clone(),
                    },
                    id,
                ));
                assert_eq!(*index_name.table(), document.id().tablet_id);
                result.push(document)
            }
        }
        result
    }

    fn pack(doc: ResolvedDocument) -> PackedDocument {
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

    fn create_search_token(
        table_id: TabletIdAndTableNumber,
        terms: Vec<TextQueryTerm>,
    ) -> anyhow::Result<Token> {
        let index_name: GenericIndexName<TabletId> = GenericIndexName::new(
            table_id.tablet_id,
            IndexDescriptor::from_str("index").unwrap(),
        )?;
        let field_path = FieldPath::from_str("path")?;

        Ok(Token::text_search_token(
            index_name.clone(),
            field_path.clone(),
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
            subscriptions.push(subscription_manager.subscribe(token.clone()).unwrap());
        }
        for subscription in subscriptions {
            subscription_manager._remove(*subscription.id());
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
        let subscription = subscription_manager.subscribe(token.clone()).unwrap();
        subscription_manager._remove(*subscription.id());

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
                    subscriptions.push(subscription_manager.subscribe(token.clone()).unwrap());
                }
                for (token, id) in tokens.into_iter().zip(subscriptions.into_iter()) {
                    let notifications = notify_subscribed_tokens(
                        &mut id_generator,
                        &mut subscription_manager,
                        vec![token.clone()],
                    );

                    if !contains_text_query(token) {
                        assert!(notifications.is_empty());
                    } else {
                        assert_eq!(notifications, btreeset! { *id.id() });
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
                subscription_manager.subscribe(token.clone()).unwrap();
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
                subscriptions.push(subscription_manager.subscribe(token.clone()).unwrap());
                }
                for subscription in &subscriptions {
                    subscription_manager._remove(*subscription.id());
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
                    let subscription = subscription_manager.subscribe(token.clone()).unwrap();
                    subscription_manager._remove(*subscription.id());
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
            for doc in &documents {
                subscription_manager.overlapping(doc, &mut to_notify, PersistenceVersion::V5);
            }
        }
        to_notify
    }
}
