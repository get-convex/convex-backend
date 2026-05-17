//! Authentication: tokens, deploy keys, and identity extraction.

pub mod deploy_keys;
pub mod identity;
pub mod tokens;

pub use deploy_keys::{
    encode_deploy_key,
    parse_deploy_key,
    DeployKeyParts,
};
pub use identity::{
    AuthIdentity,
    OptionalAuth,
};
pub use tokens::{
    encode_pat,
    parse_token,
    sha256_hex,
    suffix_of,
    TokenSecret,
};
