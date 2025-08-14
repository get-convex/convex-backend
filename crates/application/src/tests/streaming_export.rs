use common::{
    assert_obj,
    components::ComponentPath,
    runtime::Runtime,
};
use convex_macro::test_runtime;
use database::StreamingExportFilter;
use keybroker::Identity;
use runtime::testing::TestRuntime;
use value::{
    TableName,
    TableNamespace,
};

use crate::{
    test_helpers::ApplicationTestExt,
    Application,
    BootstrapComponentsModel,
    UserFacingModel,
};

pub fn table_name() -> TableName {
    "table1".parse().unwrap()
}

#[test_runtime]
async fn test_streaming_export_from_component(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;

    application.load_component_tests_modules("mounted").await?;
    let component_path = "component".parse()?;

    // Insert documents in the component
    let mut tx = application.begin(Identity::system()).await?;
    insert_documents(&mut tx, component_path, &table_name()).await?;
    application.commit_test(tx).await?;

    // Get all documents through list_snapshot
    let mut seen_documents = Vec::new();
    let mut snapshot = None;
    let mut cursor = None;

    loop {
        let snapshot_page = application
            .database
            .list_snapshot(
                Identity::system(),
                snapshot,
                cursor,
                StreamingExportFilter::default(),
                5,
                5,
            )
            .await?;

        snapshot = Some(snapshot_page.snapshot);
        cursor = snapshot_page.cursor;

        for (_, _, _, doc) in snapshot_page.documents {
            seen_documents.push(doc);
        }

        if !snapshot_page.has_more {
            break;
        }
    }

    // Verify we got all documents from both components
    assert_eq!(seen_documents.len(), 10);

    Ok(())
}

async fn insert_documents<RT: Runtime>(
    tx: &mut crate::Transaction<RT>,
    component_path: ComponentPath,
    table_name: &TableName,
) -> anyhow::Result<()> {
    let mut components_model = BootstrapComponentsModel::new(tx);
    let (_, component_id) = components_model.must_component_path_to_ids(&component_path)?;
    let table_namespace = TableNamespace::from(component_id);
    let mut user_facing_model = UserFacingModel::new(tx, table_namespace);

    for i in 0..10 {
        user_facing_model
            .insert(table_name.clone(), assert_obj!("index" => i))
            .await?;
    }

    Ok(())
}
