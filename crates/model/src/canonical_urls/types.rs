use common::http::RequestDestination;
use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct CanonicalUrl {
    pub request_destination: RequestDestination,
    pub url: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SerializedCanonicalUrl {
    request_destination: String,
    url: String,
}

impl From<CanonicalUrl> for SerializedCanonicalUrl {
    fn from(value: CanonicalUrl) -> Self {
        Self {
            request_destination: match value.request_destination {
                RequestDestination::ConvexCloud => "convexCloud".to_string(),
                RequestDestination::ConvexSite => "convexSite".to_string(),
            },
            url: value.url,
        }
    }
}

impl TryFrom<SerializedCanonicalUrl> for CanonicalUrl {
    type Error = anyhow::Error;

    fn try_from(value: SerializedCanonicalUrl) -> Result<Self, Self::Error> {
        Ok(Self {
            request_destination: match value.request_destination.as_str() {
                "convexCloud" => RequestDestination::ConvexCloud,
                "convexSite" => RequestDestination::ConvexSite,
                _ => anyhow::bail!("Invalid request destination: {}", value.request_destination),
            },
            url: value.url,
        })
    }
}

codegen_convex_serialization!(CanonicalUrl, SerializedCanonicalUrl);
