use std::{
    collections::BTreeMap,
    sync::Arc,
};

use common::{
    bootstrap_model::index::{
        search_index::{
            DeveloperSearchIndexConfig,
            SearchIndexSnapshotData,
            SearchIndexState,
            TextIndexSnapshot,
            TextSnapshotVersion,
        },
        IndexConfig,
        IndexMetadata,
    },
    knobs::{
        DATABASE_WORKERS_MAX_CHECKPOINT_AGE,
        DEFAULT_DOCUMENTS_PAGE_SIZE,
        SEARCH_INDEX_SIZE_SOFT_LIMIT,
    },
    runtime::Runtime,
    types::{
        IndexId,
        TabletIndexName,
        Timestamp,
    },
    value::ResolvedDocumentId,
};
use futures::{
    channel::oneshot,
    pin_mut,
    TryStreamExt,
};
use keybroker::Identity;
use search::{
    disk_index::{
        index_writer_for_directory,
        upload_index_archive_from_path,
    },
    SearchFileType,
    TantivySearchIndexSchema,
};
use storage::Storage;
use tempfile::TempDir;

use crate::{
    bootstrap_model::index_workers::IndexWorkerMetadataModel,
    index_workers::BuildReason,
    metrics,
    Database,
    IndexModel,
    SystemMetadataModel,
    Token,
};

/// A worker to build search indexes.
///
/// This is used both during the initial backfill as well as to recompute
/// the index when documents are edited.
pub struct SearchIndexFlusher<RT: Runtime> {
    runtime: RT,
    database: Database<RT>,
    storage: Arc<dyn Storage>,
    index_size_soft_limit: usize,
}

impl<RT: Runtime> SearchIndexFlusher<RT> {
    pub(crate) fn new(runtime: RT, database: Database<RT>, storage: Arc<dyn Storage>) -> Self {
        SearchIndexFlusher {
            runtime,
            database,
            storage,
            index_size_soft_limit: *SEARCH_INDEX_SIZE_SOFT_LIMIT,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_soft_limit(
        runtime: RT,
        database: Database<RT>,
        storage: Arc<dyn Storage>,
        index_size_soft_limit: usize,
    ) -> Self {
        SearchIndexFlusher {
            runtime,
            database,
            storage,
            index_size_soft_limit,
        }
    }

    /// Run one step of the SearchIndexWorker's main loop.
    ///
    /// Returns a map of IndexName to number of documents indexed for each
    /// index that was built.
    pub(crate) async fn step(&mut self) -> anyhow::Result<(BTreeMap<TabletIndexName, u32>, Token)> {
        let mut metrics = BTreeMap::new();

        let (to_build, token) = self.needs_backfill().await?;
        let num_to_build = to_build.len();
        if num_to_build > 0 {
            tracing::info!("{num_to_build} search indexes to build");
        }

        for job in to_build {
            let index_name = job.index_name.clone();
            let num_documents_indexed = self.build_one(job).await?;
            metrics.insert(index_name, num_documents_indexed as u32);
        }

        Ok((metrics, token))
    }

    /// Compute the set of indexes that need to be backfilled.
    async fn needs_backfill(&mut self) -> anyhow::Result<(Vec<IndexBuild>, Token)> {
        let mut to_build = vec![];

        let mut tx = self.database.begin(Identity::system()).await?;
        let step_ts = tx.begin_timestamp();

        let expected_version = TextSnapshotVersion::new(tx.persistence_version());
        let search_index = self.database.snapshot(tx.begin_timestamp())?.search_indexes;
        let ready_index_sizes = search_index
            .backfilled_and_enabled_index_sizes()?
            .collect::<BTreeMap<_, _>>();

        for index_doc in IndexModel::new(&mut tx).get_all_indexes().await? {
            let (index_id, index_metadata) = index_doc.into_id_and_value();
            let IndexMetadata {
                name,
                config:
                    IndexConfig::Search {
                        on_disk_state,
                        developer_config,
                    },
            } = index_metadata
            else {
                continue;
            };
            // If the index is in the `Backfilling` state, or is already `SnapshottedAt` but
            // has grown too large or has the wrong format, it needs to be backfilled.
            let needs_backfill = match &on_disk_state {
                SearchIndexState::Backfilling(_) => Some(BuildReason::Backfilling),
                SearchIndexState::SnapshottedAt(TextIndexSnapshot { version, .. })
                | SearchIndexState::Backfilled(TextIndexSnapshot { version, .. })
                    if *version != expected_version =>
                {
                    Some(BuildReason::VersionMismatch)
                },
                SearchIndexState::SnapshottedAt(TextIndexSnapshot { ts, .. })
                | SearchIndexState::Backfilled(TextIndexSnapshot { ts, .. }) => {
                    let ts = IndexWorkerMetadataModel::new(&mut tx)
                        .get_fast_forward_ts(*ts, index_id.internal_id())
                        .await?;

                    let index_size = ready_index_sizes
                        .get(&index_id.internal_id())
                        .cloned()
                        .unwrap_or(0);

                    anyhow::ensure!(ts <= *step_ts);
                    let index_age = *step_ts - ts;
                    let too_old = (index_age >= *DATABASE_WORKERS_MAX_CHECKPOINT_AGE
                        && index_size > 0)
                        .then_some(BuildReason::TooOld);
                    if too_old.is_some() {
                        tracing::info!(
                            "Non-empty index is too old, age: {:?}, size: {index_size}",
                            index_age,
                        );
                    }
                    let too_large =
                        (index_size > self.index_size_soft_limit).then_some(BuildReason::TooLarge);

                    // Order matters! Too large is more urgent than too old.
                    too_large.or(too_old)
                },
            };
            if let Some(build_reason) = needs_backfill {
                tracing::info!("Queueing search index for rebuild: {name:?} ({build_reason:?})");
                let table_id = name.table();
                let by_id_metadata = IndexModel::new(&mut tx)
                    .by_id_index_metadata(*table_id)
                    .await?;
                let job = IndexBuild {
                    index_name: name.clone(),
                    by_id: by_id_metadata.id().internal_id(),
                    developer_config: developer_config.clone(),
                    metadata_id: index_id,
                    on_disk_state,
                    _build_reason: build_reason,
                };
                to_build.push(job);
            }
        }

        Ok((to_build, tx.into_token()?))
    }

    /// Build a single search index.
    ///
    /// Returns the number of documents indexed.
    async fn build_one(&mut self, job: IndexBuild) -> anyhow::Result<usize> {
        let timer = metrics::search::build_one_timer();

        // 1. Build the index in a temporary directory.
        let index_path = TempDir::new()?;
        let (snapshot_ts, num_indexed_documents) = self.build_one_in_dir(&job, &index_path).await?;

        // 2. Zip and upload the directory.
        let archive_name =
            upload_index_archive_from_path(index_path, self.storage.clone(), SearchFileType::Text)
                .await?;

        // 3. Update the search index metadata.
        let mut tx = self.database.begin(Identity::system()).await?;
        let snapshot_data = TextIndexSnapshot {
            data: SearchIndexSnapshotData::SingleSegment(archive_name.clone()),
            ts: snapshot_ts,
            version: TextSnapshotVersion::new(tx.persistence_version()),
        };
        let new_on_disk_state = match job.on_disk_state {
            SearchIndexState::Backfilling(_) | SearchIndexState::Backfilled(_) => {
                SearchIndexState::Backfilled(snapshot_data)
            },
            SearchIndexState::SnapshottedAt(_) => SearchIndexState::SnapshottedAt(snapshot_data),
        };
        let index_name = job.index_name.clone();
        SystemMetadataModel::new(&mut tx)
            .replace(
                job.metadata_id,
                IndexMetadata::new_search_index(
                    job.index_name,
                    job.developer_config,
                    new_on_disk_state,
                )
                .try_into()?,
            )
            .await?;
        self.database
            .commit_with_write_source(tx, "search_index_worker_build_index")
            .await?;
        tracing::info!("Built search index {} at {}", index_name, snapshot_ts);

        timer.finish();
        metrics::search::log_documents_per_index(num_indexed_documents);
        Ok(num_indexed_documents)
    }

    /// Build a search index in a given temporary directory.
    ///
    /// Returns the snapshot timestamp and the number of documents indexed.
    async fn build_one_in_dir(
        &mut self,
        job: &IndexBuild,
        index_path: &TempDir,
    ) -> anyhow::Result<(Timestamp, usize)> {
        let tantivy_schema = TantivySearchIndexSchema::new(&job.developer_config);

        let snapshot_ts = self.database.now_ts_for_reads();
        let table_iterator =
            self.database
                .table_iterator(snapshot_ts, *DEFAULT_DOCUMENTS_PAGE_SIZE as usize, None);

        let index_name = &job.index_name;
        let job = job.clone();
        let (tx, rx) = oneshot::channel();
        let index_path = index_path.path().to_path_buf();
        self.runtime.spawn_thread(move || async move {
            let result: anyhow::Result<usize> = try {
                let mut index_writer = index_writer_for_directory(&index_path, &tantivy_schema)?;

                let revision_stream = table_iterator.stream_documents_in_table(
                    *job.index_name.table(),
                    job.by_id,
                    None,
                );
                pin_mut!(revision_stream);

                let mut num_indexed_documents = 0;
                while let Some((revision, revision_ts)) = revision_stream.try_next().await? {
                    let tantivy_document =
                        tantivy_schema.index_into_tantivy_document(&revision, revision_ts);
                    metrics::search::log_document_indexed(&tantivy_schema, &tantivy_document);
                    index_writer.add_document(tantivy_document)?;
                    num_indexed_documents += 1;
                }

                index_writer.commit()?;
                index_writer.wait_merging_threads()?;
                num_indexed_documents
            };
            _ = tx.send(result);
        });
        let num_indexed_documents = rx.await??;

        tracing::info!(
            "SearchIndexWorker built index {} with {} documents",
            index_name,
            num_indexed_documents
        );
        Ok((*snapshot_ts, num_indexed_documents))
    }

    /// Builds a single index.
    ///
    /// If this index is already backfilled it still produces a new snapshot.
    #[cfg(any(test, feature = "testing"))]
    pub async fn build_index_in_test(
        index_name: TabletIndexName,
        table_name: value::TableName,
        runtime: RT,
        database: Database<RT>,
        storage: Arc<dyn Storage>,
    ) -> anyhow::Result<()> {
        use common::types::IndexName;

        let mut tx = database.begin(Identity::system()).await?;
        let index_name_ = IndexName::new(table_name.clone(), index_name.descriptor().clone())?;
        let mut model = IndexModel::new(&mut tx);
        let metadata = model
            .pending_index_metadata(&index_name_)?
            .unwrap_or_else(|| {
                model
                    .enabled_index_metadata(&index_name_)
                    .unwrap()
                    .unwrap_or_else(|| panic!("Missing pending or enabled index: {:?}", index_name))
            });
        let IndexConfig::Search {
            developer_config,
            on_disk_state,
        } = &metadata.config
        else {
            anyhow::bail!("Index was not a search index: {index_name:?}");
        };
        let by_id_metadata = IndexModel::new(&mut tx)
            .by_id_index_metadata(*index_name.table())
            .await?;

        let mut worker = SearchIndexFlusher::new(runtime, database, storage);
        let job = IndexBuild {
            index_name,
            by_id: by_id_metadata.id().internal_id(),
            developer_config: developer_config.clone(),
            metadata_id: metadata.clone().id(),
            on_disk_state: on_disk_state.clone(),
            _build_reason: BuildReason::TooOld,
        };
        worker.build_one(job).await?;
        Ok(())
    }

    /// Backfills all search indexes that are in a "backfilling" state.
    #[cfg(any(test, feature = "testing"))]
    pub async fn backfill_all_in_test(
        runtime: RT,
        database: Database<RT>,
        storage: Arc<dyn Storage>,
    ) -> anyhow::Result<()> {
        let mut worker = SearchIndexFlusher::new(runtime, database, storage);
        worker.step().await?;
        Ok(())
    }
}

#[derive(Clone)]
struct IndexBuild {
    index_name: TabletIndexName,
    by_id: IndexId,
    developer_config: DeveloperSearchIndexConfig,
    metadata_id: ResolvedDocumentId,
    on_disk_state: SearchIndexState,
    _build_reason: BuildReason,
}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::Duration;

    use anyhow::Context;
    use common::{
        assert_obj,
        bootstrap_model::index::{
            search_index::{
                SearchIndexState,
                TextIndexSnapshot,
            },
            IndexConfig,
            IndexMetadata,
        },
        types::IndexName,
    };
    use maplit::btreemap;
    use must_let::must_let;
    use runtime::testing::TestRuntime;
    use sync_types::Timestamp;

    use crate::{
        test_helpers::new_test_database,
        tests::search_test_utils::{
            add_document,
            assert_backfilled,
            create_search_index_with_document,
            new_search_worker,
            IndexData,
        },
        Database,
        IndexModel,
        TestFacingModel,
    };

    async fn assert_snapshotted(
        database: &Database<TestRuntime>,
        index_name: &IndexName,
    ) -> anyhow::Result<Timestamp> {
        let mut tx = database.begin_system().await?;
        let new_metadata = IndexModel::new(&mut tx)
            .enabled_index_metadata(index_name)?
            .context("Index missing or in an unexpected state")?
            .into_value();
        must_let!(let IndexMetadata {
            config: IndexConfig::Search {
                on_disk_state: SearchIndexState::SnapshottedAt(TextIndexSnapshot { ts, .. }),
                ..
            },
            ..
        } = new_metadata);
        Ok(ts)
    }

    async fn enable_pending_index(
        database: &Database<TestRuntime>,
        index_name: &IndexName,
    ) -> anyhow::Result<()> {
        let mut tx = database.begin_system().await.unwrap();
        let mut model = IndexModel::new(&mut tx);
        let index = model
            .pending_index_metadata(index_name)?
            .context(format!("Missing pending index for {index_name:?}"))?;
        model
            .enable_backfilled_indexes(vec![index.into_value()])
            .await?;
        database.commit(tx).await?;
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_build_search_index(rt: TestRuntime) -> anyhow::Result<()> {
        let database = new_test_database(rt.clone()).await;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = create_search_index_with_document(&database).await?;
        let mut worker = new_search_worker(&rt, &database)?;

        // Run one interation of the search index worker.
        let (metrics, _) = worker.step().await?;

        // Make sure we actually built this index with one document.
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});

        // Check that the metadata is updated so it's no longer backfilling.
        assert_backfilled(&database, &index_name).await?;

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_rebuild_backfilled_search_index(rt: TestRuntime) -> anyhow::Result<()> {
        let database = new_test_database(rt.clone()).await;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = create_search_index_with_document(&database).await?;
        let mut worker = new_search_worker(&rt, &database)?;

        // Run one interation of the search index worker.
        let (metrics, _) = worker.step().await?;

        // Make sure we actually built this index with one document.
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});

        // Check that the metadata is updated so it's no longer backfilling.
        let initial_snapshot_ts = assert_backfilled(&database, &index_name).await?;

        // Write 10 more documents into the table to trigger a new snapshot.
        let mut tx = database.begin_system().await.unwrap();
        for _ in 0..10 {
            add_document(
                &mut tx,
                index_name.table(),
                "hello world, this is a message with more than just a few terms in it",
            )
            .await?;
        }
        database.commit(tx).await?;

        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 11});

        // Check that the metadata is updated so it's no longer backfilling.
        let new_snapshot_ts = assert_backfilled(&database, &index_name).await?;
        assert!(new_snapshot_ts > initial_snapshot_ts);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_rebuild_enabled_search_index(rt: TestRuntime) -> anyhow::Result<()> {
        let database = new_test_database(rt.clone()).await;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = create_search_index_with_document(&database).await?;
        let mut worker = new_search_worker(&rt, &database)?;

        // Run one interation of the search index worker.
        let (metrics, _) = worker.step().await?;

        // Make sure we actually built this index with one document.
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});
        // Check that the metadata is updated so it's no longer backfilling.
        let initial_snapshot_ts = assert_backfilled(&database, &index_name).await?;
        // Enable the index so it's in the Snapshotted state.
        enable_pending_index(&database, &index_name).await?;
        // Write 10 more documents into the table to trigger a new snapshot.
        let mut tx = database.begin_system().await.unwrap();
        for _ in 0..10 {
            add_document(
                &mut tx,
                index_name.table(),
                "hello world, this is a message with more than just a few terms in it",
            )
            .await?;
        }
        database.commit(tx).await?;

        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 11});

        // Check that the metadata is updated and still enabled.
        let new_snapshot_ts = assert_snapshotted(&database, &index_name).await?;
        assert!(new_snapshot_ts > initial_snapshot_ts);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_advance_old_snapshot(rt: TestRuntime) -> anyhow::Result<()> {
        common::testing::init_test_logging();
        let database = new_test_database(rt.clone()).await;
        let mut worker = new_search_worker(&rt, &database)?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = create_search_index_with_document(&database).await?;

        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});
        let initial_snapshot_ts = assert_backfilled(&database, &index_name).await?;

        // Write a single document underneath our soft limit and check that we don't
        // snapshot.
        let mut tx = database.begin_system().await?;
        add_document(&mut tx, index_name.table(), "too small to count").await?;
        database.commit(tx).await?;

        let (metrics, _) = worker.step().await?;
        assert!(metrics.is_empty());
        assert_eq!(
            initial_snapshot_ts,
            assert_backfilled(&database, &index_name).await?
        );

        // Advance time past the max index age (and do an unrelated commit to bump the
        // repeatable timestamp).
        rt.advance_time(Duration::from_secs(7200));
        let mut tx = database.begin_system().await?;
        let unrelated_document = assert_obj!("wise" => "ambience");
        TestFacingModel::new(&mut tx)
            .insert(&"unrelated".parse()?, unrelated_document)
            .await?;
        database.commit(tx).await?;

        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 2});
        assert!(initial_snapshot_ts < assert_backfilled(&database, &index_name).await?);

        Ok(())
    }
}
