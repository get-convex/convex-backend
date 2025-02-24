use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    time::Duration,
};

use common::{
    bootstrap_model::index::{
        database_index::IndexedFields,
        IndexMetadata,
    },
    runtime::Runtime,
    testing::TestPersistence,
    types::{
        IndexDescriptor,
        IndexName,
    },
};
use database::IndexModel;
use futures::{
    pin_mut,
    select_biased,
    FutureExt,
};
use keybroker::Identity;
use maplit::btreemap;
use model::airbyte_import::AIRBYTE_PRIMARY_KEY_INDEX_DESCRIPTOR;
use runtime::testing::TestRuntime;
use value::{
    FieldPath,
    TableName,
    TableNamespace,
};

use crate::{
    airbyte_import::PrimaryKey,
    test_helpers::{
        ApplicationFixtureArgs,
        ApplicationTestExt,
    },
    Application,
};

#[convex_macro::test_runtime]
async fn test_system_indexes(rt: TestRuntime) -> anyhow::Result<()> {
    let persistence = TestPersistence::new();
    let application = Application::new_for_tests_with_args(
        &rt,
        ApplicationFixtureArgs {
            tp: Some(persistence.clone()),
            ..Default::default()
        },
    )
    .await?;

    let (index_name, indexes) = new_system_indexes_on_user_table()?;

    let validated_index_metadata = IndexMetadata::new_backfilling(
        *application.now_ts_for_reads(),
        index_name.clone(),
        application
            .validate_user_defined_index_fields(indexes.get(&index_name).unwrap().clone())?,
    );

    // Add index, check that it's there
    application
        ._add_system_indexes(&Identity::system(), indexes.clone())
        .await?;
    wait_for_backfill(&rt, &application, TableNamespace::test_user(), &index_name).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let found_index = IndexModel::new(&mut tx)
        .enabled_index_metadata(TableNamespace::test_user(), &index_name)?
        .unwrap();
    assert!(found_index
        .config
        .same_config(&validated_index_metadata.config));

    // Add the same index, should be no-op
    application
        ._add_system_indexes(&Identity::system(), indexes)
        .await?;
    let found_index = IndexModel::new(&mut tx)
        .enabled_index_metadata(TableNamespace::test_user(), &index_name)?
        .unwrap();
    assert!(found_index
        .config
        .same_config(&validated_index_metadata.config));

    // Add different index field, should update
    let field_path: FieldPath = "field".parse()?;
    let field_paths = vec![field_path];
    let index_fields: IndexedFields = field_paths.try_into()?;
    let indexes = btreemap! {index_name.clone() => index_fields.clone()};
    application
        ._add_system_indexes(&Identity::system(), indexes)
        .await?;
    wait_for_backfill(&rt, &application, TableNamespace::test_user(), &index_name).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let found_index = IndexModel::new(&mut tx)
        .enabled_index_metadata(TableNamespace::test_user(), &index_name)?
        .unwrap();
    let validated_index_metadata = IndexMetadata::new_backfilling(
        *application.now_ts_for_reads(),
        index_name.clone(),
        application.validate_user_defined_index_fields(index_fields.clone())?,
    );
    assert!(found_index
        .config
        .same_config(&validated_index_metadata.config));

    // Add many indexes
    let new_index_name: IndexName = IndexName::new_reserved(
        "table2".parse()?,
        AIRBYTE_PRIMARY_KEY_INDEX_DESCRIPTOR.clone(),
    )?;
    let new_index_fields = vec!["id".parse()?].try_into()?;
    let indexes =
        btreemap! {index_name.clone() => index_fields, new_index_name.clone() => new_index_fields};
    application
        ._add_system_indexes(&Identity::system(), indexes)
        .await?;
    wait_for_backfill(&rt, &application, TableNamespace::test_user(), &index_name).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let found_index = IndexModel::new(&mut tx)
        .enabled_index_metadata(TableNamespace::test_user(), &index_name)?
        .unwrap();
    assert!(found_index
        .config
        .same_config(&validated_index_metadata.config));
    assert!(IndexModel::new(&mut tx)
        .enabled_index_metadata(TableNamespace::test_user(), &new_index_name)?
        .is_some());
    Ok(())
}

/// Observe a specific index via a subscription on all indexes returning
/// successfully when that index moves into the enabled state.
///
/// If the index is not enabled within a timeout, the method fails.
///
/// This method will only work if we're writing a test where an index worker is
/// already active ( e.g. the application). Some other worker must be running or
/// the index state will never advance. Similarly, while this method looks
/// generally, it'll currently only work for database indexes. Search indexes
/// will fail because their index workers only advance them to the `Backfilled`
/// state while this method will only succeed if the index moves to `Enabled`.
/// We could make this modification if necessary in the future.
async fn wait_for_backfill(
    rt: &TestRuntime,
    application: &Application<TestRuntime>,
    namespace: TableNamespace,
    index_name: &IndexName,
) -> anyhow::Result<()> {
    let timeout = rt.wait(Duration::from_secs(5));
    pin_mut!(timeout);
    let mut timeout = timeout.fuse();
    loop {
        let db = &application.database;
        let mut tx = db.begin_system().await?;
        let mut model = IndexModel::new(&mut tx);
        if model
            .enabled_index_metadata(namespace, index_name)?
            .is_some()
        {
            return Ok(());
        }

        model.get_all_indexes().await?;
        let token = tx.into_token()?;
        let subscription = db.subscribe(token).await?;
        let subscription_fut = subscription.wait_for_invalidation();
        pin_mut!(subscription_fut);
        select_biased! {
            _ = subscription_fut.fuse() => {},
            _ = timeout => {
                anyhow::bail!("Timed out!");
            }
        }
    }
}

fn new_system_indexes_on_user_table(
) -> anyhow::Result<(IndexName, BTreeMap<IndexName, IndexedFields>)> {
    let table_name: TableName = "table".parse()?;
    let index_name: IndexName =
        IndexName::new_reserved(table_name, AIRBYTE_PRIMARY_KEY_INDEX_DESCRIPTOR.clone())?;
    new_indexes(index_name)
}

fn new_indexes(
    index_name: IndexName,
) -> anyhow::Result<(IndexName, BTreeMap<IndexName, IndexedFields>)> {
    let field: FieldPath = "some_field".parse()?;
    let index_fields: IndexedFields = vec![field].try_into()?;
    Ok((index_name.clone(), btreemap! {index_name => index_fields}))
}

#[convex_macro::test_runtime]
async fn new_system_indexes_are_enabled_automatically_by_index_worker(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let persistence = TestPersistence::new();
    let application = Application::new_for_tests_with_args(
        &rt,
        ApplicationFixtureArgs {
            tp: Some(persistence.clone()),
            ..Default::default()
        },
    )
    .await?;

    let (index_name, indexes) = new_system_indexes_on_user_table()?;
    application
        ._add_system_indexes(&Identity::system(), indexes.clone())
        .await?;
    wait_for_backfill(&rt, &application, TableNamespace::test_user(), &index_name).await?;

    let mut tx = application.begin(Identity::system()).await?;
    IndexModel::new(&mut tx)
        .enabled_index_metadata(TableNamespace::test_user(), &index_name)?
        .unwrap();
    Ok(())
}

#[convex_macro::test_runtime]
async fn new_indexes_on_system_tables_are_enabled_automatically_by_index_worker(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let persistence = TestPersistence::new();
    let application = Application::new_for_tests_with_args(
        &rt,
        ApplicationFixtureArgs {
            tp: Some(persistence.clone()),
            ..Default::default()
        },
    )
    .await?;

    let table_name: TableName = "_my_system_table".parse()?;
    let mut tx = application.begin(Identity::system()).await?;
    assert!(
        tx.create_system_table_testing(TableNamespace::test_user(), &table_name, None)
            .await?
    );
    application.commit_test(tx).await?;

    let index_name = IndexName::new(table_name, IndexDescriptor::new("my_index")?)?;
    let (index_name, indexes) = new_indexes(index_name)?;
    application
        ._add_system_indexes(&Identity::system(), indexes.clone())
        .await?;
    wait_for_backfill(&rt, &application, TableNamespace::test_user(), &index_name).await?;

    let mut tx = application.begin(Identity::system()).await?;
    IndexModel::new(&mut tx)
        .enabled_index_metadata(TableNamespace::test_user(), &index_name)?
        .unwrap();
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_indexes_ready(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let table_1 = "table_1".parse::<TableName>()?;
    let table_2 = "table_2".parse::<TableName>()?;
    let primary_key = PrimaryKey::try_from(vec![vec!["id".to_string()]])?;
    let indexes =
        btreemap! {table_1.clone() => primary_key.clone(), table_2.clone() => primary_key};
    application
        .add_primary_key_indexes(&Identity::system(), indexes)
        .await?;

    let indexes = BTreeSet::from([table_1, table_2]);
    application
        .wait_for_primary_key_indexes_ready(Identity::system(), indexes.clone())
        .await?;
    let ready = application
        .primary_key_indexes_ready(Identity::system(), indexes)
        .await?;
    assert!(ready);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_indexes_ready_can_be_called_multiple_times(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let table_1 = "table_1".parse::<TableName>()?;
    let table_2 = "table_2".parse::<TableName>()?;
    let primary_key = PrimaryKey::try_from(vec![vec!["id".to_string()]])?;
    let indexes =
        btreemap! {table_1.clone() => primary_key.clone(), table_2.clone() => primary_key};
    application
        .add_primary_key_indexes(&Identity::system(), indexes)
        .await?;

    let indexes = BTreeSet::from([table_1.clone(), table_2.clone()]);
    application
        .wait_for_primary_key_indexes_ready(Identity::system(), indexes.clone())
        .await?;
    let ready = application
        .primary_key_indexes_ready(Identity::system(), indexes.clone())
        .await?;
    assert!(ready);
    let ready = application
        .primary_key_indexes_ready(Identity::system(), indexes.clone())
        .await?;
    assert!(ready);

    Ok(())
}
