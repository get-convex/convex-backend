use common::{
    assert_obj,
    types::MemberId,
    value::ConvexValue,
};
use keybroker::{
    testing::TestUserIdentity,
    AdminIdentity,
    Identity,
    UserIdentity,
};
use must_let::must_let;
use runtime::testing::TestRuntime;
use sync_types::UserIdentityAttributes;

use crate::test_helpers::UdfTest;

#[convex_macro::test_runtime]
async fn test_auth_basic(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    // UDF with no identity should return `null`
    must_let!(let ConvexValue::Null = t.query("auth:getName", assert_obj!()).await?);
    // With an identity, it should return the user's name
    let identity = Identity::user(UserIdentity::test());
    must_let!(let ConvexValue::String(s) = t.query_with_identity("auth:getName", assert_obj!(), identity.clone()).await?);
    assert_eq!(*s, UserIdentity::test().attributes.name.unwrap());
    must_let!(let ConvexValue::String(s) = t.query_with_identity("auth:getIdentifier", assert_obj!(), identity).await?);
    assert_eq!(&*s, &*UserIdentity::test().attributes.token_identifier);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_auth_identity_for_acting_user(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    // UDF with no identity should return `null`
    must_let!(let ConvexValue::Null = t.query("auth:getName", assert_obj!()).await?);
    // With an identity, it should return the user's name
    let identity = Identity::ActingUser(
        AdminIdentity::new_for_test_only("chocolate-charlie-420".to_string(), MemberId(0)),
        UserIdentityAttributes::test(),
    );
    must_let!(let ConvexValue::String(s) = t.query_with_identity("auth:getName", assert_obj!(), identity.clone()).await?);
    assert_eq!(*s, UserIdentityAttributes::test().name.unwrap());
    must_let!(let ConvexValue::String(s) = t.query_with_identity("auth:getIdentifier", assert_obj!(), identity).await?);
    assert_eq!(&*s, &*UserIdentityAttributes::test().token_identifier);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_auth_identity_for_admin(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    // UDF with no identity should return `null`
    must_let!(let ConvexValue::Null = t.query("auth:getName", assert_obj!()).await?);
    // With an identity, it should return the user's name
    let identity = Identity::InstanceAdmin(AdminIdentity::new_for_test_only(
        "bozotown".to_string(),
        MemberId(77),
    ));
    must_let!(let ConvexValue::Null = t.query_with_identity("auth:getName", assert_obj!(), identity.clone()).await?);
    Ok(())
}
