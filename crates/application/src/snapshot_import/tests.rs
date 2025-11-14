use std::{
    collections::BTreeMap,
    path::Path,
    str::FromStr,
    sync::Arc,
};

use anyhow::Context;
use async_zip::{
    base::write::ZipFileWriter,
    Compression,
    ZipEntryBuilder,
};
use bytes::Bytes;
use common::{
    bootstrap_model::{
        components::ComponentState,
        index::{
            IndexConfig,
            IndexMetadata,
        },
    },
    components::{
        ComponentId,
        ComponentPath,
    },
    db_schema,
    document::ResolvedDocument,
    ext::PeekableExt,
    object_validator,
    pause::PauseController,
    query::Order,
    runtime::Runtime,
    schemas::{
        validator::{
            FieldValidator,
            Validator,
        },
        DatabaseSchema,
        DocumentSchema,
    },
    testing::assert_contains,
    tokio::select,
    types::{
        IndexDescriptor,
        IndexName,
        MemberId,
    },
    value::ConvexValue,
};
use database::{
    BootstrapComponentsModel,
    IndexModel,
    ResolvedQuery,
    SchemaModel,
    TableModel,
    UserFacingModel,
};
use errors::ErrorMetadataAnyhowExt;
use futures::{
    pin_mut,
    stream::{
        self,
        BoxStream,
    },
    FutureExt,
    StreamExt,
    TryStreamExt,
};
use keybroker::{
    AdminIdentity,
    Identity,
};
use maplit::btreemap;
use model::snapshot_imports::types::{
    ImportRequestor,
    ImportState,
};
use must_let::must_let;
use runtime::testing::TestRuntime;
use serde_json::{
    json,
    Value as JsonValue,
};
use storage::{
    LocalDirStorage,
    Storage,
    StorageUseCase,
    Upload,
};
use usage_tracking::FunctionUsageTracker;
use value::{
    assert_obj,
    assert_val,
    id_v6::DeveloperDocumentId,
    val,
    ConvexObject,
    FieldName,
    InternalId,
    TableName,
    TableNamespace,
    TableNumber,
};

use crate::{
    snapshot_import::{
        do_import,
        do_import_from_object_key,
        import_objects,
        parse::{
            parse_import_file,
            ParsedImport,
        },
        start_stored_import,
        wait_for_import_worker,
        ImportFormat,
        ImportMode,
    },
    test_helpers::ApplicationTestExt,
    Application,
};

#[convex_macro::test_runtime]
async fn test_peeking_take_while(_rt: TestRuntime) {
    let s = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8]);
    let mut p = Box::pin(s.peekable());
    // First check that raw take_while causes us to skip an item.
    let prefix = p.as_mut().take_while(|x| {
        let is_prefix = *x <= 2;
        async move { is_prefix }
    });
    pin_mut!(prefix);
    assert_eq!(prefix.collect::<Vec<_>>().await, vec![1, 2]);
    assert_eq!(p.next().await, Some(4));
    // Next check that peeking_take_while doesn't skip an item.
    {
        let prefix = p.as_mut().peeking_take_while(|x| *x <= 6);
        pin_mut!(prefix);
        assert_eq!(prefix.collect::<Vec<_>>().await, vec![5, 6]);
    }
    assert_eq!(p.next().await, Some(7));
}

async fn run_parse_objects<RT: Runtime>(
    rt: RT,
    format: ImportFormat,
    v: impl AsRef<[u8]>,
) -> anyhow::Result<Vec<JsonValue>> {
    let storage_dir = tempfile::TempDir::new()?;
    let storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::for_use_case(
        rt.clone(),
        &storage_dir.path().to_string_lossy(),
        StorageUseCase::SnapshotImports,
    )?);
    let mut upload = storage.start_upload().await?;
    upload.write(Bytes::copy_from_slice(v.as_ref())).await?;
    let object_key = upload.complete().await?;
    let import = parse_import_file(
        format,
        ComponentPath::root(),
        storage.clone(),
        storage.fully_qualified_key(&object_key),
    )
    .await?;

    stream::iter(import.documents.into_iter().map(|(_, _, stream)| stream))
        .flatten()
        .try_collect()
        .await
}

fn stream_from_str(str: &str) -> BoxStream<'static, anyhow::Result<Bytes>> {
    stream::iter(vec![anyhow::Ok(str.to_string().into_bytes().into())]).boxed()
}

#[convex_macro::test_runtime]
async fn test_csv(rt: TestRuntime) -> anyhow::Result<()> {
    let test1 = r#"
a,b,c
1,a string i guess,1.2
5.10,-100,"a string in quotes"
"#;
    let objects = run_parse_objects(rt, ImportFormat::Csv("table".parse().unwrap()), test1).await?;
    let expected = vec![
        json!({
            "a": 1.,
            "b": "a string i guess",
            "c": 1.2,
        }),
        json!({
            "a": 5.10,
            "b": -100.,
            "c": "a string in quotes",
        }),
    ];
    assert_eq!(objects, expected);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_duplicate_id(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name = "table1";
    let test_csv = r#"
_id,value
"jd7f2yq3tcc5h4ce9qhqdk0ach6hbmyb","hi"
"jd7f2yq3tcc5h4ce9qhqdk0ach6hbmyb","there"
"#;
    let err = run_csv_import(&app, table_name, test_csv)
        .await
        .unwrap_err();
    assert!(err.is_bad_request());
    assert!(
        err.to_string()
            .contains("Objects in table \"table1\" have duplicate _id fields"),
        "{err}"
    );
    Ok(())
}

// See https://github.com/BurntSushi/rust-csv/issues/114. TL;DR CSV can't distinguish between empty string and none.
#[convex_macro::test_runtime]
async fn test_csv_empty_strings(rt: TestRuntime) -> anyhow::Result<()> {
    let test1 = r#"
a,b,c,d
"",,"""",""""""
"#;
    let objects = run_parse_objects(rt, ImportFormat::Csv("table".parse().unwrap()), test1).await?;
    let expected = vec![json!({
        "a": "",
        "b": "",
        "c": "\"",
        "d": "\"\"",
    })];
    assert_eq!(objects, expected);
    Ok(())
}

#[convex_macro::test_runtime]
#[ignore]
async fn import_huge_csv(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name = "table1";
    let mut test_csv = vec!["value".to_string()];
    let mut expected = vec![];
    // Too big to write or read in a single transaction.
    for value in 0..10000 {
        test_csv.push(value.to_string());
        expected.push(btreemap!("value" => ConvexValue::from(value as f64)));
    }
    run_csv_import(&app, table_name, &test_csv.join("\n")).await?;

    let objects = load_fields_as_maps(&app, table_name, vec!["value"]).await?;

    assert_eq!(objects, expected);
    Ok(())
}

#[convex_macro::test_runtime]
async fn import_with_empty_strings_and_no_schema_defaults_to_empty_strings(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name = "table1";
    let test_csv = r#"
a,b,c,d
"",,"""",""""""
"#;
    run_csv_import(&app, table_name, test_csv).await?;

    let objects = load_fields_as_maps(&app, table_name, vec!["a", "b", "c", "d"]).await?;

    let expected = vec![btreemap!(
        "a" => assert_val!(""),
        "b" => assert_val!(""),
        "c" => assert_val!("\""),
        "d" => assert_val!("\"\""),
    )];
    assert_eq!(objects, expected);
    Ok(())
}

#[convex_macro::test_runtime]
async fn import_with_empty_strings_and_string_schema_treats_empty_as_empty(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name = "table1";
    let test_csv = r#"
a,b,c,d
"",,"""",""""""
"#;

    let fields = vec!["a", "b", "c", "d"];
    let schema = db_schema!(
        table_name => DocumentSchema::Union(
            vec![
                object_validator!(
                    "a" => FieldValidator::required_field_type(Validator::String),
                    "b" => FieldValidator::required_field_type(Validator::String),
                    "c" => FieldValidator::required_field_type(Validator::String),
                    "d" => FieldValidator::required_field_type(Validator::String),
                )
            ]
        )
    );

    activate_schema(&app, schema).await?;

    run_csv_import(&app, table_name, test_csv).await?;

    let objects = load_fields_as_maps(&app, table_name, fields).await?;

    assert_eq!(
        objects,
        vec![btreemap!(
            "a" => assert_val!(""),
            "b" => assert_val!(""),
            "c" => assert_val!("\""),
            "d" => assert_val!("\"\""),
        )]
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn import_with_empty_strings_and_optional_string_schema_treats_empty_as_none(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name = "table1";
    let test_csv = r#"
a,b,c,d
"",,"""",""""""
"#;

    let schema = db_schema!(
        table_name => DocumentSchema::Union(
            vec![
                object_validator!(
                    "a" => FieldValidator::optional_field_type(Validator::String),
                    "b" => FieldValidator::optional_field_type(Validator::String),
                    "c" => FieldValidator::optional_field_type(Validator::String),
                    "d" => FieldValidator::optional_field_type(Validator::String),
                )
            ]
        )
    );

    activate_schema(&app, schema).await?;
    run_csv_import(&app, table_name, test_csv).await?;

    let objects = load_fields_as_maps(&app, table_name, vec!["a", "b", "c", "d"]).await?;

    assert_eq!(
        objects,
        vec![btreemap!(
            "c" => assert_val!("\""),
            "d" => assert_val!("\"\""),
        )]
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn import_with_empty_strings_and_optional_number_schema_treats_empty_as_none(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name = "table1";
    let test_csv = r#"
a,b
"",
"#;

    let schema = db_schema!(
        table_name => DocumentSchema::Union(
            vec![
                object_validator!(
                    "a" => FieldValidator::optional_field_type(Validator::Float64),
                    "b" => FieldValidator::optional_field_type(Validator::Int64),
                )
            ]
        )
    );

    activate_schema(&app, schema).await?;
    run_csv_import(&app, table_name, test_csv).await?;

    let objects = load_fields_as_maps(&app, table_name, vec!["a", "b"]).await?;

    assert_eq!(objects, vec![BTreeMap::default()]);

    Ok(())
}

#[convex_macro::test_runtime]
async fn import_validates_against_schema(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name = "table1";
    let test_csv = r#"
a
"string"
"#;

    let schema = db_schema!(
        table_name => DocumentSchema::Union(
            vec![
                object_validator!(
                    "a" => FieldValidator::optional_field_type(Validator::Float64),
                )
            ]
        )
    );

    activate_schema(&app, schema).await?;
    let err = run_csv_import(&app, table_name, test_csv)
        .await
        .unwrap_err();
    assert!(err.is_bad_request());

    Ok(())
}

#[convex_macro::test_runtime]
async fn import_replace_confirmation_message(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name = "table1";
    let test_csv = r#"
a
"string"
"#;
    // Create some data so there's something to replace.
    run_csv_import(&app, table_name, test_csv).await?;

    let object_key = app
        .upload_snapshot_import(stream_from_str(test_csv))
        .await?;
    let import_id = start_stored_import(
        &app,
        new_admin_id(),
        ImportFormat::Csv(table_name.parse()?),
        ImportMode::Replace,
        ComponentPath::root(),
        object_key,
        ImportRequestor::SnapshotImport,
    )
    .await?;

    let snapshot_import = wait_for_import_worker(&app, new_admin_id(), import_id).await?;

    let state = snapshot_import.state.clone();
    must_let!(let ImportState::WaitingForConfirmation {
            info_message,
            require_manual_confirmation,
        } = state);

    assert_eq!(
        info_message,
        r#"Import change summary:
table  | create | delete |
--------------------------
table1 | 1      | 1 of 1 |
Once the import has started, it will run in the background.
Interrupting `npx convex import` will not cancel it."#
    );
    assert!(require_manual_confirmation);

    Ok(())
}

// Hard to control timing in race test with background job moving state forward.
#[convex_macro::test_runtime]
async fn import_races_with_schema_update(
    rt: TestRuntime,
    pause_controller: PauseController,
) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name = "table1";
    let test_csv = r#"
a
"string"
"#;

    let initial_schema = db_schema!(
        table_name => DocumentSchema::Union(
            vec![
                object_validator!(
                    "a" => FieldValidator::optional_field_type(Validator::String),
                )
            ]
        )
    );

    activate_schema(&app, initial_schema).await?;

    let hold_guard = pause_controller.hold("before_finalize_import");

    let mut import_fut = run_csv_import(&app, table_name, test_csv).boxed();

    select! {
        r = import_fut.as_mut().fuse() => {
            anyhow::bail!("import finished before pausing: {r:?}");
        },
        pause_guard = hold_guard.wait_for_blocked().fuse() => {
            let pause_guard = pause_guard.unwrap();
            let mismatch_schema = db_schema!(
                table_name => DocumentSchema::Union(
                    vec![
                        object_validator!(
                            "a" => FieldValidator::optional_field_type(Validator::Float64),
                        )
                    ]
                )
            );
            // This succeeds (even in prod) because the table is Hidden.
            activate_schema(&app, mismatch_schema).await?;
            pause_guard.unpause();
        },
    }
    let err = import_fut.await.unwrap_err();
    assert!(err.is_bad_request());
    assert!(
        err.msg()
            .contains("Could not complete import because schema changed"),
        "{err:?}"
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn import_would_break_foreign_key(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name = "table1";
    let table_with_foreign_key = "table_with_foreign_key";
    let identity = new_admin_id();

    {
        let mut tx = app.begin(identity).await?;
        let validated_id = UserFacingModel::new_root_for_test(&mut tx)
            .insert(table_name.parse()?, assert_obj!())
            .await?;
        UserFacingModel::new_root_for_test(&mut tx)
            .insert(
                table_with_foreign_key.parse()?,
                assert_obj!(
                    "a" => validated_id.encode()
                ),
            )
            .await?;
        app.commit_test(tx).await?;
    }

    // table1 initially has number 10001
    // table_with_foreign_key has number 10002
    // Import table1 with number 10003
    let test_csv = r#"
_id,a
"jd7f2yq3tcc5h4ce9qhqdk0ach6hbmyb","string"
"#;

    let initial_schema = db_schema!(
        table_with_foreign_key => DocumentSchema::Union(
            vec![
                object_validator!(
                    "a" => FieldValidator::optional_field_type(Validator::Id(table_name.parse()?)),
                )
            ]
        )
    );

    activate_schema(&app, initial_schema).await?;

    let err = run_csv_import(&app, table_name, test_csv)
        .await
        .unwrap_err();
    assert!(err.is_bad_request());
    assert_eq!(
        err.msg(),
        "Hit an error while importing:\nImport changes table 'table1' which is referenced by \
         'table_with_foreign_key' in the schema"
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn import_preserves_foreign_key(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name = "table1";
    let identity = new_admin_id();

    let doc_id;
    {
        let mut tx = app.begin(identity).await?;
        doc_id = UserFacingModel::new_root_for_test(&mut tx)
            .insert(table_name.parse()?, assert_obj!())
            .await?;
        app.commit_test(tx).await?;
    }

    let table_with_foreign_key = "table_with_foreign_key";
    // table1 initially has number 10001
    // table_with_foreign_key has number 10002
    // Import table1 with number 10001 (clearing the table)
    let test_csv = r#"
a
"#;

    let initial_schema = db_schema!(
        table_with_foreign_key => DocumentSchema::Union(
            vec![
                object_validator!(
                    "a" => FieldValidator::optional_field_type(Validator::Id(table_name.parse()?)),
                )
            ]
        )
    );

    activate_schema(&app, initial_schema).await?;

    run_csv_import(&app, table_name, test_csv).await?;

    // Now import a document into table_with_foreign_key that references table1
    run_csv_import(
        &app,
        table_with_foreign_key,
        &format!(
            r#"a
{doc_id}
{doc_id}
"#
        ),
    )
    .await?;
    Ok(())
}

/// Add three tables (table1, table2, table3)
///
/// table1: [ doc1 ]
/// table2: [ doc2 ]
/// table3: [ doc3 ]
///
/// Schema only contains table3
///
/// Do an import with an ID from table1, but import into table2
///
/// Expect that in the end, table2/table3 exist, but table3 is truncated
///
/// table2: [ doc1 ]
/// table3: []
#[convex_macro::test_runtime]
async fn import_replace_all(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name1: TableName = "table1".parse()?;
    let table_name2: TableName = "table2".parse()?;
    let table_name3: TableName = "table3".parse()?;
    let identity = new_admin_id();

    // Create tables
    let t1_doc = {
        let mut tx = app.begin(identity.clone()).await?;
        let mut ufm = UserFacingModel::new_root_for_test(&mut tx);
        let t1_doc = ufm.insert(table_name1, assert_obj!()).await?;
        ufm.insert(table_name2.clone(), assert_obj!()).await?;
        ufm.insert(table_name3.clone(), assert_obj!()).await?;
        app.commit_test(tx).await?;
        t1_doc
    };

    // Add table3 to schema
    let initial_schema = db_schema!("table3" => DocumentSchema::Any);
    activate_schema(&app, initial_schema).await?;

    // ID is for a table corresponding to table1, but we're writing it into table2
    let test_csv = format!(
        r#"
_id,a
"{t1_doc}","string"
"#
    );

    assert_eq!(
        TableModel::new(&mut app.begin(identity.clone()).await?).count_user_tables(),
        3
    );

    // Import into table2
    do_import(
        &app,
        new_admin_id(),
        ImportFormat::Csv(table_name2.clone()),
        ImportMode::ReplaceAll,
        ComponentPath::root(),
        stream_from_str(&test_csv),
    )
    .await?;

    let mut tx = app.begin(identity.clone()).await?;
    assert_eq!(TableModel::new(&mut tx).count_user_tables(), 2);
    assert_eq!(
        TableModel::new(&mut tx)
            .must_count(TableNamespace::Global, &table_name2)
            .await?,
        1
    );
    assert_eq!(
        TableModel::new(&mut tx)
            .must_count(TableNamespace::Global, &table_name3)
            .await?,
        0
    );
    assert_eq!(
        UserFacingModel::new_root_for_test(&mut tx)
            .get(t1_doc, None)
            .await?
            .context("Not found")?
            .into_value()
            .into_value()
            .get("a"),
        Some(&val!("string")),
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn import_replace_all_table_number_mismatch(rt: TestRuntime) -> anyhow::Result<()> {
    let test_case = |mode: ImportMode, expect_success: bool| {
        let rt = rt.clone();
        async move {
            let app = Application::new_for_tests(&rt).await?;
            let table_name1: TableName = "table1".parse()?;
            let table_name2: TableName = "table2".parse()?;
            let identity = new_admin_id();

            // Create tables
            let t1_doc = {
                let mut tx = app.begin(identity.clone()).await?;
                let mut ufm = UserFacingModel::new_root_for_test(&mut tx);
                let t1_doc = ufm.insert(table_name1, assert_obj!()).await?;
                ufm.insert(table_name2.clone(), assert_obj!()).await?;
                app.commit_test(tx).await?;
                t1_doc
            };

            // Add table2 to schema, so the importer tries to clear it.
            let initial_schema = db_schema!("table2" => DocumentSchema::Any);
            activate_schema(&app, initial_schema).await?;

            // ID is for a table corresponding to table1, but we're writing it into table2
            let test_csv = format!(
                r#"
_id,a
"{t1_doc}","string"
"#
            );

            assert_eq!(
                TableModel::new(&mut app.begin(identity.clone()).await?).count_user_tables(),
                2
            );

            // Import into table2
            let result = do_import(
                &app,
                new_admin_id(),
                ImportFormat::Csv(table_name2.clone()),
                mode,
                ComponentPath::root(),
                stream_from_str(&test_csv),
            )
            .await;

            if expect_success {
                assert_eq!(result?, 1);
            } else {
                result.unwrap_err();
                return Ok(());
            }

            let mut tx = app.begin(identity.clone()).await?;
            assert_eq!(TableModel::new(&mut tx).count_user_tables(), 1);
            assert_eq!(
                TableModel::new(&mut tx)
                    .must_count(TableNamespace::Global, &table_name2)
                    .await?,
                1
            );
            assert_eq!(
                UserFacingModel::new_root_for_test(&mut tx)
                    .get(t1_doc, None)
                    .await?
                    .context("Not found")?
                    .into_value()
                    .into_value()
                    .get("a"),
                Some(&val!("string")),
            );
            anyhow::Ok(())
        }
    };
    // Append table1's id into table2 results in conflicting IDs in table2
    test_case(ImportMode::Append, false).await?;
    // Replacing table1's id into table2 results in two tables with the same ID.
    test_case(ImportMode::Replace, false).await?;
    // Replacing all deletes table2 and replaces table1, so it's good.
    test_case(ImportMode::ReplaceAll, true).await?;
    // Require empty fails because table2 is not empty.
    test_case(ImportMode::RequireEmpty, false).await?;

    Ok(())
}

#[convex_macro::test_runtime]
async fn import_zip_flip_table_number(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name1: TableName = "table1".parse()?;
    let table_name2: TableName = "table2".parse()?;
    let identity = new_admin_id();

    // Create tables (t1 then t2)
    let mut tx = app.begin(identity.clone()).await?;
    let mut ufm = UserFacingModel::new_root_for_test(&mut tx);
    ufm.insert(table_name1.clone(), assert_obj!()).await?;
    ufm.insert(table_name2.clone(), assert_obj!()).await?;
    app.commit_test(tx).await?;
    let export_object_key = app.export_and_wait().await?;

    for (mode, expect_success) in [
        (ImportMode::Append, false),
        (ImportMode::Replace, true),
        (ImportMode::ReplaceAll, true),
        (ImportMode::RequireEmpty, false),
    ] {
        let app = Application::new_for_tests(&rt).await?;

        // Create tables (t2 then t1)
        let mut tx = app.begin(identity.clone()).await?;
        let mut ufm = UserFacingModel::new_root_for_test(&mut tx);
        ufm.insert(table_name2.clone(), assert_obj!()).await?;
        ufm.insert(table_name1.clone(), assert_obj!()).await?;
        app.commit_test(tx).await?;

        let rows_written = do_import_from_object_key(
            &app,
            identity.clone(),
            ImportFormat::Zip,
            mode,
            ComponentPath::root(),
            export_object_key.clone(),
        )
        .await;
        tracing::info!("Imported in test for {mode}");
        if expect_success {
            assert_eq!(rows_written?, 2);
        } else {
            rows_written.unwrap_err();
        }
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn import_zip_to_clone_of_deployment(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name1: TableName = "table1".parse()?;
    let table_name2: TableName = "table2".parse()?;
    let identity = new_admin_id();

    // Create tables (t1 then t2)
    let mut tx = app.begin(identity.clone()).await?;
    let mut ufm = UserFacingModel::new_root_for_test(&mut tx);
    ufm.insert(table_name1.clone(), assert_obj!()).await?;
    ufm.insert(table_name2.clone(), assert_obj!()).await?;
    app.commit_test(tx).await?;
    let export_object_key = app.export_and_wait().await?;

    for (mode, expect_success) in [
        (ImportMode::Append, true),
        (ImportMode::Replace, true),
        (ImportMode::ReplaceAll, true),
        (ImportMode::RequireEmpty, false),
    ] {
        let app = Application::new_for_tests(&rt).await?;

        // Create tables (t1 then t2) again
        let mut tx = app.begin(identity.clone()).await?;
        let mut ufm = UserFacingModel::new_root_for_test(&mut tx);
        ufm.insert(table_name1.clone(), assert_obj!()).await?;
        ufm.insert(table_name2.clone(), assert_obj!()).await?;
        app.commit_test(tx).await?;

        let rows_written = do_import_from_object_key(
            &app,
            identity.clone(),
            ImportFormat::Zip,
            mode,
            ComponentPath::root(),
            export_object_key.clone(),
        )
        .await;
        tracing::info!("Imported in test for {mode}");
        if expect_success {
            assert_eq!(rows_written?, 2);
        } else {
            rows_written.unwrap_err();
        }
    }

    Ok(())
}

async fn make_zip(files: &[(impl AsRef<str>, impl AsRef<[u8]>)]) -> anyhow::Result<Bytes> {
    let mut zip = ZipFileWriter::new(vec![]);
    for (filename, content) in files {
        zip.write_entry_whole(
            ZipEntryBuilder::new(filename.as_ref().into(), Compression::Stored),
            content.as_ref(),
        )
        .await?;
    }
    Ok(zip.close().await?.into())
}

#[convex_macro::test_runtime]
async fn import_conflicting_table_numbers(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let identity = new_admin_id();

    let id_table10001 = DeveloperDocumentId::new(TableNumber::try_from(10001)?, InternalId::MAX);

    let test_cases = [
        // _tables has two conflicting table numbers
        make_zip(&[(
            "_tables/documents.jsonl",
            r#"{"name":"table1","id":10001}
            {"name":"table2","id":10001}"#,
        )])
        .await?,
        // Same, but in a component
        make_zip(&[(
            "component/_tables/documents.jsonl",
            r#"{"name":"table1","id":10001}
                {"name":"table2","id":10001}"#,
        )])
        .await?,
        // Two tables with conflicting inferred table numbers
        make_zip(&[
            (
                "table1/documents.jsonl",
                format!(r#"{{"_id":"{id_table10001}"}}"#),
            ),
            (
                "table2/documents.jsonl",
                format!(r#"{{"_id":"{id_table10001}"}}"#),
            ),
        ])
        .await?,
        // Inferred table number conflicts with declared table number
        make_zip(&[
            (
                "_tables/documents.jsonl",
                format!(r#"{{"name":"table1","id":10001}}"#),
            ),
            (
                "table2/documents.jsonl",
                format!(r#"{{"_id":"{id_table10001}"}}"#),
            ),
        ])
        .await?,
    ];

    for zip_data in test_cases {
        let object_key = app
            .upload_snapshot_import(stream::iter([zip_data]).map(Ok).boxed())
            .await?;

        for mode in [
            ImportMode::Replace,
            ImportMode::ReplaceAll,
            ImportMode::RequireEmpty,
        ] {
            assert_contains(
                &do_import_from_object_key(
                    &app,
                    identity.clone(),
                    ImportFormat::Zip,
                    mode,
                    ComponentPath::root(),
                    object_key.clone(),
                )
                .await
                .unwrap_err(),
                "conflict between `table1` and `table2`",
            );
        }
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn import_table_number_conflict_with_existing_table(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let identity = new_admin_id();

    let mut tx = app.begin(identity.clone()).await?;
    let id = UserFacingModel::new_root_for_test(&mut tx)
        .insert("table1".parse()?, assert_obj!())
        .await?;
    app.commit_test(tx).await?;

    let test_case = make_zip(&[("table2/documents.jsonl", format!(r#"{{"_id":"{id}"}}"#))]).await?;
    let object_key = app
        .upload_snapshot_import(stream::iter([test_case]).map(Ok).boxed())
        .await?;
    assert_contains(
        &do_import_from_object_key(
            &app,
            identity.clone(),
            ImportFormat::Zip,
            ImportMode::Replace,
            ComponentPath::root(),
            object_key.clone(),
        )
        .await
        .unwrap_err(),
        "New table `table2` has IDs that conflict with existing table `table1`",
    );

    Ok(())
}
#[convex_macro::test_runtime]
async fn import_table_number_conflict_race(
    rt: TestRuntime,
    pause_controller: PauseController,
) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let identity = new_admin_id();

    let id_table10001 = DeveloperDocumentId::new(TableNumber::try_from(10001)?, InternalId::MAX);

    let test_case = make_zip(&[(
        "table1/documents.jsonl",
        format!(r#"{{"_id":"{id_table10001}"}}"#),
    )])
    .await?;
    let object_key = app
        .upload_snapshot_import(stream::iter([test_case]).map(Ok).boxed())
        .await?;

    let hold_guard = pause_controller.hold("before_assign_table_numbers");
    let import = do_import_from_object_key(
        &app,
        identity.clone(),
        ImportFormat::Zip,
        ImportMode::Replace,
        ComponentPath::root(),
        object_key.clone(),
    );
    pin_mut!(import);

    let pause_guard = select! {
        _ = import.as_mut() => panic!("being held"),
        r = hold_guard.wait_for_blocked() => r.unwrap(),
    };

    // Create a table
    let mut tx = app.begin(identity.clone()).await?;
    assert_eq!(
        UserFacingModel::new_root_for_test(&mut tx)
            .insert("table2".parse()?, assert_obj!())
            .await?
            .table(),
        id_table10001.table()
    );
    app.commit_test(tx).await?;

    pause_guard.unpause();

    assert_contains(
        &import.await.unwrap_err(),
        "New table `table1` has IDs that conflict with existing table `table2`",
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn import_zip_to_deployment_with_unrelated_tables(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name1: TableName = "table1".parse()?;
    let table_name2: TableName = "table2".parse()?;
    let identity = new_admin_id();

    // unrelated tables
    let table_name3: TableName = "table3".parse()?;
    let table_name4: TableName = "table4".parse()?;

    // Create tables (t1 then t2)
    let mut tx = app.begin(identity.clone()).await?;
    let mut ufm = UserFacingModel::new_root_for_test(&mut tx);
    ufm.insert(table_name1.clone(), assert_obj!()).await?;
    ufm.insert(table_name2.clone(), assert_obj!()).await?;
    app.commit_test(tx).await?;
    let export_object_key = app.export_and_wait().await?;

    for (mode, expect_success) in [
        (ImportMode::Append, false),
        (ImportMode::Replace, false),
        (ImportMode::ReplaceAll, true),
        (ImportMode::RequireEmpty, false),
    ] {
        let app = Application::new_for_tests(&rt).await?;

        // Create unrelated tables (t3 then t4)
        let mut tx = app.begin(identity.clone()).await?;
        let mut ufm = UserFacingModel::new_root_for_test(&mut tx);
        ufm.insert(table_name3.clone(), assert_obj!()).await?;
        ufm.insert(table_name4.clone(), assert_obj!()).await?;
        app.commit_test(tx).await?;

        let rows_written = do_import_from_object_key(
            &app,
            identity.clone(),
            ImportFormat::Zip,
            mode,
            ComponentPath::root(),
            export_object_key.clone(),
        )
        .await;
        tracing::info!("Imported in test for {mode}");
        if expect_success {
            assert_eq!(rows_written?, 2);
        } else {
            rows_written.unwrap_err();
        }
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn import_zip_to_empty(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name1: TableName = "table1".parse()?;
    let table_name2: TableName = "table2".parse()?;
    let identity = new_admin_id();

    // Create tables (t1 then t2)
    let mut tx = app.begin(identity.clone()).await?;
    let mut ufm = UserFacingModel::new_root_for_test(&mut tx);
    ufm.insert(table_name1.clone(), assert_obj!()).await?;
    ufm.insert(table_name2.clone(), assert_obj!()).await?;
    app.commit_test(tx).await?;
    let export_object_key = app.export_and_wait().await?;

    for (mode, expect_success) in [
        (ImportMode::Append, true),
        (ImportMode::Replace, true),
        (ImportMode::ReplaceAll, true),
        (ImportMode::RequireEmpty, true),
    ] {
        let app = Application::new_for_tests(&rt).await?;
        let rows_written = do_import_from_object_key(
            &app,
            identity.clone(),
            ImportFormat::Zip,
            mode,
            ComponentPath::root(),
            export_object_key.clone(),
        )
        .await;
        tracing::info!("Imported in test for {mode}");
        if expect_success {
            assert_eq!(rows_written?, 2);
        } else {
            rows_written.unwrap_err();
        }
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn import_zip_to_same_deployment(rt: TestRuntime) -> anyhow::Result<()> {
    for (mode, expect_success) in [
        (ImportMode::Append, false),
        (ImportMode::Replace, true),
        (ImportMode::ReplaceAll, true),
        (ImportMode::RequireEmpty, false),
    ] {
        let app = Application::new_for_tests(&rt).await?;
        let table_name1: TableName = "table1".parse()?;
        let table_name2: TableName = "table2".parse()?;
        let identity = new_admin_id();

        // Create tables (t1 then t2)
        let mut tx = app.begin(identity.clone()).await?;
        let mut ufm = UserFacingModel::new_root_for_test(&mut tx);
        ufm.insert(table_name1.clone(), assert_obj!()).await?;
        ufm.insert(table_name2.clone(), assert_obj!()).await?;
        app.commit_test(tx).await?;
        let export_object_key = app.export_and_wait().await?;

        let rows_written = do_import_from_object_key(
            &app,
            identity.clone(),
            ImportFormat::Zip,
            mode,
            ComponentPath::root(),
            export_object_key.clone(),
        )
        .await;
        tracing::info!("Imported in test for {mode}");
        if expect_success {
            assert_eq!(rows_written?, 2);
        } else {
            rows_written.unwrap_err();
        }
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn import_copies_indexes(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name: TableName = "table1".parse()?;
    let test_csv = r#"
a
"string"
"#;
    let identity = new_admin_id();
    let index_name = IndexName::new(table_name.clone(), IndexDescriptor::new("by_a")?)?;

    let index_id = {
        let mut tx = app.begin(identity.clone()).await?;
        let mut index_model = IndexModel::new(&mut tx);
        let index_id = index_model
            .add_application_index(
                TableNamespace::test_user(),
                IndexMetadata::new_enabled(index_name.clone(), vec!["a".parse()?].try_into()?),
            )
            .await?;
        app.commit_test(tx).await?;
        index_id
    };

    run_csv_import(&app, &table_name, test_csv).await?;

    {
        let mut tx = app.begin(identity.clone()).await?;
        let mut index_model = IndexModel::new(&mut tx);
        let index = index_model
            .enabled_index_metadata(TableNamespace::test_user(), &index_name)?
            .context("index does not exist")?;
        assert_ne!(index.id(), index_id);
        assert!(index.config.is_enabled());
        must_let!(let IndexConfig::Database { spec, .. } = &index.config);
        assert_eq!(spec.fields[0], "a".parse()?);
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_import_counts_bandwidth(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let component_path = ComponentPath::root();
    let table_name: TableName = "table1".parse()?;
    let identity = new_admin_id();

    let storage_id = "kg21pzwemsm55e1fnt2kcsvgjh6h6gtf";
    let storage_idv6 = DeveloperDocumentId::decode(storage_id)?;

    let import = ParsedImport {
        generated_schemas: vec![],
        documents: vec![
            (
                component_path.clone(),
                "_storage".parse()?,
                stream::iter(vec![Ok(json!({"_id": storage_id}))]).boxed(),
            ),
            (
                component_path.clone(),
                table_name.clone(),
                stream::iter(vec![Ok(json!({"foo": "bar"})), Ok(json!({"foo": "baz"}))]).boxed(),
            ),
        ],
        storage_files: vec![(
            component_path.clone(),
            storage_idv6,
            stream::iter(vec![Ok(Bytes::from_static(b"foobarbaz"))]).boxed(),
        )],
    };

    let usage = FunctionUsageTracker::new();

    import_objects(
        &app.database,
        &app.file_storage,
        identity,
        &vec![],
        ImportMode::Replace,
        import,
        usage.clone(),
        None,
        ImportRequestor::SnapshotImport,
    )
    .await?;

    let stats = usage.gather_user_stats();
    assert!(stats.database_ingress_size[&(component_path.clone(), table_name.to_string())] > 0);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_import_file_storage_changing_table_number(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let old_storage_id: DeveloperDocumentId = "4d9wy5r5x7rmjdjqnx45ct829fff4ar".parse()?;
    let import = ParsedImport {
        generated_schemas: vec![],
        documents: vec![(
            ComponentPath::root(),
            "_storage".parse()?,
            stream::iter(vec![Ok(json!({"_id": old_storage_id.to_string()}))]).boxed(),
        )],
        storage_files: vec![(
            ComponentPath::root(),
            old_storage_id,
            stream::iter(vec![Ok(Bytes::from_static(b"foobarbaz"))]).boxed(),
        )],
    };

    // Regression test: used to fail with "cannot find table with id 35"
    import_objects(
        &app.database,
        &app.file_storage,
        new_admin_id(),
        &vec![],
        ImportMode::Replace,
        import,
        FunctionUsageTracker::new(),
        None,
        ImportRequestor::SnapshotImport,
    )
    .await?;

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_import_into_component(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    app.load_component_tests_modules("with-schema").await?;
    let table_name: TableName = "table1".parse()?;
    let component_path: ComponentPath = "component".parse()?;
    let test_csv = r#"
a,b
"foo","bar"
"#;
    do_import(
        &app,
        new_admin_id(),
        ImportFormat::Csv(table_name.clone()),
        ImportMode::Replace,
        component_path.clone(),
        stream_from_str(test_csv),
    )
    .await?;

    let mut tx = app.begin(new_admin_id()).await?;
    assert!(!TableModel::new(&mut tx).table_exists(ComponentId::Root.into(), &table_name));
    let (_, component_id) =
        BootstrapComponentsModel::new(&mut tx).must_component_path_to_ids(&component_path)?;
    assert_eq!(tx.must_count(component_id.into(), &table_name).await?, 1);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_import_into_missing_component(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name: TableName = "table1".parse()?;
    let component_path: ComponentPath = "component".parse()?;
    let test_csv = r#"
a,b
"foo","bar"
"#;
    let num_rows_written = do_import(
        &app,
        new_admin_id(),
        ImportFormat::Csv(table_name.clone()),
        ImportMode::Replace,
        component_path.clone(),
        stream_from_str(test_csv),
    )
    .await?;

    assert_eq!(num_rows_written, 1);

    let mut tx = app.begin(new_admin_id()).await?;
    let metadata = BootstrapComponentsModel::new(&mut tx)
        .resolve_path(&component_path)?
        .context("Component missing")?
        .into_value();
    assert_eq!(metadata.state, ComponentState::Unmounted);
    Ok(())
}

async fn activate_schema<RT: Runtime>(
    app: &Application<RT>,
    schema: DatabaseSchema,
) -> anyhow::Result<()> {
    let mut tx = app.begin(new_admin_id()).await?;
    let mut model = SchemaModel::new_root_for_test(&mut tx);
    let (schema_id, _) = model.submit_pending(schema).await?;
    model.mark_validated(schema_id).await?;
    model.mark_active(schema_id).await?;
    app.commit_test(tx).await?;
    Ok(())
}

/// Returns a BTreeMap for every item in the given table that contains only
/// the requesetd fields provided in `relevant_fields`. If one or more
/// fields in `relevant_fields` are missing in one or more objects in the
/// table, then the returned BTreeMap will not have an entry for the
/// missing fields.
async fn load_fields_as_maps<'a, RT: Runtime>(
    app: &Application<RT>,
    table_name: &str,
    relevant_fields: Vec<&'a str>,
) -> anyhow::Result<Vec<BTreeMap<&'a str, ConvexValue>>> {
    let mut tx = app.begin(new_admin_id()).await?;
    let table_name = TableName::from_str(table_name)?;
    let query = common::query::Query::full_table_scan(table_name.clone(), Order::Asc);
    let mut query_stream = ResolvedQuery::new(&mut tx, TableNamespace::test_user(), query)?;

    let mut docs: Vec<ResolvedDocument> = Vec::new();
    while let Some(doc) = query_stream.next(&mut tx, None).await? {
        docs.push(doc);
        if docs.len() % 100 == 0 {
            // Occasionally start a new transaction in case there are lots
            // of documents.
            tx = app.begin(new_admin_id()).await?;
        }
    }

    let objects: Vec<ConvexObject> = docs.into_iter().map(|doc| doc.into_value().0).collect();

    let mut fields_list: Vec<BTreeMap<&str, ConvexValue>> = Vec::default();
    for object in objects {
        let mut current = BTreeMap::default();
        for field in &relevant_fields {
            let value = object.get(&FieldName::from_str(field)?);
            if let Some(value) = value {
                current.insert(*field, value.clone());
            }
        }
        fields_list.push(current);
    }
    Ok(fields_list)
}

fn new_admin_id() -> Identity {
    Identity::InstanceAdmin(AdminIdentity::new_for_test_only(
        "test".to_string(),
        MemberId(1),
    ))
}

async fn run_csv_import(
    app: &Application<TestRuntime>,
    table_name: &str,
    input: &str,
) -> anyhow::Result<()> {
    do_import(
        app,
        new_admin_id(),
        ImportFormat::Csv(table_name.parse()?),
        ImportMode::Replace,
        ComponentPath::root(),
        stream_from_str(input),
    )
    .await
    .map(|_| ())
}

#[convex_macro::test_runtime]
async fn test_cancel_in_progress_import(
    rt: TestRuntime,
    pause_controller: PauseController,
) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    let table_name = "table1";
    let test_csv = r#"
a,b
"foo","bar"
"#;

    let hold_guard = pause_controller.hold("before_finalize_import");

    let mut import_fut = run_csv_import(&app, table_name, test_csv).boxed();

    select! {
        r = import_fut.as_mut().fuse() => {
            anyhow::bail!("import finished before pausing: {r:?}");
        },
        pause_guard = hold_guard.wait_for_blocked().fuse() => {
            let pause_guard = pause_guard.unwrap();

            // Cancel the import while it's in progress
            let mut tx = app.begin(new_admin_id()).await?;
            let mut import_model = model::snapshot_imports::SnapshotImportModel::new(&mut tx);

            // Find the in-progress import
            let snapshot_import = import_model.import_in_state(ImportState::InProgress {
                progress_message: String::new(),
                checkpoint_messages: vec![],
            }).await?.context("No in-progress import found")?;

            import_model.cancel_import(snapshot_import.id()).await?;
            app.commit_test(tx).await?;

            pause_guard.unpause();
        },
    }

    let err = import_fut.await.unwrap_err();
    assert!(err.is_bad_request());
    assert!(
        err.msg().contains("Import canceled"),
        "Unexpected error message: {}",
        err.msg()
    );

    // Verify the import was actually canceled
    let mut tx = app.begin(new_admin_id()).await?;
    let mut import_model = model::snapshot_imports::SnapshotImportModel::new(&mut tx);
    let snapshot_import = import_model
        .import_in_state(ImportState::Failed("Import was canceled".into()))
        .await?
        .context("No failed import found")?;
    assert!(matches!(
        snapshot_import.state.clone(),
        ImportState::Failed(msg) if msg == "Import canceled"
    ));
    // Verify no data written
    let table_name = TableName::from_str(table_name)?;
    let table_size = tx
        .must_count(TableNamespace::test_user(), &table_name)
        .await?;
    assert_eq!(table_size, 0);
    assert!(!TableModel::new(&mut tx).table_exists(TableNamespace::test_user(), &table_name));

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_utf8_bom_jsonarray(rt: TestRuntime) -> anyhow::Result<()> {
    // UTF-8 BOM is the byte sequence: EF BB BF
    let utf8_bom = [0xEF, 0xBB, 0xBF];

    // Test JsonArray format with UTF-8 BOM - should now produce an error
    let json_content = r#"[{"name": "test", "value": 42}, {"name": "hello", "value": 123}]"#;
    let mut content_with_bom = Vec::new();
    content_with_bom.extend_from_slice(&utf8_bom);
    content_with_bom.extend_from_slice(json_content.as_bytes());

    let result = run_parse_objects(
        rt.clone(),
        ImportFormat::JsonArray("test_table".parse()?),
        content_with_bom,
    )
    .await;

    // Should fail with UTF-8 BOM error
    assert!(result.is_err());
    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("UTF-8 BOM is not supported"));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_utf8_bom_jsonlines(rt: TestRuntime) -> anyhow::Result<()> {
    // UTF-8 BOM is the byte sequence: EF BB BF
    let utf8_bom = [0xEF, 0xBB, 0xBF];

    // Test JsonLines format with UTF-8 BOM - should now produce an error
    let jsonl_content = r#"{"name": "test", "value": 42}
{"name": "hello", "value": 123}
{"name": "world", "value": 456}"#;
    let mut content_with_bom = Vec::new();
    content_with_bom.extend_from_slice(&utf8_bom);
    content_with_bom.extend_from_slice(jsonl_content.as_bytes());

    let result = run_parse_objects(
        rt.clone(),
        ImportFormat::JsonLines("test_table".parse()?),
        content_with_bom,
    )
    .await;

    // Should fail with UTF-8 BOM error
    assert!(result.is_err());
    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("UTF-8 BOM is not supported"));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_utf8_bom_jsonarray_without_bom(rt: TestRuntime) -> anyhow::Result<()> {
    // Test JsonArray format without UTF-8 BOM (should still work)
    let json_content = r#"[{"name": "test", "value": 42}]"#;

    let objects = run_parse_objects(
        rt,
        ImportFormat::JsonArray("test_table".parse()?),
        json_content,
    )
    .await?;

    let expected = vec![json!({"name": "test", "value": 42})];
    assert_eq!(objects, expected);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_utf8_bom_jsonlines_without_bom(rt: TestRuntime) -> anyhow::Result<()> {
    // Test JsonLines format without UTF-8 BOM (should still work)
    let jsonl_content = r#"{"name": "test", "value": 42}
{"name": "hello", "value": 123}"#;

    let objects = run_parse_objects(
        rt,
        ImportFormat::JsonLines("test_table".parse()?),
        jsonl_content,
    )
    .await?;

    let expected = vec![
        json!({"name": "test", "value": 42}),
        json!({"name": "hello", "value": 123}),
    ];
    assert_eq!(objects, expected);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_utf8_bom_jsonlines_empty_lines(rt: TestRuntime) -> anyhow::Result<()> {
    // UTF-8 BOM is the byte sequence: EF BB BF
    let utf8_bom = [0xEF, 0xBB, 0xBF];

    // Test JsonLines format with UTF-8 BOM and empty lines - should now produce an
    // error
    let jsonl_content = r#"{"name": "test", "value": 42}

{"name": "hello", "value": 123}

{"name": "world", "value": 456}"#;
    let mut content_with_bom = Vec::new();
    content_with_bom.extend_from_slice(&utf8_bom);
    content_with_bom.extend_from_slice(jsonl_content.as_bytes());

    let result = run_parse_objects(
        rt.clone(),
        ImportFormat::JsonLines("test_table".parse()?),
        content_with_bom,
    )
    .await;

    // Should fail with UTF-8 BOM error
    assert!(result.is_err());
    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("UTF-8 BOM is not supported"));
    Ok(())
}

/// Test we can import over a componentless namespace. Componentless namespaces
/// are created during start_push - the component is only created during
/// finish_push
#[convex_macro::test_runtime]
async fn test_import_over_componentless_namespace(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;

    // Do just a start_push w/o finish_push
    let request = Application::<TestRuntime>::load_start_push_request(Path::new("basic"))?;
    let config = request.into_project_config()?;
    app.start_push(&config).await?;

    let test_csv = r#"
a,b
"foo","bar"
"#;
    let num_rows_written = do_import(
        &app,
        new_admin_id(),
        ImportFormat::Csv("table1".parse()?),
        ImportMode::ReplaceAll,
        ComponentPath::root(),
        stream_from_str(test_csv),
    )
    .await?;
    assert_eq!(num_rows_written, 1);
    Ok(())
}
