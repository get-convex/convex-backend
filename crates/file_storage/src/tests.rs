use std::sync::Arc;

use common::{
    runtime::Runtime,
    sha256::Sha256,
};
use database::{
    test_helpers::{
        new_test_database,
        DbFixtures,
    },
    Database,
};
use errors::{
    ErrorCode,
    ErrorMetadata,
};
use events::usage::NoOpUsageEventLogger;
use futures::stream;
use keybroker::Identity;
use model::{
    file_storage::FileStorageId,
    test_helpers::DbFixturesWithModel,
};
use runtime::testing::TestRuntime;
use storage::LocalDirStorage;
use usage_tracking::UsageCounter;
use value::TableNamespace;

use super::FileStorage;
use crate::TransactionalFileStorage;

fn setup_file_storage(
    rt: TestRuntime,
    database: &Database<TestRuntime>,
) -> anyhow::Result<FileStorage<TestRuntime>> {
    let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
    let file_storage = TransactionalFileStorage::new(rt, storage, "http://127.0.0.1:8000".into());
    Ok(FileStorage {
        database: database.clone(),
        transactional_file_storage: file_storage,
    })
}

#[convex_macro::test_runtime]
async fn test_get_file_404(rt: TestRuntime) -> anyhow::Result<()> {
    let database = DbFixtures::new_with_model(&rt).await?.db;
    let file_storage = setup_file_storage(rt.clone(), &database)?;

    let bogus_storage_id = FileStorageId::LegacyStorageId(rt.new_uuid_v4().into());
    let mut tx = database.begin(Identity::system()).await?;
    assert!(file_storage
        .transactional_file_storage
        .get_file_entry(&mut tx, TableNamespace::test_user(), bogus_storage_id)
        .await?
        .is_none());

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_store_file_sha_mismatch(rt: TestRuntime) -> anyhow::Result<()> {
    let database = new_test_database(rt.clone()).await;
    let file_storage = setup_file_storage(rt, &database)?;

    let big_file_size = 2 * 1024 * 1024;

    let big_file = vec![55; big_file_size + 1];
    let wrong = Sha256::hash(b"Wrong thing");
    let err: ErrorMetadata = file_storage
        .store_file(
            TableNamespace::test_user(),
            None,
            None,
            stream::iter([Ok(big_file)]),
            Some(wrong.clone()),
            &UsageCounter::new(Arc::new(NoOpUsageEventLogger)),
        )
        .await
        .unwrap_err()
        .downcast()?;
    assert_eq!(err.code, ErrorCode::BadRequest);
    assert_eq!(err.short_msg, "Sha256Mismatch");

    Ok(())
}
