use chrono::DateTime;
use common::{
    bootstrap_model::index::IndexMetadata,
    components::ComponentPath,
    query::{
        Order,
        Query,
    },
    types::IndexName,
};
use database::{
    IndexModel,
    ResolvedQuery,
    TableModel,
    UserFacingModel,
};
use errors::ErrorMetadataAnyhowExt;
use fivetran_destination::{
    api_types::{
        BatchWriteOperation,
        BatchWriteRow,
        DeleteType,
    },
    constants::{
        FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR,
        FIVETRAN_SYNCED_INDEX_DESCRIPTOR,
    },
};
use itertools::Itertools;
use keybroker::Identity;
use must_let::must_let;
use runtime::testing::TestRuntime;
use value::{
    assert_obj,
    ConvexValue,
    TableName,
    TableNamespace,
};

use crate::{
    test_helpers::ApplicationTestExt,
    Application,
};

#[convex_macro::test_runtime]
async fn test_create_new_row(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let table: TableName = "users".parse()?;
    IndexModel::new(&mut tx)
        .add_system_index(
            TableNamespace::test_user(),
            IndexMetadata::new_enabled(
                IndexName::new_reserved(
                    table.clone(),
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.clone(),
                )?,
                vec!["id".parse()?].try_into()?,
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    let row = assert_obj!(
        "id" => ConvexValue::Int64(42),
        "fivetran" => assert_obj!(
            "synced" => ConvexValue::Float64(1715172902504.0),
        ),
    );

    let usage = usage_tracking::FunctionUsageTracker::new();
    application
        .apply_fivetran_operations(
            &Identity::system(),
            vec![BatchWriteRow {
                table: table.to_string(),
                operation: BatchWriteOperation::Upsert,
                row,
            }],
            usage.clone(),
        )
        .await?;

    // Get the actual document to calculate expected ingress (includes system fields
    // and index overhead)
    let mut tx = application.begin(Identity::system()).await?;
    let mut query_stream = database::ResolvedQuery::new(
        &mut tx,
        TableNamespace::Global,
        common::query::Query::full_table_scan(table.clone(), common::query::Order::Asc),
    )?;
    let doc = query_stream.next(&mut tx, None).await?.unwrap();
    // Ingress accounts for the document plus index writes
    let expected_ingress = doc.size() as u64;

    // Verify usage stats are tracked for database writes
    let usage_stats = usage.gather_user_stats();
    let ingress = usage_stats
        .database_ingress_v2
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .expect("Expected database_ingress_v2 to be tracked");
    assert_eq!(
        ingress, expected_ingress,
        "Expected exact ingress for inserting row"
    );
    // Verify v1 ingress is not used for streaming imports
    let v1_ingress = usage_stats
        .database_ingress
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .unwrap_or(0);
    assert_eq!(
        v1_ingress, 0,
        "Expected database_ingress (v1) to be 0 for streaming import, only v2 should be used"
    );

    let mut tx = application.begin(Identity::system()).await?;
    assert!(
        !TableModel::new(&mut tx)
            .table_is_empty(TableNamespace::Global, &table)
            .await?
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_update_row(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let table: TableName = "posts".parse()?;
    IndexModel::new(&mut tx)
        .add_system_index(
            TableNamespace::test_user(),
            IndexMetadata::new_enabled(
                IndexName::new_reserved(
                    table.clone(),
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.clone(),
                )?,
                vec!["fivetran.deleted".parse()?, "fivetran.id".parse()?].try_into()?,
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    let mut tx = application.begin(Identity::system()).await?;
    let document_id = UserFacingModel::new_root_for_test(&mut tx)
        .insert(
            table.clone(),
            assert_obj!(
                "name" => "How Convex Works",
                "otherField" => ConvexValue::Boolean(true),
                "objectField" => assert_obj!(
                    "field_a" => true,
                    "field_b" => true,
                ),
                "fivetran" => assert_obj!(
                    "id" => ConvexValue::Int64(42),
                    "deleted" => false,
                    "synced" => ConvexValue::Float64(1715172902504.0),
                ),
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    let row = assert_obj!(
        "name" => "How Convex Rocks",
        // A new value for otherField isn't specified → the old value is kept
        "objectField" => assert_obj!(
            "field_a" => true,
            "field_z" => true,
        ),
        "fivetran" => assert_obj!(
            "id" => ConvexValue::Int64(42),
            "synced" => ConvexValue::Float64(1715176847664.0),
        ),
    );

    let usage = usage_tracking::FunctionUsageTracker::new();
    application
        .apply_fivetran_operations(
            &Identity::system(),
            vec![BatchWriteRow {
                table: table.clone().to_string(),
                operation: BatchWriteOperation::Update,
                row,
            }],
            usage.clone(),
        )
        .await?;

    // Get the actual document to calculate expected ingress (includes system fields
    // and index overhead)
    let mut tx = application.begin(Identity::system()).await?;
    let mut query_stream = ResolvedQuery::new(
        &mut tx,
        TableNamespace::Global,
        Query::full_table_scan(table.clone(), Order::Asc),
    )?;
    let doc = query_stream.next(&mut tx, None).await?.unwrap();
    // Ingress accounts for the document including system fields
    let expected_ingress = doc.size() as u64;

    // Verify usage stats are tracked for database operations (ingress only for
    // imports)
    let usage_stats = usage.gather_user_stats();
    let ingress = usage_stats
        .database_ingress_v2
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .expect("Expected database_ingress_v2 to be tracked");
    assert_eq!(
        ingress, expected_ingress,
        "Expected exact ingress for updating row"
    );
    // Verify v1 ingress is not used for streaming imports
    let v1_ingress = usage_stats
        .database_ingress
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .unwrap_or(0);
    assert_eq!(
        v1_ingress, 0,
        "Expected database_ingress (v1) to be 0 for streaming import, only v2 should be used"
    );

    let mut tx = application.begin(Identity::system()).await?;
    assert_eq!(
        UserFacingModel::new_root_for_test(&mut tx)
            .get(document_id, None)
            .await?
            .unwrap()
            .into_value()
            .0
            .filter_system_fields(),
        assert_obj!(
            "name" => "How Convex Rocks",
            "otherField" => ConvexValue::Boolean(true),
            "objectField" => assert_obj!( // the content of the object is entirely replaced
                "field_a" => true,
                "field_z" => true,
            ),
            "fivetran" => assert_obj!(
                "id" => ConvexValue::Int64(42),
                "deleted" => false, // in the metadata object, fields that are not set are kept
                "synced" => ConvexValue::Float64(1715176847664.0),
            ),
        )
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_soft_delete_row(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let table: TableName = "posts".parse()?;
    IndexModel::new(&mut tx)
        .add_system_index(
            TableNamespace::test_user(),
            IndexMetadata::new_enabled(
                IndexName::new_reserved(
                    table.clone(),
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.clone(),
                )?,
                vec!["fivetran.deleted".parse()?, "fivetran.id".parse()?].try_into()?,
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    let mut tx = application.begin(Identity::system()).await?;
    let document_id = UserFacingModel::new_root_for_test(&mut tx)
        .insert(
            table.clone(),
            assert_obj!(
                "name" => "How Convex Works",
                "otherField" => ConvexValue::Boolean(true),
                "objectField" => assert_obj!(
                    "field_a" => true,
                    "field_b" => true,
                ),
                "fivetran" => assert_obj!(
                    "id" => ConvexValue::Int64(42),
                    "deleted" => false,
                    "synced" => ConvexValue::Float64(1715172902504.0),
                ),
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    let row = assert_obj!(
        "name" => "How Convex Rocks",
        // A new value for otherField isn't specified → the old value is kept
        "objectField" => assert_obj!(
            "field_a" => true,
            "field_z" => true,
        ),
        "fivetran" => assert_obj!(
            "id" => ConvexValue::Int64(42),
            "deleted" => ConvexValue::Boolean(true),
        ),
    );

    let usage = usage_tracking::FunctionUsageTracker::new();
    application
        .apply_fivetran_operations(
            &Identity::system(),
            vec![BatchWriteRow {
                table: table.clone().to_string(),
                operation: BatchWriteOperation::Update,
                row,
            }],
            usage.clone(),
        )
        .await?;

    // Get the actual document to calculate expected ingress (includes system fields
    // and index overhead)
    let mut tx = application.begin(Identity::system()).await?;
    let mut query_stream = ResolvedQuery::new(
        &mut tx,
        TableNamespace::Global,
        Query::full_table_scan(table.clone(), Order::Asc),
    )?;
    let doc = query_stream.next(&mut tx, None).await?.unwrap();
    // Ingress accounts for the document including system fields
    let expected_ingress = doc.size() as u64;

    // Verify usage stats are tracked for soft delete operation (ingress only for
    // imports)
    let usage_stats = usage.gather_user_stats();
    let ingress = usage_stats
        .database_ingress_v2
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .expect("Expected database_ingress_v2 to be tracked");
    assert_eq!(
        ingress, expected_ingress,
        "Expected exact ingress for soft delete (updating deleted flag)"
    );
    // Verify v1 ingress is not used for streaming imports
    let v1_ingress = usage_stats
        .database_ingress
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .unwrap_or(0);
    assert_eq!(
        v1_ingress, 0,
        "Expected database_ingress (v1) to be 0 for streaming import, only v2 should be used"
    );

    let mut tx = application.begin(Identity::system()).await?;
    assert_eq!(
        UserFacingModel::new_root_for_test(&mut tx)
            .get(document_id, None)
            .await?
            .unwrap()
            .into_value()
            .0
            .filter_system_fields(),
        assert_obj!(
            "name" => "How Convex Rocks",
            "otherField" => ConvexValue::Boolean(true),
            "objectField" => assert_obj!( // the content of the object is entirely replaced
                "field_a" => true,
                "field_z" => true,
            ),
            "fivetran" => assert_obj!(
                "id" => ConvexValue::Int64(42),
                "deleted" => true,
                "synced" => ConvexValue::Float64(1715172902504.0), // in the metadata object, fields that are not set are kept
            ),
        )
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_update_missing_row(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let table: TableName = "posts".parse()?;
    IndexModel::new(&mut tx)
        .add_system_index(
            TableNamespace::test_user(),
            IndexMetadata::new_enabled(
                IndexName::new_reserved(
                    table.clone(),
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.clone(),
                )?,
                vec!["fivetran.id".parse()?].try_into()?,
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    let usage = usage_tracking::FunctionUsageTracker::new();
    must_let!(
        let Err(err) = application
        .apply_fivetran_operations(
            &Identity::system(),
            vec![BatchWriteRow {
                table: table.clone().to_string(),
                operation: BatchWriteOperation::Update,
                row: assert_obj!(
                    "name" => "How Convex Rocks",
                    "fivetran" => assert_obj!(
                        "id" => ConvexValue::Int64(42),
                        "synced" => ConvexValue::Float64(1715176847664.0),
                    ),
                ),
            }],
            usage,
        )
    .await);
    assert!(err.is_not_found());
    assert_eq!(err.short_msg(), "FivetranMissingUpdatedRow");

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_replace_row(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let table: TableName = "posts".parse()?;
    IndexModel::new(&mut tx)
        .add_system_index(
            TableNamespace::test_user(),
            IndexMetadata::new_enabled(
                IndexName::new_reserved(
                    table.clone(),
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.clone(),
                )?,
                vec!["id".parse()?].try_into()?,
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    let mut tx = application.begin(Identity::system()).await?;
    let document_id = UserFacingModel::new_root_for_test(&mut tx)
        .insert(
            table.clone(),
            assert_obj!(
                "id" => ConvexValue::Int64(42),
                "name" => "How Convex Works",
                "otherField" => ConvexValue::Boolean(true),
                "fivetran" => assert_obj!(
                    "synced" => ConvexValue::Float64(1715172902504.0),
                ),
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    let row = assert_obj!(
        "id" => ConvexValue::Int64(42),
        "name" => "How Convex Rocks",
        // A new value for otherField isn't specified → the old value is removed
        "fivetran" => assert_obj!(
            "synced" => ConvexValue::Float64(1715176847664.0),
        ),
    );

    let usage = usage_tracking::FunctionUsageTracker::new();
    application
        .apply_fivetran_operations(
            &Identity::system(),
            vec![BatchWriteRow {
                table: table.clone().to_string(),
                operation: BatchWriteOperation::Upsert,
                row,
            }],
            usage.clone(),
        )
        .await?;

    // Get the actual document to calculate expected ingress (includes system fields
    // and index overhead)
    let mut tx = application.begin(Identity::system()).await?;
    let mut query_stream = ResolvedQuery::new(
        &mut tx,
        TableNamespace::Global,
        Query::full_table_scan(table.clone(), Order::Asc),
    )?;
    let doc = query_stream.next(&mut tx, None).await?.unwrap();
    // Ingress accounts for the document including system fields
    let expected_ingress = doc.size() as u64;

    // Verify usage stats are tracked for upsert operation (ingress only for
    // imports)
    let usage_stats = usage.gather_user_stats();
    let ingress = usage_stats
        .database_ingress_v2
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .expect("Expected database_ingress_v2 to be tracked");
    assert_eq!(
        ingress, expected_ingress,
        "Expected exact ingress for upsert (replacing existing row)"
    );
    // Verify v1 ingress is not used for streaming imports
    let v1_ingress = usage_stats
        .database_ingress
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .unwrap_or(0);
    assert_eq!(
        v1_ingress, 0,
        "Expected database_ingress (v1) to be 0 for streaming import, only v2 should be used"
    );

    let mut tx = application.begin(Identity::system()).await?;
    assert_eq!(
        UserFacingModel::new_root_for_test(&mut tx)
            .get(document_id, None)
            .await?
            .unwrap()
            .into_value()
            .0
            .filter_system_fields(),
        assert_obj!(
            "id" => ConvexValue::Int64(42),
            "name" => "How Convex Rocks",
            "fivetran" => assert_obj!(
                "synced" => ConvexValue::Float64(1715176847664.0),
            ),
        )
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_hard_delete_row(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let table: TableName = "posts".parse()?;
    IndexModel::new(&mut tx)
        .add_system_index(
            TableNamespace::test_user(),
            IndexMetadata::new_enabled(
                IndexName::new_reserved(
                    table.clone(),
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.clone(),
                )?,
                vec!["id".parse()?].try_into()?,
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    let mut tx = application.begin(Identity::system()).await?;
    UserFacingModel::new_root_for_test(&mut tx)
        .insert(
            table.clone(),
            assert_obj!(
                "id" => ConvexValue::Int64(42),
                "name" => "How Convex Works",
                "fivetran" => assert_obj!(
                    "synced" => ConvexValue::Float64(1715172902504.0),
                ),
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    // Query the document to calculate expected egress
    let mut tx = application.begin(Identity::system()).await?;
    let mut query_stream = ResolvedQuery::new(
        &mut tx,
        TableNamespace::test_user(),
        Query::full_table_scan(table.clone(), Order::Asc),
    )?;
    let doc = query_stream.next(&mut tx, None).await?.unwrap();
    // Hard delete reads the document twice: once via primary key index lookup, once
    // for the actual document
    let expected_egress = 2 * doc.size() as u64;
    drop(query_stream);
    application.commit_test(tx).await?;

    let usage = usage_tracking::FunctionUsageTracker::new();
    application
        .apply_fivetran_operations(
            &Identity::system(),
            vec![BatchWriteRow {
                table: table.clone().to_string(),
                operation: BatchWriteOperation::HardDelete,
                row: assert_obj!(
                    "id" => ConvexValue::Int64(42),
                ),
            }],
            usage.clone(),
        )
        .await?;

    // Verify usage: hard delete reads the document (egress_v2) but doesn't write
    // (no ingress)
    let usage_stats = usage.gather_user_stats();

    // No ingress - hard delete doesn't write anything
    assert!(
        usage_stats.database_ingress_v2.is_empty(),
        "Expected no database_ingress_v2 usage from hard delete (no writes)"
    );

    // Should have exact egress_v2 from reading the document to delete
    let egress_v2 = usage_stats
        .database_egress_v2
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .unwrap_or(0);
    assert_eq!(
        egress_v2, expected_egress,
        "Expected database_egress_v2 to equal document size for hard delete"
    );

    // Verify v1 ingress is 0 (only v2 should be tracked for streaming imports)
    let v1_ingress = usage_stats
        .database_ingress
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .unwrap_or(0);
    assert_eq!(
        v1_ingress, 0,
        "Expected database_ingress (v1) to be 0 for streaming import"
    );

    let mut tx = application.begin(Identity::system()).await?;
    assert!(
        TableModel::new(&mut tx)
            .table_is_empty(TableNamespace::Global, &table)
            .await?
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_ignores_soft_deleted_rows(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let table: TableName = "posts".parse()?;
    IndexModel::new(&mut tx)
        .add_system_index(
            TableNamespace::test_user(),
            IndexMetadata::new_enabled(
                IndexName::new_reserved(
                    table.clone(),
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.clone(),
                )?,
                vec!["fivetran.synced".parse()?, "id".parse()?].try_into()?,
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    let mut tx = application.begin(Identity::system()).await?;
    UserFacingModel::new_root_for_test(&mut tx)
        .insert(
            table.clone(),
            assert_obj!(
                "id" => ConvexValue::Int64(42),
                "name" => "How Convex Works",
                "fivetran" => assert_obj!(
                    "deleted" => ConvexValue::Boolean(true),
                    "synced" => ConvexValue::Float64(1715172902504.0),
                ),
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    let row = assert_obj!(
        "id" => ConvexValue::Int64(42),
        "name" => "How Convex Rocks",
        "fivetran" => assert_obj!(
            "deleted" => ConvexValue::Boolean(true),
            "synced" => ConvexValue::Float64(1715176847664.0),
        ),
    );

    let usage = usage_tracking::FunctionUsageTracker::new();
    application
        .apply_fivetran_operations(
            &Identity::system(),
            vec![BatchWriteRow {
                table: table.clone().to_string(),
                operation: BatchWriteOperation::Upsert,
                row,
            }],
            usage.clone(),
        )
        .await?;

    // Get the actual document to calculate expected ingress (includes system fields
    // and index overhead) We need to get the second document (the newly
    // inserted one)
    let mut tx = application.begin(Identity::system()).await?;
    let mut query_stream = ResolvedQuery::new(
        &mut tx,
        TableNamespace::Global,
        Query::full_table_scan(table.clone(), Order::Asc),
    )?;
    // Skip the first document (the one that already existed)
    let _first_doc = query_stream.next(&mut tx, None).await?.unwrap();
    let doc = query_stream.next(&mut tx, None).await?.unwrap();
    // Ingress accounts for the document including system fields
    let expected_ingress = doc.size() as u64;

    // Verify usage stats are tracked for inserting a new soft-deleted row
    let usage_stats = usage.gather_user_stats();
    let ingress = usage_stats
        .database_ingress_v2
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .expect("Expected database_ingress_v2 to be tracked");
    assert_eq!(
        ingress, expected_ingress,
        "Expected exact ingress for inserting new soft-deleted row"
    );

    // Verify v1 ingress is not used for streaming imports
    let v1_ingress = usage_stats
        .database_ingress
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .unwrap_or(0);
    assert_eq!(
        v1_ingress, 0,
        "Expected database_ingress (v1) to be 0 for streaming import, only v2 should be used"
    );

    let mut tx = application.begin(Identity::system()).await?;
    assert_eq!(
        2,
        TableModel::new(&mut tx)
            .must_count(TableNamespace::Global, &table)
            .await?
    );

    Ok(())
}

#[convex_macro::test_runtime]
#[ignore = "This test is correct but takes too long to run in the regular build"]
async fn test_batch_of_operations_taking_more_than_one_transaction(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let table: TableName = "items".parse()?;
    IndexModel::new(&mut tx)
        .add_system_index(
            TableNamespace::test_user(),
            IndexMetadata::new_enabled(
                IndexName::new_reserved(
                    table.clone(),
                    FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR.clone(),
                )?,
                vec!["id".parse()?].try_into()?,
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    let rows: Vec<BatchWriteRow> = (0..10000)
        .map(|i| {
            let row = assert_obj!(
                "id" => ConvexValue::Int64(i),
                "fivetran" => assert_obj!(
                    "synced" => ConvexValue::Float64(1715177931182.0),
                ),
            );
            BatchWriteRow {
                table: table.clone().to_string(),
                operation: BatchWriteOperation::Upsert,
                row,
            }
        })
        .collect();

    let usage = usage_tracking::FunctionUsageTracker::new();
    application
        .apply_fivetran_operations(&Identity::system(), rows, usage.clone())
        .await?;

    // Get all actual documents to calculate expected ingress (includes system
    // fields and index overhead)
    let mut tx = application.begin(Identity::system()).await?;
    let mut query_stream = ResolvedQuery::new(
        &mut tx,
        TableNamespace::Global,
        Query::full_table_scan(table.clone(), Order::Asc),
    )?;
    let mut expected_ingress: u64 = 0;
    while let Some(doc) = query_stream.next(&mut tx, None).await? {
        // Ingress accounts for the document including system fields
        expected_ingress += doc.size() as u64;
    }

    // Verify usage stats are tracked for large batch of inserts
    let usage_stats = usage.gather_user_stats();
    let ingress = usage_stats
        .database_ingress_v2
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .expect("Expected database_ingress_v2 to be tracked");
    assert_eq!(
        ingress, expected_ingress,
        "Expected exact ingress for inserting 10000 rows"
    );
    // Verify v1 ingress is not used for streaming imports
    let v1_ingress = usage_stats
        .database_ingress
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .unwrap_or(0);
    assert_eq!(
        v1_ingress, 0,
        "Expected database_ingress (v1) to be 0 for streaming import, only v2 should be used"
    );

    let mut tx = application.begin(Identity::system()).await?;
    assert_eq!(
        10000,
        TableModel::new(&mut tx)
            .must_count(TableNamespace::Global, &table)
            .await?
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_truncate_nonexistent_table(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let table: TableName = "nonexistent".parse()?;
    let usage = usage_tracking::FunctionUsageTracker::new();
    application
        .fivetran_truncate(
            &Identity::system(),
            table.clone(),
            None,
            DeleteType::SoftDelete,
            usage,
        )
        .await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_soft_truncate_all(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let table: TableName = "table".parse()?;
    IndexModel::new(&mut tx)
        .add_system_index(
            TableNamespace::test_user(),
            IndexMetadata::new_enabled(
                IndexName::new_reserved(table.clone(), FIVETRAN_SYNCED_INDEX_DESCRIPTOR.clone())?,
                vec![
                    "fivetran.deleted".parse()?,
                    "fivetran.synced".parse()?,
                    "_creationTime".parse()?,
                ]
                .try_into()?,
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    let mut tx = application.begin(Identity::system()).await?;
    let original_doc = assert_obj!(
        "name" => ConvexValue::String("Document".try_into()?),
        "fivetran" => assert_obj!(
            "deleted" => ConvexValue::Boolean(false),
            "synced" => ConvexValue::Float64(0.0),
        ),
    );
    let document_id = UserFacingModel::new_root_for_test(&mut tx)
        .insert(table.clone(), original_doc.clone())
        .await?;
    application.commit_test(tx).await?;

    let usage = usage_tracking::FunctionUsageTracker::new();
    application
        .fivetran_truncate(
            &Identity::system(),
            table.clone(),
            None,
            DeleteType::SoftDelete,
            usage.clone(),
        )
        .await?;

    // Get the actual document to calculate expected ingress (includes system fields
    // and index overhead)
    let mut tx = application.begin(Identity::system()).await?;
    let mut query_stream = ResolvedQuery::new(
        &mut tx,
        TableNamespace::Global,
        Query::full_table_scan(table.clone(), Order::Asc),
    )?;
    let doc = query_stream.next(&mut tx, None).await?.unwrap();
    // Ingress accounts for the document including system fields
    let expected_ingress = doc.size() as u64;

    // Verify usage stats are tracked for soft truncate operation (ingress only for
    // imports)
    let usage_stats = usage.gather_user_stats();
    let ingress = usage_stats
        .database_ingress_v2
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .expect("Expected database_ingress_v2 to be tracked");
    assert_eq!(
        ingress, expected_ingress,
        "Expected exact ingress for soft truncate (updating deleted flags)"
    );
    // Verify v1 ingress is not used for streaming imports
    let v1_ingress = usage_stats
        .database_ingress
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .unwrap_or(0);
    assert_eq!(
        v1_ingress, 0,
        "Expected database_ingress (v1) to be 0 for streaming import, only v2 should be used"
    );

    let mut tx = application.begin(Identity::system()).await?;
    assert_eq!(
        UserFacingModel::new_root_for_test(&mut tx)
            .get(document_id, None)
            .await?
            .unwrap()
            .into_value()
            .0
            .filter_system_fields(),
        assert_obj!(
            "name" => ConvexValue::String("Document".try_into()?),
            "fivetran" => assert_obj!(
                "deleted" => ConvexValue::Boolean(true),
                "synced" => ConvexValue::Float64(0.0),
            ),
        )
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_hard_truncate_since_timestamp(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let table: TableName = "table".parse()?;
    IndexModel::new(&mut tx)
        .add_system_index(
            TableNamespace::test_user(),
            IndexMetadata::new_enabled(
                IndexName::new_reserved(table.clone(), FIVETRAN_SYNCED_INDEX_DESCRIPTOR.clone())?,
                vec![
                    "fivetran.deleted".parse()?,
                    "fivetran.synced".parse()?,
                    "_creationTime".parse()?,
                ]
                .try_into()?,
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    let mut tx = application.begin(Identity::system()).await?;
    for i in 0..10 {
        UserFacingModel::new_root_for_test(&mut tx)
            .insert(
                table.clone(),
                assert_obj!(
                    "id" => ConvexValue::Int64(i),
                    "fivetran" => assert_obj!(
                        "deleted" => ConvexValue::Boolean(false),
                        "synced" => ConvexValue::Float64((i as f64) * 1000.0),
                    ),
                ),
            )
            .await?;
    }
    application.commit_test(tx).await?;

    // Query the documents that will be deleted to calculate expected egress
    // (those with timestamps 0, 1000, 2000 - first 3 documents)
    let mut tx = application.begin(Identity::system()).await?;
    let mut query_stream = ResolvedQuery::new(
        &mut tx,
        TableNamespace::test_user(),
        Query::full_table_scan(table.clone(), Order::Asc),
    )?;
    let mut single_doc_size = 0u64;
    let mut count = 0;
    while let Some(doc) = query_stream.next(&mut tx, None).await? {
        // Will delete rows with timestamps 0, 1000, 2000 (first 3 documents with ids 0,
        // 1, 2)
        if count < 3 {
            single_doc_size += doc.size() as u64;
        }
        count += 1;
    }
    // Hard truncate reads each document twice: once via index query, once for the
    // actual document
    let expected_egress = 2 * single_doc_size;
    drop(query_stream);
    application.commit_test(tx).await?;

    let usage = usage_tracking::FunctionUsageTracker::new();
    application
        .fivetran_truncate(
            &Identity::system(),
            table.clone(),
            // will delete rows with timestamps 0, 1000, 2000
            Some(DateTime::from_timestamp(3, 0).unwrap()),
            DeleteType::HardDelete,
            usage.clone(),
        )
        .await?;

    // Verify usage: hard truncate reads documents (egress_v2) but doesn't write (no
    // ingress)
    let usage_stats = usage.gather_user_stats();

    // No ingress - hard truncate doesn't write anything
    assert!(
        usage_stats.database_ingress_v2.is_empty(),
        "Expected no database_ingress_v2 usage from hard truncate (no writes)"
    );

    // Should have exact egress_v2 from reading the documents to delete
    let egress_v2 = usage_stats
        .database_egress_v2
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .unwrap_or(0);
    assert_eq!(
        egress_v2, expected_egress,
        "Expected database_egress_v2 to equal size of 3 deleted documents"
    );

    // Verify v1 ingress is 0 (only v2 should be tracked for streaming imports)
    let v1_ingress = usage_stats
        .database_ingress
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .unwrap_or(0);
    assert_eq!(
        v1_ingress, 0,
        "Expected database_ingress (v1) to be 0 for streaming import"
    );

    let mut tx = application.begin(Identity::system()).await?;
    assert_eq!(
        7,
        TableModel::new(&mut tx)
            .must_count(TableNamespace::Global, &table)
            .await?
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_soft_truncate_since_timestamp(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let table: TableName = "table".parse()?;
    IndexModel::new(&mut tx)
        .add_system_index(
            TableNamespace::test_user(),
            IndexMetadata::new_enabled(
                IndexName::new_reserved(table.clone(), FIVETRAN_SYNCED_INDEX_DESCRIPTOR.clone())?,
                vec![
                    "fivetran.deleted".parse()?,
                    "fivetran.synced".parse()?,
                    "_creationTime".parse()?,
                ]
                .try_into()?,
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    let mut tx = application.begin(Identity::system()).await?;
    let old_document_id = UserFacingModel::new_root_for_test(&mut tx)
        .insert(
            table.clone(),
            assert_obj!(
                "name" => ConvexValue::String("Old document".try_into()?),
                "fivetran" => assert_obj!(
                    "deleted" => ConvexValue::Boolean(false),
                    "synced" => ConvexValue::Float64(0.0),
                ),
            ),
        )
        .await?;
    let new_document_id = UserFacingModel::new_root_for_test(&mut tx)
        .insert(
            table.clone(),
            assert_obj!(
                "name" => ConvexValue::String("New document".try_into()?),
                "fivetran" => assert_obj!(
                    "deleted" => ConvexValue::Boolean(false),
                    "synced" => ConvexValue::Float64(10_000.0),
                ),
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    let usage = usage_tracking::FunctionUsageTracker::new();
    application
        .fivetran_truncate(
            &Identity::system(),
            table.clone(),
            Some(DateTime::from_timestamp(2, 0).unwrap()),
            DeleteType::SoftDelete,
            usage.clone(),
        )
        .await?;

    // Get the actual old document to calculate expected ingress (includes system
    // fields and index overhead) Only the old document was updated, so we
    // calculate ingress based on it
    let mut tx = application.begin(Identity::system()).await?;
    let doc = UserFacingModel::new_root_for_test(&mut tx)
        .get(old_document_id, None)
        .await?
        .unwrap();
    // Ingress accounts for the document including system fields
    let expected_ingress = doc.size() as u64;

    // Verify usage stats are tracked for soft truncate with timestamp filter
    // (ingress only for imports)
    let usage_stats = usage.gather_user_stats();
    let ingress = usage_stats
        .database_ingress_v2
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .expect("Expected database_ingress_v2 to be tracked");
    assert_eq!(
        ingress, expected_ingress,
        "Expected exact ingress for soft truncate with timestamp (updating deleted flag)"
    );
    // Verify v1 ingress is not used for streaming imports
    let v1_ingress = usage_stats
        .database_ingress
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .unwrap_or(0);
    assert_eq!(
        v1_ingress, 0,
        "Expected database_ingress (v1) to be 0 for streaming import, only v2 should be used"
    );

    let mut tx = application.begin(Identity::system()).await?;
    assert_eq!(
        UserFacingModel::new_root_for_test(&mut tx)
            .get(old_document_id, None)
            .await?
            .unwrap()
            .into_value()
            .0
            .filter_system_fields(),
        assert_obj!(
            "name" => ConvexValue::String("Old document".try_into()?),
            "fivetran" => assert_obj!(
                "deleted" => ConvexValue::Boolean(true),
                "synced" => ConvexValue::Float64(0.0),
            ),
        )
    );
    assert_eq!(
        UserFacingModel::new_root_for_test(&mut tx)
            .get(new_document_id, None)
            .await?
            .unwrap()
            .into_value()
            .0
            .filter_system_fields(),
        assert_obj!(
            "name" => ConvexValue::String("New document".try_into()?),
            "fivetran" => assert_obj!(
                "deleted" => ConvexValue::Boolean(false),
                "synced" => ConvexValue::Float64(10_000.0),
            ),
        )
    );

    Ok(())
}

/// If the range of rows affected by `fivetran_truncate` is large enough, the
/// changes it causes might not fit in a single transaction, and the
/// implementation will split the changes in several transactions that will be
/// applied sequentially.
///
/// This test verifies that the behavior is implemented correctly by forcing the
/// request to use multiple transaction by deleting 10,000 rows, which is larger
/// than the number of documents that can be edited in a single transaction (https://docs.convex.dev/production/state/limits#functions).
#[convex_macro::test_runtime]
#[ignore = "This test is correct but takes too long to run in the regular build"]
async fn test_soft_truncate_larger_than_one_transaction(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let table: TableName = "table".parse()?;
    IndexModel::new(&mut tx)
        .add_system_index(
            TableNamespace::test_user(),
            IndexMetadata::new_enabled(
                IndexName::new_reserved(table.clone(), FIVETRAN_SYNCED_INDEX_DESCRIPTOR.clone())?,
                vec![
                    "fivetran.deleted".parse()?,
                    "fivetran.synced".parse()?,
                    "_creationTime".parse()?,
                ]
                .try_into()?,
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    let doc_to_insert = assert_obj!(
        "name" => ConvexValue::String("My document".try_into()?),
        "fivetran" => assert_obj!(
            "deleted" => ConvexValue::Boolean(false),
            "synced" => ConvexValue::Float64(0.0),
        ),
    );

    for chunk in &(0..10000).chunks(4000) {
        let mut tx = application.begin(Identity::system()).await?;
        for _ in chunk {
            UserFacingModel::new_root_for_test(&mut tx)
                .insert(table.clone(), doc_to_insert.clone())
                .await?;
        }
        application.commit_test(tx).await?;
    }

    let usage = usage_tracking::FunctionUsageTracker::new();
    application
        .fivetran_truncate(
            &Identity::system(),
            table.clone(),
            None,
            DeleteType::SoftDelete,
            usage.clone(),
        )
        .await?;

    // Get all actual documents to calculate expected ingress (includes system
    // fields and index overhead)
    let mut tx = application.begin(Identity::system()).await?;
    let mut query_stream = ResolvedQuery::new(
        &mut tx,
        TableNamespace::Global,
        Query::full_table_scan(table.clone(), Order::Asc),
    )?;
    let mut expected_ingress: u64 = 0;
    while let Some(doc) = query_stream.next(&mut tx, None).await? {
        // Ingress accounts for the document including system fields
        expected_ingress += doc.size() as u64;
    }

    // Verify usage stats are tracked for large soft truncate operation (ingress
    // only for imports)
    let usage_stats = usage.gather_user_stats();
    let ingress = usage_stats
        .database_ingress_v2
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .expect("Expected database_ingress_v2 to be tracked");
    assert_eq!(
        ingress, expected_ingress,
        "Expected exact ingress for soft truncate of 10000 rows"
    );
    // Verify v1 ingress is not used for streaming imports
    let v1_ingress = usage_stats
        .database_ingress
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .unwrap_or(0);
    assert_eq!(
        v1_ingress, 0,
        "Expected database_ingress (v1) to be 0 for streaming import, only v2 should be used"
    );

    let mut tx = application.begin(Identity::system()).await?;
    assert_eq!(
        10000,
        TableModel::new(&mut tx)
            .must_count(TableNamespace::Global, &table)
            .await?
    );

    Ok(())
}

/// Similar to `test_soft_truncate_larger_than_one_transaction` but for hard
/// deletes.
#[convex_macro::test_runtime]
#[ignore = "This test is correct but takes too long to run in the regular build"]
async fn test_hard_truncate_larger_than_one_transaction(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let table: TableName = "table".parse()?;
    IndexModel::new(&mut tx)
        .add_system_index(
            TableNamespace::test_user(),
            IndexMetadata::new_enabled(
                IndexName::new_reserved(table.clone(), FIVETRAN_SYNCED_INDEX_DESCRIPTOR.clone())?,
                vec![
                    "fivetran.deleted".parse()?,
                    "fivetran.synced".parse()?,
                    "_creationTime".parse()?,
                ]
                .try_into()?,
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    for chunk in &(0..10000).chunks(4000) {
        let mut tx = application.begin(Identity::system()).await?;
        for _ in chunk {
            UserFacingModel::new_root_for_test(&mut tx)
                .insert(
                    table.clone(),
                    assert_obj!(
                        "name" => ConvexValue::String("My document".try_into()?),
                        "fivetran" => assert_obj!(
                            "deleted" => ConvexValue::Boolean(false),
                            "synced" => ConvexValue::Float64(0.0),
                        ),
                    ),
                )
                .await?;
        }
        application.commit_test(tx).await?;
    }

    // Query all documents to calculate expected egress
    let mut tx = application.begin(Identity::system()).await?;
    let mut query_stream = ResolvedQuery::new(
        &mut tx,
        TableNamespace::test_user(),
        Query::full_table_scan(table.clone(), Order::Asc),
    )?;
    let mut single_doc_total_size = 0u64;
    while let Some(doc) = query_stream.next(&mut tx, None).await? {
        single_doc_total_size += doc.size() as u64;
    }
    // Hard truncate reads each document twice: once via index query, once for the
    // actual document
    let expected_egress = 2 * single_doc_total_size;
    drop(query_stream);
    application.commit_test(tx).await?;

    let usage = usage_tracking::FunctionUsageTracker::new();
    application
        .fivetran_truncate(
            &Identity::system(),
            table.clone(),
            None,
            DeleteType::HardDelete,
            usage.clone(),
        )
        .await?;

    // Verify usage: hard truncate reads documents (egress_v2) but doesn't write (no
    // ingress)
    let usage_stats = usage.gather_user_stats();

    // No ingress - hard truncate doesn't write anything
    assert!(
        usage_stats.database_ingress_v2.is_empty(),
        "Expected no database_ingress_v2 usage from hard truncate (no writes)"
    );

    // Should have exact egress_v2 from reading all documents to delete
    let egress_v2 = usage_stats
        .database_egress_v2
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .unwrap_or(0);
    assert_eq!(
        egress_v2, expected_egress,
        "Expected database_egress_v2 to equal size of all 10000 documents"
    );

    // Verify v1 ingress is 0 (only v2 should be tracked for streaming imports)
    let v1_ingress = usage_stats
        .database_ingress
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .unwrap_or(0);
    assert_eq!(
        v1_ingress, 0,
        "Expected database_ingress (v1) to be 0 for streaming import"
    );

    let mut tx = application.begin(Identity::system()).await?;
    assert!(
        TableModel::new(&mut tx)
            .table_is_empty(TableNamespace::Global, &table)
            .await?
    );

    Ok(())
}
