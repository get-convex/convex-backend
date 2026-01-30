use common::{
    assert_obj,
    bootstrap_model::index::{
        database_index::IndexedFields,
        IndexMetadata,
    },
    components::{
        ComponentId,
        ComponentPath,
    },
    document::PackedDocument,
    maybe_val,
    query::{
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
    },
    types::{
        IndexDescriptor,
        IndexName,
        TableName,
    },
};
use indexing::index_registry::IndexedDocument;
use keybroker::Identity;
use maplit::btreeset;
use pretty_assertions::assert_eq;
use runtime::testing::TestRuntime;
use usage_tracking::FunctionUsageTracker;
use value::{
    Size,
    TableNamespace,
};
use vector::VectorSearch;

use crate::{
    test_helpers::DbFixtures,
    tests::{
        text_test_utils::{
            add_document,
            TextFixtures,
            TextIndexData,
        },
        vector_test_utils::{
            add_document_vec_array,
            VectorFixtures,
            VectorIndexData,
        },
    },
    IndexModel,
    ResolvedQuery,
    TestFacingModel,
    UserFacingModel,
};

#[convex_macro::test_runtime]
async fn vector_insert_with_no_index_does_not_count_usage(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = VectorFixtures::new(rt).await?;

    let table_name: TableName = "my_table".parse()?;
    // Use a user transaction, not a system transaction
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = fixtures
        .db
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    add_document_vec_array(&mut tx, &table_name, [3f64, 4f64]).await?;
    fixtures.db.commit(tx).await?;
    fixtures
        .usage_counter
        .track_call_test(tx_usage.gather_user_stats())
        .await;

    let stats = fixtures.test_usage_logger.collect();
    assert!(stats.recent_vector_ingress_size.is_empty());
    assert!(stats.recent_vector_ingress_size_v2.is_empty());
    Ok(())
}

#[convex_macro::test_runtime]
async fn vector_insert_counts_usage_for_backfilling_indexes(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = VectorFixtures::new(rt).await?;
    let VectorIndexData {
        index_name,
        qdrant_schema,
        ..
    } = fixtures.backfilling_vector_index().await?;

    // Use a user transaction, not a system transaction
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = fixtures
        .db
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    let doc_id = add_document_vec_array(&mut tx, index_name.table(), [3f64, 4f64]).await?;
    let document = tx.get(doc_id).await?.unwrap();
    fixtures.db.commit(tx).await?;
    fixtures
        .usage_counter
        .track_call_test(tx_usage.gather_user_stats())
        .await;

    let stats = fixtures.test_usage_logger.collect();
    let value = stats
        .recent_vector_ingress_size
        .get(&(*index_name.table()).to_string())
        .cloned();
    let value_v2 = stats
        .recent_vector_ingress_size_v2
        .get(&(*index_name.table()).to_string())
        .cloned();

    assert_eq!(value, Some((document.size()) as u64));
    assert_eq!(
        value_v2,
        Some((qdrant_schema.estimate_vector_size() + doc_id.size()) as u64)
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn vector_insert_counts_usage_for_enabled_indexes(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = VectorFixtures::new(rt).await?;
    let VectorIndexData {
        index_name,
        qdrant_schema,
        ..
    } = fixtures.enabled_vector_index().await?;

    // Use a user transaction, not a system transaction
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = fixtures
        .db
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    let doc_id = add_document_vec_array(&mut tx, index_name.table(), [3f64, 4f64]).await?;
    let document = tx.get(doc_id).await?.unwrap();
    fixtures.db.commit(tx).await?;
    fixtures
        .usage_counter
        .track_call_test(tx_usage.gather_user_stats())
        .await;

    let mut stats = fixtures.test_usage_logger.collect();
    let value = stats
        .recent_vector_ingress_size
        .remove(&(*index_name.table()).to_string());
    let value_v2 = stats
        .recent_vector_ingress_size_v2
        .remove(&(*index_name.table()).to_string());
    assert_eq!(value, Some((document.size()) as u64));
    assert_eq!(
        value_v2,
        Some((qdrant_schema.estimate_vector_size() + doc_id.size()) as u64)
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn vectors_in_segment_count_as_usage(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = VectorFixtures::new(rt).await?;
    let VectorIndexData { index_name, .. } = fixtures.enabled_vector_index().await?;

    // Use a user transaction, not a system transaction
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = fixtures
        .db
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    add_document_vec_array(&mut tx, index_name.table(), [3f64, 4f64]).await?;
    fixtures.db.commit(tx).await?;
    fixtures
        .usage_counter
        .track_call_test(tx_usage.gather_user_stats())
        .await;

    fixtures.new_live_index_flusher()?.step().await?;

    let storage = fixtures
        .db
        .latest_database_snapshot()?
        .get_vector_index_storage(&Identity::system())?;

    let key = (ComponentPath::root(), index_name.table().clone());
    let value = storage.get(&key).cloned();
    assert_eq!(value, Some(8_u64));
    Ok(())
}

#[convex_macro::test_runtime]
async fn vectors_backfilled_unstaged_does_not_count(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = VectorFixtures::new(rt).await?;
    let VectorIndexData { index_name, .. } = fixtures.backfilled_vector_index().await?;

    // Use a user transaction, not a system transaction
    let mut tx = fixtures.db.begin_system().await?;
    add_document_vec_array(&mut tx, index_name.table(), [3f64, 4f64]).await?;
    fixtures.db.commit(tx).await?;
    fixtures.new_live_index_flusher()?.step().await?;

    let storage = fixtures
        .db
        .latest_database_snapshot()?
        .get_vector_index_storage(&Identity::system())?;

    let key = (ComponentPath::root(), index_name.table().clone());
    let value = storage.get(&key).cloned();
    assert_eq!(value, Some(0_u64));
    Ok(())
}

#[convex_macro::test_runtime]
async fn vectors_backfilled_staged_counts(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = VectorFixtures::new(rt).await?;
    let VectorIndexData { index_name, .. } = fixtures.staged_backfilled_vector_index().await?;

    // Use a user transaction, not a system transaction
    let mut tx = fixtures.db.begin_system().await?;
    add_document_vec_array(&mut tx, index_name.table(), [3f64, 4f64]).await?;
    fixtures.db.commit(tx).await?;
    fixtures.new_live_index_flusher()?.step().await?;

    let storage = fixtures
        .db
        .latest_database_snapshot()?
        .get_vector_index_storage(&Identity::system())?;

    let key = (ComponentPath::root(), index_name.table().clone());
    let value = storage.get(&key).cloned();
    assert_eq!(value, Some(8_u64));
    Ok(())
}

#[convex_macro::test_runtime]
async fn vector_query_counts_bandwidth(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = VectorFixtures::new(rt).await?;
    let VectorIndexData { index_name, .. } = fixtures.enabled_vector_index().await?;

    // Use a user transaction, not a system transaction
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = fixtures
        .db
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    add_document_vec_array(&mut tx, index_name.table(), [3f64, 4f64]).await?;
    fixtures.db.commit(tx).await?;
    fixtures.new_live_index_flusher()?.step().await?;

    let (results, usage_stats) = fixtures
        .db
        .vector_search(
            Identity::Unknown(None),
            VectorSearch {
                index_name: index_name.clone(),
                component_id: ComponentId::Root,
                limit: Some(10),
                vector: vec![0.; 2],
                expressions: btreeset![],
            },
        )
        .await?;
    tx_usage.add(usage_stats);

    fixtures
        .usage_counter
        .track_call_test(tx_usage.gather_user_stats())
        .await;

    let total_size = results.into_iter().map(|row| row.size() as u64).sum();
    let mut stats = fixtures.test_usage_logger.collect();
    assert_eq!(
        stats
            .recent_vector_egress_size
            .remove(&index_name.table().to_string()),
        Some(total_size)
    );
    assert_eq!(
        stats
            .recent_database_egress_size
            .remove(&index_name.table().to_string()),
        Some(total_size)
    );
    let (num_searches, bytes_searched, dimensions) = stats
        .recent_vector_queries
        .remove(&(index_name.table().to_string(), index_name.to_string()))
        .unwrap();
    assert_eq!(num_searches, 1);
    // 2 dimensional vector, and qdrant uses f32 so 4-byte floats
    assert_eq!(bytes_searched, 8);
    assert_eq!(dimensions, 2);
    Ok(())
}

#[convex_macro::test_runtime]
async fn text_fields_in_segment_count_as_usage(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = TextFixtures::new(rt).await?;
    let TextIndexData { index_name, .. } = fixtures.enabled_text_index().await?;

    // Use a user transaction, not a system transaction
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = fixtures
        .db
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    add_document(&mut tx, index_name.table(), "test").await?;
    fixtures.db.commit(tx).await?;
    fixtures
        .usage_counter
        .track_call_test(tx_usage.gather_user_stats())
        .await;

    fixtures.new_live_text_flusher().step().await?;

    let storage = fixtures
        .db
        .latest_database_snapshot()?
        .get_text_index_storage(&Identity::system())?;

    let key = (ComponentPath::root(), index_name.table().clone());
    let value = storage.get(&key).copied();
    assert_eq!(value, Some(2658_u64));
    Ok(())
}

#[convex_macro::test_runtime]
async fn text_index_backfilled_unstaged_does_not_count_as_usage(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let fixtures = TextFixtures::new(rt).await?;
    let TextIndexData { index_name, .. } = fixtures.backfilled_text_index().await?;
    let mut tx = fixtures.db.begin_system().await?;
    add_document(&mut tx, index_name.table(), "test").await?;
    fixtures.db.commit(tx).await?;
    fixtures.new_live_text_flusher().step().await?;

    let storage = fixtures
        .db
        .latest_database_snapshot()?
        .get_text_index_storage(&Identity::system())?;
    let key = (ComponentPath::root(), index_name.table().clone());
    let value = storage.get(&key).copied();
    assert_eq!(value, Some(0_u64));
    Ok(())
}

#[convex_macro::test_runtime]
async fn text_index_backfilled_staged_counts_as_usage(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = TextFixtures::new(rt).await?;

    let TextIndexData { index_name, .. } = fixtures.staged_backfilled_text_index().await?;
    let mut tx = fixtures.db.begin_system().await?;
    add_document(&mut tx, index_name.table(), "test").await?;
    fixtures.db.commit(tx).await?;
    fixtures.new_live_text_flusher().step().await?;

    let storage = fixtures
        .db
        .latest_database_snapshot()?
        .get_text_index_storage(&Identity::system())?;
    let key = (ComponentPath::root(), index_name.table().clone());
    let value = storage.get(&key).copied();
    assert_eq!(value, Some(2657_u64));
    Ok(())
}

#[convex_macro::test_runtime]
async fn text_insert_with_no_index_does_not_count_usage(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = TextFixtures::new(rt).await?;

    let table_name: TableName = "my_table".parse()?;
    // Use a user transaction, not a system transaction
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = fixtures
        .db
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    add_document(&mut tx, &table_name, "hello").await?;
    fixtures.db.commit(tx).await?;
    fixtures
        .usage_counter
        .track_call_test(tx_usage.gather_user_stats())
        .await;

    let stats = fixtures.test_usage_logger.collect();
    assert!(stats.recent_text_ingress_size.is_empty());
    Ok(())
}

#[convex_macro::test_runtime]
async fn text_insert_counts_usage_for_backfilling_indexes(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = TextFixtures::new(rt).await?;
    let TextIndexData {
        index_name,
        tantivy_schema,
        ..
    } = fixtures.insert_backfilling_text_index().await?;

    // Use a user transaction, not a system transaction
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = fixtures
        .db
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    let document_id = add_document(&mut tx, index_name.table(), "hello").await?;
    let document = tx.get(document_id).await?.unwrap();
    fixtures.db.commit(tx).await?;
    fixtures
        .usage_counter
        .track_call_test(tx_usage.gather_user_stats())
        .await;

    let stats = fixtures.test_usage_logger.collect();
    let value = stats
        .recent_text_ingress_size
        .get(&(*index_name.table()).to_string())
        .cloned();

    let expected_size = tantivy_schema.estimate_size(&document);
    assert_eq!(value, Some(expected_size));
    Ok(())
}

#[convex_macro::test_runtime]
async fn text_insert_counts_usage_for_enabled_indexes(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = TextFixtures::new(rt).await?;
    let TextIndexData {
        index_name,
        tantivy_schema,
        ..
    } = fixtures.enabled_text_index().await?;

    // Use a user transaction, not a system transaction
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = fixtures
        .db
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    let document_id = add_document(&mut tx, index_name.table(), "hello").await?;
    let document = tx.get(document_id).await?.unwrap();
    fixtures.db.commit(tx).await?;
    fixtures
        .usage_counter
        .track_call_test(tx_usage.gather_user_stats())
        .await;

    let stats = fixtures.test_usage_logger.collect();
    let value = stats
        .recent_text_ingress_size
        .get(&(*index_name.table()).to_string())
        .cloned();

    let expected_size = tantivy_schema.estimate_size(&document);
    assert_eq!(value, Some(expected_size));
    Ok(())
}

#[convex_macro::test_runtime]
async fn text_query_counts_usage(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = TextFixtures::new(rt).await?;
    let TextIndexData { index_name, .. } = fixtures.enabled_text_index().await?;

    // Use a user transaction, not a system transaction
    let mut tx = fixtures.db.begin_system().await?;
    add_document(&mut tx, index_name.table(), "hello").await?;
    fixtures.db.commit(tx).await?;
    fixtures.new_live_text_flusher().step().await?;
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = fixtures
        .db
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    fixtures
        .search_with_tx(&mut tx, index_name.clone(), "he")
        .await?;
    fixtures
        .search_with_tx(&mut tx, index_name.clone(), "he")
        .await?;
    fixtures
        .usage_counter
        .track_call_test(tx_usage.gather_user_stats())
        .await;

    let stats = fixtures.test_usage_logger.collect();
    let (num_searches, bytes_searched) = stats
        .recent_text_queries
        .get(&(index_name.table().to_string(), index_name.to_string()))
        .unwrap();

    assert_eq!(*num_searches, 2);
    assert_eq!(*bytes_searched, 2 * 2659);
    Ok(())
}

#[convex_macro::test_runtime]
async fn empty_text_query_does_not_count_usage(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = TextFixtures::new(rt).await?;
    let TextIndexData { index_name, .. } = fixtures.enabled_text_index().await?;

    // Use a user transaction, not a system transaction
    let mut tx = fixtures.db.begin_system().await?;
    add_document(&mut tx, index_name.table(), "hello").await?;
    fixtures.db.commit(tx).await?;
    fixtures.new_live_text_flusher().step().await?;
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = fixtures
        .db
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    fixtures
        .search_with_tx(&mut tx, index_name.clone(), "")
        .await?;
    fixtures
        .usage_counter
        .track_call_test(tx_usage.gather_user_stats())
        .await;

    let stats = fixtures.test_usage_logger.collect();
    assert!(!stats
        .recent_text_queries
        .contains_key(&(index_name.table().to_string(), index_name.to_string())));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_usage_tracking_basic_insert_and_get(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures {
        db,
        test_usage_logger,
        usage_counter,
        ..
    } = DbFixtures::new(&rt).await?;

    let tx_usage = FunctionUsageTracker::new();
    let mut tx = db
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    let obj = assert_obj!("key" => vec![0; 100]);
    let table_name: TableName = "my_table".parse()?;
    let doc_id = TestFacingModel::new(&mut tx)
        .insert(&table_name, obj.clone())
        .await?;
    db.commit(tx).await?;
    usage_counter
        .track_call_test(tx_usage.gather_user_stats())
        .await;

    let stats = test_usage_logger.collect();
    // Database ingress counted for write to user table, rounded up
    let mut database_ingress = stats.recent_database_ingress_size;
    assert_eq!(database_ingress.len(), 1);
    assert!(database_ingress.contains_key("my_table"));
    let document = db.begin_system().await?.get(doc_id).await?.unwrap();
    assert_eq!(
        database_ingress.remove("my_table"),
        Some(document.size() as u64)
    );
    let database_egress = stats.recent_database_egress_size;
    assert_eq!(database_egress.values().sum::<u64>(), 0);

    let mut database_ingress_v2 = stats.recent_database_ingress_size_v2;
    assert_eq!(database_ingress_v2.len(), 1);
    assert!(database_ingress_v2.contains_key("my_table"));
    assert_eq!(
        database_ingress_v2.remove("my_table"),
        Some(document.size() as u64)
    );
    let database_egress_v2 = stats.recent_database_egress_size_v2;
    assert_eq!(database_egress_v2.values().sum::<u64>(), 0);

    // Database egress counted for read to user table, rounded up
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = db
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    UserFacingModel::new_root_for_test(&mut tx)
        .get_with_ts(doc_id.developer_id, None)
        .await?;
    db.commit(tx).await?;
    usage_counter
        .track_call_test(tx_usage.gather_user_stats())
        .await;

    let stats = test_usage_logger.collect();
    let database_ingress = stats.recent_database_ingress_size;
    assert_eq!(database_ingress.values().sum::<u64>(), 0);
    let mut database_egress = stats.recent_database_egress_size;
    assert_eq!(database_egress.len(), 1);
    assert!(database_egress.contains_key("my_table"));
    assert_eq!(
        database_egress.remove("my_table"),
        Some(document.size() as u64)
    );

    let database_ingress_v2 = stats.recent_database_ingress_size_v2;
    assert_eq!(database_ingress_v2.len(), 1);
    let mut database_egress_v2 = stats.recent_database_egress_size_v2;
    assert_eq!(database_egress_v2.len(), 1);
    assert!(database_egress_v2.contains_key("my_table"));
    assert_eq!(
        database_egress_v2.remove("my_table"),
        Some(document.size() as u64)
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_usage_tracking_insert_with_index(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures {
        db,
        test_usage_logger,
        usage_counter,
        ..
    } = DbFixtures::new(&rt).await?;

    // Add a user index
    let table_name: TableName = "my_table".parse()?;
    let namespace = TableNamespace::test_user();
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = db
        .begin_with_usage(Identity::system(), tx_usage.clone())
        .await?;
    let index_name = IndexName::new(table_name.clone(), IndexDescriptor::new("by_key")?)?;
    let fields: IndexedFields = vec!["key".parse()?].try_into()?;
    IndexModel::new(&mut tx)
        .add_application_index(
            namespace,
            IndexMetadata::new_enabled(index_name.clone(), fields.clone()),
        )
        .await
        .unwrap_or_else(|e| panic!("Failed to add index for {} {:?}", "by_key", e));
    db.commit(tx).await?;
    usage_counter
        .track_call_test(tx_usage.gather_user_stats())
        .await;

    let tx_usage = FunctionUsageTracker::new();
    let mut tx = db
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    let obj = assert_obj!("key" => 1);
    let obj2 = assert_obj!("key" => 3);
    let obj3 = assert_obj!("key" => 1);
    let doc_id1 = TestFacingModel::new(&mut tx)
        .insert(&table_name, obj.clone())
        .await?;
    let doc_id2 = TestFacingModel::new(&mut tx)
        .insert(&table_name, obj2.clone())
        .await?;
    let doc_id3 = TestFacingModel::new(&mut tx)
        .insert(&table_name, obj3.clone())
        .await?;
    db.commit(tx).await?;
    usage_counter
        .track_call_test(tx_usage.gather_user_stats())
        .await;

    let mut tx = db.begin_system().await?;
    let doc1 = tx.get(doc_id1).await?.unwrap();
    let doc2 = tx.get(doc_id2).await?.unwrap();
    let doc3 = tx.get(doc_id3).await?.unwrap();

    let stats = test_usage_logger.collect();
    let mut database_ingress = stats.recent_database_ingress_size;
    assert_eq!(database_ingress.len(), 1);
    assert!(database_ingress.contains_key("my_table"));
    assert_eq!(
        database_ingress.remove("my_table"),
        // double it for the index
        Some((doc1.size() + doc2.size() + doc3.size()) as u64 * 2)
    );
    let database_egress = stats.recent_database_egress_size;
    assert_eq!(database_egress.values().sum::<u64>(), 0);

    let mut database_ingress_v2 = stats.recent_database_ingress_size_v2;
    assert_eq!(database_ingress_v2.len(), 1);
    assert!(database_ingress_v2.contains_key("my_table"));
    assert_eq!(
        database_ingress_v2.remove("my_table"),
        Some((doc1.size() + doc2.size() + doc3.size()) as u64 * 2)
    );
    let database_egress_v2 = stats.recent_database_egress_size_v2;
    assert_eq!(database_egress_v2.values().sum::<u64>(), 0);

    let tx_usage = FunctionUsageTracker::new();
    let mut tx = db
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    let index_query = Query::index_range(IndexRange {
        index_name,
        range: vec![IndexRangeExpression::Eq("key".parse()?, maybe_val!(1))],
        order: Order::Asc,
    });
    let mut query_stream = ResolvedQuery::new(&mut tx, namespace, index_query)?;
    while query_stream.next(&mut tx, None).await?.is_some() {}
    db.commit(tx).await?;
    usage_counter
        .track_call_test(tx_usage.gather_user_stats())
        .await;

    let stats = test_usage_logger.collect();
    let database_ingress = stats.recent_database_ingress_size;
    assert_eq!(database_ingress.values().sum::<u64>(), 0);
    let mut database_egress = stats.recent_database_egress_size;
    assert_eq!(database_egress.len(), 1);
    assert!(database_egress.contains_key("my_table"));
    assert_eq!(
        database_egress.remove("my_table"),
        Some(
            (doc1.size()
                + doc3.size()
                + PackedDocument::pack(&doc1).index_key_bytes(&fields).len()
                + PackedDocument::pack(&doc3).index_key_bytes(&fields).len()) as u64
        )
    );

    let database_ingress_v2 = stats.recent_database_ingress_size_v2;
    assert_eq!(database_ingress_v2.values().sum::<u64>(), 0);
    let mut database_egress_v2 = stats.recent_database_egress_size_v2;
    assert_eq!(database_egress_v2.len(), 1);
    assert!(database_egress_v2.contains_key("my_table"));
    assert_eq!(
        database_egress_v2.remove("my_table"),
        Some(
            (doc1.size()
                + doc3.size()
                + PackedDocument::pack(&doc1).index_key_bytes(&fields).len()
                + PackedDocument::pack(&doc3).index_key_bytes(&fields).len()) as u64
        )
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_action_counts_compute(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures {
        test_usage_logger,
        usage_counter,
        ..
    } = DbFixtures::new(&rt).await?;

    let tx_usage = FunctionUsageTracker::new();
    usage_counter
        .track_call_test(tx_usage.gather_user_stats())
        .await;
    let stats = test_usage_logger.collect();
    assert_eq!(
        stats.recent_node_action_compute_time.values().sum::<u64>(),
        0
    );
    assert_eq!(
        stats.recent_v8_action_compute_time.values().sum::<u64>(),
        100000
    );
    assert_eq!(
        *stats
            .recent_v8_action_compute_time
            .get("test.js:default")
            .unwrap(),
        100000
    );
    Ok(())
}
