//! Common "non-privileged" identity type.

use std::{
    fmt::{
        self,
        Display,
    },
    ops::Deref,
    str::FromStr,
};

#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use sync_types::{
    UserIdentifier,
    UserIdentityAttributes,
};
use value::heap_size::HeapSize;

use crate::types::MemberId;

/// An "inert" version of [`keybroker::broker::Identity`] that doesn't bestow
/// any powers by virtue of ownership. This is used when persisting execution
/// state so that that authorization doesn't leak. It should not be possible to
/// turn an InertIdentity into Identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum InertIdentity {
    /// Admin for an instance.
    InstanceAdmin(String),
    /// System admin.
    System,
    /// Unknown.
    Unknown,
    User(UserIdentifier),
    ActingUser(MemberId, UserIdentifier),
}

impl InertIdentity {
    pub fn user_identifier(&self) -> Option<&UserIdentifier> {
        match self {
            InertIdentity::User(identifier) | InertIdentity::ActingUser(_, identifier) => {
                Some(identifier)
            },
            _ => None,
        }
    }
}

impl HeapSize for InertIdentity {
    fn heap_size(&self) -> usize {
        match self {
            InertIdentity::InstanceAdmin(s) => s.heap_size(),
            InertIdentity::System => 0,
            InertIdentity::Unknown => 0,
            InertIdentity::User(u) => u.0.heap_size(),
            InertIdentity::ActingUser(_m, u) => u.0.heap_size(),
        }
    }
}

// This type is different from InertIdentity because it requires knowledge of
// the user's underlying attributes to correctly invalidate queries when a
// user's attributes (e.g. email) change. Meanwhile InertIdentity only
// identifies users by their UserIdentifier for simplicity in serialization.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum IdentityCacheKey {
    /// Admin for an instance.
    InstanceAdmin(String),
    /// System admin.
    System,
    /// Unknown.
    Unknown,
    User(UserIdentityAttributes),
    ActingUser(MemberId, UserIdentityAttributes),
}

impl HeapSize for IdentityCacheKey {
    fn heap_size(&self) -> usize {
        match self {
            IdentityCacheKey::InstanceAdmin(s) => s.heap_size(),
            IdentityCacheKey::System => 0,
            IdentityCacheKey::Unknown => 0,
            IdentityCacheKey::User(u) => u.heap_size(),
            IdentityCacheKey::ActingUser(_m, u) => u.heap_size(),
        }
    }
}

#[cfg(any(test, feature = "testing"))]
impl Arbitrary for InertIdentity {
    // If your strategy function takes parameters, use a tuple or something to be
    // able to pass them along
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = InertIdentity>;

    fn arbitrary_with(_: ()) -> Self::Strategy {
        prop_oneof![
            Just(InertIdentity::System),
            Just(InertIdentity::Unknown),
            // Hardcode the InstanceAdmin identity for testing purposes
            // because the stringified identities should not contain ":" symbols,
            // conflicting with string serialization delimiters.
            "AdminIdentity".prop_map(InertIdentity::InstanceAdmin),
            (any::<UserIdentifier>()).prop_map(InertIdentity::User),
            (any::<MemberId>(), any::<UserIdentifier>())
                .prop_map(|(a, b)| InertIdentity::ActingUser(a, b)),
        ]
    }
}

impl FromStr for InertIdentity {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "system" {
            return Ok(InertIdentity::System);
        }
        if s == "unknown" {
            return Ok(InertIdentity::Unknown);
        }
        let mut parts = s.splitn(2, ':');
        match (parts.next(), parts.next()) {
            (Some("admin"), Some(s)) => Ok(InertIdentity::InstanceAdmin(s.to_string())),
            (Some("user"), Some(s)) => Ok(InertIdentity::User(UserIdentifier(s.to_string()))),
            (Some("impersonated_user"), Some(admin_id_and_user_id)) => {
                let mut parts = admin_id_and_user_id.splitn(2, ':');
                if let (Some(admin_id), Some(user_id)) = (parts.next(), parts.next()) {
                    Ok(InertIdentity::ActingUser(
                        MemberId(admin_id.parse()?),
                        UserIdentifier(user_id.to_string()),
                    ))
                } else {
                    anyhow::bail!("Missing instance in identity string {}", s);
                }
            },
            (_, Some(_)) => anyhow::bail!("Unrecognized identity type {}", s),
            _ => anyhow::bail!("Missing instance in identity string {}", s),
        }
    }
}

impl Display for InertIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InertIdentity::InstanceAdmin(s) => write!(f, "admin:{}", s),
            InertIdentity::System => write!(f, "system"),
            InertIdentity::Unknown => write!(f, "unknown"),
            InertIdentity::User(id) => write!(f, "user:{}", id.deref()),
            InertIdentity::ActingUser(admin_id, id) => {
                write!(f, "impersonated_user:{}:{}", admin_id, id.deref())
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::InertIdentity;
    fn assert_identity_string_roundtrips(left: String) {
        let right = InertIdentity::from_str(&left).unwrap().to_string();
        assert_eq!(left, right);
    }

    // backwards compatability test to litmus check that some strings
    // still correctly deserialize to InertIdentity
    #[test]
    fn test_backwards_compatability_roundtrip() {
        assert_identity_string_roundtrips("system".to_string());
        assert_identity_string_roundtrips("unknown".to_string());
        assert_identity_string_roundtrips("admin:AdminIdentifier".to_string());
        assert_identity_string_roundtrips("user:UserIdentifier".to_string());
    }
}
