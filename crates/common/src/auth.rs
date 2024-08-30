use std::sync::LazyLock;

use openidconnect::IssuerUrl;
use regex::Regex;
use serde::{
    Deserialize,
    Deserializer,
    Serialize,
};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct AuthInfo {
    #[serde(rename = "applicationID")]
    pub application_id: String,
    #[serde(deserialize_with = "deserialize_url_default_to_https")]
    pub domain: IssuerUrl,
}

static PROTOCOL_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\w+://").unwrap());

fn deserialize_url_default_to_https<'de, D>(deserializer: D) -> Result<IssuerUrl, D::Error>
where
    D: Deserializer<'de>,
{
    let url = String::deserialize(deserializer)?;
    let url = if PROTOCOL_REGEX.is_match(&url) {
        url
    } else {
        format!("https://{url}")
    };
    if url.starts_with("http://") {
        let parsed_url: IssuerUrl = serde_json::to_string(&url)
            .and_then(|json| serde_json::from_str(&json))
            .map_err(|error| {
                serde::de::Error::custom(format!("Invalid provider domain URL \"{url}\": {error}"))
            })?;
        if parsed_url.url().host_str() == Some("localhost")
            || parsed_url.url().host_str() == Some("127.0.0.1")
        {
            return Ok(parsed_url);
        } else {
            return Err(serde::de::Error::custom("must use HTTPS"));
        }
    };
    url.starts_with("https://")
        .then_some(url.clone())
        .ok_or(serde::de::Error::custom("must use HTTPS"))
        .and_then(|url| serde_json::to_string(&url))
        .and_then(|json| serde_json::from_str(&json))
        .map_err(|error| {
            serde::de::Error::custom(format!("Invalid provider domain URL \"{url}\": {error}"))
        })
}

impl AuthInfo {
    #[cfg(any(test, feature = "testing"))]
    pub fn test_example() -> Self {
        Self {
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
        any::<(String, proptest_http::ArbitraryUri)>().prop_filter_map(
            "String and URI weren't valid AuthInfo",
            |(s, uri)| {
                IssuerUrl::new(format!("{}", uri.0))
                    .map(|domain| Self {
                        application_id: s,
                        domain,
                    })
                    .ok()
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::auth::AuthInfo;

    #[test]
    fn test_auth_info_https_prefix() {
        let info: AuthInfo =
            serde_json::from_str(r#"{"applicationID": "123", "domain": "example.com"}"#).unwrap();
        assert_eq!(info.domain.to_string(), "https://example.com");
    }

    #[test]
    fn test_auth_info_http_fails() {
        serde_json::from_str::<AuthInfo>(
            r#"{"applicationID": "123", "domain": "http://example.com"}"#,
        )
        .unwrap_err();
    }

    #[test]
    fn test_auth_info_http_localhost() {
        let info: AuthInfo = serde_json::from_str::<AuthInfo>(
            r#"{"applicationID": "123", "domain": "http://localhost:3211"}"#,
        )
        .unwrap();
        assert_eq!(info.domain.to_string(), "http://localhost:3211");

        let info: AuthInfo = serde_json::from_str::<AuthInfo>(
            r#"{"applicationID": "123", "domain": "http://127.0.0.1:3211"}"#,
        )
        .unwrap();
        assert_eq!(info.domain.to_string(), "http://127.0.0.1:3211");

        // fails because host is not localhost
        serde_json::from_str::<AuthInfo>(
            r#"{"applicationID": "123", "domain": "http://localhost.foo.com:3211"}"#,
        )
        .unwrap_err();
    }
}
