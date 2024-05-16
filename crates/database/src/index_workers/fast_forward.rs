use std::{
    cmp::max,
    collections::BTreeSet,
    future::Future,
    time::Duration,
};

use async_trait::async_trait;
use common::{
    bootstrap_model::index::{
        search_index::TextSnapshotVersion,
        IndexConfig,
        TabletIndexMetadata,
    },
    document::ParsedDocument,
    interval::{
        BinaryKey,
        Interval,
    },
    knobs::{
        DATABASE_WORKERS_MAX_CHECKPOINT_AGE,
        DATABASE_WORKERS_MIN_COMMITS,
        DATABASE_WORKERS_POLL_INTERVAL,
    },
    persistence::PersistenceSnapshot,
    query::Order,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    types::{
        GenericIndexName,
        IndexId,
        TabletIndexName,
    },
};
use futures::TryStreamExt;
use indexing::index_registry::IndexRegistry;
use keybroker::Identity;
use sync_types::{
    backoff::Backoff,
    Timestamp,
};
use value::{
    values_to_bytes,
    ConvexValue,
    ResolvedDocumentId,
    TableMapping,
    TabletId,
};

use super::retriable_worker::retry_loop_expect_occs_and_overloaded;
use crate::{
    bootstrap_model::index_workers::{
        IndexWorkerMetadataModel,
        IndexWorkerMetadataRecord,
    },
    index_workers::{
        retriable_worker::RetriableWorker,
        timeout_with_jitter,
    },
    metrics::log_worker_starting,
    text_index_worker::fast_forward::TextFastForward,
    vector_index_worker::fast_forward::VectorFastForward,
    Database,
    IndexModel,
    Snapshot,
    SystemMetadataModel,
    Transaction,
    INDEX_DOC_ID_INDEX,
    INDEX_WORKER_METADATA_TABLE,
};

pub struct LastFastForwardInfo {
    ts: UnixTimestamp,
    observed_commits: usize,
}

#[async_trait]
impl<RT: Runtime> RetriableWorker<RT> for FastForwardIndexWorker {
    async fn work_loop(
        &mut self,
        _name: &'static str,
        rt: &RT,
        db: &Database<RT>,
        backoff: &mut Backoff,
    ) -> anyhow::Result<()> {
        self.fast_forward_loop(rt, db, backoff).await
    }
}

pub struct FastForwardIndexWorker;

impl FastForwardIndexWorker {
    pub fn create_and_start<RT: Runtime>(
        rt: RT,
        db: Database<RT>,
    ) -> impl Future<Output = ()> + Send {
        retry_loop_expect_occs_and_overloaded(
            "FastForwardWorker",
            rt,
            db,
            Duration::ZERO,
            FastForwardIndexWorker,
        )
    }

    async fn fast_forward_loop<RT: Runtime>(
        &self,
        rt: &RT,
        db: &Database<RT>,
        backoff: &mut Backoff,
    ) -> anyhow::Result<()> {
        // The `Database` records how many commits it has processed since startup. We
        // record the last known number to avoid fast-forwarding when no commits
        // have actually happened and only the latest timestamp has moved
        // forward. Because other workers make periodic commits, we fudge this
        // by a fairly large number. To ensure we don't go too long without fast
        // forwarding when a small number of commits is present, we also include
        // the timestamp when we last fast forwarded.
        let mut text_search_last_fast_forward_info: Option<LastFastForwardInfo> = None;
        let mut vector_search_last_fast_forward_info: Option<LastFastForwardInfo> = None;

        loop {
            let status = log_worker_starting("TextSearchFastForward");
            tracing::debug!("FastForwardWorker checking if we can fast forward");
            Self::fast_forward::<RT, TextSnapshotVersion, TextFastForward>(
                "TextSearch",
                rt,
                db,
                &mut text_search_last_fast_forward_info,
            )
            .await?;
            drop(status);
            let status = log_worker_starting("VectorSearchFastForward");
            Self::fast_forward::<RT, (), VectorFastForward>(
                "VectorSearch",
                rt,
                db,
                &mut vector_search_last_fast_forward_info,
            )
            .await?;
            drop(status);

            backoff.reset();
            timeout_with_jitter(rt, *DATABASE_WORKERS_POLL_INTERVAL).await
        }
    }

    /// Fast-forward search indexes, bumping timestamps for backfilled indexes
    /// that haven't had any writes to their tables. Conveniently, we can
    /// know if an indexed table has had any writes consistently by asking
    /// the memory index, which is guaranteed to have all revisions past our
    /// checkpoint. If the index reports that it doesn't have any subsequent
    /// transactions, we're safe to fast-forward.
    ///
    /// This method internally debounces, so it's safe to call it frequently.
    ///
    /// Returns the set of indexes fast-forwarded.
    pub(crate) async fn fast_forward<
        RT: Runtime,
        V: PartialEq + Send,
        Worker: IndexFastForward<RT, V>,
    >(
        log_name: &'static str,
        rt: &RT,
        database: &Database<RT>,
        last_fast_forward_info: &mut Option<LastFastForwardInfo>,
    ) -> anyhow::Result<BTreeSet<TabletIndexName>> {
        let mut indexes_fast_forwarded = BTreeSet::new();

        let commits_since_load = database.write_commits_since_load();
        let now = rt.unix_timestamp();
        // If we haven't observed any commits yet, we should fast forward. Otherwise a
        // restart looping backend could accumulate N-1 changes per restart,
        // leading to us having to read an unbounded amount of data during
        // startup.
        if let Some(last_fast_forward_info) = last_fast_forward_info
            && commits_since_load - last_fast_forward_info.observed_commits
                < *DATABASE_WORKERS_MIN_COMMITS
            && now - last_fast_forward_info.ts < *DATABASE_WORKERS_MAX_CHECKPOINT_AGE
        {
            tracing::debug!(
                "{log_name} not enough commits and too recent to fast forward: {}, {:?}",
                commits_since_load - last_fast_forward_info.observed_commits,
                now - last_fast_forward_info.ts
            );
            return Ok(indexes_fast_forwarded);
        }

        let mut tx = database.begin(Identity::system()).await?;
        let fast_forward_ts = *tx.begin_timestamp();
        let expected_version = Worker::current_version(&mut tx);

        for index_doc in IndexModel::new(&mut tx).get_all_indexes().await? {
            let (index_id, index_metadata) = index_doc.into_id_and_value();
            let internal_id = index_id.internal_id();

            let TabletIndexMetadata { name, config } = index_metadata;

            let Some((ts, version)) = Worker::snapshot_info(&config) else {
                continue;
            };

            // We can't fast-forward if we're on the wrong version.
            if version != expected_version {
                continue;
            }
            // If the index isn't ready in memory, we can't fast-forward it.
            let Some(num_transactions) =
                Worker::num_transactions(database.snapshot(tx.begin_timestamp())?, internal_id)?
            else {
                continue;
            };

            // If the memory index contains any transactions, we need to write a new
            // checkpoint and can't fast-forward.
            if num_transactions > 0 {
                tracing::debug!("{log_name}: Memory index is not empty: {num_transactions}");
                continue;
            }

            let index_worker_meta = Worker::get_or_create_worker_meta(
                IndexWorkerMetadataModel::new(&mut tx),
                index_id.internal_id(),
            )
            .await?;

            let (worker_meta_doc_id, mut worker_meta) = index_worker_meta.into_id_and_value();
            let previous_fast_forward_ts = worker_meta.index_metadata.mut_fast_forward_ts();
            let ts = max(ts, *previous_fast_forward_ts);

            // Okay, we're good! Just update the timestamp and update the system record.
            tracing::info!("Fast-forwarding {name} from {ts} to {fast_forward_ts}");
            *previous_fast_forward_ts = fast_forward_ts;

            SystemMetadataModel::new(&mut tx)
                .replace(worker_meta_doc_id, worker_meta.try_into()?)
                .await?;

            indexes_fast_forwarded.insert(name);
        }
        database
            .commit_with_write_source(tx, "index_worker_commit_ff")
            .await?;

        *last_fast_forward_info = Some(LastFastForwardInfo {
            ts: now,
            observed_commits: commits_since_load,
        });

        Ok(indexes_fast_forwarded)
    }
}

#[async_trait]
pub trait IndexFastForward<RT: Runtime, V: PartialEq + Send> {
    fn current_version(tx: &mut Transaction<RT>) -> V;
    fn snapshot_info(config: &IndexConfig) -> Option<(Timestamp, V)>;
    async fn get_or_create_worker_meta(
        model: IndexWorkerMetadataModel<'_, RT>,
        index_id: IndexId,
    ) -> anyhow::Result<ParsedDocument<IndexWorkerMetadataRecord>>;
    fn num_transactions(snapshot: Snapshot, index_id: IndexId) -> anyhow::Result<Option<usize>>;
}

pub async fn load_metadata_fast_forward_ts(
    registry: &IndexRegistry,
    snapshot: &PersistenceSnapshot,
    table_mapping: &TableMapping,
    index: ResolvedDocumentId,
) -> anyhow::Result<Option<Timestamp>> {
    let metadata_table_id = table_mapping.id(&INDEX_WORKER_METADATA_TABLE)?;
    let metadata_index_id = (*INDEX_DOC_ID_INDEX)
        .clone()
        .map_table(&table_mapping.name_to_id())?;
    let metadata_index_id: GenericIndexName<TabletId> = metadata_index_id.into();
    let metadata_index_internal_id = registry.get_enabled(&metadata_index_id).unwrap().id();

    let id_value = ConvexValue::String(index.internal_id().to_string().try_into()?);
    let id_value_bytes = values_to_bytes(&[Some(id_value)]);
    let interval = Interval::prefix(BinaryKey::from(id_value_bytes));

    let stream = snapshot.index_scan(
        metadata_index_internal_id,
        metadata_table_id.tablet_id,
        &interval,
        Order::Asc,
        100,
    );
    let mut results: Vec<_> = stream
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .map(|(_, _, doc)| doc)
        .collect();
    let fast_forward_ts = if !results.is_empty() {
        let mut doc = ParsedDocument::<IndexWorkerMetadataRecord>::try_from(results.remove(0))?;
        // This defaults to Timestamp(0) if a document isn't present, which is fine for
        // our purpose
        Some(*doc.index_metadata.mut_fast_forward_ts())
    } else {
        None
    };
    Ok(fast_forward_ts)
}
