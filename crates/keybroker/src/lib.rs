#![feature(try_blocks)]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

mod broker;
mod encryptor;
mod legacy_encryptor;
mod metrics;
mod operations;
mod secret;
pub use pb::convex_identity::DeploymentOperation;
pub use sync_types::UserIdentityAttributes;

pub use self::{
    broker::{
        AdminIdentity,
        AdminIdentityPrincipal,
        CoreIdTokenWithCustomClaims,
        CustomClaims,
        FunctionRunnerKeyBroker,
        GetFileAuthorization,
        Identity,
        KeyBroker,
        StoreFileAuthorization,
        SystemKey,
        UserIdentity,
    },
    encryptor::Encryptor,
    legacy_encryptor::LegacyEncryptor,
    operations::{
        bad_admin_key_error,
        operations_for_deploy_key,
        read_only_operations,
        DeploymentOp,
    },
    secret::{
        InstanceSecret,
        Secret,
    },
};

pub const DEV_INSTANCE_NAME: &str = include_str!("../dev/instance_name.txt");
pub const DEV_SECRET: &str = include_str!("../dev/secret.txt");
