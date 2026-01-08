use std::collections::BTreeSet;

use common::{
    components::ComponentPath,
    knobs::TRANSACTION_MAX_NUM_USER_WRITES,
    query::{
        Order,
        Query,
    },
};
use database::{
    ResolvedQuery,
    TableModel,
};
use keybroker::Identity;
use maplit::btreemap;
use model::snapshot_imports::types::ImportRequestor;
use runtime::testing::TestRuntime;
use value::{
    assert_obj,
    TableName,
    TableNamespace,
};

use crate::{
    airbyte_import::{
        AirbyteRecord,
        PrimaryKey,
        ValidatedAirbyteStream,
    },
    test_helpers::ApplicationTestExt,
    Application,
};
#[convex_macro::test_runtime]
async fn test_clear_tables_and_import(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let table: TableName = "table".parse()?;

    // Insert a bunch of empty messages over multiple transactions via streaming
    // import
    let mut records = vec![];
    let rows_to_insert = *TRANSACTION_MAX_NUM_USER_WRITES + 5;
    for _ in 0..rows_to_insert {
        records.push(AirbyteRecord::new(table.clone(), false, assert_obj!()));
    }
    let streams = btreemap! { table.clone() =>
        ValidatedAirbyteStream::Append
    };
    let usage = usage_tracking::FunctionUsageTracker::new();
    let num_records_written = application
        .import_airbyte_records(&Identity::system(), records, streams, usage.clone())
        .await?;
    // Verify the correct number of records were written
    assert_eq!(num_records_written, rows_to_insert as u64);

    // Query all documents to calculate expected ingress (includes system fields)
    let mut tx = application.begin(Identity::system()).await?;
    let mut query_stream = ResolvedQuery::new(
        &mut tx,
        TableNamespace::test_user(),
        Query::full_table_scan(table.clone(), Order::Asc),
    )?;
    let mut expected_ingress = 0u64;
    while let Some(doc) = query_stream.next(&mut tx, None).await? {
        expected_ingress += doc.size() as u64;
    }

    // Verify usage stats are tracked for database writes
    let usage_stats = usage.gather_user_stats();
    let ingress = usage_stats
        .database_ingress_v2
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .expect("Expected database_ingress_v2 to be tracked");
    assert_eq!(
        ingress, expected_ingress,
        "Expected exact ingress for {rows_to_insert} inserted records",
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

    // clear_tables should delete documents
    let clear_usage = usage_tracking::FunctionUsageTracker::new();
    let deleted_docs = application
        .clear_tables(
            &Identity::system(),
            vec![(ComponentPath::root(), table.clone())],
            ImportRequestor::StreamingImport,
            clear_usage.clone(),
        )
        .await?;
    assert_eq!(deleted_docs, rows_to_insert as u64);

    // Note: clear_tables uses table-level operations (creating empty replacement
    // tables and swapping) rather than reading/writing individual documents, so it
    // doesn't track usage in the same way as document-level operations. The usage
    // tracker is passed through for consistency with the API, but no usage is
    // currently tracked for this operation.
    let clear_usage_stats = clear_usage.gather_user_stats();
    assert!(
        clear_usage_stats.database_ingress_v2.is_empty(),
        "Expected no database_ingress_v2 usage from clear_tables"
    );
    assert!(
        clear_usage_stats.database_ingress.is_empty(),
        "Expected no database_ingress usage from clear_tables"
    );
    assert!(
        clear_usage_stats.database_egress_v2.is_empty(),
        "Expected no database_egress_v2 usage from clear_tables"
    );
    assert!(
        clear_usage_stats.database_egress.is_empty(),
        "Expected no database_egress usage from clear_tables"
    );
    assert!(
        clear_usage_stats.database_egress_rows.is_empty(),
        "Expected no database_egress_rows usage from clear_tables"
    );

    let mut tx = application.begin(Identity::system()).await?;
    assert!(
        TableModel::new(&mut tx)
            .table_is_empty(TableNamespace::test_user(), &table)
            .await?
    );
    assert!(TableModel::new(&mut tx).table_exists(TableNamespace::test_user(), &table));

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_delete_tables_and_import(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let table: TableName = "table".parse()?;

    // Insert a bunch of empty messages over multiple transactions via streaming
    // import
    let mut records = vec![];
    let rows_to_insert = *TRANSACTION_MAX_NUM_USER_WRITES + 5;
    for _ in 0..rows_to_insert {
        records.push(AirbyteRecord::new(table.clone(), false, assert_obj!()));
    }
    let streams = btreemap! { table.clone() =>
        ValidatedAirbyteStream::Append
    };
    let usage = usage_tracking::FunctionUsageTracker::new();
    let num_records_written = application
        .import_airbyte_records(&Identity::system(), records, streams, usage.clone())
        .await?;
    assert_eq!(num_records_written, rows_to_insert as u64);

    // Query all documents to calculate expected ingress (includes system fields)
    let mut tx = application.begin(Identity::system()).await?;
    let mut query_stream = ResolvedQuery::new(
        &mut tx,
        TableNamespace::test_user(),
        Query::full_table_scan(table.clone(), Order::Asc),
    )?;
    let mut expected_ingress = 0u64;
    while let Some(doc) = query_stream.next(&mut tx, None).await? {
        expected_ingress += doc.size() as u64;
    }

    // Verify usage stats are tracked for database writes
    let usage_stats = usage.gather_user_stats();
    let ingress = usage_stats
        .database_ingress_v2
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .expect("Expected database_ingress_v2 to be tracked");
    assert_eq!(
        ingress, expected_ingress,
        "Expected exact ingress for {rows_to_insert} inserted records",
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

    // delete_tables should delete documents
    let table_namespace = TableNamespace::test_user();
    let deleted_docs = application
        .delete_tables(&Identity::system(), vec![table.clone()], table_namespace)
        .await?;
    assert_eq!(deleted_docs, rows_to_insert as u64);
    // Check the table is empty after clearing it
    let mut tx = application.begin(Identity::system()).await?;
    assert!(
        TableModel::new(&mut tx)
            .table_is_empty(table_namespace, &table)
            .await?
    );
    assert!(!TableModel::new(&mut tx).table_exists(table_namespace, &table));

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_dedup_import(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let table: TableName = "table".parse()?;
    let primary_key = PrimaryKey::try_from(vec![vec!["primary_key".to_string()]])?;
    let indexes = btreemap! {table.clone() => primary_key.clone()};
    application
        .add_primary_key_indexes(&Identity::system(), indexes)
        .await?;
    // Make sure the indexes are enabled. Backfill would happen asynchronously, but
    // because the table is empty, it should be done here.
    application
        .wait_for_primary_key_indexes_ready(Identity::system(), BTreeSet::from([table.clone()]))
        .await?;
    let objects = vec![
        // Updated value for key1
        (
            assert_obj!("field1" => "value1", "primary_key" => "key1"),
            false,
        ),
        (
            assert_obj!("field1" => "value2", "primary_key" => "key1"),
            false,
        ),
        // No changes to key2
        (
            assert_obj!("field1" => "value1", "primary_key" => "key2"),
            false,
        ),
        // Deleted value for key3
        (
            assert_obj!("field1" => "value1", "primary_key" => "key3"),
            false,
        ),
        (assert_obj!("primary_key" => "key3"), true),
    ];
    let records = objects
        .iter()
        .cloned()
        .map(|(obj, deleted)| AirbyteRecord::new(table.clone(), deleted, obj))
        .collect();
    let streams = btreemap! { table.clone() =>
        ValidatedAirbyteStream::Dedup(primary_key)
    };
    let usage = usage_tracking::FunctionUsageTracker::new();
    let num_records_written = application
        .import_airbyte_records(&Identity::system(), records, streams, usage.clone())
        .await?;
    // Verify the correct number of records were processed (5 total records)
    assert_eq!(num_records_written, objects.len() as u64);

    // Query all documents to calculate expected ingress (includes system fields)
    // After dedup, we have 2 records in the table: key1 (latest value) and key2
    // key3 is deleted (last operation), so it won't be in the table
    let mut tx = application.begin(Identity::system()).await?;
    let mut query_stream = ResolvedQuery::new(
        &mut tx,
        TableNamespace::test_user(),
        Query::full_table_scan(table.clone(), Order::Asc),
    )?;
    let mut expected_ingress = 0u64;
    while let Some(doc) = query_stream.next(&mut tx, None).await? {
        expected_ingress += doc.size() as u64;
    }

    // Verify usage stats are tracked for database operations (dedup reads and
    // writes)
    let usage_stats = usage.gather_user_stats();
    let ingress = usage_stats
        .database_ingress_v2
        .get(&(ComponentPath::root(), table.to_string()))
        .copied()
        .expect("Expected database_ingress_v2 to be tracked");
    assert_eq!(
        ingress, expected_ingress,
        "Expected ingress for dedup import writes"
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
    let mut query_stream = ResolvedQuery::new(
        &mut tx,
        TableNamespace::test_user(),
        Query::full_table_scan(table, Order::Asc),
    )?;
    let mut objects_in_table = vec![];
    while let Some(doc) = query_stream.next(&mut tx, Some(3)).await? {
        objects_in_table.push(doc.into_value());
    }
    let objects = &objects[1..objects.len() - 2];
    assert_eq!(objects.len(), objects_in_table.len());
    for (i, (obj, _)) in objects.iter().enumerate() {
        let obj_in_table = &*objects_in_table[i];
        for field in ["field1", "primary_key"] {
            assert_eq!(obj.get(field), obj_in_table.get(field))
        }
    }
    Ok(())
}
