use std::sync::LazyLock;

use chrono::{
    Duration,
    Utc,
};
use openidconnect::{
    core::{
        CoreIdToken,
        CoreIdTokenClaims,
        CoreIdTokenVerifier,
        CoreJwsSigningAlgorithm,
        CoreRsaPrivateSigningKey,
    },
    Audience,
    EmptyAdditionalClaims,
    EndUserEmail,
    EndUserName,
    IssuerUrl,
    JsonWebKeyId,
    StandardClaims,
    SubjectIdentifier,
};
use rsa::pkcs1::EncodeRsaPrivateKey;
use sync_types::UserIdentityAttributes;

use crate::UserIdentity;

pub static TEST_SIGNING_KEY: LazyLock<CoreRsaPrivateSigningKey> = LazyLock::new(|| {
    let key = rsa::RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 2048).unwrap();
    let pem = key.to_pkcs1_pem(rsa::pkcs8::LineEnding::LF).unwrap();
    CoreRsaPrivateSigningKey::from_pem(&pem, Some(JsonWebKeyId::new("key1".to_string()))).unwrap()
});

pub trait TestUserIdentity {
    fn test() -> Self;
}

impl TestUserIdentity for UserIdentity {
    fn test() -> Self {
        let subject = "testauth|123".to_owned();
        let issuer = "https://testauth.fake.domain".to_owned();
        let audience = Audience::new("client-id-123".to_string());

        let token = CoreIdToken::new(
            CoreIdTokenClaims::new(
                IssuerUrl::new(issuer).unwrap(),
                vec![audience],
                Utc::now() + Duration::seconds(600),
                Utc::now(),
                StandardClaims::new(SubjectIdentifier::new(subject))
                    .set_email(Some(EndUserEmail::new("foo@bar.com".to_string())))
                    .set_name(Some(EndUserName::new("Al Pastor".to_string()).into())),
                EmptyAdditionalClaims {},
            ),
            &*TEST_SIGNING_KEY,
            CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
            None,
            None,
        )
        .unwrap();

        let verifier = CoreIdTokenVerifier::new_insecure_without_verification();
        UserIdentity::from_token(token, verifier).unwrap()
    }
}

impl TestUserIdentity for UserIdentityAttributes {
    fn test() -> Self {
        UserIdentityAttributes {
            subject: Some("fake_user".to_string()),
            issuer: Some("convex".to_string()),
            name: Some("bozo".to_string()),
            email: Some("bozo@convex.dev".to_string()),
            ..Default::default()
        }
    }
}
