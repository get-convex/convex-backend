use std::str::FromStr;

use chrono::{
    TimeZone,
    Utc,
};
use openidconnect::core::CoreIdTokenVerifier;

use crate::{
    CoreIdTokenWithCustomClaims,
    UserIdentity,
};

#[test]
fn test_custom_claims() {
    let header_json = "{\"alg\":\"none\"}";
    let payload_json = "{
        \"iss\": \"https://server.example.com\",
        \"sub\": \"24400320\",
        \"aud\": [\"s6BhdRkqt3\"],
        \"exp\": 1311281970,
        \"iat\": 1311280970,
        \"tfa_method\": \"u2f\",
        \"bool_me\": true
    }";
    let signature = "foo";
    let token_str = [header_json, payload_json, signature]
        .map(|s| base64::encode_config(s, base64::URL_SAFE_NO_PAD))
        .join(".");
    let token = CoreIdTokenWithCustomClaims::from_str(&token_str).expect("failed to parse");
    let verifier = CoreIdTokenVerifier::new_insecure_without_verification()
        .set_time_fn(|| Utc.timestamp_opt(1311281969, 0).unwrap());
    let identity = UserIdentity::from_token(token, verifier).unwrap();
    let custom_claims = identity.attributes.custom_claims;
    assert_eq!(custom_claims.get("tfa_method").unwrap(), "\"u2f\"");
    assert_eq!(custom_claims.get("bool_me").unwrap(), "true");
}
