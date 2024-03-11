use std::str::FromStr;

use common::{
    bootstrap_model::schema::{
        SchemaMetadata,
        SchemaState,
    },
    db_schema,
    object_validator,
    runtime::Runtime,
    schemas::{
        validator::{
            FieldValidator,
            ValidationContext,
            ValidationError,
            Validator,
        },
        DatabaseSchema,
        DocumentSchema,
        SchemaValidationError,
    },
    value::{
        id_v6::DocumentIdV6,
        ConvexValue,
        ResolvedDocumentId,
        TableName,
    },
};
use errors::ErrorMetadataAnyhowExt;
use keybroker::Identity;
use runtime::testing::TestRuntime;
use value::assert_obj;

use crate::{
    bootstrap_model::schema::MAX_TIME_TO_KEEP_FAILED_AND_OVERWRITTEN_SCHEMAS,
    test_helpers::{
        new_test_database,
        new_tx,
    },
    SchemaModel,
    Transaction,
    UserFacingModel,
};

#[convex_macro::test_runtime]
async fn test_submit_same_pending_schema(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt.clone()).await;
    let mut tx = db.begin(Identity::system()).await?;
    let mut model = SchemaModel::new(&mut tx);

    assert!(model.get_by_state(SchemaState::Pending).await?.is_none());
    let db_schema = DatabaseSchema::default();
    let (id, state) = model.submit_pending(db_schema.clone()).await?;
    assert_eq!(state, SchemaState::Pending);
    assert_eq!(
        model.get_by_state(SchemaState::Pending).await?,
        Some((id, db_schema.clone()))
    );

    // Submitting the same schema should give back the same id
    assert_eq!(
        model.submit_pending(db_schema.clone()).await?,
        (id, SchemaState::Pending)
    );

    // Submitting a pending schema the same as a validated schema should return the
    // id of the validated schema
    model.mark_validated(id).await?;
    assert_eq!(
        model.submit_pending(db_schema.clone()).await?,
        (id, SchemaState::Validated)
    );
    // Submitting a pending schema the same as an active schema should return the id
    // of the active schema
    model.mark_active(id).await?;
    assert_eq!(
        model.submit_pending(db_schema.clone()).await?,
        (id, SchemaState::Active)
    );

    // Submitting a pending schema that matches the active schema should overwrite
    // any existing pending or validated schemas
    let new_db_schema = db_schema!("table" => DocumentSchema::Any);
    model.submit_pending(new_db_schema.clone()).await?;
    model.submit_pending(db_schema.clone()).await?;
    assert!(model.get_by_state(SchemaState::Pending).await?.is_none());

    let (id, state) = model.submit_pending(new_db_schema.clone()).await?;
    assert_eq!(state, SchemaState::Pending);
    model.mark_validated(id).await?;
    let (_id, state) = model.submit_pending(db_schema.clone()).await?;
    assert_eq!(state, SchemaState::Active);
    assert!(model.get_by_state(SchemaState::Pending).await?.is_none());

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_submit_new_pending_schema(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt.clone()).await;
    let mut tx = db.begin(Identity::system()).await?;
    let mut model = SchemaModel::new(&mut tx);

    let db_schema_1 = DatabaseSchema::default();
    let (id, state) = model.submit_pending(db_schema_1.clone()).await?;
    assert_eq!(state, SchemaState::Pending);
    assert_eq!(
        model.get_by_state(SchemaState::Pending).await?,
        Some((id, db_schema_1.clone()))
    );
    // New schema submitted should replace the old pending schema
    let db_schema_2 = db_schema!("table" => DocumentSchema::Any);
    let (db_schema_2_id, schema_2_state) = model.submit_pending(db_schema_2.clone()).await?;
    assert_eq!(schema_2_state, SchemaState::Pending);
    assert_ne!(db_schema_2_id, id);
    let pending_schema = model.get_by_state(SchemaState::Pending).await?;
    assert_eq!(pending_schema, Some((db_schema_2_id, db_schema_2.clone())));
    model.mark_validated(db_schema_2_id).await?;
    let validated_schema = model.get_by_state(SchemaState::Validated).await?;
    assert_eq!(validated_schema, Some((db_schema_2_id, db_schema_2)));
    assert!(model.get_by_state(SchemaState::Pending).await?.is_none());

    // Submit db_schema_1 as pending again, the validated schema should be
    // overwritten
    let (id, state) = model.submit_pending(db_schema_1.clone()).await?;
    assert_eq!(state, SchemaState::Pending);
    assert_eq!(
        model.get_by_state(SchemaState::Pending).await?,
        Some((id, db_schema_1.clone()))
    );
    assert!(model.get_by_state(SchemaState::Validated).await?.is_none());
    let SchemaMetadata { state, schema: _ } = tx
        .get(db_schema_2_id)
        .await?
        .unwrap()
        .into_value()
        .into_value()
        .try_into()?;
    assert_eq!(state, SchemaState::Overwritten);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_mark_schema_as_validated(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt.clone()).await;
    let mut tx = db.begin(Identity::system()).await?;
    let mut model = SchemaModel::new(&mut tx);

    let db_schema = DatabaseSchema::default();
    let (id, state) = model.submit_pending(db_schema.clone()).await?;
    assert_eq!(state, SchemaState::Pending);
    model.mark_validated(id).await?;
    let schema_metadata: SchemaMetadata = tx
        .get(id)
        .await?
        .unwrap()
        .into_value()
        .into_value()
        .try_into()?;
    assert_eq!(schema_metadata.state, SchemaState::Validated);
    // Marking an already validated schema as validated should fail
    let mut model = SchemaModel::new(&mut tx);
    assert_eq!(
        model.mark_validated(id).await.unwrap_err().to_string(),
        String::from("Schema is already validated.")
    );

    // Add another schema and make sure the old one is overwritten.
    let new_db_schema = db_schema!("new_table" => DocumentSchema::Any);
    let (new_id, state) = model.submit_pending(new_db_schema).await?;
    assert_eq!(state, SchemaState::Pending);
    model.mark_validated(new_id).await?;
    let schema_metadata: SchemaMetadata = tx
        .get(id)
        .await?
        .unwrap()
        .into_value()
        .into_value()
        .try_into()?;
    assert_eq!(schema_metadata.state, SchemaState::Overwritten);
    let mut model = SchemaModel::new(&mut tx);
    let error = model.mark_validated(id).await.unwrap_err();
    assert!(error.is_bad_request());
    assert_eq!(error.short_msg(), "SchemaAlreadyOverwritten");

    let schema_metadata: SchemaMetadata = tx
        .get(new_id)
        .await?
        .unwrap()
        .into_value()
        .into_value()
        .try_into()?;
    assert_eq!(schema_metadata.state, SchemaState::Validated);

    // Active schema cannot be marked validated
    let mut model = SchemaModel::new(&mut tx);
    model.mark_active(new_id).await?;
    assert_eq!(
        model.mark_validated(new_id).await.unwrap_err().to_string(),
        String::from("Schema is already active.")
    );

    // Check that we tolerate failed schemas
    let (failed_id, state) = model.submit_pending(db_schema).await?;
    assert_eq!(state, SchemaState::Pending);
    let schema_error = SchemaValidationError::ExistingDocument {
        validation_error: ValidationError::NoMatch {
            value: ConvexValue::Null,
            validator: Validator::Boolean,
            context: ValidationContext::new(),
        },
        table_name: TableName::from_str("table")?,
        id: DocumentIdV6::min(),
    };
    model.mark_failed(failed_id, schema_error.clone()).await?;
    let schema_error_string = schema_error.to_string();
    let error = model.mark_validated(failed_id).await.unwrap_err();
    assert!(error.is_bad_request());
    assert_eq!(error.short_msg(), "SchemaAlreadyFailed");
    assert!(error.msg().contains(&schema_error_string));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_mark_schema_as_active(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt.clone()).await;
    let mut tx = db.begin(Identity::system()).await?;
    let mut model = SchemaModel::new(&mut tx);

    let db_schema = DatabaseSchema::default();
    let (id, state) = model.submit_pending(db_schema.clone()).await?;
    assert_eq!(state, SchemaState::Pending);
    model.mark_validated(id).await?;
    model.mark_active(id).await?;
    let schema_metadata: SchemaMetadata = tx
        .get(id)
        .await?
        .unwrap()
        .into_value()
        .into_value()
        .try_into()?;
    assert_eq!(schema_metadata.state, SchemaState::Active);

    // Ok to mark as active twice
    let mut model = SchemaModel::new(&mut tx);
    model.mark_active(id).await?;

    // Add another schema and make sure the old one is overwritten.
    let new_db_schema = db_schema!("new_table" => DocumentSchema::Any);
    let (new_id, state) = model.submit_pending(new_db_schema).await?;
    assert_eq!(state, SchemaState::Pending);
    model.mark_validated(new_id).await?;
    model.mark_active(new_id).await?;
    let schema_metadata: SchemaMetadata = tx
        .get(id)
        .await?
        .unwrap()
        .into_value()
        .into_value()
        .try_into()?;
    assert_eq!(schema_metadata.state, SchemaState::Overwritten);
    let mut model = SchemaModel::new(&mut tx);
    let err = model.mark_active(id).await.unwrap_err();
    assert!(err.is_bad_request());
    assert_eq!(err.short_msg(), "SchemaAlreadyOverwritten");
    let schema_metadata: SchemaMetadata = tx
        .get(new_id)
        .await?
        .unwrap()
        .into_value()
        .into_value()
        .try_into()?;
    assert_eq!(schema_metadata.state, SchemaState::Active);

    // Check that failed schemas can't be marked active
    let mut model = SchemaModel::new(&mut tx);
    let (failed_id, state) = model.submit_pending(db_schema).await?;
    assert_eq!(state, SchemaState::Pending);
    let schema_error = SchemaValidationError::ExistingDocument {
        validation_error: ValidationError::NoMatch {
            value: ConvexValue::Null,
            validator: Validator::Boolean,
            context: ValidationContext::new(),
        },
        table_name: TableName::from_str("table")?,
        id: DocumentIdV6::min(),
    };
    model.mark_failed(failed_id, schema_error.clone()).await?;
    let schema_error_string = schema_error.to_string();
    let err = model.mark_active(failed_id).await.unwrap_err();
    assert!(err.is_bad_request());
    assert_eq!(err.short_msg(), "SchemaAlreadyFailed");
    assert!(err.msg().contains(&schema_error_string));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_mark_schema_as_failed(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt.clone()).await;
    let mut tx = db.begin(Identity::system()).await?;
    let mut model = SchemaModel::new(&mut tx);

    let db_schema = DatabaseSchema::default();
    let (id, state) = model.submit_pending(db_schema.clone()).await?;
    assert_eq!(state, SchemaState::Pending);
    let schema_error = SchemaValidationError::ExistingDocument {
        validation_error: ValidationError::NoMatch {
            value: ConvexValue::Null,
            validator: Validator::Boolean,
            context: ValidationContext::new(),
        },
        table_name: TableName::from_str("table")?,
        id: DocumentIdV6::min(),
    };
    model.mark_failed(id, schema_error.clone()).await?;

    // Failed schemas can still be marked failed
    model.mark_failed(id, schema_error.clone()).await?;

    // Active schema cannot be marked failed
    let (id, state) = model.submit_pending(db_schema).await?;
    assert_eq!(state, SchemaState::Pending);
    model.mark_validated(id).await?;
    model.mark_active(id).await?;
    assert_eq!(
        model
            .mark_failed(id, schema_error.clone())
            .await
            .unwrap_err()
            .to_string(),
        String::from("Active schemas cannot be marked as failed.")
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_schema_enforced_on_write(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt.clone()).await;
    let mut tx = db.begin(Identity::system()).await?;
    let mut model = SchemaModel::new(&mut tx);

    let table = "table".parse::<TableName>()?;
    let object_validator = object_validator!("name" => FieldValidator::required_field_type(Validator::String), "age" => FieldValidator::required_field_type(Validator::Int64));
    let document_schema = DocumentSchema::Union(vec![object_validator]);
    let db_schema = db_schema!(table.clone() => document_schema);
    let (schema_id, _state) = model.submit_pending(db_schema.clone()).await?;
    model.mark_validated(schema_id).await?;
    model.mark_active(schema_id).await?;

    // Inserting a document that matches the schema should succeed
    let object = assert_obj!("name" => "emma", "age" => 24);
    let id = UserFacingModel::new(&mut tx)
        .insert(table.clone(), object)
        .await?;

    // Replacing a document that matches the schema should succeed
    let object = assert_obj!("name" => "lee", "age" => 24);
    UserFacingModel::new(&mut tx).replace(id, object).await?;

    // Updating a document that matches the schema should succeed
    let object = assert_obj!("name" => "alex", "age" => 24);
    UserFacingModel::new(&mut tx)
        .patch(id, object.into())
        .await?;

    // Inserting a document that does not match the schema should fail
    let bad_object = assert_obj!("name" => "emma", "age" => "24");
    let err = UserFacingModel::new(&mut tx)
        .insert(table, bad_object.clone())
        .await
        .unwrap_err();
    assert_eq!(err.short_msg(), "SchemaEnforcementError");

    // Replacing a document that does not match the schema should fail
    let err = UserFacingModel::new(&mut tx)
        .replace(id, bad_object.clone())
        .await
        .unwrap_err();
    assert_eq!(err.short_msg(), "SchemaEnforcementError");

    // Updating a document that does not match the schema should fail
    let err = UserFacingModel::new(&mut tx)
        .patch(id, bad_object.into())
        .await
        .unwrap_err();
    assert_eq!(err.short_msg(), "SchemaEnforcementError");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_schema_failed_after_bad_insert(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt.clone()).await;
    let mut tx = db.begin(Identity::system()).await?;
    let mut model = SchemaModel::new(&mut tx);

    let table = "table".parse::<TableName>()?;
    let object_validator = object_validator!("name" => FieldValidator::required_field_type(Validator::String), "age" => FieldValidator::required_field_type(Validator::Int64));
    let document_schema = DocumentSchema::Union(vec![object_validator]);
    let db_schema = db_schema!(table.clone() => document_schema);
    let (schema_id, _state) = model.submit_pending(db_schema.clone()).await?;

    // Inserting a document that matches the schema should succeed
    let object = assert_obj!("name" => "emma", "age" => 24);
    let id = UserFacingModel::new(&mut tx)
        .insert(table.clone(), object)
        .await?;

    // Replacing a document that matches the schema should succeed
    let object = assert_obj!("name" => "lee", "age" => 24);
    UserFacingModel::new(&mut tx).replace(id, object).await?;

    // Updating a document that matches the schema should succeed
    let object = assert_obj!("name" => "alex", "age" => 24);
    UserFacingModel::new(&mut tx)
        .patch(id, object.into())
        .await?;

    // Inserting a document that does not match the schema should fail
    let bad_object = assert_obj!("name" => "emma", "age" => "24");
    UserFacingModel::new(&mut tx)
        .insert(table.clone(), bad_object.clone())
        .await?;
    let SchemaMetadata { state, schema: _ } = tx
        .get(schema_id)
        .await?
        .unwrap()
        .into_value()
        .into_value()
        .try_into()?;
    assert!(matches!(state, SchemaState::Failed { .. }));

    // Replacing a document that does not match the schema should mark the schema as
    // failed and succeed
    let mut model = SchemaModel::new(&mut tx);
    let (schema_id, _state) = model.submit_pending(db_schema.clone()).await?;
    UserFacingModel::new(&mut tx)
        .replace(id, bad_object.clone())
        .await?;
    let SchemaMetadata { state, schema: _ } = tx
        .get(schema_id)
        .await?
        .unwrap()
        .into_value()
        .into_value()
        .try_into()?;
    assert!(matches!(state, SchemaState::Failed { .. }));

    // Updating a document that does not match the schema should mark the schema as
    // failed and succeed
    let mut model = SchemaModel::new(&mut tx);
    let (schema_id, _state) = model.submit_pending(db_schema.clone()).await?;
    UserFacingModel::new(&mut tx)
        .patch(id, bad_object.into())
        .await?;
    let SchemaMetadata { state, schema: _ } = tx
        .get(schema_id)
        .await?
        .unwrap()
        .into_value()
        .into_value()
        .try_into()?;
    assert!(matches!(state, SchemaState::Failed { .. }));
    Ok(())
}

#[convex_macro::test_runtime]
async fn overwrite_schema_by_state_with_no_schemas_does_nothing(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut tx = new_tx(rt).await?;
    let mut model = SchemaModel::new(&mut tx);

    model.overwrite_all().await?;

    let mut schemas = vec![];
    for state in [
        SchemaState::Active,
        SchemaState::Pending,
        SchemaState::Validated,
    ] {
        schemas.push(model.get_by_state(state).await?);
    }

    assert_eq!(schemas, vec![None, None, None]);
    Ok(())
}

async fn insert_new_pending_schema_of_table(
    tx: &mut Transaction<TestRuntime>,
    name: &str,
) -> anyhow::Result<ResolvedDocumentId> {
    let mut model = SchemaModel::new(tx);
    let table = name.parse::<TableName>()?;
    let object_validator = object_validator!("name" => FieldValidator::required_field_type(Validator::String), "age" => FieldValidator::required_field_type(Validator::Int64));
    let document_schema = DocumentSchema::Union(vec![object_validator]);
    let db_schema = db_schema!(table.clone() => document_schema);
    let (schema_id, _state) = model.submit_pending(db_schema.clone()).await?;
    Ok(schema_id)
}

async fn insert_new_pending_schema(
    tx: &mut Transaction<TestRuntime>,
) -> anyhow::Result<ResolvedDocumentId> {
    insert_new_pending_schema_of_table(tx, "default_table").await
}

async fn query_schema_metadata(
    tx: &mut Transaction<TestRuntime>,
    schema_id: ResolvedDocumentId,
) -> anyhow::Result<SchemaMetadata> {
    tx.get(schema_id)
        .await?
        .unwrap()
        .into_value()
        .into_value()
        .try_into()
}

async fn mark_schema_as_failed(
    tx: &mut Transaction<TestRuntime>,
    schema_id: &ResolvedDocumentId,
) -> anyhow::Result<()> {
    let mut model = SchemaModel::new(tx);
    let schema_error = SchemaValidationError::ExistingDocument {
        validation_error: ValidationError::NoMatch {
            value: ConvexValue::Null,
            validator: Validator::Boolean,
            context: ValidationContext::new(),
        },
        table_name: TableName::from_str("table")?,
        id: DocumentIdV6::min(),
    };
    model.mark_failed(*schema_id, schema_error.clone()).await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn overwrite_schema_by_state_with_active_schema_returns_true(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut tx = new_tx(rt).await?;
    let schema_id = insert_new_pending_schema(&mut tx).await?;
    let mut model = SchemaModel::new(&mut tx);
    model.mark_validated(schema_id).await?;
    model.mark_active(schema_id).await?;
    assert!(model.overwrite_all().await?);
    Ok(())
}

#[convex_macro::test_runtime]
async fn overwrite_schema_by_state_with_active_schema_sets_active_to_overwritten(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut tx = new_tx(rt).await?;
    let schema_id = insert_new_pending_schema(&mut tx).await?;
    let mut model = SchemaModel::new(&mut tx);
    model.mark_validated(schema_id).await?;
    model.mark_active(schema_id).await?;
    model.overwrite_all().await?;

    let schema_metadata = query_schema_metadata(&mut tx, schema_id).await?;

    assert_eq!(schema_metadata.state, SchemaState::Overwritten);

    Ok(())
}

#[convex_macro::test_runtime]
async fn overwrite_schema_by_state_with_validated_schema_sets_state_to_overwritten(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut tx = new_tx(rt).await?;
    let schema_id = insert_new_pending_schema(&mut tx).await?;
    let mut model = SchemaModel::new(&mut tx);
    model.mark_validated(schema_id).await?;
    model.overwrite_all().await?;

    let schema_metadata = query_schema_metadata(&mut tx, schema_id).await?;

    assert_eq!(schema_metadata.state, SchemaState::Overwritten);

    Ok(())
}

#[convex_macro::test_runtime]
async fn overwrite_schema_by_state_with_pending_schema_sets_state_to_overwritten(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut tx = new_tx(rt).await?;
    let schema_id = insert_new_pending_schema(&mut tx).await?;
    let mut model = SchemaModel::new(&mut tx);
    model.overwrite_all().await?;
    let schema_metadata = query_schema_metadata(&mut tx, schema_id).await?;

    assert_eq!(schema_metadata.state, SchemaState::Overwritten);
    Ok(())
}

#[convex_macro::test_runtime]
async fn overwrite_schema_by_state_with_failed_schema_returns_false(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut tx = new_tx(rt).await?;
    let schema_id = insert_new_pending_schema(&mut tx).await?;
    mark_schema_as_failed(&mut tx, &schema_id).await?;
    let mut model = SchemaModel::new(&mut tx);
    assert!(!(model.overwrite_all().await?));
    Ok(())
}

#[convex_macro::test_runtime]
async fn overwrite_schema_by_state_with_failed_schema_does_not_overwrite_schema(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut tx = new_tx(rt).await?;
    let schema_id = insert_new_pending_schema(&mut tx).await?;
    mark_schema_as_failed(&mut tx, &schema_id).await?;
    let mut model = SchemaModel::new(&mut tx);
    model.overwrite_all().await?;
    let schema_metadata = query_schema_metadata(&mut tx, schema_id).await?;

    assert!(matches!(schema_metadata.state, SchemaState::Failed { .. }));
    Ok(())
}

// Verify we don't do anything dumb (like throw, or ignore an active schema) if
// there's an existing already overwritten schema.
#[convex_macro::test_runtime]
async fn overwrite_schema_by_state_with_overwritten_and_active_schema_overrides_active_schema(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut tx = new_tx(rt).await?;
    let first_schema_id = insert_new_pending_schema_of_table(&mut tx, "first_table").await?;
    let mut model = SchemaModel::new(&mut tx);
    model.mark_validated(first_schema_id).await?;
    model.mark_active(first_schema_id).await?;

    // Activate a second schema so that the first is marked as overwritten.
    let second_schema_id = insert_new_pending_schema_of_table(&mut tx, "other_table").await?;
    let mut model = SchemaModel::new(&mut tx);
    model.mark_validated(second_schema_id).await?;
    model.mark_active(second_schema_id).await?;

    let first_schema_metadata = query_schema_metadata(&mut tx, first_schema_id).await?;
    // Precondition - ensure our setup actually overwrote the first schema.
    assert_eq!(first_schema_metadata.state, SchemaState::Overwritten);

    // Verify we neither changed the overwritten schema state nor crashed when
    // processing.
    let mut model = SchemaModel::new(&mut tx);
    model.overwrite_all().await?;
    let first_schema_metadata = query_schema_metadata(&mut tx, first_schema_id).await?;
    let second_schema_metadata = query_schema_metadata(&mut tx, second_schema_id).await?;
    assert_eq!(first_schema_metadata.state, SchemaState::Overwritten);
    assert_eq!(second_schema_metadata.state, SchemaState::Overwritten);
    Ok(())
}

#[convex_macro::test_runtime]
async fn overwrite_schema_by_state_with_overwritten_and_active_schema_returns_true(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut tx = new_tx(rt).await?;
    let first_schema_id = insert_new_pending_schema_of_table(&mut tx, "first_table").await?;
    let mut model = SchemaModel::new(&mut tx);
    model.mark_validated(first_schema_id).await?;
    model.mark_active(first_schema_id).await?;

    // Activate a second schema so that the first is marked as overwritten.
    let second_schema_id = insert_new_pending_schema_of_table(&mut tx, "other_table").await?;
    let mut model = SchemaModel::new(&mut tx);
    model.mark_validated(second_schema_id).await?;
    model.mark_active(second_schema_id).await?;

    // We should override second_schema because it should be active.
    assert!(model.overwrite_all().await?);
    Ok(())
}

#[convex_macro::test_runtime]
async fn overwrite_schema_deletes_old_overwritten_schemas(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt.clone()).await;
    let mut tx = db.begin_system().await?;
    let first_schema_id = insert_new_pending_schema_of_table(&mut tx, "first_table").await?;
    assert!(tx.get(first_schema_id).await?.is_some());
    db.commit(tx).await?;

    rt.wait(MAX_TIME_TO_KEEP_FAILED_AND_OVERWRITTEN_SCHEMAS * 2)
        .await;
    let mut tx = db.begin_system().await?;
    insert_new_pending_schema_of_table(&mut tx, "second_table").await?;
    assert!(tx.get(first_schema_id).await?.is_none());

    Ok(())
}
#[convex_macro::test_runtime]
async fn failed_schema_deletes_old_failed_schema(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt.clone()).await;
    let mut tx = db.begin_system().await?;
    let first_schema_id = insert_new_pending_schema_of_table(&mut tx, "first_table").await?;
    assert!(tx.get(first_schema_id).await?.is_some());
    mark_schema_as_failed(&mut tx, &first_schema_id).await?;
    let second_schema_id = insert_new_pending_schema_of_table(&mut tx, "second_table").await?;
    db.commit(tx).await?;

    rt.wait(MAX_TIME_TO_KEEP_FAILED_AND_OVERWRITTEN_SCHEMAS * 2)
        .await;
    let mut tx = db.begin_system().await?;
    mark_schema_as_failed(&mut tx, &second_schema_id).await?;
    assert!(tx.get(first_schema_id).await?.is_none());
    db.commit(tx).await?;

    Ok(())
}
