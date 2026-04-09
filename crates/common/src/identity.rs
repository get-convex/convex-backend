//! Common "non-privileged" identity type.

use std::{
    fmt::{
        self,
        Display,
    },
    ops::Deref,
    str::FromStr,
};

use metrics::StaticMetricLabel;
use sync_types::{
    UserIdentifier,
    UserIdentityAttributes,
};
use value::heap_size::HeapSize;

use crate::types::{
    MemberId,
    TeamId,
};

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
    MemberActingUser(MemberId, UserIdentifier),
    TeamActingUser(TeamId, UserIdentifier),
}

pub const IDENTITY_LABEL: &str = "identity";
impl InertIdentity {
    pub fn user_identifier(&self) -> Option<&UserIdentifier> {
        match self {
            InertIdentity::User(identifier)
            | InertIdentity::MemberActingUser(_, identifier)
            | InertIdentity::TeamActingUser(_, identifier) => Some(identifier),
            _ => None,
        }
    }

    pub fn tag(&self) -> StaticMetricLabel {
        let type_str = match self {
            InertIdentity::System => "system",
            InertIdentity::InstanceAdmin(_) => "instance_admin",
            InertIdentity::Unknown => "unknown",
            InertIdentity::User(_) => "user",
            InertIdentity::MemberActingUser(..) => "member_acting_user",
            InertIdentity::TeamActingUser(..) => "team_acting_user",
        };
        StaticMetricLabel::new(IDENTITY_LABEL, type_str)
    }
}

impl HeapSize for InertIdentity {
    fn heap_size(&self) -> usize {
        match self {
            InertIdentity::InstanceAdmin(s) => s.heap_size(),
            InertIdentity::System => 0,
            InertIdentity::Unknown => 0,
            InertIdentity::User(u) => u.0.heap_size(),
            InertIdentity::MemberActingUser(_m, u) => u.0.heap_size(),
            InertIdentity::TeamActingUser(_t, u) => u.0.heap_size(),
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
    Unknown(Option<String>),
    User(UserIdentityAttributes),
}

impl HeapSize for IdentityCacheKey {
    fn heap_size(&self) -> usize {
        match self {
            IdentityCacheKey::InstanceAdmin(s) => s.heap_size(),
            IdentityCacheKey::System => 0,
            IdentityCacheKey::Unknown(s) => s.heap_size(),
            IdentityCacheKey::User(u) => u.heap_size(),
        }
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
            (Some("impersonated_user"), Some(admin_id_and_user_id))
            | (Some("member_acting_as_user"), Some(admin_id_and_user_id)) => {
                let mut parts = admin_id_and_user_id.splitn(2, ':');
                let (Some(admin_id), Some(user_id)) = (parts.next(), parts.next()) else {
                    anyhow::bail!("Missing instance in identity string {s}");
                };
                Ok(InertIdentity::MemberActingUser(
                    MemberId(admin_id.parse()?),
                    UserIdentifier(user_id.to_string()),
                ))
            },
            (Some("team_acting_as_user"), Some(team_id_and_user_id)) => {
                let mut parts = team_id_and_user_id.splitn(2, ':');
                let (Some(team_id), Some(user_id)) = (parts.next(), parts.next()) else {
                    anyhow::bail!("Missing instance in identity string {s}");
                };
                Ok(InertIdentity::TeamActingUser(
                    TeamId(team_id.parse()?),
                    UserIdentifier(user_id.to_string()),
                ))
            },
            (_, Some(_)) => anyhow::bail!("Unrecognized identity type {s}"),
            _ => anyhow::bail!("Missing instance in identity string {s}"),
        }
    }
}

impl Display for InertIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InertIdentity::InstanceAdmin(s) => write!(f, "admin:{s}"),
            InertIdentity::System => write!(f, "system"),
            InertIdentity::Unknown => write!(f, "unknown"),
            InertIdentity::User(id) => write!(f, "user:{}", id.deref()),
            InertIdentity::MemberActingUser(member_id, id) => {
                write!(f, "member_acting_as_user:{}:{}", member_id, id.deref())
            },
            InertIdentity::TeamActingUser(team_id, id) => {
                write!(f, "team_acting_as_user:{}:{}", team_id, id.deref())
            },
        }
    }
}
