use chrono::DateTime;
use common::{
    bootstrap_model::index::IndexMetadata,
    types::IndexName,
};
use convex_fivetran_destination::{
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
use database::{
    IndexModel,
    TableModel,
    UserFacingModel,
};
use errors::ErrorMetadataAnyhowExt;
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

    application
        .apply_fivetran_operations(
            &Identity::system(),
            vec![BatchWriteRow {
                table: table.to_string(),
                operation: BatchWriteOperation::Upsert,
                row: assert_obj!(
                    "id" => ConvexValue::Int64(42),
                    "fivetran" => assert_obj!(
                        "synced" => ConvexValue::Float64(1715172902504.0),
                    ),
                ),
            }],
        )
        .await?;

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

    application
        .apply_fivetran_operations(
            &Identity::system(),
            vec![BatchWriteRow {
                table: table.clone().to_string(),
                operation: BatchWriteOperation::Update,
                row: assert_obj!(
                    "name" => "How Convex Rocks",
                    // A new value for otherField isn’t specified → the old value is kept
                    "objectField" => assert_obj!(
                        "field_a" => true,
                        "field_z" => true,
                    ),
                    "fivetran" => assert_obj!(
                        "id" => ConvexValue::Int64(42),
                        "synced" => ConvexValue::Float64(1715176847664.0),
                    ),
                ),
            }],
        )
        .await?;

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

    application
        .apply_fivetran_operations(
            &Identity::system(),
            vec![BatchWriteRow {
                table: table.clone().to_string(),
                operation: BatchWriteOperation::Update,
                row: assert_obj!(
                    "name" => "How Convex Rocks",
                    // A new value for otherField isn’t specified → the old value is kept
                    "objectField" => assert_obj!(
                        "field_a" => true,
                        "field_z" => true,
                    ),
                    "fivetran" => assert_obj!(
                        "id" => ConvexValue::Int64(42),
                        "deleted" => ConvexValue::Boolean(true),
                    ),
                ),
            }],
        )
        .await?;

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

    application
        .apply_fivetran_operations(
            &Identity::system(),
            vec![BatchWriteRow {
                table: table.clone().to_string(),
                operation: BatchWriteOperation::Upsert,
                row: assert_obj!(
                    "id" => ConvexValue::Int64(42),
                    "name" => "How Convex Rocks",
                    // A new value for otherField isn’t specified → the old value is removed
                    "fivetran" => assert_obj!(
                        "synced" => ConvexValue::Float64(1715176847664.0),
                    ),
                ),
            }],
        )
        .await?;

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
        )
        .await?;

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

    application
        .apply_fivetran_operations(
            &Identity::system(),
            vec![BatchWriteRow {
                table: table.clone().to_string(),
                operation: BatchWriteOperation::Upsert,
                row: assert_obj!(
                    "id" => ConvexValue::Int64(42),
                    "name" => "How Convex Rocks",
                    "fivetran" => assert_obj!(
                        "deleted" => ConvexValue::Boolean(true),
                        "synced" => ConvexValue::Float64(1715176847664.0),
                    ),
                ),
            }],
        )
        .await?;

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

    application
        .apply_fivetran_operations(
            &Identity::system(),
            (0..10000)
                .map(|i| BatchWriteRow {
                    table: table.clone().to_string(),
                    operation: BatchWriteOperation::Upsert,
                    row: assert_obj!(
                        "id" => ConvexValue::Int64(i),
                        "fivetran" => assert_obj!(
                            "synced" => ConvexValue::Float64(1715177931182.0),
                        ),
                    ),
                })
                .collect(),
        )
        .await?;

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
    application
        .fivetran_truncate(
            &Identity::system(),
            table.clone(),
            None,
            DeleteType::SoftDelete,
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
    let document_id = UserFacingModel::new_root_for_test(&mut tx)
        .insert(
            table.clone(),
            assert_obj!(
                "name" => ConvexValue::String("Document".try_into()?),
                "fivetran" => assert_obj!(
                    "deleted" => ConvexValue::Boolean(false),
                    "synced" => ConvexValue::Float64(0.0),
                ),
            ),
        )
        .await?;
    application.commit_test(tx).await?;

    application
        .fivetran_truncate(
            &Identity::system(),
            table.clone(),
            None,
            DeleteType::SoftDelete,
        )
        .await?;

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

    application
        .fivetran_truncate(
            &Identity::system(),
            table.clone(),
            // will delete rows with timestamps 0, 1000, 2000
            Some(DateTime::from_timestamp(3, 0).unwrap()),
            DeleteType::HardDelete,
        )
        .await?;

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

    application
        .fivetran_truncate(
            &Identity::system(),
            table,
            Some(DateTime::from_timestamp(2, 0).unwrap()),
            DeleteType::SoftDelete,
        )
        .await?;

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

    application
        .fivetran_truncate(
            &Identity::system(),
            table.clone(),
            None,
            DeleteType::SoftDelete,
        )
        .await?;

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

    application
        .fivetran_truncate(
            &Identity::system(),
            table.clone(),
            None,
            DeleteType::HardDelete,
        )
        .await?;

    let mut tx = application.begin(Identity::system()).await?;
    assert!(
        TableModel::new(&mut tx)
            .table_is_empty(TableNamespace::Global, &table)
            .await?
    );

    Ok(())
}
