use common::types::remove_type_prefix_from_admin_key;
use hmac::{
    Hmac,
    Mac,
};
use sha2::Sha256;

use crate::DeploymentSecret;

/// Number of trailing characters of the normalized admin key core to expose as
/// an identifying preview in UIs.
pub const ADMIN_KEY_SUFFIX_LEN: usize = 8;

type HmacSha256 = Hmac<Sha256>;

/// Stable 32-byte identity for an admin key. Used as the lookup key in the
/// `_admin_keys` system table.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AdminKeyHash(pub [u8; 32]);

impl AdminKeyHash {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Normalize a raw admin key (strip deployment-type prefix and impersonation
/// suffix) and compute `HMAC-SHA-256(normalized, INSTANCE_SECRET)`.
pub fn admin_key_hash(raw: &str, secret: &DeploymentSecret) -> AdminKeyHash {
    let core = normalized_core(raw);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(core.as_bytes());
    let out = mac.finalize().into_bytes();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&out);
    AdminKeyHash(bytes)
}

/// Last `n` ASCII characters of the normalized admin key core, for displaying
/// a short identifying preview in UIs (e.g. so users can tell which key they
/// are about to revoke). Returns the whole core if it is shorter than `n`.
pub fn admin_key_suffix(raw: &str, n: usize) -> String {
    let core = normalized_core(raw);
    if core.len() <= n {
        core
    } else {
        // Admin keys are ASCII, so byte slicing matches char slicing.
        core[core.len() - n..].to_string()
    }
}

fn normalized_core(raw: &str) -> String {
    let without_prefix = remove_type_prefix_from_admin_key(raw);
    match without_prefix.split_once(':') {
        Some((base, _acting_user_b64)) => base.to_string(),
        None => without_prefix,
    }
}

#[cfg(test)]
mod tests {
    use common::types::MemberId;

    use super::{
        admin_key_hash,
        AdminKeyHash,
    };
    use crate::{
        DeploymentSecret,
        KeyBroker,
    };

    fn broker() -> KeyBroker {
        let secret = DeploymentSecret::try_from(
            "4242424242424242424242424242424242424242424242424242424242424242",
        )
        .unwrap();
        KeyBroker::new("flying-fox-123", secret).unwrap()
    }

    #[test]
    fn hash_is_stable_across_prefix_and_impersonation_suffix() {
        let broker = broker();
        let secret = broker.deployment_secret();
        let admin_key = broker.issue_admin_key(MemberId(0));
        let raw = admin_key.as_str().to_string();

        let with_type_prefix = format!("prod:{raw}");
        let with_impersonation = format!("{raw}:dGVzdA"); // "test" b64

        let a: AdminKeyHash = admin_key_hash(&raw, secret);
        let b: AdminKeyHash = admin_key_hash(&with_type_prefix, secret);
        let c: AdminKeyHash = admin_key_hash(&with_impersonation, secret);

        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn suffix_strips_prefix_and_impersonation() {
        let broker = broker();
        let admin_key = broker.issue_admin_key(MemberId(0));
        let raw = admin_key.as_str().to_string();
        let with_type_prefix = format!("prod:{raw}");
        let with_impersonation = format!("{raw}:dGVzdA");

        let s_a = super::admin_key_suffix(&raw, 8);
        let s_b = super::admin_key_suffix(&with_type_prefix, 8);
        let s_c = super::admin_key_suffix(&with_impersonation, 8);

        assert_eq!(s_a.len(), 8);
        assert_eq!(s_a, s_b);
        assert_eq!(s_a, s_c);
        assert!(raw.ends_with(&s_a));
    }

    #[test]
    fn hash_differs_for_different_secrets() {
        let broker_a = broker();
        let broker_b = {
            let secret = DeploymentSecret::try_from(
                "1111111111111111111111111111111111111111111111111111111111111111",
            )
            .unwrap();
            KeyBroker::new("flying-fox-123", secret).unwrap()
        };
        let key = broker_a.issue_admin_key(MemberId(0));
        let a = admin_key_hash(key.as_str(), broker_a.deployment_secret());
        let b = admin_key_hash(key.as_str(), broker_b.deployment_secret());
        assert_ne!(a, b);
    }
}
