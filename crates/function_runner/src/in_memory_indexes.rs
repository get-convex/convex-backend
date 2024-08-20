use std::{
    cmp,
    collections::BTreeMap,
    sync::Arc,
};

use anyhow::Context;
use async_lru::async_lru::{
    AsyncLru,
    SizedValue,
};
use async_trait::async_trait;
use common::{
    bootstrap_model::{
        index::INDEX_TABLE,
        tables::TABLES_TABLE,
    },
    document::{
        CreationTime,
        PackedDocument,
        ResolvedDocument,
    },
    index::IndexKeyBytes,
    interval::Interval,
    knobs::{
        FUNRUN_INDEX_CACHE_CONCURRENCY,
        FUNRUN_INDEX_CACHE_SIZE,
    },
    persistence::{
        PersistenceReader,
        PersistenceSnapshot,
        RepeatablePersistence,
        RetentionValidator,
    },
    query::Order,
    runtime::Runtime,
    types::{
        IndexId,
        RepeatableTimestamp,
    },
    virtual_system_mapping::VirtualSystemMapping,
};
use database::{
    BootstrapMetadata,
    DatabaseSnapshot,
    TableCountSnapshot,
    TableRegistry,
    Transaction,
    TransactionIdGenerator,
    TransactionIndex,
    TransactionTextSnapshot,
};
use futures::{
    FutureExt,
    TryStreamExt,
};
use indexing::{
    backend_in_memory_indexes::{
        DatabaseIndexSnapshot,
        InMemoryIndexes,
    },
    index_registry::IndexRegistry,
};
use keybroker::Identity;
use model::virtual_system_mapping;
use sync_types::Timestamp;
use usage_tracking::FunctionUsageTracker;
use value::{
    heap_size::{
        HeapSize,
        WithHeapSize,
    },
    InternalId,
    TableName,
    TabletId,
};

use super::metrics::{
    load_index_timer,
    log_funrun_index_cache_get,
    log_funrun_index_load_rows,
};
use crate::{
    metrics::begin_tx_timer,
    FunctionWrites,
};

fn make_transaction<RT: Runtime>(
    ts: RepeatableTimestamp,
    identity: Identity,
    existing_writes: FunctionWrites,
    rt: RT,
    table_registry: TableRegistry,
    index_registry: IndexRegistry,
    table_count_snapshot: Arc<dyn TableCountSnapshot>,
    database_index_snapshot: DatabaseIndexSnapshot,
    text_index_snapshot: Arc<dyn TransactionTextSnapshot>,
    retention_validator: Arc<dyn RetentionValidator>,
    virtual_system_mapping: VirtualSystemMapping,
    usage_tracker: FunctionUsageTracker,
) -> anyhow::Result<Transaction<RT>> {
    let id_generator = TransactionIdGenerator::new(&rt)?;
    // The transaction timestamp might be few minutes behind if the backend
    // has been idle. Make sure creation time is always recent. Existing writes to
    // the transaction will advance next_creation_time in `merge_writes` below.
    let creation_time = CreationTime::try_from(cmp::max(*ts, rt.generate_timestamp()?))?;
    let transaction_index =
        TransactionIndex::new(index_registry, database_index_snapshot, text_index_snapshot);
    let mut tx = Transaction::new(
        identity,
        id_generator,
        creation_time,
        transaction_index,
        table_registry,
        table_count_snapshot,
        rt.clone(),
        usage_tracker,
        retention_validator,
        virtual_system_mapping,
    );
    tx.merge_writes(existing_writes.updates)?;
    Ok(tx)
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct CacheKey {
    instance_name: String,
    index_id: InternalId,
    last_modified: Timestamp,
}

/// The cache value is the same as [DatabaseIndexMap] apart from keeping track
/// of last modified timestamps. The [OrdMap] keys are the index keys.
#[derive(Clone)]
struct CacheValue(WithHeapSize<BTreeMap<Vec<u8>, (Timestamp, PackedDocument)>>);

impl SizedValue for CacheValue {
    fn size(&self) -> u64 {
        self.0.heap_size() as u64
    }
}

#[derive(Clone)]
pub(crate) struct InMemoryIndexCache<RT: Runtime> {
    cache: AsyncLru<RT, CacheKey, CacheValue>,
    rt: RT,
}

#[minitrace::trace]
async fn load_index(
    instance_name: String,
    index_id: IndexId,
    persistence_snapshot: PersistenceSnapshot,
    tablet_id: TabletId,
    table_name: String,
) -> anyhow::Result<CacheValue> {
    let _timer = load_index_timer(&table_name, &instance_name);
    let index_map: BTreeMap<Vec<u8>, (Timestamp, PackedDocument)> = persistence_snapshot
        .index_scan(
            index_id,
            tablet_id,
            &Interval::all(),
            Order::Asc,
            usize::MAX,
        )
        .map_ok(|(key, ts, doc)| (key.0, (ts, PackedDocument::pack(doc))))
        .try_collect()
        .await?;
    log_funrun_index_load_rows(index_map.len() as u64, &table_name, &instance_name);
    Ok(CacheValue(index_map.into()))
}

impl<RT: Runtime> InMemoryIndexCache<RT> {
    pub fn new(rt: RT) -> Self {
        Self {
            cache: AsyncLru::new(
                rt.clone(),
                *FUNRUN_INDEX_CACHE_SIZE,
                *FUNRUN_INDEX_CACHE_CONCURRENCY,
                "funrun_index_cache",
            ),
            rt,
        }
    }

    /// Get the index from the cache or load it from persistence and put it in
    /// the cache. If the index is not in the last_modified map, it is not an
    /// in-memory index and should not be cached.
    async fn get_or_load(
        &self,
        instance_name: String,
        index_id: IndexId,
        last_modified: &BTreeMap<IndexId, Timestamp>,
        persistence_snapshot: PersistenceSnapshot,
        tablet_id: TabletId,
        table_name: TableName,
    ) -> anyhow::Result<Option<CacheValue>> {
        let Some(key) = last_modified.get(&index_id).map(|ts| CacheKey {
            instance_name: instance_name.clone(),
            index_id,
            last_modified: *ts,
        }) else {
            return Ok(None);
        };
        let table_name = table_name.to_string();
        let cache_value_result = self
            .cache
            .get(
                key,
                load_index(
                    instance_name.clone(),
                    index_id,
                    persistence_snapshot,
                    tablet_id,
                    table_name.clone(),
                )
                .boxed(),
            )
            .await
            .map(|cache_value| Some(Arc::unwrap_or_clone(cache_value)));
        log_funrun_index_cache_get(&table_name, &instance_name);
        cache_value_result
    }

    pub async fn must_get_or_load_unpacked(
        &self,
        instance_name: String,
        index_id: IndexId,
        last_modified: &BTreeMap<IndexId, Timestamp>,
        persistence_snapshot: PersistenceSnapshot,
        tablet_id: TabletId,
        table_name: TableName,
    ) -> anyhow::Result<impl Iterator<Item = ResolvedDocument>> {
        let index_map = self
            .get_or_load(
                instance_name.clone(),
                index_id,
                last_modified,
                persistence_snapshot,
                tablet_id,
                table_name.clone(),
            )
            .await?
            .with_context(|| format!("Index on {table_name} for {instance_name} not found"))?;
        Ok(index_map.0.into_iter().map(|(_k, (_ts, v))| v.unpack()))
    }

    async fn load_table_and_index_registry(
        &self,
        persistence_snapshot: PersistenceSnapshot,
        instance_name: String,
        in_memory_index_last_modified: BTreeMap<IndexId, Timestamp>,
        BootstrapMetadata {
            tables_by_id,
            index_by_id,
            tables_tablet_id,
            index_tablet_id,
        }: BootstrapMetadata,
    ) -> anyhow::Result<(TableRegistry, IndexRegistry, DatabaseIndexSnapshot)> {
        let index_documents_fut = self.must_get_or_load_unpacked(
            instance_name.clone(),
            index_by_id,
            &in_memory_index_last_modified,
            persistence_snapshot.clone(),
            index_tablet_id,
            INDEX_TABLE.clone(),
        );
        let table_documents_fut = self.must_get_or_load_unpacked(
            instance_name.clone(),
            tables_by_id,
            &in_memory_index_last_modified,
            persistence_snapshot.clone(),
            tables_tablet_id,
            TABLES_TABLE.clone(),
        );
        let (index_documents, table_documents) =
            futures::future::try_join(index_documents_fut, table_documents_fut).await?;
        let (table_mapping, table_states) = DatabaseSnapshot::<RT>::table_mapping_and_states(
            table_documents.map(|doc| doc.try_into()).try_collect()?,
        );
        let index_registry = IndexRegistry::bootstrap(
            &table_mapping,
            index_documents.collect::<Vec<_>>().iter(),
            persistence_snapshot.persistence().version(),
        )?;

        let table_registry = TableRegistry::bootstrap(
            table_mapping.clone(),
            table_states,
            persistence_snapshot.persistence().version(),
        )?;
        DatabaseSnapshot::<RT>::verify_invariants(&table_registry, &index_registry)?;
        let in_memory_indexes = FunctionRunnerInMemoryIndexes {
            cache: self.clone(),
            instance_name: instance_name.clone(),
            backend_last_modified: in_memory_index_last_modified,
            persistence_snapshot: persistence_snapshot.clone(),
        };
        let database_index_snapshot = DatabaseIndexSnapshot::new(
            index_registry.clone(),
            Arc::new(in_memory_indexes),
            table_mapping,
            persistence_snapshot,
        );
        Ok((table_registry, index_registry, database_index_snapshot))
    }

    /// Loads table and index registry from cache or persistence snapshot.
    #[minitrace::trace]
    pub(crate) async fn begin_tx(
        &self,
        identity: Identity,
        ts: RepeatableTimestamp,
        existing_writes: FunctionWrites,
        persistence: Arc<dyn PersistenceReader>,
        instance_name: String,
        in_memory_index_last_modified: BTreeMap<IndexId, Timestamp>,
        bootstrap_metadata: BootstrapMetadata,
        table_count_snapshot: Arc<dyn TableCountSnapshot>,
        text_index_snapshot: Arc<dyn TransactionTextSnapshot>,
        usage_tracker: FunctionUsageTracker,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> anyhow::Result<Transaction<RT>> {
        let _timer = begin_tx_timer();
        for (index_id, last_modified) in &in_memory_index_last_modified {
            anyhow::ensure!(
                *last_modified <= *ts,
                "Last modified timestamp {last_modified} for index {index_id} is ahead of \
                 transaction timestamp {}",
                *ts
            );
        }
        let repeatable_persistence =
            RepeatablePersistence::new(persistence.clone(), ts, retention_validator.clone());
        let persistence_snapshot =
            repeatable_persistence.read_snapshot(repeatable_persistence.upper_bound())?;

        let (table_registry, index_registry, database_index_snapshot) = self
            .load_table_and_index_registry(
                persistence_snapshot,
                instance_name,
                in_memory_index_last_modified,
                bootstrap_metadata,
            )
            .await?;
        make_transaction(
            ts,
            identity,
            existing_writes,
            self.rt.clone(),
            table_registry,
            index_registry,
            table_count_snapshot,
            database_index_snapshot,
            text_index_snapshot,
            retention_validator,
            virtual_system_mapping(),
            usage_tracker,
        )
    }
}

#[derive(Clone)]
pub(crate) struct FunctionRunnerInMemoryIndexes<RT: Runtime> {
    pub(crate) cache: InMemoryIndexCache<RT>,
    pub(crate) instance_name: String,
    /// The last modified timestamp for each index at the beginning of the
    /// Transaction.
    pub(crate) backend_last_modified: BTreeMap<IndexId, Timestamp>,
    pub(crate) persistence_snapshot: PersistenceSnapshot,
}

#[async_trait]
impl<RT: Runtime> InMemoryIndexes for FunctionRunnerInMemoryIndexes<RT> {
    async fn range(
        &self,
        index_id: IndexId,
        interval: &Interval,
        order: Order,
        tablet_id: TabletId,
        table_name: TableName,
    ) -> anyhow::Result<Option<Vec<(IndexKeyBytes, Timestamp, ResolvedDocument)>>> {
        let Some(index_map) = self
            .cache
            .get_or_load(
                self.instance_name.clone(),
                index_id,
                &self.backend_last_modified,
                self.persistence_snapshot.clone(),
                tablet_id,
                table_name,
            )
            .await?
        else {
            return Ok(None);
        };
        let range = order
            .apply(
                index_map
                    .0
                    .range(interval)
                    .map(|(k, (ts, v))| (IndexKeyBytes(k.clone()), *ts, v.unpack())),
            )
            .collect::<Vec<(IndexKeyBytes, Timestamp, ResolvedDocument)>>();
        Ok(Some(range))
    }
}
