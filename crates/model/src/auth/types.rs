use std::collections::{
    BTreeMap,
    BTreeSet,
};

use common::auth::AuthInfo;
use openidconnect::IssuerUrl;
use serde_json::Value as JsonValue;
use value::{
    obj,
    remove_vec_of_strings,
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

#[derive(Debug, Clone)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, PartialEq, Default)
)]
pub struct AuthDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

impl TryFrom<AuthDiff> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: AuthDiff) -> Result<Self, Self::Error> {
        let added_values: Vec<ConvexValue> = value
            .added
            .into_iter()
            .map(ConvexValue::try_from)
            .try_collect::<Vec<ConvexValue>>()?;
        let removed_values: Vec<ConvexValue> = value
            .removed
            .into_iter()
            .map(ConvexValue::try_from)
            .try_collect::<Vec<ConvexValue>>()?;
        obj!("added" => added_values, "removed" => removed_values)
    }
}

impl TryFrom<ConvexObject> for AuthDiff {
    type Error = anyhow::Error;

    fn try_from(obj: ConvexObject) -> anyhow::Result<Self> {
        let mut fields = BTreeMap::from(obj);
        Ok(Self {
            added: remove_vec_of_strings(&mut fields, "added")?,
            removed: remove_vec_of_strings(&mut fields, "removed")?,
        })
    }
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

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use common::testing::assert_roundtrips;
    use proptest::prelude::*;
    use value::ConvexObject;

    use crate::auth::types::{
        AuthDiff,
        AuthInfoPersisted,
    };

    proptest! {
        #![proptest_config(ProptestConfig { cases: 16 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]
        #[test]
        fn test_auth_info_roundtrips(v in any::<AuthInfoPersisted>()) {
            assert_roundtrips::<AuthInfoPersisted, ConvexObject>(v);
        }

        #[test]
        fn test_auth_diff_to_object(v in any::<AuthDiff>()) {
            ConvexObject::try_from(v).unwrap();
        }
    }
}
