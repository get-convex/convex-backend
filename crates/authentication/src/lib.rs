use std::{
    str::FromStr,
    sync::LazyLock,
    time::SystemTime,
};

use anyhow::Context;
use biscuit::{
    jwk::JWKSet,
    ClaimPresenceOptions,
    Presence,
    TemporalOptions,
    Validation,
    ValidationOptions,
    JWT,
};
use chrono::TimeZone;
use common::auth::AuthInfo;
use errors::ErrorMetadata;
use futures::Future;
use keybroker::{
    CoreIdTokenWithCustomClaims,
    UserIdentity,
};
use oauth2::{
    HttpRequest,
    HttpResponse,
};
use openidconnect::{
    core::{
        CoreIdTokenVerifier,
        CoreJwsSigningAlgorithm,
        CoreProviderMetadata,
    },
    ClaimsVerificationError,
    ClientId,
    DiscoveryError,
};
use serde::{
    Deserialize,
    Serialize,
};
use sync_types::AuthenticationToken;
use url::Url;

pub mod access_token_auth;
pub mod application_auth;
pub mod metrics;

/// Issuer for API access tokens
pub static CONVEX_AUTH_URL: LazyLock<Url> =
    LazyLock::new(|| Url::parse("https://auth.convex.dev/").unwrap());
const CONFIG_URL_SUFFIX: &str = ".well-known/jwks.json";
/// Audience for API access tokens
///
/// This value was created long ago, and cannot be changed easily.
/// It's just a fixed string used for identifying the Auth0 token, so it's fine
/// and not user-facing. These API access tokens are constructed from multiple
/// clients (eg dashboard/cli)
pub const CONVEX_CONSOLE_API_AUDIENCE: &str = "https://console.convex.dev/api/";
// This is the client ID for the Auth0 application used for the dashboard.
// For... reasons, it's hard to get some clients to set the audience to the
// console one, so accept this one as well.
pub const ALTERNATE_CONVEX_API_AUDIENCE: &str = "nANKpAFe4scUPxW77869QHVKYAgrPwy7";

/// Extract the bearer token from an `Authorization: Bearer` header.
pub async fn extract_bearer_token(header: Option<String>) -> anyhow::Result<Option<String>> {
    let Some(header) = header else {
        return Ok(None);
    };
    if header.len() <= 7 || !header[..7].eq_ignore_ascii_case("bearer ") {
        anyhow::bail!(ErrorMetadata::unauthenticated(
            "InvalidAuthHeader",
            "Header must begin with `bearer `"
        ));
    }
    let token = header[7..].trim();
    Ok(Some(token.to_owned()))
}

pub fn token_to_authorization_header(token: AuthenticationToken) -> anyhow::Result<Option<String>> {
    match token {
        AuthenticationToken::Admin(key, user) => match user {
            Some(user) => {
                let encoded = base64::encode(
                    serde_json::to_vec(&serde_json::Value::try_from(user)?).map_err(|e| {
                        anyhow::anyhow!("Failed to serialize acting user attributes {e}")
                    })?,
                );
                Ok(Some(format!("Convex {}:{}", key, encoded)))
            },
            None => Ok(Some(format!("Convex {}", key))),
        },
        AuthenticationToken::User(key) => Ok(Some(format!("Bearer {}", key))),
        AuthenticationToken::None => Ok(None),
    }
}

/// Validate an OpenID Connect ID token.
pub async fn validate_id_token<F, E>(
    token_str: Auth0IdToken,
    // The http client is injected here so we can unit test this filter without needing to actually
    // serve an HTTP response from an identity provider.
    http_client: impl Fn(HttpRequest) -> F + 'static,
    auth_infos: Vec<AuthInfo>,
    system_time: SystemTime,
) -> anyhow::Result<UserIdentity>
where
    F: Future<Output = Result<HttpResponse, E>>,
    E: std::error::Error + 'static + Send + Sync,
{
    let token = CoreIdTokenWithCustomClaims::from_str(&token_str.0).context(
        ErrorMetadata::unauthenticated("InvalidAuthHeader", "Could not parse as id token"),
    )?;
    let (audiences, issuer) = {
        let verifier = CoreIdTokenVerifier::new_insecure_without_verification();
        let claims = match token.claims(&verifier, |_: Option<&openidconnect::Nonce>| Ok(())) {
            Ok(claims) => Ok(claims),
            Err(e @ ClaimsVerificationError::Expired(_)) => {
                let msg = e.to_string();
                Err(e).context(ErrorMetadata::unauthenticated("IdTokenExpired", msg))
            },
            e @ Err(_) => e.context("Token claim verification error"),
        }?;
        (
            claims
                .audiences()
                .iter()
                .map(|aud| aud.to_string())
                .collect::<Vec<_>>(),
            claims.issuer(),
        )
    };
    // Find the provider matching this token
    let auth_info = auth_infos
        .into_iter()
        .find(|info| {
            // Some authentication providers (Auth0, lookin' at you) tell developers that
            // their identity domain doesn't have a trailing slash, but the OIDC tokens do
            // have one in the `issuer` field. This is consistent with what the OIDC
            // Discovery response will contain, but the value entered in the instance config
            // may or may not have the slash.
            audiences.contains(&info.application_id)
                && info.domain.trim_end_matches('/') == issuer.trim_end_matches('/')
        })
        .context(ErrorMetadata::unauthenticated(
            "NoAuthProvider",
            "No auth provider found matching the given token",
        ))?;
    // Use the OpenID Connect Discovery protocol to get the public keys for this
    // provider.
    // TODO(CX-606): Add an caching layer that respects the HTTP cache headers
    // in the response.
    let metadata = CoreProviderMetadata::discover_async(issuer.clone(), &http_client)
        .await
        .map_err(|e| {
            let short = "AuthProviderDiscoveryFailed";
            let long = format!("Auth provider discovery of {} failed", issuer.as_str());
            match e {
                DiscoveryError::Response(code, body, _) => {
                    let long = format!("{long}: {} {}", code, String::from_utf8_lossy(&body));
                    let Ok(code) = http::StatusCode::from_u16(code.as_u16()) else {
                        return ErrorMetadata::bad_request(short, long);
                    };
                    if let Some(em) =
                        ErrorMetadata::from_http_status_code(code, short, long.clone())
                    {
                        em
                    } else {
                        ErrorMetadata::bad_request(short, long)
                    }
                },
                e => {
                    tracing::error!(
                        "Error discovering auth provider: {}, {}",
                        issuer.as_str(),
                        e
                    );
                    ErrorMetadata::bad_request(short, long)
                },
            }
        })?;
    // Create a verifier for the provider using this metadata. Set the verifier
    // to enforce that the issuer and audience match.
    // Note for posterity: this verifier will reject tokens containing multiple
    // audiences. It's very uncommon for an identity provider to create a token with
    // multiple valid audiences, so we don't handle that case yet.
    let verifier = CoreIdTokenVerifier::new_public_client(
        ClientId::new(auth_info.application_id),
        metadata.issuer().clone(),
        metadata.jwks().clone(),
    )
    .set_allowed_algs([
        // RS256, the most common algorithm and used by Clerk and Auth0 (by default)
        CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
        // EdDSA (this is only Ed25519)
        CoreJwsSigningAlgorithm::EdDsa,
    ])
    .require_issuer_match(true)
    .require_audience_match(true)
    .set_time_fn(|| {
        chrono::Utc
            .timestamp_opt(
                system_time
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("couldn't calculate unix timestamp?")
                    .as_secs() as i64,
                0,
            )
            .unwrap()
    });
    UserIdentity::from_token(token, verifier).context(ErrorMetadata::unauthenticated(
        "Unauthenticated",
        "Could not verify token claim",
    ))
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Auth0AccessToken(pub String);
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Auth0IdToken(pub String);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceToken(pub String);
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ConsoleClaims {
    // Auth0 requires that any custom claims overlapping with the OIDC spec be namespaced behind a
    // domain name.
    #[serde(rename = "https://convex.dev/email")]
    email: String,
}
#[derive(Clone, Debug)]
pub struct ConsoleAccessToken {
    email: String,
    sub: String,
    name: Option<String>,
    nickname: Option<String>,
}
impl ConsoleAccessToken {
    #[cfg(any(test, feature = "testing"))]
    pub fn new(email: String, sub: String) -> Self {
        Self {
            email,
            sub,
            name: None,
            nickname: None,
        }
    }

    pub fn email(&self) -> &str {
        &self.email
    }
}

impl From<ConsoleAccessToken> for UserInfo {
    fn from(value: ConsoleAccessToken) -> Self {
        Self {
            email: value.email,
            name: value.name,
            nickname: value.nickname,
        }
    }
}

#[derive(Deserialize, Clone)]
/// Relevant fields returned from the Auth0 userinfo endpoint
pub struct UserInfo {
    nickname: Option<String>,
    name: Option<String>,
    email: String,
}

impl UserInfo {
    pub fn nickname(&self) -> Option<&String> {
        self.nickname.as_ref()
    }

    pub fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }

    pub fn email(&self) -> &str {
        &self.email
    }
}

/// AuthenticatedLogin can only be constructed from a ConsoleAccessToken which
/// has been validated
pub struct AuthenticatedLogin {
    email: String,
    sub: String,
    user_info: Option<UserInfo>,
}

impl AuthenticatedLogin {
    pub fn new(token: ConsoleAccessToken, user_info: Option<UserInfo>) -> Self {
        AuthenticatedLogin {
            email: token.email,
            sub: token.sub,
            user_info,
        }
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn sub(&self) -> &str {
        &self.sub
    }

    pub fn user_info(&self) -> Option<&UserInfo> {
        self.user_info.as_ref()
    }
}

fn jwks_url(base_url: &Url) -> Url {
    base_url
        .join(CONFIG_URL_SUFFIX)
        .expect("Appending JWKS suffix to a valid URL should always succeed")
}

pub async fn validate_access_token<F, E>(
    access_token: &Auth0AccessToken,
    auth_url: &Url,
    http_client: impl Fn(HttpRequest) -> F + 'static,
    system_time: SystemTime,
) -> anyhow::Result<ConsoleAccessToken>
where
    F: Future<Output = Result<HttpResponse, E>>,
    E: std::error::Error + 'static + Send + Sync,
{
    let encoded_token = JWT::<ConsoleClaims, biscuit::Empty>::new_encoded(&access_token.0);
    let jwks_request = http::Request::builder()
        .uri(jwks_url(auth_url).to_string())
        .method(http::Method::GET)
        .header(
            http::header::ACCEPT,
            const { http::HeaderValue::from_static("application/json") },
        )
        .body(vec![])?;
    let (response, body) = http_client(jwks_request).await?.into_parts();
    if response.status != http::StatusCode::OK {
        anyhow::bail!(
            "Error from auth jwks request {} {}: {}",
            response.status,
            response.status.canonical_reason().unwrap_or("Unknown"),
            String::from_utf8_lossy(&body),
        )
    }
    let jwks: JWKSet<biscuit::Empty> = serde_json::de::from_slice(&body).with_context(|| {
        format!(
            "Invalid auth jwks response body: {}",
            String::from_utf8_lossy(&body)
        )
    })?;

    let algorithm = encoded_token
        .unverified_header()
        .context(ErrorMetadata::unauthenticated(
            "AccessTokenInvalid",
            "Access Token could not be decoded",
        ))?
        .registered
        .algorithm;
    // Encountering this error message while running `npx convex` against a dev
    // environment? Make sure you’re using the `--override-auth-url` and
    // `--override-auth-client` options as indicated in `README.md`.
    let decoded_token = encoded_token
        .decode_with_jwks(&jwks, Some(algorithm))
        .context(ErrorMetadata::unauthenticated(
            "AccessTokenInvalid",
            "Access Token could not be decoded",
        ))?;

    let mut validation_options = ValidationOptions {
        claim_presence_options: ClaimPresenceOptions {
            issuer: Presence::Required,
            audience: Presence::Required,
            subject: Presence::Required,
            expiry: Presence::Required,
            ..Default::default()
        },
        temporal_options: TemporalOptions {
            epsilon: chrono::Duration::zero(),
            now: Some(chrono::DateTime::from(system_time)),
        },
        issuer: Validation::Validate(auth_url.to_string()),
        audience: Validation::Validate(CONVEX_CONSOLE_API_AUDIENCE.to_string()),
        ..ValidationOptions::default()
    };
    let validation_result = decoded_token.validate(validation_options.clone());

    if matches!(
        validation_result,
        Err(biscuit::errors::Error::ValidationError(
            biscuit::errors::ValidationError::InvalidAudience(_)
        ))
    ) {
        validation_options.audience =
            Validation::Validate(ALTERNATE_CONVEX_API_AUDIENCE.to_string());
        decoded_token
            .validate(validation_options)
            .context(ErrorMetadata::unauthenticated(
                "AccessTokenInvalid",
                "Access Token could not be validated",
            ))?;
    } else {
        validation_result.context(ErrorMetadata::unauthenticated(
            "AccessTokenInvalid",
            "Access Token could not be validated",
        ))?;
    }
    let claims = decoded_token
        .payload()
        .context(ErrorMetadata::unauthenticated(
            "Unauthenticated",
            "Could not deserialize jwt claims",
        ))?;
    Ok(ConsoleAccessToken {
        email: claims.private.email.clone(),
        sub: claims
            .registered
            .subject
            .as_ref()
            .expect("Already validated subject is present")
            .to_owned(),
        // TODO(sarah) read these from the token if possible
        nickname: None,
        name: None,
    })
}

#[cfg(test)]
mod tests {
    use std::{
        convert::Infallible,
        pin::Pin,
        time::SystemTime,
    };

    use chrono::{
        Duration,
        Utc,
    };
    use common::auth::AuthInfo;
    use futures::{
        Future,
        FutureExt,
    };
    use keybroker::testing::TEST_SIGNING_KEY;
    use openidconnect::{
        core::{
            CoreClaimName,
            CoreGenderClaim,
            CoreIdToken,
            CoreIdTokenClaims,
            CoreJsonWebKeySet,
            CoreJweContentEncryptionAlgorithm,
            CoreJwsSigningAlgorithm,
            CoreProviderMetadata,
            CoreResponseType,
            CoreSubjectIdentifierType,
        },
        AdditionalClaims,
        Audience,
        EmptyAdditionalClaims,
        EmptyAdditionalProviderMetadata,
        EndUserEmail,
        HttpRequest,
        HttpResponse,
        IdToken,
        IdTokenClaims,
        IssuerUrl,
        JsonWebKeySetUrl,
        PrivateSigningKey,
        ResponseTypes,
        Scope,
        StandardClaims,
        SubjectIdentifier,
        TokenUrl,
        UserInfoUrl,
    };
    use serde::{
        Deserialize,
        Serialize,
    };

    use crate::{
        validate_access_token,
        validate_id_token,
        Auth0AccessToken,
        Auth0IdToken,
        CONVEX_AUTH_URL,
        CONVEX_CONSOLE_API_AUDIENCE,
    };

    fn fake_http_client(
        metadata: String,
        jwks: String,
    ) -> impl Fn(HttpRequest) -> Pin<Box<dyn Future<Output = Result<HttpResponse, Infallible>>>>
    {
        move |request: HttpRequest| {
            let metadata_ = metadata.clone();
            let jwks_ = jwks.clone();
            async move {
                if request.uri().path().ends_with("openid-configuration") {
                    Ok(http::Response::builder()
                        .status(http::StatusCode::OK)
                        .body(metadata_.into_bytes())
                        .unwrap())
                } else if request.uri().path().ends_with("jwks.json") {
                    Ok(http::Response::builder()
                        .status(http::StatusCode::OK)
                        .body(jwks_.into_bytes())
                        .unwrap())
                } else {
                    panic!("unexpected request path {:?}", request.uri());
                }
            }
            .boxed_local()
        }
    }

    #[tokio::test]
    async fn test_id_token_auth() -> anyhow::Result<()> {
        let issuer_url = IssuerUrl::new("https://dev-1sfr-rpl.us.auth0.com".to_string()).unwrap();
        let audience = Audience::new("client-id-123".to_string());
        let provider_metadata = serde_json::to_string(
            &CoreProviderMetadata::new(
                issuer_url.clone(),
                None,
                JsonWebKeySetUrl::new(
                    "https://dev-1sfr-rpl.us.auth0.com/.well-known/jwks.json".to_string(),
                )
                .unwrap(),
                vec![
                    ResponseTypes::new(vec![CoreResponseType::Code]),
                    ResponseTypes::new(vec![CoreResponseType::Token, CoreResponseType::IdToken]),
                    ResponseTypes::new(vec![CoreResponseType::IdToken]),
                ],
                vec![CoreSubjectIdentifierType::Public],
                vec![CoreJwsSigningAlgorithm::RsaSsaPssSha256],
                EmptyAdditionalProviderMetadata {},
            )
            .set_token_endpoint(Some(
                TokenUrl::new("https://dev-1sfr-rpl.us.auth0.com/oauth/token".to_string()).unwrap(),
            ))
            .set_userinfo_endpoint(Some(
                UserInfoUrl::new("https://dev-1sfr-rpl.us.auth0.com/userinfo".to_string()).unwrap(),
            ))
            .set_scopes_supported(Some(vec![
                Scope::new("openid".to_string()),
                Scope::new("email".to_string()),
                Scope::new("profile".to_string()),
            ]))
            .set_claims_supported(Some(vec![
                CoreClaimName::new("sub".to_string()),
                CoreClaimName::new("aud".to_string()),
                CoreClaimName::new("auth_time".to_string()),
                CoreClaimName::new("email".to_string()),
                CoreClaimName::new("exp".to_string()),
                CoreClaimName::new("iss".to_string()),
                CoreClaimName::new("iat".to_string()),
                CoreClaimName::new("name".to_string()),
                CoreClaimName::new("picture".to_string()),
            ])),
        )
        .unwrap();
        let jwks = serde_json::to_string(&CoreJsonWebKeySet::new(vec![
            TEST_SIGNING_KEY.as_verification_key()
        ]))
        .unwrap();
        let id_token = CoreIdToken::new(
            CoreIdTokenClaims::new(
                issuer_url.clone(),
                vec![audience.clone()],
                Utc::now() + Duration::seconds(120),
                Utc::now(),
                StandardClaims::new(SubjectIdentifier::new("1234-abcd".to_string()))
                    .set_email(Some(EndUserEmail::new("foo@bar.com".to_string()))),
                EmptyAdditionalClaims {},
            ),
            &*TEST_SIGNING_KEY,
            CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
            None,
            None,
        )
        .unwrap()
        .to_string();
        validate_id_token(
            Auth0IdToken(id_token),
            fake_http_client(provider_metadata, jwks),
            vec![AuthInfo {
                application_id: (*audience).clone(),
                domain: issuer_url,
            }],
            SystemTime::now(),
        )
        .await
        .unwrap();
        Ok(())
    }

    #[tokio::test]
    async fn test_access_token_auth() -> anyhow::Result<()> {
        let issuer_url = IssuerUrl::from_url(CONVEX_AUTH_URL.clone());
        let audience = Audience::new(CONVEX_CONSOLE_API_AUDIENCE.to_string());
        let jwks = serde_json::to_string(&CoreJsonWebKeySet::new(vec![
            TEST_SIGNING_KEY.as_verification_key()
        ]))
        .unwrap();

        // Cheat a little and just make an ID Token here, as an ID token is still an
        // access token. Add on some additional claims so it fits our desired format.
        // (This is to avoid pulling in yet another library just to create a JWT).
        #[derive(Debug, Deserialize, Serialize, Clone)]
        struct CvxClaims {
            #[serde(rename = "https://convex.dev/email")]
            pub email: String,
            pub scope: String,
        }
        impl AdditionalClaims for CvxClaims {}
        let id_token = IdToken::<
            CvxClaims,
            CoreGenderClaim,
            CoreJweContentEncryptionAlgorithm,
            CoreJwsSigningAlgorithm,
        >::new(
            IdTokenClaims::new(
                issuer_url.clone(),
                vec![audience.clone()],
                Utc::now() + Duration::seconds(120),
                Utc::now(),
                StandardClaims::new(SubjectIdentifier::new("1234-abcd".to_string())),
                CvxClaims {
                    email: "foo@bar.com".to_string(),
                    scope: "list:instances manage:instances".to_string(),
                },
            ),
            &*TEST_SIGNING_KEY,
            CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
            None,
            None,
        )
        .unwrap()
        .to_string();
        // Validates correctly
        validate_access_token(
            &Auth0AccessToken(id_token.clone()),
            &CONVEX_AUTH_URL,
            fake_http_client(String::new(), jwks.clone()),
            SystemTime::now(),
        )
        .await
        .unwrap();
        // Try again with a different audience
        validate_access_token(
            &Auth0AccessToken(id_token.clone()),
            &CONVEX_AUTH_URL.join("foo").unwrap(),
            fake_http_client(String::new(), jwks.clone()),
            SystemTime::now(),
        )
        .await
        .unwrap_err();
        // Try again with time moved past the token expiry.
        validate_access_token(
            &Auth0AccessToken(id_token.clone()),
            &CONVEX_AUTH_URL,
            fake_http_client(String::new(), jwks.clone()),
            (Utc::now() + Duration::seconds(200)).into(),
        )
        .await
        .unwrap_err();
        Ok(())
    }
}
