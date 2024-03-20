use std::{
    convert::TryInto,
    time::Duration,
};

use common::{
    assert_obj,
    bootstrap_model::index::IndexMetadata,
    maybe_val,
    query::{
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
    },
    types::{
        IndexName,
        ModuleEnvironment,
        TableName,
        UdfIdentifier,
    },
};
use keybroker::Identity;
use maplit::btreeset;
use pretty_assertions::assert_eq;
use request_context::ExecutionId;
use runtime::testing::TestRuntime;
use usage_tracking::{
    CallType,
    FunctionUsageTracker,
    KB,
};
use vector::VectorSearch;

use crate::{
    test_helpers::new_test_database,
    tests::vector_test_utils::{
        add_document_vec_array,
        IndexData,
        VectorFixtures,
    },
    IndexModel,
    ResolvedQuery,
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
        .begin_with_usage(Identity::Unknown, tx_usage.clone())
        .await?;
    add_document_vec_array(&mut tx, &table_name, [3f64, 4f64]).await?;
    fixtures.db.commit(tx).await?;
    fixtures.db.usage_counter().track_call(
        UdfIdentifier::Function("test.js:default".parse()?),
        ExecutionId::new(),
        CallType::Action {
            env: ModuleEnvironment::Isolate,
            duration: Duration::from_secs(10),
            memory_in_mb: 10,
        },
        tx_usage.gather_user_stats(),
    );

    let stats = fixtures.db.usage_counter().collect();
    assert!(stats.recent_vector_ingress_size.is_empty());
    Ok(())
}

#[convex_macro::test_runtime]
async fn vector_insert_counts_usage_for_backfilling_indexes(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = VectorFixtures::new(rt).await?;
    let IndexData { index_name, .. } = fixtures.backfilling_vector_index().await?;

    // Use a user transaction, not a system transaction
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = fixtures
        .db
        .begin_with_usage(Identity::Unknown, tx_usage.clone())
        .await?;
    add_document_vec_array(&mut tx, index_name.table(), [3f64, 4f64]).await?;
    fixtures.db.commit(tx).await?;
    fixtures.db.usage_counter().track_call(
        UdfIdentifier::Function("test.js:default".parse()?),
        ExecutionId::new(),
        CallType::Mutation,
        tx_usage.gather_user_stats(),
    );

    fixtures
        .add_document_vec_array(index_name.table(), [3f64, 4f64])
        .await?;
    let stats = fixtures.db.usage_counter().collect();
    let value = stats
        .recent_vector_ingress_size
        .get(&(*index_name.table()).to_string())
        .cloned();

    // round up.
    assert_eq!(value, Some(1024));
    Ok(())
}

#[convex_macro::test_runtime]
async fn vector_insert_counts_usage_for_enabled_indexes(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = VectorFixtures::new(rt).await?;
    let IndexData { index_name, .. } = fixtures.enabled_vector_index().await?;

    // Use a user transaction, not a system transaction
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = fixtures
        .db
        .begin_with_usage(Identity::Unknown, tx_usage.clone())
        .await?;
    add_document_vec_array(&mut tx, index_name.table(), [3f64, 4f64]).await?;
    fixtures.db.commit(tx).await?;
    fixtures.db.usage_counter().track_call(
        UdfIdentifier::Function("test.js:default".parse()?),
        ExecutionId::new(),
        CallType::Action {
            env: ModuleEnvironment::Isolate,
            duration: Duration::from_secs(10),
            memory_in_mb: 10,
        },
        tx_usage.gather_user_stats(),
    );

    let stats = fixtures.db.usage_counter().collect();
    let value = stats
        .recent_vector_ingress_size
        .get(&(*index_name.table()).to_string())
        .cloned();
    // We round up to the nearest KB
    assert_eq!(value, Some(1024_u64));
    Ok(())
}

#[convex_macro::test_runtime]
async fn vectors_in_segment_count_as_usage(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = VectorFixtures::new(rt).await?;
    let IndexData { index_name, .. } = fixtures.enabled_vector_index().await?;

    // Use a user transaction, not a system transaction
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = fixtures
        .db
        .begin_with_usage(Identity::Unknown, tx_usage.clone())
        .await?;
    add_document_vec_array(&mut tx, index_name.table(), [3f64, 4f64]).await?;
    fixtures.db.commit(tx).await?;
    fixtures.db.usage_counter().track_call(
        UdfIdentifier::Function("test.js:default".parse()?),
        ExecutionId::new(),
        CallType::Action {
            env: ModuleEnvironment::Isolate,
            duration: Duration::from_secs(10),
            memory_in_mb: 10,
        },
        tx_usage.gather_user_stats(),
    );

    fixtures.new_index_flusher()?.step().await?;

    let storage = fixtures.db.get_vector_index_storage(Identity::system())?;

    let value = storage.get(&(*index_name.table()).to_string()).cloned();
    assert_eq!(value, Some(8_u64));
    Ok(())
}

#[convex_macro::test_runtime]
async fn vector_query_counts_bandwidth(rt: TestRuntime) -> anyhow::Result<()> {
    let fixtures = VectorFixtures::new(rt).await?;
    let IndexData { index_name, .. } = fixtures.enabled_vector_index().await?;

    // Use a user transaction, not a system transaction
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = fixtures
        .db
        .begin_with_usage(Identity::Unknown, tx_usage.clone())
        .await?;
    add_document_vec_array(&mut tx, index_name.table(), [3f64, 4f64]).await?;
    fixtures.db.commit(tx).await?;
    fixtures.new_index_flusher()?.step().await?;

    let (_, usage_stats) = fixtures
        .db
        .vector_search(
            Identity::Unknown,
            VectorSearch {
                index_name: index_name.clone(),
                limit: Some(10),
                vector: vec![0.; 2],
                expressions: btreeset![],
            },
        )
        .await?;
    tx_usage.add(usage_stats);

    fixtures.db.usage_counter().track_call(
        UdfIdentifier::Function("test.js:default".parse()?),
        ExecutionId::new(),
        CallType::Action {
            env: ModuleEnvironment::Isolate,
            duration: Duration::from_secs(10),
            memory_in_mb: 10,
        },
        tx_usage.gather_user_stats(),
    );

    let stats = fixtures.db.usage_counter().collect();
    let vector_egress = stats.recent_vector_egress_size;
    let bandwidth_egress = stats.recent_database_egress_size;
    // Rounded up.
    assert_eq!(
        *vector_egress.get(&index_name.table().to_string()).unwrap(),
        KB
    );
    assert_eq!(
        *bandwidth_egress
            .get(&index_name.table().to_string())
            .unwrap(),
        KB
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_usage_tracking_basic_insert_and_get(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt.clone()).await;

    let tx_usage = FunctionUsageTracker::new();
    let mut tx = db
        .begin_with_usage(Identity::Unknown, tx_usage.clone())
        .await?;
    let obj = assert_obj!("key" => vec![0; 100]);
    let table_name: TableName = "my_table".parse()?;
    let doc_id = UserFacingModel::new(&mut tx)
        .insert(table_name.clone(), obj.clone())
        .await?;
    db.commit(tx).await?;
    db.usage_counter().track_call(
        UdfIdentifier::Function("test.js:default".parse()?),
        ExecutionId::new(),
        CallType::Mutation,
        tx_usage.gather_user_stats(),
    );

    let stats = db.usage_counter().collect();
    // Database ingress counted for write to user table, rounded up
    let database_ingress = stats.recent_database_ingress_size;
    assert_eq!(database_ingress.len(), 1);
    assert!(database_ingress.contains_key("my_table"));
    assert_eq!(*database_ingress.get("my_table").unwrap(), KB);
    let database_egress = stats.recent_database_egress_size;
    assert!(database_egress.is_empty());

    // Database egress counted for read to user table, rounded up
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = db
        .begin_with_usage(Identity::Unknown, tx_usage.clone())
        .await?;
    UserFacingModel::new(&mut tx)
        .get_with_ts(doc_id, None)
        .await?;
    db.commit(tx).await?;
    db.usage_counter().track_call(
        UdfIdentifier::Function("test.js:default".parse()?),
        ExecutionId::new(),
        CallType::Mutation,
        tx_usage.gather_user_stats(),
    );

    let stats = db.usage_counter().collect();
    let database_ingress = stats.recent_database_ingress_size;
    assert!(database_ingress.is_empty());
    let database_egress = stats.recent_database_egress_size;
    assert_eq!(database_egress.len(), 1);
    assert!(database_egress.contains_key("my_table"));
    assert_eq!(*database_egress.get("my_table").unwrap(), KB);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_usage_tracking_insert_with_index(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt.clone()).await;

    // Add a user index
    let table_name: TableName = "my_table".parse()?;
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = db
        .begin_with_usage(Identity::system(), tx_usage.clone())
        .await?;
    let index_name = IndexName::new(table_name.clone(), "by_key".parse()?)?;
    IndexModel::new(&mut tx)
        .add_application_index(IndexMetadata::new_enabled(
            index_name.clone(),
            vec!["key".parse()?].try_into()?,
        ))
        .await
        .unwrap_or_else(|e| panic!("Failed to add index for {} {:?}", "by_key", e));
    db.commit(tx).await?;
    db.usage_counter().track_call(
        UdfIdentifier::Function("test.js:default".parse()?),
        ExecutionId::new(),
        CallType::Mutation,
        tx_usage.gather_user_stats(),
    );

    let tx_usage = FunctionUsageTracker::new();
    let mut tx = db
        .begin_with_usage(Identity::Unknown, tx_usage.clone())
        .await?;
    let obj = assert_obj!("key" => 1);
    let obj2 = assert_obj!("key" => 3);
    let obj3 = assert_obj!("key" => 1);
    UserFacingModel::new(&mut tx)
        .insert(table_name.clone(), obj.clone())
        .await?;
    UserFacingModel::new(&mut tx)
        .insert(table_name.clone(), obj2.clone())
        .await?;
    UserFacingModel::new(&mut tx)
        .insert(table_name.clone(), obj3.clone())
        .await?;
    db.commit(tx).await?;
    db.usage_counter().track_call(
        UdfIdentifier::Function("test.js:default".parse()?),
        ExecutionId::new(),
        CallType::Mutation,
        tx_usage.gather_user_stats(),
    );

    let stats = db.usage_counter().collect();
    // Database bandwidth ingress is 6 KB from 3 document writes and 3 index writes
    let database_ingress = stats.recent_database_ingress_size;
    assert_eq!(database_ingress.len(), 1);
    assert!(database_ingress.contains_key("my_table"));
    assert_eq!(*database_ingress.get("my_table").unwrap(), 6 * KB);
    let database_egress = stats.recent_database_egress_size;
    assert!(database_egress.is_empty());

    let tx_usage = FunctionUsageTracker::new();
    let mut tx = db
        .begin_with_usage(Identity::Unknown, tx_usage.clone())
        .await?;
    let index_query = Query::index_range(IndexRange {
        index_name,
        range: vec![IndexRangeExpression::Eq("key".parse()?, maybe_val!(1))],
        order: Order::Asc,
    });
    let mut query_stream = ResolvedQuery::new(&mut tx, index_query)?;
    while query_stream.next(&mut tx, None).await?.is_some() {}
    db.commit(tx).await?;
    db.usage_counter().track_call(
        UdfIdentifier::Function("test.js:default".parse()?),
        ExecutionId::new(),
        CallType::Mutation,
        tx_usage.gather_user_stats(),
    );

    let stats = db.usage_counter().collect();
    // Database bandwidth egress is 4 KB from 2 document writes and 2 index writes
    let database_ingress = stats.recent_database_ingress_size;
    assert!(database_ingress.is_empty());
    let database_egress = stats.recent_database_egress_size;
    assert_eq!(database_egress.len(), 1);
    assert!(database_egress.contains_key("my_table"));
    assert_eq!(*database_egress.get("my_table").unwrap(), 4 * KB);

    Ok(())
}

#[convex_macro::test_runtime]
async fn http_action_counts_compute(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt.clone()).await;

    let tx_usage = FunctionUsageTracker::new();
    db.usage_counter().track_call(
        UdfIdentifier::Function("test.js:default".parse()?),
        ExecutionId::new(),
        CallType::HttpAction {
            duration: Duration::from_secs(5),
            memory_in_mb: 100,
        },
        tx_usage.gather_user_stats(),
    );
    let stats = db.usage_counter().collect();
    assert_eq!(
        stats.recent_node_action_compute_time.values().sum::<u64>(),
        0
    );
    assert_eq!(
        stats.recent_v8_action_compute_time.values().sum::<u64>(),
        500000
    );
    assert_eq!(
        *stats
            .recent_v8_action_compute_time
            .get("test.js:default")
            .unwrap(),
        500000
    );
    Ok(())
}
