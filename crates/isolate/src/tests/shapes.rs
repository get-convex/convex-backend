use common::{
    assert_obj,
    document::InternalId,
    shapes::{
        dashboard_shape_json,
        reduced::ReducedShape,
    },
    testing::TestIdGenerator,
    value::ConvexValue,
};
use maplit::{
    btreemap,
    btreeset,
};
use must_let::must_let;
use runtime::testing::TestRuntime;
use shape_inference::{
    testing::TestConfig,
    CountedShape,
};
use value::{
    assert_val,
    id_v6::DeveloperDocumentId,
    val,
    ResolvedDocumentId,
    TableNamespace,
};

use crate::test_helpers::UdfTest;

#[convex_macro::test_runtime]
async fn test_shape_inference_js(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let mut id_generator = TestIdGenerator::new();
    let table_id = id_generator.user_table_id(&"test".parse()?);
    let table_number = id_generator.generate_virtual_table(&"test2".parse()?);
    let table_mapping = id_generator.namespace(TableNamespace::test_user());
    let virtual_table_mapping = id_generator
        .virtual_table_mapping
        .namespace(TableNamespace::test_user());
    let values: Vec<(ConvexValue, &'static str)> = vec![
        (
            ConvexValue::from(ResolvedDocumentId::new(
                table_id.tablet_id,
                DeveloperDocumentId::new(table_id.table_number, InternalId::MIN),
            )),
            r#"Id<"test">"#,
        ),
        (
            ConvexValue::from(DeveloperDocumentId::new(table_number, InternalId::MIN)),
            r#"Id<"test2">"#,
        ),
        (val!(null), "null"),
        (val!(0), "bigint"),
        (val!(0.), "number"),
        (val!(true), "boolean"),
        (val!(""), "string"),
        (val!(Vec::<u8>::new()), "ArrayBuffer"),
        (val!(vec![ConvexValue::Null]), "Array<null>"),
        (
            ConvexValue::Set(btreeset!(ConvexValue::Null).try_into()?),
            "Set<null>",
        ),
        (
            ConvexValue::Map(btreemap!(ConvexValue::Null => ConvexValue::Null).try_into()?),
            "Map<null,null>",
        ),
        (assert_val!({"a" => 0, "b" => 0.}), "{a: bigint,b: number}"),
    ];
    for (value, expected) in values {
        let shape = ReducedShape::from_type(
            &CountedShape::<TestConfig>::empty().insert_value(&value),
            &table_mapping.table_number_exists(),
            &virtual_table_mapping.table_number_exists(),
        );
        let shape_json = dashboard_shape_json(&shape, &table_mapping, &virtual_table_mapping)?;
        must_let!(let ConvexValue::String(s) = t.query("shapes", assert_obj!("shapeJson" => serde_json::to_string(&shape_json)?)).await?);
        assert_eq!(&s[..], expected);
    }

    // // Try a union shape.
    let shape = ReducedShape::from_type(
        &CountedShape::<TestConfig>::empty()
            .insert_value(&ConvexValue::Null)
            .insert_value(&ConvexValue::from(0)),
        &table_mapping.table_number_exists(),
        &virtual_table_mapping.table_number_exists(),
    );
    let shape_json = dashboard_shape_json(&shape, &table_mapping, &virtual_table_mapping)?;
    must_let!(let ConvexValue::String(s) = t.query("shapes", assert_obj!("shapeJson" => serde_json::to_string(&shape_json)?)).await?);
    assert_eq!(&s[..], "null|bigint");

    // Try the `never` shape.
    let shape = ReducedShape::from_type(
        &CountedShape::<TestConfig>::empty(),
        &table_mapping.table_number_exists(),
        &virtual_table_mapping.table_number_exists(),
    );
    let shape_json = dashboard_shape_json(&shape, &table_mapping, &virtual_table_mapping)?;
    must_let!(let ConvexValue::String(s) = t.query("shapes", assert_obj!("shapeJson" => serde_json::to_string(&shape_json)?)).await?);
    assert_eq!(&s[..], "never");
    Ok(())
}
