use std::time::Duration;

use common::{
    assert_obj,
    bootstrap_model::index::IndexMetadata,
    components::{
        ComponentFunctionPath,
        ComponentId,
        ComponentPath,
    },
    execution_context::ExecutionId,
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
        ModuleEnvironment,
        TableName,
        UdfIdentifier,
    },
    RequestId,
};
use keybroker::Identity;
use maplit::btreeset;
use pretty_assertions::assert_eq;
use runtime::testing::TestRuntime;
use sync_types::CanonicalizedUdfPath;
use usage_tracking::{
    CallType,
    FunctionUsageTracker,
};
use value::{
    sha256::Sha256Digest,
    TableNamespace,
};
use vector::VectorSearch;

use crate::{
    test_helpers::DbFixtures,
    tests::vector_test_utils::{
        add_document_vec_array,
        IndexData,
        VectorFixtures,
    },
    IndexModel,
    ResolvedQuery,
    UserFacingModel,
};

fn test_udf_identifier() -> UdfIdentifier {
    let udf_path: CanonicalizedUdfPath = "test.js:default".parse().unwrap();
    let component = ComponentPath::root();
    let path = ComponentFunctionPath {
        component,
        udf_path: udf_path.strip(),
    };
    UdfIdentifier::Function(path.canonicalize())
}

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
        .db
        .usage_counter()
        .track_call(
            test_udf_identifier(),
            ExecutionId::new(),
            RequestId::new(),
            CallType::Action {
                env: ModuleEnvironment::Isolate,
                duration: Duration::from_secs(10),
                memory_in_mb: 10,
            },
            true,
            tx_usage.gather_user_stats(),
        )
        .await;

    let stats = fixtures.test_usage_logger.collect();
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
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    add_document_vec_array(&mut tx, index_name.table(), [3f64, 4f64]).await?;
    fixtures.db.commit(tx).await?;
    fixtures
        .db
        .usage_counter()
        .track_call(
            test_udf_identifier(),
            ExecutionId::new(),
            RequestId::new(),
            CallType::Mutation { occ_info: None },
            true,
            tx_usage.gather_user_stats(),
        )
        .await;

    fixtures
        .add_document_vec_array(index_name.table(), [3f64, 4f64])
        .await?;
    let stats = fixtures.test_usage_logger.collect();
    let value = stats
        .recent_vector_ingress_size
        .get(&(*index_name.table()).to_string())
        .cloned();

    assert!(value.unwrap() > 0);
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
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    add_document_vec_array(&mut tx, index_name.table(), [3f64, 4f64]).await?;
    fixtures.db.commit(tx).await?;
    fixtures
        .db
        .usage_counter()
        .track_call(
            test_udf_identifier(),
            ExecutionId::new(),
            RequestId::new(),
            CallType::Action {
                env: ModuleEnvironment::Isolate,
                duration: Duration::from_secs(10),
                memory_in_mb: 10,
            },
            true,
            tx_usage.gather_user_stats(),
        )
        .await;

    let stats = fixtures.test_usage_logger.collect();
    let value = stats
        .recent_vector_ingress_size
        .get(&(*index_name.table()).to_string())
        .cloned();
    assert!(value.unwrap() > 0);
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
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    add_document_vec_array(&mut tx, index_name.table(), [3f64, 4f64]).await?;
    fixtures.db.commit(tx).await?;
    fixtures
        .db
        .usage_counter()
        .track_call(
            test_udf_identifier(),
            ExecutionId::new(),
            RequestId::new(),
            CallType::Action {
                env: ModuleEnvironment::Isolate,
                duration: Duration::from_secs(10),
                memory_in_mb: 10,
            },
            true,
            tx_usage.gather_user_stats(),
        )
        .await;

    fixtures.new_index_flusher()?.step().await?;

    let storage = fixtures
        .db
        .get_vector_index_storage(Identity::system())
        .await?;

    let key = (ComponentPath::root(), index_name.table().clone());
    let value = storage.get(&key).cloned();
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
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    add_document_vec_array(&mut tx, index_name.table(), [3f64, 4f64]).await?;
    fixtures.db.commit(tx).await?;
    fixtures.new_index_flusher()?.step().await?;

    let (_, usage_stats) = fixtures
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
        .db
        .usage_counter()
        .track_call(
            test_udf_identifier(),
            ExecutionId::new(),
            RequestId::new(),
            CallType::Action {
                env: ModuleEnvironment::Isolate,
                duration: Duration::from_secs(10),
                memory_in_mb: 10,
            },
            true,
            tx_usage.gather_user_stats(),
        )
        .await;

    let stats = fixtures.test_usage_logger.collect();
    let vector_egress = stats.recent_vector_egress_size;
    let bandwidth_egress = stats.recent_database_egress_size;
    assert!(*vector_egress.get(&index_name.table().to_string()).unwrap() > 0);
    assert!(
        *bandwidth_egress
            .get(&index_name.table().to_string())
            .unwrap()
            > 0
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_usage_tracking_basic_insert_and_get(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures {
        db,
        test_usage_logger,
        ..
    } = DbFixtures::new(&rt).await?;

    let tx_usage = FunctionUsageTracker::new();
    let mut tx = db
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    let obj = assert_obj!("key" => vec![0; 100]);
    let table_name: TableName = "my_table".parse()?;
    let doc_id = UserFacingModel::new_root_for_test(&mut tx)
        .insert(table_name.clone(), obj.clone())
        .await?;
    db.commit(tx).await?;
    db.usage_counter()
        .track_call(
            test_udf_identifier(),
            ExecutionId::new(),
            RequestId::new(),
            CallType::Mutation { occ_info: None },
            true,
            tx_usage.gather_user_stats(),
        )
        .await;

    let stats = test_usage_logger.collect();
    // Database ingress counted for write to user table, rounded up
    let database_ingress = stats.recent_database_ingress_size;
    assert_eq!(database_ingress.len(), 1);
    assert!(database_ingress.contains_key("my_table"));
    assert!(*database_ingress.get("my_table").unwrap() > 0);
    let database_egress = stats.recent_database_egress_size;
    assert_eq!(database_egress.values().sum::<u64>(), 0);

    // Database egress counted for read to user table, rounded up
    let tx_usage = FunctionUsageTracker::new();
    let mut tx = db
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    UserFacingModel::new_root_for_test(&mut tx)
        .get_with_ts(doc_id, None)
        .await?;
    db.commit(tx).await?;
    db.usage_counter()
        .track_call(
            test_udf_identifier(),
            ExecutionId::new(),
            RequestId::new(),
            CallType::Mutation { occ_info: None },
            true,
            tx_usage.gather_user_stats(),
        )
        .await;

    let stats = test_usage_logger.collect();
    let database_ingress = stats.recent_database_ingress_size;
    assert_eq!(database_ingress.values().sum::<u64>(), 0);
    let database_egress = stats.recent_database_egress_size;
    assert_eq!(database_egress.len(), 1);
    assert!(database_egress.contains_key("my_table"));
    assert!(*database_egress.get("my_table").unwrap() > 0);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_usage_tracking_insert_with_index(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures {
        db,
        test_usage_logger,
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
    IndexModel::new(&mut tx)
        .add_application_index(
            namespace,
            IndexMetadata::new_enabled(index_name.clone(), vec!["key".parse()?].try_into()?),
        )
        .await
        .unwrap_or_else(|e| panic!("Failed to add index for {} {:?}", "by_key", e));
    db.commit(tx).await?;
    db.usage_counter()
        .track_call(
            test_udf_identifier(),
            ExecutionId::new(),
            RequestId::new(),
            CallType::Mutation { occ_info: None },
            true,
            tx_usage.gather_user_stats(),
        )
        .await;

    let tx_usage = FunctionUsageTracker::new();
    let mut tx = db
        .begin_with_usage(Identity::Unknown(None), tx_usage.clone())
        .await?;
    let obj = assert_obj!("key" => 1);
    let obj2 = assert_obj!("key" => 3);
    let obj3 = assert_obj!("key" => 1);
    UserFacingModel::new_root_for_test(&mut tx)
        .insert(table_name.clone(), obj.clone())
        .await?;
    UserFacingModel::new_root_for_test(&mut tx)
        .insert(table_name.clone(), obj2.clone())
        .await?;
    UserFacingModel::new_root_for_test(&mut tx)
        .insert(table_name.clone(), obj3.clone())
        .await?;
    db.commit(tx).await?;
    db.usage_counter()
        .track_call(
            test_udf_identifier(),
            ExecutionId::new(),
            RequestId::new(),
            CallType::Mutation { occ_info: None },
            true,
            tx_usage.gather_user_stats(),
        )
        .await;

    let stats = test_usage_logger.collect();
    let database_ingress = stats.recent_database_ingress_size;
    assert_eq!(database_ingress.len(), 1);
    assert!(database_ingress.contains_key("my_table"));
    assert!(*database_ingress.get("my_table").unwrap() > 0);
    let database_egress = stats.recent_database_egress_size;
    assert_eq!(database_egress.values().sum::<u64>(), 0);

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
    db.usage_counter()
        .track_call(
            test_udf_identifier(),
            ExecutionId::new(),
            RequestId::new(),
            CallType::Mutation { occ_info: None },
            true,
            tx_usage.gather_user_stats(),
        )
        .await;

    let stats = test_usage_logger.collect();
    let database_ingress = stats.recent_database_ingress_size;
    assert_eq!(database_ingress.values().sum::<u64>(), 0);
    let database_egress = stats.recent_database_egress_size;
    assert_eq!(database_egress.len(), 1);
    assert!(database_egress.contains_key("my_table"));
    assert!(*database_egress.get("my_table").unwrap() > 0);

    Ok(())
}

#[convex_macro::test_runtime]
async fn http_action_counts_compute(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures {
        db,
        test_usage_logger,
        ..
    } = DbFixtures::new(&rt).await?;

    let tx_usage = FunctionUsageTracker::new();
    db.usage_counter()
        .track_call(
            test_udf_identifier(),
            ExecutionId::new(),
            RequestId::new(),
            CallType::HttpAction {
                duration: Duration::from_secs(5),
                memory_in_mb: 100,
                response_sha256: Sha256Digest::from([0; 32]),
            },
            true,
            tx_usage.gather_user_stats(),
        )
        .await;
    let stats = test_usage_logger.collect();
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
