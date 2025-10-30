//! Retention deletes old versions of data that can no longer be accessed.
use std::{
    cmp::{
        self,
    },
    collections::{
        hash_map::DefaultHasher,
        BTreeMap,
    },
    hash::{
        Hash,
        Hasher,
    },
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};

use anyhow::Context;
use async_trait::async_trait;
use common::{
    backoff::Backoff,
    bootstrap_model::index::{
        database_index::{
            DatabaseIndexState,
            IndexedFields,
        },
        IndexConfig,
        IndexMetadata,
    },
    document::{
        ParseDocument,
        ParsedDocument,
        ResolvedDocument,
    },
    errors::{
        lease_lost_error,
        report_error,
        LeaseLostError,
    },
    fastrace_helpers::get_sampled_span,
    index::{
        IndexEntry,
        SplitKey,
    },
    interval::Interval,
    knobs::{
        DEFAULT_DOCUMENTS_PAGE_SIZE,
        DOCUMENT_RETENTION_BATCH_INTERVAL_SECONDS,
        DOCUMENT_RETENTION_DELAY,
        DOCUMENT_RETENTION_DELETE_CHUNK,
        DOCUMENT_RETENTION_DELETE_PARALLEL,
        DOCUMENT_RETENTION_MAX_SCANNED_DOCUMENTS,
        INDEX_RETENTION_DELAY,
        INDEX_RETENTION_DELETE_CHUNK,
        INDEX_RETENTION_DELETE_PARALLEL,
        MAX_RETENTION_DELAY_SECONDS,
        RETENTION_DELETES_ENABLED,
        RETENTION_DELETE_BATCH,
        RETENTION_DOCUMENT_DELETES_ENABLED,
        RETENTION_FAIL_ALL_MULTIPLIER,
        RETENTION_FAIL_ENABLED,
        RETENTION_FAIL_START_MULTIPLIER,
    },
    persistence::{
        new_static_repeatable_recent,
        DocumentLogEntry,
        NoopRetentionValidator,
        Persistence,
        PersistenceGlobalKey,
        PersistenceReader,
        RepeatablePersistence,
        RetentionValidator,
        TimestampRange,
    },
    persistence_helpers::{
        DocumentRevision,
        RevisionPair,
    },
    query::Order,
    runtime::{
        shutdown_and_join,
        RateLimiter,
        Runtime,
        SpawnHandle,
    },
    sha256::Sha256,
    shutdown::ShutdownSignal,
    sync::split_rw_lock::{
        new_split_rw_lock,
        Reader,
        Writer,
    },
    try_chunks::TryChunksExt,
    types::{
        GenericIndexName,
        IndexId,
        PersistenceVersion,
        RepeatableTimestamp,
        Timestamp,
    },
    value::{
        ConvexValue,
        TabletId,
    },
};
use errors::ErrorMetadata;
use fastrace::future::FutureExt as _;
use futures::{
    future::try_join_all,
    pin_mut,
    TryStreamExt,
};
use futures_async_stream::try_stream;
use governor::{
    InsufficientCapacity,
    Jitter,
};
use parking_lot::Mutex;
use rand::Rng;
use tokio::{
    sync::watch::{
        self,
        Receiver,
        Sender,
    },
    time::MissedTickBehavior,
};
use value::InternalDocumentId;

use crate::{
    metrics::{
        latest_min_document_snapshot_timer,
        latest_min_snapshot_timer,
        log_document_retention_cursor_age,
        log_document_retention_cursor_lag,
        log_document_retention_no_cursor,
        log_document_retention_scanned_document,
        log_retention_cursor_age,
        log_retention_cursor_lag,
        log_retention_documents_deleted,
        log_retention_expired_index_entry,
        log_retention_index_entries_deleted,
        log_retention_no_cursor,
        log_retention_scanned_document,
        log_retention_ts_advanced,
        log_snapshot_verification_age,
        retention_advance_timestamp_timer,
        retention_delete_chunk_timer,
        retention_delete_document_chunk_timer,
        retention_delete_documents_timer,
        retention_delete_timer,
    },
    snapshot_manager::SnapshotManager,
    BootstrapMetadata,
};

#[derive(Debug, Clone, Copy)]
pub enum RetentionType {
    Document,
    Index,
}

#[derive(Clone)]
pub struct SnapshotBounds {
    /// min_snapshot_ts is the earliest snapshot at which we are guaranteed
    /// to not have deleted data.
    min_index_snapshot_ts: RepeatableTimestamp,

    /// min_document_snapshot_ts is the earliest snapshot at which we are
    /// guaranteed to not have deleted views of data in the write-ahead log.
    min_document_snapshot_ts: RepeatableTimestamp,
}

impl SnapshotBounds {
    fn advance_min_snapshot_ts(&mut self, candidate: RepeatableTimestamp) {
        self.min_index_snapshot_ts = cmp::max(self.min_index_snapshot_ts, candidate);
    }

    fn advance_min_document_snapshot_ts(&mut self, candidate: RepeatableTimestamp) {
        self.min_document_snapshot_ts = cmp::max(self.min_document_snapshot_ts, candidate);
    }
}

pub struct Checkpoint {
    checkpoint: Option<RepeatableTimestamp>,
}

impl Checkpoint {
    fn advance_checkpoint(&mut self, candidate: RepeatableTimestamp) {
        match self.checkpoint {
            Some(ref mut checkpoint) => {
                *checkpoint = cmp::max(*checkpoint, candidate);
            },
            None => {
                self.checkpoint = Some(candidate);
            },
        }
    }
}

pub struct LeaderRetentionManager<RT: Runtime> {
    rt: RT,
    bounds_reader: Reader<SnapshotBounds>,
    checkpoint_reader: Reader<Checkpoint>,
    document_checkpoint_reader: Reader<Checkpoint>,
    handles: Arc<Mutex<Vec<Box<dyn SpawnHandle>>>>,
}

impl<RT: Runtime> Clone for LeaderRetentionManager<RT> {
    fn clone(&self) -> Self {
        Self {
            rt: self.rt.clone(),
            bounds_reader: self.bounds_reader.clone(),
            checkpoint_reader: self.checkpoint_reader.clone(),
            document_checkpoint_reader: self.document_checkpoint_reader.clone(),
            handles: self.handles.clone(),
        }
    }
}

pub async fn latest_retention_min_snapshot_ts(
    persistence: &dyn PersistenceReader,
    retention_type: RetentionType,
) -> anyhow::Result<Timestamp> {
    let _timer = match retention_type {
        RetentionType::Document => latest_min_document_snapshot_timer(),
        RetentionType::Index => latest_min_snapshot_timer(),
    };
    let key = match retention_type {
        RetentionType::Document => PersistenceGlobalKey::DocumentRetentionMinSnapshotTimestamp,
        RetentionType::Index => PersistenceGlobalKey::RetentionMinSnapshotTimestamp,
    };
    let min_snapshot_value = persistence
        .get_persistence_global(key)
        .await?
        .map(ConvexValue::try_from)
        .transpose()?;
    let min_snapshot_ts = match min_snapshot_value {
        Some(ConvexValue::Int64(ts)) => Timestamp::try_from(ts)?,
        None => Timestamp::MIN,
        _ => anyhow::bail!("invalid retention snapshot {min_snapshot_value:?}"),
    };
    Ok(min_snapshot_ts)
}

const INITIAL_BACKOFF: Duration = Duration::from_millis(50);

impl<RT: Runtime> LeaderRetentionManager<RT> {
    pub async fn new(
        rt: RT,
        persistence: Arc<dyn Persistence>,
        bootstrap_metadata: BootstrapMetadata,
        snapshot_reader: Reader<SnapshotManager>,
        follower_retention_manager: FollowerRetentionManager<RT>,
        lease_lost_shutdown: ShutdownSignal,
        retention_rate_limiter: Arc<RateLimiter<RT>>,
    ) -> anyhow::Result<LeaderRetentionManager<RT>> {
        let reader = persistence.reader();
        let latest_ts = snapshot_reader.lock().latest_ts();
        let min_index_snapshot_ts = latest_ts.prior_ts(
            latest_retention_min_snapshot_ts(reader.as_ref(), RetentionType::Index).await?,
        )?;
        let min_document_snapshot_ts = latest_ts.prior_ts(
            latest_retention_min_snapshot_ts(reader.as_ref(), RetentionType::Document).await?,
        )?;
        let bounds = SnapshotBounds {
            min_index_snapshot_ts,
            min_document_snapshot_ts,
        };
        let (bounds_reader, bounds_writer) = new_split_rw_lock(bounds);
        let checkpoint = Checkpoint { checkpoint: None };
        let document_checkpoint = Checkpoint { checkpoint: None };
        let (checkpoint_reader, checkpoint_writer) = new_split_rw_lock(checkpoint);
        let (document_checkpoint_reader, document_checkpoint_writer) =
            new_split_rw_lock(document_checkpoint);

        let index_table_id = bootstrap_metadata.index_tablet_id;
        let follower_retention_manager = Arc::new(follower_retention_manager);
        // We need to delete from all indexes that might be queried.
        // Therefore we scan _index.by_id at min_index_snapshot_ts before
        // min_index_snapshot_ts starts moving, and update the map before
        // confirming any deletes.
        let mut all_indexes = {
            let mut meta_index_scan = reader.index_scan(
                bootstrap_metadata.index_by_id,
                bootstrap_metadata.index_tablet_id,
                *min_index_snapshot_ts,
                &Interval::all(),
                Order::Asc,
                usize::MAX,
                follower_retention_manager.clone(),
            );
            let mut indexes = BTreeMap::new();
            while let Some((_, rev)) = meta_index_scan.try_next().await? {
                Self::accumulate_index_document(rev.value, &mut indexes)?;
            }
            indexes
        };
        let mut index_cursor = min_index_snapshot_ts;
        // Also update the set of indexes up to the current timestamp before document
        // retention starts moving.
        Self::accumulate_indexes(
            persistence.as_ref(),
            &mut all_indexes,
            &mut index_cursor,
            latest_ts,
            index_table_id,
            follower_retention_manager.clone(),
        )
        .await?;

        let (send_min_snapshot, receive_min_snapshot) = watch::channel(min_index_snapshot_ts);
        let (send_min_document_snapshot, receive_min_document_snapshot) =
            watch::channel(min_document_snapshot_ts);
        let advance_min_snapshot_handle = rt.spawn(
            "retention_advance_min_snapshot",
            Self::go_advance_min_snapshot(
                bounds_writer,
                checkpoint_reader.clone(),
                rt.clone(),
                persistence.clone(),
                send_min_snapshot,
                send_min_document_snapshot,
                snapshot_reader.clone(),
                lease_lost_shutdown.clone(),
            ),
        );
        let deletion_handle = rt.spawn(
            "retention_delete",
            Self::go_delete_indexes(
                bounds_reader.clone(),
                rt.clone(),
                persistence.clone(),
                all_indexes,
                index_table_id,
                index_cursor,
                follower_retention_manager.clone(),
                receive_min_snapshot,
                checkpoint_writer,
                snapshot_reader.clone(),
            ),
        );
        let document_deletion_handle = rt.spawn(
            "document_retention_delete",
            Self::go_delete_documents(
                bounds_reader.clone(),
                rt.clone(),
                persistence.clone(),
                receive_min_document_snapshot,
                document_checkpoint_writer,
                snapshot_reader.clone(),
                retention_rate_limiter.clone(),
            ),
        );
        Ok(Self {
            rt,
            bounds_reader,
            checkpoint_reader,
            document_checkpoint_reader,
            handles: Arc::new(Mutex::new(vec![
                // Order matters because we need to shutdown the threads that have
                // receivers before the senders
                deletion_handle,
                document_deletion_handle,
                advance_min_snapshot_handle,
            ])),
        })
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        let handles: Vec<_> = self.handles.lock().drain(..).collect();
        for handle in handles.into_iter() {
            shutdown_and_join(handle).await?;
        }
        Ok(())
    }

    /// Returns the timestamp which we would like to use as min_snapshot_ts.
    /// This timestamp is created relative to the `max_repeatable_ts`.
    async fn candidate_min_snapshot_ts(
        snapshot_reader: &Reader<SnapshotManager>,
        checkpoint_reader: &Reader<Checkpoint>,
        retention_type: RetentionType,
    ) -> anyhow::Result<RepeatableTimestamp> {
        let delay = match retention_type {
            RetentionType::Document => *DOCUMENT_RETENTION_DELAY,
            RetentionType::Index => *INDEX_RETENTION_DELAY,
        };
        let mut candidate = snapshot_reader
            .lock()
            .persisted_max_repeatable_ts()
            .sub(delay)
            .context("Cannot calculate retention timestamp")?;

        if matches!(retention_type, RetentionType::Document) {
            // Ensures the invariant that the index retention confirmed deleted timestamp
            // is always greater than the minimum document snapshot timestamp. It is
            // important that we do this because it prevents us from deleting
            // documents before their indexes are deleted + ensures that the
            // index retention deleter is always reading from a valid snapshot.
            let index_confirmed_deleted = match checkpoint_reader.lock().checkpoint {
                Some(val) => val,
                None => RepeatableTimestamp::MIN,
            };
            candidate = cmp::min(candidate, index_confirmed_deleted);
        }

        Ok(candidate)
    }

    async fn advance_timestamp(
        bounds_writer: &mut Writer<SnapshotBounds>,
        persistence: &dyn Persistence,
        snapshot_reader: &Reader<SnapshotManager>,
        checkpoint_reader: &Reader<Checkpoint>,
        retention_type: RetentionType,
        lease_lost_shutdown: ShutdownSignal,
    ) -> anyhow::Result<Option<RepeatableTimestamp>> {
        let candidate =
            Self::candidate_min_snapshot_ts(snapshot_reader, checkpoint_reader, retention_type)
                .await?;
        let min_snapshot_ts = match retention_type {
            RetentionType::Document => bounds_writer.read().min_document_snapshot_ts,
            RetentionType::Index => bounds_writer.read().min_index_snapshot_ts,
        };
        // Skip advancing the timestamp if the `max_repeatable_ts` hasn't increased
        if candidate <= min_snapshot_ts {
            return Ok(None);
        }
        let new_min_snapshot_ts = candidate;
        let persistence_key = match retention_type {
            RetentionType::Document => PersistenceGlobalKey::DocumentRetentionMinSnapshotTimestamp,
            RetentionType::Index => PersistenceGlobalKey::RetentionMinSnapshotTimestamp,
        };
        // It's very important that we write to persistence before writing to memory,
        // because reads (follower reads and leader on restart) use persistence, while
        // the actual deletions use memory. With the invariant that persistence >=
        // memory, we will never read something that has been deleted.
        if let Err(e) = persistence
            .write_persistence_global(
                persistence_key,
                ConvexValue::from(i64::from(*new_min_snapshot_ts)).into(),
            )
            .await
        {
            // An idle instance with no commits at all may never notice that it has lost its
            // lease, except that we'll keep erroring here when we try to advance a
            // timestamp.
            // We want to signal that the instance should shut down if that's the case.
            if let Some(LeaseLostError) = e.downcast_ref() {
                lease_lost_shutdown
                    .signal(lease_lost_error().context("Failed to advance timestamp"));
            }
            return Err(e);
        }
        match retention_type {
            RetentionType::Document => bounds_writer
                .write()
                .advance_min_document_snapshot_ts(new_min_snapshot_ts),
            RetentionType::Index => bounds_writer
                .write()
                .advance_min_snapshot_ts(new_min_snapshot_ts),
        }
        tracing::debug!("Advance {retention_type:?} min snapshot to {new_min_snapshot_ts}");
        // Also log the deletion checkpoint here, so it is periodically reported
        // even if the deletion future is stuck.
        Self::get_checkpoint(
            persistence.reader().as_ref(),
            snapshot_reader.clone(),
            retention_type,
        )
        .await?;
        Ok(Some(new_min_snapshot_ts))
    }

    async fn emit_timestamp(
        snapshot_sender: &Sender<RepeatableTimestamp>,
        ts: anyhow::Result<Option<RepeatableTimestamp>>,
        retention_type: RetentionType,
    ) {
        match ts {
            Err(mut err) => {
                report_error(&mut err).await;
            },
            Ok(Some(ts)) => {
                log_retention_ts_advanced(retention_type);
                if let Err(err) = snapshot_sender.send(ts) {
                    report_error(&mut err.into()).await;
                }
            },
            Ok(None) => {},
        }
    }

    async fn go_advance_min_snapshot(
        mut bounds_writer: Writer<SnapshotBounds>,
        checkpoint_reader: Reader<Checkpoint>,
        rt: RT,
        persistence: Arc<dyn Persistence>,
        min_snapshot_sender: Sender<RepeatableTimestamp>,
        min_document_snapshot_sender: Sender<RepeatableTimestamp>,
        snapshot_reader: Reader<SnapshotManager>,
        shutdown: ShutdownSignal,
    ) {
        loop {
            {
                let _timer = retention_advance_timestamp_timer();

                let index_ts = Self::advance_timestamp(
                    &mut bounds_writer,
                    persistence.as_ref(),
                    &snapshot_reader,
                    &checkpoint_reader,
                    RetentionType::Index,
                    shutdown.clone(),
                )
                .await;
                Self::emit_timestamp(&min_snapshot_sender, index_ts, RetentionType::Index).await;

                let document_ts = Self::advance_timestamp(
                    &mut bounds_writer,
                    persistence.as_ref(),
                    &snapshot_reader,
                    &checkpoint_reader,
                    RetentionType::Document,
                    shutdown.clone(),
                )
                .await;
                Self::emit_timestamp(
                    &min_document_snapshot_sender,
                    document_ts,
                    RetentionType::Document,
                )
                .await;
            }
            // We jitter every loop to avoid synchronization of polling the database
            // across different instances
            Self::wait_with_jitter(&rt, ADVANCE_RETENTION_TS_FREQUENCY).await;
        }
    }

    /// Finds expired index entries in the index table and returns a tuple of
    /// the form (scanned_index_ts, expired_index_entry)
    #[try_stream(ok = (Timestamp, IndexEntry), error = anyhow::Error)]
    async fn expired_index_entries(
        reader: RepeatablePersistence,
        cursor: RepeatableTimestamp,
        min_snapshot_ts: RepeatableTimestamp,
        all_indexes: &BTreeMap<IndexId, (GenericIndexName<TabletId>, IndexedFields)>,
        persistence_version: PersistenceVersion,
    ) {
        tracing::trace!(
            "expired_index_entries: reading expired index entries from {cursor:?} to {:?}",
            min_snapshot_ts,
        );
        let mut revs = reader.load_revision_pairs(
            None, /* tablet_id */
            TimestampRange::new(*cursor..*min_snapshot_ts),
            Order::Asc,
        );
        while let Some(rev) = revs.try_next().await? {
            // Prev revs are the documents we are deleting.
            // Each prev rev has 1 or 2 index entries to delete per index -- one entry at
            // the prev rev's ts, and a tombstone at the current rev's ts if
            // the document was deleted or its index key changed.
            let RevisionPair {
                id,
                rev:
                    DocumentRevision {
                        ts,
                        document: maybe_doc,
                    },
                prev_rev,
            } = rev;
            // If there is no prev rev, there's nothing to delete.
            // If this happens for a tombstone, it means the document was created and
            // deleted in the same transaction, with no index rows.
            let Some(prev_rev) = prev_rev else {
                log_retention_scanned_document(maybe_doc.is_none(), false);
                continue;
            };
            let DocumentRevision {
                ts: prev_rev_ts,
                document: Some(prev_rev),
            } = prev_rev
            else {
                // This is unexpected: if there is a prev_ts, there should be a prev_rev.
                let mut e = anyhow::anyhow!(
                    "Skipping deleting indexes for {id}@{ts}. It has a prev_ts of {prev_ts} but \
                     no previous revision.",
                    prev_ts = prev_rev.ts
                );
                report_error(&mut e).await;
                log_retention_scanned_document(maybe_doc.is_none(), false);
                continue;
            };
            log_retention_scanned_document(maybe_doc.is_none(), true);
            for (index_id, (_, index_fields)) in all_indexes
                .iter()
                .filter(|(_, (index, _))| *index.table() == id.table())
            {
                let index_key = prev_rev
                    .index_key(index_fields, persistence_version)
                    .to_bytes();
                let key_sha256 = Sha256::hash(&index_key);
                let key = SplitKey::new(index_key.clone().0);
                log_retention_expired_index_entry(false, false);
                yield (
                    ts,
                    IndexEntry {
                        index_id: *index_id,
                        key_prefix: key.prefix.clone(),
                        key_suffix: key.suffix.clone(),
                        key_sha256: key_sha256.to_vec(),
                        ts: prev_rev_ts,
                        deleted: false,
                    },
                );
                match maybe_doc.as_ref() {
                    Some(doc) => {
                        let next_index_key =
                            doc.index_key(index_fields, persistence_version).to_bytes();
                        if index_key == next_index_key {
                            continue;
                        }
                        log_retention_expired_index_entry(true, true);
                    },
                    None => log_retention_expired_index_entry(true, false),
                }
                yield (
                    ts,
                    IndexEntry {
                        index_id: *index_id,
                        key_prefix: key.prefix,
                        key_suffix: key.suffix,
                        key_sha256: key_sha256.to_vec(),
                        ts,
                        deleted: true,
                    },
                );
            }
        }
    }

    /// Deletes some index entries based on `bounds` which identify what may be
    /// deleted. Returns a pair of the new cursor and the total expired index
    /// entries processed. The cursor is a timestamp which has been
    /// fully deleted, along with all prior timestamps. The total expired index
    /// entries is the number of index entries we found were expired, not
    /// necessarily the total we deleted or wanted to delete, though they're
    /// correlated.
    #[fastrace::trace]
    async fn delete(
        min_snapshot_ts: RepeatableTimestamp,
        persistence: Arc<dyn Persistence>,
        cursor: RepeatableTimestamp,
        all_indexes: &BTreeMap<IndexId, (GenericIndexName<TabletId>, IndexedFields)>,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> anyhow::Result<(RepeatableTimestamp, usize)> {
        if !*RETENTION_DELETES_ENABLED || *min_snapshot_ts == Timestamp::MIN {
            return Ok((cursor, 0));
        }
        // The number of rows we delete in persistence.
        let mut total_deleted_rows: usize = 0;
        // The number of expired entries we read from chunks.
        let mut total_expired_entries = 0;
        let mut new_cursor = cursor;

        let reader = persistence.reader();
        let persistence_version = reader.version();
        let snapshot_ts = min_snapshot_ts;
        let reader = RepeatablePersistence::new(reader, snapshot_ts, retention_validator.clone());

        tracing::trace!("delete: about to grab chunks");
        let expired_chunks = Self::expired_index_entries(
            reader,
            cursor,
            min_snapshot_ts,
            all_indexes,
            persistence_version,
        )
        .try_chunks2(*INDEX_RETENTION_DELETE_CHUNK);
        pin_mut!(expired_chunks);
        while let Some(delete_chunk) = expired_chunks.try_next().await? {
            tracing::trace!(
                "delete: got a chunk and finished waiting {:?}",
                delete_chunk.len()
            );
            total_expired_entries += delete_chunk.len();
            let results = try_join_all(Self::partition_chunk(delete_chunk).into_iter().map(
                |delete_chunk| Self::delete_chunk(delete_chunk, persistence.clone(), *new_cursor),
            ))
            .await?;
            let (chunk_new_cursors, deleted_rows): (Vec<_>, Vec<_>) = results.into_iter().unzip();
            // We have successfully deleted all of delete_chunk, so update
            // total_deleted_rows and new_cursor to reflect the deletions.
            total_deleted_rows += deleted_rows.into_iter().sum::<usize>();
            if let Some(max_new_cursor) = chunk_new_cursors.into_iter().max() {
                new_cursor = snapshot_ts.prior_ts(max_new_cursor)?;
            }
            if new_cursor > cursor && total_expired_entries > *RETENTION_DELETE_BATCH {
                tracing::debug!(
                    "delete: returning early with {new_cursor:?}, total expired index entries \
                     read: {total_expired_entries:?}, total rows deleted: {total_deleted_rows:?}"
                );
                // we're not done deleting everything.
                return Ok((new_cursor, total_expired_entries));
            }
        }
        tracing::debug!(
            "delete: finished loop, returning {:?}",
            min_snapshot_ts.pred()
        );
        min_snapshot_ts
            .pred()
            .map(|timestamp| (timestamp, total_expired_entries))
    }

    pub async fn delete_all_no_checkpoint(
        mut cursor_ts: RepeatableTimestamp,
        min_snapshot_ts: RepeatableTimestamp,
        persistence: Arc<dyn Persistence>,
        all_indexes: &BTreeMap<IndexId, (GenericIndexName<TabletId>, IndexedFields)>,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> anyhow::Result<()> {
        let mut last_logged = Instant::now();
        while cursor_ts.succ()? < *min_snapshot_ts {
            let (new_cursor_ts, _) = Self::delete(
                min_snapshot_ts,
                persistence.clone(),
                cursor_ts,
                all_indexes,
                retention_validator.clone(),
            )
            .await?;
            let now = Instant::now();
            let duration = now.saturating_duration_since(last_logged).as_secs_f64();
            let catchup_rate = new_cursor_ts.secs_since_f64(*cursor_ts) / duration;
            let lag = min_snapshot_ts.secs_since_f64(*new_cursor_ts);
            tracing::info!(
                "custom index retention completed between ts {cursor_ts} and {new_cursor_ts}; \
                 catchup rate: {catchup_rate:.1}s/s, {lag:.1}s behind snapshot"
            );
            cursor_ts = new_cursor_ts;
            last_logged = now;
        }
        Ok(())
    }

    /// Finds expired documents in the documents log and returns a tuple of the
    /// form (scanned_document_ts, (expired_document_ts,
    /// internal_document_ts))
    #[try_stream(ok = (Timestamp, Option<(Timestamp, InternalDocumentId)>), error = anyhow::Error)]
    async fn expired_documents(
        rt: &RT,
        persistence: Arc<dyn PersistenceReader>,
        cursor: RepeatableTimestamp,
        min_document_snapshot_ts: RepeatableTimestamp,
    ) {
        tracing::trace!(
            "expired_documents: reading expired documents from {cursor:?} to {:?}",
            min_document_snapshot_ts,
        );
        let mut revs = persistence.load_documents(
            TimestampRange::new(*cursor..*min_document_snapshot_ts),
            Order::Asc,
            *DEFAULT_DOCUMENTS_PAGE_SIZE,
            // We are reading document log entries from outside of the retention
            // window (which we have possibly just shrunk ourselves); there is
            // no need to check retention.
            Arc::new(NoopRetentionValidator),
        );
        while let Some(rev) = revs.try_next().await? {
            // Prev revs are the documents we are deleting.
            // Each prev rev has 1 or 2 entries to delete per document -- one entry at
            // the prev rev's ts, and a tombstone at the current rev's ts if
            // the document was deleted.
            // A NoopRetentionValidator is used here because we are fetching revisions
            // outside of the document retention window.
            let DocumentLogEntry {
                id,
                ts,
                value: maybe_doc,
                prev_ts,
            } = rev;
            {
                // If there is no prev rev, there's nothing to delete.
                // If this happens for a tombstone, it means the document was created and
                // deleted in the same transaction.
                let Some(prev_rev_ts) = prev_ts else {
                    log_document_retention_scanned_document(maybe_doc.is_none(), false);
                    if maybe_doc.is_none() {
                        anyhow::ensure!(
                            ts <= Timestamp::try_from(rt.unix_timestamp().as_system_time())?
                                .sub(*DOCUMENT_RETENTION_DELAY)?,
                            "Tried to delete document (id: {id}, ts: {ts}), which was out of the \
                             retention window"
                        );
                        yield (ts, Some((ts, id)));
                    } else {
                        yield (ts, None);
                    }
                    continue;
                };

                anyhow::ensure!(
                    prev_rev_ts
                        <= Timestamp::try_from(rt.unix_timestamp().as_system_time())?
                            .sub(*DOCUMENT_RETENTION_DELAY)?,
                    "Tried to delete document (id: {id}, ts: {prev_rev_ts}), which was out of the \
                     retention window"
                );
                log_document_retention_scanned_document(maybe_doc.is_none(), true);
                yield (ts, Some((prev_rev_ts, id)));

                // Deletes tombstones
                if maybe_doc.is_none() {
                    yield (ts, Some((ts, id)));
                }
            }
        }
    }

    /// Deletes some documents based on `bounds` which identify what may be
    /// deleted. Returns a pair of the new cursor and the total number of
    /// documents processed. The cursor is a timestamp which has been
    /// fully deleted, along with all prior timestamps. The total expired
    /// document count is the number of documents we found were expired, not
    /// necessarily the total we deleted or wanted to delete, though they're
    /// correlated.
    async fn delete_documents(
        min_snapshot_ts: RepeatableTimestamp,
        persistence: Arc<dyn Persistence>,
        rt: &RT,
        cursor: RepeatableTimestamp,
        retention_rate_limiter: Arc<RateLimiter<RT>>,
    ) -> anyhow::Result<(RepeatableTimestamp, usize)> {
        if !*RETENTION_DOCUMENT_DELETES_ENABLED || *min_snapshot_ts == Timestamp::MIN {
            return Ok((cursor, 0));
        }
        // The number of rows we delete in persistence.
        let mut total_deleted_rows: usize = 0;
        // The number of expired entries we read from chunks.
        let mut total_expired_entries = 0;
        let mut new_cursor = cursor;
        // The number of scanned documents
        let mut scanned_documents = 0;

        let reader = persistence.reader();
        let snapshot_ts = min_snapshot_ts;

        tracing::trace!("delete_documents: about to grab chunks");
        let expired_chunks = Self::expired_documents(rt, reader, cursor, min_snapshot_ts)
            .try_chunks2(*DOCUMENT_RETENTION_DELETE_CHUNK);
        pin_mut!(expired_chunks);
        while let Some(scanned_chunk) = expired_chunks.try_next().await? {
            tracing::trace!(
                "delete_documents: got a chunk and finished waiting {:?}",
                scanned_chunk.len()
            );
            // Converts scanned documents to the actual documents we want to delete
            scanned_documents += scanned_chunk.len();
            let delete_chunk: Vec<(Timestamp, (Timestamp, InternalDocumentId))> = scanned_chunk
                .into_iter()
                .filter_map(
                    |doc: (Timestamp, Option<(Timestamp, InternalDocumentId)>)| {
                        if doc.1.is_some() {
                            Some((doc.0, doc.1.unwrap()))
                        } else {
                            None
                        }
                    },
                )
                .collect();
            total_expired_entries += delete_chunk.len();
            let results = try_join_all(
                Self::partition_document_chunk(delete_chunk)
                    .into_iter()
                    .map(|delete_chunk| async {
                        if delete_chunk.is_empty() {
                            return Ok((*new_cursor, 0));
                        }
                        let mut chunk_len = delete_chunk.len() as u32;
                        loop {
                            match retention_rate_limiter.check_n(chunk_len.try_into().unwrap()) {
                                Ok(Ok(())) => {
                                    break;
                                },
                                Ok(Err(not_until)) => {
                                    let wait_time = Jitter::up_to(Duration::from_secs(1))
                                        + not_until.wait_time_from(rt.monotonic_now().into());
                                    rt.wait(wait_time).await;
                                    continue;
                                },
                                Err(InsufficientCapacity(n)) => {
                                    tracing::warn!(
                                        "Retention rate limiter quota is insufficient for chunks \
                                         of {} documents (current quota: {n}/sec), rate limit \
                                         will be exceeded",
                                        delete_chunk.len()
                                    );
                                    chunk_len = n;
                                    continue;
                                },
                            }
                        }
                        Self::delete_document_chunk(delete_chunk, persistence.clone(), *new_cursor)
                            .await
                    }),
            )
            .await?;
            let (chunk_new_cursors, deleted_rows): (Vec<_>, Vec<_>) = results.into_iter().unzip();
            // We have successfully deleted all of delete_chunk, so update
            // total_deleted_rows and new_cursor to reflect the deletions.
            total_deleted_rows += deleted_rows.into_iter().sum::<usize>();
            if let Some(max_new_cursor) = chunk_new_cursors.into_iter().max() {
                new_cursor = snapshot_ts.prior_ts(max_new_cursor)?;
            }
            if new_cursor > cursor && scanned_documents >= *DOCUMENT_RETENTION_MAX_SCANNED_DOCUMENTS
            {
                tracing::debug!(
                    "delete_documents: returning early with {new_cursor:?}, total expired \
                     documents read: {total_expired_entries:?}, total rows deleted: \
                     {total_deleted_rows:?}"
                );
                // we're not done deleting everything.
                return Ok((new_cursor, scanned_documents));
            }
        }
        tracing::debug!(
            "delete: finished loop, returning {:?}",
            min_snapshot_ts.pred()
        );
        min_snapshot_ts
            .pred()
            .map(|timestamp| (timestamp, total_expired_entries))
    }

    /// Partitions IndexEntry into INDEX_RETENTION_DELETE_PARALLEL parts where
    /// each index key only exists in one part.
    fn partition_chunk(
        to_partition: Vec<(Timestamp, IndexEntry)>,
    ) -> Vec<Vec<(Timestamp, IndexEntry)>> {
        let mut parts = Vec::new();
        for _ in 0..*INDEX_RETENTION_DELETE_PARALLEL {
            parts.push(vec![]);
        }
        for entry in to_partition {
            let mut hash = DefaultHasher::new();
            entry.1.key_sha256.hash(&mut hash);
            let i = (hash.finish() as usize) % *INDEX_RETENTION_DELETE_PARALLEL;
            parts[i].push(entry);
        }
        parts
    }

    /// Partitions documents into RETENTION_DELETE_PARALLEL parts where each
    /// document id only exists in one part
    fn partition_document_chunk(
        to_partition: Vec<(Timestamp, (Timestamp, InternalDocumentId))>,
    ) -> Vec<Vec<(Timestamp, (Timestamp, InternalDocumentId))>> {
        let mut parts = Vec::new();
        for _ in 0..*DOCUMENT_RETENTION_DELETE_PARALLEL {
            parts.push(vec![]);
        }
        for entry in to_partition {
            let mut hash = DefaultHasher::new();
            entry.1 .1.hash(&mut hash);
            let i = (hash.finish() as usize) % *DOCUMENT_RETENTION_DELETE_PARALLEL;
            parts[i].push(entry);
        }
        parts
    }

    #[fastrace::trace]
    async fn delete_chunk(
        delete_chunk: Vec<(Timestamp, IndexEntry)>,
        persistence: Arc<dyn Persistence>,
        mut new_cursor: Timestamp,
    ) -> anyhow::Result<(Timestamp, usize)> {
        let _timer = retention_delete_chunk_timer();
        let index_entries_to_delete = delete_chunk.len();
        tracing::trace!("delete: got entries to delete {index_entries_to_delete:?}");
        for index_entry_to_delete in delete_chunk.iter() {
            // If we're deleting the previous revision of an index entry, we've definitely
            // deleted index entries for documents at all prior timestamps.
            if index_entry_to_delete.0 > Timestamp::MIN {
                new_cursor = cmp::max(new_cursor, index_entry_to_delete.0.pred()?);
            }
        }
        let deleted_rows = if index_entries_to_delete > 0 {
            persistence
                .delete_index_entries(delete_chunk.into_iter().map(|ind| ind.1).collect())
                .await?
        } else {
            0
        };

        // If there are more entries to delete than we see in the delete chunk,
        // it means retention skipped deleting entries before, and we
        // incorrectly bumped RetentionConfirmedDeletedTimestamp anyway.
        if deleted_rows > index_entries_to_delete {
            report_error(&mut anyhow::anyhow!(
                "retention wanted to delete {index_entries_to_delete} entries but found \
                 {deleted_rows} to delete"
            ))
            .await;
        }

        tracing::trace!("delete: deleted {deleted_rows:?} rows");
        log_retention_index_entries_deleted(deleted_rows);
        Ok((new_cursor, deleted_rows))
    }

    async fn delete_document_chunk(
        delete_chunk: Vec<(Timestamp, (Timestamp, InternalDocumentId))>,
        persistence: Arc<dyn Persistence>,
        mut new_cursor: Timestamp,
    ) -> anyhow::Result<(Timestamp, usize)> {
        let _timer = retention_delete_document_chunk_timer();
        let documents_to_delete = delete_chunk.len();
        tracing::trace!("delete_documents: there are {documents_to_delete:?} documents to delete");
        for document_to_delete in delete_chunk.iter() {
            // If we're deleting the previous revision of a document, we've definitely
            // deleted entries for documents at all prior timestamps.
            if document_to_delete.0 > Timestamp::MIN {
                new_cursor = cmp::max(new_cursor, document_to_delete.0.pred()?);
            }
        }
        let deleted_rows = if documents_to_delete > 0 {
            persistence
                .delete(delete_chunk.into_iter().map(|doc| doc.1).collect())
                .await?
        } else {
            0
        };

        // If there are more documents to delete than we see in the delete chunk,
        // it means retention skipped deleting documents before, and we
        // incorrectly bumped DocumentRetentionConfirmedDeletedTimestamp anyway.
        if deleted_rows > documents_to_delete {
            report_error(&mut anyhow::anyhow!(
                "retention wanted to delete {documents_to_delete} documents but found \
                 {deleted_rows} to delete"
            ))
            .await;
        }

        tracing::trace!("delete_documents: deleted {deleted_rows:?} rows");
        log_retention_documents_deleted(deleted_rows);
        Ok((new_cursor, deleted_rows))
    }

    async fn wait_with_jitter(rt: &RT, delay: Duration) {
        // Abuse backoff to get jitter by passing in the same constant for initial and
        // max backoff.
        let mut initial_backoff = Backoff::new(delay, delay);
        let delay = initial_backoff.fail(&mut rt.rng());
        rt.wait(delay).await;
    }

    async fn go_delete_indexes(
        bounds_reader: Reader<SnapshotBounds>,
        rt: RT,
        persistence: Arc<dyn Persistence>,
        mut all_indexes: BTreeMap<IndexId, (GenericIndexName<TabletId>, IndexedFields)>,
        index_table_id: TabletId,
        mut index_cursor: RepeatableTimestamp,
        retention_validator: Arc<dyn RetentionValidator>,
        mut min_snapshot_rx: Receiver<RepeatableTimestamp>,
        mut checkpoint_writer: Writer<Checkpoint>,
        snapshot_reader: Reader<SnapshotManager>,
    ) {
        let reader = persistence.reader();

        let mut error_backoff = Backoff::new(INITIAL_BACKOFF, *MAX_RETENTION_DELAY_SECONDS);
        let mut min_snapshot_ts = RepeatableTimestamp::MIN;
        let mut is_working = false;
        loop {
            if !is_working {
                min_snapshot_ts = match min_snapshot_rx.changed().await {
                    Err(err) => {
                        report_error(&mut err.into()).await;
                        // Fall back to polling if the channel is closed or falls over. This should
                        // really never happen.
                        Self::wait_with_jitter(&rt, *MAX_RETENTION_DELAY_SECONDS).await;
                        bounds_reader.lock().min_index_snapshot_ts
                    },
                    Ok(()) => *min_snapshot_rx.borrow_and_update(),
                };
                is_working = true;
            }

            tracing::trace!(
                "go_delete_indexes: running, is_working: {is_working}, current_bounds: \
                 {min_snapshot_ts}",
            );
            let span = get_sampled_span("", "delete_indexes", &mut rt.rng());
            let r: anyhow::Result<()> = async {
                let _timer = retention_delete_timer();
                let cursor = Self::get_checkpoint(
                    reader.as_ref(),
                    snapshot_reader.clone(),
                    RetentionType::Index,
                )
                .await?;
                tracing::trace!("go_delete: loaded checkpoint: {cursor:?}");
                let latest_ts = snapshot_reader.lock().persisted_max_repeatable_ts();
                Self::accumulate_indexes(
                    persistence.as_ref(),
                    &mut all_indexes,
                    &mut index_cursor,
                    latest_ts,
                    index_table_id,
                    retention_validator.clone(),
                )
                .await?;
                tracing::trace!("go_delete: Loaded initial indexes");
                let index_count_before = all_indexes.len();
                let (new_cursor, expired_index_entries_processed) = Self::delete(
                    min_snapshot_ts,
                    persistence.clone(),
                    cursor,
                    &all_indexes,
                    retention_validator.clone(),
                )
                .await?;
                tracing::trace!("go_delete: finished running delete");
                let latest_ts = snapshot_reader.lock().persisted_max_repeatable_ts();
                Self::accumulate_indexes(
                    persistence.as_ref(),
                    &mut all_indexes,
                    &mut index_cursor,
                    latest_ts,
                    index_table_id,
                    retention_validator.clone(),
                )
                .await?;
                tracing::trace!("go_delete: loaded second round of indexes");
                if all_indexes.len() == index_count_before {
                    tracing::debug!("go_delete: Checkpointing at: {new_cursor:?}");
                    // No indexes were added while we were doing the delete.
                    // So the `delete` covered all index rows up to new_cursor.
                    Self::checkpoint(
                        persistence.as_ref(),
                        new_cursor,
                        &mut checkpoint_writer,
                        RetentionType::Index,
                        bounds_reader.clone(),
                        snapshot_reader.clone(),
                    )
                    .await?;
                } else {
                    tracing::debug!(
                        "go_delete: Skipping checkpoint, index count changed, now: {:?}, before: \
                         {index_count_before:?}",
                        all_indexes.len()
                    );
                }

                // If we deleted >= the delete batch size, we probably returned
                // early and have more work to do, so run again immediately.
                is_working = expired_index_entries_processed >= *RETENTION_DELETE_BATCH;
                if is_working {
                    tracing::trace!(
                        "go_delete: processed {expired_index_entries_processed:?} rows, more to go"
                    );
                }
                Ok(())
            }
            .in_span(span)
            .await;
            if let Err(mut err) = r {
                report_error(&mut err).await;
                let delay = error_backoff.fail(&mut rt.rng());
                tracing::debug!("go_delete: error, {err:?}, delaying {delay:?}");
                rt.wait(delay).await;
            } else {
                error_backoff.reset();
            }
        }
    }

    async fn go_delete_documents(
        bounds_reader: Reader<SnapshotBounds>,
        rt: RT,
        persistence: Arc<dyn Persistence>,
        mut min_document_snapshot_rx: Receiver<RepeatableTimestamp>,
        mut checkpoint_writer: Writer<Checkpoint>,
        snapshot_reader: Reader<SnapshotManager>,
        retention_rate_limiter: Arc<RateLimiter<RT>>,
    ) {
        // Wait with jitter on startup to avoid thundering herd
        Self::wait_with_jitter(&rt, *DOCUMENT_RETENTION_BATCH_INTERVAL_SECONDS).await;

        let reader = persistence.reader();

        let mut error_backoff =
            Backoff::new(INITIAL_BACKOFF, *DOCUMENT_RETENTION_BATCH_INTERVAL_SECONDS);
        let mut min_document_snapshot_ts = RepeatableTimestamp::MIN;
        let mut is_working = false;
        let mut interval = tokio::time::interval(*DOCUMENT_RETENTION_BATCH_INTERVAL_SECONDS);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            if !is_working {
                min_document_snapshot_ts = match min_document_snapshot_rx.changed().await {
                    Err(err) => {
                        report_error(&mut err.into()).await;
                        // Fall back to polling if the channel is closed or falls over. This should
                        // really never happen.
                        Self::wait_with_jitter(&rt, *DOCUMENT_RETENTION_BATCH_INTERVAL_SECONDS)
                            .await;
                        bounds_reader.lock().min_index_snapshot_ts
                    },
                    Ok(()) => *min_document_snapshot_rx.borrow_and_update(),
                };
                is_working = true;
            }

            // Rate limit so we don't overload the database
            interval.tick().await;

            tracing::trace!(
                "go_delete_documents: running, is_working: {is_working}, current_bounds: \
                 {min_document_snapshot_ts}",
            );
            let r: anyhow::Result<()> = try {
                let _timer = retention_delete_documents_timer();
                let cursor = Self::get_checkpoint(
                    reader.as_ref(),
                    snapshot_reader.clone(),
                    RetentionType::Document,
                )
                .await?;
                tracing::trace!("go_delete_documents: loaded checkpoint: {cursor:?}");
                let (new_cursor, scanned_documents) = Self::delete_documents(
                    min_document_snapshot_ts,
                    persistence.clone(),
                    &rt,
                    cursor,
                    retention_rate_limiter.clone(),
                )
                .await?;
                tracing::debug!("go_delete_documents: Checkpointing at: {new_cursor:?}");

                Self::checkpoint(
                    persistence.as_ref(),
                    new_cursor,
                    &mut checkpoint_writer,
                    RetentionType::Document,
                    bounds_reader.clone(),
                    snapshot_reader.clone(),
                )
                .await?;

                // If we scanned >= the scanned batch, we probably returned
                // early and have more work to do, so run again immediately.
                is_working = scanned_documents >= *DOCUMENT_RETENTION_MAX_SCANNED_DOCUMENTS;
                if is_working {
                    tracing::trace!(
                        "go_delete_documents: processed {scanned_documents:?} rows, more to go"
                    );
                }
            };
            if let Err(mut err) = r {
                report_error(&mut err).await;
                let delay = error_backoff.fail(&mut rt.rng());
                tracing::debug!("go_delete_documents: error, {err:?}, delaying {delay:?}");
                rt.wait(delay).await;
            } else {
                error_backoff.reset();
            }
        }
    }

    async fn checkpoint(
        persistence: &dyn Persistence,
        cursor: RepeatableTimestamp,
        checkpoint_writer: &mut Writer<Checkpoint>,
        retention_type: RetentionType,
        bounds_reader: Reader<SnapshotBounds>,
        snapshot_reader: Reader<SnapshotManager>,
    ) -> anyhow::Result<()> {
        let key = match retention_type {
            RetentionType::Document => {
                PersistenceGlobalKey::DocumentRetentionConfirmedDeletedTimestamp
            },
            RetentionType::Index => PersistenceGlobalKey::RetentionConfirmedDeletedTimestamp,
        };
        persistence
            .write_persistence_global(key, ConvexValue::from(i64::from(*cursor)).into())
            .await?;
        checkpoint_writer.write().advance_checkpoint(cursor);
        if *cursor > Timestamp::MIN {
            // Only log if the checkpoint has been written once, to avoid logging time since
            // epoch when the instance is first starting up.
            match retention_type {
                RetentionType::Document => {
                    log_document_retention_cursor_age(
                        (*snapshot_reader.lock().persisted_max_repeatable_ts())
                            .secs_since_f64(*cursor),
                    );
                    log_document_retention_cursor_lag(
                        bounds_reader
                            .lock()
                            .min_document_snapshot_ts
                            .secs_since_f64(*cursor),
                    );
                },
                RetentionType::Index => {
                    log_retention_cursor_age(
                        (*snapshot_reader.lock().persisted_max_repeatable_ts())
                            .secs_since_f64(*cursor),
                    );
                    log_retention_cursor_lag(
                        bounds_reader
                            .lock()
                            .min_index_snapshot_ts
                            .secs_since_f64(*cursor),
                    );
                },
            }
        } else {
            match retention_type {
                RetentionType::Document => log_document_retention_no_cursor(),
                RetentionType::Index => log_retention_no_cursor(),
            }
        }
        Ok(())
    }

    pub async fn get_checkpoint_not_repeatable(
        persistence: &dyn PersistenceReader,
        retention_type: RetentionType,
    ) -> anyhow::Result<Timestamp> {
        let key = match retention_type {
            RetentionType::Document => {
                PersistenceGlobalKey::DocumentRetentionConfirmedDeletedTimestamp
            },
            RetentionType::Index => PersistenceGlobalKey::RetentionConfirmedDeletedTimestamp,
        };
        let checkpoint_value = persistence
            .get_persistence_global(key)
            .await?
            .map(ConvexValue::try_from)
            .transpose()?;
        let checkpoint = match checkpoint_value {
            Some(ConvexValue::Int64(ts)) => Timestamp::try_from(ts)?,
            None => Timestamp::MIN,
            _ => anyhow::bail!("invalid retention checkpoint {checkpoint_value:?}"),
        };
        Ok(checkpoint)
    }

    pub async fn get_checkpoint(
        persistence: &dyn PersistenceReader,
        snapshot_reader: Reader<SnapshotManager>,
        retention_type: RetentionType,
    ) -> anyhow::Result<RepeatableTimestamp> {
        let checkpoint = Self::get_checkpoint_not_repeatable(persistence, retention_type).await?;
        snapshot_reader
            .lock()
            .persisted_max_repeatable_ts()
            .prior_ts(checkpoint)
    }

    fn accumulate_index_document(
        doc: ResolvedDocument,
        all_indexes: &mut BTreeMap<IndexId, (GenericIndexName<TabletId>, IndexedFields)>,
    ) -> anyhow::Result<()> {
        let index_id = doc.id().internal_id();
        let index: ParsedDocument<IndexMetadata<TabletId>> = doc.parse()?;
        let index = index.into_value();
        let IndexConfig::Database {
            spec,
            on_disk_state,
        } = index.config
        else {
            return Ok(());
        };

        // Don't run retention for indexes that are still backfilling unless IndexWorker
        // has explicitly opted-in to running retention. This is important for
        // correctness since index backfill and retention interact poorly.
        // NOTE: accumulate only adds indexes. Thus we won't stop running
        // retention if index is deleted or changes from Enabled to Backfilling.
        if let DatabaseIndexState::Backfilling(state) = on_disk_state
            && !state.retention_started
        {
            return Ok(());
        }

        all_indexes.insert(index_id, (index.name, spec.fields));
        Ok(())
    }

    #[fastrace::trace]
    async fn accumulate_indexes(
        persistence: &dyn Persistence,
        all_indexes: &mut BTreeMap<IndexId, (GenericIndexName<TabletId>, IndexedFields)>,
        cursor: &mut RepeatableTimestamp,
        latest_ts: RepeatableTimestamp,
        index_table_id: TabletId,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> anyhow::Result<()> {
        let reader = persistence.reader();
        let mut document_stream = reader.load_documents_from_table(
            index_table_id,
            TimestampRange::new(**cursor..*latest_ts),
            Order::Asc,
            *DEFAULT_DOCUMENTS_PAGE_SIZE,
            retention_validator,
        );
        while let Some(entry) = document_stream.try_next().await? {
            if let Some(doc) = entry.value {
                anyhow::ensure!(doc.id().tablet_id == index_table_id);
                Self::accumulate_index_document(doc, all_indexes)?;
            }
        }
        *cursor = latest_ts;
        Ok(())
    }
}

const ADVANCE_RETENTION_TS_FREQUENCY: Duration = Duration::from_secs(30);

#[async_trait]
impl<RT: Runtime> RetentionValidator for LeaderRetentionManager<RT> {
    async fn validate_snapshot(&self, ts: Timestamp) -> anyhow::Result<()> {
        let min_snapshot_ts = self.bounds_reader.lock().min_index_snapshot_ts;
        log_snapshot_verification_age(&self.rt, ts, *min_snapshot_ts, false, true);
        if ts < *min_snapshot_ts {
            anyhow::bail!(snapshot_invalid_error(
                ts,
                *min_snapshot_ts,
                RetentionType::Index
            ));
        }
        Ok(())
    }

    async fn validate_document_snapshot(&self, ts: Timestamp) -> anyhow::Result<()> {
        let min_snapshot_ts = self.bounds_reader.lock().min_document_snapshot_ts;
        if ts < *min_snapshot_ts {
            anyhow::bail!(snapshot_invalid_error(
                ts,
                *min_snapshot_ts,
                RetentionType::Document
            ));
        }
        Ok(())
    }

    fn optimistic_validate_snapshot(&self, ts: Timestamp) -> anyhow::Result<()> {
        let min_snapshot_ts = self.bounds_reader.lock().min_index_snapshot_ts;
        log_snapshot_verification_age(&self.rt, ts, *min_snapshot_ts, true, true);
        anyhow::ensure!(
            ts >= *min_snapshot_ts,
            "leader retention bounds check failed: {ts} < {min_snapshot_ts}"
        );
        Ok(())
    }

    async fn min_snapshot_ts(&self) -> anyhow::Result<RepeatableTimestamp> {
        Ok(self.bounds_reader.lock().min_index_snapshot_ts)
    }

    async fn min_document_snapshot_ts(&self) -> anyhow::Result<RepeatableTimestamp> {
        Ok(self.bounds_reader.lock().min_document_snapshot_ts)
    }

    fn fail_if_falling_behind(&self) -> anyhow::Result<()> {
        if !*RETENTION_FAIL_ENABLED {
            return Ok(());
        }

        let checkpoint = self.checkpoint_reader.lock().checkpoint;
        if let Some(checkpoint) = checkpoint {
            let age = Timestamp::try_from(self.rt.system_time())?.secs_since_f64(*checkpoint);
            let retention_delay_seconds = (*INDEX_RETENTION_DELAY).as_secs();

            let min_failure_duration = Duration::from_secs(
                retention_delay_seconds * *RETENTION_FAIL_START_MULTIPLIER as u64,
            )
            .as_secs_f64();
            let max_failure_duration = Duration::from_secs(
                retention_delay_seconds * *RETENTION_FAIL_ALL_MULTIPLIER as u64,
            )
            .as_secs_f64();
            if age < min_failure_duration {
                return Ok(());
            }
            let failure_percentage = age / max_failure_duration;
            let is_failure = if age < min_failure_duration {
                false
            } else {
                let failure_die: f64 = self.rt.rng().random();
                // failure_percentage might be >= 1.0, which will always cause failures because
                // rng.random() is between 0 and 1.0. That's totally fine, at some point it's ok
                // for all writes to fail.
                failure_die < failure_percentage
            };

            anyhow::ensure!(
                !is_failure,
                ErrorMetadata::overloaded(
                    "TooManyWritesInTimePeriod",
                    "Too many insert / update / delete operations in a short period of time. \
                     Spread your writes out over time or throttle them to avoid errors."
                )
            );
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct FollowerRetentionManager<RT: Runtime> {
    rt: RT,
    snapshot_bounds: Arc<Mutex<SnapshotBounds>>,
    persistence: Arc<dyn PersistenceReader>,
}

impl<RT: Runtime> FollowerRetentionManager<RT> {
    pub async fn new(rt: RT, persistence: Arc<dyn PersistenceReader>) -> anyhow::Result<Self> {
        let repeatable_ts = new_static_repeatable_recent(persistence.as_ref()).await?;
        Self::new_with_repeatable_ts(rt, persistence, repeatable_ts).await
    }

    pub async fn new_with_repeatable_ts(
        rt: RT,
        persistence: Arc<dyn PersistenceReader>,
        repeatable_ts: RepeatableTimestamp,
    ) -> anyhow::Result<Self> {
        let min_index_snapshot_ts =
            latest_retention_min_snapshot_ts(persistence.as_ref(), RetentionType::Index).await?;
        let min_document_snapshot_ts =
            latest_retention_min_snapshot_ts(persistence.as_ref(), RetentionType::Document).await?;
        if *repeatable_ts < min_index_snapshot_ts {
            anyhow::bail!(snapshot_invalid_error(
                *repeatable_ts,
                min_index_snapshot_ts,
                RetentionType::Index
            ));
        }
        let snapshot_bounds = Arc::new(Mutex::new(SnapshotBounds {
            min_index_snapshot_ts: repeatable_ts.prior_ts(min_index_snapshot_ts)?,
            min_document_snapshot_ts: repeatable_ts.prior_ts(min_document_snapshot_ts)?,
        }));
        Ok(Self {
            rt,
            snapshot_bounds,
            persistence,
        })
    }
}

#[async_trait]
impl<RT: Runtime> RetentionValidator for FollowerRetentionManager<RT> {
    async fn validate_snapshot(&self, ts: Timestamp) -> anyhow::Result<()> {
        let min_snapshot_ts = self.min_snapshot_ts().await?;
        log_snapshot_verification_age(&self.rt, ts, *min_snapshot_ts, false, false);
        if ts < *min_snapshot_ts {
            anyhow::bail!(snapshot_invalid_error(
                ts,
                *min_snapshot_ts,
                RetentionType::Index
            ));
        }
        Ok(())
    }

    async fn validate_document_snapshot(&self, ts: Timestamp) -> anyhow::Result<()> {
        let min_snapshot_ts = self.min_document_snapshot_ts().await?;
        if ts < *min_snapshot_ts {
            anyhow::bail!(snapshot_invalid_error(
                ts,
                *min_snapshot_ts,
                RetentionType::Document
            ));
        }
        Ok(())
    }

    fn optimistic_validate_snapshot(&self, ts: Timestamp) -> anyhow::Result<()> {
        let min_snapshot_ts = self.snapshot_bounds.lock().min_index_snapshot_ts;
        log_snapshot_verification_age(&self.rt, ts, *min_snapshot_ts, true, false);
        anyhow::ensure!(
            ts >= *min_snapshot_ts,
            "follower retention bounds check failed: {ts} < {min_snapshot_ts}"
        );
        Ok(())
    }

    async fn min_snapshot_ts(&self) -> anyhow::Result<RepeatableTimestamp> {
        let snapshot_ts = new_static_repeatable_recent(self.persistence.as_ref()).await?;
        let latest = snapshot_ts.prior_ts(
            latest_retention_min_snapshot_ts(self.persistence.as_ref(), RetentionType::Index)
                .await?,
        )?;
        let mut snapshot_bounds = self.snapshot_bounds.lock();
        snapshot_bounds.advance_min_snapshot_ts(latest);
        Ok(latest)
    }

    async fn min_document_snapshot_ts(&self) -> anyhow::Result<RepeatableTimestamp> {
        let snapshot_ts = new_static_repeatable_recent(self.persistence.as_ref()).await?;
        let latest = snapshot_ts.prior_ts(
            latest_retention_min_snapshot_ts(self.persistence.as_ref(), RetentionType::Document)
                .await?,
        )?;
        let mut snapshot_bounds = self.snapshot_bounds.lock();
        snapshot_bounds.advance_min_document_snapshot_ts(latest);
        Ok(latest)
    }

    fn fail_if_falling_behind(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

fn snapshot_invalid_error(
    ts: Timestamp,
    min_snapshot_ts: Timestamp,
    retention_type: RetentionType,
) -> anyhow::Error {
    anyhow::anyhow!(ErrorMetadata::out_of_retention()).context(format!(
        "{retention_type:?} snapshot timestamp out of retention window: {ts} < {min_snapshot_ts}"
    ))
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        sync::Arc,
    };

    use common::{
        bootstrap_model::index::{
            database_index::IndexedFields,
            INDEX_TABLE,
        },
        index::IndexKey,
        interval::Interval,
        persistence::{
            ConflictStrategy,
            NoopRetentionValidator,
            Persistence,
            PersistenceIndexEntry,
            RepeatablePersistence,
        },
        query::Order,
        runtime::testing::TestRuntime,
        testing::{
            persistence_test_suite::doc,
            TestIdGenerator,
            TestPersistence,
        },
        try_chunks::TryChunksExt,
        types::{
            unchecked_repeatable_ts,
            GenericIndexName,
            IndexDescriptor,
            RepeatableTimestamp,
            Timestamp,
        },
        value::{
            ConvexValue,
            ResolvedDocumentId,
            TableName,
        },
    };
    use errors::ErrorMetadataAnyhowExt;
    use futures::{
        future::try_join_all,
        pin_mut,
        stream,
        TryStreamExt,
    };
    use maplit::btreemap;

    use super::LeaderRetentionManager;
    use crate::retention::{
        snapshot_invalid_error,
        RetentionType,
    };

    #[convex_macro::test_runtime]
    async fn test_chunks_is_out_of_retention(_rt: TestRuntime) -> anyhow::Result<()> {
        let throws = || -> anyhow::Result<()> {
            anyhow::bail!(snapshot_invalid_error(
                Timestamp::must(1),
                Timestamp::must(30),
                RetentionType::Document
            ));
        };
        let stream_throws = stream::once(async move { throws() });
        // IMPORTANT: try_chunks fails here. try_chunks2 is necessary.
        let chunks = stream_throws.try_chunks2(1);
        let chunk_throws = async move || -> anyhow::Result<()> {
            pin_mut!(chunks);
            chunks.try_next().await?;
            anyhow::Ok(())
        };
        let err = chunk_throws().await.unwrap_err();
        assert!(err.is_out_of_retention());
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_expired_index_entries(_rt: TestRuntime) -> anyhow::Result<()> {
        let p = Arc::new(TestPersistence::new());
        let mut id_generator = TestIdGenerator::new();
        let by_id_index_id = id_generator.system_generate(&INDEX_TABLE).internal_id();
        let by_val_index_id = id_generator.system_generate(&INDEX_TABLE).internal_id();
        let table: TableName = str::parse("table")?;
        let table_id = id_generator.user_table_id(&table).tablet_id;

        let by_id = |id: ResolvedDocumentId,
                     ts: i32,
                     deleted: bool|
         -> anyhow::Result<PersistenceIndexEntry> {
            let key = IndexKey::new(vec![], id.into()).to_bytes();
            Ok(PersistenceIndexEntry {
                ts: Timestamp::must(ts),
                index_id: by_id_index_id,
                key,
                value: if deleted { None } else { Some(id.into()) },
            })
        };

        let by_val = |id: ResolvedDocumentId,
                      ts: i32,
                      val: i64,
                      deleted: bool|
         -> anyhow::Result<PersistenceIndexEntry> {
            let key = IndexKey::new(vec![ConvexValue::from(val)], id.into()).to_bytes();
            Ok(PersistenceIndexEntry {
                ts: Timestamp::must(ts),
                index_id: by_val_index_id,
                key,
                value: if deleted { None } else { Some(id.into()) },
            })
        };

        let id1 = id_generator.user_generate(&table);
        let id2 = id_generator.user_generate(&table);
        let id3 = id_generator.user_generate(&table);
        let id4 = id_generator.user_generate(&table);
        let id5 = id_generator.user_generate(&table);

        let documents = [
            doc(id1, 1, Some(5), None)?,    // expired because overwritten.
            doc(id2, 2, Some(5), None)?,    // expired because overwritten.
            doc(id1, 3, Some(6), Some(1))?, // latest.
            doc(id2, 4, None, Some(2))?,    // expired because tombstone.
            doc(id3, 5, Some(5), None)?,    // latest.
            doc(id4, 6, Some(5), None)?,    // visible at min_snapshot_ts.
            doc(id5, 7, Some(5), None)?,    // visible at min_snapshot_ts.
            // min_snapshot_ts: 8
            doc(id4, 9, None, Some(6))?,
            doc(id5, 10, Some(6), Some(7))?,
            doc(id5, 11, Some(5), Some(10))?,
        ];
        // indexes derived from documents.
        let indexes = [
            by_id(id1, 1, false)?,     // expired because overwritten.
            by_val(id1, 1, 5, false)?, // expired because overwritten.
            by_id(id2, 2, false)?,     // expired because overwritten.
            by_val(id2, 2, 5, false)?, // expired because overwritten.
            by_id(id1, 3, false)?,
            by_val(id1, 3, 5, true)?, // expired because tombstone.
            by_val(id1, 3, 6, false)?,
            by_id(id2, 4, true)?,     // expired because tombstone.
            by_val(id2, 4, 5, true)?, // expired because tombstone.
            by_id(id3, 5, false)?,
            by_val(id3, 5, 5, false)?,
            by_id(id4, 6, false)?,
            by_val(id4, 6, 5, false)?,
            by_id(id5, 7, false)?,
            by_val(id5, 7, 5, false)?,
            // min_snapshot_ts: 8
            by_id(id4, 9, true)?,
            by_val(id4, 9, 5, true)?,
            by_id(id5, 10, false)?,
            by_val(id5, 10, 5, true)?,
            by_val(id5, 10, 6, false)?,
            by_id(id5, 11, false)?,
            by_val(id5, 11, 6, true)?,
            by_val(id5, 11, 5, false)?,
        ];

        p.write(&documents, &indexes, ConflictStrategy::Error)
            .await?;
        id_generator.write_tables(p.clone()).await?;

        let min_snapshot_ts = unchecked_repeatable_ts(Timestamp::must(8));
        let repeatable_ts = min_snapshot_ts;

        let reader = p.reader();
        let persistence_version = reader.version();
        let retention_validator = Arc::new(NoopRetentionValidator);
        let reader = RepeatablePersistence::new(reader, repeatable_ts, retention_validator.clone());

        let all_indexes = btreemap!(
            by_id_index_id => (GenericIndexName::by_id(table_id), IndexedFields::by_id()),
            by_val_index_id => (GenericIndexName::new(table_id, IndexDescriptor::new("by_val")?)?, IndexedFields::try_from(vec!["value".parse()?])?),
        );
        let expired_stream = LeaderRetentionManager::<TestRuntime>::expired_index_entries(
            reader,
            RepeatableTimestamp::MIN,
            min_snapshot_ts,
            &all_indexes,
            persistence_version,
        );
        let expired: Vec<_> = expired_stream.try_collect().await?;

        assert_eq!(expired.len(), 7);
        assert_eq!(
            p.delete_index_entries(expired.into_iter().map(|ind| ind.1).collect())
                .await?,
            7
        );

        let reader = p.reader();
        let reader = RepeatablePersistence::new(reader, repeatable_ts, retention_validator);
        let snapshot_reader = reader.read_snapshot(repeatable_ts)?;

        // All documents are still visible at snapshot ts=8.
        let stream =
            snapshot_reader.index_scan(by_val_index_id, table_id, &Interval::all(), Order::Asc, 1);
        let results: Vec<_> = stream
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .map(|(_, rev)| (rev.value.id(), i64::from(rev.ts)))
            .collect();
        assert_eq!(results, vec![(id3, 5), (id4, 6), (id5, 7), (id1, 3)]);

        // Old versions of documents at snapshot ts=2 are not visible.
        let snapshot_reader = reader.read_snapshot(unchecked_repeatable_ts(Timestamp::must(2)))?;
        let stream =
            snapshot_reader.index_scan(by_val_index_id, table_id, &Interval::all(), Order::Asc, 1);
        let results: Vec<_> = stream.try_collect::<Vec<_>>().await?;
        assert_eq!(results, vec![]);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_expired_documents(rt: TestRuntime) -> anyhow::Result<()> {
        let p = TestPersistence::new();
        let mut id_generator = TestIdGenerator::new();
        let table: TableName = str::parse("table")?;

        let id1 = id_generator.user_generate(&table);
        let id2 = id_generator.user_generate(&table);
        let id3 = id_generator.user_generate(&table);
        let id4 = id_generator.user_generate(&table);
        let id5 = id_generator.user_generate(&table);
        let id6 = id_generator.user_generate(&table);
        let id7 = id_generator.user_generate(&table);

        let documents = [
            doc(id1, 1, Some(1), None)?, // no longer visible from > min_document_snapshot_ts
            doc(id2, 1, Some(2), None)?, // no longer visible from > min_document_snapshot_ts
            doc(id3, 1, Some(3), None)?, // no longer visible from > min_document_snapshot_ts
            doc(id1, 2, None, Some(1))?, // tombstone
            doc(id2, 2, Some(1), Some(1))?,
            doc(id3, 2, Some(2), Some(1))?,
            doc(id4, 2, Some(2), None)?,
            doc(id7, 2, None, None)?, // doc that was inserted and deleted in the same transaction
            // min_document_snapshot_ts: 4
            doc(id5, 5, Some(4), None)?,
            doc(id6, 6, Some(5), None)?,
        ];

        p.write(&documents, &[], ConflictStrategy::Error).await?;

        let min_snapshot_ts = unchecked_repeatable_ts(Timestamp::must(4));

        let reader = p.reader();

        let scanned_stream = LeaderRetentionManager::<TestRuntime>::expired_documents(
            &rt,
            reader,
            RepeatableTimestamp::MIN,
            min_snapshot_ts,
        );
        let scanned: Vec<_> = scanned_stream.try_collect().await?;
        let expired: Vec<_> = scanned
            .into_iter()
            .filter_map(|doc| {
                if doc.1.is_some() {
                    Some((doc.0, doc.1.unwrap()))
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(expired.len(), 5);
        assert_eq!(
            p.delete(expired.into_iter().map(|doc| doc.1).collect())
                .await?,
            5
        );

        let reader = p.reader();

        // All documents are still visible at snapshot ts=4.
        let stream = reader.load_all_documents();
        let results: Vec<_> = stream.try_collect::<Vec<_>>().await?.into_iter().collect();
        assert_eq!(
            results,
            vec![
                doc(id2, 2, Some(1), Some(1))?,
                doc(id3, 2, Some(2), Some(1))?,
                doc(id4, 2, Some(2), None)?,
                doc(id5, 5, Some(4), None)?,
                doc(id6, 6, Some(5), None)?,
            ]
        );

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_delete_document_chunk(rt: TestRuntime) -> anyhow::Result<()> {
        unsafe { env::set_var("DOCUMENT_RETENTION_DELETE_PARALLEL", "4") };
        let p = Arc::new(TestPersistence::new());
        let mut id_generator = TestIdGenerator::new();
        let table: TableName = str::parse("table")?;

        let id1 = id_generator.user_generate(&table);

        let documents = [
            doc(id1, 1, Some(1), None)?,
            doc(id1, 2, Some(2), Some(1))?,
            doc(id1, 3, Some(3), Some(2))?,
            doc(id1, 4, Some(4), Some(3))?,
            doc(id1, 5, Some(5), Some(4))?,
            doc(id1, 6, Some(6), Some(5))?,
            doc(id1, 7, Some(7), Some(6))?,
            doc(id1, 8, Some(8), Some(7))?,
            doc(id1, 9, Some(9), Some(8))?,
            doc(id1, 10, Some(10), Some(9))?,
            // min_document_snapshot_ts: 11
            doc(id1, 12, Some(12), Some(10))?,
            doc(id1, 13, Some(13), Some(12))?,
        ];

        p.write(&documents, &[], ConflictStrategy::Error).await?;

        let min_snapshot_ts = unchecked_repeatable_ts(Timestamp::must(11));

        let reader = p.reader();

        let scanned_stream = LeaderRetentionManager::<TestRuntime>::expired_documents(
            &rt,
            reader,
            RepeatableTimestamp::MIN,
            min_snapshot_ts,
        );
        let scanned: Vec<_> = scanned_stream.try_collect().await?;
        let expired: Vec<_> = scanned
            .into_iter()
            .filter_map(|doc| {
                if doc.1.is_some() {
                    Some((doc.0, doc.1.unwrap()))
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(expired.len(), 9);
        let results = try_join_all(
            LeaderRetentionManager::<TestRuntime>::partition_document_chunk(expired)
                .into_iter()
                .map(|delete_chunk| {
                    // Ensures that all documents with the same id are in the same chunk
                    assert!(delete_chunk.is_empty() || delete_chunk.len() == 9);
                    LeaderRetentionManager::<TestRuntime>::delete_document_chunk(
                        delete_chunk,
                        p.clone(),
                        *min_snapshot_ts,
                    )
                }),
        )
        .await?;
        let (_, deleted_rows): (Vec<_>, Vec<_>) = results.into_iter().unzip();
        let deleted_rows = deleted_rows.into_iter().sum::<usize>();
        assert_eq!(deleted_rows, 9);

        let reader = p.reader();

        // All documents are still visible at snapshot ts=12.
        let stream = reader.load_all_documents();
        let results: Vec<_> = stream.try_collect::<Vec<_>>().await?.into_iter().collect();
        assert_eq!(
            results,
            vec![
                doc(id1, 10, Some(10), Some(9))?,
                doc(id1, 12, Some(12), Some(10))?,
                doc(id1, 13, Some(13), Some(12))?,
            ]
        );

        Ok(())
    }
}
