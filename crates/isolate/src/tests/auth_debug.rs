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
use crate::test_helpers::UdfTest;

#[convex_macro::test_runtime]
async fn test_get_user_identity_debug_with_plaintext_user(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t| {
        // Test that getUserIdentityDebug works with regular user identity
        let identity = Identity::user(UserIdentity::test());
        let (result, outcome) = t
            .query_outcome("auth:getUserIdentityDebug", assert_obj!(), identity.clone())
            .await?;
        
        // Should return the user identity, not an error
        must_let!(let ConvexValue::Object(obj) = result);
        assert!(obj.get("name").is_some());
        assert!(outcome.observed_identity);
        
        // Test with PlaintextUser identity - should return null (no JWT to debug)
        let plaintext_identity = Identity::PlaintextUser("test-plaintext-token".to_string());
        let (result, outcome) = t
            .query_outcome("auth:getUserIdentityDebug", assert_obj!(), plaintext_identity)
            .await?;
        
        assert_eq!(result, ConvexValue::Null);
        assert!(outcome.observed_identity);
        
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_get_user_identity_insecure_with_different_identities(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t| {
        // Test with PlaintextUser identity - should return the plaintext token
        let plaintext_token = "my-test-plaintext-token-12345";
        let plaintext_identity = Identity::PlaintextUser(plaintext_token.to_string());
        let (result, outcome) = t
            .query_outcome("auth:getUserIdentityInsecure", assert_obj!(), plaintext_identity)
            .await?;
        
        must_let!(let ConvexValue::String(token) = result);
        assert_eq!(&*token, plaintext_token);
        assert!(outcome.observed_identity == false);
        
        // Test with regular User identity - should return null
        let user_identity = Identity::user(UserIdentity::test());
        let (result, outcome) = t
            .query_outcome("auth:getUserIdentityInsecure", assert_obj!(), user_identity)
            .await?;
        
        assert_eq!(result, ConvexValue::Null);
        assert!(outcome.observed_identity == false);
        
        // Test with System identity - should return null
        let system_identity = Identity::system();
        let (result, outcome) = t
            .query_outcome("auth:getUserIdentityInsecure", assert_obj!(), system_identity)
            .await?;
        
        assert_eq!(result, ConvexValue::Null);
        assert!(outcome.observed_identity == false);
        
        // Test with Admin identity - should return null
        let admin_identity = Identity::InstanceAdmin(AdminIdentity::new_for_test_only(
            "test-admin-key".to_string(),
            MemberId(1),
        ));
        let (result, outcome) = t
            .query_outcome("auth:getUserIdentityInsecure", assert_obj!(), admin_identity)
            .await?;
        
        assert_eq!(result, ConvexValue::Null);
        assert!(outcome.observed_identity == false);
        
        // Test with Unknown identity - should return null
        let unknown_identity = Identity::Unknown(None);
        let (result, outcome) = t
            .query_outcome("auth:getUserIdentityInsecure", assert_obj!(), unknown_identity)
            .await?;
        
        assert_eq!(result, ConvexValue::Null);
        assert!(outcome.observed_identity == false);
        
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_plaintext_user_admin_access_restriction(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t| {
        // Test that PlaintextUser identity cannot access admin-protected functions
        let plaintext_identity = Identity::PlaintextUser("admin-wannabe-token".to_string());
        
        // This test would verify that PlaintextUser identities are properly rejected
        // by the must_be_admin_internal function changes
        let (outcome, _token) = t
            .raw_query("auth:testAdminAccess", vec![ConvexValue::Object(assert_obj!())], plaintext_identity, None)
            .await?;
        
        // Should fail with admin access error
        assert!(outcome.result.is_err());
        let error = outcome.result.unwrap_err();
        let error_str = error.to_string();
        assert!(error_str.contains("BadDeployKey") || error_str.contains("invalid"));
        
        // Compare with regular admin identity which should succeed
        let admin_identity = Identity::InstanceAdmin(AdminIdentity::new_for_test_only(
            "valid-admin-key".to_string(),
            MemberId(1),
        ));
        
        // This should succeed for admin identities
        let (admin_outcome, _token) = t
            .raw_query("auth:testAdminAccess", vec![ConvexValue::Object(assert_obj!())], admin_identity, None)
            .await?;
            
        // Admin should have access
        assert!(admin_outcome.result.is_ok());
        
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_plaintext_user_identity_creation_and_handling(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t| {
        let test_token = "test-plaintext-auth-token-xyz";
        let plaintext_identity = Identity::PlaintextUser(test_token.to_string());
        
        // Test that PlaintextUser identity is properly handled in queries
        let (result, outcome) = t
            .query_outcome("auth:getIdentityType", assert_obj!(), plaintext_identity.clone())
            .await?;
        
        // Should indicate it's a PlaintextUser identity
        must_let!(let ConvexValue::String(identity_type) = result);
        assert_eq!(&*identity_type, "PlaintextUser");
        assert!(outcome.observed_identity);
        
        // Test that getUserIdentityInsecure returns the correct token
        let (token_result, _) = t
            .query_outcome("auth:getUserIdentityInsecure", assert_obj!(), plaintext_identity)
            .await?;
        
        must_let!(let ConvexValue::String(returned_token) = token_result);
        assert_eq!(&*returned_token, test_token);
        
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_get_user_identity_debug_error_scenarios(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t| {
        // Test getUserIdentityDebug with Unknown identity containing error
        let error_message = "JWT validation failed: token expired";
        let unknown_identity_with_error = Identity::Unknown(Some(
            errors::ErrorMetadata::bad_request("InvalidJWT", error_message)
        ));
        
        let (result, outcome) = t
            .query_outcome("auth:getUserIdentityDebug", assert_obj!(), unknown_identity_with_error)
            .await?;
        
        // Should return structured error information
        must_let!(let ConvexValue::Object(error_obj) = result);
        assert!(error_obj.get("error").is_some());
        must_let!(let ConvexValue::Object(error_obj_inner) = error_obj.get("error").unwrap());
        assert!(error_obj_inner.get("code").is_some());
        assert!(error_obj_inner.get("message").is_some());
        assert!(outcome.observed_identity);
        
        // Test with Unknown identity without error - should return null
        let unknown_identity = Identity::Unknown(None);
        let (result, outcome) = t
            .query_outcome("auth:getUserIdentityDebug", assert_obj!(), unknown_identity)
            .await?;
        
        assert_eq!(result, ConvexValue::Null);
        assert!(outcome.observed_identity);
        
        Ok(())
    })
    .await
}
