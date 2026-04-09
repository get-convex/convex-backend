#![feature(never_type)]

use std::{
    str::FromStr,
    time::SystemTime,
};

use anyhow::Context;
use biscuit::{
    jwk::JWKSet,
    ClaimPresenceOptions,
    Presence,
    TemporalOptions,
    ValidationOptions,
    JWT,
};
use chrono::TimeZone;
use common::{
    auth::AuthInfo,
    types::TeamId,
};
use data_url::DataUrl;
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
    ClientId,
    DiscoveryError,
    IssuerUrl,
};
use serde::{
    Deserialize,
    Serialize,
};
use sync_types::AuthenticationToken;
use tuple_struct::tuple_struct_string;

tuple_struct_string!(WorkOSOrgID);

pub mod access_token_auth;
pub mod application_auth;
pub mod metrics;

fn redact_jwt_error_if_needed(error_msg: &str) -> String {
    if error_msg.contains("Could not decode token. The JWT's 'kid'") {
        // Remove specific kid value details in prod
        "Could not decode token. The JWT's 'kid' (key ID) header doesn't match any key in the \
         provider's JWKS, or the JWT signature is invalid."
            .to_string()
    } else if error_msg.contains("Could not decode token. The JWT is missing a 'kid'") {
        // Remove detailed kid guidance but keep helpful info
        "Could not decode token. The JWT is missing a 'kid' (key ID) header, or the JWT signature \
         is invalid."
            .to_string()
    } else {
        error_msg.to_string()
    }
}

fn enhance_no_provider_error(auth_infos: &[AuthInfo], should_redact: bool) -> String {
    if should_redact {
        return "No auth provider found matching the given token".to_string();
    }

    let configured_providers: Vec<String> = auth_infos
        .iter()
        .map(|info| match info {
            AuthInfo::Oidc {
                domain,
                application_id,
                ..
            } => format!("OIDC(domain={domain}, app_id={application_id})"),
            AuthInfo::CustomJwt {
                issuer,
                application_id,
                ..
            } => format!(
                "CustomJWT(issuer={}, app_id={})",
                issuer,
                application_id
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("none")
            ),
        })
        .collect();

    if configured_providers.is_empty() {
        "No auth provider found matching the given token (no providers configured). Check \
         convex/auth.config.ts."
            .to_string()
    } else {
        format!(
            "No auth provider found matching the given token. Check that your JWT's issuer and \
             audience match one of your configured providers: [{}]",
            configured_providers.join(", ")
        )
    }
}

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
                Ok(Some(format!("Convex {key}:{encoded}")))
            },
            None => Ok(Some(format!("Convex {key}"))),
        },
        AuthenticationToken::User(key) => Ok(Some(format!("Bearer {key}"))),
        AuthenticationToken::None => Ok(None),
    }
}

/// Validate a token against a list of Convex auth providers.
pub async fn validate_id_token<F, E>(
    token_str: AuthIdToken,
    // The http client is injected here so we can unit test this filter without needing to actually
    // serve an HTTP response from an identity provider.
    http_client: impl Fn(HttpRequest) -> F + 'static,
    auth_infos: Vec<AuthInfo>,
    system_time: SystemTime,
    should_redact_errors: bool,
) -> anyhow::Result<UserIdentity>
where
    F: Future<Output = Result<HttpResponse, E>>,
    E: std::error::Error + 'static + Send + Sync,
{
    let since_epoch = system_time
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("couldn't calculate unix timestamp?")
        .as_secs() as i64;
    let chrono_utc = chrono::Utc.timestamp_opt(since_epoch, 0).unwrap();

    // All tokens are JWTs, so start with that to pull out the issuer and audiences
    // to route the token to the correct provider.
    let (auth_info, token_issuer) = {
        let encoded_token = JWT::<biscuit::Empty, biscuit::Empty>::new_encoded(&token_str.0);

        // NB: A malicious token can at worst point us to the wrong provider, but then
        // subsequent token verification will fail.
        let payload =
            encoded_token
                .unverified_payload()
                .context(ErrorMetadata::unauthenticated(
                    "InvalidAuthHeader",
                    "Could not parse JWT payload. Check that the token is a valid JWT format with \
                     three base64-encoded parts separated by dots.",
                ))?;
        let Some(issuer) = payload.registered.issuer else {
            anyhow::bail!(ErrorMetadata::unauthenticated(
                "InvalidAuthHeader",
                "Missing issuer claim ('iss') in JWT payload. The JWT must include an 'iss' claim \
                 that matches one of your configured auth providers."
            ));
        };
        let audiences = match payload.registered.audience {
            Some(biscuit::SingleOrMultiple::Single(audience)) => vec![audience],
            Some(biscuit::SingleOrMultiple::Multiple(audiences)) => audiences,
            None => vec![],
        };
        // Find the first provider matching this token.
        // `iss` claim must match the provider's issuer but 'aud' claim is only
        // required to match if the provider has an applicationId specified.
        let auth_info = auth_infos
            .iter()
            .find(|info| info.matches_token(&audiences, &issuer))
            .cloned()
            .context(ErrorMetadata::unauthenticated(
                "NoAuthProvider",
                enhance_no_provider_error(&auth_infos, should_redact_errors),
            ))?;
        (auth_info, issuer)
    };
    // Okay, now that we've picked with auth provider to use, actually do token
    // verification.
    match auth_info {
        AuthInfo::Oidc { application_id, .. } => {
            // Use the OpenID Connect Discovery protocol to get the public keys for this
            // provider.
            // TODO(CX-606): Add an caching layer that respects the HTTP cache headers
            // in the response.

            let issuer_url = IssuerUrl::new(token_issuer.clone())?;
            let metadata = CoreProviderMetadata::discover_async(issuer_url, &http_client)
                .await
                .map_err(|e| {
                    let short = "AuthProviderDiscoveryFailed";
                    let long = format!(
                        "Auth provider discovery of {} failed",
                        token_issuer.as_str()
                    );
                    match e {
                        DiscoveryError::Response(code, body, _) => {
                            let long =
                                format!("{long}: {} {}", code, String::from_utf8_lossy(&body));
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
                                token_issuer.as_str(),
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
                ClientId::new(application_id),
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
            .set_time_fn(|| chrono_utc);
            let token = CoreIdTokenWithCustomClaims::from_str(&token_str.0).context(
                ErrorMetadata::unauthenticated(
                    "InvalidAuthHeader",
                    "Could not parse as OIDC ID token. Token might not be an OIDC-compliant JWT.",
                ),
            )?;
            UserIdentity::from_token(token, verifier).context(ErrorMetadata::unauthenticated(
                "Unauthenticated",
                "Could not verify OIDC token claim. Check that the token signature is valid and \
                 the token hasn't expired.",
            ))
        },
        AuthInfo::CustomJwt {
            application_id,
            jwks: jwks_uri,
            issuer,
            algorithm,
        } => {
            let jwks_body = fetch_jwks(&jwks_uri, &http_client).await?;
            let jwks: JWKSet<biscuit::Empty> = serde_json::de::from_slice(&jwks_body)
                .with_context(|| {
                    ErrorMetadata::unauthenticated(
                        "InvalidAuthHeader",
                        format!(
                            "Invalid JWKS response body from '{jwks_uri}'. The response is not \
                             valid JSON or doesn't match the expected JWKS format."
                        ),
                    )
                })?;
            let token = JWT::<serde_json::Value, biscuit::Empty>::new_encoded(&token_str.0);
            let decoded_token = token
                .decode_with_jwks(&jwks, Some(algorithm.into()))
                .with_context(|| {
                    // Try to extract more specific error information
                    let unverified_header = token.unverified_header().ok();
                    let kid = unverified_header.and_then(|h| h.registered.key_id.clone());

                    let detailed_msg = if let Some(kid) = kid {
                        format!(
                            "Could not decode token. The JWT's 'kid' (key ID) header is '{kid}', \
                             does this key match any key in the provider's JWKS?"
                        )
                    } else {
                        "Could not decode token. JWT may be missing a 'kid' (key ID) header."
                            .to_string()
                    };

                    let final_msg = if should_redact_errors {
                        redact_jwt_error_if_needed(&detailed_msg)
                    } else {
                        detailed_msg
                    };

                    ErrorMetadata::unauthenticated("InvalidAuthHeader", final_msg)
                })?;
            let payload = decoded_token.payload()?;
            let Some(ref token_issuer) = payload.registered.issuer else {
                anyhow::bail!(ErrorMetadata::unauthenticated(
                    "InvalidAuthHeader",
                    "Missing issuer claim ('iss') in JWT payload. The JWT must include an 'iss' \
                     claim that matches one of your configured auth providers."
                ));
            };
            let token_issuer_with_protocol =
                if token_issuer.starts_with("https://") || token_issuer.starts_with("http://") {
                    token_issuer.to_string()
                } else {
                    format!("https://{token_issuer}")
                };

            if token_issuer_with_protocol.trim_end_matches('/')
                != issuer.as_str().trim_end_matches('/')
            {
                anyhow::bail!(ErrorMetadata::unauthenticated(
                    "InvalidAuthHeader",
                    format!("Invalid issuer: {token_issuer} != {issuer}")
                ));
            }
            if let Some(application_id) = application_id {
                let Some(ref token_audience) = payload.registered.audience else {
                    anyhow::bail!(ErrorMetadata::unauthenticated(
                        "InvalidAuthHeader",
                        "Missing audience claim ('aud') in JWT payload. The JWT must include an \
                         'aud' claim that matches your configured application ID."
                    ));
                };
                if !token_audience
                    .iter()
                    .any(|audience| audience == &application_id)
                {
                    let audiences = token_audience
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>();
                    anyhow::bail!(ErrorMetadata::unauthenticated(
                        "InvalidAuthHeader",
                        format!("Invalid audience: {application_id} not in {audiences:?}"),
                    ));
                }
            }
            let validation_options = ValidationOptions {
                temporal_options: TemporalOptions {
                    now: Some(chrono_utc),
                    epsilon: chrono::Duration::seconds(5),
                },
                ..Default::default()
            };
            decoded_token
                .validate(validation_options)
                .map_err(|original_error| {
                    eprintln!("Original validation error: {original_error:?}");
                    let msg = original_error.to_string();

                    ErrorMetadata::unauthenticated(
                        "InvalidAuthHeader",
                        format!("Could not validate token: {msg}"),
                    )
                })?;
            UserIdentity::from_custom_jwt(decoded_token, token_str.0).context(
                ErrorMetadata::unauthenticated(
                    "InvalidAuthHeader",
                    "Could not verify token claim. Check that the JWT contains valid claims and \
                     matches your auth provider configuration.",
                ),
            )
        },
    }
}

const JWKS_MEDIA_TYPES: [&str; 2] = [
    "application/json",
    // https://www.iana.org/assignments/media-types/application/jwk-set+json
    // used by WorkOS
    "application/jwk-set+json",
];

const APPLICATION_JSON: http::HeaderValue = http::HeaderValue::from_static("application/json");

async fn fetch_jwks<F, E>(
    jwks_uri: &str,
    http_client: &(impl Fn(HttpRequest) -> F + 'static),
) -> anyhow::Result<Vec<u8>>
where
    F: Future<Output = Result<HttpResponse, E>>,
    E: std::error::Error + 'static + Send + Sync,
{
    if let Ok(data_url) = DataUrl::process(jwks_uri) {
        // Don't bother checking the MIME type for data: URLs
        let (data, _fragment) = data_url.decode_to_vec().with_context(|| {
            ErrorMetadata::unauthenticated(
                "InvalidAuthHeader",
                "Invalid JWKS data URL. Check that the data URL is properly formatted and \
                 contains valid base64-encoded JSON.",
            )
        })?;
        return Ok(data);
    }

    let request = http::Request::builder()
        .uri(jwks_uri)
        .method(http::Method::GET)
        .header(http::header::ACCEPT, APPLICATION_JSON)
        .body(vec![])?;
    let response = http_client(request).await.map_err(|e| {
        ErrorMetadata::unauthenticated(
            "InvalidAuthHeader",
            format!(
                "Could not fetch JWKS from URL '{jwks_uri}': {e}. Check that the URL is correct \
                 and accessible."
            ),
        )
    })?;
    if response.status() != http::StatusCode::OK {
        anyhow::bail!(ErrorMetadata::unauthenticated(
            "InvalidAuthHeader",
            format!(
                "Could not fetch JWKS from URL '{}': HTTP {} {}. Check that the URL is correct \
                 and accessible.",
                jwks_uri,
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )
        ));
    }
    if !response
        .headers()
        .get(http::header::CONTENT_TYPE)
        .is_some_and(|ty| {
            ty.to_str()
                .ok()
                .and_then(|s| s.parse::<mime::Mime>().ok())
                .is_some_and(|mime| {
                    JWKS_MEDIA_TYPES
                        .iter()
                        .any(|&allowed| mime.essence_str().eq_ignore_ascii_case(allowed))
                        && mime.get_param("charset").is_none_or(|val| val == "utf-8")
                })
        })
    {
        let content_type = response
            .headers()
            .get(http::header::CONTENT_TYPE)
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("unknown");
        anyhow::bail!(ErrorMetadata::unauthenticated(
            "InvalidAuthHeader",
            format!(
                "Invalid Content-Type '{content_type}' when fetching JWKS from '{jwks_uri}'. \
                 Expected 'application/json' or 'application/jwk-set+json'."
            )
        ));
    }

    Ok(response.into_body())
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthAccessToken(pub String);
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthIdToken(pub String);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceToken(pub String);
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppToken {
    pub client_id: String,
    pub client_secret: String,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct WorkOSClaims {
    #[serde(rename = "workos_first_name")]
    first_name: Option<String>,
    #[serde(rename = "workos_last_name")]
    last_name: Option<String>,
    #[serde(rename = "workos_email")]
    email: Option<String>,

    #[serde(flatten)]
    vercel: Option<VercelClaims>,

    sso_team_id: Option<String>,

    org_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum VercelUserRole {
    ADMIN,
    USER,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VercelClaims {
    // The installation_id is the id of the vercel integration installation.
    // We use it to decide which team a member should belong to if they're logging
    // in via Vercel.
    #[serde(rename = "vercel_installation_id")]
    installation_id: String,
    // Obfuscated id of the vercel team.
    #[serde(rename = "vercel_account_id")]
    account_id: String,
    // The user role is the role of the user in the vercel team.
    // We use it to decide whether the user should be an admin or developer.
    #[serde(rename = "vercel_user_role")]
    user_role: VercelUserRole,
}

impl VercelClaims {
    pub fn installation_id(&self) -> &str {
        &self.installation_id
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn user_role(&self) -> &VercelUserRole {
        &self.user_role
    }
}

#[derive(Clone, Debug)]
pub struct ConsoleAccessToken {
    email: Option<String>,
    sub: String,
    name: Option<String>,
    vercel: Option<VercelClaims>,
    sso_team_id: Option<TeamId>,
    workos_org_id: Option<WorkOSOrgID>,
}

impl ConsoleAccessToken {
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    pub fn sub(&self) -> &str {
        &self.sub
    }

    pub fn workos_org_id(&self) -> Option<&WorkOSOrgID> {
        self.workos_org_id.as_ref()
    }
}

impl From<ConsoleAccessToken> for UserInfo {
    fn from(value: ConsoleAccessToken) -> Self {
        Self {
            email: value.email,
            name: value.name,
            vercel: value.vercel,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
/// Relevant fields in the WorkOS JWT
pub struct UserInfo {
    name: Option<String>,
    email: Option<String>,
    vercel: Option<VercelClaims>,
}

impl UserInfo {
    pub fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }

    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    pub fn vercel_info(&self) -> Option<&VercelClaims> {
        self.vercel.as_ref()
    }
}

/// AuthenticatedLogin can only be constructed from a ConsoleAccessToken which
/// has been validated
pub struct AuthenticatedLogin {
    email: Option<String>,
    sub: String,
    user_info: Option<UserInfo>,
    sso_team_id: Option<TeamId>,
    workos_org_id: Option<WorkOSOrgID>,
}

impl AuthenticatedLogin {
    pub fn new(token: ConsoleAccessToken, user_info: Option<UserInfo>) -> Self {
        AuthenticatedLogin {
            email: token.email,
            sub: token.sub,
            user_info,
            sso_team_id: token.sso_team_id,
            workos_org_id: token.workos_org_id,
        }
    }

    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    pub fn sub(&self) -> &str {
        &self.sub
    }

    pub fn user_info(&self) -> Option<&UserInfo> {
        self.user_info.as_ref()
    }

    pub fn vercel_info(&self) -> Option<&VercelClaims> {
        self.user_info.as_ref().and_then(|ui| ui.vercel_info())
    }

    pub fn sso_team_id(&self) -> Option<TeamId> {
        self.sso_team_id
    }

    pub fn workos_org_id(&self) -> Option<&WorkOSOrgID> {
        self.workos_org_id.as_ref()
    }
}

pub fn names_to_full_name(first_name: Option<String>, last_name: Option<String>) -> Option<String> {
    match (first_name, last_name) {
        (Some(first), Some(last)) => Some(format!("{first} {last}")),
        (Some(first), None) => Some(first),
        (None, Some(last)) => Some(last),
        (None, None) => None,
    }
}

pub async fn validate_access_token<F, E>(
    access_token: &AuthAccessToken,
    http_client: impl Fn(HttpRequest) -> F + 'static,
    system_time: SystemTime,
    workos_client_id: &str,
    workos_api_key: &str,
    workos_auth_urls: &Vec<String>,
) -> anyhow::Result<ConsoleAccessToken>
where
    F: Future<Output = Result<HttpResponse, E>>,
    E: std::error::Error + 'static + Send + Sync,
{
    if workos_api_key.is_empty() {
        anyhow::bail!(
            "WORKOS_API_KEY is not set. For local development, you may find this key in 1password \
             under 'WorkOS staging API Key'"
        );
    }

    let encoded_token = JWT::<WorkOSClaims, biscuit::Empty>::new_encoded(&access_token.0);

    // Fetch WorkOS JWKS
    let jwks_url = format!("https://apiauth.convex.dev/sso/jwks/{workos_client_id}");
    let jwks_data = fetch_jwks(&jwks_url, &http_client).await?;
    let jwks: JWKSet<biscuit::Empty> =
        serde_json::de::from_slice(&jwks_data).with_context(|| {
            format!(
                "Invalid WorkOS jwks response body: {}",
                String::from_utf8_lossy(&jwks_data)
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

    let decoded_token = encoded_token
        .decode_with_jwks(&jwks, Some(algorithm))
        .context(ErrorMetadata::unauthenticated(
            "AccessTokenInvalid",
            "Access Token could not be decoded",
        ))?;

    let validation_options = ValidationOptions {
        claim_presence_options: ClaimPresenceOptions {
            issuer: Presence::Required,
            audience: Presence::Required,
            subject: Presence::Required,
            expiry: Presence::Required,
            ..Default::default()
        },
        temporal_options: TemporalOptions {
            epsilon: chrono::Duration::seconds(5),
            now: Some(chrono::DateTime::from(system_time)),
        },
        // Use default audience validation (which is to ignore)
        ..ValidationOptions::default()
    };

    decoded_token
        .validate(validation_options)
        .context(ErrorMetadata::unauthenticated(
            "AccessTokenInvalid",
            "Access Token could not be validated",
        ))?;

    let claims = decoded_token
        .payload()
        .context(ErrorMetadata::unauthenticated(
            "Unauthenticated",
            "Could not deserialize jwt claims",
        ))?;

    let issuer = claims.registered.issuer.as_ref().ok_or_else(|| {
        anyhow::anyhow!(ErrorMetadata::unauthenticated(
            "AccessTokenInvalid",
            "Access Token missing issuer claim"
        ))
    })?;

    let allowed_issuer_domains = vec![
        "https://api.workos.com/user_management/".to_string(),
        "https://apiauth.convex.dev/user_management/".to_string(),
        "https://api.auth.convex.dev/user_management/".to_string(),
    ];

    let matching_issuer_domain = allowed_issuer_domains
        .iter()
        .find(|domain| issuer.starts_with(domain.as_str()));
    match matching_issuer_domain {
        Some(matching_issuer) => {
            anyhow::ensure!(
                *issuer == format!("{matching_issuer}{workos_client_id}"),
                ErrorMetadata::unauthenticated(
                    "AccessTokenInvalid",
                    format!("Issuer {issuer} does not match WorkOS client ID")
                )
            )
        },
        None => {
            anyhow::ensure!(
                workos_auth_urls.iter().any(|url| {
                    let normalized_url = url.trim_end_matches('/');
                    let normalized_issuer = issuer.trim_end_matches('/');
                    normalized_url == normalized_issuer
                }),
                ErrorMetadata::unauthenticated(
                    "AccessTokenInvalid",
                    format!("Issuer {issuer} not in allowed WorkOS auth URLs")
                )
            );
        },
    }

    let audiences = claims.registered.audience.as_ref().ok_or_else(|| {
        ErrorMetadata::unauthenticated("AccessTokenInvalid", "Access Token missing audience claim")
    })?;
    if !audiences.iter().any(|a| a == workos_client_id) {
        tracing::info!(
            audience = ?audiences,
            expected_audience = %workos_client_id,
            issuer = %issuer,
            subject = ?claims.registered.subject,
            "WorkOS access token does not have valid audience"
        );
        anyhow::bail!(ErrorMetadata::forbidden(
            "AccessTokenForbidden",
            format!("Audience does not contain WorkOS client ID {workos_client_id}"),
        ));
    };

    let full_name = names_to_full_name(
        claims.private.first_name.clone(),
        claims.private.last_name.clone(),
    );

    let sub = match claims.registered.subject.as_ref() {
        Some(sub) => sub.clone(),
        None => anyhow::bail!("Missing subject claim"),
    };

    let sso_team_id = claims
        .private
        .sso_team_id
        .as_ref()
        .map(|id| id.parse().map(TeamId))
        .transpose()?;

    Ok(ConsoleAccessToken {
        email: claims.private.email.clone(),
        sub,
        vercel: claims.private.vercel.clone(),
        sso_team_id,
        workos_org_id: claims.private.org_id.clone().map(WorkOSOrgID),
        name: full_name,
    })
}
