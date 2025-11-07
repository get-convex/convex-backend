use async_trait::async_trait;
use common::{
    bootstrap_model::index::{
        vector_index::{
            VectorIndexSnapshot,
            VectorIndexState,
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
    search_index_workers::fast_forward::IndexFastForward,
    Snapshot,
    Transaction,
};

pub struct VectorFastForward;

#[async_trait]
impl<RT: Runtime> IndexFastForward<RT, ()> for VectorFastForward {
    // We have exactly one version of vector metadata right now, so there's nothing
    // to compare against.
    fn current_version(_: &mut Transaction<RT>) {}

    fn snapshot_info(config: &IndexConfig) -> Option<(Timestamp, ())> {
        let IndexConfig::Vector { on_disk_state, .. } = config else {
            return None;
        };
        let VectorIndexSnapshot { ts, .. } = match on_disk_state {
            VectorIndexState::SnapshottedAt(snapshot)
            | VectorIndexState::Backfilled { snapshot, .. } => snapshot,
            VectorIndexState::Backfilling(_) => return None,
        };
        Some((*ts, ()))
    }

    async fn get_or_create_worker_meta(
        mut model: IndexWorkerMetadataModel<'_, RT>,
        index_id: IndexId,
    ) -> anyhow::Result<ParsedDocument<IndexWorkerMetadataRecord>> {
        model.get_or_create_vector_search(index_id).await
    }

    fn num_transactions(snapshot: Snapshot, index_id: IndexId) -> anyhow::Result<Option<usize>> {
        snapshot.vector_indexes.num_transactions(index_id)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use common::{
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
    use maplit::btreemap;
    use sync_types::Timestamp;
    use value::assert_obj;

    use crate::{
        bootstrap_model::index_workers::IndexWorkerMetadataModel,
        search_index_workers::fast_forward::{
            FastForwardIndexWorker,
            LastFastForwardInfo,
        },
        tests::vector_test_utils::{
            add_document_vec_array,
            assert_backfilled,
            backfilling_vector_index_with_doc,
            IndexData,
            VectorFixtures,
        },
        vector_index_worker::fast_forward::VectorFastForward,
        Database,
        TestFacingModel,
    };

    async fn get_fast_forward_ts(
        db: &Database<TestRuntime>,
        index_id: IndexId,
    ) -> anyhow::Result<Timestamp> {
        let mut tx = db.begin_system().await?;
        Ok(*IndexWorkerMetadataModel::new(&mut tx)
            .get_or_create_vector_search(index_id)
            .await?
            .into_value()
            .index_metadata
            .mut_fast_forward_ts())
    }
    #[convex_macro::test_runtime]
    async fn test_fast_forward(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;
        let mut last_fast_forward_info: Option<LastFastForwardInfo> = None;
        let database = fixtures.db.clone();

        let IndexData {
            index_id,
            index_name,
            resolved_index_name,
            namespace,
            ..
        } = backfilling_vector_index_with_doc(&database).await?;
        let worker = fixtures.new_backfill_index_flusher()?;

        // Backfill
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});
        let initial_snapshot_ts = assert_backfilled(&database, namespace, &index_name).await?;

        // Check that fast-forwarding works when we write to another table. Advance time
        // so our commit's timestamp is past the debounce window.
        rt.advance_time(Duration::from_secs(10)).await;
        for _ in 0..*DATABASE_WORKERS_MIN_COMMITS {
            let mut tx = database.begin_system().await?;
            let unrelated_document = assert_obj!("wise" => "tunes");
            TestFacingModel::new(&mut tx)
                .insert(&"unrelated".parse()?, unrelated_document)
                .await?;
            database.commit(tx).await?;
        }

        let metrics = fast_forward(&rt, &database, &mut last_fast_forward_info).await?;
        assert_eq!(
            metrics,
            vec![(resolved_index_name.clone(), initial_snapshot_ts)]
        );
        let snapshot_ts = assert_backfilled(&database, namespace, &index_name).await?;
        // Don't touch the snapshot timestamp.
        assert_eq!(initial_snapshot_ts, snapshot_ts);
        // But do write the fast forward timestamp.
        let fast_forward_ts = get_fast_forward_ts(&database, index_id.internal_id()).await?;
        assert!(fast_forward_ts > snapshot_ts);

        // Check that we don't fast-forward if we don't advance time.
        let metrics = fast_forward(&rt, &database, &mut last_fast_forward_info).await?;
        assert!(metrics.is_empty());
        assert_eq!(
            snapshot_ts,
            assert_backfilled(&database, namespace, &index_name).await?
        );
        assert_eq!(
            fast_forward_ts,
            get_fast_forward_ts(&database, index_id.internal_id()).await?
        );

        // Check that we don't fast-forward if we bump the reproducible timestamp and
        // advance time but don't perform any commits.
        rt.advance_time(Duration::from_secs(60)).await;
        database.bump_max_repeatable_ts().await?;
        let metrics = fast_forward(&rt, &database, &mut last_fast_forward_info).await?;
        assert!(metrics.is_empty());
        assert_eq!(
            snapshot_ts,
            assert_backfilled(&database, namespace, &index_name).await?
        );
        assert_eq!(
            fast_forward_ts,
            get_fast_forward_ts(&database, index_id.internal_id()).await?
        );

        // Check that we fast-forward if we advance time sufficiently far forward past
        // SEARCH_WORKERS_MAX_CHECKPOINT_AGE.
        rt.advance_time(Duration::from_secs(7200)).await;
        database.bump_max_repeatable_ts().await?;
        let metrics = fast_forward(&rt, &database, &mut last_fast_forward_info).await?;
        assert_eq!(
            metrics,
            vec![(resolved_index_name.clone(), fast_forward_ts)]
        );
        assert_eq!(
            snapshot_ts,
            assert_backfilled(&database, namespace, &index_name).await?
        );
        let new_fast_forward_ts = get_fast_forward_ts(&database, index_id.internal_id()).await?;
        assert!(fast_forward_ts < new_fast_forward_ts);

        // Check that we don't fast-forward if we advance time but also write to the
        // indexed table. In this case, we expect the snapshot to stay in place.
        rt.advance_time(Duration::from_secs(10)).await;
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
            get_fast_forward_ts(&database, index_id.internal_id()).await?,
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
    ) -> anyhow::Result<Vec<(TabletIndexName, Timestamp)>> {
        FastForwardIndexWorker::fast_forward::<RT, (), VectorFastForward>(
            "test",
            rt,
            db,
            last_fast_forward_info,
        )
        .await
    }
}
