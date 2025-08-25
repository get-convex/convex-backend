use std::{
    str::FromStr,
    sync::LazyLock,
};

use anyhow::Context;
use errors::ErrorMetadata;
use openidconnect::IssuerUrl;
use regex::Regex;
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum AuthInfo {
    Oidc {
        /// Tokens issued by the auth provider must have this application ID in
        /// their audiences.
        application_id: String,
        /// The domain of the OIDC auth provider.
        domain: IssuerUrl,
    },
    CustomJwt {
        /// Tokens issued by the auth provider must have this application ID in
        /// their audiences.
        application_id: Option<String>,
        /// The issuer of the JWT auth provider (e.g. `https://auth.example.com`)
        issuer: IssuerUrl,
        /// The URL to fetch the JWKS (e.g. `https://auth.example.com/.well-known/jwks.json`)
        jwks: String,
        /// The algorithm used to sign the JWT tokens. Convex currently only
        /// supports RS256 and ES256.
        algorithm: SignatureAlgorithm,
    },
}

impl AuthInfo {
    pub fn domain(&self) -> &IssuerUrl {
        match self {
            AuthInfo::Oidc { domain, .. } => domain,
            AuthInfo::CustomJwt { issuer, .. } => issuer,
        }
    }

    pub fn matches_token(&self, audiences: &[String], issuer: &str) -> bool {
        let (application_id, domain) = match self {
            AuthInfo::Oidc {
                application_id,
                domain,
            } => (Some(application_id), domain),
            AuthInfo::CustomJwt {
                application_id,
                issuer,
                ..
            } => (application_id.as_ref(), issuer),
        };
        if let Some(application_id) = application_id
            && !audiences.contains(application_id)
        {
            return false;
        }

        // Some JWTs (from https://www.dynamic.xyz at least) don't include https:// in
        // the `iss` field of the JWT. Since we automatically add this for the
        // auth.config.ts `issuer` property let's add it to the JWT `iss` field as well.
        let issuer_with_protocol =
            if issuer.starts_with("https://") || issuer.starts_with("http://") {
                issuer.to_string()
            } else {
                format!("https://{}", issuer)
            };

        // Some authentication providers (Auth0, lookin' at you) tell developers that
        // their identity domain doesn't have a trailing slash, but the OIDC tokens do
        // have one in the `issuer` field. This is consistent with what the OIDC
        // Discovery response will contain, but the value entered in the instance config
        // may or may not have the slash.
        domain.trim_end_matches('/') == issuer_with_protocol.trim_end_matches('/')
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Ord, PartialOrd)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum SignatureAlgorithm {
    RS256,
    ES256,
}

impl From<SignatureAlgorithm> for biscuit::jwa::SignatureAlgorithm {
    fn from(algorithm: SignatureAlgorithm) -> Self {
        match algorithm {
            SignatureAlgorithm::RS256 => biscuit::jwa::SignatureAlgorithm::RS256,
            SignatureAlgorithm::ES256 => biscuit::jwa::SignatureAlgorithm::ES256,
        }
    }
}

impl From<SignatureAlgorithm> for String {
    fn from(algorithm: SignatureAlgorithm) -> Self {
        serde_json::to_string(&algorithm)
            .expect("Failed to serialize SignatureAlgorithm to a string")
    }
}

impl FromStr for SignatureAlgorithm {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s).with_context(|| {
            ErrorMetadata::bad_request(
                "InvalidSignatureAlgorithm",
                format!("Invalid signature algorithm (only RS256 and ES256 are supported): {s}"),
            )
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Ord, PartialOrd)]
#[serde(tag = "type")]
pub enum SerializedAuthInfo {
    #[serde(rename = "customJwt")]
    CustomJwt {
        #[serde(rename = "applicationID")]
        application_id: Option<String>,
        issuer: String,
        jwks: String,
        algorithm: SignatureAlgorithm,
    },
    #[serde(untagged)]
    Oidc {
        #[serde(rename = "applicationID")]
        application_id: String,
        domain: String,
    },
}

impl TryFrom<AuthInfo> for SerializedAuthInfo {
    type Error = anyhow::Error;

    fn try_from(auth_info: AuthInfo) -> Result<Self, Self::Error> {
        let result = match auth_info {
            AuthInfo::Oidc {
                application_id,
                domain,
            } => SerializedAuthInfo::Oidc {
                application_id,
                domain: domain.to_string(),
            },
            AuthInfo::CustomJwt {
                application_id,
                issuer,
                jwks,
                algorithm,
            } => SerializedAuthInfo::CustomJwt {
                application_id,
                issuer: issuer.to_string(),
                jwks,
                algorithm,
            },
        };
        Ok(result)
    }
}

impl TryFrom<SerializedAuthInfo> for AuthInfo {
    type Error = anyhow::Error;

    fn try_from(serialized_auth_info: SerializedAuthInfo) -> Result<Self, Self::Error> {
        let result = match serialized_auth_info {
            SerializedAuthInfo::Oidc {
                application_id,
                domain,
            } => {
                let domain = deserialize_issuer_url(domain)?;
                Self::Oidc {
                    application_id,
                    domain,
                }
            },
            SerializedAuthInfo::CustomJwt {
                application_id,
                issuer,
                jwks,
                algorithm,
            } => {
                let issuer = deserialize_issuer_url(issuer)?;
                Self::CustomJwt {
                    application_id,
                    issuer,
                    jwks,
                    algorithm,
                }
            },
        };
        Ok(result)
    }
}

static PROTOCOL_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\w+://").unwrap());

fn deserialize_issuer_url(original_url: String) -> anyhow::Result<IssuerUrl> {
    let (had_scheme, url) = if PROTOCOL_REGEX.is_match(&original_url) {
        (true, original_url.clone())
    } else {
        (false, format!("https://{original_url}"))
    };
    if url.starts_with("http://") {
        let parsed_url = IssuerUrl::new(url)?;
        return Ok(parsed_url);
    };
    if !url.starts_with("https://") {
        anyhow::bail!("Invalid provider domain URL \"{original_url}\": must use HTTPS");
    }
    let parsed_url = IssuerUrl::new(url)?;
    // Check if the input really looks like a URL,
    // to catch mistakes (e.g. putting random tokens in the domain field)
    if !had_scheme && !parsed_url.url().host_str().is_some_and(ends_with_tld) {
        anyhow::bail!(
            "Invalid provider domain URL \"{original_url}\": Does not look like a URL (must have \
             a scheme or end with a top-level domain)"
        );
    }

    Ok(parsed_url)
}

fn ends_with_tld(host: &str) -> bool {
    if host == "localhost" {
        return true;
    }
    let Some((_, maybe_tld)) = host.rsplit_once('.') else {
        return false;
    };
    tld::exist(maybe_tld)
}

impl AuthInfo {
    #[cfg(any(test, feature = "testing"))]
    pub fn test_example() -> Self {
        Self::Oidc {
            application_id: "12345".to_string(),
            domain: IssuerUrl::new("https://convex.dev".to_string()).unwrap(),
        }
    }
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for AuthInfo {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = AuthInfo>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        prop_oneof![
            // Generate OIDC variant
            (any::<String>(), any::<proptest_http::ArbitraryUri>()).prop_filter_map(
                "String and URI weren't valid OIDC AuthInfo",
                |(s, uri)| {
                    IssuerUrl::new(format!("{}", uri.0))
                        .map(|domain| Self::Oidc {
                            application_id: s,
                            domain,
                        })
                        .ok()
                },
            ),
            // Generate JWT variant
            (
                any::<Option<String>>(),              // application_id
                any::<proptest_http::ArbitraryUri>(), // issuer
                any::<proptest_http::ArbitraryUri>(), // jwks
                any::<SignatureAlgorithm>(),          // algorithm
            )
                .prop_filter_map(
                    "String and URIs weren't valid JWT AuthInfo",
                    |(app_id, issuer_uri, jwks_uri, algorithm)| {
                        IssuerUrl::new(format!("{}", issuer_uri.0))
                            .map(|issuer| Self::CustomJwt {
                                application_id: app_id,
                                issuer,
                                jwks: jwks_uri.0.to_string(),
                                algorithm,
                            })
                            .ok()
                    }
                )
        ]
    }
}

#[derive(Debug)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, Clone, PartialEq)
)]
pub struct AuthConfig {
    pub providers: Vec<AuthInfo>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct SerializedAuthConfig {
    pub providers: Vec<SerializedAuthInfo>,
}

impl TryFrom<AuthConfig> for SerializedAuthConfig {
    type Error = anyhow::Error;

    fn try_from(auth_config: AuthConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            providers: auth_config
                .providers
                .into_iter()
                .map(SerializedAuthInfo::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl TryFrom<SerializedAuthConfig> for AuthConfig {
    type Error = anyhow::Error;

    fn try_from(serialized_auth_config: SerializedAuthConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            providers: serialized_auth_config
                .providers
                .into_iter()
                .map(AuthInfo::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::auth::{
        AuthInfo,
        SerializedAuthInfo,
    };

    #[test]
    fn test_auth_info_https_prefix() -> anyhow::Result<()> {
        let AuthInfo::Oidc { domain, .. } = serde_json::from_str::<SerializedAuthInfo>(
            r#"{"applicationID": "123", "domain": "example.com"}"#,
        )?
        .try_into()?
        else {
            panic!("Expected Oidc AuthInfo");
        };
        assert_eq!(domain.to_string(), "https://example.com");
        let AuthInfo::Oidc { domain, .. } = serde_json::from_str::<SerializedAuthInfo>(
            r#"{"applicationID": "123", "domain": "localhost"}"#,
        )?
        .try_into()?
        else {
            panic!("Expected Oidc AuthInfo");
        };
        assert_eq!(domain.to_string(), "https://localhost");
        Ok(())
    }

    #[test]
    fn test_auth_info_file_fails() -> anyhow::Result<()> {
        let serialized = serde_json::from_str::<SerializedAuthInfo>(
            r#"{"applicationID": "123", "domain": "file://example.com"}"#,
        )?;
        AuthInfo::try_from(serialized).unwrap_err();
        Ok(())
    }

    #[test]
    fn test_auth_info_http_localhost() -> anyhow::Result<()> {
        let AuthInfo::Oidc { domain, .. } = serde_json::from_str::<SerializedAuthInfo>(
            r#"{"applicationID": "123", "domain": "http://localhost:3211"}"#,
        )?
        .try_into()?
        else {
            panic!("Expected Oidc AuthInfo");
        };
        assert_eq!(domain.to_string(), "http://localhost:3211");

        let AuthInfo::Oidc { domain, .. } = serde_json::from_str::<SerializedAuthInfo>(
            r#"{"applicationID": "123", "domain": "http://127.0.0.1:3211"}"#,
        )?
        .try_into()?
        else {
            panic!("Expected Oidc AuthInfo");
        };
        assert_eq!(domain.to_string(), "http://127.0.0.1:3211");

        Ok(())
    }

    #[test]
    fn test_auth_info_rejects_bogus_domain() -> anyhow::Result<()> {
        let serialized = serde_json::from_str::<SerializedAuthInfo>(
            r#"{"applicationID": "123", "domain": "foobar123"}"#,
        )?;
        AuthInfo::try_from(serialized).unwrap_err();
        let serialized = serde_json::from_str::<SerializedAuthInfo>(
            r#"{"applicationID": "123", "domain": "idont.looklikeadomain"}"#,
        )?;
        AuthInfo::try_from(serialized).unwrap_err();
        Ok(())
    }
}
