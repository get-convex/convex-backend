use convex_sync_types::{
    UserIdentifier,
    UserIdentityAttributes,
};

use crate::convex_identity::UserIdentityAttributes as UserIdentityAttributesProto;

impl TryFrom<UserIdentityAttributesProto> for UserIdentityAttributes {
    type Error = anyhow::Error;

    fn try_from(
        UserIdentityAttributesProto {
            token_identifier,
            issuer,
            subject,
            name,
            given_name,
            family_name,
            nickname,
            preferred_username,
            profile_url,
            picture_url,
            website_url,
            email,
            email_verified,
            gender,
            birthday,
            timezone,
            language,
            phone_number,
            phone_number_verified,
            address,
            updated_at,
        }: UserIdentityAttributesProto,
    ) -> anyhow::Result<UserIdentityAttributes> {
        let token_identifier =
            token_identifier.ok_or_else(|| anyhow::anyhow!("Missing token_identifier"))?;
        let token_identifier = UserIdentifier(token_identifier);
        Ok(UserIdentityAttributes {
            token_identifier,
            issuer,
            subject,
            name,
            given_name,
            family_name,
            nickname,
            preferred_username,
            profile_url,
            picture_url,
            website_url,
            email,
            email_verified,
            gender,
            birthday,
            timezone,
            language,
            phone_number,
            phone_number_verified,
            address,
            updated_at,
        })
    }
}

impl From<UserIdentityAttributes> for UserIdentityAttributesProto {
    fn from(
        UserIdentityAttributes {
            token_identifier,
            subject,
            issuer,
            name,
            given_name,
            family_name,
            nickname,
            preferred_username,
            profile_url,
            picture_url,
            website_url,
            email,
            email_verified,
            gender,
            birthday,
            timezone,
            language,
            phone_number,
            phone_number_verified,
            address,
            updated_at,
        }: UserIdentityAttributes,
    ) -> UserIdentityAttributesProto {
        UserIdentityAttributesProto {
            token_identifier: Some(token_identifier.to_string()),
            issuer,
            subject,
            name,
            given_name,
            family_name,
            nickname,
            preferred_username,
            profile_url,
            picture_url,
            website_url,
            email,
            email_verified,
            gender,
            birthday,
            timezone,
            language,
            phone_number,
            phone_number_verified,
            address,
            updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use convex_sync_types::UserIdentityAttributes;
    use proptest::prelude::*;
    use value::testing::assert_roundtrips;

    use crate::user_identity_attributes::UserIdentityAttributesProto;
    proptest! {
        #![proptest_config(ProptestConfig {
            failure_persistence: None, ..ProptestConfig::default()
        })]
        #[test]
        fn test_attributes_proto_roundtrips(attributes in any::<UserIdentityAttributes>()) {
            assert_roundtrips::<UserIdentityAttributes, UserIdentityAttributesProto>(attributes);
        }
    }
}
