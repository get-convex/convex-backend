use std::{
    self,
    assert_matches::assert_matches,
    sync::Arc,
};

use ::usage_tracking::UsageCounter;
use common::{
    self,
    bootstrap_model::index::IndexMetadata,
    document::ResolvedDocument,
    maybe_val,
    pause::PauseController,
    persistence::NoopRetentionValidator,
    query::{
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
        QuerySource,
    },
    runtime::Runtime,
    types::{
        unchecked_repeatable_ts,
        IndexDescriptor,
        IndexName,
        TableName,
    },
    value::ConvexValue,
};
use events::{
    testing::TestUsageEventLogger,
    usage::NoOpUsageEventLogger,
};
use keybroker::Identity;
use must_let::must_let;
use pretty_assertions::assert_eq;
use runtime::testing::TestRuntime;
use value::{
    FieldPath,
    ResolvedDocumentId,
    TableNamespace,
};

use crate::{
    bootstrap_model::index_backfills::{
        types::BackfillCursor,
        IndexBackfillModel,
    },
    database_index_workers::index_writer::{
        IndexSelector,
        IndexWriter,
        UPDATE_BACKFILL_PROGRESS_LABEL,
    },
    test_helpers::DbFixtures,
    tests::{
        insert_documents,
        run_query,
    },
    Database,
    DatabaseSnapshot,
    IndexModel,
    IndexWorker,
};
/// A variant of test_query_index_range that adds the index *after* the
/// documents have been added, testing that index backfill works correctly.
#[convex_macro::test_runtime]
async fn test_index_backfill(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, tp, .. } = DbFixtures::new(&rt).await?;

    let (index_name, _index_id, values) = add_documents_and_index(db.clone()).await?;
    let retention_validator = Arc::new(NoopRetentionValidator);

    IndexWorker::new_terminating(rt, tp, retention_validator, db.clone(), None).await?;
    enable_index(&db, &index_name).await?;
    check_index_is_correct(db, values, index_name).await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_db_index_backfill_progress(
    rt: TestRuntime,
    pause: PauseController,
) -> anyhow::Result<()> {
    unsafe { std::env::set_var("INDEX_BACKFILL_CHUNK_SIZE", "10") };
    let DbFixtures { db, tp, .. } = DbFixtures::new(&rt).await?;

    let (_index_name, index_id, _values) = add_documents_and_index(db.clone()).await?;
    let retention_validator = Arc::new(NoopRetentionValidator);

    let hold_guard = pause.hold(UPDATE_BACKFILL_PROGRESS_LABEL);
    let rt_clone = rt.clone();
    let db_clone = db.clone();
    let _index_backfill_handle = rt.spawn("index_worker", async move {
        IndexWorker::new(
            rt_clone,
            tp,
            retention_validator,
            db_clone,
            "carnitas".into(),
            UsageCounter::new(Arc::new(NoOpUsageEventLogger)),
        )
        .await
    });
    // Wait for IndexWriter to send progress and pause
    let _pause_guard = hold_guard.wait_for_blocked().await.unwrap();

    // Check that progress was written to database
    let mut tx = db.begin_system().await?;
    let mut model = IndexBackfillModel::new(&mut tx);
    let backfill_progress = model
        .existing_backfill_metadata(index_id.developer_id)
        .await?
        .unwrap();
    assert_eq!(backfill_progress.num_docs_indexed, 10);
    assert_eq!(backfill_progress.total_docs, Some(200));

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_db_index_multi_index_backfill_progress_multiple_indexes_same_table(
    rt: TestRuntime,
    pause: PauseController,
) -> anyhow::Result<()> {
    // Small chunk size so we get an intermediate progress update
    unsafe { std::env::set_var("INDEX_BACKFILL_CHUNK_SIZE", "10") };
    let DbFixtures { db, tp, .. } = DbFixtures::new(&rt).await?;

    let (index_name, index_id1, _values) = add_documents_and_index(db.clone()).await?;

    // Add second indexes on the same table to trigger the multi-index batch
    let mut tx = db.begin_system().await?;
    let begin_ts = tx.begin_timestamp();
    let index_id2 = IndexModel::new(&mut tx)
        .add_application_index(
            TableNamespace::test_user(),
            IndexMetadata::new_backfilling(
                *begin_ts,
                IndexName::new(index_name.table().clone(), IndexDescriptor::new("by_b")?)?,
                vec![str::parse("b")?].try_into()?,
            ),
        )
        .await?;
    db.commit(tx).await?;

    let retention_validator = Arc::new(NoopRetentionValidator);
    let hold_guard = pause.hold(UPDATE_BACKFILL_PROGRESS_LABEL);
    let rt_clone = rt.clone();
    let db_clone = db.clone();
    let _index_backfill_handle = rt.spawn("index_worker", async move {
        IndexWorker::new(
            rt_clone,
            tp,
            retention_validator,
            db_clone,
            "carnitas".into(),
            UsageCounter::new(Arc::new(NoOpUsageEventLogger)),
        )
        .await
    });
    let _pause_guard = hold_guard.wait_for_blocked().await.unwrap();

    let mut tx = db.begin_system().await?;
    let mut model = IndexBackfillModel::new(&mut tx);

    for index in [index_id1, index_id2] {
        let progress = model
            .existing_backfill_metadata(index.developer_id)
            .await?
            .unwrap();
        // With CHUNK_SIZE=10 and 2 indexes, expecting 5 docs
        assert_eq!(progress.num_docs_indexed, 5,);
        assert_eq!(progress.total_docs, Some(200));
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_db_index_backfill_tracks_usage(rt: TestRuntime) -> anyhow::Result<()> {
    use indexing::index_registry::IndexedDocument;

    unsafe { std::env::set_var("INDEX_BACKFILL_CHUNK_SIZE", "10") };
    let DbFixtures { db, tp, .. } = DbFixtures::new(&rt).await?;

    let (_index_name, _index_id, values) = add_documents_and_index(db.clone()).await?;
    let retention_validator = Arc::new(NoopRetentionValidator);

    let fields: Vec<FieldPath> = vec![str::parse("a")?, str::parse("b")?];
    let expected_bytes: u64 = values
        .iter()
        .map(|doc| doc.index_key_bytes(&fields).size() as u64)
        .sum();

    let usage_logger = TestUsageEventLogger::new();

    let state = usage_logger.collect();
    assert_eq!(
        state.recent_database_ingress_size_v2.values().sum::<u64>(),
        0,
        "Expected zero database ingress before backfill"
    );

    let usage_counter = UsageCounter::new(Arc::new(usage_logger.clone()));
    IndexWorker::new_terminating(rt, tp, retention_validator, db.clone(), Some(usage_counter))
        .await?;

    let state = usage_logger.collect();
    let table_ingress = state
        .recent_database_ingress_size_v2
        .get("table")
        .copied()
        .unwrap_or(0);
    assert_eq!(
        table_ingress, expected_bytes,
        "Database ingress from index backfill should match expected index key bytes"
    );

    let expected_egress_bytes: u64 = values.iter().map(|doc| doc.size() as u64).sum();
    let table_egress = state
        .recent_database_egress_size_v2
        .get("table")
        .copied()
        .unwrap_or(0);
    assert_eq!(
        table_egress, expected_egress_bytes,
        "Database egress from index backfill should match expected document sizes"
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_db_index_backfill_resumable(
    rt: TestRuntime,
    pause: PauseController,
) -> anyhow::Result<()> {
    // Backfill for one batch
    unsafe { std::env::set_var("INDEX_BACKFILL_CHUNK_SIZE", "10") };
    let DbFixtures { db, tp, .. } = DbFixtures::new(&rt).await?;

    let (index_name, index_id, values) = add_documents_and_index(db.clone()).await?;
    let retention_validator = Arc::new(NoopRetentionValidator);

    let hold_guard = pause.hold(UPDATE_BACKFILL_PROGRESS_LABEL);
    let rt_clone = rt.clone();
    let db_clone = db.clone();
    let tp_clone = tp.clone();
    let retention_validator_clone = retention_validator.clone();
    let index_backfill_handle = rt.spawn("index_worker", async move {
        IndexWorker::new_terminating(
            rt_clone,
            tp_clone,
            retention_validator_clone,
            db_clone,
            None,
        )
        .await
        .unwrap();
    });
    // Wait for IndexWriter to send progress and pause
    let pause_guard = hold_guard.wait_for_blocked().await.unwrap();
    drop(index_backfill_handle);
    // Check that progress was written to database
    let mut tx = db.begin_system().await?;
    let mut model = IndexBackfillModel::new(&mut tx);
    let backfill_progress = model
        .existing_backfill_metadata(index_id.developer_id)
        .await?
        .unwrap();
    assert_eq!(backfill_progress.num_docs_indexed, 10);
    assert_eq!(backfill_progress.total_docs, Some(200));
    assert_matches!(
        backfill_progress.cursor,
        Some(BackfillCursor {
            snapshot_ts: _,
            cursor: Some(_cursor),
        })
    );
    pause_guard.unpause();

    // Create a new IndexWorker to show that it resumes and writes the remaining 190
    // documents
    let rt_clone = rt.clone();
    let db_clone = db.clone();
    let docs_indexed =
        IndexWorker::new_terminating(rt_clone, tp, retention_validator, db_clone, None)
            .await
            .unwrap();
    assert_eq!(docs_indexed, 190);

    let mut tx = db.begin_system().await?;
    let mut model = IndexBackfillModel::new(&mut tx);
    let backfill_progress = model
        .existing_backfill_metadata(index_id.developer_id)
        .await?
        .unwrap();
    // Check that backfill is complete
    assert_eq!(backfill_progress.num_docs_indexed, 200);
    assert_eq!(backfill_progress.total_docs, Some(200));
    assert_matches!(
        backfill_progress.cursor,
        Some(BackfillCursor {
            snapshot_ts: _,
            cursor: Some(_cursor),
        })
    );

    enable_index(&db, &index_name).await?;
    check_index_is_correct(db, values, index_name).await?;
    Ok(())
}

// Same as test_index_backfill but writing the index with IndexWriter directly.
#[convex_macro::test_runtime]
async fn test_index_write(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures {
        db: database, tp, ..
    } = DbFixtures::new(&rt).await?;

    let table_name: TableName = str::parse("table")?;
    let namespace = TableNamespace::test_user();
    let mut tx = database.begin(Identity::system()).await?;
    let values = insert_documents(&mut tx, table_name.clone()).await?;
    database.commit(tx).await?;

    let index_name = IndexName::new(table_name, IndexDescriptor::new("a_and_b")?)?;
    let mut tx = database.begin(Identity::system()).await?;
    IndexModel::new(&mut tx)
        .add_application_index(
            namespace,
            IndexMetadata::new_enabled(
                index_name.clone(),
                vec![str::parse("a")?, str::parse("b")?].try_into()?,
            ),
        )
        .await?;
    let ts = database.commit(tx).await?;

    let retention_validator = Arc::new(NoopRetentionValidator);

    let index_writer = IndexWriter::new(
        tp.clone(),
        tp.reader(),
        retention_validator.clone(),
        rt.clone(),
        None,
    );
    let database_snapshot = DatabaseSnapshot::load(
        rt.clone(),
        tp.reader(),
        unchecked_repeatable_ts(ts),
        retention_validator,
        Default::default(),
    )
    .await?;
    let index_metadata = database_snapshot.index_registry().clone();
    index_writer
        .backfill_from_ts(
            unchecked_repeatable_ts(ts),
            &index_metadata,
            IndexSelector::All(index_metadata.clone()),
            20,
            None,
        )
        .await?;

    check_index_is_correct(database, values, index_name).await?;
    Ok(())
}

async fn add_documents_and_index(
    db: Database<TestRuntime>,
) -> anyhow::Result<(IndexName, ResolvedDocumentId, Vec<ResolvedDocument>)> {
    let table_name: TableName = str::parse("table")?;
    let namespace = TableNamespace::test_user();
    let mut tx = db.begin_system().await?;
    let values = insert_documents(&mut tx, table_name.clone()).await?;
    db.commit(tx).await?;

    let index_name = IndexName::new(table_name, IndexDescriptor::new("a_and_b")?)?;
    let mut tx = db.begin_system().await?;
    let begin_ts = tx.begin_timestamp();
    let index_id = IndexModel::new(&mut tx)
        .add_application_index(
            namespace,
            IndexMetadata::new_backfilling(
                *begin_ts,
                index_name.clone(),
                vec![str::parse("a")?, str::parse("b")?].try_into()?,
            ),
        )
        .await?;
    db.commit(tx).await?;
    Ok((index_name, index_id, values))
}

async fn check_index_is_correct(
    db: Database<TestRuntime>,
    values: Vec<ResolvedDocument>,
    index_name: IndexName,
) -> anyhow::Result<()> {
    let tests: Vec<(_, _, Box<dyn Fn(i64, i64) -> bool>)> = vec![
        // single_page_asc
        (
            vec![
                IndexRangeExpression::Eq("a".parse()?, maybe_val!(3)),
                IndexRangeExpression::Gte("b".parse()?, maybe_val!(113)),
                IndexRangeExpression::Lte("b".parse()?, maybe_val!(117)),
            ],
            Order::Asc,
            Box::new(|a, b| a == 3 && (113..=117).contains(&b)),
        ),
        // prefix
        (
            vec![IndexRangeExpression::Eq("a".parse()?, maybe_val!(3))],
            Order::Asc,
            Box::new(|a, _| a == 3),
        ),
        // all_multi_page_desc
        (vec![], Order::Desc, Box::new(|_, _| true)),
    ];
    for (range, order, predicate) in tests {
        let mut expected = values
            .iter()
            .filter(|x| {
                must_let!(let ConvexValue::Int64(a) = x.value().get("a").unwrap());
                must_let!(let ConvexValue::Int64(b) = x.value().get("b").unwrap());
                predicate(*a, *b)
            })
            .cloned()
            .collect::<Vec<ResolvedDocument>>();
        if order == Order::Desc {
            expected.reverse();
        }

        let query = Query {
            source: QuerySource::IndexRange(IndexRange {
                index_name: index_name.clone(),
                range,
                order,
            }),
            operators: vec![],
        };
        let actual = run_query(db.clone(), TableNamespace::test_user(), query).await?;
        assert_eq!(actual, expected);
    }
    Ok(())
}

async fn enable_index(db: &Database<TestRuntime>, index_name: &IndexName) -> anyhow::Result<()> {
    let mut tx = db.begin_system().await?;
    let namespace = TableNamespace::test_user();
    IndexModel::new(&mut tx)
        .enable_index_for_testing(namespace, index_name)
        .await?;
    db.commit(tx).await?;
    Ok(())
}
