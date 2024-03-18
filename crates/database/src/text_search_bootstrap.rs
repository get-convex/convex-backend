use std::{
    cmp::max,
    future,
    ops::Bound,
};

use common::{
    bootstrap_model::index::{
        search_index::{
            SearchIndexSnapshot,
            SearchIndexState,
        },
        IndexConfig,
    },
    persistence::{
        RepeatablePersistence,
        TimestampRange,
    },
    persistence_helpers::stream_revision_pairs,
    query::Order,
    types::{
        PersistenceVersion,
        WriteTimestamp,
    },
};
use futures::TryStreamExt;
use imbl::OrdMap;
use indexing::index_registry::IndexRegistry;
use search::{
    MemorySearchIndex,
    SearchIndex,
    SnapshotInfo,
    TantivySearchIndexSchema,
};
use value::{
    InternalId,
    TableMapping,
};

use crate::index_workers::fast_forward::load_metadata_fast_forward_ts;

pub async fn bootstrap_search(
    registry: &IndexRegistry,
    persistence: &RepeatablePersistence,
    table_mapping: &TableMapping,
) -> anyhow::Result<(OrdMap<InternalId, SearchIndex>, PersistenceVersion)> {
    let timer = crate::metrics::search::bootstrap_timer();
    let mut num_revisions = 0;
    let mut total_size = 0;

    let mut indexes = OrdMap::new();

    // Load all of the fast forward timestamps first to ensure that we stay within
    // the comparatively short valid time for the persistence snapshot
    let snapshot = persistence.read_snapshot(persistence.upper_bound())?;
    let mut search_indexes_with_fast_forward_ts = vec![];
    for index in registry.all_search_indexes() {
        let fast_forward_ts =
            load_metadata_fast_forward_ts(registry, &snapshot, table_mapping, index.id()).await?;
        search_indexes_with_fast_forward_ts.push((index, fast_forward_ts));
    }

    // Then do the slower/more expensive reads on the document log for every index.
    for (index, fast_forward_ts) in search_indexes_with_fast_forward_ts {
        let IndexConfig::Search {
            ref developer_config,
            ref on_disk_state,
        } = index.config
        else {
            continue;
        };
        let search_index = match on_disk_state {
            SearchIndexState::Backfilling => {
                // We'll start a new memory search index starting at the next commit after our
                // persistence upper bound. After bootstrapping, all commits after
                // `persistence.upper_bound()` will flow through `Self::update`, so our memory
                // index contains all revisions `>= persistence.upper_bound().succ()?`.
                SearchIndex::Backfilling {
                    memory_index: MemorySearchIndex::new(WriteTimestamp::Committed(
                        persistence.upper_bound().succ()?,
                    )),
                }
            },
            SearchIndexState::Backfilled(SearchIndexSnapshot {
                index: disk_index,
                ts: disk_ts,
                version,
            })
            | SearchIndexState::SnapshottedAt(SearchIndexSnapshot {
                index: disk_index,
                ts: disk_ts,
                version,
            }) => {
                tracing::info!(
                    "Starting bootstrap for {}, snapshot_ts: {}, fast_forward_ts: {:?}",
                    index.name,
                    *disk_ts,
                    fast_forward_ts
                );

                let ts = max(*disk_ts, fast_forward_ts.unwrap_or_default());

                let tantivy_schema = TantivySearchIndexSchema::new(developer_config);

                let range = (
                    Bound::Excluded(ts),
                    Bound::Included(*persistence.upper_bound()),
                );
                let document_stream = persistence
                    .load_documents(TimestampRange::new(range)?, Order::Asc)
                    .try_filter(|(_, id, _)| future::ready(id.table() == index.name.table()));
                let revision_stream = stream_revision_pairs(document_stream, persistence);
                futures::pin_mut!(revision_stream);

                let mut memory_index =
                    MemorySearchIndex::new(WriteTimestamp::Committed(disk_ts.succ()?));
                while let Some(revision_pair) = revision_stream.try_next().await? {
                    memory_index.update(
                        revision_pair.id.internal_id(),
                        WriteTimestamp::Committed(revision_pair.ts()),
                        revision_pair
                            .prev_document()
                            .map(|d| {
                                anyhow::Ok((
                                    tantivy_schema.index_into_terms(d)?,
                                    d.creation_time()
                                        .expect("Document should have creation time"),
                                ))
                            })
                            .transpose()?,
                        revision_pair
                            .document()
                            .map(|d| {
                                anyhow::Ok((
                                    tantivy_schema.index_into_terms(d)?,
                                    d.creation_time()
                                        .expect("Document should have creation time"),
                                ))
                            })
                            .transpose()?,
                    )?;
                    num_revisions += 1;
                    total_size += revision_pair.document().map(|d| d.size()).unwrap_or(0);
                }
                let snapshot = SnapshotInfo {
                    disk_index: disk_index.clone(),
                    disk_index_ts: ts,
                    disk_index_version: *version,
                    memory_index,
                };
                if index.config.is_enabled() {
                    SearchIndex::Ready(snapshot)
                } else {
                    SearchIndex::Backfilled(snapshot)
                }
            },
        };
        indexes.insert(index.id().internal_id(), search_index);
    }
    tracing::info!(
        "Loaded {num_revisions} revisions ({total_size} bytes) in {:?}.",
        timer.elapsed()
    );
    crate::metrics::search::finish_bootstrap(num_revisions, total_size, timer);
    Ok((indexes, persistence.version()))
}

#[cfg(test)]
mod tests {
    use std::{
        sync::Arc,
        time::Duration,
    };

    use common::{
        bootstrap_model::index::{
            search_index::SearchIndexState,
            IndexConfig,
            IndexMetadata,
            TabletIndexMetadata,
        },
        document::ParsedDocument,
        persistence::{
            NoopRetentionValidator,
            Persistence,
            RepeatablePersistence,
        },
        runtime::Runtime,
        types::{
            IndexId,
            IndexName,
            TabletIndexName,
            WriteTimestamp,
        },
    };
    use imbl::OrdMap;
    use maplit::btreeset;
    use must_let::must_let;
    use runtime::testing::TestRuntime;
    use search::SearchIndex;
    use storage::Storage;
    use sync_types::Timestamp;
    use value::{
        assert_obj,
        InternalId,
        ResolvedDocumentId,
        TableName,
    };

    use crate::{
        bootstrap_model::index_workers::IndexWorkerMetadataModel,
        test_helpers::DbFixtures,
        text_search_bootstrap::{
            bootstrap_search,
            load_metadata_fast_forward_ts,
        },
        Database,
        IndexModel,
        SearchIndexFlusher,
        SystemMetadataModel,
        TableModel,
        TestFacingModel,
        Transaction,
    };

    async fn run_bootstrap<RT: Runtime>(
        tp: Box<dyn Persistence>,
        db: &Database<RT>,
    ) -> anyhow::Result<OrdMap<InternalId, SearchIndex>> {
        let mut tx = db.begin_system().await?;
        let retention_validator = Arc::new(NoopRetentionValidator {});
        let persistence =
            RepeatablePersistence::new(tp.reader(), tx.begin_timestamp(), retention_validator);
        let snapshot = db.snapshot(tx.begin_timestamp())?;

        let (indexes, _) =
            bootstrap_search(&snapshot.index_registry, &persistence, tx.table_mapping()).await?;
        Ok(indexes)
    }

    #[convex_macro::test_runtime]
    async fn test_load_snapshot_without_fast_forward(rt: TestRuntime) -> anyhow::Result<()> {
        let DbFixtures {
            tp,
            db,
            search_storage,
            ..
        } = DbFixtures::new(&rt).await?;
        let (index_id, _) = create_new_search_index(&rt, &db, search_storage.clone()).await?;

        let mut tx = db.begin_system().await.unwrap();
        add_document(
            &mut tx,
            &"test".parse()?,
            "hello world, this is a message with more than just a few terms in it",
        )
        .await?;
        db.commit(tx).await?;

        let indexes = run_bootstrap(tp, &db).await?;

        let index = indexes.get(&index_id).unwrap();
        let SearchIndex::Backfilled(snapshot) = index else {
            // Not using must_let because we don't implement Debug for this or nested
            // structs.
            panic!("Not backfilling?")
        };
        assert_eq!(snapshot.memory_index.num_transactions(), 1);

        Ok(())
    }
    #[convex_macro::test_runtime]
    async fn test_load_snapshot_with_fast_forward(rt: TestRuntime) -> anyhow::Result<()> {
        let DbFixtures {
            tp,
            db,
            search_storage,
            ..
        } = DbFixtures::new(&rt).await?;
        let (index_id, _) = create_new_search_index(&rt, &db, search_storage.clone()).await?;

        rt.advance_time(Duration::from_secs(10));

        let mut tx = db.begin_system().await.unwrap();
        add_document(
            &mut tx,
            &"test".parse()?,
            "hello world, this is a message with more than just a few terms in it",
        )
        .await?;
        db.commit(tx).await?;

        rt.advance_time(Duration::from_secs(10));

        // We shouldn't ever fast forward across an update in real life, but doing so
        // and verifying we don't read the document is a simple way to verify we
        // actually use the fast forward timestamp.
        let mut tx = db.begin_system().await?;
        let mut model = IndexWorkerMetadataModel::new(&mut tx);
        let (metadata_id, mut metadata) = model
            .get_or_create_text_search(index_id)
            .await?
            .into_id_and_value();
        *metadata.index_metadata.mut_fast_forward_ts() = Timestamp::MAX.pred().unwrap();
        SystemMetadataModel::new(&mut tx)
            .replace(metadata_id, metadata.try_into()?)
            .await?;
        db.commit(tx).await?;

        let indexes = run_bootstrap(tp, &db).await?;

        let index = indexes.get(&index_id).unwrap();
        let SearchIndex::Backfilled(snapshot) = index else {
            panic!("Not backfilling?")
        };
        assert_eq!(snapshot.memory_index.num_transactions(), 0);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_load_snapshot_with_fast_forward_uses_disk_ts_for_memory_index(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let DbFixtures {
            tp,
            db,
            search_storage,
            ..
        } = DbFixtures::new(&rt).await?;
        let (index_id, index_doc) =
            create_new_search_index(&rt, &db, search_storage.clone()).await?;

        // We shouldn't ever fast forward across an update in real life, but doing so
        // and verifying we don't read the document is a simple way to verify we
        // actually use the fast forward timestamp.
        let mut tx = db.begin_system().await?;
        let mut model = IndexWorkerMetadataModel::new(&mut tx);
        let (metadata_id, mut metadata) = model
            .get_or_create_text_search(index_id)
            .await?
            .into_id_and_value();
        *metadata.index_metadata.mut_fast_forward_ts() = Timestamp::MAX.pred().unwrap();
        SystemMetadataModel::new(&mut tx)
            .replace(metadata_id, metadata.try_into()?)
            .await?;
        db.commit(tx).await?;

        let indexes = run_bootstrap(tp, &db).await?;

        // No must-let because SearchIndex doesn't implement Debug.
        let SearchIndex::Backfilled(memory_snapshot) = indexes.get(&index_id).unwrap() else {
            anyhow::bail!("Unexpected index type");
        };
        must_let!(
            let IndexConfig::Search {
                on_disk_state: SearchIndexState::Backfilled(disk_snapshot), ..
            } = index_doc.into_value().config
        );

        assert_eq!(
            memory_snapshot.memory_index.min_ts(),
            WriteTimestamp::Committed(disk_snapshot.ts.succ().unwrap())
        );

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_load_fast_forward_ts(rt: TestRuntime) -> anyhow::Result<()> {
        let DbFixtures {
            tp,
            db,
            search_storage,
            ..
        } = DbFixtures::new(&rt).await?;
        let (index_id, index_doc) =
            create_new_search_index(&rt, &db, search_storage.clone()).await?;
        let mut tx = db.begin_system().await?;
        let mut model = IndexWorkerMetadataModel::new(&mut tx);
        let (metadata_id, mut metadata) = model
            .get_or_create_text_search(index_id)
            .await?
            .into_id_and_value();
        *metadata.index_metadata.mut_fast_forward_ts() = Timestamp::MAX;
        SystemMetadataModel::new(&mut tx)
            .replace(metadata_id, metadata.try_into()?)
            .await?;
        db.commit(tx).await?;

        let mut tx = db.begin_system().await?;
        let retention_validator = Arc::new(NoopRetentionValidator {});
        let persistence =
            RepeatablePersistence::new(tp.reader(), db.now_ts_for_reads(), retention_validator);
        let persistence_snapshot = persistence.read_snapshot(persistence.upper_bound())?;
        let snapshot = db.snapshot(db.now_ts_for_reads())?;

        let fast_forward_ts = load_metadata_fast_forward_ts(
            &snapshot.index_registry,
            &persistence_snapshot,
            tx.table_mapping(),
            index_doc.id(),
        )
        .await?;

        assert_eq!(fast_forward_ts, Some(Timestamp::MAX));

        Ok(())
    }
    async fn add_document(
        tx: &mut Transaction<TestRuntime>,
        table_name: &TableName,
        text: &str,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let document = assert_obj!(
            "text" => text,
        );
        TestFacingModel::new(tx).insert(table_name, document).await
    }

    async fn create_new_search_index<RT: Runtime>(
        rt: &RT,
        db: &Database<RT>,
        search_storage: Arc<dyn Storage>,
    ) -> anyhow::Result<(IndexId, ParsedDocument<TabletIndexMetadata>)> {
        let table_name: TableName = "test".parse()?;
        let mut tx = db.begin_system().await?;
        TableModel::new(&mut tx)
            .insert_table_metadata_for_test(&table_name)
            .await?;
        let index = IndexMetadata::new_backfilling_search_index(
            "test.by_text".parse()?,
            "searchField".parse()?,
            btreeset! {"filterField".parse()?},
        );
        IndexModel::new(&mut tx)
            .add_application_index(index)
            .await?;
        db.commit(tx).await?;

        let snapshot = db.latest_snapshot()?;
        let table_id = snapshot.table_mapping().id(&"test".parse()?)?.table_id;
        let index_name = TabletIndexName::new(table_id, "by_text".parse()?)?;
        SearchIndexFlusher::build_index_in_test(
            index_name.clone(),
            "test".parse()?,
            rt.clone(),
            db.clone(),
            search_storage.clone(),
        )
        .await?;

        let index_name = IndexName::new(table_name, "by_text".parse()?)?;
        let mut tx = db.begin_system().await?;
        let mut model = IndexModel::new(&mut tx);
        let index_doc = model.pending_index_metadata(&index_name)?.unwrap();
        Ok((index_doc.id().internal_id(), index_doc))
    }
}
