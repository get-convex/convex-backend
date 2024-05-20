use async_trait::async_trait;
use common::{
    bootstrap_model::index::{
        text_index::{
            TextIndexSnapshot,
            TextIndexState,
            TextSnapshotVersion,
        },
        IndexConfig,
    },
    document::ParsedDocument,
    runtime::Runtime,
    types::IndexId,
};
use sync_types::Timestamp;

use crate::{
    bootstrap_model::index_workers::{
        IndexWorkerMetadataModel,
        IndexWorkerMetadataRecord,
    },
    index_workers::fast_forward::IndexFastForward,
    Snapshot,
    Transaction,
};

pub struct TextFastForward;

#[async_trait]
impl<RT: Runtime> IndexFastForward<RT, TextSnapshotVersion> for TextFastForward {
    fn current_version(tx: &mut Transaction<RT>) -> TextSnapshotVersion {
        TextSnapshotVersion::new(tx.persistence_version())
    }

    fn snapshot_info(config: &IndexConfig) -> Option<(Timestamp, TextSnapshotVersion)> {
        let IndexConfig::Search {
            ref on_disk_state, ..
        } = config
        else {
            return None;
        };
        let TextIndexSnapshot { ts, version, .. } = match on_disk_state {
            TextIndexState::SnapshottedAt(snapshot) | TextIndexState::Backfilled(snapshot) => {
                snapshot
            },
            TextIndexState::Backfilling(_) => return None,
        };
        Some((*ts, *version))
    }

    async fn get_or_create_worker_meta(
        mut model: IndexWorkerMetadataModel<'_, RT>,
        id: IndexId,
    ) -> anyhow::Result<ParsedDocument<IndexWorkerMetadataRecord>> {
        model.get_or_create_text_search(id).await
    }

    fn num_transactions(snapshot: Snapshot, index_id: IndexId) -> anyhow::Result<Option<usize>> {
        snapshot.search_indexes.num_transactions(index_id)
    }
}

#[cfg(test)]
pub mod tests {
    use std::{
        collections::BTreeSet,
        time::Duration,
    };

    use common::{
        bootstrap_model::index::text_index::TextSnapshotVersion,
        knobs::DATABASE_WORKERS_MIN_COMMITS,
        runtime::{
            testing::TestRuntime,
            Runtime,
        },
        types::{
            IndexId,
            TabletIndexName,
        },
    };
    use maplit::{
        btreemap,
        btreeset,
    };
    use sync_types::Timestamp;
    use value::assert_obj;

    use crate::{
        bootstrap_model::index_workers::IndexWorkerMetadataModel,
        index_workers::fast_forward::{
            FastForwardIndexWorker,
            LastFastForwardInfo,
        },
        test_helpers::new_test_database,
        tests::{
            search_test_utils::{
                assert_backfilled,
                create_search_index_with_document,
                new_search_worker,
                IndexData,
            },
            vector_test_utils::add_document_vec_array,
        },
        text_index_worker::fast_forward::TextFastForward,
        Database,
        TestFacingModel,
    };

    async fn get_fast_forward_ts(
        db: &Database<TestRuntime>,
        index_id: IndexId,
    ) -> anyhow::Result<Timestamp> {
        let mut tx = db.begin_system().await?;
        Ok(*IndexWorkerMetadataModel::new(&mut tx)
            .get_or_create_text_search(index_id)
            .await?
            .into_value()
            .index_metadata
            .mut_fast_forward_ts())
    }

    #[convex_macro::test_runtime]
    async fn test_fast_forward(rt: TestRuntime) -> anyhow::Result<()> {
        let mut last_fast_forward_info: Option<LastFastForwardInfo> = None;

        let database = new_test_database(rt.clone()).await;

        let IndexData {
            index_id,
            index_name,
            resolved_index_name,
            namespace,
        } = create_search_index_with_document(&database).await?;
        let mut worker = new_search_worker(&rt, &database)?;

        // Backfill the index
        let (metrics, _) = worker.step().await?;

        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});
        let initial_snapshot_ts = assert_backfilled(&database, namespace, &index_name).await?;

        // Check that fast-forwarding works when we write to another table. Advance time
        // so our commit's timestamp is past the debounce window.
        rt.advance_time(Duration::from_secs(10));
        for _ in 0..*DATABASE_WORKERS_MIN_COMMITS {
            let mut tx = database.begin_system().await?;
            let unrelated_document = assert_obj!("wise" => "tunes");
            TestFacingModel::new(&mut tx)
                .insert(&"unrelated".parse()?, unrelated_document)
                .await?;
            database.commit(tx).await?;
        }

        let metrics = fast_forward(&rt, &database, &mut last_fast_forward_info).await?;
        assert_eq!(metrics, btreeset! {resolved_index_name.clone() });
        // Don't touch the snapshot timestamp
        let snapshot_ts = assert_backfilled(&database, namespace, &index_name).await?;
        assert_eq!(
            initial_snapshot_ts, snapshot_ts,
            "initial: {initial_snapshot_ts}, now: {snapshot_ts}"
        );
        // Do write a fast forward ts
        let fast_forward_ts = get_fast_forward_ts(&database, index_id).await?;
        assert!(
            fast_forward_ts > initial_snapshot_ts,
            "initial: {initial_snapshot_ts}, fast_forward_ts: {fast_forward_ts}"
        );

        // Check that we don't fast-forward if we bump the reproducible timestamp and
        // advance time but don't perform any commits.
        let metrics = fast_forward(&rt, &database, &mut last_fast_forward_info).await?;
        assert!(metrics.is_empty());
        assert_eq!(
            snapshot_ts,
            assert_backfilled(&database, namespace, &index_name).await?
        );
        assert_eq!(
            fast_forward_ts,
            get_fast_forward_ts(&database, index_id).await?
        );

        // Check that we fast-forward if we advance time sufficiently far forward past
        // DATABASE_WORKERS_MAX_CHECKPOINT_AGE even with no writes.
        rt.advance_time(Duration::from_secs(7200));
        database.bump_max_repeatable_ts().await?;
        let metrics = fast_forward(&rt, &database, &mut last_fast_forward_info).await?;
        assert_eq!(metrics, btreeset! {resolved_index_name.clone()});
        assert_eq!(
            snapshot_ts,
            assert_backfilled(&database, namespace, &index_name).await?
        );
        let new_fast_forward_ts = get_fast_forward_ts(&database, index_id).await?;
        assert!(fast_forward_ts < new_fast_forward_ts);

        // Check that we don't fast-forward if we advance time but also write to the
        // indexed table. In this case, we expect the snapshot to stay in place.
        rt.advance_time(Duration::from_secs(10));
        let mut tx = database.begin_system().await?;
        let unrelated_document = assert_obj!("wise" => "jams");
        TestFacingModel::new(&mut tx)
            .insert(&"unrelated".parse()?, unrelated_document)
            .await?;
        add_document_vec_array(&mut tx, index_name.table(), [2f64, 3f64]).await?;
        database.commit(tx).await?;

        let metrics = fast_forward(&rt, &database, &mut last_fast_forward_info).await?;
        assert!(metrics.is_empty(), "{metrics:?}");
        assert_eq!(
            new_fast_forward_ts,
            get_fast_forward_ts(&database, index_id).await?,
        );
        assert_eq!(
            snapshot_ts,
            assert_backfilled(&database, namespace, &index_name).await?,
        );

        Ok(())
    }

    async fn fast_forward<RT: Runtime>(
        rt: &RT,
        db: &Database<RT>,
        last_fast_forward_info: &mut Option<LastFastForwardInfo>,
    ) -> anyhow::Result<BTreeSet<TabletIndexName>> {
        FastForwardIndexWorker::fast_forward::<RT, TextSnapshotVersion, TextFastForward>(
            "test",
            rt,
            db,
            last_fast_forward_info,
        )
        .await
    }
}
