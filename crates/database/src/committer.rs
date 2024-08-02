use std::{
    cmp,
    collections::BTreeSet,
    ops::Bound,
    sync::Arc,
};

use ::metrics::{
    StatusTimer,
    Timer,
};
use common::{
    bootstrap_model::tables::{
        TableMetadata,
        TableState,
        TABLES_TABLE,
    },
    document::{
        DocumentUpdate,
        ParsedDocument,
        ResolvedDocument,
    },
    errors::recapture_stacktrace,
    knobs::{
        COMMITTER_QUEUE_SIZE,
        MAX_REPEATABLE_TIMESTAMP_COMMIT_DELAY,
        MAX_REPEATABLE_TIMESTAMP_IDLE_FREQUENCY,
    },
    minitrace_helpers::{
        initialize_root_from_parent,
        EncodedSpan,
    },
    persistence::{
        ConflictStrategy,
        Persistence,
        PersistenceGlobalKey,
        PersistenceReader,
        RepeatablePersistence,
        RetentionValidator,
        TimestampRange,
    },
    runtime::{
        Runtime,
        RuntimeInstant,
        SpawnHandle,
    },
    sync::{
        mpsc::{
            self,
            error::TrySendError,
        },
        split_rw_lock::{
            Reader,
            Writer,
        },
    },
    types::{
        DatabaseIndexUpdate,
        DatabaseIndexValue,
        RepeatableTimestamp,
        Timestamp,
        WriteTimestamp,
    },
    value::ResolvedDocumentId,
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use futures::{
    channel::oneshot,
    future::{
        BoxFuture,
        Either,
    },
    select_biased,
    stream::FuturesOrdered,
    FutureExt,
    StreamExt,
    TryStreamExt,
};
use indexing::index_registry::IndexRegistry;
use minitrace::prelude::*;
use parking_lot::Mutex;
use prometheus::VMHistogram;
use usage_tracking::FunctionUsageTracker;
use value::{
    heap_size::WithHeapSize,
    id_v6::DeveloperDocumentId,
    InternalDocumentId,
    TableMapping,
    TableName,
};
use vector::DocInVectorIndex;

use crate::{
    bootstrap_model::defaults::BootstrapTableIds,
    database::{
        ConflictingReadWithWriteSource,
        ShutdownSignal,
    },
    metrics::{
        self,
        bootstrap_update_timer,
        finish_bootstrap_update,
    },
    reads::ReadSet,
    search_index_bootstrap::{
        stream_revision_pairs_for_indexes,
        BootstrappedSearchIndexes,
    },
    snapshot_manager::SnapshotManager,
    transaction::FinalTransaction,
    write_log::{
        LogWriter,
        PackedDocumentUpdate,
        PendingWriteHandle,
        PendingWrites,
        WriteSource,
    },
    writes::DocumentWrite,
    Transaction,
};

enum PersistenceWrite {
    Commit {
        pending_write: PendingWriteHandle,

        commit_timer: StatusTimer,

        result: oneshot::Sender<anyhow::Result<Timestamp>>,

        parent_trace: EncodedSpan,
    },
    MaxRepeatableTimestamp {
        new_max_repeatable: Timestamp,
        timer: Timer<VMHistogram>,
        result: oneshot::Sender<Timestamp>,
    },
}

pub struct Committer<RT: Runtime> {
    // Internal staged commits for conflict checking.
    pending_writes: PendingWrites,
    // External log of writes for subscriptions.
    log: LogWriter,

    snapshot_manager: Writer<SnapshotManager<RT>>,
    persistence: Arc<dyn Persistence>,
    runtime: RT,

    last_assigned_ts: Timestamp,

    // Allows us to send signal to the app to shutdown.
    shutdown: ShutdownSignal,

    persistence_writes: FuturesOrdered<BoxFuture<'static, anyhow::Result<PersistenceWrite>>>,

    retention_validator: Arc<dyn RetentionValidator>,
}

impl<RT: Runtime> Committer<RT> {
    pub(crate) fn start(
        log: LogWriter,
        snapshot_manager: Writer<SnapshotManager<RT>>,
        persistence: Arc<dyn Persistence>,
        runtime: RT,
        retention_validator: Arc<dyn RetentionValidator>,
        shutdown: ShutdownSignal,
    ) -> CommitterClient<RT> {
        let persistence_reader = persistence.reader();
        let conflict_checker = PendingWrites::new(persistence_reader.version());
        let (tx, rx) = mpsc::channel(*COMMITTER_QUEUE_SIZE);
        let snapshot_reader = snapshot_manager.reader();
        let committer = Self {
            pending_writes: conflict_checker,
            log,
            snapshot_manager,
            persistence,
            runtime: runtime.clone(),
            last_assigned_ts: Timestamp::MIN,
            persistence_writes: FuturesOrdered::new(),
            shutdown,
            retention_validator: retention_validator.clone(),
        };
        let handle = runtime.spawn("committer", committer.go(rx));
        CommitterClient {
            handle: Arc::new(Mutex::new(handle)),
            sender: tx,
            persistence_reader,
            retention_validator,
            snapshot_reader,
        }
    }

    async fn go(mut self, mut rx: mpsc::Receiver<CommitterMessage<RT>>) {
        let mut last_bumped_repeatable_ts = self.runtime.monotonic_now();
        // Assume there were commits just before the backend restarted, so first do a
        // quick bump.
        // None means a bump is ongoing. Avoid parallel bumps in case they
        // commit out of order and regress the repeatable timestamp.
        let mut next_bump_wait = Some(*MAX_REPEATABLE_TIMESTAMP_COMMIT_DELAY);
        loop {
            let bump_fut = if let Some(wait) = &next_bump_wait {
                Either::Left(
                    self.runtime
                        .wait(wait.saturating_sub(last_bumped_repeatable_ts.elapsed())),
                )
            } else {
                Either::Right(std::future::pending())
            };
            select_biased! {
                _ = bump_fut.fuse() => {
                    // Advance the repeatable read timestamp so non-leaders can
                    // establish a recent repeatable snapshot.
                    next_bump_wait = None;
                    let (tx, _rx) = oneshot::channel();
                    self.bump_max_repeatable_ts(tx);
                    last_bumped_repeatable_ts = self.runtime.monotonic_now();
                }
                result = self.persistence_writes.select_next_some() => {
                    let pending_commit = match result {
                        Ok(pending_commit) => pending_commit,
                        Err(err) => {
                            self.shutdown.signal(err.context("Write failed. Unsure if transaction committed to disk."));
                            // Exit the go routine, while we are shutting down.
                            tracing::info!("Shutting down committer");
                            return;
                        },
                    };
                    match pending_commit {
                        PersistenceWrite::Commit {
                            pending_write,
                            commit_timer,
                            result,
                            parent_trace,
                        } => {
                            let root = initialize_root_from_parent("Committer::publish_commit", parent_trace);
                            let _guard = root.set_local_parent();
                            let commit_ts = pending_write.must_commit_ts();
                            self.publish_commit(pending_write);
                            let _ = result.send(Ok(commit_ts));

                            // When we next get free cycles and there is no ongoing bump,
                            // bump max_repeatable_ts so followers can read this commit.
                            if next_bump_wait.is_some() {
                                next_bump_wait = Some(*MAX_REPEATABLE_TIMESTAMP_COMMIT_DELAY);
                            }
                            commit_timer.finish();
                        },
                        PersistenceWrite::MaxRepeatableTimestamp {
                            new_max_repeatable,
                            timer,
                            result,
                        } => {
                            self.publish_max_repeatable_ts(new_max_repeatable);
                            next_bump_wait = Some(*MAX_REPEATABLE_TIMESTAMP_IDLE_FREQUENCY);
                            let _ = result.send(new_max_repeatable);
                            drop(timer);
                        },
                    }
                }
                maybe_message = rx.recv().fuse() => {
                    match maybe_message {
                        None => {
                            tracing::info!("All clients have gone away, shutting down committer...");
                            return;
                        },
                        Some(CommitterMessage::Commit {
                            queue_timer,
                            transaction,
                            result,
                            write_source,
                            parent_trace,
                        }) => {
                            let root = initialize_root_from_parent("handle_commit_message", parent_trace.clone())
                                .with_property(|| ("time_in_queue_ms", format!("{}", queue_timer.elapsed().as_secs_f64() * 1000.0)));
                            let _guard = root.set_local_parent();
                            drop(queue_timer);
                            self.start_commit(transaction, result, write_source, parent_trace);
                        },
                        #[cfg(any(test, feature = "testing"))]
                        Some(CommitterMessage::BumpMaxRepeatableTs { result }) => {
                            self.bump_max_repeatable_ts(result);
                        },
                        Some(CommitterMessage::FinishTextAndVectorBootstrap {
                            bootstrapped_indexes,
                            bootstrap_ts,
                            result,
                        }) => {
                            self.finish_search_and_vector_bootstrap(
                                bootstrapped_indexes,
                                bootstrap_ts,
                                result
                            ).await;
                        },
                        Some(CommitterMessage::LoadIndexesIntoMemory {
                            tables, result
                        }) => {
                            let response = self.load_indexes_into_memory(tables).await;
                            let _ = result.send(response);
                        }
                    }
                },
            }
        }
    }

    async fn update_indexes_since_bootstrap(
        BootstrappedSearchIndexes {
            text_index_manager,
            vector_index_manager,
            tables_with_indexes,
        }: &mut BootstrappedSearchIndexes<RT>,
        bootstrap_ts: Timestamp,
        persistence: RepeatablePersistence,
        registry: &IndexRegistry,
    ) -> anyhow::Result<()> {
        let _timer = bootstrap_update_timer();
        anyhow::ensure!(
            !text_index_manager.is_bootstrapping(),
            "Trying to update search index while it's still bootstrapping"
        );
        anyhow::ensure!(
            !vector_index_manager.is_bootstrapping(),
            "Trying to update vector index while it's still bootstrapping"
        );
        let range = TimestampRange::new((Bound::Excluded(bootstrap_ts), Bound::Unbounded))?;

        let revision_stream =
            stream_revision_pairs_for_indexes(tables_with_indexes, &persistence, range);
        futures::pin_mut!(revision_stream);

        let mut num_revisions = 0;
        let mut total_size = 0;
        while let Some(revision_pair) = revision_stream.try_next().await? {
            num_revisions += 1;
            total_size += revision_pair.document().map(|d| d.size()).unwrap_or(0);
            text_index_manager.update(
                registry,
                revision_pair.prev_document(),
                revision_pair.document(),
                WriteTimestamp::Committed(revision_pair.ts()),
            )?;
            vector_index_manager.update(
                registry,
                revision_pair.prev_document(),
                revision_pair.document(),
                WriteTimestamp::Committed(revision_pair.ts()),
            )?;
        }
        finish_bootstrap_update(num_revisions, total_size);
        Ok(())
    }

    async fn finish_search_and_vector_bootstrap(
        &mut self,
        mut bootstrapped_indexes: BootstrappedSearchIndexes<RT>,
        bootstrap_ts: RepeatableTimestamp,
        result: oneshot::Sender<anyhow::Result<()>>,
    ) {
        let (last_snapshot, latest_ts) = {
            let snapshot_manager = self.snapshot_manager.read();
            (
                snapshot_manager.latest_snapshot(),
                snapshot_manager.latest_ts(),
            )
        };
        if latest_ts > bootstrap_ts {
            let repeatable_persistence = RepeatablePersistence::new(
                self.persistence.reader(),
                latest_ts,
                self.retention_validator.clone(),
            );

            let res = Self::update_indexes_since_bootstrap(
                &mut bootstrapped_indexes,
                *bootstrap_ts,
                repeatable_persistence,
                &last_snapshot.index_registry,
            )
            .await;
            if res.is_err() {
                let _ = result.send(res);
                return;
            }
        }
        // Committer is currently single threaded, so commits should be blocked until we
        // finish and the timestamp shouldn't be able to advance.
        let mut snapshot_manager = self.snapshot_manager.write();
        if latest_ts != snapshot_manager.latest_ts() {
            panic!("Snapshots were changed concurrently during commit?");
        }
        snapshot_manager.overwrite_last_snapshot_text_and_vector_indexes(
            bootstrapped_indexes.text_index_manager,
            bootstrapped_indexes.vector_index_manager,
        );
        tracing::info!("Committed backfilled vector indexes");
        let _ = result.send(Ok(()));
    }

    // This blocks the committer and loads the in-memory indexes for the latest
    // snapshot in memory. A potential further improvement is to pick a base
    // timestamp and load the indexes at that timestamp outside of the committer.
    // The committer can then replay recent writes to derive the latest in-memory
    // indexes. This would either need to do another database query for the log
    // or rely on the write log to not have trimmed the base timestamp yet.
    async fn load_indexes_into_memory(
        &mut self,
        tables: BTreeSet<TableName>,
    ) -> anyhow::Result<()> {
        let (last_snapshot, latest_ts) = {
            let snapshot_manager = self.snapshot_manager.read();
            (
                snapshot_manager.latest_snapshot(),
                snapshot_manager.latest_ts(),
            )
        };

        let repeatable_persistence = RepeatablePersistence::new(
            self.persistence.reader(),
            latest_ts,
            self.retention_validator.clone(),
        );
        let mut in_memory_indexes = last_snapshot.in_memory_indexes.clone();
        in_memory_indexes
            .load_enabled_for_tables(
                &last_snapshot.index_registry,
                last_snapshot.table_mapping(),
                &repeatable_persistence.read_snapshot(latest_ts)?,
                &tables,
            )
            .await?;

        // Committer is currently single threaded, so commits should be blocked until we
        // finish and the timestamp shouldn't be able to advance.
        let mut snapshot_manager = self.snapshot_manager.write();
        if latest_ts != snapshot_manager.latest_ts() {
            panic!("Snapshots were changed concurrently during commit?");
        }
        snapshot_manager.overwrite_last_snapshot_in_memory_indexes(in_memory_indexes);
        tracing::info!("Loaded indexes into memory");
        Ok(())
    }

    fn bump_max_repeatable_ts(&mut self, result: oneshot::Sender<Timestamp>) {
        let timer = metrics::bump_repeatable_ts_timer();
        // next_max_repeatable_ts bumps the last_assigned_ts, so all future commits on
        // this committer will be after new_max_repeatable.
        let new_max_repeatable = self
            .next_max_repeatable_ts()
            .expect("new_max_repeatable should exist");
        let persistence = self.persistence.clone();
        self.persistence_writes.push_back(
            async move {
                // The MaxRepeatableTimestamp persistence global ensures all future
                // commits on future leaders will be after new_max_repeatable, and followers
                // can know this timestamp is repeatable.
                persistence
                    .write_persistence_global(
                        PersistenceGlobalKey::MaxRepeatableTimestamp,
                        new_max_repeatable.into(),
                    )
                    .await?;
                Ok(PersistenceWrite::MaxRepeatableTimestamp {
                    new_max_repeatable,
                    timer,
                    result,
                })
            }
            .boxed(),
        );
    }

    fn publish_max_repeatable_ts(&mut self, new_max_repeatable: Timestamp) {
        // Bump the latest snapshot in snapshot_manager so reads on this leader
        // can know this timestamp is repeatable.
        let mut snapshot_manager = self.snapshot_manager.write();
        let (latest_ts, new_snapshot) = snapshot_manager.latest();
        if new_max_repeatable > *latest_ts {
            snapshot_manager.push(new_max_repeatable, new_snapshot);
            self.log.append(
                new_max_repeatable,
                WithHeapSize::default(),
                "publish_max_repeatable_ts".into(),
            );
        }
    }

    /// First, check that it's valid to apply this transaction in-memory. If it
    /// passes validation, we can rebase the transaction to a new timestamp
    /// if other transactions have committed.
    #[minitrace::trace]
    fn validate_commit(
        &mut self,
        transaction: FinalTransaction,
        write_source: WriteSource,
    ) -> anyhow::Result<ValidatedCommit> {
        let commit_ts = self.next_commit_ts()?;
        let timer = metrics::commit_is_stale_timer();
        if let Some(conflicting_read) = self.commit_has_conflict(
            transaction.reads.read_set(),
            *transaction.begin_timestamp,
            commit_ts,
        )? {
            anyhow::bail!(conflicting_read.into_error(&transaction.table_mapping, &write_source));
        }
        timer.finish();

        let updates: Vec<_> = transaction.writes.into_coalesced_writes().collect();
        // The updates are ordered using table_dependency_sort_key,
        // which is the same order they should be applied to database metadata
        // and index data structures
        let mut ordered_updates = updates;
        ordered_updates.sort_by_key(|(id, update)| {
            table_dependency_sort_key(
                BootstrapTableIds::new(&transaction.table_mapping),
                InternalDocumentId::from(*id),
                update.new_document.as_ref(),
            )
        });

        let (document_writes, index_writes) = self.compute_writes(commit_ts, &ordered_updates)?;

        // Append the updates to pending_writes, so future conflicting commits
        // will fail the `commit_has_conflict` check above, even before
        // this transaction writes to persistence or is visible to reads. Note that
        // this can cause theoretical false conflicts, where transaction has a conflict
        // with another one, and the latter never ended up committing. This
        // should be very rare, and false positives are acceptable by design.
        let timer = metrics::pending_writes_append_timer();
        let pending_write = self.pending_writes.push_back(
            commit_ts,
            ordered_updates
                .into_iter()
                .map(|(id, update)| (id, PackedDocumentUpdate::pack(update)))
                .collect(),
            write_source,
        );
        drop(timer);

        Ok(ValidatedCommit {
            index_writes,
            document_writes,
            pending_write,
        })
    }

    fn compute_writes(
        &self,
        commit_ts: Timestamp,
        ordered_updates: &Vec<(ResolvedDocumentId, DocumentUpdate)>,
    ) -> anyhow::Result<(
        Vec<ValidatedDocumentWrite>,
        BTreeSet<(Timestamp, DatabaseIndexUpdate)>,
    )> {
        let timer = metrics::commit_prepare_writes_timer();
        let mut document_writes = Vec::new();
        let mut index_writes = Vec::new();
        // We have already checked for conflicts, so the current snapshot must have the
        // same tables and indexes as the base snapshot and the final publishing
        // snapshot. Therefore index writes can be computed from the current snapshot.
        let mut current_snapshot = self.snapshot_manager.read().latest_snapshot();
        for (id, document_update) in ordered_updates.iter() {
            let (updates, doc_in_vector_index) =
                current_snapshot.update(document_update, commit_ts)?;
            index_writes.extend(updates);
            document_writes.push(ValidatedDocumentWrite {
                commit_ts,
                id: (*id).into(),
                write: DocumentWrite {
                    document: document_update.new_document.clone(),
                },
                doc_in_vector_index,
            });
        }
        let index_writes = index_writes
            .into_iter()
            .map(|index_update| (commit_ts, index_update))
            .collect();

        timer.finish();
        Ok((document_writes, index_writes))
    }

    fn commit_has_conflict(
        &self,
        reads: &ReadSet,
        reads_ts: Timestamp,
        commit_ts: Timestamp,
    ) -> anyhow::Result<Option<ConflictingReadWithWriteSource>> {
        if let Some(conflicting_read) = self.log.is_stale(reads, reads_ts, commit_ts)? {
            return Ok(Some(conflicting_read));
        }
        if let Some(conflicting_read) = self.pending_writes.is_stale(reads, reads_ts, commit_ts)? {
            return Ok(Some(conflicting_read));
        }
        Ok(None)
    }

    /// Commit the transaction to persistence (without the lock held).
    /// This is the commit point of a transaction. If this succeeds, the
    /// transaction must be published and made visible. If we are unsure whether
    /// the write went through, we crash the process and recover from whatever
    /// has been written to persistence.
    #[minitrace::trace]
    async fn write_to_persistence(
        persistence: Arc<dyn Persistence>,
        index_writes: BTreeSet<(Timestamp, DatabaseIndexUpdate)>,
        document_writes: Vec<ValidatedDocumentWrite>,
    ) -> anyhow::Result<()> {
        let timer = metrics::commit_persistence_write_timer();
        let document_writes = document_writes
            .into_iter()
            .map(|write| (write.commit_ts, write.id, write.write.document))
            .collect();
        persistence
            .write(document_writes, index_writes, ConflictStrategy::Error)
            .await?;

        timer.finish();
        Ok(())
    }

    /// After writing the new rows to persistence, mark the commit as complete
    /// and allow the updated rows to be read by other transactions.
    fn publish_commit(&mut self, pending_write: PendingWriteHandle) {
        let apply_timer = metrics::commit_apply_timer();
        let commit_ts = pending_write.must_commit_ts();

        // Grab the `SnapshotManager` lock first. This is held until the end of this
        // function.
        let mut snapshot_manager = self.snapshot_manager.write();

        // This is the only time the `ConflictLogger` lock is acquired -- always under
        // the `SnapshotManager` write lock.
        // This is important as a reader should never be able to observe state that is
        // inconsistent between the conflict logger and the index snapshot.
        let (ordered_updates, write_source) = match self.pending_writes.pop_first(pending_write) {
            None => panic!("commit at {commit_ts} not pending"),
            Some((ts, document_updates, write_source)) => {
                if ts != commit_ts {
                    panic!("commits out of order {ts} != {commit_ts}");
                }
                (document_updates, write_source)
            },
        };

        let new_snapshot = {
            let timer = metrics::commit_validate_index_write_timer();
            let mut new_snapshot = snapshot_manager.latest_snapshot();

            for (_document_id, document_update) in ordered_updates.iter() {
                new_snapshot
                    .update(&document_update.unpack(), commit_ts)
                    .expect(
                        "Snapshot update was invalid. This update should have already been \
                         computed before commit successfully",
                    );
            }

            timer.finish();
            new_snapshot
        };

        // Write transaction state at the commit ts to the document store.
        let timer = metrics::write_log_append_timer();
        self.log.append(
            commit_ts,
            ordered_updates.into_iter().collect(),
            write_source,
        );
        drop(timer);

        metrics::log_num_keys(new_snapshot.table_summaries.num_user_documents);
        metrics::log_document_store_size(new_snapshot.table_summaries.user_size);

        // Publish the new version of our database metadata and the index.
        snapshot_manager.push(commit_ts, new_snapshot);

        apply_timer.finish();
    }

    #[minitrace::trace]
    fn start_commit(
        &mut self,
        transaction: FinalTransaction,
        result: oneshot::Sender<anyhow::Result<Timestamp>>,
        write_source: WriteSource,
        parent_trace: EncodedSpan,
    ) {
        // Quit early for read-only transactions.
        if transaction.is_readonly() {
            let _ = result.send(Ok(*transaction.begin_timestamp));
            return;
        }
        let commit_timer = metrics::commit_timer();
        metrics::log_write_tx(&transaction);

        let table_mapping = transaction.table_mapping.clone();
        let usage_tracking = transaction.usage_tracker.clone();
        let ValidatedCommit {
            index_writes,
            document_writes,
            pending_write,
        } = match self.validate_commit(transaction, write_source) {
            Ok(v) => v,
            Err(e) => {
                let _ = result.send(Err(e));
                return;
            },
        };

        // necessary because this value is moved
        let parent_trace_copy = parent_trace.clone();
        let persistence = self.persistence.clone();
        self.persistence_writes.push_back(
            async move {
                Self::track_commit(
                    usage_tracking,
                    &index_writes,
                    &document_writes,
                    &table_mapping,
                );
                Self::write_to_persistence(persistence, index_writes, document_writes).await?;
                Ok(PersistenceWrite::Commit {
                    pending_write,
                    commit_timer,
                    result,
                    parent_trace: parent_trace_copy,
                })
            }
            .in_span(initialize_root_from_parent(
                "Committer::persistence_writes_future",
                parent_trace.clone(),
            ))
            .boxed(),
        );
    }

    #[minitrace::trace]
    fn track_commit(
        usage_tracker: FunctionUsageTracker,
        index_writes: &BTreeSet<(Timestamp, DatabaseIndexUpdate)>,
        document_writes: &Vec<ValidatedDocumentWrite>,
        table_mapping: &TableMapping,
    ) {
        for (_, index_write) in index_writes {
            if let DatabaseIndexValue::NonClustered(doc) = index_write.value {
                if let Ok(table_name) = table_mapping.tablet_name(doc.tablet_id) {
                    // Index metadata is never a vector
                    // Database bandwidth for index writes
                    usage_tracker.track_database_ingress_size(
                        table_name.to_string(),
                        index_write.key.size() as u64,
                        // Exclude indexes on system tables or reserved system indexes on user
                        // tables
                        table_name.is_system() || index_write.is_system_index,
                    );
                }
            }
        }
        for validated_write in document_writes {
            let ValidatedDocumentWrite {
                id: document_id,
                write: DocumentWrite { document },
                doc_in_vector_index,
                ..
            } = validated_write;
            if let Some(document) = document {
                let document_write_size = document_id.size() + document.size();
                if let Ok(table_name) = table_mapping.tablet_name(document.id().tablet_id) {
                    // Database bandwidth for document writes
                    if *doc_in_vector_index == DocInVectorIndex::Absent {
                        usage_tracker.track_database_ingress_size(
                            table_name.to_string(),
                            document_write_size as u64,
                            table_name.is_system(),
                        );
                    } else {
                        usage_tracker.track_vector_ingress_size(
                            table_name.to_string(),
                            document_write_size as u64,
                            table_name.is_system(),
                        );
                    }
                }
            }
        }
    }

    fn next_commit_ts(&mut self) -> anyhow::Result<Timestamp> {
        let latest_ts = self.snapshot_manager.read().latest_ts();
        let max = cmp::max(
            latest_ts.succ()?,
            cmp::max(
                self.runtime.generate_timestamp()?,
                self.last_assigned_ts.succ()?,
            ),
        );
        self.last_assigned_ts = max;
        Ok(max)
    }

    fn next_max_repeatable_ts(&mut self) -> anyhow::Result<Timestamp> {
        if let Some(min_pending) = self.pending_writes.min_ts() {
            // If there's a pending write, push max_repeatable_ts to be right
            // before the pending write, so followers can choose recent
            // timestamps but can't read at the timestamp of the pending write.
            anyhow::ensure!(min_pending <= self.last_assigned_ts);
            min_pending.pred()
        } else {
            // If there are no pending writes, bump last_assigned_ts and write
            // to persistence and snapshot manager as a commit would.
            self.next_commit_ts()
        }
    }
}

struct ValidatedDocumentWrite {
    commit_ts: Timestamp,
    id: InternalDocumentId,
    write: DocumentWrite,
    doc_in_vector_index: DocInVectorIndex,
}

pub struct CommitterClient<RT: Runtime> {
    handle: Arc<Mutex<RT::Handle>>,
    sender: mpsc::Sender<CommitterMessage<RT>>,
    persistence_reader: Arc<dyn PersistenceReader>,
    retention_validator: Arc<dyn RetentionValidator>,
    snapshot_reader: Reader<SnapshotManager<RT>>,
}

impl<RT: Runtime> Clone for CommitterClient<RT> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
            sender: self.sender.clone(),
            persistence_reader: self.persistence_reader.clone(),
            retention_validator: self.retention_validator.clone(),
            snapshot_reader: self.snapshot_reader.clone(),
        }
    }
}

impl<RT: Runtime> CommitterClient<RT> {
    pub async fn finish_search_and_vector_bootstrap(
        &self,
        bootstrapped_indexes: BootstrappedSearchIndexes<RT>,
        bootstrap_ts: RepeatableTimestamp,
    ) -> anyhow::Result<()> {
        let (tx, rx) = oneshot::channel();
        let message = CommitterMessage::FinishTextAndVectorBootstrap {
            bootstrapped_indexes,
            bootstrap_ts,
            result: tx,
        };
        self.sender.try_send(message).map_err(|e| match e {
            TrySendError::Full(..) => metrics::committer_full_error().into(),
            TrySendError::Closed(..) => metrics::shutdown_error(),
        })?;
        // The only reason we might fail here if the committer is shutting down.
        rx.await.map_err(|_| metrics::shutdown_error())?
    }

    // Tell the committer to load all indexes for the given tables into memory.
    pub async fn load_indexes_into_memory(
        &self,
        tables: BTreeSet<TableName>,
    ) -> anyhow::Result<()> {
        let (tx, rx) = oneshot::channel();
        let message = CommitterMessage::LoadIndexesIntoMemory { tables, result: tx };
        self.sender.try_send(message).map_err(|e| match e {
            TrySendError::Full(..) => metrics::committer_full_error().into(),
            TrySendError::Closed(..) => metrics::shutdown_error(),
        })?;
        // The only reason we might fail here if the committer is shutting down.
        rx.await.map_err(|_| metrics::shutdown_error())?
    }

    pub fn commit(
        &self,
        transaction: Transaction<RT>,
        write_source: WriteSource,
    ) -> BoxFuture<anyhow::Result<Timestamp>> {
        self._commit(transaction, write_source).boxed()
    }

    #[minitrace::trace]
    async fn _commit(
        &self,
        transaction: Transaction<RT>,
        write_source: WriteSource,
    ) -> anyhow::Result<Timestamp> {
        let _timer = metrics::commit_client_timer();
        self.check_generated_ids(&transaction).await?;

        // Finish reading everything from persistence.
        let transaction = transaction.finalize(self.snapshot_reader.clone()).await?;

        let queue_timer = metrics::commit_queue_timer();
        let (tx, rx) = oneshot::channel();
        let message = CommitterMessage::Commit {
            queue_timer,
            transaction,
            result: tx,
            write_source,
            parent_trace: EncodedSpan::from_parent(),
        };
        self.sender.try_send(message).map_err(|e| match e {
            TrySendError::Full(..) => metrics::committer_full_error().into(),
            TrySendError::Closed(..) => metrics::shutdown_error(),
        })?;
        let Ok(result) = rx.await else {
            anyhow::bail!(metrics::shutdown_error());
        };
        if let Err(e) = result {
            // For OCC and other known commit failure error types,
            // replace the committer's stacktrace with the caller's stack trace as
            // that will be more helpful
            if e.is_occ() {
                return Err(recapture_stacktrace(e));
            }
            return Err(e);
        }
        result
    }

    pub fn shutdown(&self) {
        self.handle.lock().shutdown();
    }

    #[cfg(any(test, feature = "testing"))]
    pub async fn bump_max_repeatable_ts(&self) -> anyhow::Result<Timestamp> {
        let (tx, rx) = oneshot::channel();
        let message = CommitterMessage::BumpMaxRepeatableTs { result: tx };
        self.sender
            .try_send(message)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(rx.await?)
    }

    async fn check_generated_ids(&self, transaction: &Transaction<RT>) -> anyhow::Result<()> {
        // Check that none of the DocumentIds generated in this transaction
        // are already in use.
        // We can check at the begin_timestamp+1 because generated_ids are also
        // checked for conflict against all writes after begin_timestamp.
        let ts = transaction.begin_timestamp().succ()?;
        let timer = metrics::commit_id_reuse_timer();
        let generated_ids = transaction.writes.as_flat()?.generated_ids();
        if !generated_ids.is_empty() {
            let repeatable_persistence = RepeatablePersistence::new(
                self.persistence_reader.clone(),
                transaction.begin_timestamp(),
                self.retention_validator.clone(),
            );
            let generated_ids_with_ts: BTreeSet<_> = generated_ids
                .iter()
                .map(|id| (InternalDocumentId::from(*id), ts))
                .collect();
            let mut previous_revisions_of_ids = repeatable_persistence
                .previous_revisions(generated_ids_with_ts)
                .await?;
            if let Some(((document_id, _), (_, maybe_doc))) = previous_revisions_of_ids.pop_first()
            {
                let display_id = generated_ids
                    .iter()
                    .find(|id| InternalDocumentId::from(**id) == document_id)
                    .map(|id| DeveloperDocumentId::from(*id).encode())
                    .unwrap_or(document_id.to_string());
                if maybe_doc.is_none() {
                    anyhow::bail!(ErrorMetadata::bad_request(
                        "DocumentDeleted",
                        format!(
                            "Cannot recreate document with _id {display_id} that was deleted. Try \
                             to insert it without an _id or insert into another table."
                        ),
                    ));
                } else {
                    anyhow::bail!(ErrorMetadata::bad_request(
                        "DocumentExists",
                        format!(
                            "Cannot create document with _id {display_id} that already exists. \
                             Try to update it with `db.patch` or `db.replace`."
                        ),
                    ));
                }
            }
        }
        timer.finish();
        Ok(())
    }
}

enum CommitterMessage<RT: Runtime> {
    Commit {
        queue_timer: Timer<VMHistogram>,
        transaction: FinalTransaction,
        result: oneshot::Sender<anyhow::Result<Timestamp>>,
        write_source: WriteSource,
        parent_trace: EncodedSpan,
    },
    #[cfg(any(test, feature = "testing"))]
    BumpMaxRepeatableTs { result: oneshot::Sender<Timestamp> },
    LoadIndexesIntoMemory {
        tables: BTreeSet<TableName>,
        result: oneshot::Sender<anyhow::Result<()>>,
    },
    FinishTextAndVectorBootstrap {
        bootstrapped_indexes: BootstrappedSearchIndexes<RT>,
        bootstrap_ts: RepeatableTimestamp,
        result: oneshot::Sender<anyhow::Result<()>>,
    },
}

// Within a single transaction that writes multiple documents, this is the order
// in which we write them.
//
// Dependencies:
// - _tables table created before other tables.
// - table created before its indexes.
// - indexes created before documents in the table.
// - indexes deleted before other indexes created, in case of naming conflicts.
// - tables deleted before other tables created, in case of naming conflicts.
// - indexes on a table deleted before the table itself.
pub fn table_dependency_sort_key(
    bootstrap_tables: BootstrapTableIds,
    id: InternalDocumentId,
    update: Option<&ResolvedDocument>,
) -> (usize, InternalDocumentId) {
    let table = id.table();
    let sort_key = if table == bootstrap_tables.tables_id.tablet_id {
        match update {
            Some(insertion) => {
                let table_metadata: ParsedDocument<TableMetadata> =
                    insertion.clone().try_into().unwrap_or_else(|e| {
                        panic!("Writing invalid TableMetadata {}: {e}", insertion.value().0)
                    });
                match table_metadata.state {
                    TableState::Active => {
                        if &table_metadata.name == &*TABLES_TABLE {
                            // In bootstrapping, create _tables table first.
                            2
                        } else {
                            // Create other tables, especially the _index table, next.
                            3
                        }
                    },
                    TableState::Hidden => 3,
                    // Deleting index must come before table deletion,
                    // so we can delete the table.by_id index while the table still exists.
                    TableState::Deleting => 1,
                }
            },
            // Legacy method of deleting _tables, supported here when walking the log.
            None => 1,
        }
    } else if table == bootstrap_tables.index_id.tablet_id {
        if update.is_none() {
            // Index deletes come first, in case one is being deleted and another
            // created with the same name.
            0
        } else {
            4
        }
    } else {
        5
    };
    (sort_key, id)
}

struct ValidatedCommit {
    index_writes: BTreeSet<(Timestamp, DatabaseIndexUpdate)>,
    document_writes: Vec<ValidatedDocumentWrite>,
    pending_write: PendingWriteHandle,
}
