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

use crate::test_helpers::{
    UdfTest,
    UdfTestType,
};

#[convex_macro::test_runtime]
async fn test_auth_basic(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t| {
        // UDF with no identity should return `null`
        let (result, outcome) = t
            .query_outcome("auth:getName", assert_obj!(), Identity::system())
            .await?;
        assert_eq!(result, ConvexValue::Null);
        assert!(outcome.observed_identity);
        // With an identity, it should return the user's name
        let identity = Identity::user(UserIdentity::test());
        let (result, outcome) = t
            .query_outcome("auth:getName", assert_obj!(), identity.clone())
            .await?;
        must_let!(let ConvexValue::String(s) = result);
        assert_eq!(*s, UserIdentity::test().attributes.name.unwrap());
        assert!(outcome.observed_identity);
        let (result, outcome) = t
            .query_outcome("auth:getIdentifier", assert_obj!(), identity.clone())
            .await?;
        must_let!(let ConvexValue::String(s) = result);
        assert_eq!(&*s, &*UserIdentity::test().attributes.token_identifier);
        assert!(outcome.observed_identity);
        Ok(())
    })
    .await
}

async fn test_conditionally_observed_identity_inner(
    t: UdfTestType,
    subquery: bool,
) -> anyhow::Result<()> {
    let query = if subquery {
        "auth:conditionallyCheckAuthInSubquery"
    } else {
        "auth:conditionallyCheckAuth"
    };

    let identity = Identity::user(UserIdentity::test());
    let (_, outcome) = t
        .query_outcome(query, assert_obj!(), identity.clone())
        .await?;
    assert!(!outcome.observed_identity);
    // the function checks identity only after an object is inserted
    t.mutation("basic:insertObject", assert_obj!()).await?;
    let (_, outcome) = t
        .query_outcome(query, assert_obj!(), identity.clone())
        .await?;
    assert!(outcome.observed_identity);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_conditionally_observed_identity(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t| {
        test_conditionally_observed_identity_inner(t, false).await
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_conditionally_observed_identity_in_subquery(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate(rt, async move |t| {
        test_conditionally_observed_identity_inner(t, true).await
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_auth_identity_for_acting_user(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t| {
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
    }).await
}

#[convex_macro::test_runtime]
async fn test_auth_identity_for_admin(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t| {
        // UDF with no identity should return `null`
        must_let!(let ConvexValue::Null = t.query("auth:getName", assert_obj!()).await?);
        // With an identity, it should return the user's name
        let identity = Identity::InstanceAdmin(AdminIdentity::new_for_test_only(
            "bozotown".to_string(),
            MemberId(77),
        ));
        must_let!(let ConvexValue::Null = t.query_with_identity("auth:getName", assert_obj!(), identity.clone()).await?);
        Ok(())
    }).await
}
