use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    io::Cursor,
    sync::Arc,
};

use anyhow::Context as _;
use async_zip_reader::ZipReader;
use common::components::ComponentId;
use database::{
    BootstrapComponentsModel,
    Database,
    UserFacingModel,
};
use exports::ExportComponents;
use keybroker::Identity;
use maplit::btreeset;
use model::exports::types::{
    ExportFormat,
    ExportRequestor,
};
use runtime::testing::TestRuntime;
use serde_json::json;
use storage::{
    LocalDirStorage,
    Storage,
    StorageExt as _,
};
use tokio::io::AsyncReadExt as _;
use value::{
    assert_obj,
    export::ValueFormat,
    TableName,
};

use crate::{
    test_helpers::ApplicationTestExt as _,
    tests::components::unmount_component,
    Application,
};

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

    let mut expected_export_entries = BTreeMap::new();

    expected_export_entries.insert(
        "README.md".to_string(),
        exports::README_MD_CONTENTS.to_string(),
    );

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

    let (zip_object_key, usage) = exports::export_inner(
        &ExportComponents {
            runtime: rt.clone(),
            database: db.latest_database_snapshot()?,
            storage: storage.clone(),
            file_storage,
            instance_name: "carnitas".to_string(),
        },
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

    let (zip_object_key, usage) = exports::export_inner(
        &ExportComponents {
            runtime: rt.clone(),
            database: db.latest_database_snapshot()?,
            storage: storage.clone(),
            file_storage,
            instance_name: "carnitas".to_string(),
        },
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
