use std::{
    cmp,
    collections::BTreeMap,
    sync::Arc,
};

use anyhow::Context;
use async_lru::{
    async_lru::SizedValue,
    multi_type_async_lru::{
        LruKey,
        MultiTypeAsyncLru,
    },
};
use async_trait::async_trait;
use common::{
    document::{
        CreationTime,
        PackedDocument,
        ParseDocument,
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
    ComponentRegistry,
    DatabaseSnapshot,
    SchemaRegistry,
    TableCountSnapshot,
    TableRegistry,
    Transaction,
    TransactionIdGenerator,
    TransactionIndex,
    TransactionReadSet,
    TransactionTextSnapshot,
    COMPONENTS_TABLE,
    SCHEMAS_TABLE,
};
use futures::{
    FutureExt,
    TryStreamExt,
};
use indexing::{
    backend_in_memory_indexes::{
        DatabaseIndexSnapshot,
        InMemoryIndexes,
        LazyDocument,
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
    TableNamespace,
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
    schema_registry: SchemaRegistry,
    component_registry: ComponentRegistry,
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
        schema_registry,
        component_registry,
        table_count_snapshot,
        rt,
        usage_tracker,
        retention_validator,
        virtual_system_mapping,
    );
    tx.merge_writes(existing_writes.updates)?;
    Ok(tx)
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct IndexCacheKey {
    instance_name: String,
    index_id: InternalId,
    last_modified: Timestamp,
}

impl LruKey for IndexCacheKey {
    type Value = IndexCacheValue;
}

/// The cache value is the same as [DatabaseIndexMap] apart from keeping track
/// of last modified timestamps. The [BTreeMap] keys are the index keys.
#[derive(Clone)]
struct IndexCacheValue(WithHeapSize<BTreeMap<Vec<u8>, (Timestamp, PackedDocument)>>);

impl SizedValue for IndexCacheValue {
    fn size(&self) -> u64 {
        self.0.heap_size() as u64
    }
}

#[derive(Clone)]
pub(crate) struct InMemoryIndexCache<RT: Runtime> {
    cache: MultiTypeAsyncLru<RT>,
    rt: RT,
}

#[fastrace::trace]
async fn load_index(
    instance_name: String,
    index_id: IndexId,
    persistence_snapshot: PersistenceSnapshot,
    tablet_id: TabletId,
    table_name: String,
) -> anyhow::Result<Arc<IndexCacheValue>> {
    let _timer = load_index_timer(&table_name, &instance_name);
    let index_map: BTreeMap<Vec<u8>, (Timestamp, PackedDocument)> = persistence_snapshot
        .index_scan(
            index_id,
            tablet_id,
            &Interval::all(),
            Order::Asc,
            usize::MAX,
        )
        .map_ok(|(key, rev)| (key.0, (rev.ts, PackedDocument::pack(&rev.value))))
        .try_collect()
        .await?;
    log_funrun_index_load_rows(index_map.len() as u64, &table_name, &instance_name);
    Ok(Arc::new(IndexCacheValue(index_map.into())))
}

#[fastrace::trace]
async fn load_unpacked_index(
    instance_name: &str,
    index_id: IndexId,
    persistence_snapshot: &PersistenceSnapshot,
    tablet_id: TabletId,
    table_name: &str,
) -> anyhow::Result<(Vec<ResolvedDocument>, u64)> {
    let _timer = load_index_timer(table_name, instance_name);
    let documents: Vec<ResolvedDocument> = persistence_snapshot
        .index_scan(
            index_id,
            tablet_id,
            &Interval::all(),
            Order::Asc,
            usize::MAX,
        )
        .map_ok(|(_, rev)| rev.value)
        .try_collect()
        .await?;
    log_funrun_index_load_rows(documents.len() as u64, table_name, instance_name);
    let size = documents.iter().map(|d| d.size() as u64).sum();
    Ok((documents, size))
}

struct WithSize<T>(T, u64);
impl<T> SizedValue for WithSize<T> {
    fn size(&self) -> u64 {
        self.1
    }
}

impl<RT: Runtime> InMemoryIndexCache<RT> {
    pub fn new(rt: RT) -> Self {
        Self {
            cache: MultiTypeAsyncLru::new(
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
    #[fastrace::trace]
    async fn get_or_load(
        &self,
        instance_name: String,
        index_id: IndexId,
        in_memory_index_last_modified: &BTreeMap<IndexId, Timestamp>,
        persistence_snapshot: PersistenceSnapshot,
        tablet_id: TabletId,
        table_name: TableName,
    ) -> anyhow::Result<Option<Arc<IndexCacheValue>>> {
        let Some(key) = in_memory_index_last_modified
            .get(&index_id)
            .map(|ts| IndexCacheKey {
                instance_name: instance_name.clone(),
                index_id,
                last_modified: *ts,
            })
        else {
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
            .map(Some);
        log_funrun_index_cache_get(&table_name, &instance_name);
        cache_value_result
    }

    /// Returns the TableRegistry together with its last-modified time
    #[fastrace::trace]
    async fn load_table_registry(
        &self,
        instance_name: String,
        in_memory_index_last_modified: &BTreeMap<IndexId, Timestamp>,
        persistence_snapshot: PersistenceSnapshot,
        &BootstrapMetadata {
            tables_by_id,
            tables_tablet_id,
            ..
        }: &BootstrapMetadata,
    ) -> anyhow::Result<(Timestamp, TableRegistry)> {
        #[derive(Hash, PartialEq, Eq, Debug, Clone)]
        struct Key {
            instance_name: String,
            tables_last_modified: Timestamp,
        }
        impl LruKey for Key {
            type Value = WithSize<TableRegistry>;
        }
        let tables_last_modified = *in_memory_index_last_modified
            .get(&tables_by_id)
            .context("_tables not configured to be in-memory")?;
        const NAME: &str = "_table_registry";
        log_funrun_index_cache_get(NAME, &instance_name);
        let table_registry = self
            .cache
            .get(
                Key {
                    instance_name: instance_name.clone(),
                    tables_last_modified,
                },
                async move {
                    let (documents, size) = load_unpacked_index(
                        &instance_name,
                        tables_by_id,
                        &persistence_snapshot,
                        tables_tablet_id,
                        NAME,
                    )
                    .await?;
                    let (table_mapping, table_states) =
                        DatabaseSnapshot::<RT>::table_mapping_and_states(
                            documents.into_iter().map(|doc| doc.parse()).try_collect()?,
                        );
                    let registry = TableRegistry::bootstrap(
                        table_mapping,
                        table_states,
                        persistence_snapshot.persistence().version(),
                    )?;
                    // We don't have `HeapSize` implemented for `TableRegistry`
                    // so just approximate its size using the size of the
                    // documents it was made from.
                    Ok(WithSize(registry, size))
                }
                .boxed(),
            )
            .await?;
        Ok((tables_last_modified, table_registry.0.clone()))
    }

    #[fastrace::trace]
    async fn load_index_registry(
        &self,
        instance_name: String,
        in_memory_index_last_modified: &BTreeMap<IndexId, Timestamp>,
        table_registry: (Timestamp, TableRegistry),
        persistence_snapshot: PersistenceSnapshot,
        &BootstrapMetadata {
            index_by_id,
            index_tablet_id,
            ..
        }: &BootstrapMetadata,
    ) -> anyhow::Result<(Timestamp, IndexRegistry)> {
        #[derive(Hash, PartialEq, Eq, Debug, Clone)]
        struct Key {
            instance_name: String,
            last_modified: Timestamp,
        }
        impl LruKey for Key {
            type Value = WithSize<IndexRegistry>;
        }
        let indexes_last_modified = *in_memory_index_last_modified
            .get(&index_by_id)
            .context("_index not configured to be in-memory")?;
        const NAME: &str = "_index_registry";
        log_funrun_index_cache_get(NAME, &instance_name);
        let index_registry = self
            .cache
            .get(
                Key {
                    instance_name: instance_name.clone(),
                    // We use the max of the two timestamps as our cache key
                    // because it's "as if" `load_unpacked_index` is reading at
                    // that timestamp.
                    last_modified: table_registry.0.max(indexes_last_modified),
                },
                async move {
                    let (documents, size) = load_unpacked_index(
                        &instance_name,
                        index_by_id,
                        &persistence_snapshot,
                        index_tablet_id,
                        NAME,
                    )
                    .await?;
                    let index_registry = IndexRegistry::bootstrap(
                        table_registry.1.table_mapping(),
                        documents.into_iter(),
                        persistence_snapshot.persistence().version(),
                    )?;
                    DatabaseSnapshot::<RT>::verify_invariants(&table_registry.1, &index_registry)?;
                    Ok(WithSize(index_registry, size))
                }
                .boxed(),
            )
            .await?;
        Ok((indexes_last_modified, index_registry.0.clone()))
    }

    #[fastrace::trace]
    async fn load_component_registry(
        &self,
        instance_name: String,
        in_memory_index_last_modified: &BTreeMap<IndexId, Timestamp>,
        table_registry: (Timestamp, TableRegistry),
        index_registry: (Timestamp, IndexRegistry),
        persistence_snapshot: PersistenceSnapshot,
    ) -> anyhow::Result<(Timestamp, ComponentRegistry)> {
        #[derive(Hash, PartialEq, Eq, Debug, Clone)]
        struct Key {
            instance_name: String,
            last_modified: Timestamp,
        }
        impl LruKey for Key {
            type Value = WithSize<ComponentRegistry>;
        }
        let component_tablet_id = table_registry
            .1
            .table_mapping()
            .namespace(TableNamespace::Global)
            .id(&COMPONENTS_TABLE)?
            .tablet_id;
        let components_by_id = index_registry.1.must_get_by_id(component_tablet_id)?.id;
        let components_last_modified = *in_memory_index_last_modified
            .get(&components_by_id)
            .context("_components not configured to be in-memory")?;
        const NAME: &str = "_component_registry";
        log_funrun_index_cache_get(NAME, &instance_name);
        let component_registry = self
            .cache
            .get(
                Key {
                    instance_name: instance_name.clone(),
                    last_modified: table_registry
                        .0
                        .max(index_registry.0)
                        .max(components_last_modified),
                },
                async move {
                    let (documents, size) = load_unpacked_index(
                        &instance_name,
                        components_by_id,
                        &persistence_snapshot,
                        component_tablet_id,
                        NAME,
                    )
                    .await?;
                    let component_registry = ComponentRegistry::bootstrap(
                        table_registry.1.table_mapping(),
                        documents.into_iter().map(|d| d.parse()).try_collect()?,
                    )?;
                    Ok(WithSize(component_registry, size))
                }
                .boxed(),
            )
            .await?;
        Ok((components_last_modified, component_registry.0.clone()))
    }

    #[fastrace::trace]
    async fn load_schema_registry(
        &self,
        instance_name: String,
        in_memory_index_last_modified: &BTreeMap<IndexId, Timestamp>,
        table_registry: (Timestamp, TableRegistry),
        index_registry: (Timestamp, IndexRegistry),
        component_registry: (Timestamp, ComponentRegistry),
        persistence_snapshot: PersistenceSnapshot,
    ) -> anyhow::Result<(Timestamp, SchemaRegistry)> {
        #[derive(Hash, PartialEq, Eq, Debug, Clone)]
        struct Key {
            instance_name: String,
            last_modified: Timestamp,
        }
        impl LruKey for Key {
            type Value = WithSize<SchemaRegistry>;
        }
        let table_mapping = table_registry.1.table_mapping();
        // Each component's namespace has a _schemas table.
        // Note there may be _schemas table in other namespaces, but we don't care about
        // those (and also they're not necessarily loaded into memory yet).
        // This argument only applies because we're in the function runner, which
        // can only operate in components' namespaces -- internal database workers
        // like IndexWorker and SchemaWorker include schemas from all namespaces.
        let component_ids = component_registry
            .1
            .all_component_paths(&mut TransactionReadSet::new())
            .into_keys();
        let mut last_modified_ts = table_registry
            .0
            .max(index_registry.0)
            .max(component_registry.0);
        let mut schema_tables = vec![];
        const NAME: &str = "_schema_registry";
        for component_id in component_ids {
            let namespace = component_id.into();
            let schema_tablet =
                table_mapping.namespace(namespace).name_to_tablet()(SCHEMAS_TABLE.clone())?;
            let index_id = index_registry.1.must_get_by_id(schema_tablet)?.id;
            let schemas_last_modified = *in_memory_index_last_modified
                .get(&index_id)
                .context("_schemas not configured to be in-memory")?;
            last_modified_ts = last_modified_ts.max(schemas_last_modified);
            schema_tables.push((namespace, schema_tablet, index_id));
            log_funrun_index_cache_get(NAME, &instance_name);
        }
        let schema_registry = self
            .cache
            .get(
                Key {
                    instance_name: instance_name.clone(),
                    last_modified: last_modified_ts,
                },
                async move {
                    let mut size = 0;
                    let mut schema_docs = BTreeMap::new();
                    for (namespace, schema_tablet, index_id) in schema_tables {
                        let (component_documents, component_size) = load_unpacked_index(
                            &instance_name,
                            index_id,
                            &persistence_snapshot,
                            schema_tablet,
                            NAME,
                        )
                        .await?;
                        schema_docs.insert(
                            namespace,
                            component_documents
                                .into_iter()
                                .map(|d| d.parse())
                                .try_collect()?,
                        );
                        size += component_size;
                    }
                    let schema_registry = SchemaRegistry::bootstrap(schema_docs);
                    Ok(WithSize(schema_registry, size))
                }
                .boxed(),
            )
            .await?;
        Ok((last_modified_ts, schema_registry.0.clone()))
    }

    #[fastrace::trace]
    async fn load_registries(
        &self,
        persistence_snapshot: PersistenceSnapshot,
        instance_name: String,
        in_memory_index_last_modified: &BTreeMap<IndexId, Timestamp>,
        bootstrap_metadata: BootstrapMetadata,
    ) -> anyhow::Result<(
        TableRegistry,
        SchemaRegistry,
        ComponentRegistry,
        IndexRegistry,
    )> {
        // This is unfortunate but we need this cascade of cached lookups
        // because we don't know what cache keys to use until the previous
        // registry has been fetched.
        let table_registry = self
            .load_table_registry(
                instance_name.clone(),
                in_memory_index_last_modified,
                persistence_snapshot.clone(),
                &bootstrap_metadata,
            )
            .await?;
        let index_registry = self
            .load_index_registry(
                instance_name.clone(),
                in_memory_index_last_modified,
                table_registry.clone(),
                persistence_snapshot.clone(),
                &bootstrap_metadata,
            )
            .await?;
        let component_registry = self
            .load_component_registry(
                instance_name.clone(),
                in_memory_index_last_modified,
                table_registry.clone(),
                index_registry.clone(),
                persistence_snapshot.clone(),
            )
            .await?;
        let schema_registry = self
            .load_schema_registry(
                instance_name,
                in_memory_index_last_modified,
                table_registry.clone(),
                index_registry.clone(),
                component_registry.clone(),
                persistence_snapshot,
            )
            .await?;
        Ok((
            table_registry.1,
            schema_registry.1,
            component_registry.1,
            index_registry.1,
        ))
    }

    /// Loads table and index registry from cache or persistence snapshot.
    #[fastrace::trace]
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

        let (table_registry, schema_registry, component_registry, index_registry) = self
            .load_registries(
                persistence_snapshot.clone(),
                instance_name.clone(),
                &in_memory_index_last_modified,
                bootstrap_metadata,
            )
            .await?;

        let in_memory_indexes = FunctionRunnerInMemoryIndexes {
            cache: self.clone(),
            instance_name,
            backend_last_modified: in_memory_index_last_modified,
            persistence_snapshot: persistence_snapshot.clone(),
        };
        let database_index_snapshot = DatabaseIndexSnapshot::new(
            index_registry.clone(),
            Arc::new(in_memory_indexes),
            table_registry.table_mapping().clone(),
            persistence_snapshot,
        );

        make_transaction(
            ts,
            identity,
            existing_writes,
            self.rt.clone(),
            table_registry,
            schema_registry,
            component_registry,
            index_registry,
            table_count_snapshot,
            database_index_snapshot,
            text_index_snapshot,
            retention_validator,
            virtual_system_mapping().clone(),
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
    ) -> anyhow::Result<Option<Vec<(IndexKeyBytes, Timestamp, LazyDocument)>>> {
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
                    .map(|(k, (ts, v))| (IndexKeyBytes(k.clone()), *ts, v.clone().into())),
            )
            .collect::<Vec<(IndexKeyBytes, Timestamp, LazyDocument)>>();
        Ok(Some(range))
    }
}
