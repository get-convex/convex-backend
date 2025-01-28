use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    io::Cursor,
    str,
    sync::Arc,
};

use anyhow::Context;
use async_zip_reader::ZipReader;
use bytes::Bytes;
use common::{
    components::{
        ComponentId,
        ComponentPath,
    },
    document::ParsedDocument,
    types::{
        ConvexOrigin,
        TableName,
    },
    value::ConvexObject,
};
use database::{
    test_helpers::DbFixtures,
    BootstrapComponentsModel,
    Database,
    TableModel,
    UserFacingModel,
};
use file_storage::{
    FileStorage,
    TransactionalFileStorage,
};
use headers::ContentType;
use keybroker::Identity;
use maplit::btreeset;
use model::{
    exports::types::{
        ExportFormat,
        ExportRequestor,
    },
    file_storage::types::FileStorageEntry,
    test_helpers::DbFixturesWithModel,
};
use pretty_assertions::assert_eq;
use runtime::testing::TestRuntime;
use serde_json::json;
use storage::{
    LocalDirStorage,
    Storage,
    StorageExt,
};
use tokio::io::AsyncReadExt;
use usage_tracking::FunctionUsageTracker;
use value::{
    assert_obj,
    export::ValueFormat,
    DeveloperDocumentId,
    ResolvedDocumentId,
    TableNamespace,
};

use super::ExportWorker;
use crate::{
    exports::{
        export_inner,
        get_export_path_prefix,
        zip_uploader::README_MD_CONTENTS,
    },
    test_helpers::ApplicationTestExt,
    tests::components::unmount_component,
    Application,
};

#[convex_macro::test_runtime]
async fn test_export_zip(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
    let storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
    let file_storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
    let mut export_worker = ExportWorker::new_test(rt, db.clone(), storage.clone(), file_storage);

    let mut expected_export_entries = BTreeMap::new();

    expected_export_entries.insert("README.md".to_string(), README_MD_CONTENTS.to_string());

    expected_export_entries.insert(
        "_tables/documents.jsonl".to_string(),
        format!(
            "{}\n{}\n{}\n",
            json!({"name": "table_0", "id": 10001}),
            json!({"name": "table_1", "id": 10002}),
            json!({"name": "table_2", "id": 10003}),
        ),
    );
    expected_export_entries.insert("_storage/documents.jsonl".to_string(), format!(""));

    // Write to a bunch of tables
    for i in 0..3 {
        let table: TableName = str::parse(format!("table_{i}").as_str())?;
        let mut tx = db.begin(Identity::system()).await?;
        let id = match i {
            0 => {
                UserFacingModel::new_root_for_test(&mut tx)
                    .insert(table, assert_obj!("foo" => 1))
                    .await?
            },
            1 => {
                UserFacingModel::new_root_for_test(&mut tx)
                    .insert(table, assert_obj!("foo" => [1, "1"]))
                    .await?
            },
            _ => {
                UserFacingModel::new_root_for_test(&mut tx)
                    .insert(table, assert_obj!("foo" => "1"))
                    .await?
            },
        };
        let doc = UserFacingModel::new_root_for_test(&mut tx)
            .get(id, None)
            .await?
            .unwrap();
        let tablet_id = tx
            .table_mapping()
            .namespace(TableNamespace::test_user())
            .number_to_tablet()(doc.table())?;
        let doc = doc.to_resolved(tablet_id);
        let id_v6 = doc.developer_id().encode();
        expected_export_entries.insert(
            format!("table_{i}/documents.jsonl"),
            format!(
                "{}\n",
                serde_json::to_string(&doc.export(ValueFormat::ConvexCleanJSON))?
            ),
        );
        expected_export_entries.insert(
            format!("table_{i}/generated_schema.jsonl"),
            match i {
                0 => format!(
                    "{}\n",
                    json!(format!(
                        "{{\"_creationTime\": normalfloat64, \"_id\": \"{id_v6}\", \"foo\": \
                         int64}}"
                    ))
                ),
                1 => format!(
                    "{}\n{}\n",
                    json!(format!(
                        "{{\"_creationTime\": normalfloat64, \"_id\": \"{id_v6}\", \"foo\": \
                         array<int64 | field_name>}}"
                    )),
                    json!({id_v6: {"foo": ["int64", "infer"]}})
                ),
                _ => format!(
                    "{}\n",
                    json!(format!(
                        "{{\"_creationTime\": normalfloat64, \"_id\": \"{id_v6}\", \"foo\": \
                         field_name}}"
                    ))
                ),
            },
        );
        db.commit(tx).await?;
    }
    let (_, zip_object_key, usage) = export_inner(
        &mut export_worker,
        ExportFormat::Zip {
            include_storage: true,
        },
        ExportRequestor::SnapshotExport,
        |_| async { Ok(()) },
    )
    .await?;

    // Check we can get the stored zip.
    let storage_stream = storage
        .get(&zip_object_key)
        .await?
        .context("object missing from storage")?;
    let stored_bytes = storage_stream.collect_as_bytes().await?;
    let mut zip_reader = ZipReader::new(Cursor::new(stored_bytes)).await?;
    let mut zip_entries = BTreeMap::new();
    let filenames: Vec<_> = zip_reader.file_names().await?;
    for (i, filename) in filenames.into_iter().enumerate() {
        let entry_reader = zip_reader.by_index(i).await?;
        let mut entry_contents = String::new();
        entry_reader
            .read()
            .read_to_string(&mut entry_contents)
            .await?;
        zip_entries.insert(filename, entry_contents);
    }
    assert_eq!(zip_entries, expected_export_entries);

    let usage = usage.gather_user_stats();
    let component_path = ComponentPath::test_user();
    assert!(usage.database_egress_size[&(component_path.clone(), "table_0".to_string())] > 0);
    assert!(usage.database_egress_size[&(component_path.clone(), "table_1".to_string())] > 0);
    assert!(usage.database_egress_size[&(component_path, "table_2".to_string())] > 0);

    Ok(())
}

async fn write_test_data_in_component(
    db: &Database<TestRuntime>,
    component: ComponentId,
    path_prefix: &str,
    expected_export_entries: &mut BTreeMap<String, String>,
) -> anyhow::Result<()> {
    expected_export_entries.insert(
        format!("{path_prefix}_tables/documents.jsonl"),
        format!("{}\n", json!({"name": "messages", "id": 10001}),),
    );
    // Write to tables in each component
    let table: TableName = str::parse("messages")?;
    let mut tx = db.begin(Identity::system()).await?;
    let id = UserFacingModel::new(&mut tx, component.into())
        .insert(table, assert_obj!("channel" => "c", "text" => path_prefix))
        .await?;
    let doc = UserFacingModel::new(&mut tx, component.into())
        .get(id, None)
        .await?
        .unwrap();
    let tablet_id = tx
        .table_mapping()
        .namespace(component.into())
        .number_to_tablet()(doc.table())?;
    let doc = doc.to_resolved(tablet_id);
    let expected_documents = format!(
        "{}\n",
        serde_json::to_string(&doc.export(ValueFormat::ConvexCleanJSON))?
    );
    let expected_generated_schema = format!(
        "{}\n",
        json!(format!(
            r#"{{"_creationTime": normalfloat64, "_id": "{id}", "channel": "c", "text": field_name}}"#,
        ))
    );
    expected_export_entries.insert(
        format!("{path_prefix}messages/documents.jsonl"),
        expected_documents.clone(),
    );
    expected_export_entries.insert(
        format!("{path_prefix}messages/generated_schema.jsonl"),
        expected_generated_schema.clone(),
    );
    db.commit(tx).await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_export_components(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("with-schema")
        .await?;
    let db = application.database().clone();
    let storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
    let file_storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
    let mut export_worker = ExportWorker::new_test(rt, db.clone(), storage.clone(), file_storage);

    let mut expected_export_entries = BTreeMap::new();

    expected_export_entries.insert("README.md".to_string(), README_MD_CONTENTS.to_string());

    let mut tx = db.begin(Identity::system()).await?;
    let component_path = "component".parse()?;
    let (_, child_component) =
        BootstrapComponentsModel::new(&mut tx).must_component_path_to_ids(&component_path)?;

    for (path_prefix, component) in [
        ("", ComponentId::Root),
        ("_components/component/", child_component),
    ] {
        write_test_data_in_component(&db, component, path_prefix, &mut expected_export_entries)
            .await?;
    }

    let (_, zip_object_key, usage) = export_inner(
        &mut export_worker,
        ExportFormat::Zip {
            include_storage: false,
        },
        ExportRequestor::SnapshotExport,
        |_| async { Ok(()) },
    )
    .await?;

    // Check we can get the stored zip.
    let storage_stream = storage
        .get(&zip_object_key)
        .await?
        .context("object missing from storage")?;
    let stored_bytes = storage_stream.collect_as_bytes().await?;
    let mut zip_reader = ZipReader::new(Cursor::new(stored_bytes)).await?;
    let mut zip_entries = BTreeMap::new();
    let filenames: Vec<_> = zip_reader.file_names().await?;
    for (i, filename) in filenames.into_iter().enumerate() {
        let entry_reader = zip_reader.by_index(i).await?;
        let mut entry_contents = String::new();
        entry_reader
            .read()
            .read_to_string(&mut entry_contents)
            .await?;
        zip_entries.insert(filename, entry_contents);
    }
    assert_eq!(zip_entries, expected_export_entries);

    let usage = usage.gather_user_stats();
    assert!(usage.database_egress_size[&(component_path, "messages".to_string())] > 0);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_export_unmounted_components(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    unmount_component(&application).await?;

    let db = application.database().clone();
    let storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
    let file_storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
    let mut export_worker = ExportWorker::new_test(rt, db.clone(), storage.clone(), file_storage);

    let expected_export_entries = btreeset! {
        "README.md".to_string(),
        "_components/component/_tables/documents.jsonl".to_string(),
        "_components/component/messages/documents.jsonl".to_string(),
        "_components/component/messages/generated_schema.jsonl".to_string(),
        "_components/envVars/_components/component/_tables/documents.jsonl".to_string(),
        "_components/envVars/_components/component/messages/documents.jsonl".to_string(),
        "_components/envVars/_components/component/messages/generated_schema.jsonl".to_string(),
        "_components/envVars/_tables/documents.jsonl".to_string(),
        "_tables/documents.jsonl".to_string(),
    };

    let (_, zip_object_key, usage) = export_inner(
        &mut export_worker,
        ExportFormat::Zip {
            include_storage: false,
        },
        ExportRequestor::SnapshotExport,
        |_| async { Ok(()) },
    )
    .await?;

    // Check we can get the stored zip.
    let storage_stream = storage
        .get(&zip_object_key)
        .await?
        .context("object missing from storage")?;
    let stored_bytes = storage_stream.collect_as_bytes().await?;
    let mut zip_reader = ZipReader::new(Cursor::new(stored_bytes)).await?;
    let mut zip_entries = BTreeSet::new();
    let filenames: Vec<_> = zip_reader.file_names().await?;
    for (i, filename) in filenames.into_iter().enumerate() {
        let entry_reader = zip_reader.by_index(i).await?;
        let mut entry_contents = String::new();
        entry_reader
            .read()
            .read_to_string(&mut entry_contents)
            .await?;
        zip_entries.insert(filename);
    }
    assert_eq!(zip_entries, expected_export_entries);

    let usage = usage.gather_user_stats();
    assert!(usage.database_egress_size[&("component".parse()?, "messages".to_string())] > 0);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_export_storage(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
    let storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
    let file_storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
    let mut export_worker = ExportWorker::new_test(
        rt.clone(),
        db.clone(),
        storage.clone(),
        file_storage.clone(),
    );
    let file_storage_wrapper = FileStorage {
        database: db.clone(),
        transactional_file_storage: TransactionalFileStorage::new(
            rt,
            file_storage,
            ConvexOrigin::from("origin".to_string()),
        ),
    };
    let mut expected_export_entries = BTreeMap::new();

    expected_export_entries.insert("README.md".to_string(), README_MD_CONTENTS.to_string());
    expected_export_entries.insert("_tables/documents.jsonl".to_string(), format!(""));

    // Write a few storage files.
    let usage_tracker = FunctionUsageTracker::new();
    let file1_id = file_storage_wrapper
        .store_file(
            TableNamespace::test_user(),
            None,
            Some(ContentType::jpeg()),
            futures::stream::iter(vec![Ok(Bytes::from_static(b"abc"))]),
            None,
            &usage_tracker,
        )
        .await?;
    let mut tx = db.begin(Identity::system()).await?;
    let storage_table_id = tx
        .table_mapping()
        .namespace(TableNamespace::test_user())
        .id(&"_file_storage".parse()?)?;
    let file1: ParsedDocument<FileStorageEntry> = tx
        .get(ResolvedDocumentId::new(
            storage_table_id.tablet_id,
            DeveloperDocumentId::new(storage_table_id.table_number, file1_id.internal_id()),
        ))
        .await?
        .unwrap()
        .try_into()?;

    expected_export_entries.insert(format!("_storage/{file1_id}.jpeg"), format!("abc"));
    expected_export_entries.insert(
            "_storage/documents.jsonl".to_string(),
            format!(
                "{}\n",
                json!({"_id": file1_id.encode(), "_creationTime": f64::from(file1.creation_time().unwrap()), "sha256": "ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=", "size": 3, "contentType": "image/jpeg", "internalId": file1.storage_id.to_string()}),
            ),
        );

    let (_, zip_object_key, usage) = export_inner(
        &mut export_worker,
        ExportFormat::Zip {
            include_storage: true,
        },
        ExportRequestor::SnapshotExport,
        |_| async { Ok(()) },
    )
    .await?;

    // Check we can get the stored zip.
    let storage_stream = storage
        .get(&zip_object_key)
        .await?
        .context("object missing from storage")?;
    let stored_bytes = storage_stream.collect_as_bytes().await?;
    let mut zip_reader = ZipReader::new(Cursor::new(stored_bytes)).await?;
    let mut zip_entries = BTreeMap::new();
    let filenames: Vec<_> = zip_reader.file_names().await?;
    for (i, filename) in filenames.into_iter().enumerate() {
        let entry_reader = zip_reader.by_index(i).await?;
        let mut entry_contents = String::new();
        entry_reader
            .read()
            .read_to_string(&mut entry_contents)
            .await?;
        zip_entries.insert(filename, entry_contents);
    }
    assert_eq!(zip_entries, expected_export_entries);

    let usage = usage.gather_user_stats();
    assert!(usage.database_egress_size.is_empty());

    Ok(())
}

// Regression test: previously we were trying to export documents from deleted
// tables and table_mapping was failing.
#[convex_macro::test_runtime]
async fn test_export_with_table_delete(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new(&rt).await?;
    let storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
    let file_storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
    let mut export_worker =
        ExportWorker::new_test(rt.clone(), db.clone(), storage.clone(), file_storage);

    // Write to two tables and delete one.
    let mut tx = db.begin(Identity::system()).await?;
    UserFacingModel::new_root_for_test(&mut tx)
        .insert("table_0".parse()?, ConvexObject::empty())
        .await?;
    db.commit(tx).await?;
    let mut tx = db.begin(Identity::system()).await?;
    UserFacingModel::new_root_for_test(&mut tx)
        .insert("table_1".parse()?, ConvexObject::empty())
        .await?;
    db.commit(tx).await?;
    let mut tx = db.begin(Identity::system()).await?;
    TableModel::new(&mut tx)
        .delete_table(TableNamespace::test_user(), "table_0".parse()?)
        .await?;
    db.commit(tx).await?;

    let (_, _zip_object_key, _) = export_inner(
        &mut export_worker,
        ExportFormat::Zip {
            include_storage: false,
        },
        ExportRequestor::SnapshotExport,
        |_| async { Ok(()) },
    )
    .await?;
    Ok(())
}

#[test]
fn test_get_export_path_prefix() -> anyhow::Result<()> {
    assert_eq!(get_export_path_prefix(&ComponentPath::root()), "");
    assert_eq!(get_export_path_prefix(&"a".parse()?), "_components/a/");
    assert_eq!(
        get_export_path_prefix(&"a/b".parse()?),
        "_components/a/_components/b/"
    );
    assert_eq!(
        get_export_path_prefix(&"a/b/c".parse()?),
        "_components/a/_components/b/_components/c/"
    );
    Ok(())
}
