use std::{
    collections::{
        BTreeMap,
        HashMap,
    },
    fmt,
    time::{
        Duration,
        SystemTime,
    },
};

use anyhow::Context;
use biscuit::JWT;
pub use common::types::SystemKey;
use common::{
    components::ComponentId,
    identity::{
        IdentityCacheKey,
        InertIdentity,
    },
    index::IndexKeyBytes,
    query::{
        Cursor,
        CursorPosition,
        SerializedCursor,
    },
    query_journal::QueryJournal,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    types::{
        format_admin_key,
        remove_type_prefix_from_instance_name,
        split_admin_key,
        ActionCallbackToken,
        AdminKey,
        MemberId,
        PersistenceVersion,
        TeamId,
    },
};
use errors::ErrorMetadata;
use metrics::StaticMetricLabel;
use openidconnect::{
    core::{
        CoreGenderClaim,
        CoreIdTokenVerifier,
        CoreJweContentEncryptionAlgorithm,
        CoreJwsSigningAlgorithm,
    },
    AdditionalClaims,
    IdToken,
    Nonce,
};
use pb::{
    convex_actions::ActionCallbackToken as ActionCallbackTokenProto,
    convex_cursor::{
        instance_cursor::Position as PositionProto,
        IndexKey as IndexKeyProto,
        InstanceCursor as InstanceCursorProto,
    },
    convex_identity::{
        unchecked_identity::Identity as UncheckedIdentityProto,
        ActingUser,
        DeploymentOperation as ProtoDeploymentOperation,
        UnknownIdentity,
    },
    convex_keys::{
        admin_key::Identity as AdminIdentityProto,
        storage_token::{
            AuthorizationType as AuthorizationTypeProto,
            StoreFile as StoreFileProto,
        },
        AdminKey as AdminKeyProto,
        StorageToken as StorageTokenProto,
    },
    convex_query_journal::InstanceQueryJournal as InstanceQueryJournalProto,
};
use serde::{
    Deserialize,
    Serialize,
};
use sync_types::{
    AuthenticationToken,
    SerializedQueryJournal,
    UserIdentifier,
    UserIdentityAttributes,
};

use crate::{
    encryptor::{
        DeterministicEncryptor,
        Purpose,
        RandomEncryptor,
    },
    legacy_encryptor::LegacyEncryptor,
    metrics::{
        log_actions_token_expired,
        log_legacy_admin_key,
        log_store_file_auth_expired,
    },
    operations::{
        bad_admin_key_error,
        DeploymentOp,
    },
    secret::DeploymentSecret,
};

const ACTION_KEY_VERSION: u8 = 2;
const ADMIN_KEY_VERSION: u8 = 1;
const CURSOR_VERSION: u8 = 7;
const STORE_FILE_AUTHZ_VERSION: u8 = 1;
const QUERY_JOURNAL_VERSION: u8 = 7;

// Max delay from transaction start time -> key being issued that is tolerable.
const MAX_TS_DELAY: Duration = Duration::from_secs(15);

#[derive(Clone)]
pub struct KeyBroker {
    instance_name: String,
    encryptor: LegacyEncryptor,
    admin_key_encryptor: RandomEncryptor,
    action_callback_encryptor: RandomEncryptor,
    cursor_encryptor: DeterministicEncryptor,
    journal_encryptor: RandomEncryptor,
    store_file_encryptor: RandomEncryptor,
}

// This enum encodes a successful authentication decision, and its nontrivial
// variants cannot be constructed outside this crate. Since possession of this
// value confers access permissions, don't store it persistently: Instead, use
// [`common::identity::InertIdentity`] to store an "inert" version that records
// the variant without representation authentication.
#[derive(Clone, Debug)]
pub enum Identity {
    InstanceAdmin(AdminIdentity),
    System(SystemIdentity),
    User(UserIdentity),
    // ActingUser keeps track of the ID of the admin acting as a user,
    // and that user's fake attributes
    ActingUser(AdminIdentity, UserIdentityAttributes),
    // Unknown(None) means no identity was provided.
    // Unknown(Some(error_message)) means an error occurred while parsing the identity.
    // We allow the request to go through, but keep the error to throw when code tries to
    // access the identity (eg ctx.getUserIdentity())
    Unknown(Option<ErrorMetadata>), // include an optional error message
}

impl From<Identity> for AuthenticationToken {
    fn from(i: Identity) -> Self {
        match i {
            Identity::User(identity) => AuthenticationToken::User(identity.original_token),
            Identity::ActingUser(identity, user) => {
                AuthenticationToken::Admin(identity.key, Some(user))
            },
            Identity::InstanceAdmin(identity) => AuthenticationToken::Admin(identity.key, None),
            _ => AuthenticationToken::None,
        }
    }
}

impl From<Identity> for pb::convex_identity::UncheckedIdentity {
    fn from(i: Identity) -> Self {
        let identity = match i {
            Identity::InstanceAdmin(admin_identity) => {
                UncheckedIdentityProto::AdminIdentity(admin_identity.into())
            },
            Identity::System(_) => UncheckedIdentityProto::System(()),
            Identity::User(user_identity) => {
                UncheckedIdentityProto::UserIdentity(user_identity.into())
            },
            Identity::ActingUser(admin_identity, attributes) => {
                UncheckedIdentityProto::ActingUser(ActingUser {
                    admin_identity: Some(admin_identity.into()),
                    attributes: Some(attributes.into()),
                })
            },
            Identity::Unknown(error_message) => UncheckedIdentityProto::Unknown(UnknownIdentity {
                error_message: error_message.map(|e| e.into()),
            }),
        };
        Self {
            identity: Some(identity),
        }
    }
}

impl Identity {
    pub fn from_proto_unchecked(
        msg: pb::convex_identity::UncheckedIdentity,
    ) -> anyhow::Result<Self> {
        let identity = msg
            .identity
            .ok_or_else(|| anyhow::anyhow!("Missing nested identity"))?;
        match identity {
            UncheckedIdentityProto::AdminIdentity(admin_identity) => Ok(Identity::InstanceAdmin(
                AdminIdentity::from_proto_unchecked(admin_identity)?,
            )),
            UncheckedIdentityProto::System(()) => Ok(Identity::System(SystemIdentity)),
            UncheckedIdentityProto::UserIdentity(user_identity) => Ok(Identity::User(
                UserIdentity::from_proto_unchecked(user_identity)?,
            )),
            UncheckedIdentityProto::ActingUser(ActingUser {
                admin_identity,
                attributes,
            }) => {
                let admin_identity = AdminIdentity::from_proto_unchecked(
                    admin_identity.ok_or_else(|| anyhow::anyhow!("Missing admin identity"))?,
                )?;
                let attributes =
                    attributes.ok_or_else(|| anyhow::anyhow!("Missing user attributes"))?;
                Ok(Identity::ActingUser(admin_identity, attributes.try_into()?))
            },
            UncheckedIdentityProto::Unknown(UnknownIdentity { error_message }) => Ok(
                Identity::Unknown(error_message.map(|e| e.try_into()).transpose()?),
            ),
        }
    }

    pub fn tag(&self) -> StaticMetricLabel {
        InertIdentity::from(self.clone()).tag()
    }
}

impl From<Identity> for InertIdentity {
    fn from(i: Identity) -> Self {
        match i {
            Identity::InstanceAdmin(i) => InertIdentity::InstanceAdmin(i.instance_name),
            Identity::System(_) => InertIdentity::System,
            Identity::Unknown(_) => InertIdentity::Unknown,
            Identity::User(user) => InertIdentity::User(user.attributes.token_identifier),
            Identity::ActingUser(identity, user) => match identity.principal {
                AdminIdentityPrincipal::Member(member_id) => {
                    InertIdentity::MemberActingUser(member_id, user.token_identifier)
                },
                AdminIdentityPrincipal::Team(team_id) => {
                    InertIdentity::TeamActingUser(team_id, user.token_identifier)
                },
            },
        }
    }
}

impl PartialEq for Identity {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::InstanceAdmin(l), Self::InstanceAdmin(r)) => l == r,
            (Self::System(..), Self::System(..)) => true,
            (Self::User(l), Self::User(r)) => {
                l.attributes.token_identifier == r.attributes.token_identifier
            },
            (Self::Unknown(_), Self::Unknown(_)) => true,
            (
                Self::ActingUser(l_admin_identity, l_attributes),
                Self::ActingUser(r_admin_identity, r_attributes),
            ) => l_admin_identity == r_admin_identity && l_attributes == r_attributes,
            (Self::InstanceAdmin(_), _)
            | (Self::System(_), _)
            | (Self::User(_), _)
            | (Self::Unknown(_), _)
            | (Self::ActingUser(..), _) => false,
        }
    }
}

impl Eq for Identity {}

impl Identity {
    pub fn cache_key(&self) -> IdentityCacheKey {
        match self.clone() {
            Identity::InstanceAdmin(i) => IdentityCacheKey::InstanceAdmin(i.instance_name),
            Identity::System(_) => IdentityCacheKey::System,
            Identity::Unknown(error_message) => {
                IdentityCacheKey::Unknown(error_message.map(|e| e.to_string()))
            },
            Identity::User(user) => IdentityCacheKey::User(user.attributes),
            // Identity of the impersonator not relevant for caching. Only the one being
            // impersonated.
            Identity::ActingUser(_identity, user) => IdentityCacheKey::User(user),
        }
    }

    /// Easy-to-audit entry point for creating a global system identity.
    pub fn system() -> Self {
        Identity::System(SystemIdentity)
    }

    /// Entry point for creating a User identity after authenticating the user.
    pub fn user(user: UserIdentity) -> Self {
        Identity::User(user)
    }

    pub fn is_system(&self) -> bool {
        matches!(self, Identity::System(..))
    }

    pub fn is_admin(&self) -> bool {
        matches!(self, Identity::InstanceAdmin(..))
    }

    pub fn is_acting_as_user(&self) -> bool {
        matches!(self, Identity::ActingUser(..))
    }

    pub fn is_user(&self) -> bool {
        matches!(self, Identity::User(..))
    }

    /// Returns the admin's [`MemberId`] if this is an
    /// [`Identity::InstanceAdmin`] with a member principal
    pub fn member_id(&self) -> Option<MemberId> {
        if let Identity::InstanceAdmin(AdminIdentity { principal, .. }) = self {
            return if let AdminIdentityPrincipal::Member(member_id) = principal {
                Some(*member_id)
            } else {
                None
            };
        }
        None
    }

    pub fn instance_admin_principal(&self) -> Option<AdminIdentityPrincipal> {
        if let Identity::InstanceAdmin(AdminIdentity { principal, .. }) = self {
            return Some(principal.clone());
        }
        None
    }

    pub fn instance_name(&self) -> Option<String> {
        if let Identity::InstanceAdmin(AdminIdentity { instance_name, .. }) = self {
            return Some(instance_name.to_string());
        }
        None
    }

    pub fn user_identity(&self) -> Option<UserIdentity> {
        if let Identity::User(id) = self {
            return Some(id.clone());
        }
        None
    }

    /// Check that this identity is an admin allowed to perform `operation`.
    /// System identities are always allowed. Admin identities are checked
    /// against their allowed operations. All other identities are rejected.
    pub fn require_operation(&self, operation: DeploymentOp) -> anyhow::Result<()> {
        let admin_identity = match self {
            Identity::System(_) => return Ok(()),
            Identity::InstanceAdmin(admin_identity) | Identity::ActingUser(admin_identity, _) => {
                admin_identity
            },
            Identity::User(_) | Identity::Unknown(_) => {
                return Err(bad_admin_key_error(self.instance_name()).into());
            },
        };
        if !admin_identity.is_operation_allowed(operation) {
            anyhow::bail!(ErrorMetadata::forbidden(
                "Unauthorized",
                format!("You do not have permission to perform this operation ({operation:?})."),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive] // Prevents creating this struct without calling `from_token`
pub struct UserIdentity {
    pub subject: String,
    // Might be useful for developers to know which provider authenticated this user.
    pub issuer: String,
    pub expiration: SystemTime,
    pub attributes: UserIdentityAttributes,
    // The original token this user identity was created from. This may either by an
    // OIDC JWT or a custom JWT.
    pub original_token: String,
}

impl From<UserIdentity> for pb::convex_identity::UserIdentity {
    fn from(
        UserIdentity {
            subject,
            issuer,
            expiration,
            attributes,
            original_token,
        }: UserIdentity,
    ) -> Self {
        Self {
            subject: Some(subject),
            issuer: Some(issuer),
            expiration: Some(expiration.into()),
            attributes: Some(attributes.into()),
            original_token: Some(original_token),
        }
    }
}

macro_rules! get_string {
    ($claims: ident, $field: ident) => {
        $claims.$field().map(|v| v.to_string())
    };
}
macro_rules! get_localized_string {
    ($claims: ident, $field: ident) => {
        $claims
            .$field()
            .and_then(|loc| loc.get(None))
            .map(|v| v.to_string())
    };
}

pub type CoreIdTokenWithCustomClaims = IdToken<
    CustomClaims,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJwsSigningAlgorithm,
>;

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct CustomClaims(HashMap<String, serde_json::Value>);
impl AdditionalClaims for CustomClaims {}

impl UserIdentity {
    pub fn from_custom_jwt(
        token: JWT<serde_json::Value, biscuit::Empty>,
        original_token: String,
    ) -> Result<Self, anyhow::Error> {
        let payload = token.payload()?;
        let subject = payload.registered.subject.as_ref().ok_or_else(|| {
            ErrorMetadata::unauthenticated("InvalidAuthHeader", "Missing subject")
        })?;
        let issuer =
            payload.registered.issuer.as_ref().ok_or_else(|| {
                ErrorMetadata::unauthenticated("InvalidAuthHeader", "Missing issuer")
            })?;
        let Some(expiry) = payload.registered.expiry else {
            anyhow::bail!(ErrorMetadata::unauthenticated(
                "InvalidAuthHeader",
                "Missing expiry"
            ));
        };
        let mut custom_claims = BTreeMap::new();
        if let serde_json::Value::Object(ref properties) = payload.private {
            custom_claims = extract_custom_jwt_claims(properties);
        }
        Ok(UserIdentity {
            subject: subject.clone(),
            issuer: issuer.clone(),
            expiration: (*expiry).into(),
            attributes: UserIdentityAttributes {
                token_identifier: UserIdentifier::construct(issuer, subject),
                subject: Some(subject.clone()),
                issuer: Some(issuer.clone()),
                custom_claims,
                ..Default::default()
            },
            original_token,
        })
    }

    pub fn from_token(
        token: CoreIdTokenWithCustomClaims,
        verifier: CoreIdTokenVerifier,
    ) -> Result<Self, anyhow::Error> {
        // NB: Nonce verification is optional, and we'd need the developer to create and
        // store a nonce with their initial request to auth0 (or whatever
        // provider they use) and provide that nonce to us with the ID token.
        let nonce_verifier = |_: Option<&Nonce>| Ok(());
        let binding = token.clone();
        let claims = binding.claims(&verifier, nonce_verifier)?;
        let subject = claims.subject().to_string();
        let issuer = claims.issuer().to_string();
        let mut custom_claims = BTreeMap::new();
        for claim in &claims.additional_claims().0 {
            // Filter out standard claims and claims set by auth providers
            match claim.0.as_str() {
                // Standard claims that we support: see https://docs.convex.dev/api/interfaces/server.UserIdentity
                "sub"
                | "iss"
                | "exp"
                | "name"
                | "given_name"
                | "family_name"
                | "nickname"
                | "preferred_username"
                | "profile"
                | "picture"
                | "website"
                | "email"
                | "email_verified"
                | "gender"
                | "birthdate"
                | "zoneinfo"
                | "locale"
                | "phone_number"
                | "phone_number_verified"
                | "address"
                | "updated_at"
                // Clerk claims: see https://clerk.com/docs/backend-requests/resources/session-tokens
                | "jti" | "nbf" => {
                    continue;
                },
                _ => {
                    custom_claims.insert(claim.0.to_string(), claim.1.to_string());
                },
            }
        }
        Ok(UserIdentity {
            subject: subject.clone(),
            issuer: issuer.clone(),
            expiration: claims.expiration().into(),
            original_token: token.to_string(),
            attributes: UserIdentityAttributes {
                token_identifier: UserIdentifier::construct(&issuer, &subject),
                subject: Some(subject),
                issuer: Some(issuer),
                name: get_localized_string!(claims, name),
                given_name: get_localized_string!(claims, given_name),
                family_name: get_localized_string!(claims, family_name),
                nickname: get_localized_string!(claims, nickname),
                preferred_username: get_string!(claims, preferred_username),
                profile_url: get_localized_string!(claims, profile),
                picture_url: get_localized_string!(claims, picture),
                website_url: get_localized_string!(claims, website),
                email: get_string!(claims, email),
                email_verified: claims.email_verified(),
                gender: get_string!(claims, gender),
                birthday: get_string!(claims, birthdate),
                timezone: get_string!(claims, zoneinfo),
                language: get_string!(claims, locale),
                phone_number: get_string!(claims, phone_number),
                phone_number_verified: claims.phone_number_verified(),
                address: claims
                    .address()
                    .and_then(|a| a.formatted.as_ref())
                    .map(|f| f.to_string()),
                updated_at: claims.updated_at().map(|dt| dt.to_rfc3339()),
                custom_claims,
            },
        })
    }

    // Decode an `Identity` serialized to protobuf *without* revalidating its
    // original token. This method assumes that the protobuf comes from a
    // trusted source, like an internal backend.
    pub fn from_proto_unchecked(msg: pb::convex_identity::UserIdentity) -> anyhow::Result<Self> {
        let subject = msg
            .subject
            .ok_or_else(|| anyhow::anyhow!("Missing subject"))?;
        let issuer = msg
            .issuer
            .ok_or_else(|| anyhow::anyhow!("Missing issuer"))?;
        let expiration = msg
            .expiration
            .ok_or_else(|| anyhow::anyhow!("Missing expiration"))?
            .try_into()?;
        let attributes = msg
            .attributes
            .ok_or_else(|| anyhow::anyhow!("Missing user identity attributes"))?
            .try_into()?;
        let original_token = msg
            .original_token
            .ok_or_else(|| anyhow::anyhow!("Missing original_token"))?
            .parse()?;
        Ok(Self {
            subject,
            issuer,
            expiration,
            attributes,
            original_token,
        })
    }

    pub fn is_expired(&self, current_time: SystemTime) -> bool {
        current_time >= self.expiration
    }
}

fn extract_custom_jwt_claims(
    payload: &serde_json::Map<String, serde_json::Value>,
) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    for (key, value) in payload {
        if let serde_json::Value::Object(nested_object) = value {
            for (nested_key, value) in extract_custom_jwt_claims(nested_object) {
                result.insert(format!("{key}.{nested_key}"), value);
            }
        } else {
            result.insert(key.clone(), value.to_string());
        }
    }
    result
}

use crate::operations::operations_for_deploy_key;

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub enum AdminIdentityPrincipal {
    Member(MemberId),
    Team(TeamId),
}

// Token indicating the possessor has authenticated as the admin for an
// instance.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AdminIdentity {
    instance_name: String,
    principal: AdminIdentityPrincipal,
    key: String,
    // is_read_only being true implies that this identity should not be able to write data.
    // At the function level, read only admins are allowed to run queries but not mutations and
    // actions. At the database level, they are allowed to read data from user and system tables
    // but not write to them.
    is_read_only: bool,
    // Operations this identity is allowed to perform. Empty means all operations allowed.
    allowed_ops: Vec<DeploymentOp>,
}

impl From<AdminIdentity> for pb::convex_identity::AdminIdentity {
    fn from(
        AdminIdentity {
            instance_name,
            principal,
            key,
            is_read_only,
            allowed_ops,
        }: AdminIdentity,
    ) -> Self {
        Self {
            instance_name: Some(instance_name),
            principal: match principal {
                AdminIdentityPrincipal::Member(member_id) => Some(
                    pb::convex_identity::admin_identity::Principal::MemberId(member_id.0),
                ),
                AdminIdentityPrincipal::Team(team_id) => Some(
                    pb::convex_identity::admin_identity::Principal::TeamId(team_id.0),
                ),
            },
            key: Some(key),
            is_read_only,
            allowed_operations: allowed_ops
                .into_iter()
                .map(|op| ProtoDeploymentOperation::from(op) as i32)
                .collect(),
        }
    }
}

impl AdminIdentity {
    pub fn from_proto_unchecked(msg: pb::convex_identity::AdminIdentity) -> anyhow::Result<Self> {
        let instance_name = msg
            .instance_name
            .ok_or_else(|| anyhow::anyhow!("Missing instance_name"))?;
        let principal = match msg.principal {
            Some(pb::convex_identity::admin_identity::Principal::MemberId(id)) => {
                AdminIdentityPrincipal::Member(id.into())
            },
            Some(pb::convex_identity::admin_identity::Principal::TeamId(id)) => {
                AdminIdentityPrincipal::Team(id.into())
            },
            None => anyhow::bail!("Missing principal"),
        };
        let key = msg.key.ok_or_else(|| anyhow::anyhow!("Missing key"))?;
        let is_read_only: bool = msg.is_read_only;
        let allowed_ops: Vec<DeploymentOp> = msg
            .allowed_operations
            .into_iter()
            .map(|i| match ProtoDeploymentOperation::try_from(i) {
                Ok(proto) => DeploymentOp::from(proto),
                Err(_) => DeploymentOp::Unknown,
            })
            .collect();
        Ok(Self {
            instance_name,
            principal,
            key,
            is_read_only,
            allowed_ops,
        })
    }

    pub fn new_for_access_token(
        instance_name: String,
        principal: AdminIdentityPrincipal,
        access_token: String,
        is_read_only: bool,
        allowed_ops: Vec<DeploymentOp>,
    ) -> Self {
        Self {
            instance_name,
            principal,
            key: access_token,
            is_read_only,
            allowed_ops,
        }
    }

    pub fn principal(&self) -> &AdminIdentityPrincipal {
        &self.principal
    }

    // is_read_only being true implies that this identity should not be able to
    // write data. At the function level, read only admins are allowed to run
    // queries but not mutations and actions. At the database level, they are
    // allowed to read data from user and system tables but not write to them.
    pub fn is_read_only(&self) -> bool {
        self.is_read_only
    }

    pub fn allowed_ops(&self) -> &[DeploymentOp] {
        &self.allowed_ops
    }

    /// Check whether this identity is allowed to perform a specific operation.
    /// Empty `allowed_ops` means all operations are allowed.
    pub fn is_operation_allowed(&self, operation: DeploymentOp) -> bool {
        self.allowed_ops.is_empty() || self.allowed_ops.contains(&operation)
    }
}

impl fmt::Debug for AdminIdentity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}/{:?}/{}",
            self.instance_name, self.principal, self.key
        )
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SystemIdentity;
impl fmt::Debug for SystemIdentity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SystemIdentity")
    }
}

/// Encrypted authorization to store a file
#[derive(Debug, derive_more::Display)]
pub struct StoreFileAuthorization(String);
/// Encrypted authorization to get a file
#[derive(Debug, derive_more::Display)]
pub struct GetFileAuthorization(String);

pub fn cursor_parse_error() -> ErrorMetadata {
    ErrorMetadata::bad_request("InvalidCursor", "Failed to parse cursor")
}

impl KeyBroker {
    pub fn new(instance_name: &str, deployment_secret: DeploymentSecret) -> anyhow::Result<Self> {
        Ok(Self {
            instance_name: instance_name.to_owned(),
            encryptor: LegacyEncryptor::new(deployment_secret)?,
            admin_key_encryptor: RandomEncryptor::derive_from_secret(
                &deployment_secret,
                Purpose::ADMIN_KEY,
            )?,
            action_callback_encryptor: RandomEncryptor::derive_from_secret(
                &deployment_secret,
                Purpose::ACTION_CALLBACK_TOKEN,
            )?,
            cursor_encryptor: DeterministicEncryptor::derive_from_secret(
                &deployment_secret,
                Purpose::CURSOR,
            )?,
            journal_encryptor: RandomEncryptor::derive_from_secret(
                &deployment_secret,
                Purpose::QUERY_JOURNAL,
            )?,
            store_file_encryptor: RandomEncryptor::derive_from_secret(
                &deployment_secret,
                Purpose::STORE_FILE_AUTHORIZATION,
            )?,
        })
    }

    pub fn dev() -> Self {
        Self::new(
            crate::DEV_INSTANCE_NAME,
            DeploymentSecret::try_from(crate::DEV_SECRET).unwrap(),
        )
        .unwrap()
    }

    pub fn local_dev(instance_name: &str) -> Self {
        Self::new(
            instance_name,
            DeploymentSecret::try_from(crate::DEV_SECRET).unwrap(),
        )
        .unwrap()
    }

    pub fn function_runner_keybroker(&self) -> FunctionRunnerKeyBroker {
        FunctionRunnerKeyBroker {
            instance_name: self.instance_name.clone(),
            cursor_encryptor: self.cursor_encryptor.clone(),
            store_file_encryptor: self.store_file_encryptor.clone(),
        }
    }

    pub fn issue_admin_key(&self, member_id: MemberId) -> AdminKey {
        AdminKey::new(self.issue_key(Some(member_id), false))
    }

    pub fn issue_read_only_admin_key(&self, member_id: MemberId) -> AdminKey {
        AdminKey::new(self.issue_key(Some(member_id), true))
    }

    pub fn issue_system_key(&self) -> SystemKey {
        SystemKey::new(self.issue_key(None, false))
    }

    pub fn issue_store_file_authorization<RT: Runtime>(
        &self,
        rt: &RT,
        issued: UnixTimestamp,
        component: ComponentId,
    ) -> anyhow::Result<StoreFileAuthorization> {
        self.function_runner_keybroker()
            .issue_store_file_authorization(rt, issued, component)
    }

    /// Private helper method to generate an admin key.
    /// If `member_id` is None, it generates a system key, otherwise
    /// an admin key for the given user.
    fn issue_key(&self, member_id: Option<MemberId>, is_read_only: bool) -> String {
        let now = SystemTime::now();
        let since_epoch = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Failed to compute seconds since epoch?");

        let identity = match member_id {
            Some(member_id) => AdminIdentityProto::MemberId(member_id.0),
            None => AdminIdentityProto::System(()),
        };
        let proto = AdminKeyProto {
            instance_name: None,
            issued_s: since_epoch.as_secs(),
            identity: Some(identity),
            is_read_only,
        };
        format_admin_key(
            &self.instance_name,
            &self
                .admin_key_encryptor
                .encrypt_proto(ADMIN_KEY_VERSION, &proto),
        )
    }

    pub fn is_encrypted_admin_key(&self, key: &str) -> bool {
        let encrypted_part = split_admin_key(key).map(|(_, key)| key).unwrap_or(key);
        let admin_key: Result<AdminKeyProto, _> = self
            .admin_key_encryptor
            .decrypt_proto(ADMIN_KEY_VERSION, encrypted_part)
            .or_else(|_| {
                self.encryptor
                    .decode_proto(ADMIN_KEY_VERSION, encrypted_part)
                    .inspect(|_| log_legacy_admin_key())
            });
        admin_key.is_ok()
    }

    pub fn check_admin_key(&self, key: &str) -> anyhow::Result<Identity> {
        let (instance_name, encrypted_part) = split_admin_key(key)
            .map(|(name, key)| (Some(remove_type_prefix_from_instance_name(name)), key))
            .unwrap_or((None, key));
        let AdminKeyProto {
            instance_name: instance_name_from_encrypted_part,
            issued_s,
            identity,
            is_read_only,
        } = self
            .admin_key_encryptor
            .decrypt_proto(ADMIN_KEY_VERSION, encrypted_part)
            .or_else(|_| {
                self.encryptor
                    .decode_proto(ADMIN_KEY_VERSION, encrypted_part)
                    .inspect(|_| log_legacy_admin_key())
            })
            .with_context(|| format!("Couldn't decode the AdminKeyProto {key}"))?;
        let instance_name = instance_name
            .or(instance_name_from_encrypted_part.as_deref())
            .context("Invalid admin key format")?;

        if instance_name != self.instance_name {
            return Err(anyhow::anyhow!(
                "Key is for invalid instance {instance_name}",
            ));
        }
        anyhow::ensure!(issued_s != 0, "Proto missing issued_s");
        let identity = identity.context("Proto missing identity")?;

        Ok(match identity {
            AdminIdentityProto::MemberId(member_id) => Identity::InstanceAdmin(AdminIdentity {
                instance_name: self.instance_name.clone(),
                principal: AdminIdentityPrincipal::Member(MemberId(member_id)),
                key: key.to_string(),
                is_read_only,
                allowed_ops: operations_for_deploy_key(is_read_only),
            }),
            AdminIdentityProto::System(()) => Identity::system(),
        })
    }

    pub fn check_store_file_authorization<RT: Runtime>(
        &self,
        rt: &RT,
        store_file_authorization: &str,
        validity: Duration,
    ) -> anyhow::Result<ComponentId> {
        let StorageTokenProto {
            instance_name,
            issued_s,
            authorization_type,
            component_id,
        } = self
            .store_file_encryptor
            .decrypt_proto(STORE_FILE_AUTHZ_VERSION, store_file_authorization)
            .context(ErrorMetadata::unauthenticated(
                "StorageTokenInvalid",
                "Couldn't decode the StoreFileAuthorization token",
            ))?;

        if instance_name != self.instance_name {
            anyhow::bail!(ErrorMetadata::unauthenticated(
                "InvalidStorageToken",
                "Storage token is for invalid instance {instance_name}"
            ));
        }

        anyhow::ensure!(issued_s != 0, "Proto missing issued_s");
        let now = rt.unix_timestamp().as_secs();
        if issued_s + validity.as_secs() <= now {
            log_store_file_auth_expired();
            anyhow::bail!(ErrorMetadata::unauthenticated(
                "StorageTokenExpired",
                "Store File Authorization expired",
            ));
        }

        let Some(AuthorizationTypeProto::StoreFile(StoreFileProto {})) = authorization_type else {
            anyhow::bail!(ErrorMetadata::unauthenticated(
                "InvalidStorageToken",
                "Storage token is for invalid instance {instance_name}"
            ));
        };

        let component = ComponentId::deserialize_from_string(component_id.as_deref()).context(
            ErrorMetadata::unauthenticated("InvalidStorageToken", "Invalid component ID"),
        )?;

        Ok(component)
    }

    /// Serializes and encrypts the provided Cursor for sending to clients.
    pub fn encrypt_cursor(&self, cursor: &Cursor) -> SerializedCursor {
        self.function_runner_keybroker().encrypt_cursor(cursor)
    }

    pub fn decrypt_cursor(&self, cursor: SerializedCursor) -> anyhow::Result<Cursor> {
        self.function_runner_keybroker().decrypt_cursor(cursor)
    }

    pub fn encrypt_query_journal(
        &self,
        journal: &QueryJournal,
        persistence_version: PersistenceVersion,
    ) -> SerializedQueryJournal {
        let query_journal_version = persistence_version.index_key_version(QUERY_JOURNAL_VERSION);
        let cursor = match &journal.end_cursor {
            Some(cursor) => Some(cursor_to_proto(&self.instance_name, cursor)),
            None => return None,
        };
        let proto = InstanceQueryJournalProto { end_cursor: cursor };
        Some(
            self.journal_encryptor
                .encrypt_proto(query_journal_version, &proto),
        )
    }

    pub fn decrypt_query_journal(
        &self,
        journal: SerializedQueryJournal,
        persistence_version: PersistenceVersion,
    ) -> anyhow::Result<QueryJournal> {
        let query_journal_version = persistence_version.index_key_version(QUERY_JOURNAL_VERSION);
        match journal {
            None => Ok(QueryJournal::new()),
            Some(journal) => {
                let proto: InstanceQueryJournalProto = self
                    .journal_encryptor
                    .decrypt_proto(query_journal_version, &journal)
                    .with_context(cursor_parse_error)?;
                let end_cursor = match proto.end_cursor {
                    Some(cursor) => Some(proto_to_cursor(&self.instance_name, cursor)?),
                    None => None,
                };
                Ok(QueryJournal { end_cursor })
            },
        }
    }

    pub fn issue_action_token(&self, component_id: ComponentId) -> ActionCallbackToken {
        let now = SystemTime::now();
        let since_epoch = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Failed to compute seconds since epoch?");

        let proto = ActionCallbackTokenProto {
            issued_s: since_epoch.as_secs(),
            component_id: component_id.serialize_to_string(),
        };

        self.action_callback_encryptor
            .encrypt_proto(ACTION_KEY_VERSION, &proto)
    }

    // Checks the action token and returns its issue time.
    pub fn check_action_token(
        &self,
        token: &ActionCallbackToken,
        validity: Duration,
    ) -> anyhow::Result<(SystemTime, ComponentId)> {
        let ActionCallbackTokenProto {
            issued_s,
            component_id,
        } = self
            .action_callback_encryptor
            .decrypt_proto(ACTION_KEY_VERSION, token)
            .with_context(|| format!("Couldn't decode ActionCallbackTokenProto {token}"))?;

        anyhow::ensure!(issued_s != 0, "ActionCallbackTokenProto missing issued_s");

        let now = SystemTime::now();
        let since_epoch = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Failed to compute seconds since epoch?")
            .as_secs();
        if issued_s + validity.as_secs() <= since_epoch {
            log_actions_token_expired();
            // Note we don't use .context(AuthError::TokenExpired) since this is
            // Convex infrastructure error and should be logged as internal error.
            return Err(anyhow::anyhow!("Action callback token expired"));
        }

        let system_time = SystemTime::UNIX_EPOCH + Duration::from_secs(issued_s);
        let component_id = ComponentId::deserialize_from_string(component_id.as_deref())?;
        Ok((system_time, component_id))
    }
}

fn proto_to_cursor(instance_name: &str, proto: InstanceCursorProto) -> anyhow::Result<Cursor> {
    if proto.instance_name != instance_name {
        anyhow::bail!(ErrorMetadata::bad_request(
            "InvalidCursor",
            format!("Key is invalid for instance {:?}", proto.instance_name)
        ));
    }

    let cursor_position = match proto.position {
        Some(PositionProto::End(())) => CursorPosition::End,
        Some(PositionProto::After(IndexKeyProto {
            values: proto_values,
        })) => CursorPosition::After(IndexKeyBytes(proto_values)),
        None => anyhow::bail!(ErrorMetadata::bad_request(
            "InvalidCursor",
            "Missing position field"
        )),
    };
    Ok(Cursor {
        position: cursor_position,
        query_fingerprint: proto.query_fingerprint,
    })
}

fn cursor_to_proto(instance_name: &str, cursor: &Cursor) -> InstanceCursorProto {
    let position = match cursor.position {
        CursorPosition::End => PositionProto::End(()),
        CursorPosition::After(ref key) => PositionProto::After(IndexKeyProto {
            values: key.clone().0,
        }),
    };
    InstanceCursorProto {
        instance_name: instance_name.to_owned(),
        position: Some(position),
        query_fingerprint: cursor.query_fingerprint.clone(),
    }
}

/// More restricted KeyBroker that only has access to some secrets
#[derive(Clone)]
pub struct FunctionRunnerKeyBroker {
    pub instance_name: String,
    pub cursor_encryptor: DeterministicEncryptor,
    pub store_file_encryptor: RandomEncryptor,
}

impl FunctionRunnerKeyBroker {
    pub fn issue_store_file_authorization<RT: Runtime>(
        &self,
        rt: &RT,
        issued: UnixTimestamp,
        component: ComponentId,
    ) -> anyhow::Result<StoreFileAuthorization> {
        let now = rt.unix_timestamp();
        if (now - issued) > MAX_TS_DELAY {
            anyhow::bail!("Could not issue authorization. Issued TS too far in past.");
        }
        let component_str = component.serialize_to_string();
        Ok(StoreFileAuthorization(
            self.store_file_encryptor.encrypt_proto(
                STORE_FILE_AUTHZ_VERSION,
                &StorageTokenProto {
                    instance_name: self.instance_name.clone(),
                    issued_s: issued.as_secs(),
                    authorization_type: Some(AuthorizationTypeProto::StoreFile(StoreFileProto {})),
                    component_id: component_str,
                },
            ),
        ))
    }

    /// Serializes and encrypts the provided Cursor for sending to clients.
    pub fn encrypt_cursor(&self, cursor: &Cursor) -> SerializedCursor {
        let proto = cursor_to_proto(&self.instance_name, cursor);
        self.cursor_encryptor.encrypt_proto(CURSOR_VERSION, &proto)
    }

    /// Attempts to decrypt and deserialize the EncryptedCursor. May fail if the
    /// client is sending up an old version.
    pub fn decrypt_cursor(&self, cursor: SerializedCursor) -> anyhow::Result<Cursor> {
        let proto: InstanceCursorProto = self
            .cursor_encryptor
            .decrypt_proto(CURSOR_VERSION, &cursor)
            .with_context(cursor_parse_error)?;
        proto_to_cursor(&self.instance_name, proto)
    }
}
