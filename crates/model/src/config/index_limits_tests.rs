use common::{
    db_schema_with_vector_indexes,
    object_validator,
    persistence::Persistence,
    runtime::Runtime,
    schemas::{
        validator::{
            FieldValidator,
            Validator,
        },
        DocumentSchema,
        VECTOR_DIMENSIONS,
    },
};
use database::{
    test_helpers::{
        vector_utils::{
            random_vector_value,
            random_vector_with_dimens,
            vector_to_value,
        },
        DbFixtures,
    },
    Database,
};
use errors::ErrorMetadata;
use keybroker::Identity;
use runtime::testing::TestRuntime;
use sync_types::Timestamp;
use value::{
    assert_obj,
    ConvexValue,
};

use crate::{
    config::index_test_utils::deploy_schema,
    test_helpers::DbFixturesWithModel,
};

const TABLE: &str = "table";
const VECTOR_FIELD: &str = "field";

async fn commit_schema(
    rt: &TestRuntime,
    tp: Box<dyn Persistence>,
    db: &Database<TestRuntime>,
) -> anyhow::Result<()> {
    let document_schema = DocumentSchema::Union(vec![object_validator!(
        VECTOR_FIELD =>
            FieldValidator::required_field_type(
                Validator::Array(Box::new(Validator::Float64))
            )
    )]);
    let db_schema = db_schema_with_vector_indexes!(
        TABLE => {document_schema, [("myVectorIndex", VECTOR_FIELD)]}
    );
    deploy_schema(rt, tp, db, db_schema).await
}

#[convex_macro::test_runtime]
async fn insert_vector_doc_under_vector_limit_succeeds(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, tp, .. } = DbFixtures::new(&rt).await?.with_model().await?;
    commit_schema(&rt, tp, &db).await?;

    let vector = rt.with_rng(random_vector_value);

    let mut tx = db.begin(Identity::system()).await?;
    tx.insert_user_facing(TABLE.parse()?, assert_obj!(VECTOR_FIELD => vector))
        .await?;
    db.commit(tx).await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn insert_vector_doc_over_vector_limit_fails(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, tp, .. } = DbFixtures::new(&rt).await?.with_model().await?;
    commit_schema(&rt, tp, &db).await?;

    let vector = rt.with_rng(random_vector_value);

    let mut tx = db.begin(Identity::system()).await?;
    tx.set_index_size_hard_limit(0);
    tx.insert_user_facing(TABLE.parse()?, assert_obj!(VECTOR_FIELD => vector))
        .await?;
    assert_vector_index_too_large_error(db.commit(tx).await)
}

fn assert_vector_index_too_large_error(result: anyhow::Result<Timestamp>) -> anyhow::Result<()> {
    assert_eq!(
        result
            .unwrap_err()
            .downcast::<ErrorMetadata>()
            .unwrap()
            .short_msg,
        "VectorIndexTooLarge"
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn insert_doc_in_other_table_over_vector_limit_succeeds(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let DbFixtures { db, tp, .. } = DbFixtures::new(&rt).await?.with_model().await?;
    commit_schema(&rt, tp, &db).await?;

    let vector = random_1536_vector_value(&rt);

    let mut tx = db.begin(Identity::system()).await?;
    tx.insert_user_facing("otherTable".parse()?, assert_obj!(VECTOR_FIELD => vector))
        .await?;
    db.commit(tx).await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn insert_doc_in_same_table_without_vector_succeeds(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, tp, .. } = DbFixtures::new(&rt).await?.with_model().await?;
    commit_schema(&rt, tp, &db).await?;

    let mut tx = db.begin(Identity::system()).await?;
    tx.insert_user_facing(
        "otherTable".parse()?,
        assert_obj!(VECTOR_FIELD => ConvexValue::String("something".to_string().try_into()?)),
    )
    .await?;
    db.commit(tx).await?;
    Ok(())
}

fn random_1536_vector_value(rt: &TestRuntime) -> ConvexValue {
    rt.with_rng(|rng| vector_to_value(random_vector_with_dimens(rng, VECTOR_DIMENSIONS)))
}

// This looks like it should succeed, but here's why it doesn't:
// First we add a document to the memory index, incrementing the size
// Then we delete the document
// Which removes the document we just added, setting the size back to 0
// But it also adds a tombstone, incrementing the size again.
// The total size increase is now > 0, so the transaction fails.
// The test above passes because we start with a size > 0, then swap a document
// for a tombstone, which decreases the total size.
#[convex_macro::test_runtime]
async fn insert_and_delete_vector_doc_over_hard_limit_fails(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, tp, .. } = DbFixtures::new(&rt).await?.with_model().await?;

    commit_schema(&rt, tp, &db).await?;

    let vector = random_1536_vector_value(&rt);
    let mut tx = db.begin(Identity::system()).await?;
    tx.insert_user_facing(TABLE.parse()?, assert_obj!(VECTOR_FIELD => vector.clone()))
        .await?;
    db.commit(tx).await?;

    let mut tx = db.begin(Identity::system()).await?;
    tx.set_index_size_hard_limit(0);
    let id = tx
        .insert_user_facing(TABLE.parse()?, assert_obj!(VECTOR_FIELD => vector.clone()))
        .await?;
    tx.delete_user_facing(id).await?;
    assert_vector_index_too_large_error(db.commit(tx).await)
}
