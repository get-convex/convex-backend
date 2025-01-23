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
    components::{
        ComponentId,
        ComponentPath,
    },
    document::{
        DocumentUpdateWithPrevTs,
        ParsedDocument,
        ResolvedDocument,
    },
    errors::recapture_stacktrace,
    fastrace_helpers::{
        initialize_root_from_parent,
        EncodedSpan,
    },
    knobs::{
        COMMITTER_QUEUE_SIZE,
        COMMIT_TRACE_THRESHOLD,
        MAX_REPEATABLE_TIMESTAMP_COMMIT_DELAY,
        MAX_REPEATABLE_TIMESTAMP_IDLE_FREQUENCY,
    },
    persistence::{
        ConflictStrategy,
        DocumentLogEntry,
        Persistence,
        PersistenceGlobalKey,
        PersistenceReader,
        RepeatablePersistence,
        RetentionValidator,
        TimestampRange,
    },
    runtime::{
        Runtime,
        SpawnHandle,
    },
    shutdown::ShutdownSignal,
    sync::split_rw_lock::{
        Reader,
        Writer,
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
use fastrace::prelude::*;
use futures::{
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
use parking_lot::Mutex;
use prometheus::VMHistogram;
use tokio::sync::{
    mpsc::{
        self,
        error::TrySendError,
    },
    oneshot,
};
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
    database::ConflictingReadWithWriteSource,
    metrics::{
        self,
        bootstrap_update_timer,
        finish_bootstrap_update,
        next_commit_ts_seconds,
        table_summary_finish_bootstrap_timer,
    },
    reads::ReadSet,
    search_index_bootstrap::{
        stream_revision_pairs_for_indexes,
        BootstrappedSearchIndexes,
    },
    snapshot_manager::SnapshotManager,
    table_summary::{
        self,
    },
    transaction::FinalTransaction,
    write_log::{
        LogWriter,
        PackedDocumentUpdate,
        PendingWriteHandle,
        PendingWrites,
        WriteSource,
    },
    writes::DocumentWrite,
    ComponentRegistry,
    Transaction,
    TransactionReadSet,
};

enum PersistenceWrite {
    Commit {
        pending_write: PendingWriteHandle,
        commit_timer: StatusTimer,
        result: oneshot::Sender<anyhow::Result<Timestamp>>,
        parent_trace: EncodedSpan,
        begin_timestamp: RepeatableTimestamp,
        commit_id: usize,
    },
    MaxRepeatableTimestamp {
        new_max_repeatable: Timestamp,
        timer: Timer<VMHistogram>,
        result: oneshot::Sender<Timestamp>,
        commit_id: usize,
    },
}

impl PersistenceWrite {
    fn commit_id(&self) -> usize {
        match self {
            Self::Commit { commit_id, .. } => *commit_id,
            Self::MaxRepeatableTimestamp { commit_id, .. } => *commit_id,
        }
    }
}

pub struct Committer<RT: Runtime> {
    // Internal staged commits for conflict checking.
    pending_writes: PendingWrites,
    // External log of writes for subscriptions.
    log: LogWriter,

    snapshot_manager: Writer<SnapshotManager>,
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
        snapshot_manager: Writer<SnapshotManager>,
        persistence: Arc<dyn Persistence>,
        runtime: RT,
        retention_validator: Arc<dyn RetentionValidator>,
        shutdown: ShutdownSignal,
    ) -> CommitterClient {
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

    async fn go(mut self, mut rx: mpsc::Receiver<CommitterMessage>) {
        let mut last_bumped_repeatable_ts = self.runtime.monotonic_now();
        // Assume there were commits just before the backend restarted, so first do a
        // quick bump.
        // None means a bump is ongoing. Avoid parallel bumps in case they
        // commit out of order and regress the repeatable timestamp.
        let mut next_bump_wait = Some(*MAX_REPEATABLE_TIMESTAMP_COMMIT_DELAY);

        let mut root_span = None;
        // Keep a monotonically increasing id to keep track of honeycomb traces
        // Each commit_id tracks a single write to persistence, from the time the commit
        // message is received until the time the commit has been published. We skip
        // read-only transactions or commits that fail to validate.
        let mut commit_id = 0;
        // Keep track of the commit_id that is currently being traced.
        let mut span_commit_id = None;
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
                    let root_span = root_span.get_or_insert_with(|| {
                        span_commit_id = Some(commit_id);
                        Span::root("bump_max_repeatable", SpanContext::random())
                    });
                    let _span =  Span::enter_with_parent("queue_bump_max_repeatable", root_span);
                    // Advance the repeatable read timestamp so non-leaders can
                    // establish a recent repeatable snapshot.
                    next_bump_wait = None;
                    let (tx, _rx) = oneshot::channel();
                    self.bump_max_repeatable_ts(tx, commit_id, root_span);
                    commit_id += 1;
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
                    let pending_commit_id = pending_commit.commit_id();
                    match pending_commit {
                        PersistenceWrite::Commit {
                            pending_write,
                            commit_timer,
                            result,
                            parent_trace,
                            begin_timestamp,
                            ..
                        } => {
                            let _span = root_span.as_ref().map(|root| Span::enter_with_parent("publish_commit", root));
                            let root = initialize_root_from_parent("Committer::publish_commit", parent_trace);
                            let _guard = root.set_local_parent();
                            let commit_ts = pending_write.must_commit_ts();
                            self.publish_commit(pending_write, begin_timestamp);
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
                            ..
                        } => {
                            let _span = root_span.as_ref().map(|root| Span::enter_with_parent("publish_max_repeatable_ts", root));
                            self.publish_max_repeatable_ts(new_max_repeatable);
                            next_bump_wait = Some(*MAX_REPEATABLE_TIMESTAMP_IDLE_FREQUENCY);
                            let _ = result.send(new_max_repeatable);
                            drop(timer);
                        },
                    }
                    // Report the trace if it is longer than the threshold
                    if let Some(id) = span_commit_id && id == pending_commit_id {
                        if let Some(mut span) = root_span.take() {
                            if span.elapsed() < Some(*COMMIT_TRACE_THRESHOLD) {
                                tracing::debug!("Not sending span to honeycomb because it is below the threshold");
                                span.cancel();
                            } else {
                                tracing::debug!("Sending trace to honeycomb");
                            }
                        }
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
                            let root_span_ref = root_span.get_or_insert_with(|| {
                                span_commit_id = Some(commit_id);
                                Span::root("commit", SpanContext::random())
                            });
                            let _span =
                                Span::enter_with_parent("start_commit", root_span_ref);
                            let root = initialize_root_from_parent("handle_commit_message", parent_trace.clone())
                                .with_property(|| ("time_in_queue_ms", format!("{}", queue_timer.elapsed().as_secs_f64() * 1000.0)));
                            let _guard = root.set_local_parent();
                            drop(queue_timer);
                            if let Some(persistence_write_future) = self.start_commit(transaction,
                                result,
                                write_source,
                                parent_trace,
                                commit_id,
                                root_span_ref) {
                                    self.persistence_writes.push_back(persistence_write_future);
                                    commit_id += 1;
                            } else if span_commit_id == Some(commit_id) {
                                // If the span_commit_id is the same as the commit_id, that means we created a root span in this block
                                // and it didn't get incremented, so it's not a write to persistence and we should not trace it.
                                // We also need to reset the span_commit_id and root_span.
                                root_span_ref.cancel();
                                root_span = None;
                                span_commit_id = None;
                            }
                        },
                        #[cfg(any(test, feature = "testing"))]
                        Some(CommitterMessage::BumpMaxRepeatableTs { result }) => {
                            let span = Span::noop();
                            self.bump_max_repeatable_ts(result, commit_id, &span);
                            commit_id += 1;
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
                        Some(CommitterMessage::FinishTableSummaryBootstrap {
                            result,
                        }) => {
                            self.finish_table_summary_bootstrap(result).await;
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
        }: &mut BootstrappedSearchIndexes,
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
        mut bootstrapped_indexes: BootstrappedSearchIndexes,
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

    async fn finish_table_summary_bootstrap(
        &mut self,
        result: oneshot::Sender<anyhow::Result<()>>,
    ) {
        let _timer = table_summary_finish_bootstrap_timer();
        let latest_ts = {
            let snapshot_manager = self.snapshot_manager.read();
            snapshot_manager.latest_ts()
        };
        // This gets called by the TableSummaryWorker when it has successfully
        // checkpointed a TableSummarySnapshot.
        // Walk any changes since the last checkpoint, and update the snapshot manager
        // with the new TableSummarySnapshot.
        let bootstrap_result = table_summary::bootstrap(
            self.runtime.clone(),
            self.persistence.reader(),
            self.retention_validator.clone(),
            latest_ts,
            table_summary::BootstrapKind::FromCheckpoint,
        )
        .await;
        let (table_summary_snapshot, _) = match bootstrap_result {
            Ok(res) => res,
            Err(err) => {
                let _ = result.send(Err(err));
                return;
            },
        };
        // Committer is currently single threaded, so commits should be blocked until we
        // finish and the timestamp shouldn't be able to advance.
        let mut snapshot_manager = self.snapshot_manager.write();
        if latest_ts != snapshot_manager.latest_ts() {
            panic!("Snapshots were changed concurrently during commit?");
        }
        snapshot_manager.overwrite_last_snapshot_table_summary(table_summary_snapshot);
        tracing::info!("Bootstrapped table summaries at ts {}", latest_ts);
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

    fn bump_max_repeatable_ts(
        &mut self,
        result: oneshot::Sender<Timestamp>,
        commit_id: usize,
        root_span: &Span,
    ) {
        let timer = metrics::bump_repeatable_ts_timer();
        // next_max_repeatable_ts bumps the last_assigned_ts, so all future commits on
        // this committer will be after new_max_repeatable.
        let new_max_repeatable = self
            .next_max_repeatable_ts()
            .expect("new_max_repeatable should exist");
        let persistence = self.persistence.clone();
        let outer_span = Span::enter_with_parent("outer_bump_max_repeatable_ts", root_span);
        self.persistence_writes.push_back(
            async move {
                let _span = Span::enter_with_parent("inner_bump_max_repeatable_ts", &outer_span);
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
                    commit_id,
                })
            }
            .boxed(),
        );
    }

    fn publish_max_repeatable_ts(&mut self, new_max_repeatable: Timestamp) {
        // Bump the latest snapshot in snapshot_manager so reads on this leader
        // can know this timestamp is repeatable.
        let mut snapshot_manager = self.snapshot_manager.write();
        if snapshot_manager.bump_persisted_max_repeatable_ts(new_max_repeatable) {
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
    #[fastrace::trace]
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
                .map(|(id, update)| (id, PackedDocumentUpdate::pack(update.into())))
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
        ordered_updates: &Vec<(ResolvedDocumentId, DocumentUpdateWithPrevTs)>,
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
                prev_ts: document_update
                    .old_document
                    .as_ref()
                    .and_then(|&(_, ts)| ts),
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
    #[fastrace::trace]
    async fn write_to_persistence(
        persistence: Arc<dyn Persistence>,
        index_writes: BTreeSet<(Timestamp, DatabaseIndexUpdate)>,
        document_writes: Vec<ValidatedDocumentWrite>,
    ) -> anyhow::Result<()> {
        let timer = metrics::commit_persistence_write_timer();
        let document_writes = document_writes
            .into_iter()
            .map(|write| DocumentLogEntry {
                ts: write.commit_ts,
                id: write.id,
                value: write.write.document,
                prev_ts: write.prev_ts,
            })
            .collect();
        persistence
            .write(document_writes, index_writes, ConflictStrategy::Error)
            .await?;

        timer.finish();
        Ok(())
    }

    /// After writing the new rows to persistence, mark the commit as complete
    /// and allow the updated rows to be read by other transactions.
    fn publish_commit(
        &mut self,
        pending_write: PendingWriteHandle,
        begin_timestamp: RepeatableTimestamp,
    ) {
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
            let (snapshot_ts, mut new_snapshot) = snapshot_manager.latest();

            for (document_id, document_update) in ordered_updates.iter() {
                new_snapshot
                    .update(&document_update.unpack(), commit_ts)
                    .unwrap_or_else(|_| {
                        panic!(
                            "Snapshot update was invalid. This update should have already been \
                             computed before commit successfully. begin_ts = {begin_timestamp}. \
                             snapshot_ts = {snapshot_ts}. commit_ts = {commit_ts}. document_id = \
                             {document_id}."
                        )
                    });
            }

            timer.finish();
            new_snapshot
        };

        // Write transaction state at the commit ts to the document store.
        metrics::commit_rows(ordered_updates.len() as u64);
        let timer = metrics::write_log_append_timer();
        self.log.append(
            commit_ts,
            ordered_updates.into_iter().collect(),
            write_source,
        );
        drop(timer);

        if let Some(table_summaries) = new_snapshot.table_summaries.as_ref() {
            metrics::log_num_keys(table_summaries.num_user_documents);
            metrics::log_document_store_size(table_summaries.user_size);
        }

        // Publish the new version of our database metadata and the index.
        snapshot_manager.push(commit_ts, new_snapshot);

        apply_timer.finish();
    }

    #[fastrace::trace]
    /// Returns a future to add to the pending_writes queue, if the commit
    /// should be written.
    fn start_commit(
        &mut self,
        transaction: FinalTransaction,
        result: oneshot::Sender<anyhow::Result<Timestamp>>,
        write_source: WriteSource,
        parent_trace: EncodedSpan,
        commit_id: usize,
        root_span: &Span,
    ) -> Option<BoxFuture<'static, anyhow::Result<PersistenceWrite>>> {
        // Skip read-only transactions.
        if transaction.is_readonly() {
            let _ = result.send(Ok(*transaction.begin_timestamp));
            return None;
        }
        let commit_timer = metrics::commit_timer();
        metrics::log_write_tx(&transaction);

        let table_mapping = transaction.table_mapping.clone();
        let component_registry = transaction.component_registry.clone();
        let usage_tracking = transaction.usage_tracker.clone();
        let begin_timestamp = transaction.begin_timestamp;
        let ValidatedCommit {
            index_writes,
            document_writes,
            pending_write,
        } = match self.validate_commit(transaction, write_source) {
            Ok(v) => v,
            Err(e) => {
                let _ = result.send(Err(e));
                return None;
            },
        };

        // necessary because this value is moved
        let parent_trace_copy = parent_trace.clone();
        let persistence = self.persistence.clone();
        let request_span = initialize_root_from_parent(
            "Committer::persistence_writes_future",
            parent_trace.clone(),
        );
        let outer_span = Span::enter_with_parents("outer_write_commit", [root_span, &request_span]);
        Some(
            async move {
                Self::track_commit(
                    usage_tracking,
                    &index_writes,
                    &document_writes,
                    &table_mapping,
                    &component_registry,
                );
                Self::write_to_persistence(persistence, index_writes, document_writes).await?;
                Ok(PersistenceWrite::Commit {
                    pending_write,
                    commit_timer,
                    result,
                    parent_trace: parent_trace_copy,
                    begin_timestamp,
                    commit_id,
                })
            }
            .in_span(outer_span)
            .boxed(),
        )
    }

    #[fastrace::trace]
    fn track_commit(
        usage_tracker: FunctionUsageTracker,
        index_writes: &BTreeSet<(Timestamp, DatabaseIndexUpdate)>,
        document_writes: &Vec<ValidatedDocumentWrite>,
        table_mapping: &TableMapping,
        component_registry: &ComponentRegistry,
    ) {
        for (_, index_write) in index_writes {
            if let DatabaseIndexValue::NonClustered(doc) = index_write.value {
                let tablet_id = doc.tablet_id;
                let Ok(table_namespace) = table_mapping.tablet_namespace(tablet_id) else {
                    continue;
                };
                let component_id = ComponentId::from(table_namespace);
                let component_path = component_registry
                    .get_component_path(component_id, &mut TransactionReadSet::new())
                    // It's possible that the component gets deleted in this transaction. In that case, miscount the usage as root.
                    .unwrap_or(ComponentPath::root());
                if let Ok(table_name) = table_mapping.tablet_name(tablet_id) {
                    // Index metadata is never a vector
                    // Database bandwidth for index writes
                    usage_tracker.track_database_ingress_size(
                        component_path,
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
                let tablet_id = document.id().tablet_id;
                let Ok(table_namespace) = table_mapping.tablet_namespace(tablet_id) else {
                    continue;
                };
                let component_id = ComponentId::from(table_namespace);
                let component_path = component_registry
                    .get_component_path(component_id, &mut TransactionReadSet::new())
                    // It's possible that the component gets deleted in this transaction. In that case, miscount the usage as root.
                    .unwrap_or(ComponentPath::root());
                if let Ok(table_name) = table_mapping.tablet_name(tablet_id) {
                    // Database bandwidth for document writes
                    if *doc_in_vector_index == DocInVectorIndex::Absent {
                        usage_tracker.track_database_ingress_size(
                            component_path,
                            table_name.to_string(),
                            document_write_size as u64,
                            table_name.is_system(),
                        );
                    } else {
                        usage_tracker.track_vector_ingress_size(
                            component_path,
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
        let _timer = next_commit_ts_seconds();
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
    prev_ts: Option<Timestamp>,
}

#[derive(Clone)]
pub struct CommitterClient {
    handle: Arc<Mutex<Box<dyn SpawnHandle>>>,
    sender: mpsc::Sender<CommitterMessage>,
    persistence_reader: Arc<dyn PersistenceReader>,
    retention_validator: Arc<dyn RetentionValidator>,
    snapshot_reader: Reader<SnapshotManager>,
}

impl CommitterClient {
    pub async fn finish_search_and_vector_bootstrap(
        &self,
        bootstrapped_indexes: BootstrappedSearchIndexes,
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

    pub async fn finish_table_summary_bootstrap(&self) -> anyhow::Result<()> {
        let (tx, rx) = oneshot::channel();
        let message = CommitterMessage::FinishTableSummaryBootstrap { result: tx };
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

    pub fn commit<RT: Runtime>(
        &self,
        transaction: Transaction<RT>,
        write_source: WriteSource,
    ) -> BoxFuture<anyhow::Result<Timestamp>> {
        self._commit(transaction, write_source).boxed()
    }

    #[fastrace::trace]
    async fn _commit<RT: Runtime>(
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

    async fn check_generated_ids<RT: Runtime>(
        &self,
        transaction: &Transaction<RT>,
    ) -> anyhow::Result<()> {
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
            if let Some((
                (document_id, _),
                DocumentLogEntry {
                    value: maybe_doc, ..
                },
            )) = previous_revisions_of_ids.pop_first()
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

enum CommitterMessage {
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
        bootstrapped_indexes: BootstrappedSearchIndexes,
        bootstrap_ts: RepeatableTimestamp,
        result: oneshot::Sender<anyhow::Result<()>>,
    },
    FinishTableSummaryBootstrap {
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
