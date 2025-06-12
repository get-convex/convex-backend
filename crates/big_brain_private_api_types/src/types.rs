use std::{
    fmt::Display,
    str::FromStr,
};

use anyhow::Context;
use serde::{
    Deserialize,
    Serialize,
};
use tuple_struct::{
    tuple_struct_string,
    tuple_struct_u64,
};

tuple_struct_u64!(SubscriptionId);
tuple_struct_u64!(CloudBackupId);
tuple_struct_u64!(MemberEmailId);
tuple_struct_u64!(PartitionId);

tuple_struct_string!(AccessToken);
tuple_struct_string!(DeviceName);
tuple_struct_string!(AppName);
tuple_struct_string!(ProjectSlug);
tuple_struct_string!(ProjectName);
tuple_struct_string!(TeamName);
tuple_struct_string!(TeamSlug);
tuple_struct_string!(ReferralCode);
tuple_struct_string!(VercelTeamName);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlanId {
    ConvexBase,
    ConvexStarterPlus,
    ConvexProfessional,
}

impl Display for PlanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self).unwrap();
        write!(f, "{}", s.as_str().unwrap())
    }
}

impl FromStr for PlanId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_owned()))
            .with_context(|| format!("Failed to parse plan id: {s}"))
    }
}

impl PlanId {
    pub fn is_in_orb(&self) -> bool {
        match self {
            PlanId::ConvexBase => false,
            PlanId::ConvexStarterPlus | PlanId::ConvexProfessional => true,
        }
    }

    pub fn supports_referrals(&self) -> bool {
        match self {
            PlanId::ConvexBase => true,
            PlanId::ConvexStarterPlus | PlanId::ConvexProfessional => false,
        }
    }

    pub fn gets_pro_resources(&self) -> bool {
        match self {
            PlanId::ConvexBase | PlanId::ConvexStarterPlus => false,
            PlanId::ConvexProfessional => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_id_display() {
        let plans = [
            (PlanId::ConvexBase, "CONVEX_BASE"),
            (PlanId::ConvexStarterPlus, "CONVEX_STARTER_PLUS"),
            (PlanId::ConvexProfessional, "CONVEX_PROFESSIONAL"),
        ];
        for (plan, expected) in plans {
            assert_eq!(plan.to_string(), expected);
            assert_eq!(plan, expected.parse::<PlanId>().unwrap());
        }
    }
}
