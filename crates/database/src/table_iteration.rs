use std::{
    cmp,
    collections::{
        BTreeMap,
        BTreeSet,
    },
    sync::Arc,
};

use common::{
    bootstrap_model::index::database_index::IndexedFields,
    document::ResolvedDocument,
    index::IndexKey,
    interval::{
        BinaryKey,
        End,
        Interval,
        Start,
    },
    pause::PauseClient,
    persistence::{
        new_static_repeatable_recent,
        PersistenceReader,
        RepeatablePersistence,
        RetentionValidator,
        TimestampRange,
    },
    query::Order,
    runtime::{
        RateLimiter,
        Runtime,
        RuntimeInstant,
    },
    types::{
        IndexId,
        RepeatableTimestamp,
        Timestamp,
    },
    value::ResolvedDocumentId,
};
use futures::{
    pin_mut,
    stream,
    Stream,
    StreamExt,
    TryStreamExt,
};
use futures_async_stream::try_stream;
use value::{
    InternalDocumentId,
    TableId,
};

/// A cursor for use while scanning a table by ID.
///
/// The key is the last element processed thus far.
#[derive(Clone, Debug)]
pub struct TableScanCursor {
    pub index_key: Option<IndexKey>,
}

impl TableScanCursor {
    pub fn interval(&self) -> Interval {
        match &self.index_key {
            Some(index_key) => {
                let cursor_bytes: BinaryKey = index_key.clone().into_bytes().into();
                Interval {
                    start: Start::Included(
                        // `.increment()` should never fail because:
                        // 1. Document IDs serialize to a nonempty index key
                        // 2. At the very least, the index key starts with ID_TAG < 255
                        cursor_bytes.increment().expect("id should have increment"),
                    ),
                    end: End::Unbounded,
                }
            },
            None => Interval::all(),
        }
    }

    pub fn advance(&mut self, index_key: IndexKey) -> anyhow::Result<()> {
        if let Some(existing_key) = &self.index_key {
            anyhow::ensure!(index_key >= existing_key);
        }
        self.index_key = Some(index_key);
        Ok(())
    }
}

pub struct TableIterator<RT: Runtime> {
    runtime: RT,
    persistence: Box<dyn PersistenceReader>,
    retention_validator: Arc<dyn RetentionValidator>,
    page_size: usize,
    pause_client: PauseClient,
    snapshot_ts: RepeatableTimestamp,
}

impl<RT: Runtime> TableIterator<RT> {
    pub fn new(
        runtime: RT,
        snapshot_ts: RepeatableTimestamp,
        persistence: Box<dyn PersistenceReader>,
        retention_validator: Arc<dyn RetentionValidator>,
        page_size: usize,
        pause_client: Option<PauseClient>,
    ) -> Self {
        let pause_client = pause_client.unwrap_or_default();
        Self {
            runtime,
            persistence,
            retention_validator,
            page_size,
            pause_client,
            snapshot_ts,
        }
    }

    #[try_stream(ok = (ResolvedDocument, Timestamp), error = anyhow::Error)]
    pub async fn stream_documents_in_table(
        self,
        table_id: TableId,
        by_id: IndexId,
        cursor: Option<ResolvedDocumentId>,
        rate_limiter: &RateLimiter<RT>,
    ) {
        let stream = self.stream_documents_in_table_by_index(
            table_id,
            by_id,
            IndexedFields::by_id(),
            cursor.map(|id| IndexKey::new(vec![], id.into())),
            rate_limiter,
        );
        pin_mut!(stream);
        while let Some(item) = stream.try_next().await? {
            yield item;
        }
    }

    /// Algorithm overview:
    /// Walk a table ordered by an index at snapshot_ts which may be outside
    /// retention, so you can't walk the index directly.
    ///
    /// 1. Walk the index at recent, increasing snapshots, and
    /// collect documents that have not changed since snapshot_ts.
    /// 2. Also walk the document log between snapshot_ts and the increasing
    /// snapshots, and for each changed document look at its revision as of
    /// snapshot_ts.
    /// 3. Merge the results.
    ///
    /// Why it works:
    /// Consider a document that exists in the index at snapshot_ts.
    /// Either it has changed since snapshot_ts, in which case (2) will find
    /// it, or it has not, in which case (1) will find it.
    #[try_stream(ok = (ResolvedDocument, Timestamp), error = anyhow::Error)]
    pub async fn stream_documents_in_table_by_index(
        mut self,
        table_id: TableId,
        index_id: IndexId,
        indexed_fields: IndexedFields,
        cursor: Option<IndexKey>,
        rate_limiter: &RateLimiter<RT>,
    ) {
        let mut cursor = TableScanCursor { index_key: cursor };

        // 1. Paginate through the table at increasing timestamps.
        let mut end_ts = self.snapshot_ts;

        // skipped_keys are keys of documents that were modified after
        // snapshot_ts but before the index key was walked over.
        // Such documents must be
        // (a) modified/deleted after snapshot_ts but before new_end_ts.
        // (b) have key > cursor.
        // We insert skipped documents into future pages of the index walk when we get
        // to them.
        let mut skipped_keys = BTreeMap::new();

        loop {
            self.pause_client.wait("before_index_page").await;
            while let Err(not_until) = rate_limiter.check() {
                let delay = not_until.wait_time_from(self.runtime.monotonic_now().as_nanos());
                self.runtime.wait(delay).await;
            }
            let page_start = cursor.index_key.clone();
            let (page, new_end_ts) = self
                .fetch_page(index_id, table_id, &indexed_fields, &mut cursor)
                .await?;
            anyhow::ensure!(*new_end_ts >= end_ts);
            // If page is empty, we have exhausted the cursor.
            let page_end = if page.is_empty() {
                None
            } else {
                cursor.index_key.clone()
            };
            // Filter out rows from the index scan that were modified after
            // snapshot_ts. Such documents will be found when walking the
            // documents log to generate skipped_keys.
            let page: Vec<_> = page
                .into_iter()
                .filter(|(_, ts)| *ts <= *self.snapshot_ts)
                .collect();

            // 2. Find any keys for documents that were skipped by this
            // page or will be skipped by future pages.
            // These documents are returned with index keys and revisions as
            // they existed at snapshot_ts.
            skipped_keys.extend(
                self.fetch_skipped_keys(
                    table_id,
                    &indexed_fields,
                    page_start.as_ref(),
                    *end_ts,
                    new_end_ts,
                    rate_limiter,
                )
                .await?
                .into_iter(),
            );
            if let Some((first_skipped_key, _)) = skipped_keys.iter().next() {
                // Check all skipped ids are after the old cursor,
                // which ensures the yielded output is in index key order.
                anyhow::ensure!(page_start.as_ref() < Some(first_skipped_key));
            }
            end_ts = new_end_ts;
            // Extract the documents from skipped_keys that should be returned in
            // the current page.
            let page_skipped_keys = match page_end.as_ref() {
                Some(cursor_key) => {
                    let mut page_skipped_keys = BTreeMap::new();
                    while let Some(first_skipped_key) = skipped_keys.first_entry()
                        && first_skipped_key.key() <= cursor_key
                    {
                        let (key, value) = first_skipped_key.remove_entry();
                        page_skipped_keys.insert(key, value);
                    }
                    page_skipped_keys
                },
                None => {
                    let page_skipped_keys = skipped_keys;
                    skipped_keys = BTreeMap::new();
                    page_skipped_keys
                },
            };
            // Note we already fetched these documents earlier when calculating
            // skipped_keys, but we would rather not hold them in memory. Since
            // skipped documents are relatively rare, an extra fetch
            // occasionally is worth it for the memory savings.
            let page_skipped_docs: Vec<_> = self
                .load_revisions_at_snapshot_ts(stream::iter(
                    page_skipped_keys.into_values().map(Ok),
                ))
                .try_collect()
                .await?;
            let mut merged_page: Vec<_> = page.into_iter().chain(page_skipped_docs).collect();
            // Re-sort after merging the index walk and skipped keys.
            merged_page
                .sort_by_key(|(doc, _)| doc.index_key(&indexed_fields, self.persistence.version()));

            // Sanity check output.
            let all_ids: BTreeSet<_> = merged_page
                .iter()
                .map(|(doc, _)| doc.id().internal_id())
                .collect();
            anyhow::ensure!(
                all_ids.len() == merged_page.len(),
                "duplicate id in table iterator {merged_page:?}"
            );
            anyhow::ensure!(
                merged_page.iter().all(|(_, ts)| *ts <= *self.snapshot_ts),
                "document after snapshot in table iterator {merged_page:?}"
            );
            anyhow::ensure!(
                merged_page.iter().all(|(doc, _)| {
                    let key = doc.index_key(&indexed_fields, self.persistence.version());
                    page_start.as_ref() < Some(&key)
                        && (page_end
                            .as_ref()
                            .map_or(true, |cursor_key| key <= *cursor_key))
                }),
                "document outside page in table iterator {merged_page:?}"
            );

            for (revision, revision_ts) in merged_page {
                yield (revision, revision_ts);
            }
            if page_end.is_none() {
                // If we are done, all skipped_keys would be put in this final page.
                anyhow::ensure!(skipped_keys.is_empty());
                break;
            }
        }
        self.pause_client.close("before_index_page");
    }

    /// A document may be skipped if:
    /// 1. it is in the correct table
    /// 2. at the snapshot, it had a key higher than what we've walked so far
    /// 3. it was modified after the snapshot but before we walked its key
    /// range.
    async fn fetch_skipped_keys(
        &self,
        table_id: TableId,
        indexed_fields: &IndexedFields,
        lower_bound: Option<&IndexKey>,
        start_ts: Timestamp,
        end_ts: RepeatableTimestamp,
        rate_limiter: &RateLimiter<RT>,
    ) -> anyhow::Result<BTreeMap<IndexKey, InternalDocumentId>> {
        let reader = self.persistence.clone();
        let persistence_version = reader.version();
        let skipped_revs = self.walk_document_log(table_id, start_ts, end_ts, rate_limiter);
        let revisions_at_snapshot = self.load_revisions_at_snapshot_ts(skipped_revs);
        pin_mut!(revisions_at_snapshot);
        let mut skipped_keys = BTreeMap::new();
        while let Some((doc, _)) = revisions_at_snapshot.try_next().await? {
            let index_key = doc.index_key(indexed_fields, persistence_version);
            if lower_bound < Some(&index_key) {
                skipped_keys.insert(index_key, doc.id_with_table_id());
            }
        }
        Ok(skipped_keys)
    }

    #[try_stream(ok = InternalDocumentId, error = anyhow::Error)]
    async fn walk_document_log<'a>(
        &'a self,
        table_id: TableId,
        start_ts: Timestamp,
        end_ts: RepeatableTimestamp,
        rate_limiter: &'a RateLimiter<RT>,
    ) {
        let reader = self.persistence.clone();
        let repeatable_persistence =
            RepeatablePersistence::new(reader, end_ts, self.retention_validator.clone());
        let documents = repeatable_persistence
            .load_documents(TimestampRange::new(start_ts.succ()?..=*end_ts)?, Order::Asc)
            .try_chunks(self.page_size);
        pin_mut!(documents);
        while let Some(chunk) = documents.try_next().await? {
            while let Err(not_until) = rate_limiter.check() {
                let delay = not_until.wait_time_from(self.runtime.monotonic_now().as_nanos());
                self.runtime.wait(delay).await;
            }
            for (_, id, _) in chunk {
                if *id.table() == table_id {
                    yield id;
                }
            }
        }
    }

    /// We have these constraints:
    ///
    /// 1. we need each walk to be >= snapshot_ts
    /// 2. we need each successive walk to be >= the previous walk
    /// 3. we need each walk to be repeatable
    /// 4. we need each walk to be within retention
    ///
    /// We can satisfy these constraints by always walking at max(snapshot_ts,
    /// new_static_repeatable_ts()).
    ///
    /// 1. max(snapshot_ts, anything) >= snapshot_ts
    /// 2. snapshot_ts never changes and new_static_repeatable_ts is weakly
    ///    monotonically increasing
    /// 3. snapshot_ts and new_static_repeatable_ts are both Repeatable, and the
    /// max of Repeatable timestamps is repeatable.
    /// 4. new_static_repeatable_ts is within retention, so max(anything,
    /// new_static_repeatable_ts()) is within retention.
    async fn new_ts(&self) -> anyhow::Result<RepeatableTimestamp> {
        Ok(cmp::max(
            self.snapshot_ts,
            new_static_repeatable_recent(self.persistence.as_ref()).await?,
        ))
    }

    async fn fetch_page(
        &self,
        index_id: IndexId,
        table_id: TableId,
        indexed_fields: &IndexedFields,
        cursor: &mut TableScanCursor,
    ) -> anyhow::Result<(Vec<(ResolvedDocument, Timestamp)>, RepeatableTimestamp)> {
        let ts = self.new_ts().await?;
        let repeatable_persistence = RepeatablePersistence::new(
            self.persistence.clone(),
            ts,
            self.retention_validator.clone(),
        );
        let reader = repeatable_persistence.read_snapshot(ts)?;
        let stream = reader.index_scan(
            index_id,
            table_id,
            &cursor.interval(),
            Order::Asc,
            self.page_size,
        );
        let documents_in_page: Vec<_> = stream
            .take(self.page_size)
            .map(|item| item.map(|(_, ts, document)| (document, ts)))
            .try_collect()
            .await?;
        if let Some((document, _)) = documents_in_page.last() {
            cursor.advance(document.index_key(indexed_fields, self.persistence.version()))?;
        }
        Ok((documents_in_page, ts))
    }

    // Load the revisions of documents visible at `self.snapshot_ts`, skipping
    // documents that don't exist then.
    #[try_stream(ok = (ResolvedDocument, Timestamp), error = anyhow::Error)]
    async fn load_revisions_at_snapshot_ts<'a>(
        &'a self,
        ids: impl Stream<Item = anyhow::Result<InternalDocumentId>> + 'a,
    ) {
        // Find the revision of the documents earlier than `snapshot_ts.succ()`.
        // These are the revisions visible at `snapshot_ts`.
        let ts_succ = self.snapshot_ts.succ()?;
        let repeatable_persistence = RepeatablePersistence::new(
            self.persistence.clone(),
            self.snapshot_ts,
            self.retention_validator.clone(),
        );

        // Note even though `previous_revisions` can paginate internally, we don't want
        // to hold the entire result set in memory, because documents can be large.
        let id_chunks = ids.try_chunks(self.page_size);
        pin_mut!(id_chunks);

        while let Some(chunk) = id_chunks.try_next().await? {
            let ids_to_load = chunk.into_iter().map(|id| (id, ts_succ)).collect();
            let old_revisions = repeatable_persistence
                .previous_revisions(ids_to_load)
                .await?;
            for (_, (revision_ts, revision)) in old_revisions.into_iter() {
                if let Some(revision) = revision {
                    yield (revision, revision_ts);
                }
            }
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
        num::NonZeroU32,
    };

    use common::{
        bootstrap_model::index::{
            database_index::IndexedFields,
            IndexMetadata,
        },
        pause::PauseController,
        runtime::new_rate_limiter,
        types::{
            unchecked_repeatable_ts,
            GenericIndexName,
            IndexName,
        },
        value::{
            ConvexObject,
            FieldName,
            FieldPath,
            TableName,
        },
    };
    use futures::TryStreamExt;
    use governor::Quota;
    use keybroker::Identity;
    use prop::collection::vec as prop_vec;
    use proptest::prelude::*;
    use runtime::testing::{
        TestDriver,
        TestRuntime,
    };
    use value::{
        assert_obj,
        assert_val,
        resolved_object_strategy,
        resolved_value_strategy,
        ExcludeSetsAndMaps,
    };

    use crate::{
        test_helpers::{
            new_test_database,
            DbFixtures,
        },
        IndexModel,
        IndexWorker,
        TestFacingModel,
        UserFacingModel,
    };

    fn small_user_object() -> impl Strategy<Value = ConvexObject> {
        let values =
            resolved_value_strategy(FieldName::user_strategy, 4, 4, 4, ExcludeSetsAndMaps(false));
        resolved_object_strategy(FieldName::user_strategy(), values, 0..4)
    }

    #[derive(Debug, proptest_derive::Arbitrary)]
    enum Update {
        Insert {
            #[proptest(strategy = "small_user_object()")]
            object: ConvexObject,
        },
        Replace {
            index: usize,
            #[proptest(strategy = "small_user_object()")]
            object: ConvexObject,
        },
        Delete {
            index: usize,
        },
    }

    fn racing_updates() -> impl Strategy<Value = Vec<Vec<Update>>> {
        prop_vec(prop_vec(any::<Update>(), 0..4), 0..4)
    }

    fn small_user_objects() -> impl Strategy<Value = Vec<ConvexObject>> {
        prop_vec(small_user_object(), 1..8)
    }

    fn iterator_includes_all_documents_test(table_name: TableName, objects: Vec<ConvexObject>) {
        let mut td = TestDriver::new();
        let runtime = td.rt();
        let test = async {
            let database = new_test_database(runtime.clone()).await;
            let mut expected = BTreeSet::new();
            let mut tx = database.begin(Identity::system()).await?;
            for object in objects {
                let id = TestFacingModel::new(&mut tx)
                    .insert(&table_name, object)
                    .await?;
                expected.insert(id.internal_id());
            }
            let table_mapping = tx.table_mapping().clone();
            let by_id = IndexName::by_id(table_name.clone());
            let by_id_metadata = IndexModel::new(&mut tx)
                .enabled_index_metadata(&by_id)?
                .unwrap();
            database.commit(tx).await?;
            let iterator = database.table_iterator(database.now_ts_for_reads(), 2, None);
            let table_id = table_mapping.id(&table_name)?.table_id;
            let rate_limiter =
                new_rate_limiter(runtime, Quota::per_second(NonZeroU32::new(1000).unwrap()));
            let revision_stream = iterator.stream_documents_in_table(
                table_id,
                by_id_metadata.id().internal_id(),
                None,
                &rate_limiter,
            );
            futures::pin_mut!(revision_stream);
            let mut actual = BTreeSet::new();
            while let Some((revision, _)) = revision_stream.try_next().await? {
                actual.insert(revision.id().internal_id());
            }
            assert_eq!(expected, actual);
            Ok::<_, anyhow::Error>(())
        };

        td.run_until(test).unwrap();
    }

    async fn racing_commits_test(
        runtime: TestRuntime,
        table_name: TableName,
        initial: Vec<ConvexObject>,
        update_batches: Vec<Vec<Update>>,
    ) -> anyhow::Result<()> {
        let database = new_test_database(runtime.clone()).await;
        let mut objects = BTreeMap::new();
        let mut tx = database.begin(Identity::system()).await?;
        for object in initial {
            let inserted_id = TestFacingModel::new(&mut tx)
                .insert(&table_name, object)
                .await?;
            let inserted = tx.get(inserted_id).await?.unwrap();
            objects.insert(inserted_id, inserted.to_developer());
        }
        // We expect the iterator to produce the initial objects.
        let expected = objects.clone();

        let table_mapping = tx.table_mapping().clone();
        let by_id = IndexName::by_id(table_name.clone());
        let by_id_metadata = IndexModel::new(&mut tx)
            .enabled_index_metadata(&by_id)?
            .unwrap();
        database.commit(tx).await?;

        let (mut pause, pause_client) = PauseController::new(["before_index_page"]);
        let snapshot_ts = database.now_ts_for_reads();
        let iterator = database.table_iterator(snapshot_ts, 2, Some(pause_client));
        let rate_limiter =
            new_rate_limiter(runtime, Quota::per_second(NonZeroU32::new(1000).unwrap()));
        let table_id = table_mapping.id(&table_name)?.table_id;
        let revision_stream = iterator.stream_documents_in_table(
            table_id,
            by_id_metadata.id().internal_id(),
            None,
            &rate_limiter,
        );
        let table_name_ = table_name.clone();
        let database_ = database.clone();
        let test_driver = async move {
            for update_batch in update_batches {
                // Run the backfill process until it hits our breakpoint.
                let mut pause_guard = match pause.wait_for_blocked("before_index_page").await {
                    Some(g) => g,
                    // If the worker has finished processing index pages, stop agitating.
                    None => break,
                };

                // Agitate by doing a concurrent update while the worker is blocked.
                for update in update_batch {
                    let mut tx = database_.begin(Identity::system()).await?;
                    match update {
                        Update::Insert { object } => {
                            let inserted_id = TestFacingModel::new(&mut tx)
                                .insert(&table_name_, object)
                                .await?;
                            let inserted = tx.get(inserted_id).await?.unwrap();

                            objects.insert(inserted_id, inserted.to_developer());
                        },
                        Update::Replace { index, object } => {
                            if objects.is_empty() {
                                continue;
                            }
                            let id = *(objects.keys().nth(index % objects.len()).unwrap());
                            let replaced = UserFacingModel::new(&mut tx)
                                .replace(id.into(), object)
                                .await?;
                            objects.insert(id, replaced);
                        },
                        Update::Delete { index } => {
                            if objects.is_empty() {
                                continue;
                            }
                            let id = *(objects.keys().nth(index % objects.len()).unwrap());
                            UserFacingModel::new(&mut tx).delete(id.into()).await?;
                            objects.remove(&id).unwrap();
                        },
                    }
                    database_.commit(tx).await?;
                    // TableIterator walks by_id at pages with timestamp
                    // max(snapshot_ts, max_repeatable_ts), so we bump max_repeatable_ts
                    // to make the commit visible to TableIterator.
                    database_.bump_max_repeatable_ts().await?;
                }
                // Continue the worker.
                pause_guard.unpause();
            }
            anyhow::Ok(())
        };

        let documents_from_iterator = async move {
            futures::pin_mut!(revision_stream);
            let mut actual = BTreeMap::new();
            let mut prev_doc_id = None;
            while let Some((revision, ts)) = revision_stream.try_next().await? {
                assert!(ts <= *snapshot_ts);
                assert!(prev_doc_id.as_ref() < Some(revision.id()));
                prev_doc_id = Some(*revision.id());
                actual.insert(*revision.id(), revision.to_developer());
            }
            Ok(actual)
        };

        let result_future = async move { futures::try_join!(documents_from_iterator, test_driver) };
        let (computed, _) = result_future.await?;

        assert_eq!(expected, computed);

        anyhow::Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_deleted(rt: TestRuntime) -> anyhow::Result<()> {
        racing_commits_test(
            rt,
            "A".parse()?,
            vec![assert_obj!()],
            vec![vec![Update::Delete { index: 0 }]],
        )
        .await
    }

    #[convex_macro::test_runtime]
    async fn test_index_key_change(rt: TestRuntime) -> anyhow::Result<()> {
        let DbFixtures {
            db: database,
            tp: persistence,
            ..
        } = DbFixtures::new(&rt).await?;
        let table_name: TableName = "a".parse()?;

        // Create a.by_k and backfill.
        let index_name = GenericIndexName::new(table_name.clone(), "by_k".parse()?)?;
        let field: FieldPath = "k".parse()?;
        let index_fields = IndexedFields::try_from(vec![field.clone()])?;
        let mut tx = database.begin(Identity::system()).await?;
        IndexModel::new(&mut tx)
            .add_application_index(IndexMetadata::new_enabled(
                index_name.clone(),
                index_fields.clone(),
            ))
            .await?;
        database.commit(tx).await?;
        IndexWorker::new_terminating(
            rt.clone(),
            persistence.clone(),
            database.retention_validator(),
            database.clone(),
        )
        .await?;

        // Two documents, one which changes from "z" to "a".
        // In the first page, we read "a" and skip it.
        // Then we walk the documents log, and put it in skipped_keys as "z".
        // In the second page, we read "m" and output it. Then we output "z"
        // from the skipped keys.

        let mut tx = database.begin(Identity::system()).await?;
        TestFacingModel::new(&mut tx)
            .insert(&table_name, assert_obj!("k" => "m"))
            .await?;
        let id = TestFacingModel::new(&mut tx)
            .insert(&table_name, assert_obj!("k" => "z"))
            .await?;
        let table_mapping = tx.table_mapping().clone();
        let by_k_metadata = IndexModel::new(&mut tx)
            .enabled_index_metadata(&index_name)?
            .unwrap();
        let by_k_id = by_k_metadata.id().internal_id();
        let snapshot_ts = unchecked_repeatable_ts(database.commit(tx).await?);

        let mut tx = database.begin(Identity::system()).await?;
        UserFacingModel::new(&mut tx)
            .replace(id.into(), assert_obj!("k" => "a"))
            .await?;
        database.commit(tx).await?;
        database.bump_max_repeatable_ts().await?;

        let iterator = database.table_iterator(snapshot_ts, 1, None);
        let rate_limiter = new_rate_limiter(
            rt.clone(),
            Quota::per_second(NonZeroU32::new(1000).unwrap()),
        );
        let table_id = table_mapping.id(&table_name)?.table_id;
        let revisions: Vec<_> = iterator
            .stream_documents_in_table_by_index(
                table_id,
                by_k_id,
                index_fields,
                None,
                &rate_limiter,
            )
            .try_collect()
            .await?;
        assert_eq!(revisions.len(), 2);
        let k_values: Vec<_> = revisions
            .iter()
            .map(|(doc, _)| doc.value().get("k").unwrap().clone())
            .collect();
        assert_eq!(k_values, vec![assert_val!("m"), assert_val!("z")]);

        Ok(())
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]


        #[test]
        fn test_iterator_includes_all_documents(
            table_name in TableName::user_strategy(),
            objects in small_user_objects(),
        ) {
            iterator_includes_all_documents_test(table_name, objects);
        }

        #[test]
        fn test_racing_commits(
            table_name in TableName::user_strategy(),
            initial in small_user_objects(),
            update_batches in racing_updates(),
        ) {
            let mut td = TestDriver::new();
            td.run_until(
                racing_commits_test(td.rt(), table_name, initial, update_batches),
            ).unwrap();
        }

    }
}
