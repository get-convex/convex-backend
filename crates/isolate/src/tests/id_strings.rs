use std::str::FromStr;

use cmd_util::env::env_config;
use common::version::Version;
use database::{
    TestFacingModel,
    UserFacingModel,
};
use keybroker::Identity;
use model::udf_config::{
    types::UdfConfig,
    UdfConfigModel,
};
use must_let::must_let;
use proptest::prelude::*;
use runtime::testing::{
    TestDriver,
    TestRuntime,
};
use value::{
    assert_obj,
    id_v6::DocumentIdV6,
    ConvexValue,
    FieldName,
    InternalId,
    TableName,
};

use crate::test_helpers::UdfTest;

#[convex_macro::test_runtime]
async fn test_table_mapping_from_system_udf(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let mut tx = t.database.begin(Identity::system()).await?;
    let document = TestFacingModel::new(&mut tx)
        .insert_and_get("table".parse()?, assert_obj!())
        .await?;
    let table_number = document.id().table().table_number;
    let table_number_field: FieldName = FieldName::from_str(table_number.to_string().as_ref())?;
    t.database.commit(tx).await?;

    let value = t.query("idStrings:getTableMapping", assert_obj!()).await?;
    must_let!(let ConvexValue::Object(entries) = value);

    assert_eq!(1, entries.len());
    assert_eq!(
        &ConvexValue::String("table".try_into()?),
        entries.get(&table_number_field).unwrap()
    );

    Ok(())
}

async fn test_normalize_id(rt: TestRuntime, internal_id: InternalId) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    // Initialize table mapping with two tables
    let mut tx = t.database.begin(Identity::system()).await?;
    let table_name_a: TableName = "boats".parse()?;
    let table_name_b: TableName = "votes".parse()?;
    UserFacingModel::new(&mut tx)
        .insert(table_name_a.clone(), assert_obj!())
        .await?;
    UserFacingModel::new(&mut tx)
        .insert(table_name_b.clone(), assert_obj!())
        .await?;
    let table_number = tx.table_mapping().id(&table_name_a)?.table_number;

    // Set the UDF server version to a version with string IDs
    UdfConfigModel::new(&mut tx)
        .set(UdfConfig::new_for_test(&t.rt, Version::parse("1000.0.0")?))
        .await?;
    t.database.commit(tx).await?;

    let id_v6 = DocumentIdV6::new(table_number, internal_id);

    // Test IDv6 and correct table name
    must_let!(let ConvexValue::Object(obj) = t.query("idStrings:normalizeId", assert_obj!(
        "id" => id_v6.encode(),
        "table" => table_name_a.to_string()
    )).await?);
    must_let!(let Some(ConvexValue::String(ref normalized_id)) = obj.get("normalized"));

    assert_eq!(normalized_id.to_string(), id_v6.encode());

    // Test internal ID and correct table name
    must_let!(let ConvexValue::Object(obj) = t.query("idStrings:normalizeId", assert_obj!("id" => internal_id.to_string(), "table" => table_name_a.to_string() )).await?);
    must_let!(let Some(ConvexValue::String(ref normalized_id)) = obj.get("normalized"));

    assert_eq!(normalized_id.to_string(), id_v6.encode());

    // Test IDv6 and incorrect table name
    must_let!(let ConvexValue::Object(obj) = t.query("idStrings:normalizeId", assert_obj!(
        "id" => id_v6.encode(),
        "table" => table_name_b.to_string()
    )).await?);
    must_let!(let Some(ConvexValue::Null) = obj.get("normalized"));

    // Test internal ID and incorrect table name
    must_let!(let ConvexValue::Object(obj) = t.query("idStrings:normalizeId", assert_obj!("id" => internal_id.to_string(), "table" => table_name_a.to_string() )).await?);
    must_let!(let Some(ConvexValue::String(_)) = obj.get("normalized"));

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_system_normalize_id(rt: TestRuntime) -> anyhow::Result<()> {
    let internal_id = InternalId::MIN;

    let t = UdfTest::default(rt).await?;
    let mut tx = t.database.begin(Identity::system()).await?;
    let user_table_name: TableName = "boats".parse()?;
    UserFacingModel::new(&mut tx)
        .insert(user_table_name.clone(), assert_obj!())
        .await?;
    let user_table_number = tx.table_mapping().id(&user_table_name)?.table_number;

    let storage_virtual_table_number = tx.virtual_table_mapping().number(&"_storage".parse()?)?;
    let storage_table_number = tx
        .table_mapping()
        .id(&"_file_storage".parse()?)?
        .table_number;
    let indexes_table_number = tx.table_mapping().id(&"_index".parse()?)?.table_number;

    // Set the UDF server version to a version with string IDs
    UdfConfigModel::new(&mut tx)
        .set(UdfConfig::new_for_test(&t.rt, Version::parse("1000.0.0")?))
        .await?;
    t.database.commit(tx).await?;

    let id_v6 = DocumentIdV6::new(storage_virtual_table_number, internal_id);

    // Correct virtual table name and number.
    must_let!(let ConvexValue::String(normalized_id) = t.query("idStrings:normalizeSystemId", assert_obj!(
        "id" => id_v6.encode(),
        "table" => "_storage",
    )).await?);
    assert_eq!(normalized_id.to_string(), id_v6.encode());

    // Correct virtual table name and internal id.
    must_let!(let ConvexValue::String(normalized_id) = t.query("idStrings:normalizeSystemId", assert_obj!(
        "id" => internal_id.to_string(),
        "table" => "_storage",
    )).await?);
    assert_eq!(normalized_id.to_string(), id_v6.encode());

    // Incorrect virtual table name.
    must_let!(let ConvexValue::Null = t.query("idStrings:normalizeSystemId", assert_obj!(
        "id" => id_v6.encode(),
        "table" => "_scheduled_functions",
    )).await?);

    // Physical table name and virtual table number doesn't work.
    must_let!(let ConvexValue::Null = t.query("idStrings:normalizeSystemId", assert_obj!(
        "id" => id_v6.encode(),
        "table" => "_file_storage",
    )).await?);

    // Virtual table name and physical table number doesn't work.
    must_let!(let ConvexValue::Null = t.query("idStrings:normalizeSystemId", assert_obj!(
        "id" => DocumentIdV6::new(storage_table_number, internal_id).encode(),
        "table" => "_storage",
    )).await?);

    // Physical table name and physical table number doesn't work.
    must_let!(let ConvexValue::Null = t.query("idStrings:normalizeSystemId", assert_obj!(
        "id" => DocumentIdV6::new(storage_table_number, internal_id).encode(),
        "table" => "_file_storage",
    )).await?);

    // System table that doesn't even have a virtual table doesn't work.
    must_let!(let ConvexValue::Null = t.query("idStrings:normalizeSystemId", assert_obj!(
        "id" => DocumentIdV6::new(indexes_table_number, internal_id).encode(),
        "table" => "_index",
    )).await?);

    // Virtual table with db.normalizeId throws error.
    t.query_js_error(
        "idStrings:normalizeId",
        assert_obj!(
            "id" => id_v6.encode(),
            "table" => "_storage",
        ),
    )
    .await?;

    // User table with db.system.normalizeId throws error.
    t.query_js_error(
        "idStrings:normalizeSystemId",
        assert_obj!(
            "id" => DocumentIdV6::new(user_table_number, internal_id).encode(),
            "table" => user_table_name.to_string(),
        ),
    )
    .await?;

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_virtual_id_query(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let scheduled_id = t.mutation("idStrings:schedule", assert_obj!()).await?;

    t.query(
        "idStrings:queryVirtualId",
        assert_obj!("id" => scheduled_id),
    )
    .await?;

    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

    #[test]
    fn proptest_normalize_id(id in any::<InternalId>()) {
        let mut td = TestDriver::new();
        let rt = td.rt();
        td.run_until(test_normalize_id(rt, id)).unwrap();
    }
}
