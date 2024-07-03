use common::{
    components::ComponentId,
    testing::TestPersistence,
};
use keybroker::Identity;
use model::file_storage::FileStorageId;
use must_let::must_let;
use runtime::testing::TestRuntime;
use value::{
    assert_obj,
    ConvexValue,
};

use crate::{
    test_helpers::UdfTest,
    tests::action::action_udf_test,
    ActionCallbacks,
};

#[convex_macro::test_runtime]
async fn test_storage_store_get(rt: TestRuntime) -> anyhow::Result<()> {
    let t = action_udf_test(rt).await?;

    let data = ConvexValue::Bytes("data".as_bytes().to_vec().try_into()?);
    let id = t
        .action("storage:storeFile", assert_obj!("data" => data.clone()))
        .await?;

    let retrieved = t.action("storage:getFile", assert_obj!("id" => id)).await?;
    assert_eq!(data, retrieved);
    Ok(())
}

async fn check_storage_url(
    t: &UdfTest<TestRuntime, TestPersistence>,
    url: &ConvexValue,
    id: &ConvexValue,
) -> anyhow::Result<()> {
    must_let!(let ConvexValue::String(url_str) = url);
    must_let!(let ConvexValue::String(id_str) = id);
    must_let!(let Some(internal_id) = url_str.strip_prefix("http://127.0.0.1:8000/api/storage/"));
    must_let!(
        let Some(storage_entry) = t.storage_get_file_entry(
            Identity::system(), ComponentId::test_user(), FileStorageId::DocumentId(id_str.parse()?)
        ).await?
    );
    assert_eq!(storage_entry.storage_id, internal_id.parse()?);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_storage_get_url(rt: TestRuntime) -> anyhow::Result<()> {
    let t = action_udf_test(rt).await?;

    let data = ConvexValue::Bytes("data".as_bytes().to_vec().try_into()?);
    let id = t
        .action("storage:storeFile", assert_obj!("data" => data.clone()))
        .await?;

    let url = t
        .query("storage:getFileUrl", assert_obj!("id" => id.clone()))
        .await?;

    check_storage_url(&t, &url, &id).await?;

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_storage_get_url_parallel(rt: TestRuntime) -> anyhow::Result<()> {
    let t = action_udf_test(rt).await?;

    let data = ConvexValue::Bytes("data".as_bytes().to_vec().try_into()?);
    let mut ids = Vec::new();
    // Default parallel chunk size is 16.
    for _ in 0..20 {
        ids.push(
            t.action("storage:storeFile", assert_obj!("data" => data.clone()))
                .await?,
        );
    }
    let urls = t
        .query("storage:getFileUrls", assert_obj!("ids" => ids.clone()))
        .await?;
    must_let!(let ConvexValue::Array(urls) = urls);
    for idx in 0..20 {
        check_storage_url(&t, &urls[idx], &ids[idx]).await?;
    }

    Ok(())
}
