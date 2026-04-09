use std::{
    cmp,
    collections::BTreeMap,
    fmt,
    sync::Arc,
};

use anyhow::Context as _;
use common::{
    bootstrap_model::tables::{
        TableMetadata,
        TableState,
    },
    document::{
        ParseDocument,
        ParsedDocument,
    },
    json::JsonForm,
    persistence::{
        new_static_repeatable_recent,
        LatestDocument,
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
    runtime::Runtime,
    types::{
        IndexId,
        RepeatableReason,
        RepeatableTimestamp,
        Timestamp,
    },
    value::{
        ConvexObject,
        JsonInteger,
        Size,
        TableMapping,
        TabletId,
    },
};
use errors::ErrorMetadata;
use futures::{
    Stream,
    TryStreamExt,
};
use serde::Deserialize;
use serde_json::{
    json,
    Value as JsonValue,
};
use shape_inference::{
    CountedShape,
    ProdConfig,
    Shape,
    ShapeEnum,
};

use crate::{
    bootstrap_model::defaults::BootstrapTableIds,
    metrics,
    persistence_helpers::stream_transactions,
    Database,
    DatabaseSnapshot,
    TableIterator,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableSummary {
    inferred_type: CountedShape<ProdConfig>,
    total_size: u64,
}

impl fmt::Display for TableSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TableSummary {{ inferred_type: {}, total_size: {} }}",
            self.inferred_type, self.total_size
        )
    }
}

impl TableSummary {
    pub fn empty() -> Self {
        Self {
            inferred_type: Shape::empty(),
            total_size: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inferred_type.is_empty() && self.total_size == 0
    }

    pub fn total_size(&self) -> u64 {
        self.total_size
    }

    pub fn num_values(&self) -> u64 {
        *self.inferred_type.num_values()
    }

    pub fn inferred_type(&self) -> &CountedShape<ProdConfig> {
        &self.inferred_type
    }

    pub fn insert(&self, object: &ConvexObject) -> Self {
        let total_size = self.total_size + object.size() as u64;
        Self {
            inferred_type: self.inferred_type.insert(object),
            total_size,
        }
    }

    pub fn remove(&self, object: &ConvexObject) -> anyhow::Result<Self> {
        let size = object.size() as u64;
        Ok(Self {
            inferred_type: self.inferred_type.remove(object)?,
            total_size: self
                .total_size
                .checked_sub(size)
                .context("total_size went negative?")?,
        })
    }

    pub fn reset_shape(&mut self) {
        self.inferred_type = CountedShape::new(ShapeEnum::Unknown, self.num_values());
    }

    pub fn persistence_key() -> PersistenceGlobalKey {
        PersistenceGlobalKey::TableSummary
    }
}

impl From<&TableSummary> for JsonValue {
    fn from(summary: &TableSummary) -> Self {
        json!({
            "totalSize": JsonInteger::encode(summary.total_size as i64),
            "inferredTypeWithOptionalFields": JsonValue::from(&summary.inferred_type)
        })
    }
}

impl TryFrom<JsonValue> for TableSummary {
    type Error = anyhow::Error;

    fn try_from(json_value: JsonValue) -> anyhow::Result<Self> {
        match json_value {
            JsonValue::Object(mut v) => {
                let total_size = match v.remove("totalSize") {
                    Some(JsonValue::String(s)) => JsonInteger::decode(s)? as u64,
                    _ => anyhow::bail!("Invalid totalSize"),
                };
                anyhow::ensure!(total_size >= 0);
                let inferred_type = match v.remove("inferredTypeWithOptionalFields") {
                    Some(v) => CountedShape::<ProdConfig>::json_deserialize_value(v)?,
                    None => anyhow::bail!("Missing field inferredTypeWithOptionalFields"),
                };
                Ok(TableSummary {
                    inferred_type,
                    total_size,
                })
            },
            _ => anyhow::bail!("Wrong type of json value for TableSummaryJson"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableSummarySnapshot {
    pub tables: BTreeMap<TabletId, TableSummary>,
    pub ts: Timestamp,
}

impl TableSummarySnapshot {
    pub async fn load(
        reader: &dyn PersistenceReader,
    ) -> anyhow::Result<Option<(Self, RepeatableTimestamp)>> {
        let Some(value) = reader
            .get_persistence_global(TableSummary::persistence_key())
            .await?
        else {
            return Ok(None);
        };

        let summary_snapshot = Self::try_from(value)?;
        let ts = RepeatableTimestamp::new_validated(
            summary_snapshot.ts,
            RepeatableReason::TableSummarySnapshot,
        );
        Ok(Some((summary_snapshot, ts)))
    }
}

impl From<&TableSummarySnapshot> for JsonValue {
    fn from(snapshot: &TableSummarySnapshot) -> Self {
        json!({
            "tables": snapshot.tables
                .iter()
                .map(|(k, v)| (k.to_string(), JsonValue::from(v)))
                .collect::<serde_json::Map<String, JsonValue>>(),
            "ts": JsonInteger::encode(snapshot.ts.into()),
        })
    }
}

impl TryFrom<JsonValue> for TableSummarySnapshot {
    type Error = anyhow::Error;

    fn try_from(json_value: JsonValue) -> anyhow::Result<Self> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct TableSummarySnapshotJson {
            tables: serde_json::Map<String, JsonValue>,
            ts: String,
        }
        let snapshot: TableSummarySnapshotJson = serde_json::from_value(json_value)?;
        Ok(TableSummarySnapshot {
            tables: snapshot
                .tables
                .into_iter()
                .map(|(k, v)| {
                    let table_name = k.parse()?;
                    let summary = TableSummary::try_from(v)?;
                    Ok((table_name, summary))
                })
                .collect::<anyhow::Result<_>>()?,
            ts: JsonInteger::decode(snapshot.ts)?.try_into()?,
        })
    }
}

pub struct TableSummaryWriter<RT: Runtime> {
    persistence: Arc<dyn Persistence>,
    database: Database<RT>,
    retention_validator: Arc<dyn RetentionValidator>,
}

impl<RT: Runtime> TableSummaryWriter<RT> {
    pub fn new(
        runtime: RT,
        persistence: Arc<dyn Persistence>,
        database: Database<RT>,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> Self {
        Self::new_with_config(runtime, persistence, database, retention_validator)
    }

    pub fn new_with_config(
        _runtime: RT,
        persistence: Arc<dyn Persistence>,
        database: Database<RT>,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> Self {
        Self {
            persistence,
            database,
            retention_validator,
        }
    }

    pub async fn collect_snapshot(
        // table_iterator, table_mapping, and by_id_indexes should all be
        // computed at the same snapshot.
        snapshot_ts: Timestamp,
        table_iterator: impl Fn() -> TableIterator<RT>,
        table_mapping: &TableMapping,
        by_id_indexes: &BTreeMap<TabletId, IndexId>,
    ) -> anyhow::Result<TableSummarySnapshot> {
        let mut snapshot = BTreeMap::new();
        for (tablet_id, ..) in table_mapping.iter() {
            let by_id_index = by_id_indexes.get(&tablet_id).expect("by_id should exist");
            // table_iterator, table_mapping, and by_id_indexes should all be
            // computed at the same snapshot.
            let revision_stream =
                table_iterator().stream_documents_in_table(tablet_id, *by_id_index, None);
            let summary = Self::collect_table_revisions(revision_stream).await?;
            snapshot.insert(tablet_id, summary);
        }
        Ok(TableSummarySnapshot {
            tables: snapshot,
            ts: snapshot_ts,
        })
    }

    pub async fn collect_table_revisions(
        revision_stream: impl Stream<Item = anyhow::Result<LatestDocument>>,
    ) -> anyhow::Result<TableSummary> {
        futures::pin_mut!(revision_stream);
        let mut summary = TableSummary::empty();
        while let Some(rev) = revision_stream.try_next().await? {
            summary = summary.insert(rev.value.value());
            let num_values = summary.inferred_type.num_values();
            if num_values % 10000 == 0 {
                tracing::info!("Collecting table summary with {num_values} documents")
            }
        }
        Ok(summary)
    }

    pub async fn compute_from_last_checkpoint(&self) -> anyhow::Result<TableSummarySnapshot> {
        self.compute(BootstrapKind::FromCheckpoint).await
    }

    pub async fn compute_from_scratch(&self) -> anyhow::Result<TableSummarySnapshot> {
        self.compute(BootstrapKind::FromScratch).await
    }

    async fn compute(&self, bootstrap_kind: BootstrapKind) -> anyhow::Result<TableSummarySnapshot> {
        let reader = self.persistence.reader();
        let upper_bound = self.database.now_ts_for_reads();
        let (new_snapshot, _) = bootstrap(
            self.database.runtime().clone(),
            reader,
            self.retention_validator.clone(),
            upper_bound,
            bootstrap_kind,
        )
        .await?;
        Ok(new_snapshot)
    }
}

pub async fn write_snapshot(
    persistence: &dyn Persistence,
    snapshot: &TableSummarySnapshot,
) -> anyhow::Result<()> {
    persistence
        .write_persistence_global(TableSummary::persistence_key(), JsonValue::from(snapshot))
        .await
}

pub enum BootstrapKind {
    FromScratch,
    FromCheckpoint,
}

pub fn table_summary_bootstrapping_error(msg: Option<&'static str>) -> anyhow::Error {
    anyhow::anyhow!(ErrorMetadata::feature_temporarily_unavailable(
        "TableSummariesUnavailable",
        msg.unwrap_or("Table summary unavailable (still bootstrapping)")
    ))
}

/// Compute a `TableSummarySnapshot` at a given timestamp.
/// If there is no stored snapshot or `from_scratch` is true, we recompute
/// by walking by_id indexes using TableIterator.
/// If the snapshot is <target_ts, we walk the documents log forwards and add
/// the documents to the snapshot.
/// If the snapshot is >target_ts, we walk the documents log backwards and
/// remove the documents from the snapshot.
///
/// Returns:
/// * The new table summary snapshot
/// * The number of log entries processed
pub async fn bootstrap<RT: Runtime>(
    runtime: RT,
    persistence: Arc<dyn PersistenceReader>,
    retention_validator: Arc<dyn RetentionValidator>,
    target_ts: RepeatableTimestamp,
    bootstrap_kind: BootstrapKind,
) -> anyhow::Result<(TableSummarySnapshot, usize)> {
    let _timer = metrics::bootstrap_table_summaries_timer();
    let stored_snapshot = match bootstrap_kind {
        BootstrapKind::FromScratch => None,
        BootstrapKind::FromCheckpoint => TableSummarySnapshot::load(persistence.as_ref()).await?,
    };
    let recent_ts = new_static_repeatable_recent(persistence.as_ref()).await?;
    let (table_mapping, _, index_registry, ..) =
        DatabaseSnapshot::<RT>::load_table_and_index_metadata(
            &RepeatablePersistence::new(
                persistence.clone(),
                recent_ts,
                retention_validator.clone(),
            )
            .read_snapshot(recent_ts)?,
        )
        .await?;
    let (base_snapshot, base_snapshot_ts) = match stored_snapshot {
        Some(base) => base,
        None => {
            let by_id_indexes = index_registry.by_id_indexes();
            let base_snapshot = TableSummaryWriter::<RT>::collect_snapshot(
                *recent_ts,
                || {
                    TableIterator::new(
                        runtime.clone(),
                        recent_ts,
                        persistence.clone(),
                        retention_validator.clone(),
                        1000,
                    )
                },
                &table_mapping,
                &by_id_indexes,
            )
            .await?;
            (base_snapshot, recent_ts)
        },
    };
    let bootstrap_tables = BootstrapTableIds::new(&table_mapping);
    let (range, order) = match base_snapshot_ts.cmp(&target_ts) {
        std::cmp::Ordering::Less => (
            TimestampRange::new(base_snapshot_ts.succ()?..=*target_ts),
            Order::Asc,
        ),
        std::cmp::Ordering::Equal => return Ok((base_snapshot, 0)),
        std::cmp::Ordering::Greater => (
            TimestampRange::new(target_ts.succ()?..=*base_snapshot_ts),
            Order::Desc,
        ),
    };
    let mut tables = base_snapshot.tables;
    let repeatable_persistence = RepeatablePersistence::new(
        persistence.clone(),
        cmp::max(base_snapshot_ts, target_ts),
        retention_validator.clone(),
    );
    let transaction_stream =
        stream_transactions(bootstrap_tables, &repeatable_persistence, range, order);
    futures::pin_mut!(transaction_stream);

    let mut num_added = 0;
    while let Some(transaction) = transaction_stream.try_next().await? {
        for revision_pair in transaction.revision_pairs {
            let revision_pair = match order {
                Order::Asc => revision_pair,
                Order::Desc => time_reverse_revision_pair(revision_pair),
            };
            add_revision(bootstrap_tables, &mut tables, &revision_pair)?;
            num_added += 1;
        }
    }
    let snapshot = TableSummarySnapshot {
        tables,
        ts: *target_ts,
    };
    Ok((snapshot, num_added))
}

fn time_reverse_revision_pair(revision_pair: RevisionPair) -> RevisionPair {
    let RevisionPair {
        id,
        rev: DocumentRevision { ts, document },
        prev_rev,
    } = revision_pair;
    RevisionPair {
        id,
        rev: DocumentRevision {
            ts,
            document: prev_rev.and_then(|rev| rev.document),
        },
        prev_rev: document.map(|doc| DocumentRevision {
            ts: Timestamp::MAX, // we don't know when the current revision was/will be changed
            document: Some(doc),
        }),
    }
}

fn add_revision(
    table_mapping: BootstrapTableIds,
    tables: &mut BTreeMap<TabletId, TableSummary>,
    revision_pair: &RevisionPair,
) -> anyhow::Result<()> {
    // First, create tables for all new tables within the transaction.
    // And delete tables dropped within the transaction.
    // Since our table metadata is fixed at `start_ts`, we know that all
    // subsequent table creations aren't in `snapshot` and must be
    // included.
    let tablet_id = TabletId(revision_pair.id.internal_id());
    if table_mapping.is_tables_table(revision_pair.id.table()) {
        match (revision_pair.prev_document(), revision_pair.document()) {
            (None, Some(_)) => {
                // Table creation creates a TableSummary::empty, if none exists.
                // In historical instances, some _tables rows were created after the records for
                // that table had been inserted.
                tables.entry(tablet_id).or_insert_with(TableSummary::empty);
            },
            (Some(_), None) => {
                // Table deletion removes table summary.
                tables.remove(&tablet_id);
            },
            _ => {},
        }
        if let Some(new_doc) = revision_pair.document() {
            let table_metadata: ParsedDocument<TableMetadata> = new_doc.parse()?;
            if table_metadata.state == TableState::Deleting {
                // Hax alert! Remove shape tracking from soft-deleted tables'
                // summaries, to prevent old shapes from filling up the overall
                // table summary object.
                // It's not correct to remove the summary entry entirely because
                // we still want to be able to rewind through this revision.
                if let Some(summary) = tables.get_mut(&tablet_id) {
                    summary.reset_shape();
                }
            }
        }
    }
    let id = &revision_pair.id;
    let summary = tables.entry(id.table()).or_insert_with(
        // In historical instances, some rows were created before their corresponding
        // `_table` row.
        TableSummary::empty,
    );
    if let Some(old_document) = revision_pair.prev_document() {
        *summary = summary.remove(old_document.value())?;
    }
    if let Some(new_document) = revision_pair.document() {
        *summary = summary.insert(new_document.value());
    }
    Ok(())
}
