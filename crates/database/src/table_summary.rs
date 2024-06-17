use std::{
    cmp,
    collections::BTreeMap,
    fmt,
    sync::Arc,
};

#[cfg(any(test, feature = "testing"))]
use common::pause::PauseClient;
use common::{
    document::ResolvedDocument,
    persistence::{
        new_static_repeatable_recent,
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
use futures::{
    Stream,
    TryStreamExt,
};
#[cfg(any(test, feature = "testing"))]
use keybroker::Identity;
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use serde::Deserialize;
use serde_json::{
    json,
    Value as JsonValue,
};
use shape_inference::{
    CountedShape,
    ProdConfigWithOptionalFields,
    Shape,
    ShapeEnum,
};

#[cfg(any(test, feature = "testing"))]
use crate::IndexModel;
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
    inferred_type: CountedShape<ProdConfigWithOptionalFields>,
    total_size: i64,
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

    pub fn total_size(&self) -> usize {
        self.total_size as usize
    }

    pub fn num_values(&self) -> usize {
        *self.inferred_type.num_values() as usize
    }

    pub fn inferred_type(&self) -> &CountedShape<ProdConfigWithOptionalFields> {
        &self.inferred_type
    }

    pub fn insert(&self, object: &ConvexObject) -> Self {
        let total_size = self.total_size + object.size() as i64;
        Self {
            inferred_type: self.inferred_type.insert(object),
            total_size,
        }
    }

    pub fn remove(&self, object: &ConvexObject) -> anyhow::Result<Self> {
        let size = object.size() as i64;
        Ok(Self {
            inferred_type: self.inferred_type.remove(object)?,
            total_size: self.total_size - size,
        })
    }

    pub fn reset_shape(&mut self) {
        self.inferred_type = CountedShape::new(ShapeEnum::Unknown, self.num_values() as u64);
    }

    pub fn persistence_key() -> PersistenceGlobalKey {
        PersistenceGlobalKey::TableSummary
    }
}

impl From<&TableSummary> for JsonValue {
    fn from(summary: &TableSummary) -> Self {
        json!({
            "totalSize": JsonInteger::encode(summary.total_size),
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
                    Some(JsonValue::String(s)) => JsonInteger::decode(s)?,
                    _ => anyhow::bail!("Invalid totalSize"),
                };
                anyhow::ensure!(total_size >= 0);
                let inferred_type = match v.remove("inferredTypeWithOptionalFields") {
                    Some(v) => CountedShape::<ProdConfigWithOptionalFields>::try_from(v)?,
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

#[cfg(any(test, feature = "testing"))]
impl Arbitrary for TableSummary {
    type Parameters = ();

    type Strategy = impl Strategy<Value = TableSummary>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        let values = prop::collection::vec((any::<bool>(), any::<ConvexObject>()), 0..10);
        values.prop_map(|values| {
            let mut summary = TableSummary::empty();
            for (_, value) in values.iter() {
                summary = summary.insert(value);
            }
            for (deleted, value) in values.iter() {
                if *deleted {
                    summary = summary
                        .remove(value)
                        .expect("inserted value should be removable")
                }
            }
            summary
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableSummarySnapshot {
    pub tables: BTreeMap<TabletId, TableSummary>,
    pub ts: Timestamp,
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for TableSummarySnapshot {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = TableSummarySnapshot>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (
            any::<Timestamp>(),
            proptest::collection::btree_map(any::<TabletId>(), any::<TableSummary>(), 0..4),
        )
            .prop_map(|(ts, tables)| TableSummarySnapshot { tables, ts })
    }
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

    #[cfg(any(test, feature = "testing"))]
    pub async fn compute_snapshot(
        &self,
        pause_client: Option<PauseClient>,
        page_size: usize,
    ) -> anyhow::Result<TableSummarySnapshot> {
        let mut tx = self.database.begin(Identity::system()).await?;
        let start_ts = tx.begin_timestamp();
        let table_mapping = tx.table_mapping().clone();
        let by_id_indexes = IndexModel::new(&mut tx).by_id_indexes().await?;
        drop(tx);

        let snapshot_ts = self.database.now_ts_for_reads();

        let mut pause_client = pause_client.unwrap_or_default();
        pause_client.wait("table_summary_snapshot_picked").await;
        let database = self.database.clone();
        Self::collect_snapshot(
            *start_ts,
            move || database.table_iterator(snapshot_ts, page_size, None),
            &table_mapping,
            &by_id_indexes,
        )
        .await
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
        revision_stream: impl Stream<Item = anyhow::Result<(ResolvedDocument, Timestamp)>>,
    ) -> anyhow::Result<TableSummary> {
        futures::pin_mut!(revision_stream);
        let mut summary = TableSummary::empty();
        while let Some((document, _ts)) = revision_stream.try_next().await? {
            summary = summary.insert(document.value());
            let num_values = summary.inferred_type.num_values();
            if num_values % 10000 == 0 {
                tracing::info!("Collecting table summary with {num_values} documents")
            }
        }
        Ok(summary)
    }

    pub async fn compute_from_last_checkpoint(&self) -> anyhow::Result<TableSummarySnapshot> {
        self.compute(false).await
    }

    pub async fn compute_from_scratch(&self) -> anyhow::Result<TableSummarySnapshot> {
        self.compute(true).await
    }

    async fn compute(&self, from_scratch: bool) -> anyhow::Result<TableSummarySnapshot> {
        let reader = self.persistence.reader();
        let upper_bound = self.database.now_ts_for_reads();
        let (new_snapshot, _) = bootstrap(
            &self.database.runtime,
            reader,
            self.retention_validator.clone(),
            upper_bound,
            from_scratch,
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
    rt: &RT,
    persistence: Arc<dyn PersistenceReader>,
    retention_validator: Arc<dyn RetentionValidator>,
    target_ts: RepeatableTimestamp,
    from_scratch: bool,
) -> anyhow::Result<(TableSummarySnapshot, usize)> {
    let _timer = metrics::bootstrap_table_summaries_timer();
    let stored_snapshot = if from_scratch {
        None
    } else {
        TableSummarySnapshot::load(persistence.as_ref()).await?
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
            let base_snapshot = TableSummaryWriter::collect_snapshot(
                *recent_ts,
                || {
                    TableIterator::new(
                        rt.clone(),
                        recent_ts,
                        persistence.clone(),
                        retention_validator.clone(),
                        1000,
                        None,
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
            TimestampRange::new(base_snapshot_ts.succ()?..=*target_ts)?,
            Order::Asc,
        ),
        std::cmp::Ordering::Equal => return Ok((base_snapshot, 0)),
        std::cmp::Ordering::Greater => (
            TimestampRange::new(target_ts.succ()?..=*base_snapshot_ts)?,
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
    if table_mapping.is_tables_table(*revision_pair.id.table()) {
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
    }
    let id = &revision_pair.id;
    let summary = match tables.get_mut(id.table()) {
        Some(i) => i,
        None => {
            // In historical instances, some rows were created before their corresponding
            // `_table` row.
            tables.insert(*id.table(), TableSummary::empty());
            tables.get_mut(id.table()).unwrap()
        },
    };
    if let Some(old_document) = revision_pair.prev_document() {
        *summary = summary.remove(old_document.value())?;
    }
    if let Some(new_document) = revision_pair.document() {
        *summary = summary.insert(new_document.value());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        sync::Arc,
    };

    use cmd_util::env::env_config;
    use common::{
        persistence::NoopRetentionValidator,
        types::{
            unchecked_repeatable_ts,
            FieldName,
            TableName,
        },
        value::ConvexObject,
    };
    use keybroker::Identity;
    use prop::collection::vec as prop_vec;
    use proptest::prelude::*;
    use runtime::testing::{
        TestDriver,
        TestRuntime,
    };
    use serde_json::Value as JsonValue;
    use value::{
        assert_obj,
        resolved_object_strategy,
        resolved_value_strategy,
        ExcludeSetsAndMaps,
        TableNamespace,
    };

    use super::{
        TableSummary,
        TableSummarySnapshot,
        TableSummaryWriter,
    };
    use crate::{
        table_summary::{
            bootstrap,
            write_snapshot,
        },
        test_helpers::DbFixtures,
        TestFacingModel,
    };

    #[convex_macro::test_runtime]
    async fn test_bootstrap_directions(rt: TestRuntime) -> anyhow::Result<()> {
        // Three documents written at different timestamps: ts1, ts2, ts3.
        // Test the two reasons for walking by_id, and the documents log walk
        // forwards and backwards.

        let DbFixtures {
            db: database,
            tp: persistence,
            ..
        } = DbFixtures::new(&rt).await?;
        let rv = database.retention_validator();
        let table_name: TableName = "t".parse()?;

        let mut tx = database.begin(Identity::system()).await?;
        let inserted = TestFacingModel::new(&mut tx)
            .insert_and_get(table_name.clone(), assert_obj!("f" => 1))
            .await?;
        let value = inserted.value().0.clone();
        let expected_ts1 = TableSummary::empty().insert(&value);
        let table_id = tx
            .table_mapping()
            .namespace(TableNamespace::test_user())
            .id(&table_name)?;
        let ts1 = unchecked_repeatable_ts(database.commit(tx).await?);

        let mut tx = database.begin(Identity::system()).await?;
        let inserted = TestFacingModel::new(&mut tx)
            .insert_and_get(table_name.clone(), assert_obj!("f" => true))
            .await?;
        let value = inserted.value().0.clone();
        let expected_ts2 = expected_ts1.insert(&value);
        let ts2 = unchecked_repeatable_ts(database.commit(tx).await?);

        let mut tx = database.begin(Identity::system()).await?;
        let inserted = TestFacingModel::new(&mut tx)
            .insert_and_get(table_name.clone(), assert_obj!("f" => 5.0))
            .await?;
        let value = inserted.value().0.clone();
        let expected_ts3 = expected_ts2.insert(&value);
        let ts3 = unchecked_repeatable_ts(database.commit(tx).await?);

        // Bootstrap at ts2 by walking by_id, and write the snapshot that later
        // test cases will use.
        let (snapshot, _) = bootstrap(&rt, persistence.reader(), rv.clone(), ts2, false).await?;
        assert_eq!(
            snapshot.tables.get(&table_id.tablet_id),
            Some(&expected_ts2)
        );
        assert_eq!(snapshot.ts, *ts2);
        write_snapshot(persistence.as_ref(), &snapshot).await?;

        // Bootstrap at ts2 by reading the snapshot and returning it.
        let (snapshot, walked) =
            bootstrap(&rt, persistence.reader(), rv.clone(), ts2, false).await?;
        assert_eq!(walked, 0);
        assert_eq!(
            snapshot.tables.get(&table_id.tablet_id),
            Some(&expected_ts2)
        );
        assert_eq!(snapshot.ts, *ts2);

        // Bootstrap at ts3 by reading the snapshot and walking forwards.
        let (snapshot, walked) =
            bootstrap(&rt, persistence.reader(), rv.clone(), ts3, false).await?;
        assert_eq!(walked, 1);
        assert_eq!(
            snapshot.tables.get(&table_id.tablet_id),
            Some(&expected_ts3)
        );
        assert_eq!(snapshot.ts, *ts3);

        // Bootstrap at ts1 by reading the snapshot and walking backwards.
        let (snapshot, walked) =
            bootstrap(&rt, persistence.reader(), rv.clone(), ts1, false).await?;
        assert_eq!(walked, 1);
        assert_eq!(
            snapshot.tables.get(&table_id.tablet_id),
            Some(&expected_ts1)
        );
        assert_eq!(snapshot.ts, *ts1);

        // Bootstrap from scratch at ts3 by walking by_id.
        let (snapshot, _) = bootstrap(&rt, persistence.reader(), rv.clone(), ts3, true).await?;
        assert_eq!(
            snapshot.tables.get(&table_id.tablet_id),
            Some(&expected_ts3)
        );
        assert_eq!(snapshot.ts, *ts3);

        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 32 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]
        #[test]
        fn test_snapshot_roundtrips(v in any::<TableSummarySnapshot>()) {
            let roundtripped = TableSummarySnapshot::try_from(JsonValue::from(&v)).unwrap();
            assert_eq!(v, roundtripped);
        }
    }

    fn small_user_object() -> impl Strategy<Value = ConvexObject> {
        let values =
            resolved_value_strategy(FieldName::user_strategy, 4, 4, 4, ExcludeSetsAndMaps(false));
        resolved_object_strategy(FieldName::user_strategy(), values, 0..4)
    }

    fn small_user_objects() -> impl Strategy<Value = Vec<ConvexObject>> {
        prop_vec(small_user_object(), 0..8)
    }

    fn backfill_matches_test(table_name: TableName, vs: Vec<ConvexObject>) {
        let td = TestDriver::new();
        let runtime = td.rt();
        let test = async {
            let is_empty = vs.is_empty();
            let DbFixtures {
                db: database,
                tp: persistence,
                ..
            } = DbFixtures::new(&runtime).await?;
            let mut expected = TableSummary::empty();
            let mut tx = database.begin(Identity::system()).await?;
            for v in vs {
                let inserted = TestFacingModel::new(&mut tx)
                    .insert_and_get(table_name.clone(), v)
                    .await?;
                let value = inserted.value().0.clone();
                expected = expected.insert(&value);
            }
            let table_mapping = tx.table_mapping().clone();
            database.commit(tx).await?;

            let writer = TableSummaryWriter::new_with_config(
                runtime.clone(),
                persistence,
                database,
                Arc::new(NoopRetentionValidator),
            );
            let computed = writer.compute_snapshot(None, 2).await?;

            if !is_empty {
                let table_id = table_mapping
                    .namespace(TableNamespace::test_user())
                    .id(&table_name)?;
                assert_eq!(computed.tables.get(&table_id.tablet_id), Some(&expected));
            }

            Ok::<_, anyhow::Error>(())
        };
        td.run_until(test).unwrap();
    }

    fn multiple_tables_test(values: BTreeMap<TableName, Vec<ConvexObject>>) {
        let td = TestDriver::new();
        let runtime = td.rt();
        let test = async {
            let DbFixtures {
                db: database,
                tp: persistence,
                ..
            } = DbFixtures::new(&runtime).await?;
            let mut expected: BTreeMap<_, TableSummary> = BTreeMap::new();
            let mut tx = database.begin(Identity::system()).await?;

            for (table_name, values) in &values {
                for value in values {
                    let inserted = TestFacingModel::new(&mut tx)
                        .insert_and_get(table_name.clone(), value.clone())
                        .await?;
                    let table_id = tx
                        .table_mapping()
                        .namespace(TableNamespace::test_user())
                        .id(table_name)?;
                    let summary = expected.entry(table_id).or_insert_with(TableSummary::empty);
                    let inserted = inserted.value().0.clone();
                    *summary = summary.insert(&inserted);
                }
            }
            let table_mapping = tx.table_mapping().clone();
            database.commit(tx).await?;

            let writer = TableSummaryWriter::new_with_config(
                runtime.clone(),
                persistence,
                database,
                Arc::new(NoopRetentionValidator),
            );
            let computed = writer.compute_snapshot(None, 2).await?;

            for (table_name, values) in &values {
                if !values.is_empty() {
                    let table_id = table_mapping
                        .namespace(TableNamespace::test_user())
                        .id(table_name)?;
                    let expected = expected.get(&table_id).unwrap();
                    assert_eq!(expected, computed.tables.get(&table_id.tablet_id).unwrap());
                }
            }
            Ok::<_, anyhow::Error>(())
        };
        td.run_until(test).unwrap();
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_backfill_matches(
            table_name in TableName::user_strategy(),
            objects in small_user_objects(),
        ) {
            backfill_matches_test(table_name, objects);
        }

        #[test]
        fn test_multiple_tables(
            values in prop::collection::btree_map(
                TableName::user_strategy(),
                small_user_objects(),
                0..4,
            ),
        ) {
            multiple_tables_test(values);
        }
    }
}
