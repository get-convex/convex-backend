use cmd_util::env::env_config;
use common::{
    assert_obj,
    runtime::testing::TestRuntime,
    value::ConvexValue,
};
use must_let::must_let;
use proptest::prelude::*;
use runtime::testing::TestDriver;
use value::{
    base32,
    id_v6::DeveloperDocumentId,
};

use crate::test_helpers::UdfTest;

async fn test_idv6_js_decode(rt: TestRuntime, id: DeveloperDocumentId) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    must_let!(let ConvexValue::Object(obj) = t.query("idEncoding:decode", assert_obj!("id" => id.encode())).await?);
    must_let!(let Some(ConvexValue::Float64(ref table_number)) = obj.get("tableNumber"));
    must_let!(let Some(ConvexValue::Bytes(ref internal_id)) = obj.get("internalId"));
    assert_eq!(*table_number, u32::from(*id.table()) as f64);
    assert_eq!(&internal_id[..], &id.internal_id()[..]);
    Ok(())
}

async fn test_idv6_js_is_id(rt: TestRuntime, id: String) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    must_let!(let ConvexValue::Object(obj) = t.query("idEncoding:isId", assert_obj!("id" => id.clone())).await?);
    must_let!(let Some(ConvexValue::Boolean(is_id)) = obj.get("result"));
    assert_eq!(&DeveloperDocumentId::decode(&id).is_ok(), is_id);
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

    #[test]
    fn proptest_idv6_js_decode(id in any::<DeveloperDocumentId>()) {
        let td = TestDriver::new();
        let rt = td.rt();
        td.run_until(test_idv6_js_decode(rt, id)).unwrap();
    }

    #[test]
    fn proptest_idv6_js_is_id_bytes(bytes in prop::collection::vec(any::<u8>(), 19..=23)) {
        // Generate bytestrings that pass the first few checks in decode to get more code
        // coverage for later errors.
        let td = TestDriver::new();
        let rt = td.rt();
        let id_str = base32::encode(&bytes);
        td.run_until(test_idv6_js_is_id(rt, id_str)).unwrap();
    }

    #[test]
    fn proptest_idv6_js_is_id(id in any::<String>()) {
        let td = TestDriver::new();
        let rt = td.rt();
        td.run_until(test_idv6_js_is_id(rt, id)).unwrap();
    }
}
