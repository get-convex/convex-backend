use std::{
    collections::BTreeMap,
    str,
    sync::Arc,
};

use anyhow::Context;
use bytes::Bytes;
use common::{
    components::ComponentPath,
    document::{
        ParseDocument,
        ParsedDocument,
    },
    types::{
        ConvexOrigin,
        TableName,
    },
    value::ConvexObject,
};
use database::{
    test_helpers::DbFixtures,
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
use model::{
    components::config::ComponentConfigModel,
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
};
use storage_zip_reader::StorageZipArchive;
use tokio::io::AsyncReadExt;
use usage_tracking::FunctionUsageTracker;
use value::{
    assert_obj,
    export::ValueFormat,
    DeveloperDocumentId,
    ResolvedDocumentId,
    TableNamespace,
};

use super::ExportComponents;
use crate::{
    export_inner,
    get_export_path_prefix,
    zip_uploader::README_MD_CONTENTS,
};

struct ExportFixtures {
    export_components: ExportComponents<TestRuntime>,
    db: Database<TestRuntime>,
}

async fn setup_export_test(rt: &TestRuntime) -> anyhow::Result<ExportFixtures> {
    let DbFixtures { db, .. } = DbFixtures::new_with_model(rt).await?;
    let exports_storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
    let file_storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
    Ok(ExportFixtures {
        export_components: ExportComponents {
            runtime: rt.clone(),
            database: db.latest_database_snapshot()?,
            exports_storage,
            file_storage,
            instance_name: "carnitas".to_string(),
        },
        db,
    })
}

#[convex_macro::test_runtime]
async fn test_export_zip(rt: TestRuntime) -> anyhow::Result<()> {
    let ExportFixtures {
        mut export_components,
        db,
    } = setup_export_test(&rt).await?;

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
    export_components.database = db.latest_database_snapshot()?;
    let (zip_object_key, usage) = export_inner(
        &export_components,
        ExportFormat::Zip {
            include_storage: true,
        },
        ExportRequestor::SnapshotExport,
        |_| async { Ok(()) },
    )
    .await?;

    // Check we can get the stored zip.
    let zip_reader =
        StorageZipArchive::open(export_components.exports_storage.clone(), &zip_object_key).await?;
    let mut zip_entries = BTreeMap::new();
    for entry in zip_reader.entries() {
        let mut entry_contents = String::new();
        zip_reader
            .read_entry(entry.clone())
            .read_to_string(&mut entry_contents)
            .await?;
        zip_entries.insert(entry.name.clone(), entry_contents);
    }
    assert_eq!(zip_entries, expected_export_entries);

    let usage = usage.gather_user_stats();
    let component_path = ComponentPath::test_user();
    assert!(usage.database_egress_size[&(component_path.clone(), "table_0".to_string())] > 0);
    assert!(usage.database_egress_size[&(component_path.clone(), "table_1".to_string())] > 0);
    assert!(usage.database_egress_size[&(component_path, "table_2".to_string())] > 0);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_export_storage(rt: TestRuntime) -> anyhow::Result<()> {
    let ExportFixtures {
        mut export_components,
        db,
    } = setup_export_test(&rt).await?;
    let file_storage_wrapper = FileStorage {
        database: db.clone(),
        transactional_file_storage: TransactionalFileStorage::new(
            rt,
            export_components.file_storage.clone(),
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
        .parse()?;

    expected_export_entries.insert(format!("_storage/{file1_id}.jpeg"), format!("abc"));
    expected_export_entries.insert(
        "_storage/documents.jsonl".to_string(),
        format!(
            "{}\n",
            json!({
                "_id": file1_id.encode(),
                "_creationTime": f64::from(file1.creation_time()),
                "sha256": "ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=",
                "size": 3,
                "contentType": "image/jpeg",
                "internalId": file1.storage_id.to_string(),
            }),
        ),
    );

    export_components.database = db.latest_database_snapshot()?;
    let (zip_object_key, usage) = export_inner(
        &export_components,
        ExportFormat::Zip {
            include_storage: true,
        },
        ExportRequestor::SnapshotExport,
        |_| async { Ok(()) },
    )
    .await?;

    // Check we can get the stored zip.
    let zip_reader =
        StorageZipArchive::open(export_components.exports_storage.clone(), &zip_object_key).await?;
    let mut zip_entries = BTreeMap::new();
    for entry in zip_reader.entries() {
        let mut entry_contents = String::new();
        zip_reader
            .read_entry(entry.clone())
            .read_to_string(&mut entry_contents)
            .await?;
        zip_entries.insert(entry.name.clone(), entry_contents);
    }
    assert_eq!(zip_entries, expected_export_entries);

    let usage = usage.gather_user_stats();
    assert!(usage.database_egress_size.is_empty());

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_export_many_storage_files(rt: TestRuntime) -> anyhow::Result<()> {
    let ExportFixtures {
        mut export_components,
        db,
    } = setup_export_test(&rt).await?;
    let file_storage_wrapper = FileStorage {
        database: db.clone(),
        transactional_file_storage: TransactionalFileStorage::new(
            rt,
            export_components.file_storage.clone(),
            ConvexOrigin::from("origin".to_string()),
        ),
    };

    // Write a lot of storage files.
    let usage_tracker = FunctionUsageTracker::new();
    let mut ids = vec![];
    for i in 0..256 {
        let id = file_storage_wrapper
            .store_file(
                TableNamespace::test_user(),
                None,
                None,
                futures::stream::iter([Ok(format!("file{i}").into_bytes())]),
                None,
                &usage_tracker,
            )
            .await?;
        ids.push(id);
    }

    export_components.database = db.latest_database_snapshot()?;
    let (zip_object_key, _) = export_inner(
        &export_components,
        ExportFormat::Zip {
            include_storage: true,
        },
        ExportRequestor::SnapshotExport,
        |_| async { Ok(()) },
    )
    .await?;

    // Check that all the files made it into the zip.
    let zip_reader =
        StorageZipArchive::open(export_components.exports_storage.clone(), &zip_object_key).await?;
    for (i, id) in ids.into_iter().enumerate() {
        let entry = zip_reader
            .by_name(format!("_storage/{id}"))
            .context("storage file missing")?;
        let mut entry_reader = zip_reader.read_entry(entry.clone());
        let mut entry_contents = String::new();
        entry_reader.read_to_string(&mut entry_contents).await?;
        assert_eq!(entry_contents, format!("file{i}"));
    }

    Ok(())
}

// Regression test: previously we were trying to export documents from deleted
// tables and table_mapping was failing.
#[convex_macro::test_runtime]
async fn test_export_with_table_delete(rt: TestRuntime) -> anyhow::Result<()> {
    let ExportFixtures {
        mut export_components,
        db,
    } = setup_export_test(&rt).await?;

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
        .delete_active_table(TableNamespace::test_user(), "table_0".parse()?)
        .await?;
    db.commit(tx).await?;

    export_components.database = db.latest_database_snapshot()?;
    let (_zip_object_key, _) = export_inner(
        &export_components,
        ExportFormat::Zip {
            include_storage: false,
        },
        ExportRequestor::SnapshotExport,
        |_| async { Ok(()) },
    )
    .await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_export_with_namespace_without_component(rt: TestRuntime) -> anyhow::Result<()> {
    let ExportFixtures {
        mut export_components,
        db,
    } = setup_export_test(&rt).await?;

    // Make a namespace without a component.
    let mut tx = db.begin(Identity::system()).await?;
    let id = ComponentConfigModel::new(&mut tx)
        .initialize_component_namespace(false)
        .await?;
    TableModel::new(&mut tx)
        .insert_table_metadata(TableNamespace::ByComponent(id), &"table_0".parse()?)
        .await?;
    db.commit(tx).await?;

    // Export the namespace.
    export_components.database = db.latest_database_snapshot()?;
    let (..) = export_inner(
        &export_components,
        ExportFormat::Zip {
            include_storage: false,
        },
        ExportRequestor::CloudBackup,
        |s| async move {
            tracing::info!("{s}");
            Ok(())
        },
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
