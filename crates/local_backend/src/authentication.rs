//! Code for handling authentication between the CLI user / dashboard and the
//! backend.

use anyhow::{
    anyhow,
    Context,
};
use async_trait::async_trait;
use authentication::extract_bearer_token;
use axum::{
    extract::FromRequestParts,
    RequestPartsExt,
};
use common::{
    http::{
        extract::Query,
        HttpResponseError,
    },
    runtime::Runtime,
    types::remove_type_prefix_from_admin_key,
};
use errors::ErrorMetadata;
use keybroker::Identity;
use serde::Deserialize;
use sync_types::{
    AuthenticationToken,
    UserIdentityAttributes,
};

use crate::LocalAppState;

pub struct ExtractAuthenticationToken(pub AuthenticationToken);

#[async_trait]
impl FromRequestParts<()> for ExtractAuthenticationToken {
    type Rejection = HttpResponseError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _st: &(),
    ) -> Result<Self, Self::Rejection> {
        // First, try extracting from headers
        if let Some(h) = parts.headers.get(http::header::AUTHORIZATION) {
            let h_str = h.to_str().context(ErrorMetadata::bad_request(
                "HeaderParseFailure",
                format!("Failed to parse header {h:?}"),
            ))?;
            let is_admin_key = h_str
                .get(..7)
                .ok_or_else(|| anyhow!("Invalid Header"))
                .context(ErrorMetadata::bad_request(
                    "InvalidHeaderFailure",
                    format!("Invalid authentication header"),
                ))?
                .eq_ignore_ascii_case("convex ");

            return if is_admin_key {
                // This is an admin key, not an OIDC bearer token. These are sent from the
                // dashboard in lieu of our old cookie-based auth.
                Ok(Self(extract_admin_key(h_str)?))
            } else {
                let auth: String = extract_bearer_token(Some(h_str.to_string()))
                    .await
                    .map_err(|_| {
                        anyhow::anyhow!(ErrorMetadata::bad_request(
                            "InvalidAdminKey",
                            "Invalid admin key",
                        ))
                    })?
                    .unwrap();
                Ok(Self(AuthenticationToken::User(auth)))
            };
        }

        // If no header is provided, also allow extracting admin key from query param.
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct QueryParams {
            admin_key: Option<String>,
        }
        if let Query(QueryParams {
            admin_key: Some(admin_key),
        }) = parts.extract().await?
        {
            return Ok(Self(AuthenticationToken::Admin(admin_key, None)));
        }

        Ok(Self(AuthenticationToken::None))
    }
}

impl From<ExtractAuthenticationToken> for AuthenticationToken {
    fn from(token: ExtractAuthenticationToken) -> Self {
        token.0
    }
}

pub struct ExtractIdentity(pub Identity);

#[async_trait]
impl FromRequestParts<LocalAppState> for ExtractIdentity {
    type Rejection = HttpResponseError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        st: &LocalAppState,
    ) -> Result<Self, Self::Rejection> {
        let token: AuthenticationToken =
            parts.extract::<ExtractAuthenticationToken>().await?.into();

        Ok(Self(
            st.application
                .authenticate(token, st.application.runtime().system_time())
                .await?,
        ))
    }
}

impl From<ExtractIdentity> for Identity {
    fn from(identity: ExtractIdentity) -> Self {
        identity.0
    }
}

pub struct TryExtractIdentity(pub anyhow::Result<Identity>);

#[async_trait]
impl FromRequestParts<LocalAppState> for TryExtractIdentity {
    type Rejection = HttpResponseError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        st: &LocalAppState,
    ) -> Result<Self, Self::Rejection> {
        let token: AuthenticationToken =
            parts.extract::<ExtractAuthenticationToken>().await?.into();

        Ok(Self(
            st.application
                .authenticate(token, st.application.runtime().system_time())
                .await,
        ))
    }
}

fn extract_admin_key(header: &str) -> anyhow::Result<AuthenticationToken> {
    let key = strip_prefix_ignore_case(header, "convex ")
        .context("Called extract_admin_key with a non-admin authorization header.")?;
    // We need to strip the unencrypted deployment type prefix ending in ':'
    // which clashes with the user impersonation logic below.
    // So theoretically this method accepts a key in the format:
    // "prod:some-depl-name123|sa67asd6a5da6d5:sd6f5sdf76dsf4ds6f4s68fd"
    // where the last part is the `acting_user_b64`.
    let key_without_prefix = remove_type_prefix_from_admin_key(key);
    // Looks for two parts split by a colon -- the first part always being the admin
    // key, and the second part being an optional base64 encoded
    // user to act as.
    match key_without_prefix.split_once(':') {
        // An admin acting as a user
        Some((key, acting_user_b64)) => {
            let attributes_s = base64::decode(acting_user_b64).context(
                ErrorMetadata::bad_request("HeaderParseFailure", "Malformed Authorization header."),
            )?;
            let attributes: UserIdentityAttributes =
                serde_json::from_slice::<serde_json::Value>(&attributes_s)
                    .context(ErrorMetadata::bad_request(
                        "HeaderParseFailure",
                        "Malformed Authorization header.",
                    ))?
                    .try_into()
                    .context(ErrorMetadata::bad_request(
                        "HeaderParseFailure",
                        "Malformed Authorization header.",
                    ))?;
            Ok(AuthenticationToken::Admin(
                key.to_string(),
                Some(attributes),
            ))
        },
        // Just an admin
        None => Ok(AuthenticationToken::Admin(key_without_prefix, None)),
    }
}

// Like `str::strip_prefix`, but ignores casing.
fn strip_prefix_ignore_case<'a>(string: &'a str, prefix: &str) -> Option<&'a str> {
    if string.len() >= prefix.len() && string[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&string[prefix.len()..])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use errors::ErrorMetadataAnyhowExt;
    use keybroker::testing::TestUserIdentity;
    use sync_types::{
        AuthenticationToken,
        UserIdentityAttributes,
    };

    use super::extract_admin_key;
    #[test]
    fn test_extracts_admin_key() -> anyhow::Result<()> {
        // Check that we don't panic no matter how short the admin key is
        assert_eq!(
            extract_admin_key("").unwrap_err().to_string(),
            "Called extract_admin_key with a non-admin authorization header."
        );

        assert_eq!(
            extract_admin_key("invalidHeader").unwrap_err().to_string(),
            "Called extract_admin_key with a non-admin authorization header."
        );

        assert_eq!(
            extract_admin_key("convex abc")?,
            AuthenticationToken::Admin("abc".to_string(), None)
        );

        // Capital C in header
        assert_eq!(
            extract_admin_key("Convex abc")?,
            AuthenticationToken::Admin("abc".to_string(), None)
        );

        let encoded = base64::encode(
            serde_json::to_vec(&serde_json::Value::try_from(UserIdentityAttributes::test())?)
                .unwrap(),
        );

        // With acting user
        assert_eq!(
            extract_admin_key(&format!("convex abc:{}", encoded))?,
            AuthenticationToken::Admin("abc".to_string(), Some(UserIdentityAttributes::test()))
        );

        // With acting user that isn't base64
        assert_eq!(
            extract_admin_key("convex abc:heyThisIsNotBase64")
                .unwrap_err()
                .short_msg(),
            "HeaderParseFailure",
        );

        // With deployment name and deployment type prefix
        assert_eq!(
            extract_admin_key(&format!("convex prod:high-horse-42|abc"))?,
            AuthenticationToken::Admin("high-horse-42|abc".to_string(), None)
        );

        Ok(())
    }
}
