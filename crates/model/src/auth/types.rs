use std::collections::{
    BTreeMap,
    BTreeSet,
};

use common::auth::AuthInfo;
use openidconnect::IssuerUrl;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use value::{
    obj,
    ConvexObject,
    ConvexValue,
};

/// Persisted version of AuthInfo that impls try_from to ConvexObject
/// Ideally this remains local to this crate (has to be pub for db-info)
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct AuthInfoPersisted(pub AuthInfo);

impl TryFrom<ConvexObject> for AuthInfoPersisted {
    type Error = anyhow::Error;

    fn try_from(o: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = o.into();
        let application_id = match fields.remove("applicationID") {
            Some(ConvexValue::String(s)) => s.into(),
            _ => anyhow::bail!("Missing or invalid applicationID field for AuthInfo"),
        };
        let domain = match fields.remove("domain") {
            Some(ConvexValue::String(s)) => IssuerUrl::new(s.into())?,
            _ => anyhow::bail!("Missing or invalid domain field for AuthInfo"),
        };
        Ok(Self(AuthInfo {
            application_id,
            domain,
        }))
    }
}

impl TryFrom<AuthInfoPersisted> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(info: AuthInfoPersisted) -> Result<Self, Self::Error> {
        obj!(
            "applicationID" => info.0.application_id,
            "domain" => info.0.domain.to_string(),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, PartialEq)
)]
pub struct AuthDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

impl AuthDiff {
    pub fn new(added: BTreeSet<AuthInfo>, removed: BTreeSet<AuthInfo>) -> anyhow::Result<Self> {
        let added_strings = added
            .into_iter()
            .map(|auth_info| {
                let auth_info_obj = ConvexObject::try_from(AuthInfoPersisted(auth_info))?;
                let auth_info_json: JsonValue = auth_info_obj.into();
                anyhow::Ok(auth_info_json.to_string())
            })
            .try_collect::<Vec<String>>()?;

        let removed_strings = removed
            .into_iter()
            .map(|auth_info| {
                let auth_info_obj = ConvexObject::try_from(AuthInfoPersisted(auth_info))?;
                let auth_info_json: JsonValue = auth_info_obj.into();
                anyhow::Ok(auth_info_json.to_string())
            })
            .try_collect::<Vec<String>>()?;

        Ok(Self {
            added: added_strings,
            removed: removed_strings,
        })
    }
}
